#!/usr/bin/env python3
"""Exercise actual child dispatch, persistence, hook, and terminal decisions."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

# A round whose defect stays unresolved cannot converge, so its review keeps counting.
OPEN = {"findings": [{"kind": "defect", "origin": "diff", "decision": "fix",
                      "reason": "fixture", "evidence": "fixture"}], "unresolved": ["D1"]}

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
import review_session as session  # noqa: E402


class ReviewSessionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        (self.root / "note.md").write_text("base\n", encoding="utf-8")
        self.git("add", ".")
        self.git("commit", "-qm", "base")
        self.git("branch", "base")
        self.record = Path(self.temp.name) / "input.json"
        self.marker = Path(self.temp.name) / "launched.txt"
        self.init()

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], stderr=subprocess.STDOUT).decode().strip()

    def cli(self, *args, expected=0, root=None):
        command = [sys.executable, str(SCRIPTS / "review_gate.py"), "session", args[0],
                   "--repository-root", str(root or self.root), "--work", "465", "--base", "base", *args[1:]]
        result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
        return result

    def init(self, prior=0, root=None):
        return self.cli("init", "--prior-rounds", str(prior), "--history-source", "https://github.com/example/repo/issues/465", root=root)

    def write(self, record):
        self.record.write_text(json.dumps(record), encoding="utf-8")
        return str(self.record)

    def result(self, **changes):
        result = {"passes": ["requirements-pass", "fresh-context-pass"], "findings": [],
                  "unresolved": [], "disposition": "continue", "cap_reason": "fixture: human decided to cap"}
        result.update(changes)
        return result

    def finish(self, **changes):
        return self.cli("finish", "--record", self.write(self.result(**changes)))

    def run_review(self, expected=0, scope="Issue scope", **changes):
        # The child marker is the observation: a denied launch must never write it.
        program = ("from pathlib import Path; import json; "
                   f"p=Path({str(self.marker)!r}); p.write_text(p.read_text()+'x' if p.exists() else 'x'); "
                   f"print({json.dumps(self.result(**changes))!r})")
        return self.cli("run", "--scope", scope, "--", sys.executable, "-c", program, expected=expected)

    def approval(self, **changes):
        record = {"work": "465", "actor_kind": "human", "actor": "fixture-human",
                  "source": "https://github.com/example/repo/issues/465#issuecomment-123",
                  "review": 1, "review_after_round": 5, "review_through_round": 7,
                  "scope": "remaining defect D1"}
        record.update(changes)
        return record

    def test_sixth_process_never_starts(self):
        for _ in range(5):
            self.run_review(**OPEN)
        blocked = self.run_review(expected=2)
        self.assertIn("review=1 rounds=5 limit=5 total=5", blocked.stderr)
        self.assertEqual(self.marker.read_text(), "xxxxx")

    def test_ai_approval_and_unbounded_approval_rejected(self):
        for _ in range(5):
            self.run_review(**OPEN)
        legacy = {"review": None, "review_after_round": None, "review_through_round": None,
                  "after_round": 5, "through_round": 7}
        for changes in ({"actor_kind": "AI"}, {"actor_kind": "PM"}, {"review_through_round": None},
                        {"source": ""}, {"work": "463"}, {"review_after_round": 4}, {"review": 2}, legacy):
            self.cli("approve", "--record", self.write(self.approval(**changes)), expected=2)
        self.run_review(expected=2)

    def test_human_approval_is_scope_and_round_bounded(self):
        for _ in range(5):
            self.run_review(**OPEN)
        self.cli("approve", "--record", self.write(self.approval()))
        self.run_review(expected=2, **OPEN)
        self.run_review(scope="remaining defect D1", **OPEN)
        self.run_review(scope="remaining defect D1", **OPEN)
        self.run_review(scope="remaining defect D1", expected=2, **OPEN)
        self.assertEqual(len(self.marker.read_text()), 7)

    def test_commit_rebase_push_and_diff_change_do_not_reset(self):
        for _ in range(5):
            self.run_review(**OPEN)
        (self.root / "note.md").write_text("changed\n", encoding="utf-8")
        self.git("add", ".")
        self.git("commit", "-qm", "version 2")
        branch = self.git("branch", "--show-current")
        old_head = self.git("rev-parse", "HEAD")
        self.git("checkout", "base")
        (self.root / "other.md").write_text("upstream\n", encoding="utf-8")
        self.git("add", ".")
        self.git("commit", "-qm", "upstream change")
        self.git("checkout", branch)
        self.git("rebase", "base")
        self.assertNotEqual(self.git("rev-parse", "HEAD"), old_head)
        remote = Path(self.temp.name) / "remote.git"
        subprocess.check_call(["git", "init", "-q", "--bare", str(remote)])
        self.git("remote", "add", "origin", str(remote))
        self.git("push", "-q", "origin", branch)  # A successful push does not end a review.
        self.run_review(expected=2)
        self.cli("init", "--prior-rounds", "0", "--history-source", "new session", expected=2)
        self.assertEqual(len(self.marker.read_text()), 5)

    def test_worktree_shares_count_and_restored_session_preserves_count(self):
        for _ in range(5):
            self.run_review(**OPEN)
        other = Path(self.temp.name) / "worktree"
        self.git("worktree", "add", "--detach", str(other), "HEAD")
        self.cli("begin", expected=2, root=other)
        snapshot = json.loads(self.cli("status").stdout)
        clone = Path(self.temp.name) / "clone"
        self.git("clone", "-q", str(self.root), str(clone))
        self.cli("begin", expected=2, root=clone)  # Missing state is not zero.
        self.cli("init", "--record", self.write(snapshot), root=clone)
        self.cli("begin", expected=2, root=clone)

    def test_pending_round_and_failed_child_are_not_refunded(self):
        self.cli("run", "--", sys.executable, "-c", "raise SystemExit(1)", expected=2)
        self.cli("begin", expected=2)
        self.finish(disposition="interrupted")
        entry = json.loads(self.cli("begin").stdout)
        self.assertEqual(entry["number"], 2)

    def test_final_diff_evidence_is_separate_from_total(self):
        self.run_review()
        self.run_review()
        self.assertIn("converged", self.cli("check").stdout)
        # A non-Markdown file keeps the default limit; the docs-only limit has its own test.
        (self.root / "new.txt").write_text("pending\n", encoding="utf-8")
        self.cli("check", expected=2)
        self.run_review()
        self.cli("check", expected=2)
        self.run_review()
        self.assertIn("TOTAL_ROUNDS=4", self.cli("check").stdout)

    def test_docs_only_diff_stops_after_three_rounds(self):
        (self.root / "guide.md").write_text("docs only\n", encoding="utf-8")
        for _ in range(3):
            self.run_review(**OPEN)
        blocked = self.run_review(expected=2)
        self.assertIn("rounds=3 limit=3", blocked.stderr)
        self.assertEqual(self.marker.read_text(), "xxx")

    def test_instruction_source_markdown_keeps_default_limit(self):
        (self.root / "docs" / "hardware").mkdir(parents=True)
        (self.root / "docs" / "hardware" / "limits.md").write_text("safety\n", encoding="utf-8")
        for _ in range(5):
            self.run_review(**OPEN)
        self.run_review(expected=2)
        self.assertEqual(self.marker.read_text(), "xxxxx")

    def ending(self, **changes):
        record = {"work": "465", "actor_kind": "human", "actor": "fixture-human",
                  "source": "https://github.com/example/repo/issues/465#issuecomment-456",
                  "after_round": 1}
        record.update(changes)
        return record

    def test_human_ending_caps_a_clean_final_diff(self):
        self.run_review()
        self.cli("check", expected=2)
        self.cli("end", "--record", self.write(self.ending()))
        self.assertIn("REVIEW_STATE=capped TOTAL_ROUNDS=1 REVIEW=1 REVIEW_ROUNDS=1", self.cli("check").stdout)
        (self.root / "note.md").write_text("changed after ending\n", encoding="utf-8")
        self.cli("check", expected=2)

    def test_human_ending_rejects_ai_stale_round_and_unresolved_defect(self):
        self.run_review()
        self.cli("end", "--record", self.write(self.ending(actor_kind="ai")), expected=2)
        self.cli("end", "--record", self.write(self.ending(after_round=0)), expected=2)
        self.cli("begin")
        self.cli("end", "--record", self.write(self.ending(after_round=2)), expected=2)
        self.finish(unresolved=["D1"], findings=[{"kind": "defect", "origin": "diff", "decision": "fix",
                                                  "reason": "fixture", "evidence": "fixture"}])
        self.cli("end", "--record", self.write(self.ending(after_round=2)), expected=2)
        self.cli("check", expected=2)

    def test_unresolved_and_interrupted_never_converge_or_cap(self):
        self.run_review()
        self.cli("begin")
        self.finish(unresolved=["D1: execution bypass"], disposition="capped")
        self.cli("check", expected=2)
        self.cli("begin")
        self.finish(disposition="interrupted")
        self.cli("check", expected=2)

    def test_capped_is_distinct_and_requires_both_passes(self):
        self.cli("begin")
        self.finish(passes=["requirements-pass"], disposition="capped")
        self.cli("check", expected=2)
        self.cli("begin")
        self.finish(passes=["fresh-context-pass"], disposition="capped", findings=[{
            "kind": "optional", "origin": "prior-explanation", "decision": "decline",
            "reason": "No behavior or decision changes", "evidence": "note.md:1"}])
        # Two rounds free of defects may converge even with declined optional wording.
        self.assertIn("converged", self.cli("check").stdout)

    def test_explicit_cap_is_not_convergence_or_automatic_at_limit(self):
        self.cli("begin")
        self.finish(disposition="capped")
        self.assertIn("REVIEW_STATE=capped", self.cli("check").stdout)
        self.cli("begin")
        self.cli("finish", "--record", self.write(self.result(disposition="capped", cap_reason="")), expected=2)

    def test_diff_mutation_during_review_requires_interruption(self):
        self.cli("begin")
        (self.root / "note.md").write_text("new\n", encoding="utf-8")
        self.cli("finish", "--record", self.write(self.result()), expected=2)
        self.finish(disposition="interrupted")
        self.cli("check", expected=2)

    def test_subdirectory_root_gives_same_fingerprint_and_paths(self):
        (self.root / "sub").mkdir()
        (self.root / "sub" / "inner.md").write_text("inner\n", encoding="utf-8")
        (self.root / "outer.py").write_text("print()\n", encoding="utf-8")
        sub = self.root / "sub"
        self.assertEqual(session.fingerprint(sub, "base"), session.fingerprint(self.root, "base"))
        self.assertEqual(session.changed_paths(sub, "base"), ["outer.py", "sub/inner.md"])
        self.assertEqual(session.changed_paths(sub, "base"), session.changed_paths(self.root, "base"))

    def test_lock_denies_concurrent_start(self):
        path = session.state_path(self.root, "465").with_suffix(".lock")
        path.write_text("", encoding="utf-8")
        self.run_review(expected=2)
        self.assertFalse(self.marker.exists())

    def test_registered_hook_denies_actual_dispatch_at_five(self):
        # Run the command registered in settings, not only an imported predicate.
        settings = json.loads((SCRIPTS.parent / ".claude/settings.json").read_text(encoding="utf-8"))
        hook = next(h for h in settings["hooks"]["PreToolUse"] if h["matcher"] == "Agent|Task")
        command = hook["hooks"][0]["command"]
        # Repository fixture needs the guard code at the configured project path.
        import shutil
        shutil.copytree(SCRIPTS, self.root / "scripts", ignore=shutil.ignore_patterns("__pycache__"))
        env = dict(os.environ, CLAUDE_PROJECT_DIR=str(self.root), DESKCAT_REVIEW_WORK="465", DESKCAT_REVIEW_BASE="base")
        payload = json.dumps({"tool_name": "Agent", "tool_input": {"subagent_type": "fresh-context-reviewer"}})
        for number in range(1, 7):
            result = subprocess.run(["bash", "-c", command], input=payload, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0 if number <= 5 else 2, result.stderr)
            if result.returncode == 0:
                self.marker.write_text(str(number), encoding="utf-8")
                self.finish(**OPEN)
        self.assertEqual(self.marker.read_text(), "5")

    def test_local_receipt_rejects_unfinished_stale_or_wrong_head_record(self):
        (self.root / "note.md").write_text("updated\n", encoding="utf-8")
        self.git("add", ".")
        self.git("commit", "-qm", "change\n\nChange-Class: review-required\nSelf-Review: requirements-pass\nSelf-Review: fresh-context-pass\nSelf-Review: converged")

        def receipt(expected, head="HEAD"):
            result = subprocess.run([sys.executable, str(SCRIPTS / "review_gate.py"), "receipt",
                "--repository-root", str(self.root), "--base", "base", "--head", head,
                "--review-work", "465"], capture_output=True, text=True)
            self.assertEqual(result.returncode, expected, result.stdout + result.stderr)

        receipt(1)
        self.run_review()
        self.run_review()
        receipt(0)
        receipt(1, "base")
        (self.root / "note.md").write_text("changed again\n", encoding="utf-8")
        receipt(1)

    def test_hook_missing_record_and_invalid_json_fail_closed(self):
        hook = str(SCRIPTS / "hooks/review_round_guard.py")
        env = dict(os.environ, CLAUDE_PROJECT_DIR=str(self.root), DESKCAT_REVIEW_WORK="999")
        for payload in ("{", json.dumps({"tool_name": "Task", "tool_input": {"subagent_type": "consistency-inspector"}})):
            result = subprocess.run([sys.executable, hook], input=payload, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)

    def test_restored_history_cannot_claim_unapproved_sixth_round(self):
        for _ in range(5):
            self.run_review(**OPEN)
        snapshot = json.loads(self.cli("status").stdout)
        sixth = dict(snapshot["rounds"][-1], number=6, review_round=6)
        snapshot["rounds"].append(sixth)
        clone = Path(self.temp.name) / "clone"
        self.git("clone", "-q", str(self.root), str(clone))
        self.cli("init", "--record", self.write(snapshot), root=clone, expected=2)

    def change(self, text):
        # Not Markdown: keep the default limit so the counts below are about reviews only.
        (self.root / "work.txt").write_text(text, encoding="utf-8")

    def test_converged_review_ends_and_next_review_counts_from_zero(self):
        self.run_review(**OPEN)
        self.change("fixed\n")
        self.run_review()
        self.run_review()
        self.assertIn("REVIEW_STATE=converged TOTAL_ROUNDS=3 REVIEW=1 REVIEW_ROUNDS=3", self.cli("check").stdout)
        self.change("next change\n")
        for _ in range(5):
            self.run_review(**OPEN)
        blocked = self.run_review(expected=2)
        self.assertIn("review=2 rounds=5 limit=5 total=8", blocked.stderr)
        state = json.loads(self.cli("status").stdout)
        self.assertEqual([(r["number"], r["review"], r["review_round"]) for r in state["rounds"]][2:4],
                         [(3, 1, 3), (4, 2, 1)])

    def test_capped_review_ends_and_stopped_review_does_not(self):
        self.run_review()
        self.cli("end", "--record", self.write(self.ending()))
        self.change("after the capped review\n")
        self.assertEqual(json.loads(self.cli("begin").stdout)["review"], 2)
        self.finish(**OPEN)
        self.change("still open\n")
        self.assertEqual(json.loads(self.cli("begin").stdout)["review"], 2)

    def test_self_declared_cap_does_not_end_the_review(self):
        for _ in range(4):
            self.run_review(**OPEN)
        self.cli("begin")
        self.finish(disposition="capped", cap_reason="fixture: reviewer's own claim")
        self.assertIn("REVIEW_STATE=capped", self.cli("check").stdout)
        blocked = self.run_review(expected=2)
        self.assertIn("review=1 rounds=5 limit=5", blocked.stderr)

    def test_new_review_does_not_borrow_rounds_of_the_ended_one(self):
        self.run_review()
        self.run_review()
        self.assertIn("REVIEW_STATE=converged TOTAL_ROUNDS=2 REVIEW=1 REVIEW_ROUNDS=2 NEXT_REVIEW=2 NEXT_ROUND=1",
                      self.cli("check").stdout)
        self.run_review()  # Same diff, new review: one clean round is not two.
        self.assertIn("REVIEW_STATE=stopped TOTAL_ROUNDS=3 REVIEW=2 REVIEW_ROUNDS=1", self.cli("check", expected=2).stdout)
        self.run_review()
        self.assertIn("REVIEW_STATE=converged TOTAL_ROUNDS=4 REVIEW=2 REVIEW_ROUNDS=2", self.cli("check").stdout)

    def restart(self, **changes):
        record = {"work": "465", "actor_kind": "human", "actor": "fixture-human",
                  "source": "https://github.com/example/repo/issues/465#issuecomment-789", "after_round": 5}
        record.update(changes)
        return record

    def test_human_restart_starts_a_new_review_and_carries_unresolved_defects(self):
        for _ in range(5):
            self.run_review(**OPEN)
        for changes in ({"actor_kind": "ai"}, {"source": ""}, {"after_round": 4}, {"work": "463"}):
            self.cli("restart", "--record", self.write(self.restart(**changes)), expected=2)
        self.run_review(expected=2)
        self.cli("restart", "--record", self.write(self.restart()))
        entry = json.loads(self.cli("begin").stdout)
        self.assertEqual((entry["number"], entry["review"], entry["review_round"]), (6, 2, 1))
        self.finish(disposition="interrupted")  # An interrupted round does not let D1 drop.
        self.cli("begin")
        self.cli("finish", "--record", self.write(self.result()), expected=2)  # Same diff: D1 cannot be gone.
        self.finish(**OPEN)
        self.change("fixed\n")
        self.run_review()
        self.run_review()
        # The old review's rounds on another diff do not count; convergence is within the new review.
        self.assertIn("REVIEW_STATE=converged TOTAL_ROUNDS=9 REVIEW=2 REVIEW_ROUNDS=4", self.cli("check").stdout)
        self.cli("restart", "--record", self.write(self.restart(after_round=9)), expected=2)  # Already ended.
        snapshot = json.loads(self.cli("status").stdout)
        snapshot["restarts"].append(self.restart(after_round=10))  # A boundary placed ahead of the record.
        clone = Path(self.temp.name) / "clone"
        self.git("clone", "-q", str(self.root), str(clone))
        self.cli("init", "--record", self.write(snapshot), root=clone, expected=2)

    def test_restart_carries_defects_past_an_interrupted_last_round(self):
        for _ in range(4):
            self.run_review(**OPEN)
        self.cli("begin")
        self.finish(disposition="interrupted")
        self.cli("restart", "--record", self.write(self.restart()))
        self.assertEqual(json.loads(self.cli("status").stdout)["version"], 2)
        self.cli("begin")
        self.cli("finish", "--record", self.write(self.result()), expected=2)
        self.finish(**OPEN)

    def test_legacy_approval_does_not_reach_a_restarted_review(self):
        legacy = {key: value for key, value in self.approval(after_round=33, through_round=60).items()
                  if not key.startswith("review")}
        self.legacy_record(1, [legacy])
        self.cli("restart", "--record", self.write(self.restart(after_round=34)))
        for _ in range(5):
            self.run_review(scope="remaining defect D1", **OPEN)
        blocked = self.run_review(scope="remaining defect D1", expected=2)
        self.assertIn("review=2 rounds=5 limit=5 total=39", blocked.stderr)

    def test_legacy_approval_granted_at_a_boundary_reaches_no_new_review(self):
        legacy = {key: value for key, value in self.approval(after_round=33, through_round=60).items()
                  if not key.startswith("review")}
        path = self.legacy_record(0, [legacy])
        state = json.loads(path.read_text(encoding="utf-8"))
        state["restarts"] = [self.restart(after_round=33)]
        path.write_text(json.dumps(state), encoding="utf-8")
        for _ in range(5):
            self.run_review(scope="remaining defect D1", **OPEN)
        self.run_review(scope="remaining defect D1", expected=2)

    def test_legacy_approval_before_the_last_prior_round(self):
        early = {key: value for key, value in self.approval(after_round=31, through_round=60).items()
                 if not key.startswith("review")}
        self.legacy_record(1, [early])
        state = json.loads(self.cli("status").stdout)
        self.assertEqual(session.review_of(state, 31), 1)
        self.run_review(scope="remaining defect D1", **OPEN)

    def legacy_record(self, rounds, approvals=()):
        diff = session.fingerprint(self.root, "base")
        result = self.result(**OPEN)
        state = {"version": 1, "work": "465", "prior_rounds": 33, "history_source": "fixture handoff",
                 "rounds": [{"number": 33 + n, "diff": diff, "scope": "remaining defect D1",
                             "approval_source": approvals[0]["source"] if approvals else None,
                             "free_limit": 5, "result": result} for n in range(1, rounds + 1)],
                 "approvals": list(approvals)}
        path = session.state_path(self.root, "465")
        path.unlink()
        path.write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        return path

    def test_version1_record_is_read_unchanged_and_counts_strictly(self):
        legacy = self.approval(review=None, review_after_round=None, review_through_round=None,
                               after_round=33, through_round=36)
        for key in ("review", "review_after_round", "review_through_round"):
            del legacy[key]
        path = self.legacy_record(1, [legacy])
        before = path.read_bytes()
        self.cli("status")
        self.cli("check", expected=2)
        self.assertEqual(path.read_bytes(), before)
        # Rounds before and inside the record whose end is unknown stay in the current review.
        entry = json.loads(self.cli("begin", "--scope", "remaining defect D1").stdout)
        self.assertEqual((entry["number"], entry["review"], entry["review_round"]), (35, 1, 35))
        self.finish(**OPEN)
        self.run_review(scope="remaining defect D1", **OPEN)
        self.run_review(scope="remaining defect D1", expected=2)
        self.assertEqual(json.loads(self.cli("status").stdout)["version"], 2)
        self.legacy_record(1)  # The pre-#526 rule still rejects an unapproved legacy round 34.
        self.cli("status", expected=2)

    def test_pre_526_writer_stops_on_version2_record(self):
        self.run_review()
        old = Path(self.temp.name) / "old"
        old.mkdir()
        for name in ("review_session.py", "review_gate.py"):
            # 497950b is the develop commit this #526 change was written on (old record format).
            shown = subprocess.run(["git", "-C", str(SCRIPTS.parent), "show", f"497950b:scripts/{name}"],
                                   capture_output=True)
            if shown.returncode:
                self.skipTest("pre-#526 commit unavailable in this checkout")
            source = shown.stdout
            (old / name).write_bytes(source)
        result = subprocess.run([sys.executable, str(old / "review_gate.py"), "session", "begin",
                                 "--repository-root", str(self.root), "--work", "465", "--base", "base"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("version", result.stderr)


if __name__ == "__main__":
    unittest.main()
