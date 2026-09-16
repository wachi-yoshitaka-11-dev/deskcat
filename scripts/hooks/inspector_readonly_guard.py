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
2. 引用符が閉じていない、または`shlex`がcommandを語へ分けられない（**fail closed**。下記）
3. **引用の外に** shell metacharacter（`>;&()`` ` ``$<{}!`）がある
   （**語ではなく生の行の文字へ当てる。**#396）
4. **ダブルクォートの中に展開構文（`$`／`` ` ``）がある。**`"$(id)"`はクォートされていても実行される
5. `||`がある（**pipeではない。**前が失敗したときに後ろが走る）
6. `command_line.SEPARATORS`に一致する語がある（**クォートしていても拒否する。**#384）
7. command位置の前置語（`sudo`／`env`／`exec`など）か、環境変数代入がある
8. command位置のprogram語に`/`が含まれる
9. command位置のprogramが`ALLOWED_PROGRAMS`に無い
10. `rg`のoptionが`DENIED_RG_OPTIONS`に当たる（`--pre`／`--hostname-bin`／`-z`／`--search-zip`。
   **short optionの束ね（`-nz`）も見る**）
11. `git`のoptionが拒否一覧に当たる（完全一致・`=`付き・**short optionの束ね**）
12. `git`の`--help`がある（位置に依らない）
13. `git`の語が pretty format の`%G*`を含む（**署名検証が`gpg`を起動する**）
14. `git`のoptionが拒否一覧の**短縮綴り**に当たる
15. `git`に許可していないglobal optionがある（**allowlistである**）
16. `git`の subcommand が`GIT_READONLY_SUBCOMMANDS`に無い
17. `git status`に`--no-optional-locks`が**optionとして解釈される位置に**無い
    （`status`の`-v`／`--verbose`は項目14で拒否する）
18. `git diff`／`log`／`show`／`blame`の**subcommandの直後2語**が
    `--no-ext-diff --no-textconv`でない（順序は問わない）

**判定は行ごとに行う。**`shlex`は改行を空白として扱うため、
`git show`と`rm -rf /`を改行で並べると1つの語列に潰れ、`rm`がcommand位置として見えない。

**行はさらに pipe で区間へ割り、区間ごとに判定する**（#396）。`cat f | wc -l`は2つのcommandであり、
**read-onlyはそれぞれの区間で決まる。**`cat f | tee out.txt`は2つ目の区間で落ちる。
**以前は`|`を含む語をそれ自体で拒否していた。**効いたのは2つの側である（2026-09-14に旧版で実測）。
**1つはpatternである。**`rg -c '^\|' <file>`も`grep -c '^|' <file>`も拒否されていた。
**この repository の正本はMarkdownの表であり、区切り文字が`|`である。**
`docs/hardware/tbd-register.md`は519行で最長行が6778字あり、
**表の構造についてpatternを書けなかった。**
**もう1つは行の窓である。**`head -n 752 <file>`も`tail -n +735 <file>`も単体では通るが、
**その2つを繋いで範囲を取り出す手段が無かった**（`sed`はallowlistに無い）。
**桁方向は`cut -c`で取れていた。**取れなかったのは行方向である。

**上の一覧のうち7以降は、語頭の`#`から行末までを見ない。**
**pipeもコメントの中では割らない**（bashも割らない。#396で実測した）。
**ただしコメントの中の`|`は、1〜6の側で拒否する**（`;`と同じ扱いである）。`command_line.segments`が
bashと同じくコメントとして落とすためである（#389。[ADR-0020](../../docs/decisions/0020-inspector-readonly-by-hook.md)
の決定3）。**bashも実行しないため、allowlistの外へ出る経路は増えない。**
**1〜6は生の行と、その語へ当たる。**`tokenize`はコメントを落とす前の行へ掛けており、
CRの検査、語へ分けられない場合、引用の外のmetacharacterと
ダブルクォートの中の展開、区切り語の拒否は、**コメントの中に書いても効く**（`… HEAD # ; rm -rf /`は今も拒否する）。
**境界は7以降でだけ緩む。**

**この hook は tokenize 失敗を素通りさせない。**他の hook（`gh_metadata_guard.py`等）は
素通りさせる。**目的が違う。**あちらは「書き忘れを指摘する」ものであり、素通りは
指摘漏れで済む。こちらは**書き込みを止める境界**であり、解釈できない入力を通すことは
境界を開けることと同じである。

## 取れないもの

`command_line`の module docstring が挙げるものは、ここでも取れない。
alias、shell function、変数展開、`xargs`経由、`sh -c`の内側。
**config 由来の option も取れない。**guardが見るのは argv だけである。
`log.showSignature`（git config）が真なら、**guardが許可する形でも**署名検証が走り、
`gpg.program`が起動する。`rg`は`RIPGREP_CONFIG_PATH`が指すfileから option を読むため、
そこに`--pre=<command>`があれば`rg <pattern> <file>`だけで前処理commandが走る。
**どちらも argv に現れないため、この guard では原理的に見えない**（#384 の22巡目で判明）。
**`log.showSignature`の側は実測した。**`gpg.program`をscriptへ差し替え、有無で比べた。
**`RIPGREP_CONFIG_PATH`の側は測っていない。**

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
#
# **巻き添えがある。**pretty formatでない使い方も落ちる。
# `git grep -n %G -- docs`や`git log ... -G %G`のような、`%G`を検索語やpathspecに含む形である（実測）。
# **引き受ける。**綴りを数え上げる形に戻るより単純であり、検索は`Grep` toolで足りる。
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

# 引用の外に置けない文字。**bashがcommandの構造として読む。**
#
# **判定は語ではなく、生の行の文字へ当てる**（#396）。`shlex`は引用符を剥いだ後の語を
# 返すため、**語を見ても引用されていたかが分からない。**以前はそのため語の中に1文字でも
# あれば拒否しており、**クォートしたpatternまで落としていた**（`rg -n 'a|b' f`）。
# `_quoting_reason`が引用の内外を追い、**引用の外にあるものだけを拒否する。**
#
# **`|`は入れない。**pipeとして`_pipe_stages`が区間へ割り、各区間の先頭programを
# allowlistで検査する（#396。[ADR-0020](../../docs/decisions/0020-inspector-readonly-by-hook.md)
# の決定3）。**read-onlyは各区間で保たれる。**
#
# **`!`を入れている。**`>`や`;`のようにbashのcommand構造を作る文字ではないが、
# `SEPARATORS`にあるため`command_line`が区切りとして扱う。
# `rg <pattern> ! cat --pre sha1sum <file>`で invocation が切れて
# **その先のoption検査が届かなかった**（2026-09-11に実測）。
# **引用の中の`!`は、区切り語そのものの拒否（`_check_stage`）が受け持つ。**
FORBIDDEN_OUTSIDE_QUOTES = frozenset(">;&()`$<{}!")

# ダブルクォートの中でも展開される文字。**「クォートされているか」では足りない。**
# `"$(id)"`はクォートされているが実行される（2026-09-14に実測。
# `'$(id -u)'`は文字列のまま、`"$(id -u)"`は`1000`、`"${HOME}"`はhome directoryのpathへ置き換わる）。
# **シングルクォートの中は展開されないため、ここは見ない。**
EXPANDS_INSIDE_DOUBLE_QUOTES = frozenset("$`")

# 引用の状態。`_scan`が文字ごとに返す。
OUTSIDE_QUOTES = 0
SINGLE_QUOTED = 1
DOUBLE_QUOTED = 2

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


def _scan(text):
    """文字ごとに`(文字, 引用の状態, escapeされたか, 元のindex)`を返す。**閉じない引用符では`None`。**

    **引用符そのものは返さない。**返すのは引用の中身と、引用の外の文字である。

    escapeの扱いはbashに合わせる。**引用の外の`\\`は次の1文字をliteralにする。**
    **ダブルクォートの中では`$`／`` ` ``／`"`／`\\`／改行だけがescapeできる**
    （`"\\d"`は2文字のまま残る）。**シングルクォートの中にescapeは無い。**

    **閉じない引用符を`None`で返すのは、この先の判定を止めるためである。**
    `shlex`も同じ入力で失敗するため、`_check_stage`がfail closedで拒否する。
    """
    scanned = []
    state = OUTSIDE_QUOTES
    index = 0
    length = len(text)
    while index < length:
        char = text[index]
        if state == OUTSIDE_QUOTES:
            if char == "\\" and index + 1 < length:
                scanned.append((text[index + 1], state, True, index))
                index += 2
                continue
            if char == "'":
                state = SINGLE_QUOTED
                index += 1
                continue
            if char == '"':
                state = DOUBLE_QUOTED
                index += 1
                continue
        elif state == SINGLE_QUOTED:
            if char == "'":
                state = OUTSIDE_QUOTES
                index += 1
                continue
        else:
            if char == "\\" and index + 1 < length and text[index + 1] in '$`"\\\n':
                scanned.append((text[index + 1], state, True, index))
                index += 2
                continue
            if char == '"':
                state = OUTSIDE_QUOTES
                index += 1
                continue
        scanned.append((char, state, False, index))
        index += 1
    if state != OUTSIDE_QUOTES:
        return None
    return scanned


def _comment_start(scanned):
    """語頭の`#`が現れた位置（scanのindex）を返す。無ければ末尾の位置。

    **bashと同じ扱いである。**`cat f # note a | b`の`|`はコメントの中であり、
    **bashはpipeとして読まない。**`_pipe_stages`がここで区間へ割ると、
    コメントの後半が独立したcommandとして program allowlist と option 検査へ入り、
    **bashが実行しない語で拒否することになる**（2026-09-14に実測）。

    **コメントの中身を見なくなるわけではない。**引用の外のmetacharacterと区切り語の拒否
    （一覧の1〜6）は、`tokenize`が生の行へ掛かるため今も効く。
    緩むのは`command_starts`／`invocations`を通る段だけである。
    """
    breaks = " \t;&|"
    for index, entry in enumerate(scanned):
        char, state, escaped, _position = entry
        if char != "#" or state != OUTSIDE_QUOTES or escaped:
            continue
        if index == 0:
            return index
        previous, previous_state, previous_escaped, _ = scanned[index - 1]
        if (previous_state == OUTSIDE_QUOTES and not previous_escaped
                and previous in breaks):
            return index
    return len(scanned)


def _pipe_stages(line, scanned):
    """引用の外の`|`で区間へ割り、`(生の部分文字列, その走査結果)`の list を返す。

    **`||`では割らない。**論理ORであり、pipeとは別の構造である。この変更の範囲外として
    `_quoting_reason`が拒否する（`|`が引用の外に残るため）。

    **区間は生の部分文字列で返す。**引用符を落とした文字列を渡すと、
    `shlex`がそこを別の語へ割る（`rg 'a b' f`が3語ではなく4語になる）。
    """
    boundaries = []
    index = 0
    # **コメントの中の`|`では割らない。**bashはpipeとして読まない。
    comment = _comment_start(scanned)
    while index < comment:
        char, state, escaped, position = scanned[index]
        if char == "|" and state == OUTSIDE_QUOTES and not escaped:
            following = scanned[index + 1] if index + 1 < len(scanned) else None
            if following and following[0] == "|" and following[1] == OUTSIDE_QUOTES:
                index += 2
                continue
            boundaries.append((index, position))
        index += 1
    if not boundaries:
        return [(line, scanned)]
    stages = []
    scan_start = 0
    text_start = 0
    for scan_end, position in boundaries + [(len(scanned), len(line))]:
        stages.append((line[text_start:position], scanned[scan_start:scan_end]))
        scan_start = scan_end + 1
        text_start = position + 1
    return stages


def _quoting_reason(stage):
    """引用の外のmetacharacterと、ダブルクォートの中の展開を拒否する。**理由か`None`。**

    **判定は生の文字へ当てる。**`shlex`が引用符を剥いだ後の語では、
    `rg -n 'a|b' f`の`a|b`と`rg -n a|b f`の`a|b`が区別できない。**bashは別のものとして扱う。**
    """
    comment = _comment_start(stage)
    for index, (char, state, escaped, _position) in enumerate(stage):
        if char == "|" and state == OUTSIDE_QUOTES and not escaped and index >= comment:
            # **コメントの中の`|`。**`_pipe_stages`はここで割らない（bashも割らない）。
            # **通しもしない。**コメントの中身は一覧の1〜6の対象であり、`;`と同じ扱いにする。
            # **過検出の側であり、bashが実行しない範囲である。**
            return (
                f"{PREFIX}"
                " コメントの中の `|` のため拒否した。"
                "**bash は pipe として読まないが、この guard はコメントの中も"
                "引用の外の文字として見る**（`# ; rm -rf /` と同じ扱いである）。"
                " **検査 subagent にコメントは要らない。**外して書く。"
            )
        if state == SINGLE_QUOTED:
            continue
        if state == DOUBLE_QUOTED:
            if char in EXPANDS_INSIDE_DOUBLE_QUOTES and not escaped:
                return (
                    f"{PREFIX}"
                    f" ダブルクォートの中で展開される文字のため拒否した: {char!r}。"
                    "**クォートしても実行される。**`\"$(id)\"`は`id`を起動し、"
                    "`\"${HOME}\"`は値へ置き換わる。"
                    " **展開させたくないならシングルクォートで囲む。**"
                )
            continue
        if char == "|" and not escaped:
            # **単独の`|`は`_pipe_stages`が区間の境として取り除いている。**
            # ここへ残るのは`||`だけである。**論理ORはpipeではない。**
            # `cat f || rm -rf /`の`rm`は、1つ目が失敗したときに走る。
            return (
                f"{PREFIX}"
                " `||` のため拒否した。**pipe ではない。**"
                "前の command が失敗したときに後ろが走る。"
                " **pipe（`|`）は区間ごとに検査して通している。**"
            )
        if char in FORBIDDEN_OUTSIDE_QUOTES:
            return (
                f"{PREFIX}"
                f" 引用の外のshell metacharacterのため拒否した: {char!r}。"
                "**bashがcommandの構造として読む。**`cat a>b`はfileへ書き、"
                "`cat a;rm -rf /`は次のcommandを実行する。"
                " **引数として渡したいならクォートする**"
                "（`rg -n 'a|b' <file>`は通る）。"
            )
    return None


def _check_line(line):
    """1行分の拒否理由を返す。問題が無ければ`None`。

    **pipeで区間へ割り、区間ごとに検査する**（#396）。`cat f | wc -l`は
    2つのcommandであり、**read-onlyはそれぞれの区間で決まる。**
    `cat f | sh`は2つ目の区間で落ちる。
    """
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
    if not line.strip():
        return None
    scanned = _scan(line)
    if scanned is None:
        # **閉じない引用符。**`shlex`も同じ入力で失敗する。fail closedで拒否する。
        return (
            f"{PREFIX}"
            f" 引用符が閉じていないため拒否した: {line!r}。"
            "**解釈できない入力を通すことは、境界を開けることと同じである。**"
        )
    for stage, stage_scan in _pipe_stages(line, scanned):
        reason = _quoting_reason(stage_scan)
        if reason is not None:
            return reason
        reason = _check_stage(stage)
        if reason is not None:
            return reason
    return None


def _check_stage(line):
    """pipeで割った1区間の拒否理由を返す。問題が無ければ`None`。

    **引用の外のmetacharacterは`_quoting_reason`が先に見ている。**ここが見るのは
    program allowlistとoption検査であり、どちらも`command_line`の語り分けを通る。
    """
    tokens = command_line.tokenize(line)
    for token in tokens:
        # **区切り語は、クォートされていても拒否する**（#384。#396でも残した）。
        # **`shlex`は引用符を剥いだ後の語を返すため、`command_line`は引用の有無を見ない。**
        # `rg <pattern> '{' cat --pre sha1sum <file>`では、bashは`{`をliteralな引数として
        # rgへ渡すのに、`invocations`は`{`でinvocationを切り、
        # **その先の`--pre`がoption検査へ一度も届かない**（2026-09-14に実測）。
        # **guardがcommandを切る位置と、bashが切る位置がずれる型そのものである。**
        # **引用の外かどうかでは判定できない。**判定するのは`command_line`の側だからである。
        if token in command_line.SEPARATORS:
            return (
                f"{PREFIX}"
                f" 区切りとして扱われる語のため拒否した: {token!r}。"
                "**クォートしても拒否する。**`shlex`は引用符を剥いだ後の語を返すため、"
                "`command_line`は bash が引数として渡す語と区別が付かない。"
                "**通すと、その先の option 検査が届かなくなる。**"
                " pattern の一部として使うなら、`rg -n 'a|b' <file>` のように"
                "**語全体を区切り語にしない形で書く。**"
            )
    if not tokens:
        if not line.strip():
            # **pipeの区間が空。**`cat f | | wc -l`のような形である。
            # **bashはsyntax errorにする。**こちらは拒否する側へ倒す。
            return (
                f"{PREFIX}"
                " pipe の区間が空のため拒否した。"
                "**`|` の前後には command が要る。**"
            )
        return (
            f"{PREFIX}"
            f" command を語へ分けられなかったため拒否した: {line!r}。"
            "**解釈できない入力を通すことは、境界を開けることと同じである。**"
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
