#!/usr/bin/env python3
"""変更の分類と、自己レビューを宣言するcommit trailerを検査する。

このscriptが答えるのは次の問いだけである。

1. `classify`     この変更範囲は軽微（`minor`）と機械的に**証明できる**か
2. `receipt`      範囲のhead commitが、分類と自己レビューをtrailerで宣言しているか
3. `instructions` 指示sourceが変わったなら、dataとしてreviewした宣言があるか
4. `history`      範囲の**各commit**が分類を宣言しているか

`gate`は1から3をまとめて実行する。分類と指示sourceの検査は範囲全体を見て、trailerの
検査はhead commitだけを見る。

**`history`は`gate`に含めない。**feature branchの中間commitにまで宣言を要求すると、
最後にまとめて宣言する運用が成立しない。`main`昇格では範囲に複数のsquash commitが
入るため、そこでだけ各commitを見る。

**意味は判定しない。**「言い回しの修正」と「意味の反転」を区別するcodeは書かない。
`〜しない`を`〜する`へ変える差分は、数値もcommandもlinkも含まない純粋な文字列変更であり、
構文では区別できない。近似で答えると、精度を上げるたびに逆向きの穴が開く。

代わりに、**規則を持つfileそのものを軽微経路から外す**（`INSTRUCTION_SOURCES`）。
そのうえで、残るfileの変更行が数値・command・link・表・見出し・checkboxに触っていない
ことだけを字句的に確かめる。**軽微と証明できないものはすべて`review-required`にする。**
偽陰性（本当は軽微なのにreview必須になる）は意図した失敗方向であり、Pull Requestを1本
作れば済む。偽陽性だけが危険なので、そちら側に倒さない。

**下の規則表は意図して粗い。**edge caseを追って条件を足すと、上の「近似の穴」に戻る。

**`fixup`は宣言専用の区分であり、`classify`は返さない。**「既にmergeされreviewを通った
作業の後始末である」は申告であって、差分からは読めない。機械が確かめるのは整合だけで、
`Refs`で後始末の対象を示していることを要求する。**`minor`と`review-required`の判定は
どちらも変えていない**（[ADR-0015](../docs/decisions/0015-fixup-class-and-direct-commit-scope.md)）。
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
TRAILER_REFS = "Refs"

CLASS_MINOR = "minor"
CLASS_REVIEW = "review-required"
CLASS_FIXUP = "fixup"
INSTRUCTION_ACK = "reviewed-as-data"

# `Change-Class`が取りうる値。**宣言の検査はこの集合だけを見る。**
#
# 以前は`(CLASS_MINOR, CLASS_REVIEW)`というtupleを`_check_receipt`と`_check_history`へ
# 別々に埋め込んでいた。値を足すときに片方だけを直す事故が起きる形だったため集約した。
CLASS_VALUES = (CLASS_MINOR, CLASS_REVIEW, CLASS_FIXUP)

# `CLASS_FIXUP`は**宣言専用の区分である。`classify`は返さない。**
#
# `classify`が返すのは`minor`と`review-required`だけである。`fixup`が意味するのは
# 「既にmergeされreviewを通った作業の後始末である」という**申告**であり、
# 差分からは読めない。人間の意図（新しい判断か、前の判断の後始末か）は字句に現れない。
#
# 機械で確かめられるのは整合だけである。**`fixup`を宣言したら`Refs`で対象を示すことを
# 要求する。**後始末である以上、後始末の対象が存在する。示せないものは後始末ではない。
#
# **`minor`の判定を緩めない。**`fixup`はminor経路を主張しないため、
# 「宣言が計算結果より緩くない」検査（`minor`に対するもの）の外側に置く。
# **`review-required`の判定も緩めない。**`fixup`は`classify`の出力を変えない。
#
# 判定を機械化せず申告に寄せた理由は
# [ADR-0015](../docs/decisions/0015-fixup-class-and-direct-commit-scope.md)にある。

# `Refs`が対象を示しているかの判定。**番号を含むことだけを見る。**
# その番号が実在するか、正しい対象かは判定しない。**示していないことだけを落とす。**
#
# **`Refs`はtrailerの形（`Refs: #204`）で書く必要がある。**`git interpret-trailers`は
# コロンの無い行をtrailerとして読まない。この repositoryのcommit messageは本文の段落として
# `Refs #204`と書いてきたが、それはtrailerではない。**さらにコロンの無い行をtrailer blockと
# 同じ段落へ置くと、blockごと無効になり`Change-Class`も`Self-Review`も消える。**
# 落とし穴はCONTRIBUTINGの「Merge方式」に書いた。
FIXUP_REFERENCE_RE = re.compile(r"#\d+")

# `Self-Review`で宣言する内容。**下の値がすべて要る。**
#
# 収束（新規指摘0件が2 round）と、2つのPassは別の軸である。1つの値にまとめると、
# どれをやっていないのかが分からなくなる。要件照合Passとfresh-context Passは
# 同じ最終diffに対して行い、差分が変わったら両方が無効になる。定義は
# `CONTRIBUTING.md`の「自己レビュー」にある。
#
# **これは宣言であって証拠ではない。**scriptが確かめられるのは、下の値が揃っていることと、
# その宣言がこのcommitに結び付いていることだけである。
REVIEW_DECLARATIONS = ("requirements-pass", "fresh-context-pass", "converged")

MARKDOWN_SUFFIXES = (".md", ".markdown")

# 理由を並べる上限。`main`昇格の範囲では数百行になり、宣言の問題が埋もれる。
# 打ち切った件数は必ず出す。silent capを作らない。
REASON_PREVIEW_LIMIT = 20

# 宣言を求める範囲の起点。trailer運用を導入したcommitである（PR #161のsquash）。
# これより前のcommitは検査しない。宣言を求める規則が存在しなかった。
DECLARATION_CUTOVER = "57734371384d18f31de7557a7a60fd1aa856edff"

# 起点より後だが、宣言を持たないことを許すcommit。
#
# `AGENTS.md`が共有branchの履歴書き換えを禁じているため、後からtrailerを付けられない。
# **いずれも`develop`へmerge済みであり、共有履歴である。**
# 中身はいずれもPull Requestのreviewを通っており、失われるのは宣言の記録である。
#
# **ただし免除は`Change-Class`と`Self-Review`だけを飛ばすのではない。**`history`は
# 免除commitで`continue`するため、`_check_instructions`も掛からない。**指示source変更の
# 宣言（`Instruction-Change`）の検査まで抑止する。**
# 実際に効いているのは`9c91f913`と`b71c7ef`である（どちらも`docs/hardware/`を触りながら
# `Instruction-Change`を持たない）。`18298ae`と`619c843`と`1a5dda8`は`INSTRUCTION_SOURCES`
# のpathを1つも触らないため、この抑止は掛かっていない。
#
# **理由は1つではない。原因ごとに書き分ける。**同じ「後から付けられない」で
# まとめると、再発を止める手がかりが消える。
#
# - `9c91f913`（PR #160）。**Gateがrequired status checkでなかった期間にmergeされた。**
#   規則を強制する仕組みが無かった。
# - `18298ae`（PR #191）、`619c843`（PR #196）。**規則も強制も揃っていた。**
#   head commit（`57c099b`、`e1553a1`）は`Change-Class`と3値の`Self-Review`を持ち、
#   Gateの`Verify change class and self-review`はどちらも`success`だった。
#   `gh pr merge --squash`をmessage指定なしで実行したため、GitHubが合成したmessageへ
#   trailerが入らず、squash commitへ引き継がれなかった。**書き忘れである。**
#   確認手順はCONTRIBUTINGの「Merge方式」にある。
# - `b93b309`（#227）。既存の2種類と別の原因である。
#   **規則も強制も揃っていたが、強制が掛かる経路を通らないcommitだった。**
#   `Change-Class: fixup`による`develop`への直接pushであり、Pull Requestを通らないため
#   `review-gate.yml`の`gate`が走らない。push前に実行したのが`receipt`だけで、
#   `receipt`はhead commitのtrailerの整合しか見ず`_check_instructions`を呼ばない。
#   `gate`なら落ちていた（`receipt` exit 0／`gate` exit 1）。**subcommandの選び間違いである。**
#   **これは`fixup`区分そのものの欠陥ではない。**直接commit経路に、押す瞬間の検査が
#   無かったことによる。同じ形を止めるため`scripts/hooks/push_gate.py`を入れた。
#
# - `b71c7ef`（PR #258）。**`18298ae`／`619c843`と同じ原因である。**
#   head commit（`63fe1b3`）は`Change-Class`、3値の`Self-Review`、`Instruction-Change`、
#   `Refs: #257`をすべて持ち、Gateの`Verify change class and self-review`は`success`だった。
#   squash merge時にmessageへtrailerが載らず、squash commitへ引き継がれなかった。
#   **中身のreviewは通っており、失われたのは宣言の記録だけである。**
#
#   **前2件と違う点が2つある。**
#
#   1. **`docs/hardware/`を3 file触る。**そのため免除は`Instruction-Change`の検査まで
#      抑止する（`9c91f913`と同じ側になる）。**head commitはその宣言を持っていた**ため、
#      抑止されるのは記録であって判断ではない。
#   2. **`gh_metadata_guard.py`が`gh pr merge`のsquash messageを検査する状態で起きた。**
#      同hookは`Change-Class`と`Self-Review`を持たないmessageを`deny`する。
#      **それでも止まらなかった。**hookはBash tool経由の`gh pr merge`だけを見るため、
#      別の経路を通ったと考えられるが、**経路は特定していない。推測で書かない。**
#      **「hookを入れたから同じ型は起きない」とは言えない、という記録である。**
#
# - `1a5dda8`（Refs #6）。**既存5件のどれとも原因が違う。**
#   `Change-Class: fixup`による`develop`への直接pushで、**push前に`gate`を実行し
#   exit 0を確認していた。**しかし`Refs: #6`と`Co-Authored-By`の行の間に空行を
#   入れてしまい、trailerの段落が2つに割れた。`git interpret-trailers --parse`は
#   最後の段落しかtrailerと見なさないため、`Change-Class`と`Refs`を含む先頭の
#   段落がまるごと無効になった（`Self-Review`はこのcommitではまだ書いておらず、
#   元から無い）。続くcommitで`Self-Review`を足した際に空行の問題へ気づき、
#   **その後続commitだけ**`git commit --amend`で直した（rebase後`fb60e31`。
#   trailerは正しく認識される）。**`1a5dda8`自身は直っていない。**
#   **`gate`は間違っていない。**Pull RequestのRequired status checkとしてhead
#   commit 1本を見る設計であり、それは正しい。**欠けていたのは、複数commitを
#   まとめて`fixup`で直接pushする場面で`history`も合わせて確認するという
#   運用手順だった。**`docs/toolchains/version-records/`のみを変更しており
#   `INSTRUCTION_SOURCES`を触っていないため、`Instruction-Change`の抑止は
#   このentryでは掛からない（`18298ae`／`619c843`と同じ側）。
#
# **この列挙を増やさない。**増やす変更は`scripts/`の変更であり、reviewと
# `Instruction-Change`の宣言を通る。通したうえで増やすなら、それは判断である。
DECLARATION_EXEMPT = (
    "9c91f913696033ca3da9b26d10ac793ee2c2291e",
    "18298ae3127f31a81411a6c723122dad17a91299",
    "619c8439be8489451ec9f6a9b79613bc01c1605d",
    "b93b309c7f6a39967d2eb3ba62807bc4bb1a5dfe",
    "b71c7ef9e58240d71667c95b551b113288ee450f",
    "1a5dda877e4994309cd35dd033d66881ea431a2f",
)

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


def _merge_base(root, base, head):
    """diffの起点をmerge baseへ解決する。

    **`git diff A..B`は端点間の差分であり、merge baseを起点にしない**（3点diffは
    `A...B`である）。`.github/workflows/review-gate.yml`が渡すのは
    `github.event.pull_request.base.sha`＝**base branchのtip**であって、merge base
    ではない。したがってbranch点より後にbase branchへ入ったcommitが、逆向きの変更として
    範囲へ混ざる。影響は2方向ある。

    - 偽陽性: base側だけが指示sourceを触っていると、Pull Requestが触っていないfileに対して
      `Instruction-Change`を要求し、gateが落ちる
    - 偽陰性: base側の変更が指示sourceをhead側と同じ内容にすると、端点diffに現れず、
      宣言の要求そのものが消える

    **3点diffへ書き換えるだけでは足りない。**`_inspect_side`がbase側の行番号を
    `git show {revision}:{path}`で読むため、diffの起点と`git show`の起点が食い違うと
    行番号がずれる。**commitを1回解決し、diffと`git show`の両方で同じものを使う。**

    `_check_history`の`git rev-list base..head`は「headから辿れてbaseから辿れない
    commit」であり、既にmerge base相当の意味を持つ。**そちらは変換しない。**

    共通の祖先が無い場合は`base`をそのまま返す。無関係なhistory同士では`merge-base`が
    失敗するため、そこで検査を止めない。
    """
    result = subprocess.run(
        ["git", "-C", root, "merge-base", base, head],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if result.returncode != 0:
        return base
    return result.stdout.strip() or base


def _changed_line_numbers(root, base, head, path):
    """変更のあった行番号を、base側とhead側に分けて返す。

    `base`は`_merge_base`で解決済みのcommitを受け取る。呼び出し側で解決するのは、
    `_inspect_side`の`git show`と同じ起点を使うためである。
    """
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
    """範囲を分類し、`(class, reasons)`を返す。軽微と証明できなければ`review-required`。

    **返すのは`CLASS_MINOR`と`CLASS_REVIEW`だけである。`CLASS_FIXUP`は返さない。**
    後始末かどうかは差分から読めないため、計算結果には現れない。宣言だけが持つ。
    """
    # 起点をここで1回だけ解決する。以降の`git diff`と`_inspect_side`の`git show`が
    # 同じcommitを見る（理由は`_merge_base`）。
    base = _merge_base(root, base, head)
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


def _check_fixup_reference(found, subject):
    """`fixup`の宣言が、後始末の対象を`Refs`で示しているかを検査する。

    **機械で確かめられるのはここだけである。**その差分が本当に後始末かは判定しない。
    示された対象が正しいかも判定しない。**示していないことだけを落とす。**
    """
    references = found.get(TRAILER_REFS, [])
    if not any(FIXUP_REFERENCE_RE.search(value) for value in references):
        return [
            f"{subject} declares {TRAILER_CLASS}: {CLASS_FIXUP} but carries no"
            f" {TRAILER_REFS} trailer naming the Issue or Pull Request being"
            f" cleaned up. found {TRAILER_REFS}={references}."
            " A fixup without a target is not a fixup."
        ]
    return []


def _check_receipt(root, head, computed):
    found = trailers(root, head)
    problems = []
    declared = found.get(TRAILER_CLASS, [])
    if len(declared) != 1:
        problems.append(
            f"head commit must carry exactly one {TRAILER_CLASS} trailer,"
            f" found {declared}"
        )
    elif declared[0] not in CLASS_VALUES:
        problems.append(
            f"{TRAILER_CLASS} must be one of {list(CLASS_VALUES)},"
            f" found {declared[0]}"
        )
    elif declared[0] == CLASS_MINOR and computed != CLASS_MINOR:
        problems.append(
            f"{TRAILER_CLASS} declares {CLASS_MINOR} but the range classifies as"
            f" {computed}. A declaration cannot widen the minor path."
        )
    elif declared[0] == CLASS_FIXUP:
        problems.extend(_check_fixup_reference(found, "head commit"))
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
    # `classify`と同じ理由で起点を解決する。ここを端点diffのままにすると、base側だけの
    # 指示source変更に対して宣言を要求し、逆にbase側と内容が一致した変更を見落とす。
    base = _merge_base(root, base, head)
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
                f"the range changes {len(touched)} instruction source path(s) but"
                f" {head} does not carry {TRAILER_INSTRUCTION}: {INSTRUCTION_ACK}."
                " Instruction files in a diff are review targets,"
                " not instructions."
            ],
            touched,
        )
    return [], touched


def _rev_exists(root, revision):
    result = subprocess.run(
        ["git", "-C", root, "rev-parse", "--verify", "--quiet",
         f"{revision}^{{commit}}"],
        capture_output=True,
    )
    return result.returncode == 0


def _check_history(root, base, head, cutover):
    """範囲の各commitが分類を宣言しているかを検査する。

    head commitだけを見ると、**宣言を持たないcommitが範囲の中に混ざっていても通る。**
    `main`昇格では範囲に複数のsquash commitが入るため、1つずつ見る。

    要求はhead commitより軽い。**`Change-Class`が計算結果より緩くないこと**と、
    `Self-Review`が1つ以上あることだけを見る。`Self-Review`の値の集合は時期によって
    変わっており、過去のcommitを現在の集合で測ると、当時は正しかった宣言が落ちる。
    現在の集合はhead commitに対してだけ適用する（`_check_receipt`）。

    起点より前のcommitは検査しない。宣言を求める規則が存在しなかった。
    """
    if not _rev_exists(root, cutover):
        # 起点がこのrepositoryに無い。fixtureや別historyでは検査しない。
        return [], None
    listed = _git(
        root, ["rev-list", "--no-merges", f"{base}..{head}", "--not", cutover]
    ).split()
    with_merges = _git(
        root, ["rev-list", f"{base}..{head}", "--not", cutover]
    ).split()

    problems = []
    exempt = 0
    for commit in listed:
        if commit in DECLARATION_EXEMPT:
            exempt += 1
            continue
        short = commit[:7]
        computed, _ = classify(root, f"{commit}^", commit)
        found = trailers(root, commit)
        declared = found.get(TRAILER_CLASS, [])
        if len(declared) != 1 or declared[0] not in CLASS_VALUES:
            problems.append(
                f"{short} must carry exactly one valid {TRAILER_CLASS} trailer,"
                f" found {declared}"
            )
        elif declared[0] == CLASS_MINOR and computed != CLASS_MINOR:
            problems.append(
                f"{short} declares {CLASS_MINOR} but classifies as {computed}"
            )
        elif declared[0] == CLASS_FIXUP:
            # 直接commitは`gate`を通らないため、`fixup`の整合はここで初めて検査される。
            problems.extend(_check_fixup_reference(found, short))
        if not found.get(TRAILER_REVIEW):
            problems.append(f"{short} carries no {TRAILER_REVIEW} trailer")
        instruction_problems, _ = _check_instructions(root, f"{commit}^", commit)
        problems.extend(instruction_problems)
    # 数えたものと数えなかったものを必ず出す。silent capを作らない。
    summary = (len(listed) - exempt, len(with_merges) - len(listed), exempt)
    return problems, summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command", choices=("classify", "receipt", "instructions", "history", "gate")
    )
    parser.add_argument("--repository-root", default="")
    parser.add_argument("--base", default="origin/develop")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--expect", default="")
    # 起点の既定は`DECLARATION_CUTOVER`である。上書きはtestとdry runのためにある。
    parser.add_argument("--since", default="")
    options = parser.parse_args(argv)
    root = options.repository_root.strip() or str(
        Path(__file__).resolve().parent.parent
    )
    base, head = options.base, options.head

    computed, reasons = classify(root, base, head)
    problems = []
    touched = None
    history = None

    if options.command in ("receipt", "gate"):
        problems.extend(_check_receipt(root, head, computed))
    if options.command in ("instructions", "gate"):
        instruction_problems, touched = _check_instructions(root, base, head)
        problems.extend(instruction_problems)
    if options.command == "history":
        history_problems, history = _check_history(
            root, base, head, options.since.strip() or DECLARATION_CUTOVER
        )
        problems.extend(history_problems)
    if options.expect and options.expect != computed:
        problems.append(f"expected CLASS={options.expect} but computed {computed}")

    print(f"CLASS={computed} RANGE={base}..{head}")
    if computed == CLASS_REVIEW:
        print(
            f"  meaning: {CLASS_REVIEW} means the {CLASS_MINOR} path is unavailable,"
            " not that a CodeRabbit review is required"
        )
    if touched is not None:
        print(f"INSTRUCTION_SOURCES_TOUCHED={len(touched)}")
    if history is None and options.command == "history":
        print("HISTORY=not-checked (declaration cutover is not in this history)")
    elif history is not None:
        print(
            f"HISTORY_CHECKED={history[0]} MERGES_SKIPPED={history[1]}"
            f" EXEMPT={history[2]}"
        )
    for reason in reasons[:REASON_PREVIEW_LIMIT]:
        print(f"  reason: {reason}")
    hidden = len(reasons) - REASON_PREVIEW_LIMIT
    if hidden > 0:
        print(f"  ({hidden} more reason(s) not listed)")
    for problem in problems:
        print(problem, file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
