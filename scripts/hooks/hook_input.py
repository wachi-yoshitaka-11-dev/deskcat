#!/usr/bin/env python3
"""Claude Code の hook が受け取る入力（stdin の JSON）を読み、失敗を stderr へ出す。

`hooks/` 配下の hook が使う。**読み方を各 hook へ複製しない。**

**入力は UTF-8 と明示して読む。**`json.load(sys.stdin)` は、Python の既定の文字コードで読む。
Windows の既定（cp932）では、レビューの報告のような ASCII でない本文を読めないことがある。
2026-11-28 に、`review_record.py` が fork のレビュー8回のうち3回で、完了の記録を
黙って落とした。`gh_metadata_guard.py` は、読めない入力を何もせずに通していた
（#814 の作業リストの9）。

**読めないときの動きは、hook の役割で決める。**止める側の hook（PreToolUse の guard）は、
`BLOCK`（exit 2）で止める。Claude Code では exit 2 か JSON の deny だけが tool を止め、
exit 1 は止めない。記録や通知の側の hook は、`NOTICE`（exit 1）で失敗を出すだけにする。
"""

import json
import sys

# 止める側の hook が、読めない入力で返す exit code。Claude Code は exit 2 で tool を止める。
BLOCK = 2
# 記録や通知の側の hook が、失敗を知らせるときの exit code。tool は止めない。
NOTICE = 1


class HookInputError(Exception):
    """hook の入力を読めない。"""


def read_payload(stream=None):
    """stdin の byte を UTF-8 として読み、JSON の値を返す。UTF-8 か JSON として読めなければ
    `HookInputError`。

    **値の形（object か、必要な key があるか）は見ない。**形の検査は各 hook が持つ
    （`command_line.command_from` など）。形の違う入力を止めずに通す決まりは、各 hook の
    test（`HookPayloadShapeTests`）が持つ。
    """
    try:
        # stdin が無い（`None`）、byte で読めない、読む途中で OS が失敗する、のどれも読めない入力として扱う。
        raw = (stream if stream is not None else sys.stdin.buffer).read()
        return json.loads(raw.decode("utf-8"))
    except (OSError, AttributeError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise HookInputError(f"{type(error).__name__}: {error}") from error


def report(message):
    """stderr へ1行を UTF-8 で書く。

    **stderr の文字コードによらず UTF-8 で書く。**入力と同じく、文字コードを UTF-8 の1つに決める。
    stderr の文字コード（Windows の cp932）で書くと、UTF-8 として読む側で日本語の理由が文字化けする。
    """
    sys.stderr.buffer.write((message + "\n").encode("utf-8"))
    sys.stderr.buffer.flush()
