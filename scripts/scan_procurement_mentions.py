#!/usr/bin/env python3
"""GitHubのIssue/Pull Request本文・commentから、調達状態を示す語を走査する。

`docs/hardware/tbd-register.md`のcommand 3)群A（調達状態）と同じ語を使う。
正本は`docs/hardware/hardware-bom.md`の発注前の走査であり、ここでは再定義しない。
語の一覧は**tbd-register.md自身の表から読み取る**（ハードコードしない）。
同文書は「同じ語の一覧を2箇所に書くと、片方だけを広げた状態で『走査した』と
言えてしまう」と明記しており、ここへ複製するとまさにその形になる。

command 3)のgrepは`docs/hardware docs/decisions docs/toolchains`配下のtracked
Markdownしか見ない。GitHubのIssue本文・Pull Request本文・commentはどの引数にも
入らず、台帳への登録漏れを生む経路になっている（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。

**この検査は報告のみで、何も止めない。**「古い」と「不要」は語だけでは
区別できず、日付を打った過去の事実として残す記述を誤って問題として報告し
うる。ヒットの分類（既存台帳行が包含する／新規登録する／台帳の対象外／
規則文等）は、`tbd-register.md`の[区別の基準](../docs/hardware/tbd-register.md#区別の基準)
に従って人が行う。設計は`scripts/hooks/merge_trailer_report.py`に倣う。

GitHub APIのrate limitは未測定であり、走査頻度に影響しうる（#305）。
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import publish_guards as guards  # noqa: E402

TBD_REGISTER = "docs/hardware/tbd-register.md"
_PROCUREMENT_ROW_RE = re.compile(r"^\|\s*A:\s*調達状態\s*\|(?P<words>.+?)\|", re.MULTILINE)
_BACKTICK_WORD_RE = re.compile(r"`([^`]+)`")

TIMEOUT = 30


def load_procurement_words(root):
    """`tbd-register.md`の群A行から、調達状態を示す語の一覧を読み取る。"""
    path = Path(root) / TBD_REGISTER
    if not path.is_file():
        raise guards.ValidationError(f"{TBD_REGISTER} が見つからない。")
    text = guards.get_file_text(str(path))
    match = _PROCUREMENT_ROW_RE.search(text)
    if not match:
        raise guards.ValidationError(
            f"{TBD_REGISTER} の群A（調達状態）行を読めなかった。"
            " 表の見出しや列の書式が変わっている可能性がある。"
        )
    words = _BACKTICK_WORD_RE.findall(match.group("words"))
    if not words:
        raise guards.ValidationError(
            f"{TBD_REGISTER} の群A行からbacktick語を1つも抽出できなかった。"
        )
    return words


def build_pattern(words):
    return re.compile("|".join(re.escape(word) for word in words))


def _run_gh(arguments, input_text=None):
    result = subprocess.run(
        ["gh", *arguments],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        input=input_text,
        timeout=TIMEOUT,
    )
    if result.returncode != 0:
        raise guards.ValidationError(
            f"gh {' '.join(arguments)} failed: {result.stdout}{result.stderr}"
        )
    return result.stdout


_QUERY = """
query($owner: String!, $repo: String!, $states: [{state_enum}!], $cursor: String) {{
  repository(owner: $owner, name: $repo) {{
    {field}(states: $states, first: 50, after: $cursor, orderBy: {{field: UPDATED_AT, direction: DESC}}) {{
      pageInfo {{ hasNextPage endCursor }}
      nodes {{
        number
        url
        body
        comments(first: 100) {{
          pageInfo {{ hasNextPage endCursor }}
          nodes {{ url body }}
        }}
      }}
    }}
  }}
}}
"""


# 1件のIssue／Pull Requestのcommentの続きを取るquery。**`_QUERY`のcomments connection
# は1ページしか返さない。**comment数が`first`を超えるnodeでは、超過分の調達状態の記述を
# 見落とす。**この scriptは「台帳への登録漏れを探す」ための道具であり、取りこぼすと
# 「無い」と読むことになる。**報告のみで止めない設計とは別の問題である。
#
# `{field}`は単数形（`issue`／`pullRequest`）で埋める。**複数形から文字列操作で導かない。**
# 導くと、connection名の変更で静かにずれる。
_COMMENTS_QUERY = """
query($owner: String!, $repo: String!, $number: Int!, $cursor: String) {{
  repository(owner: $owner, name: $repo) {{
    {field}(number: $number) {{
      comments(first: 100, after: $cursor) {{
        pageInfo {{ hasNextPage endCursor }}
        nodes {{ url body }}
      }}
    }}
  }}
}}
"""


def _fetch_remaining_comments(owner, repo, singular, number, cursor):
    """`cursor`より後のcommentを全ページ取得して返す。"""
    query = _COMMENTS_QUERY.format(field=singular)
    nodes = []
    while cursor:
        arguments = [
            "api", "graphql",
            "-f", f"query={query}",
            "-f", f"owner={owner}",
            "-f", f"repo={repo}",
            # `number`はInt!である。`-f`は文字列として渡すため型が合わない。
            "-F", f"number={number}",
            "-f", f"cursor={cursor}",
        ]
        output = _run_gh(arguments)
        data = json.loads(output)["data"]["repository"][singular]["comments"]
        nodes.extend(data["nodes"])
        page = data["pageInfo"]
        cursor = page["endCursor"] if page["hasNextPage"] else None
    return nodes


def _complete_comments(owner, repo, singular, node):
    """nodeのcommentが途中で切れていれば、続きを足して完全にする。

    **`pageInfo`が無い形は1ページで収まった応答として扱う。**`_QUERY`が`pageInfo`を
    要求しているため実際の応答には必ず含まれるが、mockした単一ページのfixtureを
    壊さないために欠落を許す。**続きがある形の検査はtestが持つ。**
    """
    comments = node.get("comments") or {}
    page = comments.get("pageInfo") or {}
    if not page.get("hasNextPage"):
        return
    cursor = page.get("endCursor")
    if not cursor:
        # **続きがあると言いながらcursorが無い応答は、黙って捨てない。**
        # 捨てるとこのscriptが直そうとしている取りこぼしそのものになる。
        raise guards.ValidationError(
            f"{singular} #{node['number']}: comments has hasNextPage but no endCursor"
        )
    comments["nodes"] = (comments.get("nodes") or []) + _fetch_remaining_comments(
        owner, repo, singular, node["number"], cursor
    )


def _fetch_kind(owner, repo, field, singular, state_enum, states):
    """`issues`または`pullRequests`connectionを全ページ取得する。

    **commentも全ページ取る**（`_complete_comments`）。
    """
    query = _QUERY.format(field=field, state_enum=state_enum)
    cursor = None
    nodes = []
    while True:
        arguments = [
            "api", "graphql",
            "-f", f"query={query}",
            "-f", f"owner={owner}",
            "-f", f"repo={repo}",
        ]
        for state in states:
            arguments += ["-f", f"states[]={state}"]
        if cursor:
            arguments += ["-f", f"cursor={cursor}"]
        output = _run_gh(arguments)
        data = json.loads(output)["data"]["repository"][field]
        for node in data["nodes"]:
            _complete_comments(owner, repo, singular, node)
        nodes.extend(data["nodes"])
        if not data["pageInfo"]["hasNextPage"]:
            break
        cursor = data["pageInfo"]["endCursor"]
    return nodes


def scan_kind(owner, repo, kind, field, singular, state_enum, states, pattern):
    """1種類（Issue／Pull Request）分の`[(kind, number, location, url, matched_words)]`を返す。"""
    findings = []
    for node in _fetch_kind(owner, repo, field, singular, state_enum, states):
        body = node.get("body") or ""
        matches = sorted(set(pattern.findall(body)))
        if matches:
            findings.append((kind, node["number"], "本文", node["url"], matches))
        for comment in node.get("comments", {}).get("nodes", []):
            comment_body = comment.get("body") or ""
            matches = sorted(set(pattern.findall(comment_body)))
            if matches:
                findings.append(
                    (kind, node["number"], "comment", comment["url"], matches)
                )
    return findings


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", default="")
    parser.add_argument("--owner", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument(
        "--state", choices=("open", "closed", "all"), default="open",
        help="走査対象のIssue/Pull Requestの状態（既定: open）",
    )
    arguments = parser.parse_args(argv)

    repository_root = arguments.repository_root
    if not repository_root.strip():
        repository_root = str(Path(__file__).resolve().parent.parent)
    root = guards.full_path(repository_root)

    words = load_procurement_words(root)
    pattern = build_pattern(words)

    if arguments.state == "open":
        issue_states, pr_states = ["OPEN"], ["OPEN"]
    elif arguments.state == "closed":
        issue_states, pr_states = ["CLOSED"], ["CLOSED", "MERGED"]
    else:
        issue_states, pr_states = ["OPEN", "CLOSED"], ["OPEN", "CLOSED", "MERGED"]

    print(
        "**これは報告のみである。ヒットを検出しても、このscriptはmergeを"
        "止めない。分類（既存台帳行が包含する／新規登録する／対象外）は"
        f"tbd-register.mdの区別の基準に従って人が行う。**走査語: {', '.join(words)}"
    )

    findings = []
    for kind, field, singular, state_enum, states in (
        ("Issue", "issues", "issue", "IssueState", issue_states),
        ("Pull Request", "pullRequests", "pullRequest", "PullRequestState", pr_states),
    ):
        findings.extend(
            scan_kind(
                arguments.owner, arguments.repo, kind, field, singular,
                state_enum, states, pattern,
            )
        )

    if not findings:
        print(f"PROCUREMENT_MENTIONS=0 STATE={arguments.state}")
        return 0

    for kind, number, location, url, matches in findings:
        print(f"{kind} #{number} [{location}] {', '.join(matches)} — {url}")
    print(f"PROCUREMENT_MENTIONS={len(findings)} STATE={arguments.state}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except guards.ValidationError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
