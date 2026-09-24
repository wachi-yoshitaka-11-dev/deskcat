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


def _run(args, cwd=None, env=None):
    try:
        result = subprocess.run(
            args, capture_output=True, text=True,
            timeout=TIMEOUT, check=False, cwd=cwd,
            env=None if not env else {**os.environ, **env},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return None, str(error)
    return result.returncode, (result.stdout or "") + (result.stderr or "")


def _git(args, cwd=None, env=None):
    code, output = _run(["git"] + args, cwd=cwd, env=env)
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


# 値を次の語で取るglobal optionと、同じ意味を持つ環境変数。`pushed_sources`がupstreamを
# 引くとき、pushするrepositoryと同じものを見るために使う（#472）。**検査そのものには渡さない。**
# これらが付いたpushは`_inspect`が止める（下の`UNRESOLVED_OPTIONS`）。
GLOBAL_OPTION_ENV = {
    "--git-dir": "GIT_DIR",
    "--work-tree": "GIT_WORK_TREE",
    "--namespace": "GIT_NAMESPACE",
}

# 指定されると、検査するrepository・tree・refを一意に決められないもの。`_inspect`が止める。
# **`--namespace`も含める。**refの探し方を変えるoptionであり、検査側で再現しない（#472）。
UNRESOLVED_OPTIONS = ("--git-dir", "--work-tree", "--namespace")


def _strip_global_options(args):
    """`git`の大域optionを外し、`(残りの語, -Cで指定されたdirectory, 環境変数)`を返す。

    環境変数は`--git-dir`／`--work-tree`／`--namespace`を同じ意味の`GIT_*`へ移したもの。
    **相対pathは、gitと同じく`-C`の後のdirectoryから解決して絶対pathにする。**
    空でなければ`_inspect`がそのpushを止める（検査するrepositoryを一意に決められない）。

    `git -C dir push ...`と`git -c key=value push ...`を拾うために要る。
    **外さないと`args[0]`が`push`にならず、素通りする。**

    **判定は`command_line`が持つ。**2026-09-16に[#325](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/325)で移した。
    このfileが持っていた規則をそのまま共有側へ置き、ここは呼ぶだけにした。
    **同じ読み飛ばしを要るhookが複数になったためである**（`truncation_guard.py`は
    狭い版を私有しており、`git --no-pager`／`git -c`を取り落としていた）。
    **その後、subcommandを位置で読んでいた5 hookも同じ関数を呼ぶようにした**
    （review指摘。`gh --repo o/r pr create`でdenyが抜けることを実測した）。
    """
    directory = command_line.global_option_value(args, "git", "-C")
    base = Path(directory) if directory is not None else Path.cwd()
    env = {}
    for option, name in GLOBAL_OPTION_ENV.items():
        value = command_line.global_option_value(args, "git", option)
        if value is None:
            continue
        if option == "--namespace":
            env[name] = value
        else:
            env[name] = str((base / value).resolve())
    return (
        command_line.skip_global_options(args, "git"),
        directory,
        env,
    )


def pushed_sources(command):
    """`develop`を更新するpushを、commandに現れる順にすべて`(押す側のref, 実行するdirectory, 環境変数)`の
    listで返す。無ければ空のlistを返す。

    **最初の1件で止めない。**止めると、`git push origin HEAD:develop && git --git-dir=X push ...`の
    2つ目を見落とす（#472。PR #473のreview指摘）。2件以上あれば`main`が止める。
    1回の`git push`の中では、最初に一致したrefspecを1件として数える。

    `git push origin HEAD:develop`は`HEAD`を、`git push origin develop`は`develop`を
    返す。`git push`だけの形はupstreamを引いて判定する。

    **`git push origin :develop`は返さない。**これはbranchの削除であり、
    押すcommitが無い。`HEAD`を押すものとして扱うと、無関係な範囲を検査する。
    """
    found = []
    for raw in command_line.invocations(command, "git"):
        args, directory, env = _strip_global_options(raw)
        if not args or args[0] != "push":
            continue
        if SKIP_OPTIONS.intersection(args):
            continue
        positional = [t for t in args[1:] if not t.startswith("-")]
        if not positional:
            # `git push`だけの形。upstreamが`origin/develop`なら対象である。
            ok, upstream = _git(
                ["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"],
                cwd=directory, env=env,
            )
            if ok and upstream == f"{REMOTE}/{GUARDED}":
                found.append(("HEAD", directory, env))
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
                found.append((source, directory, env))
                break
    return found


def pushed_source(command):
    """[`pushed_sources`]の最初の1件を返す。無ければ`None`。"""
    found = pushed_sources(command)
    return found[0] if found else None


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
    targets = pushed_sources(command)
    if len(targets) > 1:
        # **1件でも、各段（rev-parse・fetch・rev-list・`gate`の4段×`TIMEOUT`50秒）の上限を合わせると
        # hookの制限時間を超えうる（変更前から同じ）。件数が増えると、その分さらに延びる。**
        # 時間切れに頼ると、どちらに倒れても検査漏れになりうる。**止めて1件にさせる**（#472）。
        _deny(
            f"1つのcommandの中に`develop`へのpushが{len(targets)}件ある。"
            "検査が時間内に終わらない恐れがあるため止めた。1件ずつpushし直す。"
            f" 意図して押す場合は{SKIP_ENV}=1を付け、理由を残す。"
        )
    for target in targets:
        _inspect(*target)
    return 0


def _inspect(source, directory, env):
    """1件のpushを検査する。止めるなら`_deny`で抜ける。通すなら何も返さない。"""
    unresolved = [option for option in UNRESOLVED_OPTIONS if GLOBAL_OPTION_ENV[option] in env]
    if unresolved:
        # **検査するrepository・tree・refを一意に決められない。**`--git-dir`だけならgitは
        # cwdをwork treeとして扱い、`--work-tree`だけならrepositoryをcwdから探す。
        # `--namespace`はrefの探し方を変える（検査側で再現しない）。どれも検査側の`root`（`scripts/review_gate.py`を
        # 読む場所）とpushするrepositoryが食い違いうる。**見ないまま通すと、この形が
        # 検査を外す経路になる**（#472）。
        _deny(
            f"`{'`／`'.join(unresolved)}`を付けた`develop`へのpushは、"
            "このhookが検査するrepositoryを決める方法（`-C`とcwd）と合わないため止めた。"
            " `git -C <repository> push ...`の形で押し直す。"
            f" 意図して押す場合は{SKIP_ENV}=1を付け、理由を残す。"
        )

    root = _repository_root(directory)
    if root is None:
        return
    gate = root / "scripts" / "review_gate.py"
    if not gate.is_file():
        # 検査する道具が無い環境で作業を止めない。**ただし検査していない。**
        return

    # fetchしなければ、localの`origin/develop`が古いまま範囲に入る。
    # **他人のcommitを自分の宣言漏れとして数えることになる。**
    ok, _ = _git(["fetch", REMOTE], cwd=str(root))
    if not ok:
        # offline等。**古いbaseで測ると誤検知になるため、測らない。**
        # 通したことを、検査したことと読まない。
        return

    base = f"{REMOTE}/{GUARDED}"
    ok, ahead = _git(["rev-list", "--count", f"{base}..{source}"], cwd=str(root))
    if not ok:
        # refを解決できない。**押す前の判定材料が無いので止めない。**
        return
    if ahead == "0":
        return

    code, output = _run(
        [
            sys.executable, str(gate), "gate",
            "--repository-root", str(root),
            "--base", base, "--head", source,
        ],
        cwd=str(root),
    )
    if code is None or code == 0:
        return
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
