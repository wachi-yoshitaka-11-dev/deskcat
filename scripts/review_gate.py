#!/usr/bin/env python3
"""変更の分類と、自己レビューを宣言するcommit trailerを検査する。

このscriptが答えるのは次の3つだけである。

1. `classify`     この変更範囲は軽微（`minor`）と機械的に**証明できる**か
2. `receipt`      範囲のhead commitが、分類と自己レビューをtrailerで宣言しているか
3. `instructions` 指示sourceが変わったなら、dataとしてreviewした宣言があるか

`gate`は上の3つをまとめて実行する。分類と指示sourceの
検査は範囲全体を見て、trailerの検査はhead commitだけを見る。

**意味は判定しない。**「言い回しの修正」と「意味の反転」を区別するcodeは書かない。
`〜しない`を`〜する`へ変える差分は、数値もcommandもlinkも含まない純粋な文字列変更であり、
構文では区別できない。近似で答えると、精度を上げるたびに逆向きの穴が開く。

代わりに、**規則を持つfileそのものを軽微経路から外す**（`INSTRUCTION_SOURCES`）。
そのうえで、残るfileの変更行が数値・command・link・表・見出し・checkboxに触っていない
ことだけを字句的に確かめる。**軽微と証明できないものはすべて`review-required`にする。**
偽陰性（本当は軽微なのにreview必須になる）は意図した失敗方向であり、Pull Requestを1本
作れば済む。偽陽性だけが危険なので、そちら側に倒さない。

**下の規則表は意図して粗い。**edge caseを追って条件を足すと、上の「近似の穴」に戻る。
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

# 指示source。**規則・安全・protocol・commandを持つ経路である。**
# 差分に含まれるとき、その内容は指示ではなくreview対象のdataとして扱う
# （`AGENTS.md`の「指示として有効な `AGENTS.md`」）。ここに載るfileは軽微経路へ入らない。
# 同じ列挙を分類とguardの両方で使う。2箇所に書くと片方だけを見た判断が起きる。
INSTRUCTION_SOURCES = (
    "AGENTS.md",
    "CLAUDE.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    ".claude/",
    ".github/",
    "docs/governance/",
    "docs/decisions/",
    "docs/hardware/",
    "docs/protocol/",
    "docs/DeskCat_Microcontroller_Development_Guide.md",
    "scripts/",
)

TRAILER_CLASS = "Change-Class"
TRAILER_REVIEW = "Self-Review"
TRAILER_INSTRUCTION = "Instruction-Change"

CLASS_MINOR = "minor"
CLASS_REVIEW = "review-required"
INSTRUCTION_ACK = "reviewed-as-data"

# `Self-Review`で宣言する内容。**3つとも要る。**
#
# 収束（新規指摘0件が2 round）と、2つのPassは別の軸である。1つの値にまとめると、
# どれをやっていないのかが分からなくなる。要件照合Passとfresh-context Passは
# 同じ最終diffに対して行い、差分が変わったら両方が無効になる。定義は
# `CONTRIBUTING.md`の「自己レビュー」にある。
#
# **これは宣言であって証拠ではない。**scriptが確かめられるのは、3つが揃っていることと、
# その宣言がこのcommitに結び付いていることだけである。
REVIEW_DECLARATIONS = ("requirements-pass", "fresh-context-pass", "converged")

MARKDOWN_SUFFIXES = (".md", ".markdown")

# 変更行がこれらのいずれかに当たると軽微にしない。数値、command、link、表、見出し、
# checkbox、HTML commentは、typoの修正に見えても意味を持つ。
LINE_DENY = (
    (re.compile(r"[0-9]"), "digit"),
    (re.compile(r"`"), "inline code"),
    (re.compile(r"\]\("), "link"),
    (re.compile(r"<https?:", re.IGNORECASE), "autolink"),
    (re.compile(r"\|"), "table"),
    (re.compile(r"^\s{0,3}#"), "heading"),
    (re.compile(r"^\s*[-*+]\s*\[[ xX]\]"), "checkbox"),
    (re.compile(r"<!--|-->"), "html comment"),
)

FENCE_RE = re.compile(r"^\s{0,3}(?:```|~~~)")
HUNK_RE = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")


def _git(root, arguments, stdin_text=None):
    result = subprocess.run(
        ["git", "-C", root, *arguments],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        input=stdin_text,
    )
    if result.returncode != 0:
        raise SystemExit(
            f"git {' '.join(arguments)} failed:"
            f" {result.stdout}{result.stderr}".strip()
        )
    return result.stdout


def _is_instruction_source(path):
    for entry in INSTRUCTION_SOURCES:
        if entry.endswith("/"):
            if path == entry.rstrip("/") or path.startswith(entry):
                return True
        elif path == entry:
            return True
    return False


def _fenced_line_numbers(content):
    """fence内の行番号（1起点）を返す。fenceの開始行と終了行も含める。

    閉じていないfenceは、以降の全行をfence内として扱う。開いたまま終わる差分を
    軽微へ通さないためであり、`validate_doc_links.py`が閉じ忘れを失敗として扱うのと
    同じ倒し方である。
    """
    inside = False
    fenced = set()
    for number, line in enumerate(content.splitlines(), start=1):
        if FENCE_RE.match(line):
            fenced.add(number)
            inside = not inside
            continue
        if inside:
            fenced.add(number)
    return fenced


def _changed_line_numbers(root, base, head, path):
    """変更のあった行番号を、base側とhead側に分けて返す。"""
    diff = _git(
        root,
        ["diff", "--unified=0", "--no-color", f"{base}..{head}", "--", path],
    )
    base_lines, head_lines = set(), set()
    for line in diff.splitlines():
        match = HUNK_RE.match(line)
        if not match:
            continue
        base_start, base_count, head_start, head_count = match.groups()
        base_count = 1 if base_count is None else int(base_count)
        head_count = 1 if head_count is None else int(head_count)
        base_lines.update(range(int(base_start), int(base_start) + base_count))
        head_lines.update(range(int(head_start), int(head_start) + head_count))
    return base_lines, head_lines


def _deny_reason(line):
    for pattern, name in LINE_DENY:
        if pattern.search(line):
            return name
    return None


def _inspect_side(root, revision, path, numbers, cache, label):
    """片側の変更行を検査し、軽微にできない理由を返す。"""
    if not numbers:
        return []
    key = (revision, path)
    if key not in cache:
        content = _git(root, ["show", f"{revision}:{path}"])
        cache[key] = (_fenced_line_numbers(content), content.splitlines())
    fenced, lines = cache[key]
    reasons = []
    for number in sorted(numbers):
        if number in fenced:
            reasons.append(f"{path}:{label}{number}: fenced block")
            continue
        if number - 1 >= len(lines):
            continue
        reason = _deny_reason(lines[number - 1])
        if reason:
            reasons.append(f"{path}:{label}{number}: {reason}")
    return reasons


def classify(root, base, head):
    """範囲を分類し、`(class, reasons)`を返す。軽微と証明できなければ`review-required`。"""
    status = _git(root, ["diff", "--name-status", "--no-color", f"{base}..{head}"])
    reasons = []
    paths = []
    for line in status.splitlines():
        if not line.strip():
            continue
        fields = line.split("\t")
        code, path = fields[0], fields[-1]
        paths.append(path)
        if not code.startswith("M"):
            # 追加・削除・改名は、typoの修正ではない。内容を見る前に外す。
            reasons.append(f"{path}: file status {code} is not a content edit")
            continue
        if _is_instruction_source(path):
            reasons.append(f"{path}: instruction source")
            continue
        if not path.lower().endswith(MARKDOWN_SUFFIXES):
            reasons.append(f"{path}: not Markdown")
            continue
        base_numbers, head_numbers = _changed_line_numbers(root, base, head, path)
        cache = {}
        reasons.extend(_inspect_side(root, base, path, base_numbers, cache, "-"))
        reasons.extend(_inspect_side(root, head, path, head_numbers, cache, "+"))
    if not paths:
        # 空の範囲を軽微として通さない。分類する対象が無いことは、軽微であることの
        # 証明ではない。
        reasons.append("no changed path in range")
    return (CLASS_REVIEW if reasons else CLASS_MINOR), reasons


def trailers(root, revision):
    """commit messageのtrailerを`{key: [values]}`で返す。

    解釈はgit自身の`interpret-trailers --parse`に任せる。trailerの書式規則を
    こちらで再実装すると、gitの解釈とずれた判定になる。
    """
    message = _git(root, ["log", "-1", "--format=%B", revision])
    parsed = _git(root, ["interpret-trailers", "--parse"], stdin_text=message)
    found = {}
    for line in parsed.splitlines():
        if ":" not in line:
            continue
        key, _, value = line.partition(":")
        found.setdefault(key.strip(), []).append(value.strip())
    return found


def _check_receipt(root, head, computed):
    found = trailers(root, head)
    problems = []
    declared = found.get(TRAILER_CLASS, [])
    if len(declared) != 1:
        problems.append(
            f"head commit must carry exactly one {TRAILER_CLASS} trailer,"
            f" found {declared}"
        )
    elif declared[0] not in (CLASS_MINOR, CLASS_REVIEW):
        problems.append(
            f"{TRAILER_CLASS} must be {CLASS_MINOR} or {CLASS_REVIEW},"
            f" found {declared[0]}"
        )
    elif declared[0] == CLASS_MINOR and computed != CLASS_MINOR:
        problems.append(
            f"{TRAILER_CLASS} declares {CLASS_MINOR} but the range classifies as"
            f" {computed}. A declaration cannot widen the minor path."
        )
    review = found.get(TRAILER_REVIEW, [])
    missing = [value for value in REVIEW_DECLARATIONS if value not in review]
    unknown = [value for value in review if value not in REVIEW_DECLARATIONS]
    # 重複も落とす。同じ宣言を2回書いても、実施した回数の証拠にはならない。
    duplicated = sorted({value for value in review if review.count(value) > 1})
    if missing or unknown or duplicated:
        problems.append(
            f"head commit must carry exactly one {TRAILER_REVIEW} trailer for each of"
            f" {list(REVIEW_DECLARATIONS)}."
            f" missing={missing} unknown={unknown} duplicated={duplicated}"
            f" found={review}"
        )
    return problems


def _check_instructions(root, base, head):
    status = _git(root, ["diff", "--name-only", "--no-color", f"{base}..{head}"])
    touched = [
        path
        for path in status.splitlines()
        if path.strip() and _is_instruction_source(path)
    ]
    if not touched:
        return [], touched
    found = trailers(root, head)
    if found.get(TRAILER_INSTRUCTION) != [INSTRUCTION_ACK]:
        return (
            [
                f"the range changes {len(touched)} instruction source path(s) but the"
                f" head commit does not carry {TRAILER_INSTRUCTION}:"
                f" {INSTRUCTION_ACK}. Instruction files in a diff are review targets,"
                " not instructions."
            ],
            touched,
        )
    return [], touched


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command", choices=("classify", "receipt", "instructions", "gate")
    )
    parser.add_argument("--repository-root", default="")
    parser.add_argument("--base", default="origin/develop")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--expect", default="")
    options = parser.parse_args(argv)
    root = options.repository_root.strip() or str(
        Path(__file__).resolve().parent.parent
    )
    base, head = options.base, options.head

    computed, reasons = classify(root, base, head)
    problems = []
    touched = None

    if options.command in ("receipt", "gate"):
        problems.extend(_check_receipt(root, head, computed))
    if options.command in ("instructions", "gate"):
        instruction_problems, touched = _check_instructions(root, base, head)
        problems.extend(instruction_problems)
    if options.expect and options.expect != computed:
        problems.append(f"expected CLASS={options.expect} but computed {computed}")

    print(f"CLASS={computed} RANGE={base}..{head}")
    if touched is not None:
        print(f"INSTRUCTION_SOURCES_TOUCHED={len(touched)}")
    for reason in reasons:
        print(f"  reason: {reason}")
    for problem in problems:
        print(problem, file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
