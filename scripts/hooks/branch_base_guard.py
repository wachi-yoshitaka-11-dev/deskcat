#!/usr/bin/env python3
"""branchを作る操作の基点が、最新の`origin/develop`かを検査する。

Claude CodeのPreToolUse hookとして、`git checkout -b`／`git switch -c`の前に走る。

**基点のずれは、単に古いだけでなく事実誤認の断定を生む。**古い基点では、後から入った
文書を「存在しない」と読む。そこから「捏造されている」という結論へ進むと、
誤りがPull Request本文として公開される。**遅れているかどうかは、遅れている側からは
見えない。**だからbranchを作る時点で機械が見る。

このhookを入れたsession自身、開始時のHEADが`origin/develop`から4 commit遅れていた。

**この検査は`git fetch`を伴う。**fetchしなければ、localの`origin/develop`自体が
古いままで一致してしまい、**本当の失敗（remoteから遅れていること）を検出できない。**
branchを作る頻度は低いため、その都度fetchする方を採る。

`hotfix/`で始まるbranchは対象外である。`main`から作るのが正しい（ADR-0004）。

`DESKCAT_SKIP_BASE_GUARD=1`で無効化できる。**意図して別の基点から作る場合の
逃げ道である。**使ったら理由を残す。
"""

import json
import os
import subprocess
import sys

import command_line

TRUNK = "origin/develop"
HOTFIX_PREFIX = "hotfix/"
SKIP_ENV = "DESKCAT_SKIP_BASE_GUARD"
TIMEOUT = 60

# branchを作る語の並び。`git branch <name>`は含めない（checkoutを伴わないため、
# その場で作業を始める経路ではない）。
CREATE_FORMS = (
    ["checkout", "-b"],
    ["switch", "-c"],
    ["switch", "--create"],
)


def _deny(reason):
    json.dump(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        },
        sys.stdout,
    )
    sys.stdout.write("\n")
    raise SystemExit(0)


def _git(args):
    try:
        result = subprocess.run(
            ["git"] + args, capture_output=True, text=True,
            timeout=TIMEOUT, check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return False, str(error)
    if result.returncode != 0:
        return False, (result.stderr or result.stdout).strip()
    return True, result.stdout.strip()


def _branch_creation(command):
    """branchを作る操作なら`(branch_name, explicit_start_point)`を返す。

    `explicit_start_point`は`git checkout -b NAME START`のSTARTである。
    明示されている場合、作成者は基点を選んでいる。**選んだ基点は尊重する。**
    """
    for args in command_line.invocations(command, "git"):
        for verb, flag in CREATE_FORMS:
            if args[:2] != [verb, flag]:
                continue
            positional = [t for t in args[2:] if not t.startswith("-")]
            if not positional:
                return None
            return positional[0], (
                positional[1] if len(positional) > 1 else None
            )
    return None


def main():
    if os.environ.get(SKIP_ENV) == "1":
        return 0
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError):
        return 0
    command = (payload.get("tool_input") or {}).get("command")
    if not isinstance(command, str):
        return 0
    creation = _branch_creation(command)
    if creation is None:
        return 0
    name, start_point = creation
    if name.startswith(HOTFIX_PREFIX):
        return 0
    if start_point is not None:
        # 基点を明示している。判断済みとして扱う。
        return 0

    ok, _ = _git(["fetch", "origin"])
    if not ok:
        # fetchできない環境（offline等）で作業を止めない。**ただし検査していない。**
        return 0
    ok, behind = _git(["rev-list", "--count", f"HEAD..{TRUNK}"])
    if not ok or not behind.isdigit():
        return 0
    if behind == "0":
        return 0
    ok, head = _git(["rev-parse", "--short", "HEAD"])
    ok2, trunk = _git(["rev-parse", "--short", TRUNK])
    _deny(
        f"branch `{name}`の基点が`{TRUNK}`から{behind} commit遅れている"
        f"（HEAD={head if ok else '?'}／{TRUNK}={trunk if ok2 else '?'}）。"
        " 古い基点では、後から入った文書を「存在しない」と読む。"
        "遅れているかどうかは、遅れている側からは見えない。"
        f" 先に基点をそろえる（git merge --ff-only {TRUNK}）。"
        f" 意図して別の基点から作る場合は基点を明示するか、{SKIP_ENV}=1を付ける。"
    )


if __name__ == "__main__":
    sys.exit(main())
