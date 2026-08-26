#!/usr/bin/env python3
"""hookが受け取ったcommand文字列から、目的のprogramの呼び出しを取り出す。

`gh_metadata_guard.py`と`branch_base_guard.py`が使う。**同じ判定を各hookへ複製しない。**

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
