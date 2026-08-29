#!/usr/bin/env python3
"""hookが受け取ったcommand文字列から、目的のprogramの呼び出しを取り出す。

`gh_metadata_guard.py`と`branch_base_guard.py`が使う。**同じ判定を各hookへ複製しない。**

**hookの入力からcommandを取り出す`command_from`も持つ。**payloadの形の検査を
各hookへ複製しないためである（#242）。

**語がcommand位置にあるかを見る。**単に`gh`という語を探すと、`echo gh pr merge`のような
引数を呼び出しと読んでしまう。実際にそれで`merge_trailer_report.py`が誤報告し、
`gh_metadata_guard.py`なら誤って拒否する（偽陽性でhookごと無効化される側の失敗である）。

**判定は字句だけで行う。**shellの意味論は再現しない。次は取れない。

- alias、shell function、変数展開経由の呼び出し
- `xargs gh ...`のように、引数をstdinから受ける経由での起動
- `sh -c '...'`の内側

**取れないものは取れないままにする。**推測で拾うと誤検知になり、誤検知はhookそのものを
無効化される側の失敗である。取り切れない範囲はCONTRIBUTINGの「hookが止めたとき」に書く。
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


def invocations(command, program):
    """`program`の呼び出しごとに、続く引数の並びを返す。

    `cd x && gh pr create ...`は拾い、`echo gh pr create ...`は拾わない。
    """
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
