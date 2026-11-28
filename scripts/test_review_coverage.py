#!/usr/bin/env python3
"""`scripts/review_coverage.py` と、それを使う hook の test（#814 の作業リストの9）。"""

import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT))

import review_coverage as rc  # noqa: E402
from test_hooks import GIT_IDENTITY, _reason, guard  # noqa: E402

RECORD_HOOK = str(SCRIPTS_ROOT / "hooks" / "review_record.py")


def _linear(*names):
    """名前の列を1本の履歴とみなす ancestor 関数を返す。"""
    order = {name: index for index, name in enumerate(names)}
    return lambda a, b: a in order and b in order and order[a] <= order[b]


class ParseArgRangeTests(unittest.TestCase):
    def test_single_range_is_taken(self):
        self.assertEqual(rc.parse_arg_range("high abc~1..def --max-findings all"),
                         ("abc~1", "def"))
        self.assertEqual(rc.parse_arg_range("origin/develop..HEAD"),
                         ("origin/develop", "HEAD"))

    def test_no_range_or_several_or_three_dots_is_rejected(self):
        for args in ("high --max-findings all", "a..b c..d", "high a...b", "", None,
                     "a..", "..b", "a....b"):
            with self.subTest(args=args):
                self.assertIsNone(rc.parse_arg_range(args))


class UncoveredTests(unittest.TestCase):
    ANCESTOR = staticmethod(_linear("m", "c1", "c2", "c3", "h"))

    def test_contiguous_ranges_reach_head(self):
        ranges = [("m", "c2"), ("c2", "h")]
        self.assertIsNone(rc.uncovered("m", "h", ranges, self.ANCESTOR))

    def test_order_and_overlap_do_not_matter(self):
        ranges = [("c1", "h"), ("m", "c2")]
        self.assertIsNone(rc.uncovered("m", "h", ranges, self.ANCESTOR))

    def test_gap_reports_where_it_stopped(self):
        ranges = [("m", "c1"), ("c2", "h")]
        self.assertEqual(rc.uncovered("m", "h", ranges, self.ANCESTOR), "c1")

    def test_no_ranges_stop_at_the_start(self):
        self.assertEqual(rc.uncovered("m", "h", [], self.ANCESTOR), "m")
        self.assertIsNone(rc.uncovered("h", "h", [], self.ANCESTOR))

    def test_a_dead_end_does_not_hide_another_path(self):
        # m から b1 と b2 に分かれ、b2 だけが h へつながる（merge commit のある履歴）。
        edges = {("m", "b1"), ("m", "b2"), ("b2", "h"), ("m", "h"), ("b1", "h")}

        def ancestor(a, b):
            return a == b or (a, b) in edges

        ranges = [("m", "b1"), ("m", "b2"), ("b2", "h")]
        self.assertIsNone(rc.uncovered("m", "h", ranges, ancestor))

    def test_range_off_the_head_path_is_not_used(self):
        # x は head の祖先でない（rebase の前の commit など）。
        ancestor = _linear("m", "c1", "h")
        self.assertEqual(rc.uncovered("m", "h", [("m", "x"), ("x", "h")], ancestor), "m")


class CompletedRangesTests(unittest.TestCase):
    A, B, C = "a" * 40, "b" * 40, "c" * 40

    def test_pairs_by_tool_use_id_and_needs_success(self):
        records = [
            {"event": "started", "tool_use_id": "t1", "from": self.A, "to": self.B},
            {"event": "completed", "tool_use_id": "t1", "success": True},
            {"event": "started", "tool_use_id": "t2", "from": self.B, "to": self.C},
            {"event": "completed", "tool_use_id": "t2", "success": False},
            {"event": "started", "tool_use_id": "t3", "from": self.B, "to": self.C},
            {"event": "completed", "tool_use_id": "zz", "success": True},
            {"event": "started", "tool_use_id": "t4", "from": "short", "to": self.C},
            {"event": "completed", "tool_use_id": "t4", "success": True},
        ]
        self.assertEqual(rc.completed_ranges(records), [(self.A, self.B)])


class BodyRangesTests(unittest.TestCase):
    def test_only_the_self_review_section_is_read(self):
        body = (
            "## 結果\n`aaaaaaa..bbbbbbb`\n"
            "## 自己レビュー\n範囲は `1111111..2222222`、`2222222..3333333` で、"
            "`4444444...5555555` は読まない。\n"
            "## Review thread\n`6666666..7777777`\n"
        )
        self.assertEqual(rc.body_ranges(body),
                         [("1111111", "2222222"), ("2222222", "3333333")])


class _Repo:
    """3 commit の履歴を持つ一時の repository。"""

    def __init__(self, directory):
        self.root = directory
        self._git("init", "-q", "-b", "main")
        self.shas = []
        for index in range(3):
            Path(directory, "f.txt").write_text(str(index), encoding="utf-8")
            self._git("add", "f.txt")
            self._git("commit", "-q", "-m", f"c{index}")
            self.shas.append(self._git("rev-parse", "HEAD").strip())

    def _git(self, *arguments):
        result = subprocess.run(
            ["git", "-C", self.root, *GIT_IDENTITY, *arguments],
            capture_output=True, text=True, encoding="utf-8", check=True,
        )
        return result.stdout

    @property
    def log(self):
        return Path(self.root, ".git", rc.LOG_NAME)


def _hook(payload, cwd):
    result = subprocess.run(
        [sys.executable, RECORD_HOOK], input=json.dumps(payload),
        capture_output=True, text=True, encoding="utf-8", cwd=cwd, timeout=60,
        env=dict(os.environ, PYTHONIOENCODING="utf-8"),
    )
    text = result.stdout.strip()
    return result.returncode, (json.loads(text) if text else None)


class ReviewRecordHookTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = _Repo(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def _skill(self, event, args, agent=True, tool_use_id="t1", response=None):
        payload = {"hook_event_name": event, "tool_name": "Skill", "cwd": self.repo.root,
                   "tool_use_id": tool_use_id,
                   "tool_input": {"skill": "code-review", "args": args}}
        if agent:
            payload["agent_id"] = "agent-1"
        if response is not None:
            payload["tool_response"] = response
        return _hook(payload, self.repo.root)

    def test_fork_run_is_recorded_with_full_shas(self):
        first, _, last = self.repo.shas
        self._skill("PreToolUse", f"high {first[:7]}..HEAD --max-findings all")
        self._skill("PostToolUse", "x", response={"success": True, "status": "forked",
                                                  "result": "指摘は0件"})
        records = rc.read_log(self.repo.log)
        self.assertEqual(rc.completed_ranges(records), [(first, last)])

    def test_run_without_a_report_is_not_counted(self):
        first = self.repo.shas[0]
        for index, response in enumerate(({"success": True, "status": "forked"},
                                          {"success": True, "result": "  "},
                                          {"success": False, "result": "途中で止まった"})):
            key = f"t{index}"
            self._skill("PreToolUse", f"{first}..HEAD", tool_use_id=key)
            self._skill("PostToolUse", "x", tool_use_id=key, response=response)
        self.assertEqual(rc.completed_ranges(rc.read_log(self.repo.log)), [])

    def test_main_context_run_is_not_recorded(self):
        self._skill("PreToolUse", "high HEAD~2..HEAD", agent=False)
        self._skill("PostToolUse", "x", agent=False, response={"success": True})
        self.assertFalse(self.repo.log.exists())

    def test_bad_ranges_are_not_recorded_but_not_blocked(self):
        for args in ("high --max-findings all", "HEAD..HEAD~2", "nope..HEAD", "HEAD..HEAD"):
            with self.subTest(args=args):
                code, output = self._skill("PreToolUse", args)
                self.assertEqual(code, 0)
                self.assertIsNone(output)
        self.assertFalse(self.repo.log.exists())

    def test_other_skills_are_ignored(self):
        payload = {"hook_event_name": "PreToolUse", "tool_name": "Skill",
                   "cwd": self.repo.root, "tool_use_id": "t", "agent_id": "a",
                   "tool_input": {"skill": "simplify", "args": "HEAD~1..HEAD"}}
        _hook(payload, self.repo.root)
        self.assertFalse(self.repo.log.exists())

    def test_writes_to_the_log_are_denied(self):
        for tool, tool_input in (
            ("Write", {"file_path": str(self.repo.log), "content": "x"}),
            ("Edit", {"file_path": f".git/{rc.LOG_NAME}"}),
            ("Bash", {"command": f"echo x >> .git/{rc.LOG_NAME}"}),
            ("PowerShell", {"command": f"Add-Content .git/{rc.LOG_NAME} x"}),
            ("Bash", {"command": "echo '{}' | python3 scripts/hooks/review_record.py"}),
        ):
            with self.subTest(tool=tool):
                payload = {"hook_event_name": "PreToolUse", "tool_name": tool,
                           "cwd": self.repo.root, "tool_input": tool_input}
                _, output = _hook(payload, self.repo.root)
                self.assertEqual(output["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_other_writes_are_allowed(self):
        payload = {"hook_event_name": "PreToolUse", "tool_name": "Write",
                   "cwd": self.repo.root, "tool_input": {"file_path": "a.txt"}}
        self.assertEqual(_hook(payload, self.repo.root), (0, None))


class ReviewCoverageMergeTests(unittest.TestCase):
    """`gh pr merge` の前に、記録が merge-base から head までつながるかを見る。"""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = _Repo(self._tmp.name)
        self.base, self.mid, self.head = self.repo.shas

    def tearDown(self):
        self._tmp.cleanup()

    def _log(self, *ranges):
        with open(self.repo.log, "a", encoding="utf-8") as handle:
            for index, (a, b) in enumerate(ranges):
                key = f"t{index}"
                handle.write(json.dumps({"event": "started", "tool_use_id": key,
                                         "from": a, "to": b}) + "\n")
                handle.write(json.dumps({"event": "completed", "tool_use_id": key,
                                         "success": True}) + "\n")

    def _merge(self, body_ranges=(), files=("README.md",), comments=(), refs=("develop", "x"),
               squashed=()):
        """`_check_review_coverage` を process の中で呼ぶ。`gh` は差し替える。

        `gh` の shim を使わないのは、Windows で shim を起動できないためである（#819）。
        git は一時の repository で本物を使う。
        """
        body = "## 自己レビュー\n" + "、".join(
            f"`{a[:7]}..{b[:7]}`" for a, b in body_ranges) + "\n"

        def fake_gh(command, deadline=None):
            if command[:2] == ["pr", "view"]:
                return json.dumps({"number": 7, "url": "https://github.com/o/r/pull/7",
                                   "baseRefOid": self.base, "headRefOid": self.head,
                                   "baseRefName": refs[0], "headRefName": refs[1],
                                   "body": body})
            path = next(a for a in command if a.startswith("repos/"))
            if path.endswith("/pulls") and "/commits/" in path:
                sha = path.split("/commits/")[1].split("/")[0]
                pulls = [{"merge_commit_sha": sha, "merged_at": "t",
                          "base": {"ref": "develop"}}] if sha in squashed else []
                return json.dumps(pulls)
            if "/files" in path:
                return "\n".join(files)
            if "/comments" in path:
                # 文字列は人のコメント、(type, 本文) の組はその type のコメントにする。
                items = [{"user": {"type": "User"}, "body": c} if isinstance(c, str)
                         else {"user": {"type": c[0]}, "body": c[1]} for c in comments]
                return json.dumps([items])
            raise AssertionError(command)

        out, here = io.StringIO(), os.getcwd()
        os.chdir(self.repo.root)
        try:
            with mock.patch.object(guard, "_gh_json", fake_gh), \
                    contextlib.redirect_stdout(out):
                try:
                    guard._check_review_coverage(["7"], None)
                except SystemExit:
                    pass
        finally:
            os.chdir(here)
        text = out.getvalue().strip()
        return json.loads(text) if text else None

    def assertDenied(self, output, text):
        self.assertIsNotNone(output, "通してしまった")
        self.assertIn(text, _reason(output))

    def test_no_record_is_denied(self):
        self.assertDenied(self._merge(), "つながっていない")

    def test_connected_records_and_body_are_allowed(self):
        ranges = [(self.base, self.mid), (self.mid, self.head)]
        self._log(*ranges)
        self.assertIsNone(self._merge(ranges))

    def test_gap_in_records_is_denied(self):
        self._log((self.base, self.mid))
        self.assertDenied(self._merge([(self.base, self.mid)]), self.mid[:7])

    def test_body_range_without_record_is_denied(self):
        self._log((self.base, self.head))
        self.assertDenied(self._merge([(self.base, self.mid), (self.mid, self.head)]),
                          "記録が無い")

    def test_body_must_reach_head_too(self):
        self._log((self.base, self.head))
        self.assertDenied(self._merge([]), "本文")

    def test_promotion_checks_each_commit_and_the_approval_comment(self):
        promotion = ("main", "develop")
        squashed = (self.mid, self.head)
        self.assertIsNone(self._merge(refs=promotion, squashed=squashed))
        files = ["AGENTS.md"]
        self.assertDenied(self._merge(files=files, refs=promotion, squashed=squashed),
                          "承認のコメント")
        self.assertIsNone(self._merge(files=files, comments=["承認 " + self.head],
                                      refs=promotion, squashed=squashed))

    def test_promotion_direct_commit_needs_a_record(self):
        promotion = ("main", "develop")
        # mid は PR を通らずに入った commit。記録が無ければ止め、`mid^..mid` を含む記録で通す。
        self.assertDenied(self._merge(refs=promotion, squashed=(self.head,)), self.mid[:7])
        self._log((self.base, self.mid))
        self.assertIsNone(self._merge(refs=promotion, squashed=(self.head,)))
        # base が main でも head が develop でなければ、昇格として扱わない。
        self.assertDenied(self._merge(refs=("main", "feature")), "つながっていない")

    def test_sensitive_path_needs_a_comment_with_the_head_sha(self):
        self._log((self.base, self.head))
        ranges = [(self.base, self.head)]
        files = ["scripts/hooks/review_record.py"]
        self.assertDenied(self._merge(ranges, files, ["承認 " + self.mid]), "承認のコメント")
        self.assertIsNone(self._merge(ranges, files, ["承認 " + self.head[:7]]))
        self.assertIsNone(self._merge(ranges, ["README.md"], []))
        # 日本語の文に続けて書いた SHA と、大文字の SHA も読む。
        self.assertIsNone(self._merge(ranges, files, [self.head[:7] + "で承認した"]))
        self.assertIsNone(self._merge(ranges, files, ["承認" + self.head[:9].upper()]))
        # bot のコメントと、「承認」の語の無いコメントは数えない。
        self.assertDenied(self._merge(ranges, files, [("Bot", "承認 " + self.head)]),
                          "承認のコメント")
        self.assertDenied(self._merge(ranges, files, ["between " + self.head]), "承認のコメント")


class CiEntryPointTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = _Repo(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def _run(self, body):
        base, _, head = self.repo.shas
        env = dict(os.environ, PR_BODY=body, PYTHONIOENCODING="utf-8")
        return subprocess.run(
            [sys.executable, str(SCRIPTS_ROOT / "review_coverage.py"), "--base", base,
             "--head", head, "--pr-body-env", "PR_BODY"],
            capture_output=True, text=True, encoding="utf-8", cwd=self.repo.root, env=env,
        )

    def test_connected_body_passes_and_gap_fails(self):
        base, mid, head = self.repo.shas
        ok = self._run(f"## 自己レビュー\n`{base[:7]}..{mid[:7]}` `{mid[:7]}..{head[:7]}`\n")
        self.assertEqual(ok.returncode, 0, ok.stderr)
        gap = self._run(f"## 自己レビュー\n`{base[:7]}..{mid[:7]}`\n")
        self.assertEqual(gap.returncode, 1)
        self.assertIn("つながっていない", gap.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
