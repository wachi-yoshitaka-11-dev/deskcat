#!/usr/bin/env python3
"""配線の記録が変わったPull Requestで、ブレッドボード配線図の更新を要求する。

`hardware/README.md`は「配線を変えるPRは、同じPRでKiCad回路図とブレッドボード配線図も
更新する」と定めていた。ところが`EXP-026`〜`EXP-034`の配線の変更は、
`docs/hardware/experiment-log.md`の`Wiring revision`の行に文字で書かれただけで、
図は2026-10-20の[#624](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/624)まで
更新されなかった（[#512](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/512)）。
**規則を文書に書くだけでは守られなかったため、機械で止める。**

検出は字句だけで行う。`base`と`head`の差分に、`experiment-log.md`の`| Wiring revision |`で
始まる行の**追加または変更**があり、`hardware/breadboard/layout.svg`が差分に無ければ、
図の更新漏れとして落とす。意味は判定しない。

図に関係しない変更（配線を変えていない試験の記録など）は、**理由を書いた明示の宣言**が
あるときだけ通す。宣言は次のどちらかで、理由が`MIN_REASON_LENGTH`字未満なら受け付けない。

- head commit messageのtrailer `Wiring-Figure: not-applicable <理由>`
- Pull Request本文の行 `Wiring-Figure: not-applicable <理由>`

`kicad_sch`は要求しない。穴位置や線色の変更は論理回路を変えないためである。論理接続が
変わる場合は`hardware/README.md`のとおり同じPRで更新する（この検査の対象外）。
"""

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import review_gate  # noqa: E402

LOG_PATH = "docs/hardware/experiment-log.md"
FIGURE_PATH = "hardware/breadboard/layout.svg"

WIRING_ROW_PREFIX = "| Wiring revision |"

TRAILER_NAME = "Wiring-Figure"
DECLARATION_VALUE = "not-applicable"
MIN_REASON_LENGTH = 10

BODY_DECLARATION_RE = re.compile(
    rf"^[ \t]*{TRAILER_NAME}:[ \t]*{DECLARATION_VALUE}[ \t]+(?P<reason>\S.*?)[ \t]*$",
    re.MULTILINE,
)


def changed_wiring_rows(root, merge_base, head):
    """`experiment-log.md`で追加・変更された`Wiring revision`の行を返す。"""
    diff = review_gate._git(
        root,
        ["diff", "--unified=0", "--no-color", merge_base, head, "--", LOG_PATH],
    )
    return [
        line[1:]
        for line in diff.splitlines()
        if line.startswith("+")
        and not line.startswith("+++")
        and line[1:].startswith(WIRING_ROW_PREFIX)
    ]


def figure_changed(root, merge_base, head):
    names = review_gate._git(
        root, ["diff", "--name-only", merge_base, head, "--", FIGURE_PATH]
    )
    return bool(names.strip())


def _valid_reason(reason):
    """理由として受け付けるか。長さが足り、`<...>`の雛形のままでないこと。

    **雛形を貼っただけの宣言は、理由なしと同じである。**`CONTRIBUTING.md`の書式の例
    （`<配線を変えていない理由>`）は10字を超えるため、長さだけでは通ってしまう。
    """
    reason = reason.strip()
    if len(reason) < MIN_REASON_LENGTH:
        return False
    return not (reason.startswith("<") and reason.endswith(">"))


def declared_reason(root, head, pr_body):
    """理由つきの宣言があればその理由を返す。無い、または理由が短ければ`None`。"""
    for value in review_gate.trailers(root, head).get(TRAILER_NAME, []):
        prefix, _, reason = value.partition(" ")
        if prefix == DECLARATION_VALUE and _valid_reason(reason):
            return reason.strip()
    for match in BODY_DECLARATION_RE.finditer(pr_body or ""):
        if _valid_reason(match.group("reason")):
            return match.group("reason")
    return None


def check(root, base, head, pr_body=""):
    """`(ok, message)`を返す。"""
    merge_base = review_gate._merge_base(root, base, head)
    rows = changed_wiring_rows(root, merge_base, head)
    if not rows:
        return True, "Wiring revision の行の追加・変更は無い。検査対象外。"
    if figure_changed(root, merge_base, head):
        return True, f"{FIGURE_PATH} が同じ範囲で更新されている。"
    reason = declared_reason(root, head, pr_body)
    if reason is not None:
        return True, f"{FIGURE_PATH} は更新していないが、理由つきの宣言がある: {reason}"
    return False, (
        f"{LOG_PATH} の `Wiring revision` の行が {len(rows)} 行追加・変更されたが、"
        f"{FIGURE_PATH} が更新されていない。\n"
        "配線を変えたなら、同じPRで配線図を更新する（hardware/README.md）。\n"
        "図に関係しない変更なら、head commit のtrailer または PR 本文に、"
        f"`{TRAILER_NAME}: {DECLARATION_VALUE} <理由（{MIN_REASON_LENGTH}字以上）>` を書く。"
    )


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", default="")
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument(
        "--pr-body-env",
        default="",
        help="Pull Request本文を持つ環境変数の名前。shellへ展開せず環境変数で受ける。",
    )
    args = parser.parse_args(argv)
    root = args.repository_root or str(Path(__file__).resolve().parent.parent)
    pr_body = os.environ.get(args.pr_body_env, "") if args.pr_body_env else ""
    ok, message = check(root, args.base, args.head, pr_body)
    print(message, file=sys.stdout if ok else sys.stderr)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
