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


def _comments_page(nodes, has_next=False, end_cursor=None, field="issue"):
    """1件のIssue／Pull Requestのcomment 1ページ分の応答を作る。"""
    return json.dumps({
        "data": {"repository": {field: {"comments": {
            "pageInfo": {"hasNextPage": has_next, "endCursor": end_cursor},
            "nodes": nodes,
        }}}}
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
                "o", "r", "Issue", "issues", "issue", "IssueState", ["OPEN"], pattern
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
                "o", "r", "Issue", "issues", "issue", "IssueState", ["OPEN"], pattern
            )
        self.assertEqual(run.call_count, 2)
        self.assertEqual({f[1] for f in findings}, {1, 2})

    def test_query_requests_page_info_for_both_connections(self):
        """`_QUERY`が2つのconnectionの`pageInfo`を要求していること。

        **`_complete_comments`は`pageInfo`の欠落を「1ページで収まった」と読む。**
        queryから落とすと、取りこぼしが静かに戻る。**落ちたことをここで検出する。**
        2つとは、`issues`／`pullRequests`のconnectionと、その中の`comments`である。
        """
        formatted = scan._QUERY.format(field="issues", state_enum="IssueState")
        self.assertEqual(formatted.count("pageInfo"), 2, formatted)
        self.assertIn("pageInfo", scan._COMMENTS_QUERY)

    def test_paginates_comments_within_a_node(self):
        """**commentが1ページで収まらないnodeでも、超過分を走査する。**

        `_QUERY`のcomments connectionは1ページしか返さない。取りこぼすと、
        台帳に無い調達状態の記述を「無い」と読むことになる。
        **1ページ目に無く、2ページ目と3ページ目にある語を検出できること**を見る。
        """
        nodes = [{
            "number": 7,
            "url": "u7",
            "body": "関係ない文章",
            "comments": {
                "pageInfo": {"hasNextPage": True, "endCursor": "C1"},
                "nodes": [{"url": "u7#c1", "body": "関係ない"}],
            },
        }]
        pattern = scan.build_pattern(["未購入", "発注"])
        pages = [
            _page(nodes),
            _comments_page(
                [{"url": "u7#c2", "body": "未購入である"}],
                has_next=True, end_cursor="C2",
            ),
            _comments_page([{"url": "u7#c3", "body": "発注した"}]),
        ]
        with mock.patch.object(scan, "_run_gh", side_effect=pages) as run:
            findings = scan.scan_kind(
                "o", "r", "Issue", "issues", "issue", "IssueState", ["OPEN"], pattern
            )
        self.assertEqual(run.call_count, 3)
        self.assertEqual({f[3] for f in findings}, {"u7#c2", "u7#c3"})

    def test_does_not_ask_again_when_the_comment_page_is_complete(self):
        """`hasNextPage`が偽なら追加の問い合わせをしない。**空振りの1回を作らない。**"""
        nodes = [{
            "number": 8,
            "url": "u8",
            "body": "未購入",
            "comments": {
                "pageInfo": {"hasNextPage": False, "endCursor": None},
                "nodes": [],
            },
        }]
        pattern = scan.build_pattern(["未購入"])
        with mock.patch.object(scan, "_run_gh", return_value=_page(nodes)) as run:
            findings = scan.scan_kind(
                "o", "r", "Issue", "issues", "issue", "IssueState", ["OPEN"], pattern
            )
        self.assertEqual(run.call_count, 1)
        self.assertEqual({f[1] for f in findings}, {8})

    def test_comment_query_uses_the_singular_connection(self):
        """続きのcommentは単数形のconnectionへ問い合わせる。

        **複数形から文字列操作で導かない。**導くとconnection名の変更で静かにずれ、
        GraphQLのerrorとして実行時にしか出ない。
        """
        nodes = [{
            "number": 9,
            "url": "u9",
            "body": "",
            "comments": {
                "pageInfo": {"hasNextPage": True, "endCursor": "C1"},
                "nodes": [],
            },
        }]
        pattern = scan.build_pattern(["未購入"])
        pages = [
            _page(nodes, field="pullRequests"),
            _comments_page([], field="pullRequest"),
        ]
        with mock.patch.object(scan, "_run_gh", side_effect=pages) as run:
            scan.scan_kind(
                "o", "r", "Pull Request", "pullRequests", "pullRequest",
                "PullRequestState", ["OPEN"], pattern,
            )
        sent = " ".join(run.call_args_list[1].args[0])
        self.assertIn("pullRequest(number: $number)", sent)
        self.assertNotIn("pullRequests(number: $number)", sent)
        self.assertIn("number=9", sent)

    def test_missing_cursor_with_more_pages_is_an_error(self):
        """`hasNextPage`が真で`endCursor`が無い応答を、黙って捨てない。

        **捨てると、このscriptが直そうとしている取りこぼしそのものになる。**
        """
        nodes = [{
            "number": 10,
            "url": "u10",
            "body": "",
            "comments": {
                "pageInfo": {"hasNextPage": True, "endCursor": None},
                "nodes": [],
            },
        }]
        pattern = scan.build_pattern(["未購入"])
        with mock.patch.object(scan, "_run_gh", return_value=_page(nodes)):
            with self.assertRaises(guards.ValidationError):
                scan.scan_kind(
                    "o", "r", "Issue", "issues", "issue", "IssueState",
                    ["OPEN"], pattern,
                )

    def test_no_hits_returns_empty(self):
        nodes = [{"number": 1, "url": "u1", "body": "関係ない", "comments": {"nodes": []}}]
        pattern = scan.build_pattern(["未購入"])
        with mock.patch.object(scan, "_run_gh", return_value=_page(nodes)):
            findings = scan.scan_kind(
                "o", "r", "Issue", "issues", "issue", "IssueState", ["OPEN"], pattern
            )
        self.assertEqual(findings, [])


class MainWiringTests(unittest.TestCase):
    """`main`が`scan_kind`へ渡す組のtest。

    **単数形は続きのcommentのqueryへ埋まる。**間違えるとGraphQLのerrorとして実行時に
    しか出ない。`scan_kind`側のtestは単数形を引数で受け取るため、**`main`の配線は
    そこでは固定されない。**ここで固定する。
    """

    def test_main_passes_the_matching_singular_connection_name(self):
        with mock.patch.object(scan, "scan_kind", return_value=[]) as scan_kind:
            code = scan.main(["--owner", "o", "--repo", "r"])
        self.assertEqual(code, 0)
        pairs = [(call.args[3], call.args[4]) for call in scan_kind.call_args_list]
        self.assertEqual(pairs, [("issues", "issue"), ("pullRequests", "pullRequest")])


if __name__ == "__main__":
    unittest.main()
