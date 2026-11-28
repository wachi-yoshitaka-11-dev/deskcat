#!/usr/bin/env python3
"""レビューの記録の範囲が、PR の base から head まで途切れずにつながっているかを判定する。

#814 の作業リストの9。`scripts/hooks/review_record.py`（手元の log を書く hook）、
`scripts/hooks/gh_metadata_guard.py`（`gh pr merge` の前の検査）、
`.github/workflows/review-coverage.yml`（CI）が、この module の判定を共有する。

**証拠に数えるのは、background の subagent（fork）の中で、範囲 `A..B` を明示して
走らせた `/code-review` だけである。**main の文脈での実行と、ユーザーが打った
`/code-review` は、hook に `agent_id` も結果も届かないので数えない（2026-11-28 の実験）。

範囲のつながりの判定（`uncovered`）：
- 始まりは PR の base と head の merge-base である。
- 範囲 `A..B` は、A が今の到達点の祖先（同じでもよい）で、今の到達点が B の祖先のとき、
  到達点を B まで進める。B が head の祖先でない範囲（rebase の前の範囲など）は使わない。
- 到達点が head に届けば、つながっている。

**限界：**
- hook は、fork が範囲の全部を実際に読んだかまでは確かめない。
- 所有者が画面から merge する経路は、手元の hook を通らない。その経路では CI だけが見るので、
  PR の本文に書いた範囲を信じるしかない。
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

LOG_NAME = "deskcat-review-log.jsonl"

# PR の本文で範囲を探す節。template（`.github/pull_request_template.md`）の見出しと同じ。
BODY_SECTION = "自己レビュー"

# 本文の範囲は backtick で囲んだ `<hex>..<hex>` だけを読む。`...`（3点）は範囲として読まない。
BODY_RANGE_RE = re.compile(r"`([0-9a-fA-F]{7,40})\.\.([0-9a-fA-F]{7,40})`")

# `/code-review` の引数の範囲。字句は git の revision の書き方に限る。
ARG_RANGE_RE = re.compile(r"^([^.\s]+(?:\.[^.\s]+)*)\.\.([^.\s]+(?:\.[^.\s]+)*)$")

SHA_RE = re.compile(r"^[0-9a-f]{40}$")


def _git(args, cwd=None):
    result = subprocess.run(
        ["git", *args], cwd=cwd, capture_output=True, text=True, encoding="utf-8",
        timeout=30,
    )
    return result.returncode, result.stdout.strip()


def resolve(rev, cwd=None):
    """revision を 40 桁の commit の SHA に直す。直せなければ `None`。"""
    if rev.startswith("-"):
        return None
    code, out = _git(["rev-parse", "--verify", "--quiet", f"{rev}^{{commit}}"], cwd)
    return out if code == 0 and SHA_RE.match(out) else None


def is_ancestor(a, b, cwd=None):
    """a が b の祖先（同じ commit を含む）か。"""
    code, _ = _git(["merge-base", "--is-ancestor", a, b], cwd)
    return code == 0


def merge_base(a, b, cwd=None):
    code, out = _git(["merge-base", a, b], cwd)
    return out if code == 0 and SHA_RE.match(out) else None


def commits_between(base, head, cwd=None):
    """`base..head` の commit の SHA の一覧。求められなければ例外を上げる。"""
    code, out = _git(["rev-list", f"{base}..{head}"], cwd)
    if code != 0:
        raise ValueError(f"{base[:7]}..{head[:7]} の commit を列挙できない")
    return [line for line in out.splitlines() if SHA_RE.match(line)]


def covers_commit(sha, ranges, cwd=None, ancestor=None):
    """完了した記録の範囲のどれかが、commit `sha` 1つの変更（`sha^..sha`）を含むか。"""
    ancestor = ancestor or (lambda a, b: is_ancestor(a, b, cwd))
    code, parent = _git(["rev-parse", "--verify", "--quiet", f"{sha}^"], cwd)
    if code != 0 or not SHA_RE.match(parent):
        return False
    return any(ancestor(a, parent) and ancestor(sha, b) for a, b in ranges)


def log_path(cwd=None):
    """手元の log の path。worktree でも同じ場所になるよう、git の共通の dir に置く。"""
    code, out = _git(["rev-parse", "--path-format=absolute", "--git-common-dir"], cwd)
    if code != 0 or not out:
        return None
    return Path(out) / LOG_NAME


def parse_arg_range(args):
    """`/code-review` の引数から、範囲の字句（A, B）を1つだけ取る。

    範囲の字句が無い、2つ以上ある、`...`（3点）である、のどれでも `None` を返す。
    """
    found = []
    for token in (args or "").split():
        if ".." not in token:
            continue
        if "..." in token:
            return None
        match = ARG_RANGE_RE.match(token)
        if not match:
            return None
        found.append((match.group(1), match.group(2)))
    return found[0] if len(found) == 1 else None


def read_log(path):
    """log の行を読む。読めない行は飛ばす（壊れた行を証拠に数えない）。"""
    records = []
    try:
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(record, dict):
                    records.append(record)
    except FileNotFoundError:
        return []
    return records


def completed_ranges(records):
    """完了した記録の範囲 `(from, to)` の一覧。前と後の記録を `tool_use_id` で組にする。

    後の記録の `success` が真のものだけを数える。前の記録だけ（途中で止まった実行）は数えない。
    """
    started = {}
    done = []
    for record in records:
        key = record.get("tool_use_id")
        if not isinstance(key, str) or not key:
            continue
        if record.get("event") == "started":
            start, end = record.get("from"), record.get("to")
            if isinstance(start, str) and isinstance(end, str) \
                    and SHA_RE.match(start) and SHA_RE.match(end):
                started[key] = (start, end)
        elif record.get("event") == "completed" and record.get("success") is True:
            if key in started:
                done.append(started[key])
    return done


def uncovered(base_sha, head_sha, ranges, ancestor=is_ancestor):
    """merge-base（base_sha）から head_sha までつながらなければ、届いた先の SHA を返す。

    つながっていれば `None` を返す。`ancestor(a, b)` は a が b の祖先（同じを含む）か。
    """
    cache = {}

    def known(a, b):
        # 同じ組の git の呼び出しを繰り返さない（log は消さないので、範囲は増え続ける）。
        if (a, b) not in cache:
            cache[(a, b)] = ancestor(a, b)
        return cache[(a, b)]

    usable = [(a, b) for a, b in sorted(set(ranges)) if known(b, head_sha)]
    # 届く点を幅で探す。1つの道だけを選ぶと、merge commit のある履歴で、別の道でつながるのに
    # 行き止まりを報告する。
    # 抜けの始まりとして報告する点（best）は、届いた点を足すたびに、今の best の子孫なら置き換える。
    # 届いた点の全部の組を比べない（組の数だけ git を呼ぶと、hook の時間切れを超えうる）。
    reached, frontier, best = {base_sha}, [base_sha], base_sha
    while frontier:
        current = frontier.pop(0)
        if current == head_sha:
            return None
        for a, b in usable:
            if b not in reached and known(a, current) and known(current, b):
                reached.add(b)
                frontier.append(b)
                if known(best, b):
                    best = b
    return best


def body_section(body):
    """PR の本文の「自己レビュー」の節の文字列。無ければ空文字列。"""
    lines = (body or "").splitlines()
    out, inside = [], False
    for line in lines:
        if line.startswith("## "):
            inside = line[3:].strip() == BODY_SECTION
            continue
        if inside:
            out.append(line)
    return "\n".join(out)


def body_ranges(body):
    """本文の「自己レビュー」の節に書いた範囲の字句 `(A, B)` の一覧。"""
    return BODY_RANGE_RE.findall(body_section(body))


def resolve_ranges(pairs, cwd=None):
    """字句の範囲を 40 桁に直す。直せないものは `unresolved` に入れて返す。"""
    resolved, unresolved = [], []
    for a, b in pairs:
        full_a, full_b = resolve(a, cwd), resolve(b, cwd)
        if full_a and full_b:
            resolved.append((full_a, full_b))
        else:
            unresolved.append(f"{a}..{b}")
    return resolved, unresolved


def _main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--base", required=True, help="PR の base の SHA")
    parser.add_argument("--head", required=True, help="PR の head の SHA")
    parser.add_argument("--pr-body-env", required=True,
                        help="PR の本文を入れた環境変数の名前")
    options = parser.parse_args()
    body = os.environ.get(options.pr_body_env, "")
    base, head = resolve(options.base), resolve(options.head)
    if not base or not head:
        print("::error::PR の base か head の commit を読めない。", file=sys.stderr)
        return 1
    start = merge_base(base, head)
    if not start:
        print("::error::PR の base と head の merge-base を求められない。", file=sys.stderr)
        return 1
    pairs = body_ranges(body)
    ranges, unresolved = resolve_ranges(pairs)
    if unresolved:
        print("::error::本文の「自己レビュー」の範囲のうち、commit を読めないものがある: "
              + ", ".join(unresolved), file=sys.stderr)
        return 1
    reached = uncovered(start, head, ranges)
    if reached:
        print(
            f"::error::本文の「自己レビュー」に書いたレビューの範囲が、merge-base（{start[:7]}）"
            f"から head（{head[:7]}）までつながっていない。{reached[:7]} から先が抜けている。"
            " 抜けた範囲を background の subagent の `/code-review` で見て、範囲を本文に書く"
            "（CONTRIBUTING の「自己レビュー」）。",
            file=sys.stderr,
        )
        return 1
    print(f"レビューの範囲は {start[:7]}..{head[:7]} をつないでいる（{len(ranges)} 本）。")
    return 0


if __name__ == "__main__":
    sys.exit(_main())
