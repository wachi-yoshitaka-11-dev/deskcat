#!/usr/bin/env python3
"""応答が完了を主張しているのに、対応するtool呼び出しが無いことをblockする。

Claude CodeのStop hookとして、応答の生成が終わるたびに走る。stdinからhookの
入力JSONを読み、`last_assistant_message`（直前の応答本文）と`transcript_path`
（会話全体の記録）を見る。

**なぜ作るか。**セッションが「送りました」「起票しました」のように完了を過去形で
主張しながら、対応するtool呼び出しが実際には無い事例が繰り返している
（[#350](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/350)）。
既存hookは全て`PreToolUse`／`PostToolUse`の`matcher: "Bash"`であり、
`mcp__ccd_session_mgmt__send_message`のようなMCP tool呼び出しや、
tool呼び出しを伴わない応答文だけで起きるこの型を検出できない。

## 主張の検出

`last_assistant_message`だけを見る。**transcriptからは語を拾わない**
（PMの決定4）。stdinに応答本文がそのまま渡っており、経験則に依存する範囲を
transcriptの構造解釈（下記）だけに絞るためである。

検出する語は最小構成である（`CLAIM_CATEGORIES`）。引用（`「」`）の中の語、
否定形、未来形・意志形は主張として数えない。**これらの区別は、固定した語の
完全一致で足りる。**「送りました」と「送っていません」／「送ります」は
文字列として重ならない（日本語の活用がそもそも別の文字列になるため）。

## このターンのtool呼び出し

**「このターン」の境界は公式契約ではなく、実測に基づく経験則である**
（担当セッションが2026-09-05〜06に、自身のtranscript 2079行を観測した。
観測は1セッションのみである）。`transcript_path`のJSONLを逆順に走査し、
直近の「`type: "user"`かつ`content`が**tool_resultだけで構成されていない**」
entryより後ろにある、全`type: "assistant"`entryの`tool_use`blockを集める。

判定の軸は「人間の入力かどうか」ではなく「**toolの戻りかどうか**」に置く
（PMの決定1）。`content`が文字列ならもちろん人間の入力だが、`content`が
listでも`tool_result`以外のblock（`text`、`image`等）を含めば人間の入力として
扱う。**確認していない形（画像添付等）を、確認しないまま安全に扱える。**
`content`が`tool_result`だけのlistのときだけ、toolの戻りとして境界にしない。

この境界の前提が崩れる場合（Claude Codeのtranscript形式が変わる等）は、
再実測して見直す。**将来transcriptの形が変わったらtestが落ちる状態にするため、
`scripts/test_hooks.py`は実際のtranscript行の形をfixtureとして固定している。**

## 判定できない場合は通す（PMの決定2・3）

境界が見つからない場合はfile全体を対象にする（tool_useを多く集める方向であり、
blockを減らす安全側である）。**transcriptを読めない、JSONとして壊れている、
想定外の形の場合はblockせず通す。**

`gh_metadata_guard.py`の「特定できない場合はdeny」（[#348](
https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/348)）とは逆の
方針だが、矛盾ではない。**分かれ目は「素通りの帰結が外に残るか」である。**
`gh_metadata_guard`が止めるのはIssue／Pull Requestの作成であり、素通りさせると
記載漏れが外部に残り取り返しがつかない。`Stop` hookが止めるのは自分の応答で
あり、素通りさせても失うのは1回の検出機会だけで取り返しがつく。誤検知で
作業が止まる代償のほうが大きい（指示書「誤検知は門を殺す」）。

## 無限loop防止

**同じ主張分類に対してblockするのは1回だけとし、2回目は通す**（PMの訂正3、
2026-09-04）。`stop_hook_active`は、担当セッションの実測で再block後もfalseの
ままだったため使わない。状態は`scratchpad_dir`配下のfileに持つ。
**セッション内で完結する場所であり、セッションをまたいで残る場所（repository内
等）へ書くと、前のセッションのblockが次のセッションを素通りさせる。**
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import command_line  # noqa: E402

STATE_FILENAME = ".stop_claim_guard_blocked_categories.json"

# 完了を主張する語の最小構成（丁寧形・過去形）。**最小で始める。**
# 各分類は、対応するtoolが実際に呼ばれていれば主張として問題にしない。
CLAIM_CATEGORIES = {
    "送信": {
        "phrases": ("送りました", "送信しました", "連絡しました", "送った", "送信した", "連絡した"),
        "tool": "mcp__ccd_session_mgmt__send_message",
    },
    "起票": {
        "phrases": ("起票しました", "起票した"),
        "gh_subcommand": ("issue", "create"),
    },
    "merge": {
        "phrases": ("mergeしました", "mergeした"),
        "gh_subcommand": ("pr", "merge"),
    },
    "push": {
        "phrases": ("pushしました", "pushした"),
        "git_subcommand": "push",
    },
}

# 引用として除外する括弧。**この組の中に現れた語は、自分の主張として数えない。**
QUOTE_PAIRS = (("「", "」"), ('"', '"'))


def _quoted_spans(text):
    """`text`のうち、引用括弧で囲まれた範囲の`(start, end)`を返す。"""
    spans = []
    for open_char, close_char in QUOTE_PAIRS:
        start = None
        for index, char in enumerate(text):
            if char == open_char and start is None:
                start = index
            elif char == close_char and start is not None:
                spans.append((start, index + 1))
                start = None
    return spans


def _is_inside(position, spans):
    return any(start <= position < end for start, end in spans)


def _detected_claims(text):
    """`text`から、証拠を要求する主張分類の集合を返す。

    引用の中に現れた語は数えない。否定形・未来形は、対象の語（丁寧形・過去形）
    と文字列として重ならないため、この完全一致検出だけで自然に除外される。
    """
    quoted = _quoted_spans(text)
    found = {}
    for category, spec in CLAIM_CATEGORIES.items():
        for phrase in spec["phrases"]:
            index = text.find(phrase)
            while index != -1:
                if not _is_inside(index, quoted):
                    found[category] = phrase
                    break
                index = text.find(phrase, index + 1)
            if category in found:
                break
    return found


def _read_transcript_entries(path):
    """transcriptの各行をparseした結果を返す。読めない行は`None`として保持する。

    **行単位で失敗を許す。**1行が壊れていても、他の行から境界とtool_useを
    拾えるなら拾う。file自体を開けない場合は`None`を返す。
    """
    try:
        with open(path, encoding="utf-8") as handle:
            lines = handle.readlines()
    except OSError:
        return None
    entries = []
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            entries.append(json.loads(line))
        except json.JSONDecodeError:
            entries.append(None)
    return entries


def _is_human_turn_entry(entry):
    """`entry`が人間の入力（toolの戻りではない）かを見る。

    判定の軸は「toolの戻りかどうか」である。`content`が文字列なら人間の入力。
    `content`がlistでも、`tool_result`以外のblock（`text`／`image`等）を
    1つでも含めば人間の入力として扱う。`tool_result`だけのlistはtoolの戻り。
    """
    if not isinstance(entry, dict) or entry.get("type") != "user":
        return False
    message = entry.get("message")
    if not isinstance(message, dict):
        return False
    content = message.get("content")
    if isinstance(content, str):
        return True
    if isinstance(content, list):
        return any(
            not isinstance(block, dict) or block.get("type") != "tool_result"
            for block in content
        )
    return False


def _tool_uses_since_last_human_turn(entries):
    """直近の人間の入力より後ろにある、全`tool_use`blockを返す。

    境界が見つからない場合はfile全体を対象にする。**tool_useを多く集める
    方向であり、blockを減らす安全側である。**
    """
    boundary = 0
    for index, entry in enumerate(entries):
        if _is_human_turn_entry(entry):
            boundary = index + 1
    tool_uses = []
    for entry in entries[boundary:]:
        if not isinstance(entry, dict) or entry.get("type") != "assistant":
            continue
        message = entry.get("message")
        if not isinstance(message, dict):
            continue
        content = message.get("content")
        if not isinstance(content, list):
            continue
        for block in content:
            if isinstance(block, dict) and block.get("type") == "tool_use":
                tool_uses.append(block)
    return tool_uses


def _gh_invocation_matches(command, subcommand):
    for args in command_line.invocations(command, "gh"):
        if tuple(args[:2]) == subcommand:
            return True
    return False


def _git_invocation_matches(command, subcommand):
    for args in command_line.invocations(command, "git"):
        if args[:1] == [subcommand]:
            return True
    return False


def _category_has_evidence(category, tool_uses):
    """`category`に対応するtool呼び出しが`tool_uses`にあるかを見る。"""
    spec = CLAIM_CATEGORIES[category]
    tool_name = spec.get("tool")
    for block in tool_uses:
        name = block.get("name")
        if tool_name is not None:
            if name == tool_name:
                return True
            continue
        if name != "Bash":
            continue
        tool_input = block.get("input")
        command = tool_input.get("command") if isinstance(tool_input, dict) else None
        if not isinstance(command, str):
            continue
        gh_subcommand = spec.get("gh_subcommand")
        if gh_subcommand is not None and _gh_invocation_matches(command, gh_subcommand):
            return True
        git_subcommand = spec.get("git_subcommand")
        if git_subcommand is not None and _git_invocation_matches(command, git_subcommand):
            return True
    return False


def _state_path(scratchpad_dir):
    if not scratchpad_dir:
        return None
    return Path(scratchpad_dir) / STATE_FILENAME


def _already_blocked_categories(state_path):
    if state_path is None or not state_path.exists():
        return set()
    try:
        return set(json.loads(state_path.read_text(encoding="utf-8")))
    except (OSError, json.JSONDecodeError, ValueError):
        return set()


def _record_blocked_category(state_path, category, already):
    if state_path is None:
        return
    already = set(already)
    already.add(category)
    try:
        state_path.parent.mkdir(parents=True, exist_ok=True)
        state_path.write_text(json.dumps(sorted(already)), encoding="utf-8")
    except OSError:
        # 状態を書けなくても、判定そのものは進める。**書けないことを理由に
        # 主張の検査自体を止めない。**次回も同じcategoryをblockするだけであり、
        # 無限loopにはならない（blockはユーザーの新しい入力を挟むため）。
        pass


def _block(category, phrase):
    json.dump({"decision": "block", "reason": (
        f"「{phrase}」と主張していますが、このターンで対応するtool呼び出しが"
        f"見当たりません（分類: {category}）。実際に実行してから主張するか、"
        "主張を取り下げてください。"
    )}, sys.stdout)
    sys.stdout.write("\n")


def main():
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError):
        return 0
    if not isinstance(payload, dict):
        return 0
    text = payload.get("last_assistant_message")
    if not isinstance(text, str) or not text:
        return 0
    claims = _detected_claims(text)
    if not claims:
        return 0

    transcript_path = payload.get("transcript_path")
    entries = _read_transcript_entries(transcript_path) if transcript_path else None
    if entries is None:
        # transcriptを読めない。**判定できないため通す。**素通りの帰結は
        # 検出機会を1回失うだけであり、外部へ何かが残るわけではない。
        return 0
    tool_uses = _tool_uses_since_last_human_turn(entries)

    scratchpad_dir = payload.get("scratchpad_dir")
    state_path = _state_path(scratchpad_dir)
    already_blocked = _already_blocked_categories(state_path)

    for category, phrase in claims.items():
        if category in already_blocked:
            continue
        if _category_has_evidence(category, tool_uses):
            continue
        _record_blocked_category(state_path, category, already_blocked)
        _block(category, phrase)
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
