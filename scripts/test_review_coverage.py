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

# レビューが行われたときの報告の形（指摘の一覧の JSON の囲み）。
REPORT = '```json\n[{"file": "a.py", "summary": "x"}]\n```'


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

    def test_pairs_by_tool_use_id_and_needs_success_and_report(self):
        records = [
            {"event": "started", "tool_use_id": "t1", "from": self.A, "to": self.B},
            {"event": "completed", "tool_use_id": "t1", "success": True, "report": True},
            {"event": "started", "tool_use_id": "t2", "from": self.B, "to": self.C},
            {"event": "completed", "tool_use_id": "t2", "success": False, "report": True},
            {"event": "started", "tool_use_id": "t3", "from": self.B, "to": self.C},
            {"event": "completed", "tool_use_id": "zz", "success": True, "report": True},
            {"event": "started", "tool_use_id": "t4", "from": "short", "to": self.C},
            {"event": "completed", "tool_use_id": "t4", "success": True, "report": True},
            # 印の無い記録（印を付ける前の記録）と、報告の形でなかった記録は数えない。
            {"event": "started", "tool_use_id": "t5", "from": self.B, "to": self.C},
            {"event": "completed", "tool_use_id": "t5", "success": True},
            {"event": "started", "tool_use_id": "t6", "from": self.B, "to": self.C},
            {"event": "completed", "tool_use_id": "t6", "success": True, "report": False},
        ]
        self.assertEqual(rc.completed_ranges(records), [(self.A, self.B)])


class FindingsReportTests(unittest.TestCase):
    """完了と数える報告の形（`is_findings_report`）。"""

    def test_reports_with_one_fence_of_objects_count(self):
        for result in (
            REPORT,
            "```json\n[]\n```",
            '前置き\n```json\n[{"file": "a.py"}, {"file": "b.py"}]\n```\n後書き',
        ):
            with self.subTest(result=result):
                self.assertTrue(rc.is_findings_report(result))

    def test_other_results_do_not_count(self):
        for result in (
            # 2026-11-28 に、利用の上限で止まった fork が返した実際の文。
            "You've hit your 5-hour limit · resets 6:50pm (Asia/Tokyo)",
            '[{"file": "a.py"}]',
            REPORT + "\n" + REPORT,
            "```json\n{\"file\": \"a.py\"}\n```",
            "```json\n[1, 2]\n```",
            "```json\n[not json]\n```",
            "",
            None,
        ):
            with self.subTest(result=result):
                self.assertFalse(rc.is_findings_report(result))


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


def _hook(payload, cwd, raw=None):
    """記録の hook を、Claude Code と同じく UTF-8 の byte で入力して走らせる。

    既定の文字コードを cp932 にする（Windows の既定）。前に `PYTHONIOENCODING=utf-8` で走らせて
    いたため、hook が cp932 で読んで ASCII でない本文を落とすことを隠していた（2026-11-28）。
    """
    data = raw if raw is not None else json.dumps(payload, ensure_ascii=False).encode("utf-8")
    result = subprocess.run(
        [sys.executable, RECORD_HOOK], input=data, capture_output=True, cwd=cwd, timeout=60,
        env=dict(os.environ, PYTHONIOENCODING="cp932", PYTHONUTF8="0"),
    )
    text = result.stdout.decode("utf-8").strip()
    _hook.stderr = result.stderr
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
                                                  "result": REPORT})
        records = rc.read_log(self.repo.log)
        self.assertEqual(rc.completed_ranges(records), [(first, last)])

    def test_record_after_a_cut_fragment_still_counts(self):
        # 前の書き込みが途中で切れて、改行の無い断片が残っていても、次の記録は壊れない。
        first, _, last = self.repo.shas
        self.repo.log.parent.mkdir(parents=True, exist_ok=True)
        self.repo.log.write_bytes(b'{"event": "started", "tool_use_id": "cut"')
        self._skill("PreToolUse", f"{first[:7]}..HEAD")
        self._skill("PostToolUse", "x", response={"success": True, "result": REPORT})
        self.assertEqual(rc.completed_ranges(rc.read_log(self.repo.log)), [(first, last)])

    def test_log_cut_inside_a_multibyte_character_is_still_read(self):
        first, _, last = self.repo.shas
        self.repo.log.parent.mkdir(parents=True, exist_ok=True)
        self.repo.log.write_bytes('{"args": "指摘'.encode("utf-8")[:-1])
        self._skill("PreToolUse", f"{first[:7]}..HEAD")
        self._skill("PostToolUse", "x", response={"success": True, "result": REPORT})
        self.assertEqual(rc.completed_ranges(rc.read_log(self.repo.log)), [(first, last)])

    def test_report_with_non_ascii_text_is_recorded(self):
        # 2026-11-28 に完了の記録を落とした形：cp932 で読めない文字を含む報告の本文。
        first, _, last = self.repo.shas
        self._skill("PreToolUse", f"{first[:7]}..HEAD")
        self._skill("PostToolUse", "x", response={
            "success": True,
            "result": '```json\n[{"summary": "指摘は0件 ✓   — \U0001f600 ①"}]\n```'})
        self.assertEqual(rc.completed_ranges(rc.read_log(self.repo.log)), [(first, last)])

    def test_unreadable_input_is_reported_with_exit_1(self):
        code, output = _hook(None, self.repo.root, raw=b"\xff\xfe not json")
        self.assertEqual(code, 1)
        self.assertIsNone(output)
        # 既定の文字コードが cp932 でも、stderr は UTF-8 で書く。
        self.assertIn("記録しなかった", _hook.stderr.decode("utf-8"))
        self.assertFalse(self.repo.log.exists())

    def test_run_without_a_report_is_not_counted(self):
        first = self.repo.shas[0]
        for index, response in enumerate((
            {"success": True, "status": "forked"},
            {"success": True, "result": "  "},
            {"success": False, "result": REPORT},
            # 2026-11-28 に、利用の上限で止まった fork の結果。`success` は真で届いた。
            {"success": True, "status": "forked", "agentId": "a",
             "result": "You've hit your 5-hour limit · resets 6:50pm (Asia/Tokyo)"},
        )):
            key = f"t{index}"
            self._skill("PreToolUse", f"{first}..HEAD", tool_use_id=key)
            self._skill("PostToolUse", "x", tool_use_id=key, response=response)
        self.assertEqual(rc.completed_ranges(rc.read_log(self.repo.log)), [])

    def test_main_context_run_is_not_recorded(self):
        self._skill("PreToolUse", "high HEAD~2..HEAD", agent=False)
        self._skill("PostToolUse", "x", agent=False, response={"success": True})
        self.assertFalse(self.repo.log.exists())

    def test_bad_ranges_are_not_recorded_but_not_blocked(self):
        for args in ("high --max-findings all", "HEAD..HEAD~2", "nope..HEAD", "HEAD..HEAD",
                     "HEAD~2...HEAD", ["HEAD~2..HEAD"]):
            with self.subTest(args=args):
                code, output = self._skill("PreToolUse", args)
                # 止めない（deny を出さない）が、黙らない（exit 1 と stderr の理由）。
                self.assertEqual(code, 1)
                self.assertIsNone(output)
                self.assertIn("記録しない", _hook.stderr.decode("utf-8"))
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
            ("Bash", {"command": f"printf x | tee -a .git/{rc.LOG_NAME}"}),
            ("Bash", {"command": f"sed -i 1d .git/{rc.LOG_NAME}"}),
        ):
            with self.subTest(tool=tool):
                payload = {"hook_event_name": "PreToolUse", "tool_name": tool,
                           "cwd": self.repo.root, "tool_input": tool_input}
                _, output = _hook(payload, self.repo.root)
                self.assertEqual(output["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_commands_that_only_name_the_hook_are_allowed(self):
        # hook の名前では止めない（止めるのは log の名前だけ）。
        hook = "scripts/hooks/" + "review_record" + ".py"
        for command in (f"git add scripts/review_coverage.py {hook}", f"git diff -- {hook}",
                        f"sed -n 1,20p {hook}", f"python3 -m py_compile {hook}"):
            with self.subTest(command=command):
                payload = {"hook_event_name": "PreToolUse", "tool_name": "Bash",
                           "cwd": self.repo.root, "tool_input": {"command": command}}
                self.assertEqual(_hook(payload, self.repo.root), (0, None))

    def test_other_writes_are_allowed(self):
        payload = {"hook_event_name": "PreToolUse", "tool_name": "Write",
                   "cwd": self.repo.root, "tool_input": {"file_path": "a.txt"}}
        self.assertEqual(_hook(payload, self.repo.root), (0, None))


class HookInputTests(unittest.TestCase):
    """`hooks/hook_input.py` と、merge の前の検査が失敗をどちらへ倒すか。"""

    def test_failures_to_read_the_stream_are_input_errors(self):
        class Broken:
            def read(self):
                raise OSError("stdin を読めない")

        for stream in (Broken(), object(), io.BytesIO(b"\xff"), io.BytesIO(b"{")):
            with self.subTest(stream=type(stream).__name__):
                with self.assertRaises(guard.hook_input.HookInputError):
                    guard.hook_input.read_payload(stream)

    def _main(self, command, error):
        payload = {"tool_name": "Bash", "tool_input": {"command": command}}
        with mock.patch.object(guard.hook_input, "read_payload", return_value=payload), \
                mock.patch.object(guard.hook_input, "report") as report, \
                mock.patch.object(guard, "_check_merge"), \
                mock.patch.object(guard, "_check_coderabbit_review"), \
                mock.patch.object(guard, "_check_review_coverage", side_effect=error):
            return guard.main(), report

    def test_unexpected_error_in_the_merge_checks_blocks(self):
        code, report = self._main("gh pr merge 7 --squash", RuntimeError("想定外"))
        self.assertEqual(code, guard.hook_input.BLOCK)
        self.assertIn("RuntimeError", report.call_args.args[0])

    def test_deny_in_the_merge_checks_is_not_turned_into_a_block(self):
        # `_deny` の `SystemExit(0)` は、予想しない例外として受けない。
        with self.assertRaises(SystemExit) as raised:
            self._main("gh pr merge 7 --squash", SystemExit(0))
        self.assertEqual(raised.exception.code, 0)


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
                                         "success": True, "report": True}) + "\n")

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

    def test_unreadable_body_range_is_skipped_like_ci(self):
        # rebase の前の範囲（もう読めない commit）が本文に残っていても、CI と同じく飛ばす。
        self._log((self.base, self.head))
        gone = ("dead111" + "0" * 33, "beef222" + "0" * 33)
        with mock.patch.object(guard.hook_input, "report") as report:
            self.assertIsNone(self._merge([gone, (self.base, self.head)]))
        self.assertIn(rc.skipped_ranges_message(["dead111..beef222"]),
                      report.call_args.args[0])

    def test_promotion_checks_each_commit_and_the_approval_comment(self):
        promotion = ("main", "develop")
        squashed = (self.mid, self.head)
        self.assertIsNone(self._merge(refs=promotion, squashed=squashed))
        files = ["AGENTS.md"]
        self.assertDenied(self._merge(files=files, refs=promotion, squashed=squashed),
                          "承認のコメント")
        self.assertIsNone(self._merge(files=files, comments=[f"承認 {self.head}"],
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
        self.assertIsNone(self._merge(ranges, ["README.md"], []))
        # 本文の前後の空白を除いた全体が「承認」＋空白＋40桁の SHA なら数える。前後の空白・改行・
        # 全角の空白、画面で書いた `\r\n`、大文字の SHA を読む。字下げした1行だけの本文
        # （U+2028 で区切ったものを含む）も、意図どおり数える。
        head = self.head
        for comment in (f"承認 {head}", f"\r\n  承認 {head}\r\n", f"　承認　{head}　",
                        f"承認 {head.upper()}", f"     承認 {head}"):
            with self.subTest(comment=comment):
                self.assertIsNone(self._merge(ranges, files, [comment]))
        # 他の文があるもの、SHA が 40 桁でないもの、別の commit の SHA、bot のものは数えない。
        for comment in (f"承認をお願いします {head}", f"確認した。\n承認 {head}",
                        f"```\n承認 {head}\n```", f"承認 {head[:7]}", f"承認 {self.mid}",
                        ("Bot", f"承認 {head}")):
            with self.subTest(comment=comment):
                self.assertDenied(self._merge(ranges, files, [comment]), "承認のコメント")
        # 想定と違う形の応答は、落ちずに止める。
        self.assertDenied(self._merge(ranges, files, [("User", ["x"])]), "確認できなかった")

    # log に足す、鎖に入りえない範囲の本数。終わりは手元に無い commit（別の clone や rebase の前）。
    STALE_RANGES = 300

    def _stale(self):
        return [(f"{index:040x}", f"{index + 1:040x}") for index in range(self.STALE_RANGES)]

    def _git_calls(self, **merge):
        """`_merge` の間に git を呼んだ回数と、その出力。"""
        real = rc._git
        with mock.patch.object(rc, "_git", side_effect=real) as spy:
            output = self._merge(**merge)
        return spy.call_count, output

    def test_git_calls_do_not_grow_with_the_log(self):
        """log の範囲の本数が増えても、merge の前の検査で git を呼ぶ回数は変わらない（#822）。"""
        ranges = [(self.base, self.mid), (self.mid, self.head)]
        self._log(*ranges)
        small, output = self._git_calls(body_ranges=ranges)
        self.assertIsNone(output)
        # 鎖の外の範囲と、終わりが merge-base の範囲（到達点より先へ進めない）を足す。
        self._log(*self._stale(), (self.base, self.base))
        large, output = self._git_calls(body_ranges=ranges)
        self.assertIsNone(output)
        self.assertEqual(small, large)

    def test_logged_body_range_off_the_chain_is_not_reported_missing(self):
        """本文に残った rebase の前の範囲は、log に記録があれば「記録が無い」と言わない（#822）。"""
        self.repo._git("checkout", "-q", "-b", "old", self.base)
        Path(self.repo.root, "g.txt").write_text("old", encoding="utf-8")
        self.repo._git("add", "g.txt")
        self.repo._git("commit", "-q", "-m", "old")
        old = self.repo._git("rev-parse", "HEAD").strip()
        self.repo._git("checkout", "-q", "main")
        ranges = [(self.base, old), (self.base, self.head)]
        # log に無ければ、鎖の外の範囲でも「記録が無い」で止める。
        self._log((self.base, self.head))
        self.assertDenied(self._merge(ranges), "記録が無い")
        self._log((self.base, old))
        self.assertIsNone(self._merge(ranges))

    def test_promotion_record_ending_outside_the_range_is_not_counted(self):
        """昇格では、`mid^..mid` を含んでも、終わりが昇格の範囲の外の記録は数えない（#822）。"""
        promotion = ("main", "develop")
        self.repo._git("checkout", "-q", "-b", "side", self.mid)
        Path(self.repo.root, "g.txt").write_text("side", encoding="utf-8")
        self.repo._git("add", "g.txt")
        self.repo._git("commit", "-q", "-m", "side")
        side = self.repo._git("rev-parse", "HEAD").strip()
        self.repo._git("checkout", "-q", "main")
        self._log((self.base, side))
        output = self._merge(refs=promotion, squashed=(self.head,))
        self.assertDenied(output, self.mid[:7])
        self.assertIn("終わりが昇格の範囲の中", _reason(output))

    def test_promotion_git_calls_do_not_grow_with_the_log(self):
        promotion = ("main", "develop")
        self._log((self.base, self.mid))
        small, output = self._git_calls(refs=promotion, squashed=(self.head,))
        self.assertIsNone(output)
        # 鎖の外の範囲を、本物の記録より前に置く（後ろに置くと、照らし合わせが先に打ち切られる）。
        Path(self.repo.log).unlink()
        self._log(*self._stale(), (self.base, self.mid))
        large, output = self._git_calls(refs=promotion, squashed=(self.head,))
        self.assertIsNone(output)
        self.assertEqual(small, large)

    def test_promotion_git_calls_do_not_grow_with_repeated_records(self):
        """同じ範囲を何度もレビューして記録が重なっても、git を呼ぶ回数は変わらない（#822）。"""
        promotion = ("main", "develop")
        # mid は PR を通らずに入った commit。`mid..head` は mid の変更を含まないので覆わない。
        self._log((self.mid, self.head))
        small, output = self._git_calls(refs=promotion, squashed=(self.head,))
        self.assertIsNotNone(output)
        self._log(*[(self.mid, self.head)] * self.STALE_RANGES)
        large, output = self._git_calls(refs=promotion, squashed=(self.head,))
        self.assertIsNotNone(output)
        self.assertEqual(small, large)

    def test_promotion_record_ends_at_the_commit_or_at_the_head(self):
        """終わりが commit そのもの、head そのもの、commit が head と同じ、のどれでも覆う（#822）。"""
        promotion = ("main", "develop")
        cases = (
            # (PR を通らずに入った commit, 記録の範囲)
            (self.mid, (self.base, self.mid)),   # 終わりが commit そのもの（`<sha>^..<sha>`）
            (self.mid, (self.base, self.head)),  # 終わりが head そのもの
            (self.head, (self.mid, self.head)),  # commit が head と同じ
        )
        for direct, record in cases:
            with self.subTest(direct=direct[:7], record=(record[0][:7], record[1][:7])):
                Path(self.repo.log).unlink(missing_ok=True)
                squashed = tuple(sha for sha in (self.mid, self.head) if sha != direct)
                self.assertDenied(self._merge(refs=promotion, squashed=squashed), direct[:7])
                self._log(record)
                self.assertIsNone(self._merge(refs=promotion, squashed=squashed))

    def test_promotion_git_calls_do_not_grow_with_distinct_records(self):
        """終わりが範囲の中にある別々の記録が増えても、git を呼ぶ回数は変わらない（#822）。

        覆う記録は、並べたときに最後に来る。覆わない記録を先に照らしても、範囲ごとには git を呼ばない。
        """
        promotion = ("main", "develop")
        # head は PR を通らずに入った commit。`base..head` が `head^..head` を含む。
        self._log((self.base, self.head))
        small, output = self._git_calls(refs=promotion, squashed=(self.mid,))
        self.assertIsNone(output)
        # 始まりが別々（手元に無い commit）で、終わりが範囲の中（mid）の記録。並べると先に来る。
        Path(self.repo.log).unlink()
        self._log(*[(f"{index:040x}", self.mid) for index in range(self.STALE_RANGES)],
                  (self.base, self.head))
        large, output = self._git_calls(refs=promotion, squashed=(self.mid,))
        self.assertIsNone(output)
        self.assertEqual(small, large)

    def test_git_calls_run_under_the_check_deadline(self):
        """gh の照会の後に期限が尽きたら、git の呼び出しの前で止める（#822）。"""
        ranges = [(self.base, self.head)]
        self._log(*ranges)
        for refs in (("develop", "x"), ("main", "develop")):
            with self.subTest(refs=refs), \
                    mock.patch.object(guard, "CHECK_DEADLINE_SECONDS", 0):
                output = self._merge(ranges, refs=refs, squashed=(self.head,))
                self.assertDenied(output, "TimeoutExpired")
                # 取れる手当て（再実行、所有者への相談）を書き、範囲を分ける手順は書かない。
                self.assertIn("時間を置いて再実行", _reason(output))
                self.assertNotIn("分け", _reason(output))

    def test_other_failures_do_not_mention_the_deadline(self):
        self._log((self.base, self.head))
        output = self._merge([(self.base, self.head)], ["AGENTS.md"], [("User", ["x"])])
        self.assertDenied(output, "確認できなかった")
        self.assertNotIn("期限", _reason(output))


class WithinTests(unittest.TestCase):
    def test_only_ranges_ending_in_the_given_commits_are_kept(self):
        ranges = [("a", "b"), ("b", "c"), ("x", "y")]
        self.assertEqual(rc.within(ranges, ["b", "c"]), [("a", "b"), ("b", "c")])
        # 同じ範囲の記録が重なっても、1本として返す。
        self.assertEqual(rc.within(ranges * 3, ["c"]), [("b", "c")])
        self.assertEqual(rc.within(ranges, []), [])

    def test_expired_deadline_raises_without_running_git(self):
        with mock.patch.object(rc.subprocess, "run") as run:
            with self.assertRaises(subprocess.TimeoutExpired):
                rc.resolve("HEAD", deadline=0)
        run.assert_not_called()


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

    def test_unreadable_ranges_are_skipped_with_a_warning(self):
        # rebase の前の範囲（CI の clone では読めない commit）が本文に残っていても、今の範囲が
        # つながっていれば通す。読めない範囲だけでは通さない。
        base, mid, head = self.repo.shas
        gone = "`dead111..beef222`"
        ok = self._run(f"## 自己レビュー\n{gone} `{base[:7]}..{head[:7]}`\n")
        self.assertEqual(ok.returncode, 0, ok.stderr)
        self.assertIn("::warning::", ok.stderr)
        self.assertIn("dead111..beef222", ok.stderr)
        only_gone = self._run(f"## 自己レビュー\n{gone}\n")
        self.assertEqual(only_gone.returncode, 1)
        self.assertIn("つながっていない", only_gone.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
