#!/usr/bin/env python3
"""昇格範囲の各commitに含まれるclosing keywordを検出し、報告する。

base が`main`のPull Requestでは、範囲内commit（squash mergeで`develop`へ入った
それぞれの個別commit）の`Closes #N`等が**merge時に発火する**。GitHubの
`closingIssuesReferences`はPull Request**本文**からしか計算されないため、範囲内の
個々のcommit messageに書かれたclosing keywordは、本文を見るだけでは検出できない
（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。

実例: 2026-09-01の[#300](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/300)で
範囲内commitに`Closes #294`／`Closes #298`／`Closes #296`があり、3件とも対象Issueが
`CLOSED`だったため無害だった。`main`昇格ではこの型が繰り返しうる。

**このscriptは報告のみで、何も止めない。**検出しても消せない
（共有branchの履歴を書き換えない）。できるのは、発火を知ってboardを
先に手当てすることだけである。設計は`scripts/hooks/merge_trailer_report.py`に倣う。

参照先Issueの状態（OPEN／CLOSED）は`gh`が使えるときだけ確認する。`gh`が
無い・認証が無い環境では確認できないと明記し、「CLOSEDである」と決め付けない。
"""

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import publish_guards as guards  # noqa: E402

# GitHubが認識するclosing keyword（同一repository内の`#番号`形式のみを対象とする。
# `owner/repo#番号`のcross-repository参照はこのrepositoryの慣行では使われていない）。
_KEYWORD_RE = re.compile(
    r"\b(close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s*:?\s*#(\d+)\b",
    re.IGNORECASE,
)


def find_closing_keywords(message):
    """commit message中のclosing keywordを`(keyword, issue番号)`のtupleで返す。"""
    return [
        (match.group(1), match.group(2))
        for match in _KEYWORD_RE.finditer(message)
    ]


def _commits_in_range(root, base, head):
    result = subprocess.run(
        ["git", "-C", root, "rev-list", "--no-merges", f"{base}..{head}"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if result.returncode != 0:
        raise guards.ValidationError(
            f"git rev-list {base}..{head} failed: {result.stdout}{result.stderr}"
        )
    return [line for line in result.stdout.splitlines() if line]


def _commit_message(root, commit):
    result = subprocess.run(
        ["git", "-C", root, "log", "-1", "--format=%B", commit],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if result.returncode != 0:
        raise guards.ValidationError(
            f"git log -1 --format=%B {commit} failed: {result.stdout}{result.stderr}"
        )
    return result.stdout


def _issue_state(number, root):
    """Issue番号の状態を調べる。確認できなければNoneを返す（「CLOSEDである」と決め付けない）。

    **`root`を作業ディレクトリとして渡す。**`gh`は対象repositoryをcwdのgit remoteから
    推定するため、渡さないとlauncherのcwdの側を見る。`--repository-root`へ別のpathを
    渡す運用は実在し（`git archive`で展開したscriptsから実repositoryを測る形。
    `AGENTS.md`の検証手順にある）、**commitを読むrepositoryとIssueを問い合わせる
    repositoryが食い違う。**別repositoryの同番号を返すか、`未確認`になる。
    """
    if shutil.which("gh") is None:
        return None
    try:
        result = subprocess.run(
            ["gh", "issue", "view", number, "--json", "state", "-q", ".state"],
            cwd=root,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
    except (subprocess.TimeoutExpired, OSError):
        # **返らない・起動できないは「確認できなかった」である。**`None`へ落とす。
        # 捕まえないと、`未確認`を返す設計へ入らずtracebackになる。
        # **CLOSEDと決め付けないという既定を、失敗経路でも守る。**
        return None
    if result.returncode != 0:
        return None
    state = result.stdout.strip()
    return state or None


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", default="")
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument(
        "--check-issue-state",
        action="store_true",
        help="検出したIssue番号の現在の状態を`gh`で確認する（既定では確認しない）。",
    )
    arguments = parser.parse_args(argv)

    repository_root = arguments.repository_root
    if not repository_root.strip():
        repository_root = str(Path(__file__).resolve().parent.parent)
    root = guards.full_path(repository_root)

    if not Path(root).is_dir():
        raise guards.ValidationError("Repository root does not exist.")

    commits = _commits_in_range(root, arguments.base, arguments.head)

    findings = []
    for commit in commits:
        message = _commit_message(root, commit)
        for keyword, number in find_closing_keywords(message):
            state = (
                _issue_state(number, root) if arguments.check_issue_state else None
            )
            findings.append((commit[:7], keyword, number, state))

    print(
        "**これは報告のみである。closing keywordを検出しても、このscriptはmergeを"
        "止めない。共有branchの履歴も書き換えない。**"
    )
    if not findings:
        print(f"CLOSING_KEYWORDS=0 RANGE={arguments.base}..{arguments.head}")
        return 0

    for commit, keyword, number, state in findings:
        state_text = state if state is not None else "未確認"
        print(f"{commit}: `{keyword} #{number}` (Issue #{number} state={state_text})")

    open_count = sum(1 for _, _, _, state in findings if state == "OPEN")
    unknown_count = sum(1 for _, _, _, state in findings if state is None)
    print(
        f"CLOSING_KEYWORDS={len(findings)} OPEN={open_count} UNKNOWN={unknown_count}"
        f" RANGE={arguments.base}..{arguments.head}"
    )
    if open_count:
        print(
            "**OPENのIssueへのclosing keywordがある。**mergeが発火するとboardと"
            "二重に管理される可能性がある。board側を先に手当てすることを検討する。"
        )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except guards.ValidationError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
