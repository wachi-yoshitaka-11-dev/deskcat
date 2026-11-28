#!/usr/bin/env python3
"""`/code-review` の実行を手元の log に残し、log への書き込みを止める hook（#814 の作業リストの9）。

登録は `.claude/settings.json` の PreToolUse と PostToolUse である。役目は2つある。

1. **記録する。**Skill の前と後の hook のうち、`agent_id` を持つもの（background の
   subagent の中の実行）で、skill が `code-review` のものだけを記録する。
   - 前の hook：引数から範囲 `A..B` を1つだけ取り、A と B を 40 桁の SHA に直し、
     A が B の祖先であることを確かめて、`started` の行を書く。範囲が無い、2つ以上ある、
     SHA に直せない、A が B の祖先でない、のどれでも記録しない。**実行そのものは止めない。**
   - 後の hook：`completed` の行に、完了の状態（`success`、`report`、`status`）を書く。
     `report` は、報告が指摘の一覧の形だったか（`review_coverage.is_findings_report`）。
   - 前と後は `tool_use_id` で組にする。判定は `scripts/review_coverage.py` が持つ。
   main の文脈での実行（`agent_id` が無い）は記録しない。2026-11-28 の実験で、
   main の文脈の Skill の後の hook には結果が届かず、レビューが終わったかを知る手段が無かった。
2. **log を守る。**log の path への Write／Edit／MultiEdit／NotebookEdit と、log の file 名を
   含む Bash／PowerShell を拒否する。log を書くのはこの hook だけである。

**限界：**この hook は、fork が範囲の全部を実際に読んだかまでは確かめない。
**止めるのはうっかりの書き込みで、意図的な偽りは前提にしない。**shell の command に log の名前が
そのまま書かれたものだけを止める。名前を組み立てる script を走らせる、この hook に偽の入力を
流す、のどちらでも記録を偽れる（起動の形の列挙では抜けが尽きないので、hook の名前では止めない）。
log は改ざんを防ぐものではなく、レビューの実行を残すための記録である。
hook の入力を読めないとき、git が使えないときは、何も記録せず、何も止めない
（記録が無ければ merge の前の検査で止まるので、記録の側で止める必要が無い）。
**ただし黙らない。**失敗を stderr に出し、exit 1（`hook_input.NOTICE`）で終える。
入力は `hook_input.read_payload` で UTF-8 として読む。既定の文字コード（Windows の cp932）で
読んでいたとき、ASCII でない報告の本文を読めず、完了の記録を黙って落としていた（2026-11-28）。
"""

import datetime
import json
import os
import subprocess
import sys
from pathlib import Path

import hook_input

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import review_coverage  # noqa: E402

SKILL_NAME = "code-review"
WRITE_TOOLS = ("Write", "Edit", "MultiEdit", "NotebookEdit")
SHELL_TOOLS = ("Bash", "PowerShell")


def _deny(reason):
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


def _is_review_skill(tool_input):
    name = tool_input.get("skill") if isinstance(tool_input, dict) else None
    return isinstance(name, str) and (name == SKILL_NAME or name.endswith(":" + SKILL_NAME))


def _append(path, record):
    path.parent.mkdir(parents=True, exist_ok=True)
    # 1行を1回の write で追記する。書けた長さが足りなければ例外にする（main が分類して出す）。
    # 並行する hook の行が混ざらないことは保証しない（Windows の O_APPEND は不可分でない）。
    # 混ざった行は JSON として読めないので、`review_coverage.read_log` が飛ばし、証拠に数えない。
    # 行の頭にも改行を置く。前の書き込みが途中で切れていても、その断片をこの行の頭で閉じ、
    # この記録まで壊さない（空の行は `read_log` が飛ばす）。
    data = ("\n" + json.dumps(record, ensure_ascii=False) + "\n").encode("utf-8")
    descriptor = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT | getattr(os, "O_BINARY", 0),
                         0o644)
    try:
        written = os.write(descriptor, data)
    finally:
        os.close(descriptor)
    if written != len(data):
        raise OSError(f"log に {len(data)} byte のうち {written} byte しか書けなかった")


def _now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


def _record(payload, event):
    tool_input = payload.get("tool_input")
    if not _is_review_skill(tool_input):
        return
    agent_id = payload.get("agent_id")
    tool_use_id = payload.get("tool_use_id")
    if not isinstance(agent_id, str) or not agent_id:
        return
    if not isinstance(tool_use_id, str) or not tool_use_id:
        return
    cwd = payload.get("cwd") or None
    path = review_coverage.log_path(cwd)
    if path is None:
        return
    base = {"tool_use_id": tool_use_id, "agent_id": agent_id, "at": _now()}
    if event == "PreToolUse":
        pair = review_coverage.parse_arg_range(tool_input.get("args"))
        if pair is None:
            return
        start = review_coverage.resolve(pair[0], cwd)
        end = review_coverage.resolve(pair[1], cwd)
        if not start or not end or start == end:
            return
        if not review_coverage.is_ancestor(start, end, cwd):
            return
        _append(path, {**base, "event": "started", "from": start, "to": end,
                       "args": tool_input.get("args")})
    else:
        response = payload.get("tool_response")
        response = response if isinstance(response, dict) else {}
        result = response.get("result")
        # 完了と数えるのは、`success` が真で、結果（レビューの報告）が届いたときだけである。
        # 2026-11-28 の実験で、fork の中の実行の後の hook には、レビューの報告が `result` に届いた。
        # `report` は、報告が指摘の一覧の形だったことの印である。利用の上限で止まった実行も
        # `success` は真で届くので、`success` だけでは完了と見分けられない
        # （`review_coverage.is_findings_report`）。
        _append(path, {**base, "event": "completed",
                       "success": response.get("success") is True
                       and isinstance(result, str) and bool(result.strip()),
                       "report": review_coverage.is_findings_report(result),
                       "status": response.get("status")})


def _same_path(candidate, target, cwd):
    try:
        raw = Path(candidate)
        if not raw.is_absolute() and cwd:
            raw = Path(cwd) / raw
        return os.path.normcase(os.path.realpath(raw)) == \
            os.path.normcase(os.path.realpath(target))
    except (OSError, ValueError):
        return False


def _guard(payload):
    tool = payload.get("tool_name")
    tool_input = payload.get("tool_input")
    if not isinstance(tool_input, dict):
        return
    if tool in SHELL_TOOLS:
        command = tool_input.get("command")
        if isinstance(command, str) and review_coverage.LOG_NAME in command:
            _deny(
                f"レビューの記録の log（`{review_coverage.LOG_NAME}`）を含む command は使えない。"
                "記録は Claude Code が起動する hook だけが書く（#814 の作業リストの9）。"
            )
        return
    if tool in WRITE_TOOLS:
        cwd = payload.get("cwd") or None
        target = review_coverage.log_path(cwd)
        if target is None:
            return
        for key in ("file_path", "notebook_path", "path"):
            value = tool_input.get(key)
            if isinstance(value, str) and (
                Path(value).name == review_coverage.LOG_NAME or _same_path(value, target, cwd)
            ):
                _deny(
                    "レビューの記録の log へは書き込めない。log を書くのは"
                    " `scripts/hooks/review_record.py` だけである（#814 の作業リストの9）。"
                )


def main():
    try:
        payload = hook_input.read_payload()
    except hook_input.HookInputError as error:
        hook_input.report(f"review_record: hook の入力を読めないので、記録しなかった（{error}）")
        return hook_input.NOTICE
    if not isinstance(payload, dict):
        return 0
    event = payload.get("hook_event_name")
    tool = payload.get("tool_name")
    try:
        if event == "PreToolUse" and tool != "Skill":
            _guard(payload)
        elif event in ("PreToolUse", "PostToolUse") and tool == "Skill":
            _record(payload, event)
    except (OSError, ValueError, TypeError, subprocess.SubprocessError) as error:
        # 記録できないことは、merge の前の検査（記録が無い）で止まる。ここでは止めないが、
        # 黙らない。stderr に分類を出し、exit 1 で失敗を知らせる。
        hook_input.report(f"review_record: {event}/{tool} を記録できなかった"
                          f"（{type(error).__name__}: {error}）")
        return hook_input.NOTICE
    return 0


if __name__ == "__main__":
    sys.exit(main())
