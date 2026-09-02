#!/usr/bin/env python3
"""`validate_table_columns.py`の回帰test。

この検査は過去2回、手で数えて逆向きに外れている（#289）。**単体testはその2つの穴
（エスケープしたパイプを区切りと数える／末尾パイプ省略で1セルずれる）を、
実在した事故の形で再現する。**関数を直接importするcaseと、子processとして起動し
exit codeと診断出力まで検査するcaseの両方を持つ。

fixtureは実際にGitへcommitする。`get_tracked_files`はGitの追跡状態を見るため、
directoryへfileを置くだけでは検査対象に入らない。
"""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT / "lib"))

import publish_guards as guards  # noqa: E402

sys.path.insert(0, str(SCRIPTS_ROOT))

import validate_table_columns as check  # noqa: E402

SCRIPT = str(SCRIPTS_ROOT / "validate_table_columns.py")

GIT_IDENTITY = (
    "-c", "user.name=test",
    "-c", "user.email=test@example.invalid",
    "-c", "commit.gpgsign=false",
)


def _git(root, *arguments):
    result = subprocess.run(
        ["git", "-C", root, *GIT_IDENTITY, *arguments],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(arguments)} failed: {result.stdout}{result.stderr}"
        )
    return result.stdout


def _run(root):
    return subprocess.run(
        [sys.executable, SCRIPT, "--repository-root", root],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
        timeout=60,
    )


class SplitCellsTests(unittest.TestCase):
    """セル分割そのものの単体test。2つの既知の穴を実例で再現する。"""

    def test_escaped_pipe_is_not_a_separator(self):
        """`\\|`はセル内の文字として残り、区切りにならない。

        2026-08-31に、これを区切りと数えて`gpio-assignment.md`へ存在しない
        不整合2件が報告された。
        """
        cells = [cell.strip() for cell in check.split_cells(r"| a\|b | c |")]
        self.assertEqual(cells, ["a|b", "c"])

    def test_missing_trailing_pipe_does_not_shift_count(self):
        """末尾のパイプは省略できる。パイプ数から1を引く数え方は1セルずれる。

        2026-08-28に、この数え方で`hardware-bom.md`の実在する破損を
        「誤検知」と判定した。ここでは3列headerに対応する3セルの行が、
        末尾パイプの有無によらず3セルと数えられることを確認する。
        """
        with_trailing = [c.strip() for c in check.split_cells("| x | y | z |")]
        without_trailing = [c.strip() for c in check.split_cells("| x | y | z")]
        self.assertEqual(with_trailing, ["x", "y", "z"])
        self.assertEqual(without_trailing, ["x", "y", "z"])
        self.assertEqual(len(without_trailing), len(with_trailing))

    def test_even_backslash_run_leaves_pipe_as_delimiter(self):
        r"""`\\|`（backslash 2個）はエスケープされたbackslashに続く実区切り。

        2個目のbackslashだけを見て判定すると、直前の連続数を無視して常に
        エスケープと扱ってしまう（full reviewの指摘）。ここでは2個の行で
        パイプが区切りとして働き、3個の行では区切りにならないことを確認する。
        """
        two_backslashes = [c.strip() for c in check.split_cells(r"a\\|b")]
        self.assertEqual(two_backslashes, ["a\\", "b"])

    def test_odd_backslash_run_still_escapes_the_pipe(self):
        r"""`\\\|`（backslash 3個）はエスケープされたbackslashとエスケープされたパイプ。"""
        three_backslashes = check.split_cells(r"a\\\|b")
        self.assertEqual(three_backslashes, ["a\\|b"])

    def test_leading_and_trailing_empty_cells_are_dropped_once(self):
        """行頭・行末のパイプが作る空セルは、無ければ落とさない。"""
        self.assertEqual(check.split_cells("|a|b|"), ["a", "b"])
        self.assertEqual(check.split_cells("a|b"), ["a", "b"])


class IsDelimiterRowTests(unittest.TestCase):
    def test_accepts_alignment_variants(self):
        for row in ("| --- | --- |", "|:--|--:|", "|:-:|:-:|"):
            with self.subTest(row=row):
                self.assertTrue(check.is_delimiter_row(row))

    def test_rejects_content_row(self):
        self.assertFalse(check.is_delimiter_row("| a | b |"))


class FindMismatchesTests(unittest.TestCase):
    def test_detects_short_row(self):
        text = "\n".join([
            "| A | B | C |",
            "| --- | --- | --- |",
            "| 1 | 2 |",
        ])
        mismatches, tables, rows = check.find_mismatches(text)
        self.assertEqual(tables, 1)
        self.assertEqual(rows, 1)
        self.assertEqual(len(mismatches), 1)
        number, header_number, expected, actual, content = mismatches[0]
        self.assertEqual(number, 3)
        self.assertEqual(header_number, 1)
        self.assertEqual(expected, 3)
        self.assertEqual(actual, 2)

    def test_escaped_pipe_row_is_not_flagged(self):
        text = "\n".join([
            "| Col1 | Col2 |",
            "| --- | --- |",
            r"| a\|b | c |",
        ])
        mismatches, tables, rows = check.find_mismatches(text)
        self.assertEqual(rows, 1)
        self.assertEqual(mismatches, [])

    def test_missing_trailing_pipe_row_is_not_flagged(self):
        text = "\n".join([
            "| A | B | C |",
            "| --- | --- | --- |",
            "| x | y | z",
        ])
        mismatches, tables, rows = check.find_mismatches(text)
        self.assertEqual(rows, 1)
        self.assertEqual(mismatches, [])

    def test_fenced_table_like_content_is_ignored(self):
        """fence内の`|`はcode例示であり表ではない。"""
        text = "\n".join([
            "```text",
            "| A | B |",
            "| --- |",
            "```",
        ])
        mismatches, tables, rows = check.find_mismatches(text)
        self.assertEqual(tables, 0)
        self.assertEqual(rows, 0)
        self.assertEqual(mismatches, [])

    def test_non_table_pipes_do_not_start_a_table(self):
        """区切り行が続かない`|`は表と見なさない。"""
        text = "\n".join([
            "a | b",
            "c | d",
        ])
        mismatches, tables, rows = check.find_mismatches(text)
        self.assertEqual(tables, 0)
        self.assertEqual(mismatches, [])


class CommandLineTests(unittest.TestCase):
    """script全体を子processとして起動し、exit codeと出力を検査する。"""

    def setUp(self):
        self.directory = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(self.directory))
        _git(self.directory, "init", "--quiet", ".")

    def _commit(self, relative_path, content):
        path = Path(self.directory) / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        _git(self.directory, "add", relative_path)
        _git(self.directory, "commit", "--quiet", "-m", f"add {relative_path}")

    def test_clean_tree_exits_zero(self):
        self._commit(
            "docs/example.md",
            "\n".join([
                "# 見出し",
                "",
                "| A | B |",
                "| --- | --- |",
                r"| a\|b | c |",
                "| x | y",
                "",
            ]),
        )
        result = _run(self.directory)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("MISMATCHES=0", result.stdout)
        self.assertIn("TABLES=1", result.stdout)
        self.assertIn("ROWS=2", result.stdout)

    def test_broken_table_exits_nonzero_and_reports_location(self):
        self._commit(
            "docs/broken.md",
            "\n".join([
                "| A | B | C |",
                "| --- | --- | --- |",
                "| 1 | 2 |",
                "",
            ]),
        )
        result = _run(self.directory)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("docs/broken.md:3", result.stderr)
        self.assertIn("header L1=3列", result.stderr)
        self.assertIn("この行=2列", result.stderr)

    def test_untracked_file_is_not_scanned(self):
        """`git add`していないfileは検査対象に入らない。"""
        self._commit("docs/tracked.md", "テキストのみ\n")
        untracked = Path(self.directory) / "docs" / "untracked.md"
        untracked.write_text("| A | B |\n| --- |\n| 1 |\n", encoding="utf-8")
        result = _run(self.directory)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_no_tracked_markdown_fails_closed(self):
        _git(self.directory, "commit", "--quiet", "--allow-empty", "-m", "empty")
        result = _run(self.directory)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not working", result.stderr)


if __name__ == "__main__":
    unittest.main()
