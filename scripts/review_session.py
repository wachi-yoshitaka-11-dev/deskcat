#!/usr/bin/env python3
"""Review admission per review, recorded per Issue; shared by review_gate and the Agent hook.

This is an execution guard, not authentication or proof of review quality.
The portable JSON is copied to the existing Issue/PR for handoff; the local
copy lives in git-common-dir so branch changes cannot reset its history.
One record holds every round of the Issue (the Issue total is kept as
information). The free-round limit applies to one review: a review ends at the
round that converges, at a human-approved ending, or where a recorded human
restart begins a new one. Boundaries are computed from the recorded results, so
there is no closing step to forget; a review that never ends keeps counting.
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


# Rounds that may start without a bounded human approval, counted per review (#490, #526).
# A diff made only of Markdown files outside every instruction source gets the
# lower limit; anything else (code, scripts, safety/instruction documents,
# or an empty diff) keeps the original limit. The limit only decides when a
# human must be asked; it never lowers what convergence or capping requires.
FREE_ROUNDS = 5
FREE_ROUNDS_DOCS = 3


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args])


def toplevel(root):
    return git(root, "rev-parse", "--show-toplevel").decode().strip()


def fingerprint(root, base):
    """Include pending changes and nonignored new files; never key by HEAD."""
    root = toplevel(root)
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


def changed_paths(root, base):
    """Paths in the reviewed diff: tracked changes since merge-base plus new files."""
    root = toplevel(root)
    ancestor = git(root, "merge-base", base, "HEAD").decode().strip()
    names = git(root, "diff", "--name-only", "-z", ancestor).split(b"\0")
    names += git(root, "ls-files", "--others", "--exclude-standard", "-z").split(b"\0")
    return sorted({os.fsdecode(name) for name in names if name})


def free_limit(paths):
    """Lower limit only for a non-empty diff of Markdown outside instruction sources."""
    import review_gate  # Deferred: review_gate imports this module lazily as well.
    if paths and all(path.endswith(".md")
                     and not any(path == source or (source.endswith("/") and path.startswith(source))
                                 for source in review_gate.INSTRUCTION_SOURCES)
                     for path in paths):
        return FREE_ROUNDS_DOCS
    return FREE_ROUNDS


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
    # Version 1 records predate per-review counting (#526) and are read unchanged.
    # A writer from before #526 stops on version 2 instead of appending old-style rounds.
    if state.get("version") not in (1, 2) or state.get("work") != work:
        raise Stopped("record version/work mismatch")
    prior = state.get("prior_rounds")
    if type(prior) is not int or prior < 0 or not state.get("history_source"):
        raise Stopped("known prior round count and history source are required; unknown is not zero")
    if not isinstance(state.get("rounds"), list) or not isinstance(state.get("approvals"), list):
        raise Stopped("rounds and approvals must be lists")
    if not isinstance(state.get("endings", []), list) or not isinstance(state.get("restarts", []), list):
        raise Stopped("endings and restarts must be lists")
    for restart in state.get("restarts", []):
        if (restart.get("actor_kind") != "human" or restart.get("work") != work
                or not restart.get("actor") or not restart.get("source")
                or type(restart.get("after_round")) is not int
                or not 0 < restart["after_round"] <= prior + len(state["rounds"])):
            raise Stopped("restart must name a human actor, source, this work and a recorded round")
    reviewed = [entry for entry in state["rounds"] if "review_round" in entry]
    if reviewed and (state["version"] != 2 or state["rounds"][-len(reviewed):] != reviewed):
        raise Stopped("per-review rounds require version 2 and must follow every legacy round")
    positions, _ = layout(state)
    for (review, review_round), (number, entry) in zip(positions, enumerate(state["rounds"], prior + 1)):
        if entry.get("number") != number or not entry.get("diff"):
            raise Stopped("non-contiguous round history")
        limit = entry.get("free_limit", FREE_ROUNDS)
        if limit not in (FREE_ROUNDS, FREE_ROUNDS_DOCS):
            raise Stopped("round records an unknown free-round limit")
        if "review_round" in entry:
            if (entry.get("review"), entry["review_round"]) != (review, review_round):
                raise Stopped("round review position does not match the recorded results")
            over, approval = review_round > limit, approval_for(state, number, entry.get("scope"), review, review_round)
        else:  # Legacy round: the Issue total was the limit (pre-#526 rule).
            over, approval = number > limit, approval_for(state, number, entry.get("scope"))
        if over and (approval is None or entry.get("approval_source") != approval["source"]):
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


def approval_for(state, number, scope, review=None, review_round=None):
    """Per-review approvals bound rounds of one review; legacy ones bound Issue totals."""
    for approval in state["approvals"]:
        if not (approval.get("actor_kind") == "human"
                and approval.get("work") == state["work"]
                and approval.get("scope") == scope
                and approval.get("actor") and approval.get("source")):
            continue
        if "review" in approval:
            if (review is not None and approval["review"] == review
                    and type(approval.get("review_after_round")) is int
                    and type(approval.get("review_through_round")) is int
                    and approval["review_after_round"] < review_round <= approval["review_through_round"]):
                return approval
        elif (type(approval.get("after_round")) is int
                and type(approval.get("through_round")) is int
                and approval["after_round"] < number <= approval["through_round"]
                and (review is None or review_of(state, approval["after_round"]) == review)):
            return approval  # A legacy approval stays inside the review it was granted in.
    return None


def review_of(state, number):
    """Review that continues after Issue round `number`; none if that review has ended there."""
    positions, (next_review, _) = layout(state)
    index = number - state["prior_rounds"] - 1  # -1: the last prior round; below: earlier prior rounds.
    if index < -1:
        return 1  # Prior rounds and the round after them are all in review 1.
    review = 1 if index == -1 else positions[index][0] if index < len(positions) else None
    if index + 1 < len(positions):
        return review if positions[index + 1][0] == review else None
    return review if next_review == review else None


def restart_points(state):
    """Issue round numbers after which a human started a new review before it ended."""
    return {restart["after_round"] for restart in state.get("restarts", [])}


def status_of(state, start, count, diff):
    """Terminal state of the review holding rounds[start:count], judged on `diff`.

    Only rounds of that review count: a new review never borrows an earlier one's rounds.
    """
    rounds = state["rounds"][start:count]
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
    if human_ending(state, diff, count) is not None:
        return "capped"
    return "stopped"


def restart_carry(state):
    """The previous review's last real result, while a restarted review has recorded none.

    A restart does not clear defects: on the same diff they cannot have been fixed.
    Interrupted rounds carry no evidence, so they are skipped on both sides.
    """
    starts, _ = reviews(state)
    start, rounds = starts[-1], state["rounds"]
    if not start or state["prior_rounds"] + start not in restart_points(state):
        return None
    if any(entry.get("result") and entry["result"]["disposition"] != "interrupted" for entry in rounds[start:]):
        return None
    for entry in reversed(rounds[:start]):
        if entry.get("result") and entry["result"]["disposition"] != "interrupted":
            return entry
    return None


def reviews(state):
    """Start index of each review in rounds, and whether the latest review has ended.

    A review ends at convergence, at a human-approved ending (session end), or where
    a human restart begins the next one. A capped disposition declared in a round
    result alone does not end it: it needs only a reason string, whereas convergence
    needs two defect-free rounds on the same diff.
    """
    prior, rounds, restarts = state["prior_rounds"], state["rounds"], restart_points(state)
    starts, ended = [0], False
    for count in range(len(rounds) + 1):
        ended = prior + count in restarts or (count > starts[-1] and (
            status_of(state, starts[-1], count, rounds[count - 1]["diff"]) == "converged"
            or (rounds[count - 1].get("result") is not None
                and human_ending(state, rounds[count - 1]["diff"], count) is not None
                and status_of(state, starts[-1], count, rounds[count - 1]["diff"]) == "capped")))
        if ended and count < len(rounds):
            starts.append(count)
    return starts, bool(ended)


def layout(state):
    """(review, round within review) for each recorded round and for the next one.

    Rounds before the record (prior_rounds) count into the first review, because
    where they ended is not recorded: the unknown side is the stricter side.
    """
    starts, ended = reviews(state)
    positions = []
    for index in range(len(state["rounds"])):
        review = max(n for n, start in enumerate(starts, 1) if start <= index)
        offset = state["prior_rounds"] if review == 1 else 0
        positions.append((review, offset + index - starts[review - 1] + 1))
    if ended:
        return positions, (len(starts) + 1, 1)
    offset = state["prior_rounds"] if len(starts) == 1 else 0
    return positions, (len(starts), offset + len(state["rounds"]) - starts[-1] + 1)


def completed(state, diff):
    """Terminal declarations cannot turn an unfinished/defective round green."""
    starts, _ = reviews(state)
    count = len(state["rounds"])
    if state["prior_rounds"] + count in restart_points(state):
        return "stopped"  # A restarted review has no round yet.
    return status_of(state, starts[-1], count, diff)


def human_ending(state, diff, after_round_index):
    """A human-approved ending of review on this exact diff after the latest round."""
    for ending in state.get("endings", []):
        if (ending.get("actor_kind") == "human" and ending.get("work") == state["work"]
                and ending.get("actor") and ending.get("source")
                and ending.get("diff") == diff
                and ending.get("after_round") == state["prior_rounds"] + after_round_index):
            return ending
    return None


def begin(root, work, base, scope):
    with locked(root, work) as path:
        state = validate(read_json(path), work)
        if state["rounds"] and state["rounds"][-1].get("result") is None:
            raise Stopped("unfinished round: record its result/interruption before another review")
        number = total(state) + 1
        _, (review, review_round) = layout(state)
        approval = approval_for(state, number, scope, review, review_round)
        limit = free_limit(changed_paths(root, base))
        if review_round > limit and approval is None:
            raise Stopped(f"stopped: review={review} rounds={review_round - 1} limit={limit} total={total(state)}; "
                          f"explicit bounded human approval required before round {review_round} of this review")
        entry = {"number": number, "diff": fingerprint(root, base), "scope": scope,
                 "approval_source": approval["source"] if approval else None,
                 "free_limit": limit, "review": review, "review_round": review_round, "result": None}
        state["version"] = 2
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
        raise Stopped("capped requires the human approval source or the two-round optional-only rationale")
    with locked(root, work) as path:
        state = validate(read_json(path), work)
        if not state["rounds"] or state["rounds"][-1].get("result") is not None:
            raise Stopped("no active round")
        entry = state["rounds"][-1]
        if result["disposition"] == "interrupted":
            result["passes"] = []
        elif entry["diff"] != fingerprint(root, base):
            raise Stopped("diff changed during review; record interruption, then review the new diff")
        else:
            carried = restart_carry(state)
            if carried is not None and carried["diff"] == entry["diff"] \
                    and not set(carried["result"]["unresolved"]) <= set(result["unresolved"]):
                raise Stopped("restarted review must carry the previous review's unresolved defects on the same diff")
        entry["result"] = result
        save(path, state)
        return state


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("init", "status", "begin", "finish", "approve", "end", "restart", "run", "check"))
    parser.add_argument("--repository-root", default=".")
    parser.add_argument("--work", required=True)
    parser.add_argument("--base", default="origin/develop")
    parser.add_argument("--scope", default="Issue scope")
    parser.add_argument("--prior-rounds", type=int)
    parser.add_argument("--history-source")
    parser.add_argument("--record", help="portable handoff JSON (init), result JSON (finish), approval JSON (approve), human ending JSON (end), or human restart JSON (restart)")
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
            print(f"REVIEW_ROUND={entry['review_round']} REVIEW={entry['review']} TOTAL_ROUND={entry['number']}", flush=True)
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
                raise Stopped("approve requires --record with human approval provenance")
            approval = read_json(args.record)
            with locked(root, work) as path:
                state = validate(read_json(path), work)
                _, (review, review_round) = layout(state)
                if (approval.get("actor_kind") != "human" or approval.get("work") != work
                        or not approval.get("actor") or not approval.get("source")
                        or not approval.get("scope")
                        or approval.get("review") != review
                        or approval.get("review_after_round") != review_round - 1
                        or type(approval.get("review_through_round")) is not int
                        or approval["review_through_round"] < review_round):
                    raise Stopped(f"human approval must name this work, review={review}, review_after_round={review_round - 1}, "
                                  "a finite review_through_round, scope, actor and source")
                state["approvals"].append(approval)
                state["version"] = 2  # Writers from before #526 must not read per-review approvals as absent.
                save(path, state)
        elif args.action == "end":
            if not args.record:
                raise Stopped("end requires --record with human approval provenance")
            ending = read_json(args.record)
            with locked(root, work) as path:
                state = validate(read_json(path), work)
                diff = fingerprint(root, args.base)
                rounds = state["rounds"]
                if (ending.get("actor_kind") != "human" or ending.get("work") != work
                        or not ending.get("actor") or not ending.get("source")
                        or ending.get("after_round") != total(state)):
                    raise Stopped("human ending must name this work, the current total, actor and source")
                if not rounds or rounds[-1].get("result") is None:
                    raise Stopped("end requires the latest round to be finished")
                ending = dict(ending, diff=diff)
                trial = dict(state, endings=[*state.get("endings", []), ending])
                if completed(trial, diff) != "capped" or completed(state, diff) == "converged":
                    raise Stopped("end requires both passes on this diff, no unresolved defect, and no convergence yet")
                state["endings"] = trial["endings"]
                save(path, state)
        elif args.action == "restart":
            if not args.record:
                raise Stopped("restart requires --record with human approval provenance")
            restart = read_json(args.record)
            with locked(root, work) as path:
                state = validate(read_json(path), work)
                if (restart.get("actor_kind") != "human" or restart.get("work") != work
                        or not restart.get("actor") or not restart.get("source")
                        or restart.get("after_round") != total(state)):
                    raise Stopped("human restart must name this work, the current total, actor and source")
                if state["rounds"] and state["rounds"][-1].get("result") is None:
                    raise Stopped("restart requires the latest round to be finished")
                if not total(state) or reviews(state)[1]:
                    raise Stopped("no review in progress; the next round already starts a new review")
                state["restarts"] = [*state.get("restarts", []), restart]
                state["version"] = 2  # Writers from before #526 would ignore the boundary.
                validate(state, work)
                save(path, state)
        else:
            state = validate(read_json(state_path(root, work)), work)
        if args.action == "check":
            status = completed(state, fingerprint(root, args.base))
            positions, (next_review, next_round) = layout(state)
            review, review_rounds = positions[-1] if positions else (1, state["prior_rounds"])
            print(f"REVIEW_STATE={status} TOTAL_ROUNDS={total(state)} REVIEW={review} REVIEW_ROUNDS={review_rounds} "
                  f"NEXT_REVIEW={next_review} NEXT_ROUND={next_round}")
            return 0 if status in ("converged", "capped") else 2
        print(json.dumps(state, ensure_ascii=False, indent=2))
        return 0
    except (OSError, ValueError, TypeError, KeyError, AttributeError, subprocess.CalledProcessError) as exc:
        print(f"REVIEW_STATE=stopped: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
