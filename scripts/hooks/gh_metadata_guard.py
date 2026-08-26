#!/usr/bin/env python3
"""`gh`のIssue／Pull Request操作に、必須metadataが付いているかを検査する。

Claude CodeのPreToolUse hookとして、Bash tool呼び出しの前に走る。stdinからhookの
入力JSONを読み、`tool_input.command`だけを見る。実行はしない。

止める対象は2つである。

1. `gh issue create`／`gh pr create`に`--project`が無い。Projects v2 boardへの
   item追加は`CONTRIBUTING.md`の起票規約で必須だが、**#204／#205／#206は3件とも
   作成時に入っておらず、5分55秒〜18分29秒後に追加されている**（`added_to_project_v2`の
   event時刻と作成時刻の差）。文書に書いても実行されないため、作成の一部にする。
2. `gh pr merge`のsquash messageに`Change-Class`と`Self-Review`が無い。
   PR側のgateがSUCCESSでも、squash commitへtrailerが引き継がれないと次の昇格で
   落ちる（`18298ae`／`619c843`。どちらも`DECLARATION_EXEMPT`へ登録済み）。

**判定は字句だけで行う。**意味は判定しない。`review_gate.py`と同じ方針である。

`DESKCAT_SKIP_GH_GUARD=1`で丸ごと無効化できる。**誤検知で作業が止まったときの
逃げ道であり、常用するものではない。**使ったら理由をPull Request本文へ書く。
"""

import json
import os
import sys
from pathlib import Path

import command_line

# trailerの名前は`scripts/review_gate.py`が正本である。**hook側へ複製しない。**
# 名前がずれると、gateが要求するものとhookが見るものが食い違い、hookが素通りする。
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import review_gate  # noqa: E402

# 検査する`gh`のsubcommand。`gh`の呼び出しに続く2語がこの組のときだけ見る。
# 呼び出しの切り出しは`command_line.invocations`が行う。
CREATE_SUBCOMMANDS = (("issue", "create"), ("pr", "create"))
MERGE_SUBCOMMAND = ("pr", "merge")

# squash messageが持たなければならないtrailerの名前。**値は見ない。**
REQUIRED_MERGE_TRAILERS = (review_gate.TRAILER_CLASS, review_gate.TRAILER_REVIEW)

SKIP_ENV = "DESKCAT_SKIP_GH_GUARD"


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


# `--project`の短縮形。`gh issue create`と`gh pr create`はどちらも`-p, --project`を持つ
# （`gh <sub> create --help`で確認した）。**長い方だけを見ると、`-p deskcat`を
# 誤って拒否する。**誤検知はhookごと無効化される側の失敗である。
PROJECT_OPTIONS = ("--project", "-p")


def _has_option(args, *names):
    """`--name value`／`--name=value`／短縮形の連結（`-pvalue`）を拾う。"""
    for arg in args:
        for name in names:
            if arg == name or arg.startswith(name + "="):
                return True
            # 短縮形は値を連結して書ける（`-pdeskcat`）。長い形には当てない。
            if (
                not name.startswith("--")
                and not arg.startswith("--")
                and arg.startswith(name)
                and len(arg) > len(name)
            ):
                return True
    return False


def _option_value(args, *names):
    """`--name value`／`--name=value`の値を返す。無ければ`None`。"""
    for index, arg in enumerate(args):
        for name in names:
            if arg == name:
                if index + 1 < len(args):
                    return args[index + 1]
                return None
            if arg.startswith(name + "="):
                return arg[len(name) + 1:]
    return None


def _merge_message(args):
    """squash messageとして渡された本文を返す。

    戻りは`(text, source)`。`text`が`None`のときは本文を特定できなかったことを表し、
    `source`がその理由を持つ。**特定できないことを「入っている」と扱わない。**
    """
    path = _option_value(args, "--body-file", "-F")
    if path is not None:
        if path == "-":
            return None, "本文がstdin（`-`）から渡されており、hookからは読めない"
        try:
            with open(path, encoding="utf-8") as handle:
                return handle.read(), f"--body-file {path}"
        except OSError as error:
            return None, f"`--body-file`の読み出しに失敗した: {error}"
    body = _option_value(args, "--body", "-b")
    if body is not None:
        return body, "--body"
    return None, "本文が渡されていない"


def _option_names():
    """診断文に出す option 名の並び。定数から作り、文面へ値を複製しない。"""
    return "／".join(f"`{name}`" for name in PROJECT_OPTIONS)


def _check_create(subcommand, args):
    if _has_option(args, *PROJECT_OPTIONS):
        return
    _deny(
        f"`gh {' '.join(subcommand)}`にprojectの指定が無い"
        f"（{_option_names()}のいずれかが要る）。"
        " Projects v2 boardへのitem追加は起票・作成時に必要である"
        "（CONTRIBUTINGの「起票時に設定する項目」）。"
        " #204／#205／#206は3件とも作成時に入っておらず後から追加している。"
        " 注意ではなく、作成の一部として強制している。"
        f" 意図してboardへ入れない場合は{SKIP_ENV}=1を付けて実行し、理由を残す。"
    )


def _check_merge(args):
    text, source = _merge_message(args)
    if text is None:
        _deny(
            f"`gh pr merge`のsquash messageを確認できない（{source}）。"
            " 確認できないmessageでmergeすると、trailerがsquash commitへ"
            " 引き継がれたかを後から辿れない。messageを渡さない場合はGitHubが"
            " 合成したmessageになり、trailerは入らない"
            "（`18298ae`／`619c843`で実際に起きた）。"
            " `--subject`と`--body-file`を明示する（CONTRIBUTINGの「Merge方式」）。"
        )
    missing = [name for name in REQUIRED_MERGE_TRAILERS if f"{name}:" not in text]
    if missing:
        _deny(
            f"`gh pr merge`のsquash message（{source}）に"
            f" {', '.join(missing)} が無い。"
            " 引き継がれないと、そのmergeは通っても次の`main`昇格で"
            " `review_gate.py history`が落ちる。"
            " 値の正本は`scripts/review_gate.py`である。"
        )


def main():
    if os.environ.get(SKIP_ENV) == "1":
        return 0
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError):
        # hookの入力を読めないことを、対象commandの問題として扱わない。
        return 0
    command = (payload.get("tool_input") or {}).get("command")
    if not isinstance(command, str) or "gh" not in command:
        # 全Bash呼び出しでこのhookが走る。`if`条件でBash(gh *)へ絞ると
        # `cd x && gh pr create`が素通りするため、絞らずここで安く抜ける。
        return 0
    for args in command_line.invocations(command, "gh"):
        head = tuple(args[:2])
        if head in CREATE_SUBCOMMANDS:
            _check_create(head, args[2:])
        elif head == MERGE_SUBCOMMAND:
            _check_merge(args[2:])
    return 0


if __name__ == "__main__":
    sys.exit(main())
