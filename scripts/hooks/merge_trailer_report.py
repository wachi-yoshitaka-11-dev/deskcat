#!/usr/bin/env python3
"""mergeしたsquash commitへtrailerが実際に入ったかを、merge直後に確認する。

Claude CodeのPostToolUse hookとして、`gh pr merge`を含むBash呼び出しの後に走る。

**PR側のgateがSUCCESSでも、squash commitへtrailerが引き継がれないことがある。**
`18298ae`（PR #191）と`619c843`（PR #196）は、head commitが`Change-Class`と3値の
`Self-Review`を持ち`Verify change class and self-review`もsuccessだったが、
`gh pr merge --squash`をmessage指定なしで実行したため、squash commitへ入らなかった。
どちらも後から付けられず、`DECLARATION_EXEMPT`へ登録して免除するしかなくなった。

**この検査は事後である。**入っていなければもう直せない。それでも報告するのは、
免除へ回す判断を、次の`main`昇格まで先延ばしにしないためである。

出力は`additionalContext`だけで、判定を止めない。**確認できなかった場合は
「入っている」と書かない。**確認できなかったことを、そのまま書く。
"""

import json
import subprocess
import sys
from pathlib import Path

import command_line

# trailerの名前は`scripts/review_gate.py`が正本である。**hook側へ複製しない。**
# 名前がずれると、gateが要求するものとhookが見るものが食い違い、hookが素通りする。
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import review_gate  # noqa: E402

# squash commitが持つべきtrailerの名前。値は見ない。
REQUIRED = (review_gate.TRAILER_CLASS, review_gate.TRAILER_REVIEW)

TIMEOUT = 30


def _report(text):
    json.dump(
        {
            "hookSpecificOutput": {
                "hookEventName": "PostToolUse",
                "additionalContext": text,
            }
        },
        sys.stdout,
    )
    sys.stdout.write("\n")
    raise SystemExit(0)


def _run(args):
    """外部commandを実行し`(ok, stdout)`を返す。失敗を例外にしない。"""
    try:
        result = subprocess.run(
            args, capture_output=True, text=True, timeout=TIMEOUT, check=False
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return False, str(error)
    if result.returncode != 0:
        return False, (result.stderr or result.stdout).strip()
    return True, result.stdout


def _pr_merge(command):
    """`gh pr merge`の呼び出しを探し`(見つかったか, 番号)`を返す。

    番号を省略した場合、`gh`は現在のbranchのPull Requestを対象にする。
    `gh pr view`も同じ既定に従うため、番号は`None`のままでよい。
    """
    for args in command_line.invocations(command, "gh"):
        if args[:2] != ["pr", "merge"]:
            continue
        for candidate in args[2:]:
            if candidate.startswith("-"):
                continue
            if candidate.isdigit():
                return True, candidate
            break
        return True, None
    return False, None


def main():
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError):
        return 0
    command = command_line.command_from(payload)
    if command is None or "gh" not in command:
        return 0
    found, number = _pr_merge(command)
    if not found:
        # `gh pr merge`以外では何もしない。`if`条件で絞らないのは、複合commandを
        # 取りこぼさないためである（理由はgh_metadata_guardと同じ）。
        return 0

    view = ["gh", "pr", "view", "--json", "number,state,baseRefName,mergeCommit"]
    if number is not None:
        view.insert(3, number)
    ok, output = _run(view)
    if not ok:
        _report(f"merge後のtrailer確認: `gh pr view`が失敗した。未確認である。{output}")
    try:
        data = json.loads(output)
    except json.JSONDecodeError:
        _report("merge後のtrailer確認: `gh pr view`の出力を解釈できなかった。未確認である。")

    label = f"PR #{data.get('number')}"
    if data.get("state") != "MERGED":
        # merge前の`gh pr`呼び出しでもこのhookは走る。まだmergeされていないだけである。
        return 0
    sha = (data.get("mergeCommit") or {}).get("oid")
    if not sha:
        _report(f"merge後のtrailer確認: {label}はMERGEDだがmerge commitのSHAが取れない。未確認である。")

    ok, output = _run(["git", "fetch", "origin"])
    if not ok:
        _report(f"merge後のtrailer確認: {label}のmerge commit `{sha[:7]}`をfetchできなかった。未確認である。{output}")
    ok, message = _run(["git", "log", "-1", "--format=%B", sha])
    if not ok:
        _report(f"merge後のtrailer確認: merge commit `{sha[:7]}`を読めなかった。未確認である。{message}")
    # `interpret-trailers`はstdinから読む。`_run`はstdinを渡せないため個別に呼ぶ。
    try:
        result = subprocess.run(
            ["git", "interpret-trailers", "--parse"],
            input=message,
            capture_output=True,
            text=True,
            timeout=TIMEOUT,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        _report(f"merge後のtrailer確認: `git interpret-trailers`が失敗した。未確認である。{error}")
    parsed = result.stdout

    missing = [name for name in REQUIRED if f"{name}:" not in parsed]
    base = data.get("baseRefName")
    if missing:
        _report(
            f"**merge後のtrailer確認: {label}のsquash commit `{sha[:7]}`に"
            f" {', '.join(missing)} が無い。**"
            " 履歴書き換えを行わないため、後から付けられない。"
            f" base`{base}`から`main`へ昇格するとき`review_gate.py history`が落ちる。"
            " `DECLARATION_EXEMPT`へ登録するか、判断を人間へ渡す。"
            f"\n\n実測した trailer:\n{parsed.strip() or '(なし)'}"
        )
    _report(
        f"merge後のtrailer確認: {label}のsquash commit `{sha[:7]}`（base`{base}`）は"
        f" {', '.join(REQUIRED)} を持つ。\n\n実測した trailer:\n{parsed.strip()}"
    )


if __name__ == "__main__":
    sys.exit(main())
