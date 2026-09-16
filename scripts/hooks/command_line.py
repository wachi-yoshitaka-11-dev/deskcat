#!/usr/bin/env python3
"""hookが受け取ったcommand文字列から、目的のprogramの呼び出しを取り出す。

`hooks/`配下の他の8 hookが使う。**同じ判定を各hookへ複製しない。**
**`invocations`を使うのは7 hookである。`truncation_guard.py`だけが使わず、`tokenize`／
`SEPARATORS`／`TRANSPARENT_PREFIXES`／`is_program`を直接使う。**`invocations`を変えると
7 hook、語の切り方（`tokenize`）を変えると8 hook全部の判定が動く。

**hookの入力からcommandを取り出す`command_from`も持つ。**payloadの形の検査を
各hookへ複製しないためである（#242）。

**語がcommand位置にあるかを見る。**単に`gh`という語を探すと、`echo gh pr merge`のような
引数を呼び出しと読んでしまう。実際にそれで`merge_trailer_report.py`が誤報告し、
`gh_metadata_guard.py`なら誤って拒否する（偽陽性でhookごと無効化される側の失敗である）。

**判定は字句だけで行う。**shellの意味論は再現しない。次は取れない。

- alias、shell function、変数展開経由の呼び出し
- `xargs gh ...`のように、引数をstdinから受ける経由での起動
- `sh -c '...'`の内側

**改行もcommandの区切りである。**bashは改行で区切った両方をcommandとして読むが、`shlex`は
改行を空白へ潰すため、**2行目以降の`git`／`gh`がcommand位置から消えていた**（#389）。
`segments`で区切ってから語へ分ける。**語の切り方（`tokenize`）は変えていない。**

**同じ走査で、改行以外にも3つ揃えた。**bashが落とす語頭の`#`以降、bashが繋ぐ`\`改行、
bashが実行しないheredocのbodyである。**#389が挙げたのは改行だけだが、`segments`が
引用とheredocを追う以上、この3つは同じ走査の中で決まる。**分けて別の走査にすると、
同じ文字列を2回別の規則で読むことになる。
**残る取り落ちはCONTRIBUTINGの「取り切れていないもの」にある。**

**`\r`は区切りにしない。**bashも区切りにせず、語の一部として扱う。CRLFの改行は`\n`の側で
切れるため、2行目のcommandは検出する。**ただし`shlex`は`\r`を空白として落とすため、
`git push origin develop\r`を`develop`へのpushとして読む。**bashに渡せば
`git push origin $'develop\\r'`が`fatal: invalid refspec`で落ちる形である
（実測。**この失敗そのものはtestで固定していない。**shimで`git`を置き換えているためである）。
**止める門では厳しい側に出るが、
向きは呼び出し側で変わる。**`stop_claim_guard.py`は`git push`を「やった証拠」として読むため、
**bashでは失敗するpushを証拠として数える。**これは`shlex`が`\r`を落とす以前からの性質であり、
#389では変えていない。

**`inspector_readonly_guard.py`は自分でも`\n+`で行分割する**（`LINE_SPLIT_RE`）。
**`\r`ではそちらも割らず、CRを含む行をtokenize前に拒否する**（#384。[ADR-0020](../../docs/decisions/0020-inspector-readonly-by-hook.md)）。
同hookは行へ割ってから`command_starts`を呼ぶため、**heredocの読み飛ばしはそこでは起きない**
（`segments`へ改行が渡らない）。**判定が動くのはコメントの除去だけである。**
向きは`segments`のdocstringに書いた。

**取れないものは取れないままにする。**推測で拾うと誤検知になり、誤検知はhookそのものを
無効化される側の失敗である。取り切れない範囲はCONTRIBUTINGの「hookが止めたとき」に書く。

**このfileが「実測」と書いた挙動は、2026-09-12と2026-09-14に
bash 5.1.16(1)-release／python3 3.10.12／git 2.34.1（Linux x86_64）で取得した。**
**2026-09-14の分には日付を付けてある**（`inspector_readonly_guard.py`が#384で変わった後に
測り直した分と、新しく測った分である）。**このfileの中では、日付の無い「実測」は
2026-09-12の分である。**
**この規則をこのfileの外へ当てない。**`scripts/test_hooks.py`の「実測」は#389より前の
Issueで書いたものを含み、日付の無いものが同じ日の測定だとは限らない。
**#389で足した分は同じ環境で取っており、日付をそれぞれに書いてある。**
`shlex`とbashの挙動はどちらも実装依存である。**版が変われば測り直す。**
"""

import shlex

# commandの区切り。ここより後ろは新しいcommandとして読む。
SEPARATORS = frozenset({"&&", "||", "|", ";", "&", "(", ")", "{", "}", "!"})

# 後ろのcommandへ透過する前置語。`env FOO=1 gh ...`や`sudo gh ...`を拾うため。
TRANSPARENT_PREFIXES = frozenset({"env", "sudo", "nohup", "time", "command", "exec"})


def tokenize(command):
    """command文字列を語へ分ける。分けられなければ空を返す。

    `shlex`が失敗するのは引用符が閉じていない場合である。**そのときは検査しない。**
    壊れたcommandはshell自身が落とすため、hookで二重に報告しない。
    """
    try:
        return shlex.split(command)
    except ValueError:
        return []


def is_program(token, name):
    """語が`name`の呼び出しかを判定する。`/usr/local/bin/gh`のような絶対pathも拾う。"""
    return token == name or token.rsplit("/", 1)[-1] == name


# heredocのbodyはcommandではない。**bashは実行しない。**`segments`が読み飛ばす。
# `<<<`はherestringであり、bodyを持たない。
HEREDOC_OPERATOR = "<<"
HERESTRING_OPERATOR = "<<<"

# heredocのdelimiter語が終わる文字。空白と、command区切りになりうる記号である。
# **`\r`は入れない。**bashは`\r`を語の一部として扱うため、CRLFのfileでは
# delimiterも終端行も`EOF\r`になり、そのまま照合が成立する。
DELIMITER_END = frozenset(" \t\n;&|<>()")


def _closing_quote(command, index):
    """引用符の開きから、閉じた次のindexを返す。閉じていなければ末尾を返す。

    単一引用符の中にescapeは無い。二重引用符の中では`\\`が次の1文字をescapeする。

    **閉じていない場合は末尾まで引用の中として扱う。**引用の中の改行で切らないため、
    その command は`tokenize`が失敗して検査されない形（今も検査しない形）に落ちる。
    """
    quote = command[index]
    index += 1
    length = len(command)
    while index < length:
        char = command[index]
        if quote == '"' and char == "\\" and index + 1 < length:
            index += 2
            continue
        if char == quote:
            return index + 1
        index += 1
    return length


def _heredoc_delimiter(command, index):
    """`<<`（と続く`-`）の直後からdelimiter語を読み、`(語, 次のindex)`を返す。

    `<<EOF`／`<<'EOF'`／`<<"EOF"`／`<<\\EOF`／`<< EOF`を読む。
    **引用符とbackslashを外した値が、終端行と照合する語である。**

    delimiter自身が引用符やbackslashを含む形（bashは許す）は読み違える。
    その場合は終端行が見つからないため、`_skip_heredoc_bodies`が**1文字も捨てない。**

    **空のdelimiter（`<<''`／`<<""`）はheredocとして扱わない。**bashは空行を終端として
    bodyを読むが、こちらはbodyを落とさず**commandとして検査する。**過検出の側であり、
    取り落としにはならない。
    """
    length = len(command)
    while index < length and command[index] in " \t":
        index += 1
    word = []
    while index < length and command[index] not in DELIMITER_END:
        char = command[index]
        if char in "'\"":
            end = _closing_quote(command, index)
            closed = end - 1 > index and command[end - 1] == char
            word.append(
                command[index + 1:end - 1] if closed else command[index + 1:end]
            )
            index = end
            continue
        if char == "\\" and index + 1 < length:
            word.append(command[index + 1])
            index += 2
            continue
        word.append(char)
        index += 1
    return "".join(word), index


def _skip_heredoc_bodies(command, index, pending):
    """開いているheredocのbodyを読み飛ばし、次のcommandが始まるindexを返す。

    `pending`は開いた順のdelimiterと、`<<-`かどうか（終端行の先頭tabを無視する）。

    **1つでも終端行が見つからなければ、1文字も捨てずに元の位置を返す。**bashはEOFまで
    bodyとして読むため、捨てる方がbashに近い。それでも捨てないのは、**`<<`をheredocと
    読み違えた場合に残りの行すべてが検査から消えるためである**（`echo $((1 << 2))`の`<<`は
    delimiterを`2`と読み、終端行は現れない。`(( i <<= 1 ))`ではdelimiterが`=`になる）。
    読み違えたときに門が黙るより、**終端行の無い壊れたcommandで、実行されない行を検査する側へ
    倒す。**`cat <<A <<B`でAだけ終端している場合も、**Aのbodyごと検査する**（途中まで捨てると、
    どこまで捨てたかで判定が変わる）。

    **倒し切れてはいない。**読み違えたdelimiterと同じ行が後に現れると、そこまでを落とす。
    `echo $((1 << 2))`改行`git push origin develop`改行`2`では、delimiterを`2`と読み、
    3行目の`2`が終端行に見えるため、**bashが実行するpushが検査から消える**
    （2026-09-14に実測。bashは`git push`を起動し、`2: command not found`で終わる）。
    **門が黙る側の乖離である。**CONTRIBUTINGの「取り切れていないもの」に書いた。
    """
    length = len(command)
    cursor = index
    for delimiter, strip_tabs in pending:
        found = False
        while cursor < length:
            end = command.find("\n", cursor)
            line = command[cursor:length if end == -1 else end]
            cursor = length if end == -1 else end + 1
            if (line.lstrip("\t") if strip_tabs else line) == delimiter:
                found = True
                break
        if not found:
            return index
    return cursor


# 語頭とみなす直前の文字。bashは**語頭の**`#`だけをコメントの開始として読む
# （`sed -e s/a/b/#x`の`#`は語の中であり、コメントにならない）。
# **語の区切りだと確かめた文字だけを入れる。**入れ過ぎると、コメントでない`#`から行末までを
# 捨てて**呼び出しを取り落とす**（門が黙る側である）。入れ足りない場合は、コメントの中の語を
# commandとして数える（過検出の側である）。**迷う文字は入れない。**
#
# - `{`／`}`／`!`は語の区切りではない。入れると`test ${#x} -gt 0 && git push origin develop`と
#   `echo ${x}#1 && git push …`の`push`を取り落とす
# - `)`は形で意味が変わる。`echo $(date)#x`の`#`はコメントにならないが、
#   `(echo hi)#x`の`#`はコメントになる（どちらも実測）。**字句では区別できないため入れない**
# - `(`／`<`／`>`も同じ理由で入れない
#
# **`SEPARATORS`は`shlex`の語単位の区切りであって文字単位の語境界ではない。同じ集合を使わない。**
WORD_BREAKS = frozenset(" \t;&|")


def _at_word_start(current):
    """これまで積んだ segment の末尾から、次の文字が語頭かを判定する。

    **escapeした文字の直後は語頭にしない。**`echo a\\ #b`の`a #b`は1語であり、
    bashはこの`#`をコメントにしない。
    """
    if not current:
        return True
    chunk = current[-1]
    if len(chunk) == 2 and chunk[0] == "\\":
        return False
    return chunk[-1] in WORD_BREAKS


def segments(command):
    """commandを、**引用の外の改行**で粗く分ける。

    **改行はcommandの区切りである。**`shlex`は改行を空白へ潰すため、分けずに渡すと
    2行目以降の語がcommand位置から消える（#389）。

    **1要素が1 commandだとは限らない。**`;`／`&&`／`||`／`|`では分けない。
    それらは語の側で`SEPARATORS`が見る（`_invocations_in`）。
    `cat x && git push origin develop`は1つのsegmentのままである。

    **実行されるかは判定しない。**`if false; then`改行`git push origin develop`改行`fi`は、
    条件が偽なら走らないが呼び出しとして返す。`cat x && git push`の短絡と同じ扱いであり、
    **字句だけで見る側の過検出である。止める門としては安全側に出る。**
    **証拠として読む側では安全側ではない。**`stop_claim_guard.py`は`git push`を
    「やった証拠」として読むため、**走らなかったpushを証拠として数える。向きは呼び出し側で決まる。**
    **回数も判定しない。**`for i in 1 2; do`改行`git push origin develop`改行`done`を
    bashは2回実行するが、返すのは1件である（2026-09-14に実測）。
    **呼び出し側は「何回走るか」をこの戻り値から読めない。**現在の呼び出し側はいずれも
    有無だけを見ている。
    **返すのは、bodyが独立した行にあるときだけである。**1行で書いた
    `if false; then git push origin develop; fi`は**拾わない**（`_invocations_in`）。
    落ちる語は書き方で変わる。`false;`と続けて書くと区切り語が現れず`if`で落ち、
    `false ; then`と空けると`;`で戻ってから`then`で落ちる。**どちらも`git`まで届かない。**

    切るのは**引用の外の改行だけ**である。`\\`による行継続は、bashと同じく
    `\\`と改行を消して繋ぐ（**残すと`shlex`が改行を1語として返し、引数の位置がずれる**）。
    **二重引用符の中の`\\`改行は残す。**bashは消して1行へ繋ぐ。**command位置の判定は変わらないが、
    引数の文字列は違う**（`shlex`は引用の中では`\\`を残すため、`\\`と改行の2文字になる。実測）。
    **本文を読む判定は取り落とす側へ出る**（`--body "@coderabbitai \\`改行`full review"`は
    bashでは1行に繋がるが、`coderabbit_gate.py`は拾わない。**#389より前から同じである。**
    CONTRIBUTINGの「取り切れていないもの」に書いた）。合わせないのは、**引用の中の展開を
    再現しないという方針による。**
    **引用の外で開いたheredocのbodyは落とす。**bashが実行しないものを呼び出しとして
    数えないためであり、body の中の引用符で`tokenize`が失敗して**後続のcommandまで
    検査できなくなるのを避けるためでもある**（`<<'EOF'`のbodyに`'`が1つあるだけで起きる）。
    **引用の中へ入ったbodyは落とさない**（`--body "$(cat <<'EOF' … EOF)"`）。引用ごと1語として
    残るため、**本文を読む判定（`coderabbit_gate.py`の語の照合）はそこを見る。**
    終端行が無い場合の扱いは`_skip_heredoc_bodies`が持つ。

    **「bashが実行しない」が成り立つのは、bodyを受け取る側がそれをcommandとして実行しない
    場合だけである。**`bash <<'EOF' … EOF`／`sh <<EOF`／`ssh host <<EOF`のbodyは実行される。
    **この形では落とした側が正しくない。**`bash <<'EOF'`改行`cd x ; git push origin develop`改行`EOF`は、
    bashがpushを実行するのに呼び出しとして返さない（2026-09-14に実測）。
    **落とす対象をcommandの綴りで絞ってはいない。**`cat`だけを落とす形にすると、
    別名や絶対pathで書いた`cat`を落とし損ね、bodyの引用符で`tokenize`が失敗する側へ戻る。
    **門が黙る側の乖離であり、CONTRIBUTINGの「取り切れていないもの」に書いた。**

    **語頭の`#`から行末までは落とす。**bashと同じ扱いである（残すと
    `git push origin main # develop`の`develop`をrefspecとして数える）。
    **`inspector_readonly_guard.py`の判定はこれで通る側へだけ動く。**動くのは2経路であり、
    どちらも2026-09-14に実測した。
    **1つはcommand位置の`#`である。**同hookはcommand位置のprogramをallowlistで見るため、
    以前は`#`という語をallowlist外のprogramとして拒否していた。`# メモ`、
    `# メモ`改行`git show …`、`git show …`改行`# メモ`が通るようになる。
    `cat x # メモ`のように語の後ろへ置いた形は、以前からcommand位置ではない。
    **これは行が語へ分けられる場合の話である。**同hookはコメントを落とす前に生の行を
    `tokenize`するため、**コメントの中に閉じない引用符があると、行ごと拒否される**
    （`git show … HEAD # それはできない。don't`。2026-09-14に実測。前後で変わらない）。
    **もう1つは、コメントの中に書いた、拒否される側のoptionである。**同hookのoption検査は
    `invocations`越しに引数を読むため、コメントごと落ちる。
    `git show --no-ext-diff --no-textconv HEAD # --ext-diff`、`git version # --help`、
    `git log --no-ext-diff --no-textconv -1 # %GK`、`rg pattern file # --pre sha1sum`は、
    **以前は拒否され、今は通る。bashはどれも渡さない。**コメントの外へ同じoptionを出せば今も拒否する。
    **拒否が増える形は無く、allowlistの外へ出る経路も増えない。**落とすのはbashも
    実行しない範囲だからである。
    **引用の外のmetacharacterと区切り語の拒否は、コメントを落とす前に決まる。**同hookは#384で
    区切り語を、#396で引用の外のmetacharacterを、生の行に対して拒否する
    （[ADR-0020](../../docs/decisions/0020-inspector-readonly-by-hook.md)）。
    `cat x && # メモ`は`&&`という区切り語で、`git show … HEAD # ; rm -rf /`は
    引用の外の`;`で拒否され、**前後で変わらない。**
    **必須option側の抜け道も、ここでは閉じていない。**`git diff`／`show`／`log`／`blame`が要求する
    2 optionは#384から**subcommandの直後2語**に無ければならず、
    `git diff HEAD # --no-ext-diff --no-textconv`は**#389より前から拒否されている。**
    **コメントを落としても結論は変わらない。**

    採らなかった案:

    - **各hookで`\\n`で切る**（`inspector_readonly_guard.py`と同じ形）。同じ判定を
      **#389が挙げた3 hookだけを直せば3箇所だが、`invocations`を使う7 hookは同じ区切りで
      読む必要がある。**自分で分割済みの`inspector_readonly_guard.py`を除いて
      **6箇所への複製になる。**加えて素朴に切ると、`git commit -m "題\\n\\n本文" && git push`の
      引用符が途中で切れて`tokenize`が失敗し、**同じcommandのpushを取り落とす。**
      `cat > doc.md <<'EOF'`のbodyに書いた`git push origin develop`は、
      **bashが実行しないのに呼び出しとして数える。**誤検知はhookそのものを
      無効化される側の失敗である
    - **`shlex`の設定で改行を語として出す**（`punctuation_chars="\\n"`かつ
      `whitespace=" \\t"`）。実測し、`['cat', 'x', '\\n', 'git', ...]`が
      得られることは確かめた（**採らなかった案であり、testは持たない**）。採らないのは、**`tokenize`を`truncation_guard.py`と
      `inspector_readonly_guard.py`が共有しており**、区切りの定義が変わると
      それらの判定まで動くためである。**heredocのbodyもtokenizeの前に落とせない。**
    """
    found = []
    current = []
    pending = []
    index = 0
    length = len(command)
    while index < length:
        char = command[index]
        if char == "\\" and index + 1 < length:
            if command[index + 1] == "\n":
                # 行継続。**bashは`\`と改行を消して1行へ繋ぐ。**残すと`shlex`が
                # `\`を次の1文字のescapeとして読み、改行を次の語の中の文字にする。
                # `git push \`改行`origin develop`は`['git', 'push', '\norigin', 'develop']`
                # となり、**`origin`が第1引数の位置から外れる**
                # （2026-09-12に実測。2026-09-14に同じ版で再測した）。
                index += 2
                continue
            current.append(command[index:index + 2])
            index += 2
            continue
        if char in "'\"":
            end = _closing_quote(command, index)
            current.append(command[index:end])
            index = end
            continue
        if command.startswith(HERESTRING_OPERATOR, index):
            current.append(HERESTRING_OPERATOR)
            index += len(HERESTRING_OPERATOR)
            continue
        if command.startswith(HEREDOC_OPERATOR, index):
            start = index
            index += len(HEREDOC_OPERATOR)
            strip_tabs = index < length and command[index] == "-"
            if strip_tabs:
                index += 1
            delimiter, index = _heredoc_delimiter(command, index)
            current.append(command[start:index])
            if delimiter:
                pending.append((delimiter, strip_tabs))
            continue
        if char == "#" and _at_word_start(current):
            # bashは語頭の`#`から行末までをコメントとして落とす。**同じに扱う。**
            # 残すと、`git push origin main # develop`の`develop`をrefspecとして数える
            # （このhookが`main`へのpushを`develop`へのpushとして拒否する）。
            end = command.find("\n", index)
            index = length if end == -1 else end
            continue
        if char == "\n":
            found.append("".join(current))
            current = []
            index = _skip_heredoc_bodies(command, index + 1, pending)
            pending = []
            continue
        current.append(char)
        index += 1
    found.append("".join(current))
    return [segment for segment in found if segment.strip()]


def invocations(command, program):
    """`program`の呼び出しごとに、続く引数の並びを返す。

    `cd x && gh pr create ...`は拾い、`echo gh pr create ...`は拾わない。

    **`segments`で分けてから見る。**改行で区切った2行目以降も呼び出しである。
    """
    found = []
    for segment in segments(command):
        found.extend(_invocations_in(segment, program))
    return found


def _invocations_in(command, program):
    """改行を含まない1つのcommandから、`program`の呼び出しを取り出す。"""
    found = []
    current = None
    at_command_position = True
    for token in tokenize(command):
        if token in SEPARATORS:
            if current is not None:
                found.append(current)
                current = None
            at_command_position = True
            continue
        if current is not None:
            current.append(token)
            continue
        if not at_command_position:
            continue
        if is_program(token, program):
            current = []
            continue
        if token in TRANSPARENT_PREFIXES:
            continue
        if "=" in token and not token.startswith("-"):
            # `VAR=value`の代入は、後ろのcommandへ透過する。
            continue
        at_command_position = False
    if current is not None:
        found.append(current)
    return found


def command_starts(command):
    """command位置ごとに`(読み飛ばした前置語の並び, program語)`を返す。

    `invocations`と`programs`が捨てている**前置語そのもの**を残す。
    allowlistで判定するhook（`inspector_readonly_guard.py`）は、
    **`sudo cat x`の`sudo`と`FOO=bar cat x`の`FOO=bar`を見る必要がある。**
    透過させたまま`cat`だけを見ると、`sudo`付きの呼び出しが素通りする。

    `sudo cat x && git show HEAD`は
    `[(("sudo",), "cat"), ((), "git")]`を返す。

    program語が無いまま区切りへ達した場合（`sudo && ls`の`sudo`）は
    `(前置語, None)`を返す。**捨てない。**捨てると、呼び出し側が
    「前置語だけの command は無かった」と読む。

    **`invocations`と同じく`segments`で分けてから見る。**片方だけが改行を区切りとして
    扱うと、2つの関数で command 位置の定義がずれる。
    """
    found = []
    for segment in segments(command):
        found.extend(_starts_in(segment))
    return found


def _starts_in(command):
    """改行を含まない1つのcommandから、command位置の並びを取り出す。"""
    found = []
    skipped = []
    at_command_position = True
    for token in tokenize(command):
        if token in SEPARATORS:
            if skipped:
                found.append((tuple(skipped), None))
                skipped = []
            at_command_position = True
            continue
        if not at_command_position:
            continue
        if token in TRANSPARENT_PREFIXES:
            skipped.append(token)
            continue
        if "=" in token and not token.startswith("-"):
            # `VAR=value`の代入は、後ろのcommandへ透過する。
            skipped.append(token)
            continue
        found.append((tuple(skipped), token))
        skipped = []
        at_command_position = False
    if skipped:
        found.append((tuple(skipped), None))
    return found


def programs(command):
    """command位置に現れたprogram語を、現れた順に返す。

    `invocations`が「特定のprogramを探す」のに対し、こちらは**何が呼ばれているかを
    列挙する。**列挙する側を各hookへ複製せず、command位置の判定をこの module へ寄せる。

    `cat x && git show HEAD`は`["cat", "git"]`を返す。`echo git`は`["echo"]`だけを返す。
    **前置語は落ちる。**前置語まで要る呼び出し側は`command_starts`を使う。

    **`tokenize`が空を返した場合と、実際にcommandが空の場合を区別しない。**
    呼び出し側が「空なら安全」と読まないよう、この関数は判定をしない。
    `inspector_readonly_guard.py`は tokenize の失敗を別途 fail closed で扱う。
    """
    return [program for _, program in command_starts(command) if program is not None]


def command_from(payload):
    """hookの入力から`tool_input.command`を返す。取り出せなければ`None`。

    **payloadがmappingであることを先に確かめる。**妥当なJSONでもmappingでないことがある
    （`[]`／`"text"`／`null`）。**`payload.get`は`AttributeError`を出す。**
    hookが例外で落ちると、止めているはずの判定が走らない。**そしてhookは失敗しても
    静かなため、壊れたことに気付けない。**

    `tool_input`も同じ理由で検査する（`{"tool_input": ["x"]}`という入力がありうる）。

    **戻りが`None`のとき、hookは素通りさせる。**入力を解釈できないことを、
    対象commandの問題として扱わない。**この判断は各hookが変えない。**

    2026-08-26に#242で追加した。**それまで5本のhookが同じ形を各自で書いており、
    どれも型検査を持っていなかった**（[PR #241](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/241)のreview指摘を型で全数走査して見つけた）。
    """
    if not isinstance(payload, dict):
        return None
    tool_input = payload.get("tool_input")
    if tool_input is None:
        return None
    if not isinstance(tool_input, dict):
        return None
    command = tool_input.get("command")
    return command if isinstance(command, str) else None
