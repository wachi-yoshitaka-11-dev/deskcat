#!/usr/bin/env python3
"""Exercise actual child dispatch, persistence, hook, and terminal decisions."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

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

    def run_review(self, expected=0, scope="Issue scope"):
        # The child marker is the observation: a denied launch must never write it.
        program = ("from pathlib import Path; import json; "
                   f"p=Path({str(self.marker)!r}); p.write_text(p.read_text()+'x' if p.exists() else 'x'); "
                   f"print({json.dumps(self.result())!r})")
        return self.cli("run", "--scope", scope, "--", sys.executable, "-c", program, expected=expected)

    def approval(self, **changes):
        record = {"work": "465", "actor_kind": "human", "actor": "fixture-human",
                  "source": "https://github.com/example/repo/issues/465#issuecomment-123",
                  "after_round": 5, "through_round": 7, "scope": "remaining defect D1"}
        record.update(changes)
        return record

    def test_sixth_process_never_starts(self):
        for _ in range(5):
            self.run_review()
        blocked = self.run_review(expected=2)
        self.assertIn("total=5", blocked.stderr)
        self.assertEqual(self.marker.read_text(), "xxxxx")

    def test_ai_approval_and_unbounded_approval_rejected(self):
        for _ in range(5):
            self.run_review()
        for changes in ({"actor_kind": "AI"}, {"actor_kind": "PM"}, {"through_round": None},
                        {"source": ""}, {"work": "463"}, {"after_round": 4}):
            self.cli("approve", "--record", self.write(self.approval(**changes)), expected=2)
        self.run_review(expected=2)

    def test_human_approval_is_scope_and_round_bounded(self):
        for _ in range(5):
            self.run_review()
        self.cli("approve", "--record", self.write(self.approval()))
        self.run_review(expected=2)
        self.run_review(scope="remaining defect D1")
        self.run_review(scope="remaining defect D1")
        self.run_review(scope="remaining defect D1", expected=2)
        self.assertEqual(len(self.marker.read_text()), 7)

    def test_commit_rebase_and_diff_change_do_not_reset(self):
        for _ in range(5):
            self.run_review()
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
        self.run_review(expected=2)
        self.cli("init", "--prior-rounds", "0", "--history-source", "new session", expected=2)
        self.assertEqual(len(self.marker.read_text()), 5)

    def test_worktree_shares_count_and_restored_session_preserves_count(self):
        for _ in range(5):
            self.run_review()
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
        (self.root / "new.md").write_text("pending\n", encoding="utf-8")
        self.cli("check", expected=2)
        self.run_review()
        self.cli("check", expected=2)
        self.run_review()
        self.assertIn("TOTAL_ROUNDS=4", self.cli("check").stdout)

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
                self.finish()
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
            self.run_review()
        snapshot = json.loads(self.cli("status").stdout)
        sixth = dict(snapshot["rounds"][-1], number=6)
        snapshot["rounds"].append(sixth)
        clone = Path(self.temp.name) / "clone"
        self.git("clone", "-q", str(self.root), str(clone))
        self.cli("init", "--record", self.write(snapshot), root=clone, expected=2)


if __name__ == "__main__":
    unittest.main()
