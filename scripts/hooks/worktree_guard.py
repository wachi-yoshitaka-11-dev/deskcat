#!/usr/bin/env python3
"""未commitの変更を失うgit操作の前に、人間の判断を求める。

Claude CodeのPreToolUse hookとして、Bash tool呼び出しの前に走る。stdinからhookの
入力JSONを読み、`tool_input.command`と、必要なら`git status`だけを見る。
**破棄そのものは実行しない。**

**なぜ作るか。**`AGENTS.md`「Gitと公開」は「ユーザーの既存変更を破棄、整形、
移動しない。**`git reset --hard`と強制checkoutはこの規則で扱う**（履歴書き換えでは
なく、未commitの変更を失う操作である）。実行前にworking treeを確認する」と定めて
いるが、**強制する仕組みが無く、モデルが規則を守ることだけに依存していた**
（[#325](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/325)の候補1）。
**`.claude/settings.json`が起動する8 hookに、捨てる側を見るものが無かった**
（2026-09-16に`.claude/settings.json`と`scripts/hooks/`を読んで確認した。
#325が同じことを2026-09-03に確認しているが、**その時点では`truncation_guard.py`と
`stop_claim_guard.py`がまだ無い**。commitは2026-09-05／2026-09-06である）。
見ているのは、作る側（`branch_base_guard.py`）、押す側（`push_gate.py`）、
申告する側（`gh_metadata_guard.py`／`coderabbit_gate.py`）、数える側
（`truncation_guard.py`）、主張する側（`stop_claim_guard.py`）、
報告する側（`merge_trailer_report.py`）である。

**`inspector_readonly_guard.py`だけは例外である。**allowlistで`reset`／`clean`／
`checkout`を拒否しており、**捨てる側を既に見ている。**ただし対象は検査subagentの
`Bash`だけであり（[ADR-0020](../../docs/decisions/0020-inspector-readonly-by-hook.md)）、
`.claude/settings.json`へは意図的に置かれていない。**通常のsessionは通らない。**

**失った未commitの変更は戻らない。**commit済みのものはreflogから戻せるが、
未commitの変更はどこにも残らない。**この検査が守るのはその差である。**

**対象は3系統である。**`git reset --hard`／checkout系による破棄
（`git checkout`／`git switch`／`git restore`）／`git clean`。
**checkout系は強制optionだけではない。**pathを指した`git restore <path>`や
`git checkout -- <path>`、hunk単位の`-p`も、そのpathの変更を捨てる。
**強制optionが無ければ見ない、ではない。**

**working treeが汚れていなければ通す**（#325の「working treeが汚れていない場合の
扱いを決め、記録する」への決定である）。失うものが無い呼び出しまで毎回askすると、
`git checkout <branch>`のような日常の操作が止まり、**hookそのものが無視される側の
失敗になる**（`truncation_guard.py`が同じ理由で対象を狭めている）。

**この決定には範囲がある。**次の2つは対象にしていない。

- **clean なtreeでの`git reset --hard <ref>`が落とす未pushのcommit。**この検査が
  守るのは未commitの変更である。**commitはreflogから戻せる。**
- **`git clean`。**こちらは`-n`／`--dry-run`を除き、treeの状態を見ずにaskする。
  理由は`_is_clean_tree`のdocstringが持つ。**「常に」ではない。**消える対象を
  確かめるための`git clean -n`まで止めると、診断文が案内する手順を塞ぐ。

**判定は`ask`であり`deny`ではない。**人が明示的に頼んだ破棄と、モデルが勝手に行う
破棄は**commandの字句としては同じである。**規則が要求しているのも禁止ではなく
「実行前にworking treeを確認する」であり、askはその確認そのものである。

**判定は字句と`git status`だけで行う。**shellの意味論は再現しない。
`command_line.py`が取れない範囲（alias、`xargs`経由等）はここでも取れない。
**取り切れていない形はCONTRIBUTINGの「取り切れていないもの」にある。**

**`git status`には`--no-optional-locks`を付ける。**素の`git status`は既定で
indexをrefreshし、`.git/index`を書く（2026-09-16実測。mtimeを変えたfileがある
状態で`git status --porcelain -uno`を実行するとindexのhashが変わり、
`--no-optional-locks`では変わらない。git 2.43.0／Linux x86_64）。
**hookはtool呼び出しのたびに走るため、書き込みとlock競合を持ち込まない。**

`DESKCAT_SKIP_WORKTREE_GUARD=1`で丸ごと無効化できる。**誤検知で作業が止まった
ときの逃げ道であり、常用するものではない。**使ったら理由をPull Request本文へ書く。
"""

import json
import os
import posixpath
import subprocess
import sys

import command_line

SKIP_ENV = "DESKCAT_SKIP_WORKTREE_GUARD"

# `git status`1回あたりの制限。**`.claude/settings.json`のhook timeout（30秒）より
# 十分小さく取る**（`push_gate.py`と同じ作法。同値だと、遅い環境でhook自体が先に
# 殺され、`_check`のfail-closedなaskへ到達しないまま素通りする）。
# **1回の呼び出しで複数のgit操作を検査することがあり、その分だけ積み上がる。**
TIMEOUT = 10

# 破棄しても失うものが無いpath。**生成物だけを入れる。**
# `pages/assets-manifest.json`は`scripts/prepare_pages.py`が書き換える生成物であり、
# `deskcat-preflight` skillが「`git status`が汚れていたら`git restore`」と
# 指示している経路そのものである。**この経路を止めると、push前の手順が回らない。**
# **広げない。**ここへ入れた分だけ、この検査は効かなくなる。
GENERATED_PATHS = ("pages/assets-manifest.json",)

# 作業treeを上書きする強制option。`git switch`の`--force`は`--discard-changes`の
# 別名である。**`-C`（`--force-create`）は入れない。**branchを作り直すoptionであり、
# 作業treeを捨てない。
FORCE_LONG_OPTIONS = ("--force", "--discard-changes")
FORCE_SHORT_LETTER = "f"

# hunk単位で捨てるmode。pathを伴わない`git checkout -p`は作業tree全体が対象になる。
PATCH_LONG_OPTION = "--patch"
PATCH_SHORT_LETTER = "p"

# 値を伴うoption。**値をpathspecとして読まないために要る。**
# `git checkout -b pages`の`pages`をpathspecと読むと、`pages/`が汚れている限り
# branch作成が毎回askになる（誤検知）。
VALUE_OPTIONS = {
    "checkout": (
        "-b", "-B", "--orphan", "-t", "--track", "--conflict",
        "--pathspec-from-file",
    ),
    "restore": ("-s", "--source", "--conflict", "--pathspec-from-file"),
}

# pathspecをfileから読むoption。**中身はhookから読めない。**
# 対象が分からないため、作業tree全体を見る側へ倒す。
PATHSPEC_FROM_FILE = "--pathspec-from-file"

# helpの表示だけを求める呼び出し。**何も捨てない。**既存hookと同じ扱いに揃える。
# **位置は見ない。**引数のどこかに`-h`という語があれば検査ごと飛ばす
# （`gh_metadata_guard.py`と同じ性質であり、CONTRIBUTINGの「取り切れていないもの」が
# 既に持っている。**字句だけで値と区別する手立てが無い。**）
HELP_OPTIONS = ("--help", "-h")

# 作業tree全体を対象にすることを表す。pathspecの空listと区別する。
WHOLE_TREE = ()


def _ask(reason):
    """PreToolUse hookとして、人間へ許可を求めて終了する。

    **`deny`ではない。**理由はmodule docstringが持つ。
    """
    json.dump(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "ask",
                "permissionDecisionReason": reason,
            }
        },
        sys.stdout,
    )
    sys.stdout.write("\n")
    raise SystemExit(0)


def _git(args):
    """gitを実行し`(成否, 出力)`を返す。失敗と例外を区別せず`False`にまとめる。"""
    try:
        result = subprocess.run(
            ["git"] + list(args), capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=TIMEOUT, check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False, ""
    if result.returncode != 0:
        return False, ""
    return True, result.stdout


def _has_long_option(args, names):
    """`--force`のような長いoptionが在るかを見る。`--opt=value`の形も拾う。"""
    for arg in args:
        if arg in names:
            return True
        if "=" in arg and arg.split("=", 1)[0] in names:
            return True
    return False


def _has_short_letter(args, letter):
    """`-f`／`-fd`のように束ねた短いoptionへ、その文字が在るかを見る。

    **`--`で始まる語は見ない。**`--force-create`の`f`を短いoptionと読むと、
    branchを作り直すだけの呼び出しを破棄として扱う。
    """
    for arg in args:
        if arg.startswith("--") or not arg.startswith("-") or arg == "-":
            continue
        if letter in arg[1:]:
            return True
    return False


def _is_forced(args):
    """作業treeを上書きする強制optionが在るかを見る。"""
    return (
        _has_long_option(args, FORCE_LONG_OPTIONS)
        or _has_short_letter(args, FORCE_SHORT_LETTER)
    )


def _pathspecs(args, subcommand):
    """呼び出しの引数から、pathspecになりうる語を取り出す。

    **`--`が在る場合はその後ろだけを見る。**gitと同じ区切りであり、前にある語は
    revisionである（`git checkout HEAD -- path`）。

    **`--`が無い場合は、optionでもoptionの値でもない語をすべて候補にする。**
    `git checkout main`の`main`はbranch名でありpathspecではないが、**字句だけでは
    区別できない。**区別は`git status`へ渡して行う。存在しないpathを渡した
    `git status`は何も出力しないため、branch名は結果として落ちる
    （2026-09-16実測。`git status --porcelain -uno -- <branch名>`は空、exit 0。
    git 2.43.0／python3 3.11.15／Linux x86_64）。

    **逆向きの誤検知は残る。**汚れたdirectoryと同名のbranchへ移動する
    `git checkout pages`はaskになる。**字句では区別できない。**
    `git checkout -b pages`の側は`VALUE_OPTIONS`で落としてあるが、
    位置引数として書かれた同名のbranchは落とせない。
    """
    if "--" in args:
        return [arg for arg in args[args.index("--") + 1:] if arg]
    value_options = VALUE_OPTIONS.get(subcommand, ())
    found = []
    skip_next = False
    for arg in args:
        if skip_next:
            skip_next = False
            continue
        if arg in value_options:
            skip_next = True
            continue
        if arg.startswith("-"):
            continue
        found.append(arg)
    return found


def _is_generated(path):
    """生成物として除外してよいpathかを見る。`./`を付けた書き方も同じに扱う。

    **`GENERATED_PATHS`はrepository root相対である。**pathspecはgitがcwd相対で
    解決するため、**sub directoryで走ると除外は効かない**（`pages/`の中で
    `git restore assets-manifest.json`と書いた形）。効かない側へ倒れるだけであり、
    余分に止まる。**広い側へ倒さないために、前方一致ではなく完全一致で見る。**
    """
    normalized = posixpath.normpath(path)
    return normalized in GENERATED_PATHS


def _plan(subcommand, args):
    """この呼び出しが失いうる範囲を返す。失うものが無ければ`None`。

    返すのは`(commandの形, pathspecの並び)`である。pathspecが`WHOLE_TREE`
    （空）なら作業tree全体を見る。
    """
    if subcommand == "reset":
        if _has_long_option(args, ("--hard",)):
            return "git reset --hard", WHOLE_TREE
        # `--soft`／`--mixed`／`--merge`／`--keep`は対象外である。
        # **`--merge`と`--keep`は、衝突する未commitの変更があると中止する。**
        return None
    if subcommand == "clean":
        if _has_long_option(args, ("--dry-run",)) or _has_short_letter(args, "n"):
            return None
        return "git clean", WHOLE_TREE
    if subcommand == "switch":
        # `git switch`はpathspecを取らない。**強制optionだけが作業treeを捨てる。**
        if _is_forced(args):
            return "git switch --discard-changes", WHOLE_TREE
        return None
    if subcommand in ("checkout", "restore"):
        if subcommand == "restore" and _has_long_option(args, ("--staged",)) \
                and not _has_long_option(args, ("--worktree",)):
            # index だけを戻す。**作業treeのfileは書き換わらない。**
            return None
        if _is_forced(args) \
                or _has_long_option(args, (PATCH_LONG_OPTION,)) \
                or _has_short_letter(args, PATCH_SHORT_LETTER) \
                or _has_long_option(args, (PATHSPEC_FROM_FILE,)):
            return f"git {subcommand}", WHOLE_TREE
        paths = _pathspecs(args, subcommand)
        if paths:
            return f"git {subcommand} <path>", paths
        return None
    return None


def _is_clean_tree(prefix, paths):
    """指定範囲に未commitの変更が無いかを`git status`で見る。

    返すのは`(判定できたか, 変更行の並び)`である。

    **`--untracked-files=no`を付ける。**追跡していないfileは`git reset --hard`でも
    `git checkout -- <path>`でも消えない。数えると、捨てないものを理由にaskする
    ことになる。

    **逆に、staged済みで作業treeがindexと一致する変更は数えている。**
    `git status --porcelain -uno`は`M `として出すが、`git checkout -- <path>`は
    indexから戻すため**この変更を失わない**（2026-09-16実測。git 2.43.0）。
    **区別せずaskする。過剰に止める側である。**`git reset --hard`は同じ変更を
    失うため、字句で分かれる両者を`git status`の出力側で分けると、失う側を
    取り落とす。**`git clean`は別の理由でこの関数を通らない**（`-n`／`--dry-run`
    以外は`_plan`が`WHOLE_TREE`を返し、呼び出し側が状態を見ずにaskする）。
    **`git clean -x`は無視fileも消すが、`git status --porcelain`は`--ignored`が
    無い限り無視fileを出さない。**「汚れていない」を確かめられない対象で、
    確かめた顔をしない。

    **`prefix`は、検査対象のcommandでsubcommandより前にあったglobal optionである。**
    `git -C other reset --hard`が見るのはhookのcwdではない。**同じ`-C`を付けずに
    数えると、別のrepositoryのtreeを見て「汚れていない」と読む。**
    `-C`以外（`-c key=value`／`--no-pager`等）もそのまま渡す。**選り分けない。**
    選り分けると、どれを渡すかの一覧を`command_line`側と二重に持つことになる。
    `git --bare status`のように`status`が受け付けない組み合わせでは実行が失敗し、
    **「確認できなかった」側のaskになる**（通す側へは倒れない）。

    **それでも、pathspecはhookのcwdからの相対で解決される。**
    `cd other && git restore x`のようにcommandの中で移動した場合、ここで見るpathは
    実際の対象と違う。**取り切れていないものとしてCONTRIBUTINGに書いてある。**
    """
    arguments = list(prefix) + [
        "--no-optional-locks", "status", "--porcelain", "--untracked-files=no",
    ]
    if paths:
        arguments.append("--")
        arguments.extend(paths)
    ok, output = _git(arguments)
    if not ok:
        return False, []
    return True, [line for line in output.splitlines() if line.strip()]


def _reason(form, changes):
    """askの理由文を組み立てる。**件数は数えたものをそのまま書く。**"""
    listed = ", ".join(line.strip() for line in changes[:5])
    if len(changes) > 5:
        listed += f", ほか{len(changes) - 5}件"
    return (
        f"`{form}`は未commitの変更を捨てる。"
        f" 対象範囲に未commitの変更が{len(changes)}件ある（{listed}）。"
        " 失った未commitの変更は戻らない（commitはreflogから戻せるが、"
        "未commitの変更はどこにも残らない）。"
        " 捨ててよいかを確かめる。残すなら先にcommitまたはstashする。"
        f" 誤検知で頻発する場合は{SKIP_ENV}=1で無効化し、理由を残す。"
    )


def _check(prefix, subcommand, args):
    plan = _plan(subcommand, args)
    if plan is None:
        return
    form, paths = plan
    if subcommand == "clean":
        # **treeの状態を見ない。**理由は`_is_clean_tree`のdocstringが持つ。
        _ask(
            "`git clean`は追跡していないfileを消す。"
            " `-x`を付けると無視fileも消える。"
            " **`git status`では消える対象を数え切れないため、この検査は"
            "作業treeの状態を見ていない。**消してよいかを`git clean -n`で"
            "確かめてから実行する。"
            f" 誤検知で頻発する場合は{SKIP_ENV}=1で無効化し、理由を残す。"
        )
    targets = [path for path in paths if not _is_generated(path)]
    if paths and not targets:
        # 生成物だけが対象である。**失うものが無い。**
        return
    determined, changes = _is_clean_tree(prefix, targets)
    if not determined:
        _ask(
            f"`{form}`は未commitの変更を捨てるが、"
            "作業treeの状態を確認できなかった（`git status`が実行できない）。"
            " 捨ててよいかを人が確かめる。"
            f" 誤検知で頻発する場合は{SKIP_ENV}=1で無効化し、理由を残す。"
        )
    if changes:
        _ask(_reason(form, changes))


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
    for args in command_line.invocations(command, "git"):
        remaining = command_line.skip_global_options(args, "git")
        if not remaining:
            continue
        if any(option in remaining for option in HELP_OPTIONS):
            continue
        # 読み飛ばしたglobal option（`git -C <path>`）は、状態を見るときにも要る。
        prefix = args[:len(args) - len(remaining)]
        _check(prefix, remaining[0], remaining[1:])
    return 0


if __name__ == "__main__":
    sys.exit(main())
