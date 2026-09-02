#!/usr/bin/env python3
"""`scan_procurement_mentions.py`の回帰test。

`gh`への実際のnetwork呼び出しは行わない。`_run_gh`をmockして、
GraphQLが返すページ構造・comment・複数語ヒットを検証する。語の一覧の
抽出（`load_procurement_words`）は実repositoryの`tbd-register.md`に対して行い、
tracked fileの内容と乖離しないことを確認する。
"""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT / "lib"))

import publish_guards as guards  # noqa: E402

sys.path.insert(0, str(SCRIPTS_ROOT))

import scan_procurement_mentions as scan  # noqa: E402

REPOSITORY_ROOT = str(SCRIPTS_ROOT.parent)


class LoadProcurementWordsTests(unittest.TestCase):
    def test_reads_group_a_from_real_repository(self):
        """実repositoryの群A行から、既知の語がすべて読める。"""
        words = scan.load_procurement_words(REPOSITORY_ROOT)
        for expected in ("未購入", "購入", "発注", "未選定", "未確定",
                          "Required", "Blocked", "手配", "調達"):
            self.assertIn(expected, words)

    def test_missing_file_raises(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(guards.ValidationError):
                scan.load_procurement_words(directory)

    def test_missing_row_raises(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / scan.TBD_REGISTER
            path.parent.mkdir(parents=True)
            path.write_text("# 台帳\n\n本文だけで表が無い。\n", encoding="utf-8")
            with self.assertRaises(guards.ValidationError):
                scan.load_procurement_words(directory)


class BuildPatternTests(unittest.TestCase):
    def test_matches_any_configured_word(self):
        pattern = scan.build_pattern(["未購入", "Blocked"])
        self.assertEqual(pattern.findall("これはBlockedであり未購入でもある"), ["Blocked", "未購入"])

    def test_does_not_match_unrelated_text(self):
        pattern = scan.build_pattern(["未購入", "Blocked"])
        self.assertEqual(pattern.findall("これは関係ない文章"), [])


def _page(nodes, has_next=False, end_cursor=None, field="issues"):
    return json.dumps({
        "data": {"repository": {field: {
            "pageInfo": {"hasNextPage": has_next, "endCursor": end_cursor},
            "nodes": nodes,
        }}}
    })


class ScanKindTests(unittest.TestCase):
    def test_finds_hits_in_body_and_comments(self):
        nodes = [
            {
                "number": 1,
                "url": "https://example.invalid/issues/1",
                "body": "この部品は未購入である",
                "comments": {"nodes": [
                    {"url": "https://example.invalid/issues/1#c1", "body": "発注した"},
                ]},
            },
            {
                "number": 2,
                "url": "https://example.invalid/issues/2",
                "body": "関係ない文章",
                "comments": {"nodes": []},
            },
        ]
        pattern = scan.build_pattern(["未購入", "発注"])
        with mock.patch.object(scan, "_run_gh", return_value=_page(nodes)):
            findings = scan.scan_kind(
                "o", "r", "Issue", "issues", "IssueState", ["OPEN"], pattern
            )
        self.assertEqual(len(findings), 2)
        kinds_locations = [(f[0], f[1], f[2]) for f in findings]
        self.assertIn(("Issue", 1, "本文"), kinds_locations)
        self.assertIn(("Issue", 1, "comment"), kinds_locations)
        self.assertNotIn(("Issue", 2, "本文"), kinds_locations)

    def test_follows_pagination(self):
        page1 = _page(
            [{"number": 1, "url": "u1", "body": "未購入", "comments": {"nodes": []}}],
            has_next=True, end_cursor="CURSOR1",
        )
        page2 = _page(
            [{"number": 2, "url": "u2", "body": "未購入", "comments": {"nodes": []}}],
            has_next=False,
        )
        pattern = scan.build_pattern(["未購入"])
        with mock.patch.object(scan, "_run_gh", side_effect=[page1, page2]) as run:
            findings = scan.scan_kind(
                "o", "r", "Issue", "issues", "IssueState", ["OPEN"], pattern
            )
        self.assertEqual(run.call_count, 2)
        self.assertEqual({f[1] for f in findings}, {1, 2})

    def test_no_hits_returns_empty(self):
        nodes = [{"number": 1, "url": "u1", "body": "関係ない", "comments": {"nodes": []}}]
        pattern = scan.build_pattern(["未購入"])
        with mock.patch.object(scan, "_run_gh", return_value=_page(nodes)):
            findings = scan.scan_kind(
                "o", "r", "Issue", "issues", "IssueState", ["OPEN"], pattern
            )
        self.assertEqual(findings, [])


if __name__ == "__main__":
    unittest.main()
