#!/usr/bin/env python3
"""CodeRabbitのreviewを起動するcommandを、人間の許可を求める形で止める。

Claude CodeのPreToolUse hookとして、Bash tool呼び出しの前に走る。stdinからhookの
入力JSONを読み、`tool_input.command`だけを見る。実行はしない。

**止めるのはreviewを起動するものだけである。**判定は「`@coderabbitai`と同じ行に
`review`という語があるか」の1点で行う。**枠を消費する操作の名前はどれも`review`を含む。**
`rate limit`、`resolve`、`help`は素通りする。**免除listを持たせると、`rate limit`で
始めて後ろに`full review`を置く形で抜ける**（実測した）。

**返すのは`deny`ではなく`ask`である。**AIの判断を経由せずに人間へ許可を求める形にする
（`permissionDecision`の許容値は`allow`／`deny`／`ask`）。`deny`にすると、人間が
「reviewを投げて」と指示した通常の流れも通らなくなる。**止めたいのはAIの独断であって、
人間の依頼ではない。**

**環境変数によるバイパスを設けていない。**他のhookの`SKIP_ENV`に当たるものが無いのは
意図である。**AIが自分で外せる抑制は抑制ではない。**この判断は#240で人間が決めた。

なぜ必要か。**AIはこの型の失敗を3回繰り返した。**

1. #153（1 file、+2 −1）に対し、誰も頼んでいないreviewを取りに行き`rate limit`を投げた
2. PR #146／#148／#150。待ち時間を告げられているのに`rate limit`を約60秒ごとに
   投げ続け、**34件**のゴミコメントを積んだ
3. PR #238／#239。承認なしに`full review`を連投し、2本目が`Review rate limited`で
   空振りして安全に関わる変更のreviewが59分止まった

**AI側の記憶には3件の禁止規則が既にあり、3件とも破った。**記憶は参照するかどうかが
AIの判断に依存する。**同じ型を記憶では止められていない。**`push_gate.py`が
`develop`への直接pushを門で止めているのと同じ扱いにする。

**判定は字句だけで行う。**意味は判定しない。`review_gate.py`と同じ方針である。
"""

import json
import re
import sys

import command_line

# `@coderabbitai`を呼ぶ本文が渡されうるsubcommand。`gh`の呼び出しに続く2語で見る。
# `gh api`は1語だが、`gh api ... -f body=@coderabbitai ...`の形で投げられるため含める。
COMMENT_SUBCOMMANDS = (("pr", "comment"), ("issue", "comment"), ("pr", "review"))
API_SUBCOMMAND = ("api",)

# `gh api`で読み取りと明示されたmethod。**この場合はcommentを投げられない。**
# `-f`を付けると`gh`は既定でPOSTになるが、`-X GET`を明示すると読み取りのままである。
# **endpointのwhitelistは作らない。**`gh api graphql`のmutationでもcommentは投げられ、
# **endpoint名からは判別できない。**methodだけで絞る。
READ_ONLY_METHODS = ("get", "head")
METHOD_OPTIONS = ("-X", "--method")

# 止める対象。**`review`という語が現れたら止める。**
# `full review`は完全なreview、`review`はincrementalで、**どちらも枠を1件消費する。**
# incrementalはADR-0013で自動reviewを無効にしているこのrepositoryでは必ず空振りするが、
# **空振りでも枠は消える**（2026-08-24、PR #191で実際に焼いた）。
#
# **語の並びを列挙するのではなく、`review`の1語だけを見る。**列挙すると
# `full-review`のような表記の揺れで抜ける。**枠を消費する操作の名前は
# どれも`review`を含む**ため、これで足りる。
REVIEW_WORD = re.compile(r"\breview\b")

# 枠を消費しない操作。**別扱いにしていない。**いずれも`review`という語を含まないため、
# 上の判定で自動的に素通りする。**免除listを持たせると、`rate limit`で始めて
# 後ろに`full review`を置く形で抜けてしまう**（実測した）。
NOT_A_REVIEW = ("rate limit", "resolve", "help", "configuration", "status")

MENTION = "@coderabbitai"


def _note(message):
    """素通りさせた理由をstderrへ1行で残す。**判定は変えない。**

    hookのstdoutは`permissionDecision`のJSONに使うため、**診断はstderrへ出す。**
    素通りはこのhookの設計であって異常ではないが、**黙って通ると
    「止めているはず」と食い違ったことに気付けない。**
    """
    sys.stderr.write(f"coderabbit_gate: {message}\n")


def _ask(reason):
    """PreToolUse hookとして、人間へ許可を求めて終了する。

    **`deny`ではない。**AIの独断を止めるのが目的であり、人間の依頼を塞ぐのではない。
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


def _mentions_review(text):
    """`@coderabbitai`と同じ行に`review`があるなら`True`。

    **行単位で見る。**CodeRabbitへのcommandは`@coderabbitai <command>`の形で書き、
    1つのcommentに複数書ける。**本文全体で見ると、mentionと無関係な行の`review`まで
    拾う**（「reviewで出た指摘を反映した」という返信を止めてしまう）。
    **mentionの直後の1語だけで見ると、`rate limit`で始めて後ろに`review`を置く形で抜ける。**
    同じ行という範囲がその中間である。

    **取りこぼす側と止めすぎる側では、止めすぎる側へ倒す。**返すのは`deny`ではなく
    `ask`であり、人間が通せる。
    """
    for line in text.lower().splitlines():
        if MENTION not in line:
            continue
        if REVIEW_WORD.search(line.split(MENTION, 1)[1]):
            return True
    return False


# 本文が載りうるoption。`gh api`は`-f body=...`を複数持てるため、**全件を見る。**
# `gh_metadata_guard.py`の`_option_value`は最初の1件だけを返すので使えない。
BODY_OPTIONS = ("--body", "-b", "-f", "--field", "--raw-field", "-F")
BODY_FILE_OPTIONS = ("--body-file",)


def _option_values(args, names):
    """`--name value`と`--name=value`の値を**全件**返す。

    短縮形の連結（`-fbody=...`）は拾わない。`gh`はこの形を受けるが、
    **拾おうとすると`-F`のような別optionの値まで巻き込む。**取りこぼす側へ倒す。
    """
    found = []
    index = 0
    while index < len(args):
        arg = args[index]
        for name in names:
            if arg == name:
                if index + 1 < len(args):
                    found.append(args[index + 1])
                index += 1
                break
            if arg.startswith(name + "="):
                found.append(arg[len(name) + 1:])
                break
        index += 1
    return found


def _texts(args):
    """引数として渡された本文の候補を返す。

    **`--body-file`は読む。**`gh_metadata_guard.py`の`_merge_message`と違い、
    読めなかったことを理由に止めない。**本文が読めなければ`@coderabbitai`が
    含まれるかを判定できないため素通りさせる。**hookが読めないことを、
    AIの独断の証拠として扱わない。**この方向の取りこぼしは意図である。**
    """
    found = _option_values(args, BODY_OPTIONS)
    for path in _option_values(args, BODY_FILE_OPTIONS):
        if path == "-":
            _note(f"本文がstdin（`-`）から渡されており、hookからは読めない: {path}")
            continue
        try:
            with open(path, encoding="utf-8") as handle:
                found.append(handle.read())
        except OSError as error:
            # **握りつぶさない。**素通りさせる判断は変えないが、分類して記録する
            # （AGENTS.mdの「エラーを握りつぶさず、分類、ログ、カウンタを用意する」）。
            # `gh_metadata_guard.py`は同じ失敗を診断文へ載せているが、**こちらは
            # 素通りするため診断文が無い。**代わりにstderrへ出す。
            _note(f"`--body-file`の読み出しに失敗したため判定できない: {error}")
    return found


def _check(subcommand, args):
    for text in _texts(args):
        if not _mentions_review(text):
            continue
        _ask(
            f"`{' '.join(subcommand)}`で`{MENTION}`へreviewを投げようとしている。"
            " **CodeRabbitのreview枠は共有資源であり、AIの判断で消費してよいものではない。**"
            " 投げてよいか人間へ確認する。"
            " このrepositoryは同じ型の失敗を3回している"
            "（#153の無断依頼、PR #146／#148／#150の34件のゴミコメント、"
            " PR #238／#239の連投で安全に関わる変更のreviewが59分止まった）。"
            " 判断材料: 誰が求めたか／対象は新しい内容か／直前に同じ資源を使ったか／"
            " 対象が複数なら重要度の高いものを1本ずつか。"
            f" 枠を消費しない{'／'.join(f'`{w}`' for w in NOT_A_REVIEW)}は素通りする。"
        )


def main():
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        # hookの入力を読めないことを、対象commandの問題として扱わない。
        # **素通りさせるが、握りつぶさない。**既存hookはここを黙って返すが、
        # `_note()`を用意した以上この経路だけ黙るのは筋が通らない
        # （PR #241のreview指摘）。**return codeとstdoutは変えない。**
        _note(f"hookの入力を読めなかったため判定できない: {error}")
        return 0
    command = command_line.command_from(payload)
    if command is None:
        # 妥当なJSONでもmappingでないことがある（`[]`／`{"tool_input": ["x"]}`）。
        # **判定は`command_line.command_from`が持つ。**5本のhookへ複製しない（#242）。
        _note("hookの入力からcommandを取り出せなかったため判定できない")
        return 0
    if "gh" not in command:
        # 全Bash呼び出しでこのhookが走る。`if`条件でBash(gh *)へ絞ると
        # `cd x && gh pr comment`が素通りするため、絞らずここで安く抜ける。
        return 0
    for args in command_line.invocations(command, "gh"):
        head = tuple(args[:2])
        if head in COMMENT_SUBCOMMANDS:
            _check(head, args[2:])
        elif tuple(args[:1]) == API_SUBCOMMAND:
            rest = args[1:]
            methods = [m.lower() for m in _option_values(rest, METHOD_OPTIONS)]
            if any(m in READ_ONLY_METHODS for m in methods):
                # 読み取りと明示されている。**commentは投げられない。**
                continue
            _check(API_SUBCOMMAND, rest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
