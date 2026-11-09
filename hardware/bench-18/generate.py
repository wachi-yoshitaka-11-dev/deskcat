#!/usr/bin/env python3
"""#18 の窓（WDT、GND、I_lim、発熱）と段1の servo 電源の経路の、図と結線表を1つのデータから作る。

配線の正本は `wiring-table.md`（節点の表）である。`wiring-table.md` と `layout.svg` はこのファイルの
データから生成し、手で直さない。`hardware/breadboard-rebuild/generate.py` と同じ作りである。

描くもの: PSU の出力端子 → 新しい線 → 端子台 A・B・S の6穴（各穴の2本）→ MF-R135 → シャント →
jumper wire → 窓の (D)・(B) の負荷（両端メスの jumper wire 2本と MEAS-01 の予備）／段1の servo の
plug、X4・X5 の行き先。
描かないもの: MEAS-02（Arduino）の線。測定のためだけの一時配線である（hardware/README.md の規則）。

生成の前に、次を機械で確かめる（崩れると生成せずに止まる）。
- 端子台の6穴には、それぞれちょうど2本の導体が入る。
- すべての点は、ちょうど1つの節点（電気的に同じ点）に入る。
- 1つの穴に入る2本は、同じ節点にある（穴の中で2本がつながる）。
- 節点が期待した4つで、PSU の＋と−、W+ の両端などが混ざらない。

使い方: `python3 -I hardware/bench-18/generate.py`（標準ライブラリだけを使う。ADR-0006）。
`--check` を付けると、検査だけを行い、生成した内容が file と一致するかを確かめる。
"""

import argparse
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# ---------------------------------------------------------------- データ

# 端子台の6穴に入る導体（servo-safety-limits.md の段0の2「組む配線」）
HOLES = {
    "A赤": ["新しい線（PSU＋）", "MF-R135 の足1"],
    "A黒": ["新しい線（PSU−）", "黒い線 W−（A黒側）"],
    "B1": ["MF-R135 の足2", "赤い線 W+（B1側）"],
    "B2": ["シャントの足2", "茶の jumper wire の針（B2側）"],
    "S＋": ["赤い線 W+（S＋側）", "赤の jumper wire の針（S＋側）"],
    "S−": ["黒い線 W−（S−側）", "シャントの足1"],
}

# 節点（電気的に同じ点）。(ID, 信号・極性, その点にある物)
NETS = [
    ("N1", "PSU の＋（MF-R135 の手前）", [
        "PSU の＋端子", "新しい線（PSU＋）", "穴:A赤", "MF-R135 の足1"]),
    ("N2", "MF-R135 の後。W+ と赤の jumper wire（servo の＋側）", [
        "MF-R135 の足2", "穴:B1", "赤い線 W+（B1側）", "赤い線 W+（S＋側）", "穴:S＋",
        "赤の jumper wire の針（S＋側）", "赤の jumper wire の針（負荷側）"]),
    ("N3", "servo の−側（シャントの手前）", [
        "穴:B2", "シャントの足2", "茶の jumper wire の針（B2側）",
        "茶の jumper wire の針（負荷側）"]),
    ("N4", "PSU の−と GND（シャントの後）", [
        "穴:S−", "シャントの足1", "黒い線 W−（S−側）", "黒い線 W−（A黒側）", "穴:A黒",
        "新しい線（PSU−）", "PSU の−端子"]),
]

# 線と部品: (ID, 何, 端1, 端2, 色, 備考)。端1・端2は NETS の点の名前。
LINKS = [
    ("L1", "新しい線（`WIRE-PWR-01`、AWG22相当）", "PSU の＋端子", "新しい線（PSU＋）", "色は現物で記録（TBD）", "PSU の出力端子（ねじ）へ、A赤の穴へ。＋−の照合は小節8"),
    ("L2", "新しい線（`WIRE-PWR-01`、AWG22相当）", "PSU の−端子", "新しい線（PSU−）", "色は現物で記録（TBD）", "PSU の出力端子（ねじ）へ、A黒の穴へ。同上"),
    ("L3", "`MF-R135`（PTC）", "MF-R135 の足1", "MF-R135 の足2", "—", "A赤の穴と B1 の穴の間"),
    ("L4", "赤い線 W+", "赤い線 W+（B1側）", "赤い線 W+（S＋側）", "赤", "B1 の穴と S＋の穴の間"),
    ("L5", "黒い線 W−", "黒い線 W−（S−側）", "黒い線 W−（A黒側）", "黒", "S−の穴と A黒の穴の間"),
    ("L6", "シャント（`MEAS-01`）", "シャントの足2", "シャントの足1", "—", "B2 の穴と S−の穴の間"),
    ("L7", "赤の jumper wire（両端オス）", "赤の jumper wire の針（S＋側）", "赤の jumper wire の針（負荷側）", "赤", "(D)・(B) では両端メスの jumper wire（赤側）へ、段1では servo の plug の赤へ"),
    ("L8", "茶の jumper wire（両端オス）", "茶の jumper wire の針（B2側）", "茶の jumper wire の針（負荷側）", "茶", "(D)・(B) では両端メスの jumper wire（茶側）へ、段1では servo の plug の茶へ"),
]

# 線の針のように、1つの導体が両端で同じ節点にあるもの（線の中は導通する）は、
# 端1と端2が同じ節点でなければならない。部品（MF-R135・シャント）は節点をまたぐ。
THROUGH = {"L1", "L2", "L4", "L5", "L7", "L8"}

# 窓の (D)・(B) の負荷と、段1の servo の plug（どちらも N2 と N3 の間に入る）
LOAD_STEPS = [
    ("窓の (D)・(B)", [
        "赤い jumper wire の針（負荷側、N2）",
        "両端メスの jumper wire（赤側）",
        "`MEAS-01` の予備（セメント抵抗。赤側の足 → 茶側の足）",
        "両端メスの jumper wire（茶側）",
        "茶の jumper wire の針（負荷側、N3）",
    ]),
    ("段1", [
        "赤の jumper wire の針（負荷側、N2）",
        "servo の plug の赤",
        "（servo 本体）",
        "servo の plug の茶",
        "茶の jumper wire の針（負荷側、N3）",
    ]),
]

# (B) でプローブを当てる点（`DT830B` のプローブが届く金属だけ）と、区間（点の連なり）。
# 穴の中の導体（線の導体、jumper wire の針、メス側の接点）には届かないので、足の無い穴
# （S＋、A黒）は隣の区間に含め、届く点の連なりで13区間に区切る。
PROBES = [
    ("P1", "PSU の＋の端子の金属", "N1"),
    ("P2", "A赤の端子の金属", "N1"),
    ("P3", "MF-R135 の足1", "N1"),
    ("P4", "MF-R135 の足2", "N2"),
    ("P5", "B穴1の端子の金属", "N2"),
    ("P6", "S＋の端子の金属", "N2"),
    ("P7", "抵抗（`MEAS-01` の予備）の赤側の足", "N2"),
    ("P8", "抵抗（`MEAS-01` の予備）の茶側の足", "N3"),
    ("P9", "B穴2の端子の金属", "N3"),
    ("P10", "シャントの足2", "N3"),
    ("P11", "シャントの足1", "N4"),
    ("P12", "S−の端子の金属", "N4"),
    ("P13", "A黒の端子の金属", "N4"),
    ("P14", "PSU の−の端子の金属", "N4"),
]
# (番号, 始点, 終点, 何を含むか)
INTERVALS = [
    (1, "P1", "P2", "新しい線（＋）と、その両端の接点（PSU の＋端子、A赤の穴）"),
    (2, "P2", "P3", "A赤の穴の、MF-R135 の足1の接点"),
    (3, "P3", "P4", "MF-R135"),
    (4, "P4", "P5", "B穴1の穴の、MF-R135 の足2の接点"),
    (5, "P5", "P6", "赤い線 W+ と、その両端の接点（B穴1、S＋の穴）"),
    (6, "P6", "P7", "S＋の穴の jumper 側の接点、赤の jumper wire、針とメスの接点、両端メスの jumper wire（赤側）"),
    (7, "P7", "P8", "負荷（抵抗の2本の足の間）"),
    (8, "P8", "P9", "両端メスの jumper wire（茶側）、針とメスの接点、茶の jumper wire、B穴2の穴の jumper 側の接点"),
    (9, "P9", "P10", "B穴2の穴の、シャントの足2の接点"),
    (10, "P10", "P11", "シャント"),
    (11, "P11", "P12", "S−の穴の、シャントの足1の接点"),
    (12, "P12", "P13", "黒い線 W−と、その両端の接点（S−の穴、A黒の穴）"),
    (13, "P13", "P14", "新しい線（−）と、その両端の接点（A黒の穴、PSU の−端子）"),
]

# ESP32 から出る2本（`hardware/breadboard-rebuild/wiring-table.md` の X4・X5。段0のまま）
ESP_LINES = [
    ("X4", "B1 D7", "servo の plug の黄（段1。窓の (A) では `MEAS-02` の A0 を `X4` の servo 側の端につなぐ）", "SERVO_SIG", "黄"),
    ("X5", "B1 −30", "クリップ付コード（`PROTO-01`）で、S− の穴に入ったシャントの足1（N4）を、穴と本体のあいだの金属で挟む", "GND", "黒"),
]


# ---------------------------------------------------------------- 検査

def run_checks():
    errors = []
    for h, items in HOLES.items():
        if len(items) != 2:
            errors.append(f"{h}: 導体が2本でない")
    point_net = {}
    for nid, _, points in NETS:
        for p in points:
            if p in point_net:
                errors.append(f"点が2つの節点に入っている: {p}（{point_net[p]}・{nid}）")
            point_net[p] = nid
    for h, items in HOLES.items():
        nets = {point_net.get(f"穴:{h}")} | {point_net.get(i) for i in items}
        if None in nets:
            errors.append(f"{h}: 節点に入っていない点がある")
        elif len(nets) != 1:
            errors.append(f"{h}: 穴の2本と穴が同じ節点にない（{sorted(nets)}）")
    for lid, _, a, b, _, _ in LINKS:
        for p in (a, b):
            if p not in point_net:
                errors.append(f"{lid}: 点が節点に無い: {p}")
        if a in point_net and b in point_net:
            same = point_net[a] == point_net[b]
            if lid in THROUGH and not same:
                errors.append(f"{lid}: 線の両端が別の節点にある")
            if lid not in THROUGH and same:
                errors.append(f"{lid}: 部品の両端が同じ節点にある（短絡）")
    # 穴の導体はすべて、どれかの線か部品の端である
    link_points = {p for _, _, a, b, _, _ in LINKS for p in (a, b)}
    for h, items in HOLES.items():
        for i in items:
            if i not in link_points:
                errors.append(f"{h}: {i} が線・部品の表に無い")
    ids = {p for p, _, _ in PROBES}
    net_ids = {n for n, _, _ in NETS}
    for pid, _, n in PROBES:
        if n not in net_ids:
            errors.append(f"{pid}: 節点が無い: {n}")
    pnet = {pid: n for pid, _, n in PROBES}
    for k, (num, a, b, _) in enumerate(INTERVALS):
        if num != k + 1 or a not in ids or b not in ids:
            errors.append(f"区間{num}: 番号か点が不正")
        elif a != PROBES[k][0] or b != PROBES[k + 1][0]:
            errors.append(f"区間{num}: 点の連なりでない")
    if len(INTERVALS) != len(PROBES) - 1:
        errors.append("区間の数が点の数と合わない")
    if len(NETS) != 4:
        errors.append("節点が4つでない")
    return errors


# ---------------------------------------------------------------- 結線表

def build_table():
    L = []
    L.append("<!-- generate.py が生成する。手で直さない。 -->")
    L.append("# #18 の窓と段1の servo 電源の経路の結線表")
    L.append("")
    L.append("`PSU-BENCH-01` から端子台 A・B・S、`MF-R135`、シャント、jumper wire まで。"
             "`MEAS-02`（Arduino）の線は、測定のためだけの一時配線なので描かない（[hardware/README.md](../README.md)）。"
             "この表は配線の許可ではない。通電と駆動は [servo-safety-limits.md](../../docs/hardware/servo-safety-limits.md) の条件と、その回の実行承認に従う。")
    L.append("")
    L.append("## 節点（電気的に同じ点）")
    L.append("")
    L.append("| 節点 | 信号・極性 | その点にある物 |")
    L.append("|---|---|---|")
    for nid, sig, pts in NETS:
        L.append(f"| {nid} | {sig} | " + "、".join(p.replace("穴:", "端子台の穴 ") for p in pts) + " |")
    L.append("")
    L.append("## 端子台の6穴（各穴に2本）")
    L.append("")
    L.append("| 穴 | 入る2本 | 節点 |")
    L.append("|---|---|---|")
    point_net = {p: nid for nid, _, pts in NETS for p in pts}
    for h, items in HOLES.items():
        L.append(f"| {h} | {'、'.join(items)} | {point_net['穴:' + h]} |")
    L.append("")
    L.append("## 線と部品")
    L.append("")
    L.append("| ID | 何 | 端1（節点） | 端2（節点） | 色 | 備考 |")
    L.append("|---|---|---|---|---|---|")
    for lid, what, a, b, color, note in LINKS:
        L.append(f"| {lid} | {what} | {a}（{point_net[a]}） | {b}（{point_net[b]}） | {color} | {note} |")
    L.append("")
    L.append("## 負荷（N2 と N3 の間）")
    L.append("")
    L.append("窓の (D)・(B) と段1は、同じ2本の jumper wire の負荷側の針に、別の物をつなぐ。**両端メスの jumper wire は窓の (D)・(B) の間だけ使い、段1では外して servo の plug を jumper wire の針に挿す。**"
             "メス側の接点はハウジングの中にあり、プローブが届かない。")
    L.append("")
    for name, chain in LOAD_STEPS:
        L.append(f"- **{name}：**" + " → ".join(chain))
    L.append("")
    L.append("## プローブを当てる点と区間（窓の (B)）")
    L.append("")
    L.append("`DT830B` のプローブが届く金属だけを点にする。穴の中の導体（線の導体、jumper wire の針、メス側の接点）には届かないので、"
             "足の無い穴（S＋、A黒）は隣の区間に含め、届く点の連なりで13区間に区切る。")
    L.append("")
    L.append("| 点 | 何 | 節点 |")
    L.append("|---|---|---|")
    for pid, name, n in PROBES:
        L.append(f"| {pid} | {name} | {n} |")
    L.append("")
    L.append("| 区間 | 点 | 含むもの |")
    L.append("|---:|---|---|")
    for num, a, b, what in INTERVALS:
        L.append(f"| {num} | {a} ↔ {b} | {what} |")
    L.append("")
    L.append("## ESP32 から出る線（`breadboard-rebuild/wiring-table.md` の X4・X5。段0のまま）")
    L.append("")
    L.append("| ID | 板の端 | 板の外の端 | 信号 | 色 |")
    L.append("|---|---|---|---|---|")
    for xid, end, out, sig, color in ESP_LINES:
        L.append(f"| {xid} | {end} | {out} | {sig} | {color} |")
    L.append("")
    return "\n".join(L) + "\n"


# ---------------------------------------------------------------- 図

COLORS = {"赤": "#d22", "黒": "#222", "茶": "#8a5a2b", "黄": "#d9b400", "未定": "#888"}


def build_svg():
    W, H = 1340, 650
    o = []
    a = o.append
    a(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" font-family="Noto Sans CJK JP, Hiragino Sans, Meiryo, sans-serif" font-size="13">')
    a(f'<rect width="{W}" height="{H}" fill="#fff"/>')
    a('<text x="30" y="34" font-size="20" font-weight="bold">#18 の窓と段1の servo 電源の経路</text>')
    a('<text x="30" y="58" fill="#444">生成元: hardware/bench-18/generate.py（結線表 wiring-table.md が正）。MEAS-02 の線は描かない。#405 の図とは別物。</text>')

    def text(x, y, s, size=13, anchor="start", fill="#222", bold=False):
        w = ' font-weight="bold"' if bold else ""
        a(f'<text x="{x}" y="{y}" font-size="{size}" text-anchor="{anchor}" fill="{fill}"{w}>{s}</text>')

    def line(pts, color, width=3, dash=None):
        d = f' stroke-dasharray="{dash}"' if dash else ""
        p = " ".join(f"{x},{y}" for x, y in pts)
        a(f'<polyline points="{p}" fill="none" stroke="{color}" stroke-width="{width}" stroke-linejoin="round"{d}/>')

    def hole(x, y, label, lx, ly, anchor):
        a(f'<circle cx="{x}" cy="{y}" r="11" fill="#fff" stroke="#222" stroke-width="2"/>')
        text(lx, ly, label, 13, anchor, bold=True)

    def block(x, y, name):
        a(f'<rect x="{x - 40}" y="{y - 30}" width="80" height="130" rx="6" fill="#e9eef3" stroke="#556"/>')
        text(x, y - 40, name, 14, "middle", bold=True)

    yH, yL = 250, 350
    xA, xB, xS = 330, 600, 870
    # PSU
    a('<rect x="40" y="200" width="150" height="200" rx="8" fill="#f2f2ea" stroke="#556"/>')
    text(115, 192, "PSU-BENCH-01", 14, "middle", bold=True)
    text(115, 222, "（SPS-3010V）", 12, "middle", fill="#555")
    a(f'<circle cx="190" cy="{yH}" r="9" fill="#d22"/>')
    text(176, yH + 4, "＋", 13, "end", bold=True)
    a(f'<circle cx="190" cy="{yL}" r="9" fill="#222"/>')
    text(176, yL + 4, "−", 13, "end", bold=True)
    block(xA, yH, "端子台 A")
    block(xB, yH, "端子台 B")
    block(xS, yH, "端子台 S")
    # 線を先に描き、穴を上に重ねる
    line([(190, yH), (xA, yH)], COLORS["未定"], 3, "6 4")
    line([(190, yL), (xA, yL)], COLORS["未定"], 3, "6 4")
    text(260, yH - 10, "新しい線", 11, "middle", fill="#555")
    text(260, yL - 10, "新しい線", 11, "middle", fill="#555")
    xm = (xA + xB) // 2
    line([(xA, yH), (xm - 36, yH)], "#222", 2)
    line([(xm + 36, yH), (xB, yH)], "#222", 2)
    a(f'<rect x="{xm - 36}" y="{yH - 14}" width="72" height="28" fill="#fff" stroke="#222" stroke-width="2"/>')
    text(xm, yH + 5, "MF-R135", 12, "middle", bold=True)
    line([(xB, yH), (xB, yH - 60), (xS, yH - 60), (xS, yH)], COLORS["赤"], 4)
    text((xB + xS) // 2, yH - 70, "赤い線 W+", 12, "middle", fill=COLORS["赤"], bold=True)
    xn = (xB + xS) // 2
    line([(xB, yL), (xn - 40, yL)], "#222", 2)
    line([(xn + 40, yL), (xS, yL)], "#222", 2)
    a(f'<rect x="{xn - 40}" y="{yL - 14}" width="80" height="28" fill="#fff" stroke="#222" stroke-width="2"/>')
    text(xn, yL + 5, "シャント", 12, "middle", bold=True)
    text(xn, yL - 22, "(MEAS-01) 左が足2、右が足1", 11, "middle", fill="#555")
    yW = yL + 95
    line([(xA, yL), (xA, yW), (xS, yW), (xS, yL)], COLORS["黒"], 4)
    text(xm, yW + 22, "黒い線 W−", 12, "middle", bold=True)
    # 負荷と段1
    bx, bw = 1000, 290
    line([(xS, yH), (xS + 70, yH), (xS + 70, 150), (bx, 150)], COLORS["赤"], 4)
    text(xS + 78, 268, "赤の jumper wire", 12, fill=COLORS["赤"], bold=True)
    yBr = yW + 50
    line([(xB, yL), (xB, yBr), (bx + bw + 25, yBr), (bx + bw + 25, 150), (bx + bw, 150)], COLORS["茶"], 4)
    text(xB + 14, yBr - 10, "茶の jumper wire", 12, fill=COLORS["茶"], bold=True)
    a(f'<rect x="{bx}" y="100" width="{bw}" height="100" rx="8" fill="#fbf7ea" stroke="#556" stroke-dasharray="6 4"/>')
    text(bx, 88, "窓の (D)・(B) の負荷（plug の代わりの点）", 13, bold=True)
    a(f'<rect x="{bx + 20}" y="135" width="60" height="30" fill="#e8e8e8" stroke="#555"/>')
    text(bx + 50, 128, "両端メス", 10, "middle", fill="#555")
    text(bx + 50, 154, "赤側", 11, "middle")
    a(f'<rect x="{bx + 100}" y="135" width="90" height="30" fill="#fff" stroke="#222" stroke-width="2"/>')
    text(bx + 145, 154, "MEAS-01 予備", 11, "middle", bold=True)
    a(f'<rect x="{bx + 210}" y="135" width="60" height="30" fill="#e8e8e8" stroke="#555"/>')
    text(bx + 240, 128, "両端メス", 10, "middle", fill="#555")
    text(bx + 240, 154, "茶側", 11, "middle")
    line([(bx + 80, 150), (bx + 100, 150)], "#222", 2)
    line([(bx + 190, 150), (bx + 210, 150)], "#222", 2)
    text(bx, 222, "段1ではこの負荷を外し、servo の plug を", 11, fill="#555")
    text(bx, 238, "jumper wire の針に挿す", 11, fill="#555")
    a(f'<rect x="{bx}" y="290" width="{bw}" height="110" rx="8" fill="#eef6ee" stroke="#556" stroke-dasharray="6 4"/>')
    text(bx + 10, 312, "段1: servo の plug", 13, bold=True)
    text(bx + 10, 334, "赤 → 赤の jumper wire の針", 11)
    text(bx + 10, 352, "茶 → 茶の jumper wire の針", 11)
    text(bx + 10, 370, "黄 → X4（ESP32）", 11)
    # ESP32
    a('<rect x="40" y="520" width="200" height="90" rx="8" fill="#f2f2ea" stroke="#556"/>')
    text(140, 545, "ESP32（B1）", 14, "middle", bold=True)
    text(140, 566, "X4: D7（SERVO_SIG、黄）", 11, "middle")
    text(140, 584, "X5: −30（GND、黒）", 11, "middle")
    line([(240, 566), (bx + 100, 566), (bx + 100, 400)], COLORS["黄"], 3)
    text(300, 558, "X4 → servo の plug の黄（段1）", 11, fill="#555")
    line([(240, 584), (xS - 60, 584), (xS - 60, yL + 20), (xS - 6, yL + 20)], COLORS["黒"], 3, "3 3")
    text(300, 602, "X5 → クリップで、S− の穴に入ったシャントの足1を挟む（穴の導体を増やさない）", 11, fill="#555")
    # 穴（線の上）とラベル
    hole(xA, yH, "A 赤", xA, yH - 18, "middle")
    hole(xA, yL, "A 黒", xA - 16, yL + 26, "end")
    hole(xB, yH, "B 穴1", xB - 16, yH - 16, "end")
    hole(xB, yL, "B 穴2", xB - 16, yL + 26, "end")
    hole(xS, yH, "S ＋", xS + 16, yH - 16, "start")
    hole(xS, yL, "S −", xS + 16, yL + 26, "start")
    text(30, 640, "太い実線: 線・jumper wire。破線の新しい線は色を現物で記録する（TBD）。交差している線は接続しない。", 11, fill="#444")
    a("</svg>")
    return "\n".join(o) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="検査し、生成物が file と一致するか確かめる")
    args = ap.parse_args()
    errors = run_checks()
    if errors:
        for e in errors:
            print("NG:", e, file=sys.stderr)
        return 1
    outputs = {HERE / "layout.svg": build_svg(), HERE / "wiring-table.md": build_table()}
    if args.check:
        stale = [p.name for p, t in outputs.items() if not p.exists() or p.read_text(encoding="utf-8") != t]
        if stale:
            print("生成物が古い: " + ", ".join(stale), file=sys.stderr)
            return 1
        print("OK: 検査を通り、生成物は最新")
        return 0
    for p, t in outputs.items():
        p.write_text(t, encoding="utf-8")
    print("OK: 検査を通り、生成した")
    return 0


if __name__ == "__main__":
    sys.exit(main())
