#!/usr/bin/env python3
"""`develop`へ直接pushする操作の前に`review_gate.py gate`を実行する。

Claude CodeのPreToolUse hookとして、`git push`の前に走る。

**直接commit経路には強制点が無い。**Pull Requestを通る変更は`review-gate.yml`が
`gate`を実行するため、宣言の漏れはmerge前に落ちる。直接pushはCIを通らないので、
**押す瞬間に人が正しいsubcommandを選ぶことに依存していた。**

実際にそれで落ちた。`b93b309`は`docs/decisions/`の3ファイルを変更しながら
`Instruction-Change: reviewed-as-data`を持たないまま`develop`へ入った。
push前に実行したのが`receipt`だけだったためである。

| subcommand | `b93b309`に対する結果 |
|---|---|
| `receipt` | exit 0 |
| `gate` | exit 1 |

`receipt`はhead commitのtrailerの整合だけを見て`_check_instructions`を呼ばない。
**共有branchのため後から直せない。**免除登録にIssueとPull Requestが1本ずつ要った。

## 検査するのは`gate`だけである

**直接commitしてよい基準そのもの（`minor`または`fixup`＋`Refs`）は検査しない。**
検査すると、**trailerを落としたsquash commitをamendして直す手順**を誤って止める。
その手順はCONTRIBUTINGの「Merge方式」が定めた修復であり、対象のcommitは
`Change-Class: review-required`を持つ。基準で測ると拒否になる。

区別する手立てが字句には無い。**推測で拾うと誤検知になり、誤検知はhookそのものを
無効化される側の失敗である。**取れないものは取れないままにする。

## 取れない範囲

- `refs/heads/develop`以外の名前で同じbranchを指す書き方
- `--mirror`と`--all`のように、refspecを書かずに複数のbranchを更新する形
- `git`自体が起動できない、または`gate`が時間内に終わらない場合（**通す。ただし検査していない**）
- alias、shell function、`sh -c '...'`の内側（`command_line`の制約と同じ）

`DESKCAT_SKIP_PUSH_GATE=1`で無効化できる。**使ったら理由を残す。**
"""

import json
import os
import subprocess
import sys
from pathlib import Path

import command_line

REMOTE = "origin"
GUARDED = "develop"
SKIP_ENV = "DESKCAT_SKIP_PUSH_GATE"
# 1回あたりの制限。**fetchとgateで2回走るため、hook全体の制限時間の半分未満にする。**
TIMEOUT = 50

# 押すcommitが無い形。**検査しない。**
#
# **`--tags`／`--all`／`--mirror`はここへ置かない。**refspecを併記した
# `git push --tags origin HEAD:develop`まで取り落とす。refspecを書かない形は、
# `develop`を指すrefspecが1つも見つからないため、どのみち対象にならない。
SKIP_OPTIONS = frozenset({"--dry-run", "-n", "--delete", "-d"})

# `develop`を指す書き方。これ以外は取れない。
GUARDED_DESTINATIONS = frozenset({GUARDED, f"refs/heads/{GUARDED}"})

# `git`の大域option。`push`より前に置かれ、値を1つ取る。
GLOBAL_OPTIONS_WITH_VALUE = frozenset({"-C", "-c"})

# 値を取らない大域option。
GLOBAL_FLAGS = frozenset({"-p", "-P", "--paginate", "--no-pager", "--bare"})


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


def _run(args, cwd=None):
    try:
        result = subprocess.run(
            args, capture_output=True, text=True,
            timeout=TIMEOUT, check=False, cwd=cwd,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return None, str(error)
    return result.returncode, (result.stdout or "") + (result.stderr or "")


def _git(args, cwd=None):
    code, output = _run(["git"] + args, cwd=cwd)
    return (code == 0), output.strip()


def _repository_root(directory=None):
    """押そうとしているrepositoryのrootを返す。

    **scriptの位置ではなくcwdから引く。**worktreeではscriptの位置がworktree rootに
    なるため一致するが、cwdから引く方が「どのtreeをpushするのか」と一致する。
    引けない場合だけscriptの位置へ落とす。
    """
    ok, top = _git(["rev-parse", "--show-toplevel"], cwd=directory)
    if ok and top:
        return Path(top)
    if directory is not None:
        # `-C`で指した先がrepositoryでない。**どこを検査すべきか決まらないので見ない。**
        return None
    return Path(__file__).resolve().parent.parent.parent


def _strip_global_options(args):
    """`git`の大域optionを外し、`(残りの語, -Cで指定されたdirectory)`を返す。

    `git -C dir push ...`と`git -c key=value push ...`を拾うために要る。
    **外さないと`args[0]`が`push`にならず、素通りする。**
    """
    directory = None
    index = 0
    while index < len(args):
        token = args[index]
        if token in GLOBAL_OPTIONS_WITH_VALUE:
            value = args[index + 1] if index + 1 < len(args) else None
            if token == "-C" and value is not None:
                directory = value
            index += 2
            continue
        if token.startswith("--") or token in GLOBAL_FLAGS:
            index += 1
            continue
        break
    return args[index:], directory


def pushed_source(command):
    """`develop`を更新するpushなら`(押す側のref, 実行するdirectory)`を返す。

    `git push origin HEAD:develop`は`HEAD`を、`git push origin develop`は`develop`を
    返す。`git push`だけの形はupstreamを引いて判定する。該当しなければ`None`。

    **`git push origin :develop`は返さない。**これはbranchの削除であり、
    押すcommitが無い。`HEAD`を押すものとして扱うと、無関係な範囲を検査する。
    """
    for raw in command_line.invocations(command, "git"):
        args, directory = _strip_global_options(raw)
        if not args or args[0] != "push":
            continue
        if SKIP_OPTIONS.intersection(args):
            continue
        positional = [t for t in args[1:] if not t.startswith("-")]
        if not positional:
            # `git push`だけの形。upstreamが`origin/develop`なら対象である。
            ok, upstream = _git(
                ["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"],
                cwd=directory,
            )
            if ok and upstream == f"{REMOTE}/{GUARDED}":
                return "HEAD", directory
            continue
        if positional[0] != REMOTE:
            continue
        for refspec in positional[1:]:
            spec = refspec.lstrip("+")
            source, separator, destination = spec.partition(":")
            if separator and not source:
                # `:develop`はbranchの削除である。押すcommitが無い。
                continue
            destination = destination or source
            if destination in GUARDED_DESTINATIONS:
                return source, directory
    return None


def main():
    if os.environ.get(SKIP_ENV) == "1":
        return 0
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError):
        return 0
    command = command_line.command_from(payload)
    if command is None:
        return 0
    target = pushed_source(command)
    if target is None:
        return 0
    source, directory = target

    root = _repository_root(directory)
    if root is None:
        return 0
    gate = root / "scripts" / "review_gate.py"
    if not gate.is_file():
        # 検査する道具が無い環境で作業を止めない。**ただし検査していない。**
        return 0

    # fetchしなければ、localの`origin/develop`が古いまま範囲に入る。
    # **他人のcommitを自分の宣言漏れとして数えることになる。**
    ok, _ = _git(["fetch", REMOTE], cwd=str(root))
    if not ok:
        # offline等。**古いbaseで測ると誤検知になるため、測らない。**
        # 通したことを、検査したことと読まない。
        return 0

    base = f"{REMOTE}/{GUARDED}"
    ok, ahead = _git(["rev-list", "--count", f"{base}..{source}"], cwd=str(root))
    if not ok:
        # refを解決できない。**押す前の判定材料が無いので止めない。**
        return 0
    if ahead == "0":
        return 0

    code, output = _run(
        [
            sys.executable, str(gate), "gate",
            "--repository-root", str(root),
            "--base", base, "--head", source,
        ],
        cwd=str(root),
    )
    if code is None or code == 0:
        return 0
    _deny(
        f"`{base}..{source}`（{ahead} commit）が`review_gate.py gate`で落ちた。"
        " 直接pushはCIを通らないため、ここが最後の検査である。\n\n"
        f"{output.strip()}\n\n"
        " `receipt`は通っても`gate`は落ちることがある。"
        "`receipt`はhead commitのtrailerだけを見て、指示sourceの検査をしない。"
        " 共有branchへ入ると後から直せず、免除登録にIssueとPull Requestが要る。"
        f" 意図して押す場合は{SKIP_ENV}=1を付け、理由を残す。"
    )


if __name__ == "__main__":
    sys.exit(main())
