#!/usr/bin/env python3
"""検査 subagent の`Bash`を、読み取りだけに機構で制限する。

`.claude/agents/consistency-inspector.md`と`.claude/agents/fresh-context-reviewer.md`の
frontmatterへ`hooks.PreToolUse`として書き、**その subagent の`Bash`呼び出しにだけ**掛ける。
`.claude/settings.json`へは置かない。**置くと通常の作業 session まで止まる。**

## なぜ必要か

両 agent は本文で「read-only である。file を変更しない」と書いているが、
**担保しているのは指示だけである。**`tools`から`Bash`を外せば機構になるが、
両 agent は`git show`／`git diff`で差分を取得するため、外すと検査が成立しない
（[#373](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/373)で確認済み。
`Read`／`Grep`／`Glob`はgitを実行できない）。

`sandbox`設定は採らなかった。理由は[ADR-0020](../../docs/decisions/0020-inspector-readonly-by-hook.md)にある。

## 判定

**allowlistである。**列挙したものだけを通し、それ以外は拒否する。denylistにしない。
禁止を数え上げる形は、数え漏れがそのまま穴になる。

次のいずれかに当たれば拒否する。

1. `shlex`がcommandを語へ分けられない（**fail closed**。下記）
2. shell metacharacter（`>|;&()`` ` ``$<{}`）を含む語がある。**単独の区切り語は除く**
3. command位置のprogramが`ALLOWED_PROGRAMS`に無い
4. `git`の subcommand が`GIT_READONLY_SUBCOMMANDS`に無い

**判定は行ごとに行う。**`shlex`は改行を空白として扱うため、
`git show`と`rm -rf /`を改行で並べると1つの語列に潰れ、`rm`がcommand位置として見えない。

**この hook は tokenize 失敗を素通りさせない。**他の hook（`gh_metadata_guard.py`等）は
素通りさせる。**目的が違う。**あちらは「書き忘れを指摘する」ものであり、素通りは
指摘漏れで済む。こちらは**書き込みを止める境界**であり、解釈できない入力を通すことは
境界を開けることと同じである。

## 取れないもの

`command_line`の module docstring が挙げるものは、ここでも取れない。
alias、shell function、変数展開、`xargs`経由、`sh -c`の内側。
**ただし`sh`・`bash`・`xargs`は`ALLOWED_PROGRAMS`に無いため、command位置に現れれば拒否される。**
残るのは、許可した program 自身が持つ書き込み経路である。
そのため`ALLOWED_PROGRAMS`には**単体では file を書けないものだけ**を入れる
（`sort -o`、`uniq out`、`sed -i`、`tee`のような書き込み経路を持つものを入れない）。
"""

import json
import re
import sys

import command_line

# command位置に現れてよいprogram。**単体でfileを書けないものだけを入れる。**
#
# 入れなかったものと理由:
# - `sort` — `-o FILE`で書ける
# - `uniq` — 2つ目の位置引数が出力fileになる
# - `sed` — `-i`で書ける
# - `tee`、`dd`、`cp`、`mv`、`install` — 書くためのcommandである
# - `python3`、`sh`、`bash`、`xargs`、`env`以外のinterpreter — 任意のcodeを実行できる
#   （`env`は`command_line.TRANSPARENT_PREFIXES`側で後続commandへ透過する）
ALLOWED_PROGRAMS = frozenset(
    {
        "git",
        "cat",
        "head",
        "tail",
        "wc",
        "nl",
        "cut",
        "tr",
        "grep",
        "rg",
        "echo",
        "true",
        "ls",
        "basename",
        "dirname",
    }
)

# `git`で許すsubcommand。**flagに関係なくfileを書かないものだけを入れる。**
#
# 入れなかったものと理由:
# - `branch`、`tag` — `-d`／`-D`で消せる
# - `config` — `--global`等で書ける
# - `checkout`、`switch`、`restore`、`reset`、`clean` — working treeを変える
# - `add`、`commit`、`push`、`fetch`、`worktree`、`stash`、`gc` — 状態を変える
GIT_READONLY_SUBCOMMANDS = frozenset(
    {
        "show",
        "log",
        "diff",
        "status",
        "rev-parse",
        "rev-list",
        "merge-base",
        "ls-files",
        "ls-tree",
        "cat-file",
        "blame",
        "describe",
        "shortlog",
        "show-ref",
        "name-rev",
        "grep",
    }
)

# `git`のsubcommandより前に来るglobal option。値を取るものは次の語も読み飛ばす。
GIT_GLOBAL_OPTIONS_WITH_VALUE = frozenset({"-C", "--git-dir", "--work-tree", "--namespace"})

# **`git`自身が持つ、commandを実行するかfileを書くoption。**subcommandが読み取り専用でも効く。
#
# - `-c`／`--config-env` — `git -c core.pager='rm -rf /' log`のように、config経由で任意commandを実行できる。
#   `-c diff.external=...`も同じ（`--ext-diff`はconfigを見るだけなので、`-c`を塞げば足りる）
# - `--exec-path` — gitがsubcommandを探すpathを差し替える
# - `-O`／`--open-files-in-pager` — `git grep`がpagerとして任意commandを起動する
# - `--output` — **`git diff --output=<file>`はfileを書く。**subcommandは読み取り専用のままである
#
# **`=`付きの形（`--output=f`）も拾う。**先頭一致で判定する。
# **subcommandより前（global位置）で拒否するもの。**
DENIED_GIT_GLOBAL_OPTIONS = ("-c", "--config-env", "--exec-path")

# **subcommandより後ろで拒否するもの。**
#
# **`-c`をここへ入れない。**subcommandの後ろの`-c`は`git log -c`（merge の combined diff）であり、
# 読み取り専用である。**危険なのはglobal位置の`-c`だけである。**
DENIED_GIT_SUBCOMMAND_OPTIONS = ("-O", "--open-files-in-pager", "--output")

# command位置に現れる環境変数の代入。`command_line`は後続commandへ透過させるが、
# **`GIT_EXTERNAL_DIFF=rm git show`のように、環境変数だけで任意commandを起動できる。**
# ここでは透過させず拒否する。
ASSIGNMENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")

# 語の中に現れたら拒否するshell metacharacter。
#
# **`shlex.split`は空白でしか語を切らない。**`cat a>b`も`cat a;rm -rf /`も1語
# （`a>b`／`a;rm`）になり、**`>`や`rm`がcommand位置として見えない。**
# そのため、`SEPARATORS`として単独で現れた語を除き、これらを含む語をすべて拒否する。
#
# **`grep "=>"`のような正当な使い方も拒否する。**誤検知を承知で採る。
# 両 agent は`Grep` toolを持っており、検索はそちらで足りる。
# 境界を緩めるより、代替がある側を止める。
SHELL_METACHARACTERS = frozenset(">|;&()`$<{}")

# `shlex`は改行も空白として扱うため、改行で区切られたcommandが1つの語列に潰れる。
# **`git show\nrm -rf /`が`git show rm -rf /`に見える。**行ごとに分けて判定する。
LINE_SPLIT_RE = re.compile(r"[\r\n]+")


def _deny(reason):
    """PreToolUse hookの拒否を出力して終了する。"""
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


def _split_git_args(args):
    """`git`の引数列を`(global部, subcommand, subcommand以降)`へ分ける。

    `git -C /path --no-pager show HEAD`のようにglobal optionが前に付く形を読む。
    subcommandが見つからなければ`(args, None, [])`を返す。
    """
    index = 0
    while index < len(args):
        token = args[index]
        if not token.startswith("-"):
            return args[:index], token, args[index:]
        if token in GIT_GLOBAL_OPTIONS_WITH_VALUE:
            index += 2
            continue
        index += 1
    return args, None, []


def _matches_option(token, option):
    """語がoptionに当たるか。`--output=f`と、短いoptionの`-Ofoo`の形も拾う。"""
    if token == option or token.startswith(option + "="):
        return True
    # `-O`のような2文字のshort optionは、値を続けて書ける（`-Oless`）。
    return len(option) == 2 and option.startswith("-") and token.startswith(option)


PREFIX = "検査 subagent の Bash は読み取り専用である。"


def _check_line(line):
    """1行分の拒否理由を返す。問題が無ければ`None`。"""
    tokens = command_line.tokenize(line)
    if not tokens:
        if not line.strip():
            return None
        return (
            f"{PREFIX}"
            f" command を語へ分けられなかったため拒否した: {line!r}。"
            "**解釈できない入力を通すことは、境界を開けることと同じである。**"
        )

    for token in tokens:
        if token in command_line.SEPARATORS:
            continue
        found = sorted(SHELL_METACHARACTERS.intersection(token))
        if found:
            return (
                f"{PREFIX}"
                f" shell metacharacter（{''.join(found)}）を含む語のため拒否した: {token!r}。"
                "**`shlex`は空白でしか語を切らないため、`cat a>b`や`cat a;rm -rf /`は"
                "1語になり、redirect や次の command が command 位置として見えない。**"
                " 検索なら `Grep` tool を使う。"
            )

    for token in tokens:
        if ASSIGNMENT_RE.match(token):
            return (
                f"{PREFIX}"
                f" 環境変数の代入を含むため拒否した: {token!r}。"
                "**`GIT_EXTERNAL_DIFF=rm git show` のように、環境変数だけで任意 command を起動できる。**"
            )

    for program in command_line.programs(line):
        name = program.rsplit("/", 1)[-1]
        if name not in ALLOWED_PROGRAMS:
            return (
                f"{PREFIX}"
                f" 許可していない program のため拒否した: {name!r}。"
                f" 許可しているのは {', '.join(sorted(ALLOWED_PROGRAMS))} だけである。"
                " 検査に本当に必要なら、ADR-0020 を更新して allowlist を広げる。"
            )

    for args in command_line.invocations(line, "git"):
        globals_, subcommand, rest = _split_git_args(args)
        for token in globals_:
            for option in DENIED_GIT_GLOBAL_OPTIONS:
                if _matches_option(token, option):
                    return (
                        f"{PREFIX}"
                        f" git の global option {token!r} は、subcommand が読み取り専用でも"
                        " 任意の command を実行できるため拒否した。"
                    )
        for token in rest:
            for option in DENIED_GIT_SUBCOMMAND_OPTIONS:
                if _matches_option(token, option):
                    return (
                        f"{PREFIX}"
                        f" git の option {token!r} は、subcommand が読み取り専用でも"
                        " command を実行するか file を書くため拒否した。"
                    )
        if subcommand is None:
            return f"{PREFIX} `git` に subcommand が無いため拒否した。"
        if subcommand not in GIT_READONLY_SUBCOMMANDS:
            return (
                f"{PREFIX}"
                f" 読み取り専用でない git subcommand のため拒否した: {subcommand!r}。"
                f" 許可しているのは {', '.join(sorted(GIT_READONLY_SUBCOMMANDS))} だけである。"
            )

    return None


def check(command):
    """拒否理由を返す。問題が無ければ`None`。

    **hookの入出力から切り離してある。**testが理由の文面まで確かめられるようにするためである。

    **行ごとに判定する。**`shlex`は改行を空白として扱うため、まとめて渡すと
    2行目のcommandがcommand位置として見えない。
    """
    if not command.strip():
        return None
    for line in LINE_SPLIT_RE.split(command):
        reason = _check_line(line)
        if reason is not None:
            return reason
    return None


def main():
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError):
        # **ここは fail closed にしない。**入力がJSONでないのは hook 側の事故であり、
        # 対象 command の問題ではない。`command_from`が`None`を返す場合と同じ扱いにする。
        # **境界としては弱い。**この穴は ADR-0020 の欠点に書いてある。
        return 0
    command = command_line.command_from(payload)
    if command is None:
        return 0
    reason = check(command)
    if reason is not None:
        _deny(reason)
    return 0


if __name__ == "__main__":
    sys.exit(main())
