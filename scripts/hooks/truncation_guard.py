#!/usr/bin/env python3
"""件数・不在の根拠になりうる列挙commandの、明示的な切り詰めに注意を促す。

Claude CodeのPreToolUse hookとして、Bash tool呼び出しの前に走る。stdinからhookの
入力JSONを読み、`tool_input.command`だけを見る。実行はしない。

**なぜ作るか。**件数や不在を主張する根拠として使った列挙commandが、切り詰められた
出力を全数として読まれた事例が起きている。`head -8`で17件を8件と読んだ（2回）。
既存hook（`gh_metadata_guard.py`／`branch_base_guard.py`／`push_gate.py`）は
commandの構造（`--project`の有無等）だけを見ており、出力の完全性は見ていない
（[#349](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/349)）。

**対象commandは6つである。**`gh pr list`／`gh issue list`／`gh api`／
`git log`／`git rev-list`／`git branch`。

**検出する形は3つである。いずれも「明示的な数がcommandに現れている」場合だけを見る。**

1. 対象commandの出力を`| head -N`／`| tail -N`へ渡している（pipelineの隣接で見る）
2. `git log`／`git rev-list`に`-n N`／`-nN`／`--max-count=N`が付いている
3. `gh pr list`／`gh issue list`に`--limit N`／`-L N`が付いている

**`--limit`を指定しない呼び出し（既定30件）は対象外にする。**理由は誤検知である。
これらのcommandをlogを眺めるためだけに使う頻度は非常に高く、`--limit`省略は
そのほとんどを占める。**明示的な数が全く無い呼び出しまで対象にすると、通常の
閲覧が毎回止まり、hookそのものが無視される側の失敗になる**（指示書「迷ったら
対象を狭める」）。対して`head -N`／`--limit N`／`-n N`は、その場に**具体的な数**が
書かれており、その数を「全数」と読み違える経路がある。**区別できる境界はここにある。**

**返すのは`deny`ではなく`ask`である。**`head -8`や`--limit 50`は、件数の根拠として
使うことも、単に一部だけを見る正当な閲覧であることもあり、**commandの字句だけからは
区別できない。**`coderabbit_gate.py`が同種の判断（誤検知の代償は確認1回、見逃しの
代償はこの検査が在る理由そのもの）で`ask`を採っており、同じ理由でここも`ask`にする。

**判定は字句だけで行う。**`review_gate.py`／`gh_metadata_guard.py`と同じ方針である。
shellの意味論は再現しない。`command_line.py`が取れない範囲（alias、`xargs`経由等）は
ここでも取れない。

`DESKCAT_SKIP_TRUNCATION_GUARD=1`で丸ごと無効化できる。**誤検知で作業が止まったときの
逃げ道であり、常用するものではない。**使ったら理由をPull Request本文へ書く。
"""

import json
import os
import re
import sys

import command_line

SKIP_ENV = "DESKCAT_SKIP_TRUNCATION_GUARD"

# 対象command。`gh`／`git`に続く語の並びで判定する。`gh api`は2語目までで一致させ、
# 3語目以降（endpoint）は問わない。
TARGET_COMMANDS = (
    ("gh", "pr", "list"),
    ("gh", "issue", "list"),
    ("gh", "api"),
    ("git", "log"),
    ("git", "rev-list"),
    ("git", "branch"),
)

# `git log`／`git rev-list`にだけ`-n`／`--max-count`を要求する。`git branch`と
# `gh`系はこの形の件数指定を持たない（`git branch`の`-n`は別option、`gh`系は
# `--limit`を使う）。
MAX_COUNT_COMMANDS = (("git", "log"), ("git", "rev-list"))

# `gh pr list`／`gh issue list`にだけ`--limit`を要求する。`gh api`は
# エンドポイントによって`--paginate`の要否が変わり、字句だけでは判定できないため
# 対象外にする（推測で拾うと誤検知になる）。
LIMIT_COMMANDS = (("gh", "pr", "list"), ("gh", "issue", "list"))

HEAD_TAIL_PROGRAMS = ("head", "tail")

# helpの表示だけを求める呼び出し。**何も列挙しないため対象外にする。**
# 既存hook（`gh_metadata_guard.py`等）と同じ扱いに揃える。
HELP_OPTIONS = ("--help", "-h")


def _ask(reason):
    """PreToolUse hookとして、人間へ許可を求めて終了する。

    **`deny`ではない。**`head -N`や`--limit N`は、件数の根拠として使うことも
    正当な閲覧であることもあり、commandの字句だけからは区別できない。
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


def _pipelines(command):
    """commandを、`|`で連結されたpipelineごとの、stage（語のlist）の列へ分ける。

    `|`以外の区切り（`&&`／`;`等）は別のpipelineとして切り離す。
    `command_line.SEPARATORS`をそのまま使い、区切りの定義を複製しない。
    """
    pipelines = []
    current_pipeline = []
    current_stage = []
    for token in command_line.tokenize(command):
        if token == "|":
            current_pipeline.append(current_stage)
            current_stage = []
            continue
        if token in command_line.SEPARATORS:
            current_pipeline.append(current_stage)
            pipelines.append(current_pipeline)
            current_pipeline = []
            current_stage = []
            continue
        current_stage.append(token)
    current_pipeline.append(current_stage)
    pipelines.append(current_pipeline)
    return pipelines


def _skip_prefixes(stage):
    """stage先頭の`env FOO=1`／`sudo`等の透過語を読み飛ばす。

    `command_line.invocations`が呼び出しの前へ許す語と同じ集合
    （`TRANSPARENT_PREFIXES`）を使う。判定を複製しない。
    """
    tokens = list(stage)
    while tokens:
        token = tokens[0]
        if token in command_line.TRANSPARENT_PREFIXES:
            tokens.pop(0)
            continue
        if "=" in token and not token.startswith("-"):
            tokens.pop(0)
            continue
        break
    return tokens


def _match_target(stage):
    """stageが対象commandの呼び出しなら、一致した`TARGET_COMMANDS`の組を返す。"""
    tokens = _skip_prefixes(stage)
    if len(tokens) < 2:
        return None
    program = tokens[0]
    for pattern in TARGET_COMMANDS:
        if not command_line.is_program(program, pattern[0]):
            continue
        rest = pattern[1:]
        if tuple(tokens[1:1 + len(rest)]) == rest:
            return pattern
    return None


_NUMERIC_SHORT_LIMIT = re.compile(r"^-(\d+)$")


def _head_tail_limit(stage):
    """stageが`head`／`tail`に明示的な数値を伴う呼び出しなら、その数値を返す。

    引数の無い`head`／`tail`（既定10行）は対象外にする。**数がcommandに一切
    現れない呼び出しまで拾うと、閲覧全般を止めることになる。**
    """
    tokens = _skip_prefixes(stage)
    if not tokens or not any(
        command_line.is_program(tokens[0], name) for name in HEAD_TAIL_PROGRAMS
    ):
        return None
    args = tokens[1:]
    for index, arg in enumerate(args):
        match = _NUMERIC_SHORT_LIMIT.match(arg)
        if match:
            return match.group(1)
        if arg == "-n" and index + 1 < len(args) and args[index + 1].isdigit():
            return args[index + 1]
        if arg.startswith("-n") and arg[2:].isdigit():
            return arg[2:]
        if arg.startswith("--lines=") and arg[len("--lines="):].isdigit():
            return arg[len("--lines="):]
    return None


def _max_count_value(args):
    """`git log`／`git rev-list`の`-n`／`--max-count`の値を返す。無ければ`None`。"""
    for index, arg in enumerate(args):
        if arg == "-n" and index + 1 < len(args) and args[index + 1].isdigit():
            return args[index + 1]
        if arg.startswith("-n") and arg[2:].isdigit():
            return arg[2:]
        if arg.startswith("--max-count=") and arg[len("--max-count="):].isdigit():
            return arg[len("--max-count="):]
    return None


def _limit_value(args):
    """`gh pr list`／`gh issue list`の`--limit`／`-L`の値を返す。無ければ`None`。"""
    for index, arg in enumerate(args):
        if arg in ("--limit", "-L") and index + 1 < len(args):
            return args[index + 1]
        if arg.startswith("--limit="):
            return arg[len("--limit="):]
        if arg.startswith("-L") and len(arg) > 2:
            return arg[2:]
    return None


def _check_pipeline(pipeline):
    for index, stage in enumerate(pipeline):
        target = _match_target(stage)
        if target is None:
            continue
        if any(arg in HELP_OPTIONS for arg in _skip_prefixes(stage)):
            # helpの表示だけを求める呼び出しは何も列挙しない。**判定を要求しない。**
            continue
        command_text = " ".join(target)
        # 1. pipeで隣接する`head`／`tail`。
        if index + 1 < len(pipeline):
            limit = _head_tail_limit(pipeline[index + 1])
            if limit is not None:
                next_program = _skip_prefixes(pipeline[index + 1])[0]
                _ask(
                    f"`{command_text}`の出力を`{next_program} {limit}`へ渡している。"
                    f" 表示されるのは先頭または末尾の{limit}件だけであり、"
                    " 件数や不在の根拠にする場合は全数を取ってから数える必要がある。"
                    " 単に一部を見るだけの用途であればこのまま進めてよい。"
                    f" 誤検知で頻発する場合は{SKIP_ENV}=1で無効化し、理由を残す。"
                )
        args = _skip_prefixes(stage)[len(target):]
        # 2. `git log`／`git rev-list`の`-n`／`--max-count`。
        if target in MAX_COUNT_COMMANDS:
            count = _max_count_value(args)
            if count is not None:
                _ask(
                    f"`{command_text}`に`-n {count}`相当の件数指定が付いている。"
                    f" 表示されるのは先頭{count}件だけであり、"
                    " 件数や不在の根拠にする場合は全数を取ってから数える必要がある。"
                    " 単に一部を見るだけの用途であればこのまま進めてよい。"
                    f" 誤検知で頻発する場合は{SKIP_ENV}=1で無効化し、理由を残す。"
                )
        # 3. `gh pr list`／`gh issue list`の`--limit`。
        if target in LIMIT_COMMANDS:
            limit = _limit_value(args)
            if limit is not None:
                _ask(
                    f"`{command_text}`に`--limit {limit}`が付いている。"
                    f" 表示されるのは最大{limit}件だけであり、"
                    " 件数や不在の根拠にする場合は、実際の総数以上の`--limit`を"
                    " 明示してから絞る必要がある。単に一部を見るだけの用途であれば"
                    " このまま進めてよい。"
                    f" 誤検知で頻発する場合は{SKIP_ENV}=1で無効化し、理由を残す。"
                )


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
    for pipeline in _pipelines(command):
        _check_pipeline(pipeline)
    return 0


if __name__ == "__main__":
    sys.exit(main())
