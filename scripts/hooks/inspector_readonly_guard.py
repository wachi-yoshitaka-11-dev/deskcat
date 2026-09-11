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

**この guard は保証ではない。best effort である。**字句判定で`git`のoption文法を追う構造であり、
**guardを通るのに保護がgitへ届かない形が繰り返し見つかっている。**
**調べた範囲では、per-subagentで品質を保ったまま強制する方法が見つからなかった。**
`permissionMode`は評価しておらず、**網羅は主張していない**（ADR-0020の`欠点`と`検討した選択肢`）。
その結果としてこれを引き受けている。

## 判定

**allowlistである。**列挙したものだけを通し、それ以外は拒否する。denylistにしない。
禁止を数え上げる形は、数え漏れがそのまま穴になる。

次のいずれかに当たれば拒否する。**おおむね`_check_line`が判定する順である。**

1. 行が復帰文字（CR）を含む（**tokenize前に見る。**`shlex`はCRを空白として切るため語に残らない）
2. `shlex`がcommandを語へ分けられない（**fail closed**。下記）
3. shell metacharacter（`>|;&()`` ` ``$<{}`）を含む語がある。**単独の区切り語も拒否する**
4. command位置の前置語（`sudo`／`env`／`exec`など）か、環境変数代入がある
5. command位置のprogram語に`/`が含まれる
6. command位置のprogramが`ALLOWED_PROGRAMS`に無い
7. `rg`のoptionが`DENIED_RG_OPTIONS`に当たる（`--pre`／`--hostname-bin`／`-z`／`--search-zip`。
   **short optionの束ね（`-nz`）も見る**）
8. `git`のoptionが拒否一覧に当たる（完全一致・`=`付き・**short optionの束ね**）
9. `git`の`--help`がある（位置に依らない）
10. `git`のoptionが拒否一覧の**短縮綴り**に当たる
11. `git`に許可していないglobal optionがある（**allowlistである**）
12. `git`の subcommand が`GIT_READONLY_SUBCOMMANDS`に無い
13. `git`の語が pretty format の`%G*`を含む（**署名検証が`gpg`を起動する**）
14. `git status`に`--no-optional-locks`が**optionとして解釈される位置に**無い、
    または`-v`／`--verbose`がある（**`-vv`はtextconvを走らせる**）
15. `git diff`／`log`／`show`／`blame`の**subcommandの直後2語**が
    `--no-ext-diff --no-textconv`でない（順序は問わない）

**判定は行ごとに行う。**`shlex`は改行を空白として扱うため、
`git show`と`rm -rf /`を改行で並べると1つの語列に潰れ、`rm`がcommand位置として見えない。

**この hook は tokenize 失敗を素通りさせない。**他の hook（`gh_metadata_guard.py`等）は
素通りさせる。**目的が違う。**あちらは「書き忘れを指摘する」ものであり、素通りは
指摘漏れで済む。こちらは**書き込みを止める境界**であり、解釈できない入力を通すことは
境界を開けることと同じである。

## 取れないもの

`command_line`の module docstring が挙げるものは、ここでも取れない。
alias、shell function、変数展開、`xargs`経由、`sh -c`の内側。
**config 由来の option も取れない。**guardが見るのは argv だけである。
`log.showSignature`（git config）が真なら`git log`／`git show`は option 無しで署名検証を走らせ、
`gpg.program`が起動する。`rg`は`RIPGREP_CONFIG_PATH`が指すfileから option を読むため、
そこに`--pre=<command>`があれば`rg <pattern> <file>`だけで前処理commandが走る。
**どちらも argv に現れないため、この guard では原理的に見えない**（#384 の22巡目で判明。
**config を設定して実際に起動させるところは測っていない。**read-onlyのため）。

**pathname expansion（`*`／`?`／`[...]`）と tilde expansion（`~`）も取れない。**
guardが検査するのは**展開前の語**であり、programが受け取る語ではない。
`git grep -n -[N-P]sha1sum <pattern>`はguardを通る。cwdに`-Osha1sum`という名のfileがあれば、
bashがそれへ展開し、`git grep`がpagerとして`sha1sum`を起動する
（**展開の一段は実測していない。file名を作れないため**）。
**subagentが意図して使えば「防がない」側だが、意図せず踏む形もある。**
file名は検査対象branchが持つものであり、検査subagentの統制下に無い。
**後者は[脅威モデル](../../docs/decisions/0020-inspector-readonly-by-hook.md)の「防ぐ」側の定義に当たるが、
この guard は防げていない。**塞いでいないことを欠点として記録する。
**ただし`sh`・`bash`・`xargs`は`ALLOWED_PROGRAMS`に無いため、command位置に現れれば拒否される。**
残るのは、許可した program 自身が持つ書き込み経路である。

**`ALLOWED_PROGRAMS`へ入れてよいのは、option を含めても外部 command を起動せず
file を書かないものだけである。**「単体では file を書けない」では足りない
（`sort -o`、`uniq out`、`sed -i`、`tee`は書き込み経路を持つので入れない。
`rg --pre`と`git`の外部 helper は**program 名だけを見ても分からない**ので、
option 側でも拒否する）。**program 名の allowlist は、option の検査と対で使う。**

**program は名前で照合する。**`/`を含む語は拒否する（`./git`、`/tmp/cat`）。
basename だけで照合すると、名前が一致する任意の実行 file を起動できる。
"""

import json
import re
import sys

import command_line

# command位置に現れてよいprogram。
# **option を含めても外部 command を起動せず file を書かないものだけを入れる。**
# 入れた後も、その program が持つ危険な option は下の `DENIED_*` で個別に拒否する。
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
        # **`version`を許す。**この file の判定はすべて git の version 依存の観測であり、
        # 検査 session 側で前提を記録できるようにする。
        # **`git --version`は許可していない global option として拒否される。**`git version`と書く。
        "version",
    }
)

# `git`のsubcommandより前に来るglobal option。値を取るものは次の語も読み飛ばす。
# **allowlistである。**「危険なものを列挙して拒否し、残りは読み飛ばす」形は、
# **列挙に無いglobal optionが値を取ると、guardが読むsubcommandとgitが実行するsubcommandをずらす。**
# `git --super-prefix rev-parse submodule--helper x`では、guardが`rev-parse`を、
# gitが`submodule--helper`を subcommand として読む（実測）。
# **subcommand allowlistも必須flagも、この読みの上に乗っている。**
GIT_GLOBAL_OPTIONS_WITH_VALUE = frozenset({"-C", "--git-dir", "--work-tree", "--namespace"})
GIT_ALLOWED_GLOBAL_OPTIONS_WITHOUT_VALUE = frozenset(
    {"--no-pager", "--no-optional-locks", "--no-replace-objects", "--bare", "--literal-pathspecs"}
)

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
DENIED_GIT_SUBCOMMAND_OPTIONS = (
    "-O",
    "--open-files-in-pager",
    "--output",
    # **外部 helper を明示的に起動する option。**helper の command は git config
    # （`diff.external`、`diff.<drv>.textconv`、`filter.<drv>.smudge`）にあり、
    # **任意の command である。file を書ける。**
    "--ext-diff",
    "--textconv",
    "--filters",
)

# **外部 helper は option 無しでも走る。**上の拒否だけでは閉じない。実測した。
#
# | 経路 | `diff.external` | `diff.<drv>.textconv` |
# |---|---|---|
# | `git diff` | **走る** | **走る** |
# | `git show`／`git log -p`／`git blame` | 走らない | **走る** |
# | 他の許可 subcommand | 走らない | 走らない |
#
# **`--no-ext-diff`は textconv を止めない。**両方を要求する。
# 4 subcommand すべてが両 option を受け付けることも実測で確かめた。
GIT_HELPER_SUBCOMMANDS = frozenset({"diff", "show", "log", "blame"})
REQUIRED_GIT_HELPER_OPTIONS = ("--no-ext-diff", "--no-textconv")

# `git status`は既定でindexをrefreshし、**更新したindexをdiskへ書く**
# （https://git-scm.com/docs/git-status）。読み取り専用にするには
# **global option**の`--no-optional-locks`が要る。`git status --no-optional-locks`
# （subcommandの後ろ）はgitが受け付ける位置ではないため、globals側で見る。
GIT_STATUS_REQUIRED_GLOBAL = "--no-optional-locks"

# **`rg`が持つ、任意commandを起動するoption。**
#
# - `--pre COMMAND` — 検索対象ごとに`COMMAND`を実行する。`--pre=COMMAND`の形もある
# - `--hostname-bin COMMAND` — hostnameを得るために`COMMAND`を実行する
#
# **`--pre-glob`は当たらない。**`_matches_option`の long option 側は完全一致と`=`付きだけを見る
# （short optionの束ね判定は`--`始まりの語へは当たらない）。
# `-z`／`--search-zip`は、対象fileの拡張子に応じて**外部のdecompressor**
# （`gzip`／`xz`／`zstd`等）を起動する。**どれも`ALLOWED_PROGRAMS`に無い。**
#
# **根拠の水準が`--pre`と違う。**`rg --help`で確認できるのは option の存在だけであり
# （`-z, --search-zip`）、`--pre=COMMAND`のように help の表記が command 実行を含意しない。
# **外部processの起動は測っていない。**ripgrepの文書化された挙動に依っている。
# **拒否側へ倒す判断である。**
#
# **`rg`の短縮綴りは対象にしていない。**`rg --sear`は`unrecognized flag`になる（実測）。
# gitの`parse-options`と違い、短縮を受け付けない。
DENIED_RG_OPTIONS = ("--pre", "--hostname-bin", "--search-zip", "-z")

# **`git <cmd> --help`は`git help <cmd>`へ書き換わる。**書き換わるのは
# **global optionを剥いだ後のbuiltin引数列の先頭**に`--help`が来たとき、すなわち
# `git <global...> <cmd> --help`の形である。`git rev-parse HEAD --help`は書き換わらない。
# **それでも位置に依らず拒否する。**位置の判定を1つ増やすより、使わせない方が単純である。
# `git version --help`と`git --no-optional-locks status --help`で
# man ページが出力されることを実測した。**guardは`version`／`status`を見ているのに、
# gitが実行するのは`help`であり、外部viewer（`man`）が起動する。**
# **短縮綴りは対象にしていない。**`git help`への書き換えは完全一致でしか起きず、
# `git version --hel`／`git grep --hel`は git 自身が`unknown option`で落とす（実測）。
DENIED_GIT_HELP_OPTIONS = ("--help",)

# **短縮綴りでも拒否するoption。**parse-optionsを通すsubcommandは短縮を受理する。
# **subcommandを数え上げて限定しない。**限定すると、数え漏れがそのまま穴になる。
# 実際に`cat-file`だけへ限定していたとき、`git grep --open-files-in-pag=sha1sum`で
# **`ALLOWED_PROGRAMS`に無い`sha1sum`が実行された**（実測）。`git grep --textc`も通っていた。
#
# **`--filters`の短縮拒否だけは`cat-file`へ限定する。**`--filter`は`rev-list`の正当な
# 読み取り専用optionであり、`--filters`の短縮ではない。全体へ広げると
# `git rev-list --objects --filter=blob:none`を巻き添えにする（実測）。
# **`git log --filter=...`はgit 2.34.1では`unrecognized argument`になる**（実測）。
# 限定の根拠は`rev-list`である。
#
# **巻き添えを1つ引き受けている。**`--text`（binaryをtextとして扱う）は`--textconv`の
# 前方一致に当たるため拒否される。**検査に要らないため引き受ける。**
DENIED_GIT_OPTIONS_WITH_ABBREVIATION = (
    "--textconv",
    "--ext-diff",
    "--open-files-in-pager",
    "--output",
    # **署名検証は`gpg.program`（既定`gpg`）を起動する。**`diff.external`と同じ
    # 「configが任意commandを指す」形である。**実測で`gpg`が`~/.gnupg`を作った。**
    "--show-signature",
)

# **pretty formatの`%G*`も署名検証を走らせる。**`--show-signature`を拒否するだけでは閉じない。
# `git log --format=%GK`で`gpg`が起動することを実測した。`%G?`／`%GS`／`%GK`は同じ経路である。
# **`%G`を含む語を拒否する。**`--format=`／`--pretty=`の綴りを数え上げない。
GIT_SIGNATURE_FORMAT_MARKER = "%G"
# **`git status -vv`はtextconvを走らせる**（実測）。`-v`は走らせない。
# **`status`は`--no-ext-diff --no-textconv`を受理しない**（実測）ため、打ち消す形が無い。
# **verboseそのものを拒否する。**`-v`はcluster判定に掛かるため`-sv`のような束ねも落ちる。
# **`diff.external`は`-vv`でも走らなかった**（実測）。走ったのはtextconvだけである。
DENIED_GIT_OPTIONS_BY_SUBCOMMAND = {
    "cat-file": ("--filters",),
    "status": ("--verbose", "-v"),
}

# command位置に現れる環境変数の代入。`command_line`は後続commandへ透過させるが、
# **`GIT_EXTERNAL_DIFF=rm git show`のように、環境変数だけで任意commandを起動できる。**
# ここでは透過させず拒否する。
#
# **command位置の語だけに当てる。**全語へ当てると`grep FOO=bar file`や
# `git diff -- file=1`のような読み取り専用commandまで落ちる。
# 判定には`command_line.command_starts`が返す「読み飛ばした前置語」を使う。
ASSIGNMENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")

# 語の中に現れたら拒否するshell metacharacter。
#
# **`shlex.split`は空白でしか語を切らない。**`cat a>b`も`cat a;rm -rf /`も1語
# （`a>b`／`a;rm`）になり、**`>`や`rm`がcommand位置として見えない。**
# そのため、これらを含む語をすべて拒否する。
#
# **単独の区切り語も拒否する**（2026-09-11）。以前は`SEPARATORS`として単独で現れた語を
# 除外していたが、**`shlex`は引用符を剥いだ後の語を返すため、bashが literal な引数として
# 渡す語と区別が付かない。**`rg <pattern> { cat --pre <任意のcommand> <file>`では、
# `command_line.invocations`が`{`で invocation を切り、**その先が無検査になった。**
# **`ALLOWED_PROGRAMS`に無い program が実際に起動することを実測した。**
# pipe が使えなくなるが、**判定が`command_line`の語り分けに依存しなくなる。**
#
# **`grep "=>"`のような正当な使い方も拒否する。**誤検知を承知で採る。
# 両 agent は`Grep` toolを持っており、検索はそちらで足りる。
# 境界を緩めるより、代替がある側を止める。
SHELL_METACHARACTERS = frozenset(">|;&()`$<{}")

# `shlex`は改行も空白として扱うため、改行で区切られたcommandが1つの語列に潰れる。
# **`git show\nrm -rf /`が`git show rm -rf /`に見える。**行ごとに分けて判定する。
# **`\r`では割らない。**bashはCRをcommand終端として扱わず、語の中のただの文字にする。
# 割ると**guardには2 command、bashには1 command**に見え、2行目の語列が1行目のprogramの
# 追加引数として実行されるのに、guardはそれを検査しない。
# **CRは`_check_line`の冒頭で、tokenize前に拒否する。**`shlex.whitespace`は`' \t\r\n'`であり、
# CRを空白として切るため、語の中には残らない。metacharacter検査では捕まえられない。
LINE_SPLIT_RE = re.compile(r"\n+")


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
        name = token.split("=", 1)[0]
        if name in GIT_GLOBAL_OPTIONS_WITH_VALUE:
            # `--git-dir=x`は1語、`--git-dir x`は2語である。
            index += 1 if "=" in token else 2
            continue
        if name in GIT_ALLOWED_GLOBAL_OPTIONS_WITHOUT_VALUE:
            index += 1
            continue
        # **許可していないglobal optionは、値を取るかどうかが分からない。**
        # 読み飛ばし方を決められないため、subcommandを`None`にして拒否させる。
        return args, None, []
    return args, None, []


def _global_option_names(globals_):
    """global部のうち、**optionとして解釈される語だけ**を返す。

    `globals_`は値も含む列である。`git --namespace --no-optional-locks status`では
    `--no-optional-locks`がnamespaceの**値**であり、statusへ届かない。
    **集合検査のままでは「在る」と誤判定する**（実測）。
    """
    names = []
    index = 0
    while index < len(globals_):
        token = globals_[index]
        name = token.split("=", 1)[0]
        names.append(name)
        if name in GIT_GLOBAL_OPTIONS_WITH_VALUE and "=" not in token:
            index += 2
            continue
        index += 1
    return names


def _matches_option(token, option):
    """語がoptionに当たるか。`--output=f`と、短いoptionの`-Ofoo`の形も拾う。

    **short optionは束ねられる。**`git grep -nOless`は`-n -O less`である。
    `token.startswith("-O")`だけを見ていたとき、**`-nOzzz`が素通りし、
    gitがpagerとして`zzz`をexecしようとした**（実測）。
    そのため**cluster内に文字が現れたら拒否する。**

    **過剰に拒否する。**`-SfooO`のように、値の中へ同じ文字が入る形も落ちる。
    **引き受ける。**数え上げで綴りを追う形に戻るより単純である。
    """
    if token == option or token.startswith(option + "="):
        return True
    if not (len(option) == 2 and option.startswith("-") and not option.startswith("--")):
        return False
    if token.startswith("--") or not token.startswith("-"):
        return False
    # `-nOless`のような束ね。**clusterのどこかに文字があれば拒否する。**
    return option[1] in token[1:]


def _matches_option_or_abbreviation(token, option):
    """`_matches_option`に加えて、**長いoptionの短縮綴り**も拾う。

    **parse-optionsを通すsubcommandにだけ当てる。**`git cat-file --te`は
    `--textconv`として受理される（実測）。**完全一致だけを見ると抜けられる。**
    """
    if _matches_option(token, option):
        return True
    name = token.split("=", 1)[0]
    return (
        option.startswith("--")
        and name.startswith("--")
        and len(name) >= 3
        and option.startswith(name)
    )


PREFIX = "検査 subagent の Bash は読み取り専用である。"


def _check_line(line):
    """1行分の拒否理由を返す。問題が無ければ`None`。"""
    # **CRはtokenize前に見る。**`shlex`はCRを空白として切るため語の中に残らないが、
    # **bashはCRを終端として扱わず、語の中のただの文字にする。**
    # `rg x f<CR>cat --pre sha1sum f`は、guardには2 command、bashには1 commandに見え、
    # **後半がrgの引数として実行される。**`{`や`!`と同じ構造である。
    if "\r" in line:
        return (
            f"{PREFIX}"
            " 復帰文字（CR）を含むため拒否した。"
            "**bash は CR を command の終端として扱わないが、`shlex` は空白として切る。**"
            " 通すと、CR の後ろの語列が前の command の引数として実行される。"
        )
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
        # **`command_line`が区切りとして扱う語は、metacharacterを含まなくても拒否する。**
        # `!`は`SEPARATORS`にあるが`SHELL_METACHARACTERS`の文字を1つも含まない。
        # そのため`rg <pattern> ! cat --pre sha1sum <file>`で invocation が切れ、
        # **その先の option 検査が届かず、`sha1sum`が実際に起動した**（実測）。
        # **この形で書けば、`command_line`へ区切りが増えても穴にならない。**
        # **ただし`command_line`への依存が消えたわけではない。**program allowlistは
        # `command_starts`、gitとrgのoption検査は`invocations`を通る。
        # **それらが呼び出しを拾い損ねれば、option検査は一度も走らない。**
        if token in command_line.SEPARATORS:
            return (
                f"{PREFIX}"
                f" 区切りとして扱われる語のため拒否した: {token!r}。"
                "**`shlex`は引用符を剥いだ後の語を返すため、bash が引数として渡す語と"
                "区別が付かない。**通すと、その先の option 検査が届かなくなる。"
                " **1 行に 1 command を書く。**"
            )
        found = sorted(SHELL_METACHARACTERS.intersection(token))
        if found:
            return (
                f"{PREFIX}"
                f" shell metacharacter（{''.join(found)}）を含む語のため拒否した: {token!r}。"
                "**`shlex`は空白でしか語を切らないため、`cat a>b`や`cat a;rm -rf /`は"
                "1語になり、redirect や次の command が command 位置として見えない。**"
                " 検索なら `Grep` tool を使う。"
            )

    for skipped, program in command_line.command_starts(line):
        # **前置語は`command_line`が透過させるが、ここでは透過させない。**
        # `sudo cat /etc/shadow`は`cat`だけを見ると allowlist を通ってしまう。
        # `sudo`は実行の権限を変え、`exec`／`command`／`nohup`／`time`も
        # 「allowlist に無い program を呼ぶための足場」になりうる。
        # **1 つも許さない。**`env`も含めて拒否する（下記）。
        for token in skipped:
            if ASSIGNMENT_RE.match(token):
                return (
                    f"{PREFIX}"
                    f" command 位置の環境変数代入のため拒否した: {token!r}。"
                    "**`GIT_EXTERNAL_DIFF=rm git show` のように、"
                    "環境変数だけで任意 command を起動できる。**"
                    " 引数の中の `FOO=bar` は対象ではない。"
                )
            return (
                f"{PREFIX}"
                f" command 位置の前置語のため拒否した: {token!r}。"
                "**`sudo cat x` のように、allowlist にある program でも"
                "前置語が実行の権限や解決先を変える。**"
                " `env` も許さない。環境変数の代入自体を拒否しているため、使い道が無い。"
            )
        if program is None:
            continue
        if "/" in program:
            # **basename で照合しない。**`./git`や`/tmp/cat`は名前が一致するだけの
            # 別の実行 file であり、allowlist の前提（名前が実体を決める）が崩れる。
            # **絶対 path を許す必要は無い。**`PATH`上の program だけで検査は足りる。
            return (
                f"{PREFIX}"
                f" path を含む program のため拒否した: {program!r}。"
                "**`./git`や`/tmp/cat`は、名前が allowlist と一致するだけの"
                "別の実行 file である。**program 名だけを書く。"
            )
        name = program
        if name not in ALLOWED_PROGRAMS:
            return (
                f"{PREFIX}"
                f" 許可していない program のため拒否した: {name!r}。"
                f" 許可しているのは {', '.join(sorted(ALLOWED_PROGRAMS))} だけである。"
                " 検査に本当に必要なら、ADR-0020 を更新して allowlist を広げる。"
            )

    for args in command_line.invocations(line, "rg"):
        for token in args:
            for option in DENIED_RG_OPTIONS:
                if _matches_option(token, option):
                    return (
                        f"{PREFIX}"
                        f" rg の option {token!r} が {option!r} に当たるため拒否した。"
                        "**`--pre`／`--hostname-bin` は任意の command を、"
                        "`-z`／`--search-zip` は外部の decompressor を起動する。**"
                    )

    for args in command_line.invocations(line, "git"):
        globals_, subcommand, rest = _split_git_args(args)
        for token in globals_:
            for option in DENIED_GIT_GLOBAL_OPTIONS:
                if _matches_option(token, option):
                    return (
                        f"{PREFIX}"
                        f" git の global option {token!r} が {option!r} に当たるため拒否した。"
                        " subcommand が読み取り専用でも、この option は"
                        " config 経由などで任意の command を実行できる。"
                        "**short option は束ねを見るため、cluster に文字が入っていれば当たる。**"
                    )
        for token in rest:
            for option in DENIED_GIT_SUBCOMMAND_OPTIONS:
                if _matches_option(token, option):
                    return (
                        f"{PREFIX}"
                        f" git の option {token!r} が {option!r} に当たるため拒否した。"
                        " subcommand が読み取り専用でも、この option は"
                        " command を実行するか file を書く。"
                        "**short option は束ねを見るため、cluster に文字が入っていれば当たる**"
                        "（`git log -GFOO` は値の中の `O` で `-O` に当たる。巻き添えである）。"
                    )
        # **`--help`は位置に依らず拒否する。**argv[1]にあると`git help <cmd>`へ書き換わり、
        # guardが読んだsubcommandではなく`help`が実行され、外部viewerが起動する。
        for token in args:
            for option in DENIED_GIT_HELP_OPTIONS:
                if _matches_option(token, option):
                    return (
                        f"{PREFIX}"
                        f" git の {token!r} は `git help <subcommand>` へ書き換わり、"
                        " 外部 viewer（`man`）を起動するため拒否した。"
                        "**guard が読む subcommand と、git が実行する subcommand がずれる。**"
                    )
        for token in rest:
            if GIT_SIGNATURE_FORMAT_MARKER in token:
                return (
                    f"{PREFIX}"
                    f" git の {token!r} は pretty format の `%G*` を含む。"
                    "**署名検証が走り、`gpg.program`（既定 `gpg`）が起動する。**"
                    " 実測では `gpg` が `~/.gnupg` を作った。"
                    " 署名の確認が要るなら、**guard の外で人が行う。**"
                )
        abbreviated = DENIED_GIT_OPTIONS_WITH_ABBREVIATION + tuple(
            DENIED_GIT_OPTIONS_BY_SUBCOMMAND.get(subcommand or "", ())
        )
        for token in rest:
            for option in abbreviated:
                if _matches_option_or_abbreviation(token, option):
                    return (
                        f"{PREFIX}"
                        f" git の option {token!r} が、拒否している {option!r} の"
                        " 前方一致に当たるため拒否した。"
                        "**parse-options を通す subcommand は短縮綴りを受理して実行する**"
                        "（完全一致だけを見ていたとき、"
                        "`git grep --open-files-in-pag=sha1sum` で `sha1sum` が実際に走った）。"
                        f"**{token!r} が {option!r} の短縮でない正当な option の場合もある**"
                        "（`--text` は `--textconv` の前方一致に当たるが別の option である）。"
                        " その場合は巻き添えであり、**代わりの形を使う。**"
                    )
        if subcommand is None:
            unlisted = [
                token for token in args
                if token.startswith("-")
                and token.split("=", 1)[0] not in GIT_GLOBAL_OPTIONS_WITH_VALUE
                and token.split("=", 1)[0] not in GIT_ALLOWED_GLOBAL_OPTIONS_WITHOUT_VALUE
            ]
            if unlisted:
                allowed = sorted(
                    GIT_GLOBAL_OPTIONS_WITH_VALUE | GIT_ALLOWED_GLOBAL_OPTIONS_WITHOUT_VALUE
                )
                return (
                    f"{PREFIX}"
                    f" 許可していない git の global option のため拒否した: {unlisted[0]!r}。"
                    f" 許可しているのは {', '.join(allowed)} だけである。"
                    "**列挙に無い global option は値を取るかどうかが分からず、"
                    "guard と git で subcommand の読みがずれる。**"
                )
            return f"{PREFIX} `git` に subcommand が無いため拒否した。"
        if subcommand not in GIT_READONLY_SUBCOMMANDS:
            return (
                f"{PREFIX}"
                f" 読み取り専用でない git subcommand のため拒否した: {subcommand!r}。"
                f" 許可しているのは {', '.join(sorted(GIT_READONLY_SUBCOMMANDS))} だけである。"
            )
        if (
            subcommand == "status"
            and GIT_STATUS_REQUIRED_GLOBAL not in _global_option_names(globals_)
        ):
            return (
                f"{PREFIX}"
                f" `git status` は既定で index を refresh し `.git/index` を書くため、"
                f" `git {GIT_STATUS_REQUIRED_GLOBAL} status` の形でだけ許す。"
                "**global option であり、subcommand の後ろでは効かない。**"
            )
        if subcommand in GIT_HELPER_SUBCOMMANDS:
            # **subcommandの直後2語に固定する。**「語として在るか」だけを見る形は
            # 2通りで抜けられることを実測した。
            #
            # - `git diff -- f --no-ext-diff --no-textconv` — `--`より後ろはpathspecであり、
            #   gitはflagとして扱わない。**helperが実際に走った**
            # - `git log -S --no-ext-diff --no-textconv -p` — `-S`が次の語を検索文字列として
            #   飲む。**flagはgitへ届かない**
            #
            # **値を取るoptionを数え上げる形にしない。**`-S`／`-G`／`--grep`／`--author`など、
            # 数え漏れがそのまま穴になる。**位置で決めれば、前に何も置けない。**
            # **後ろへ`--ext-diff`／`--textconv`を置いて打ち消す形は、この規則では止まらない。**
            # `DENIED_GIT_SUBCOMMAND_OPTIONS`と`DENIED_GIT_OPTIONS_WITH_ABBREVIATION`が担う。
            # **片方だけでは閉じない。**
            # subcommand自身は値を取らないため、直後の2語は必ずflagとして解釈される。
            head_two = rest[1:3]
            if sorted(head_two) != sorted(REQUIRED_GIT_HELPER_OPTIONS):
                return (
                    f"{PREFIX}"
                    f" `git {subcommand}` は option 無しでも git config の外部 helper"
                    f"（`diff.external`／`textconv`）を実行するため、"
                    f" `git {subcommand} {' '.join(REQUIRED_GIT_HELPER_OPTIONS)} ...` の形で"
                    " **subcommand の直後2語に**明示することを要求する（順序は問わない）。"
                    "**`--no-ext-diff` だけでは textconv を止められない。**"
                    "**位置を決めているのは、`--` より後ろや `-S` のように次の語を値として飲む"
                    "option の後ろへ置くと、flag が git へ届かないためである。**"
                    f" いま直後にあるのは"
                    f" {' '.join(repr(token) for token in head_two) if head_two else '（無し）'}"
                    " である。"
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
