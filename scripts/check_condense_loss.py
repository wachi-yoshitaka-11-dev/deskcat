#!/usr/bin/env python3
"""書き直しの前後で、文書から消えた値・ID・Issue番号・URL・`code`を一覧にする（#806）。

文書は決定された最終形だけを残す。書き直しでは、置き換わった古い値や経緯の中の値が消えるのが
正しい。一方で、**今も効いている値や、安全要件5項目に効く値とその根拠へのlinkを誤って消す**
危険がある。文の意味（条件の限定）の脱落は機械で検出できないが、値・ID・番号・URL・codeの
消失は機械で一覧にできる。PRを読む人は、その一覧が**置き換わった値だけ**であることを確かめる。

- 対象は、base（`--base`とHEADのmerge-base）からHEADまでに内容が変わった、両方に存在する
  Markdown。新規・削除・改名のfileは照合しない（消したfileの値は一覧に出さない）。
- LOST: baseの文書にあり、HEADの同じ文書に無いtoken。**一覧を出すだけで、失敗にしない**
  （書き直しでは置き換わった古い値が消えるのが正しいため）。各PRの本文へ貼り、置き換わった値
  だけであることを書く。
- ADDED: HEADの同じ文書にあり、baseに無いtoken。書き直しが値を作った疑いとして表示する。
- tokenは正規表現で取る。code spanは丸ごと1つ（80文字以内で、link記法`](`を含まないもの。
  原文の閉じ忘れのbacktickが作る、離れた2つのspanにまたがる長い断片をtokenにしない）、
  英字と数字を両方含む語（`GPIO12`・`SG90`・`2SJ334`・`M-12001`・`HW-TBD-034`・`AWG22`・
  `LD1117-3.3`・`0x76`。`-`と`.`でつないだ複合語は1つ）、`#番号`、単位つきの数値（桁区切りの
  カンマを含む。長い単位を先に当てる。負号つき）、単位を持たない小数と3桁以上の整数（年を除く）、
  範囲の下限、URL。単位も小数点も持たない1〜2桁の整数は、個数などの普通の数が大半のため取らない。
  **これは文の意味の照合ではない。**
- 終了codeは、照合できた場合が0、baseを解決できない・gitが失敗した場合が2。
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT))

# 単位は`(接頭辞)(基本単位)`で取る（µA・µF・nF・pF・MHz・MΩ・us・ms…）。長い単位を先に並べる
# （`mm`・`min`は接頭辞つきの1字単位より先）。直後が英数字なら単位としない
# （`5 samples`の`5 s`、`3V3`の`3V`を拾わない）。`20k`のようにΩを省いた書き方、`250 VAC`・
# `5 VDC`、通信速度（baud・bps）、容量（mAh・kB）、角度・時間・個数（度・deg・秒・count・倍）、
# 気圧（hPa・Pa）、長さ（m・cm）も取る。日本語の単位（度・秒・倍）の前後は英数字だけで判定する。
_UNIT = (
    r"(?:mm²|mm|cm|sq|min|℃/W|℃|°C|°|%RH|%|baud|ppm|dB|rpm|mAh|Ah|Wh|[KMGkT]i?B|B|[Bb]ytes?"
    r"|bits?|ch|mg/LSB|LSB/g|LSB|[kMG]?bps|[kMG]?SPS|[kM]?Sample/s|MSa/s|[mk]?g|度|deg|秒|倍"
    r"|count|[hk]?Pa|[kMGmµμ]?(?:Ω|Hz)|(?:[mµμnpuk]?[Vv]|[mµμnpuk]?A)(?:AC|DC)?"
    r"|[mµμnpu]?[FWHs]|k|m)"
)
# 数値は桁区切りのカンマを含めて1つにする（`1,100 mA`を`100 mA`へ切らない）。
# 左が英数字・カンマ・小数点なら始めない（数値の途中から始めない）。
_NUMBER = (
    r"(?:(?<!\w)-|−)?(?<![A-Za-z0-9]-)(?:(?<![A-Za-z\d,.])|(?<=DC)|(?<=AC))(?:\d{1,3}(?:,\d{3})+(?:\.\d+)?|\d+(?:\.\d+)?)"
)
# 範囲の区切り（U+301C「〜」、U+FF5E「～」、`~`、`–`、`−`、`-`）。
_RANGE = r"[〜～~–−-]"
# 語・数の境界は英数字だけで判定する。Pythonの`\w`・`\b`はCJK文字を語の文字と見なすため、
# 日本語に隣接した`HW-TBD-035の`や`1000秒`が壊れる。
_PATTERNS = (
    re.compile(r"`(?:(?!\]\()[^`\n]){1,80}`"),
    # 英字と数字を両方含む語（`GPIO12`・`SG90`・`2SJ334`・`0x76`・`3V3`・`M-12001`・
    # `HW-TBD-034`・`LD1117-3.3`）。`-`と`.`でつないだ複合語も1つにする。
    re.compile(
        r"(?<![A-Za-z0-9])(?<!\d\.)(?=[A-Za-z0-9.-]*\d)(?=[A-Za-z0-9.-]*[A-Za-z])"
        r"[A-Za-z0-9]+(?:[-.][A-Za-z0-9]+)*(?![A-Za-z0-9])"
    ),
    # 数字を含まない大文字のハイフン複合語。
    re.compile(r"(?<![A-Za-z0-9])[A-Z][A-Z0-9]+(?:-[A-Z0-9]+)+(?![A-Za-z0-9]|\.\d)"),
    re.compile(r"#\d+"),
    re.compile(_NUMBER + r"\s?" + _UNIT + r"(?![A-Za-z0-9])"),
    # 単位を持たない小数（`0.073`・`4.80`・`0.75×VDD`の`0.75`）。
    re.compile(r"(?:(?<!\w)-|−)?(?<![A-Za-z0-9]-)(?<![A-Za-z\d,.])\d+\.\d+(?![A-Za-z0-9]|\.\d)"),
    # 単位を持たない3桁以上の整数（通信速度`115200`、秋月の通販番号`110972`、`Figure 31-365`の
    # `365`など）。年（20xx）は除く。
    re.compile(r"(?<![A-Za-z0-9.,])(?!20\d\d(?!\d))\d{3,}(?![A-Za-z0-9]|[.,]\d)"),
    # 桁区切りのカンマつきの数で単位を持たないもの（`62,488,704`）。
    re.compile(r"(?<![A-Za-z0-9.,])\d{1,3}(?:,\d{3})+(?![A-Za-z0-9]|[.,]\d)"),
    # 版・節の番号（`8.0.9`・`24.5.2`）。小数点で3つ以上つないだ数。
    re.compile(r"(?<![A-Za-z0-9.])\d+(?:\.\d+){2,}(?![A-Za-z0-9]|\.\d)"),
    # ブレッドボードの穴の座標（`(4,12)`）。基板の配線の位置を決める値である。
    re.compile(r"\(\s?\d{1,3}\s?,\s?\d{1,3}\s?\)"),
    # 範囲`A〜B unit`・`A–B unit`の下限A。単位は最後の数値にしか付かない。
    re.compile(
        _NUMBER + r"(?:\s?" + _UNIT + r")?(?=\s?" + _RANGE + r"\s?" + _NUMBER + r"\s?" + _UNIT
        + r"(?![A-Za-z0-9]))"
    ),
    re.compile(r"https?://[^\s)）\]>]+"),
)


def tokens(text):
    found = set()
    for pattern in _PATTERNS:
        found.update(m.group(0) for m in pattern.finditer(text))
    return found


def compare(old, new):
    """`(消えた, 新規)`を返す。引数は本文。"""
    old_tokens = tokens(old)
    new_tokens = tokens(new)
    return sorted(old_tokens - new_tokens), sorted(new_tokens - old_tokens)


def _git(root, *arguments):
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    return result


def _show(root, ref, relative):
    """`ref`時点のfileの本文。読めなければ`None`（呼び出し側が終了code 2にする）。"""
    result = _git(root, "show", f"{ref}:{relative}")
    return result.stdout if result.returncode == 0 else None


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", default="")
    parser.add_argument("--base", required=True, help="比較元のrefまたはsha")
    parser.add_argument("--head", default="HEAD")
    arguments = parser.parse_args(argv)
    root = Path(arguments.repository_root or SCRIPTS_ROOT.parent).resolve()

    merge_base = _git(root, "merge-base", arguments.base, arguments.head)
    if merge_base.returncode != 0:
        print(f"Cannot resolve merge-base of {arguments.base} and {arguments.head}.")
        return 2
    base = merge_base.stdout.strip()
    changed = _git(
        root, "-c", "core.quotePath=false", "diff", "--name-only", "-z", "--diff-filter=M",
        base, arguments.head, "--", "*.md", "*.markdown",
    )
    if changed.returncode != 0:
        print("git diff failed.")
        return 2

    checked = 0
    total_lost = 0
    for relative in sorted(set(changed.stdout.split("\0")) - {""}):
        checked += 1
        old = _show(root, base, relative)
        new = _show(root, arguments.head, relative)
        if old is None or new is None:
            print(f"Cannot read {relative} at {arguments.base} or {arguments.head}.")
            return 2
        lost, added = compare(old, new)
        for token in lost:
            print(f"LOST {relative}: {token}")
        for token in added:
            print(f"ADDED {relative}: {token}")
        print(f"{relative}: lost={len(lost)} added={len(added)}")
        total_lost += len(lost)
    print(f"checked={checked} lost={total_lost}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
