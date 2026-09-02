#!/usr/bin/env python3
"""`report_promotion_closing_keywords.py`の回帰test。

昇格Pull Requestの範囲内commitに含まれるclosing keywordは、Pull Request本文の
`closingIssuesReferences`では検出できない（#289）。このscriptは**報告のみ**であり、
mergeを止めず履歴も書き換えない。単体testと、fixture repositoryへ実際にcommitして
子processで起動するtestの両方を持つ。
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

import report_promotion_closing_keywords as report  # noqa: E402

SCRIPT = str(SCRIPTS_ROOT / "report_promotion_closing_keywords.py")

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


class FindClosingKeywordsTests(unittest.TestCase):
    def test_detects_closes(self):
        self.assertEqual(
            report.find_closing_keywords("fix: something\n\nCloses #294"),
            [("Closes", "294")],
        )

    def test_case_insensitive_and_multiple_keywords(self):
        message = "closes #1\nFixes #2\nRESOLVED #3"
        found = report.find_closing_keywords(message)
        self.assertEqual([number for _, number in found], ["1", "2", "3"])

    def test_no_keyword_returns_empty(self):
        self.assertEqual(
            report.find_closing_keywords("Refs #6\n\nSee #10 for context"), []
        )

    def test_requires_word_boundary_before_keyword(self):
        """`disclose #1`のような語の一部は拾わない。"""
        self.assertEqual(report.find_closing_keywords("disclose #1"), [])


class CommandLineTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(self.directory))
        _git(self.directory, "init", "--quiet", ".")
        (Path(self.directory) / "a.md").write_text("first\n", encoding="utf-8")
        _git(self.directory, "add", "a.md")
        _git(self.directory, "commit", "--quiet", "-m", "first")
        self.base = _git(self.directory, "rev-parse", "HEAD").strip()

    def _commit(self, message):
        path = Path(self.directory) / "a.md"
        path.write_text(path.read_text(encoding="utf-8") + "line\n", encoding="utf-8")
        _git(self.directory, "commit", "--quiet", "-am", message)
        return _git(self.directory, "rev-parse", "HEAD").strip()

    def _run(self, extra_args=()):
        return subprocess.run(
            [sys.executable, SCRIPT, "--repository-root", self.directory,
             "--base", self.base, "--head", "HEAD", *extra_args],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            timeout=60,
        )

    def test_no_closing_keywords_reports_zero_and_exits_zero(self):
        self._commit("ordinary change\n\nRefs #1")
        result = self._run()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("CLOSING_KEYWORDS=0", result.stdout)

    def test_detects_and_never_fails(self):
        self._commit("fix: bug\n\nCloses #294")
        self._commit("fix: bug2\n\nFixes #298")
        result = self._run()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Closes #294", result.stdout)
        self.assertIn("Fixes #298", result.stdout)
        self.assertIn("CLOSING_KEYWORDS=2", result.stdout)
        self.assertIn("報告のみ", result.stdout)

    def test_state_unknown_without_check_issue_state_flag(self):
        self._commit("fix: bug\n\nCloses #294")
        result = self._run()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("state=未確認", result.stdout)
        self.assertIn("UNKNOWN=1", result.stdout)


if __name__ == "__main__":
    unittest.main()
