#!/usr/bin/env python3
"""Issue-scoped review admission, shared by review_gate and the Agent hook.

This is an execution guard, not authentication or proof of review quality.
The portable JSON is copied to the existing Issue/PR for handoff; the local
copy lives in git-common-dir so branch changes cannot reset its history.
"""

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys


class Stopped(ValueError):
    """Admission or completion requires human attention."""


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args])


def fingerprint(root, base):
    """Include pending changes and nonignored new files; never key by HEAD."""
    ancestor = git(root, "merge-base", base, "HEAD").decode().strip()
    digest = hashlib.sha256(git(
        root, "diff", "--no-ext-diff", "--no-textconv", "--binary",
        "--no-color", ancestor,
    ))
    for name in sorted(git(root, "ls-files", "--others", "--exclude-standard", "-z").split(b"\0")):
        if name:
            path = Path(root) / os.fsdecode(name)
            digest.update(name + b"\0" + path.read_bytes() + b"\0")
    return digest.hexdigest()


def state_path(root, work):
    if not re.fullmatch(r"[1-9][0-9]*", work):
        raise Stopped("work must be the existing Issue number (not a branch or diff ID)")
    common = Path(os.fsdecode(git(root, "rev-parse", "--git-common-dir")).strip())
    if not common.is_absolute():
        common = Path(root) / common
    return common / "deskcat-review" / (work + ".json")


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def validate(state, work):
    if state.get("version") != 1 or state.get("work") != work:
        raise Stopped("record version/work mismatch")
    prior = state.get("prior_rounds")
    if type(prior) is not int or prior < 0 or not state.get("history_source"):
        raise Stopped("known prior round count and history source are required; unknown is not zero")
    if not isinstance(state.get("rounds"), list) or not isinstance(state.get("approvals"), list):
        raise Stopped("rounds and approvals must be lists")
    for number, entry in enumerate(state["rounds"], prior + 1):
        if entry.get("number") != number or not entry.get("diff"):
            raise Stopped("non-contiguous round history")
        if number > 5:
            approval = approval_for(state, number, entry.get("scope"))
            if approval is None or entry.get("approval_source") != approval["source"]:
                raise Stopped("round history exceeds its recorded human approval")
    return state


@contextmanager
def locked(root, work):
    path = state_path(root, work)
    path.parent.mkdir(parents=True, exist_ok=True)
    lock = path.with_suffix(".lock")
    try:
        fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    except FileExistsError as exc:
        raise Stopped("review state locked; inspect interrupted writer before recovery") from exc
    os.close(fd)
    try:
        yield path
    finally:
        lock.unlink()


def save(path, state):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def total(state):
    return state["prior_rounds"] + len(state["rounds"])


def approval_for(state, number, scope):
    for approval in state["approvals"]:
        if (approval.get("actor_kind") == "human"
                and approval.get("work") == state["work"]
                and type(approval.get("after_round")) is int
                and type(approval.get("through_round")) is int
                and approval["after_round"] < number <= approval["through_round"]
                and approval.get("scope") == scope
                and approval.get("actor") and approval.get("source")):
            return approval
    return None


def completed(state, diff):
    """Terminal declarations cannot turn an unfinished/defective round green."""
    rounds = state["rounds"]
    if not rounds or rounds[-1].get("result") is None:
        return "stopped"
    last = rounds[-1]
    result = last["result"]
    if last["diff"] != diff or result["unresolved"] or result["disposition"] == "interrupted":
        return "stopped"
    same = []
    for entry in reversed(rounds):
        if (entry["diff"] != diff or entry.get("result") is None
                or entry["result"]["disposition"] == "interrupted"):
            break
        same.append(entry["result"])
    passes = {p for item in same for p in item["passes"]}
    if not {"requirements-pass", "fresh-context-pass"} <= passes:
        return "stopped"
    if len(same) >= 2 and all(not any(f["kind"] == "defect" for f in item["findings"])
                              and not item["unresolved"] for item in same[:2]):
        return "converged"
    if result.get("disposition") == "capped":
        return "capped"
    return "stopped"


def begin(root, work, base, scope):
    with locked(root, work) as path:
        state = validate(read_json(path), work)
        if state["rounds"] and state["rounds"][-1].get("result") is None:
            raise Stopped("unfinished round: record its result/interruption before another review")
        number = total(state) + 1
        approval = approval_for(state, number, scope)
        if number > 5 and approval is None:
            raise Stopped(f"stopped: total={total(state)}; explicit bounded human approval required before round {number}")
        entry = {"number": number, "diff": fingerprint(root, base), "scope": scope,
                 "approval_source": approval["source"] if approval else None, "result": None}
        state["rounds"].append(entry)
        save(path, state)  # Reserve before launching: crashes also consume the round.
        return entry


def finish(root, work, base, result):
    if not isinstance(result, dict):
        raise Stopped("result must be an object")
    if (not isinstance(result.get("unresolved"), list)
            or not all(isinstance(x, str) and x.strip() for x in result["unresolved"])
            or not isinstance(result.get("findings"), list)
            or not isinstance(result.get("passes"), list)
            or not set(result["passes"]) <= {"requirements-pass", "fresh-context-pass"}
            or result.get("disposition") not in ("continue", "capped", "interrupted")):
        raise Stopped("result requires passes, findings, unresolved and disposition")
    for finding in result["findings"]:
        if (finding.get("kind") not in ("defect", "out-of-scope", "optional")
                or finding.get("origin") not in ("diff", "prior-explanation", "pre-existing")
                or finding.get("decision") not in ("fix", "defer", "decline")
                or not finding.get("reason") or not finding.get("evidence")):
            raise Stopped("each finding requires kind, origin, decision, reason and evidence")
        if finding["kind"] == "defect" and finding["decision"] != "fix":
            raise Stopped("blocking defects cannot be declined/deferred; keep them unresolved")
    if result["disposition"] == "capped" and not result.get("cap_reason"):
        raise Stopped("capped requires the human decision source or the two-round optional-only rationale")
    with locked(root, work) as path:
        state = validate(read_json(path), work)
        if not state["rounds"] or state["rounds"][-1].get("result") is not None:
            raise Stopped("no active round")
        entry = state["rounds"][-1]
        if result["disposition"] == "interrupted":
            result["passes"] = []
        elif entry["diff"] != fingerprint(root, base):
            raise Stopped("diff changed during review; record interruption, then review the new diff")
        entry["result"] = result
        save(path, state)
        return state


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("init", "status", "begin", "finish", "approve", "run", "check"))
    parser.add_argument("--repository-root", default=".")
    parser.add_argument("--work", required=True)
    parser.add_argument("--base", default="origin/develop")
    parser.add_argument("--scope", default="Issue scope")
    parser.add_argument("--prior-rounds", type=int)
    parser.add_argument("--history-source")
    parser.add_argument("--record", help="portable handoff JSON (init), result JSON (finish), or approval JSON (approve)")
    args, command = parser.parse_known_args(argv)
    if command and (args.action != "run" or command[0] != "--"):
        parser.error("review command is allowed only after run options and --")
    root, work = args.repository_root, args.work
    try:
        if args.action == "init":
            state = read_json(args.record) if args.record else {
                "version": 1, "work": work, "prior_rounds": args.prior_rounds,
                "history_source": args.history_source, "rounds": [], "approvals": [],
            }
            validate(state, work)
            with locked(root, work) as path:
                if path.exists():
                    raise Stopped("existing work record cannot be reset or replaced; use status")
                save(path, state)
        elif args.action == "begin":
            print(json.dumps(begin(root, work, args.base, args.scope), ensure_ascii=False))
            return 0
        elif args.action == "run":
            if len(command) < 2:
                raise Stopped("run requires -- executable [args]; no shell parser is used")
            entry = begin(root, work, args.base, args.scope)
            print(f"REVIEW_ROUND={entry['number']}", flush=True)
            # Reviewer writes the same result schema used by finish to stdout.
            # A failure leaves an active round; no automatic refund/retry.
            child = subprocess.run(command[1:], cwd=root, capture_output=True, text=True, encoding="utf-8")
            if child.returncode:
                raise Stopped(f"reviewer failed ({child.returncode}); round remains unfinished")
            state = finish(root, work, args.base, json.loads(child.stdout))
        elif args.action == "finish":
            if not args.record:
                raise Stopped("finish requires --record")
            state = finish(root, work, args.base, read_json(args.record))
        elif args.action == "approve":
            if not args.record:
                raise Stopped("approve requires --record with human decision provenance")
            approval = read_json(args.record)
            with locked(root, work) as path:
                state = validate(read_json(path), work)
                if (approval.get("actor_kind") != "human" or approval.get("work") != work
                        or not approval.get("actor") or not approval.get("source")
                        or not approval.get("scope")
                        or type(approval.get("after_round")) is not int
                        or type(approval.get("through_round")) is not int
                        or approval["after_round"] != total(state)
                        or approval["through_round"] <= total(state)):
                    raise Stopped("human approval must name this work, current total, finite end round, scope, actor and source")
                state["approvals"].append(approval)
                save(path, state)
        else:
            state = validate(read_json(state_path(root, work)), work)
        if args.action == "check":
            status = completed(state, fingerprint(root, args.base))
            print(f"REVIEW_STATE={status} TOTAL_ROUNDS={total(state)}")
            return 0 if status in ("converged", "capped") else 2
        print(json.dumps(state, ensure_ascii=False, indent=2))
        return 0
    except (OSError, ValueError, TypeError, KeyError, AttributeError, subprocess.CalledProcessError) as exc:
        print(f"REVIEW_STATE=stopped: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
