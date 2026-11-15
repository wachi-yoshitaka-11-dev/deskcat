#!/usr/bin/env python3
"""`PROTO-02`（電源の基板）の実装の計画図と、その表を1つのデータから作る。

**これは実装の計画であり、未実装である。**はんだ付けの前に描き、#405 のはんだ付けはこの図に
従う。付けた後に現物と照合し、照合した版を完成図（`layout.svg`）にする。この計画図を完成図として
扱わない（`hardware/README.md`）。

計画の正本はこのファイルのデータである。`plan.md`（表）と `plan.svg`（図）はここから生成し、
手で直さない。`hardware/bench-18/generate.py` と同じ作りである。

電源経路の向きと直列順序は `docs/hardware/power-budget.md` の決定を写す（ここで決めない）。
- 端子台 → `PROT-OC-01`（PTC、`MF-R135`）→ `PROT-RP-01`（`2SJ334`、Drain を PTC 側、Source を分岐点側）
  → 分岐点（`実装方法を決定した（2026-09-03）`の`向き（2026-09-04訂正）`）。
- Gate は 10 kΩ を直列に入れて共通 GND 戻りへ、最短でつなぐ（同節）。
- 分岐の節点は `PROTO-02` の上のはんだ接続で作り、各枝を別々の端子台 `114217` で終端する。
  節点の導体は `111641` の線材が決める（`HW-TBD-022`を閉じるために残っていること`の2）。

部品の寸法は各品の一次資料から取った（`plan.md` の`寸法の出所`）。資料に無い寸法は描いた形が概略で、
`plan.md` の`測る所`に挙げる。

生成の前に、次を機械で確かめる（崩れると生成せずに止まる）。
- すべての足と導体の穴が、基板に実在する穴である（四隅の欠けを含む）。
- 1つの穴に入る物は1つだけである（足どうし、足と別の節点の導体が穴を共有しない）。
- はんだ面の導体は、節点が違うものと同じ穴を通らない（交差しない）。
- 部品の外形（部品面）が基板の外に出ず、互いに重ならない。
- 足と導体から組み立てた節点が、power-budget.md の直列順序の節点と一致する。

使い方: `python3 -I hardware/universal-board/generate.py`（標準ライブラリだけ。ADR-0006）。
`--check` を付けると、検査し、生成した内容が file と一致するかを確かめる。
"""

import argparse
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# ---------------------------------------------------------------- 基板（秋月 100517 の寸法図 p517-dimm.pdf）

BOARD_W, BOARD_H = 72.0, 47.5  # mm。hardware-bom.md の PROTO-02 行
PITCH = 2.54
COLS, ROWS = 27, 17  # 穴の列（長辺方向）と行（短辺方向）。寸法図の穴を数えた（447穴）
# 穴の中心の、基板の縁からの位置。寸法図に数値の記載は無く、図の座標から読んだ値（概略）
X0, Y0 = 3.0, 3.45
MOUNT = [(3.0, 3.0), (69.0, 3.0), (3.0, 44.5), (69.0, 44.5)]  # 取付穴 φ3.2 の中心。寸法図の記載


def hole_exists(x, y):
    """寸法図の四隅の欠け。端の列は13穴（行3〜15）、その内側の列は15穴（行2〜16）。"""
    if not (1 <= x <= COLS and 1 <= y <= ROWS):
        return False
    if x in (1, COLS):
        return 3 <= y <= 15
    if x in (2, COLS - 1):
        return 2 <= y <= 16
    return True


def hole_mm(x, y):
    return X0 + (x - 1) * PITCH, Y0 + (y - 1) * PITCH


# ---------------------------------------------------------------- 部品（部品面から見た穴の番号。x は左＝後ろ、y は上＝ふたの側）

# (ID, BOM の ID と品, 足 {名前: (x, y)}, 外形 (中心x穴, 中心y穴, 幅mm, 高さmm), 向きの指示, 外形の出所)
PARTS = [
    ("J1", "`WIRE-PWR-01` の端子台 `114217`（入力。M-12001 の線を受ける）",
     {"＋": (2, 7), "−": (2, 8)}, (2, 7.5, 6.2, 5.54),
     "電線の口を基板の左の縁（後ろ）へ向ける", "WJEK254 の図面"),
    ("F1", "`PROT-OC-01`（`MF-R135`、PTC）",
     {"a": (5, 7), "b": (7, 7)}, (6, 7, 8.9, 3.0),
     "立てて挿す。極性の指定は無い（power-budget.md の`逆接可否表`）", "MF-R Series datasheet の Product Dimensions"),
    ("Q1", "`PROT-RP-01`（`2SJ334`）",
     {"S": (10, 6), "D": (10, 7), "G": (10, 8)}, (10, 7, 4.5, 10.0),
     "立てて挿し、刻印面を右（前、J2 の側）へ向ける。刻印面から見て左から 1 G・2 D・3 S（datasheet）なので、右から見ると左が下になり、上から S・D・G になる",
     "2SJ334 datasheet の外形図"),
    ("R1", "Gate 抵抗 10 kΩ（`RES-PULL-01`、秋月 125103）",
     {"1": (12, 8), "2": (12, 9)}, (12, 8, 2.5, 2.5),
     "立てて挿す（足の間隔 1 穴）。本体を上（12,8）の側に置く", "資料に寸法が無い（概略）"),
    ("J2", "`WIRE-PWR-01` の端子台 `114217`（servo の枝）",
     {"＋": (16, 7), "−": (16, 8)}, (16, 7.5, 6.2, 5.54),
     "電線の口を右（前）へ向ける", "WJEK254 の図面"),
    ("J3", "`WIRE-PWR-01` の端子台 `114217`（Pi の枝。`CABLE-PI-PWR-01` を受ける）",
     {"＋": (21, 7), "−": (21, 8)}, (21, 7.5, 6.2, 5.54),
     "電線の口を右（前）へ向ける", "WJEK254 の図面"),
    ("J4", "`WIRE-PWR-01` の端子台 `114217`（logic の枝。**付けるかは #405 で決める**）",
     {"＋": (26, 7), "−": (26, 8)}, (26, 7.5, 6.2, 5.54),
     "電線の口を右（前）へ向ける", "WJEK254 の図面"),
]

# Q1 の刻印面の向き（部品面の図の上で、刻印面が向く方向）。datasheet は「刻印面から見て左から
# 1 G・2 D・3 S」。部品は基板から立ち、見る人の頭は基板の外を向く。図は上が行1・右が列27なので、
# 刻印面が右（+x）なら見る人は左を向き、その人の左は図の下（+y）になる。
Q1_MARKING = "+x"
LEFT_OF_VIEWER = {"+x": (0, 1), "-x": (0, -1), "+y": (-1, 0), "-y": (1, 0)}

# 決めていない物の場所（穴は割り当てない）。(ID, 何, 範囲 (x1, y1, x2, y2) 穴)
RESERVED = [
    ("C1", "servo の枝の bulk capacitor（値と品は #405。足の穴は品が決まってから）", (13.0, 6.5, 14.4, 8.5)),
]

# はんだ面の導体（`111641` の芯線を穴の上に沿わせ、通る穴ごとにはんだ付けする）。
# (ID, 節点, 通る穴の並び（直線の折れ点）, 何)
CONDUCTORS = [
    ("W1", "VIN", [(2, 7), (5, 7)], "J1＋ → F1a"),
    ("W2", "VPTC", [(7, 7), (10, 7)], "F1b → Q1 D"),
    ("W3", "N+", [(10, 6), (26, 6)], "Q1 S → 分岐点（行6）"),
    ("W3a", "N+", [(16, 6), (16, 7)], "分岐点 → J2＋"),
    ("W3b", "N+", [(21, 6), (21, 7)], "分岐点 → J3＋"),
    ("W3c", "N+", [(26, 6), (26, 7)], "分岐点 → J4＋"),
    ("W4", "GND", [(2, 8), (2, 9), (26, 9)], "J1− → GND の芯線（行9）"),
    ("W4a", "GND", [(16, 8), (16, 9)], "J2− → GND"),
    ("W4b", "GND", [(21, 8), (21, 9)], "J3− → GND"),
    ("W4c", "GND", [(26, 8), (26, 9)], "J4− → GND"),
    ("W5", "GATE", [(10, 8), (12, 8)], "Q1 G → R1 の足1（電流は流れない）"),
]

# 足が属する節点。部品は節点をまたぐ（F1・Q1 の D–S・R1）。
PIN_NET = {
    ("J1", "＋"): "VIN", ("J1", "−"): "GND",
    ("F1", "a"): "VIN", ("F1", "b"): "VPTC",
    ("Q1", "D"): "VPTC", ("Q1", "S"): "N+", ("Q1", "G"): "GATE",
    ("R1", "1"): "GATE", ("R1", "2"): "GND",
    ("J2", "＋"): "N+", ("J2", "−"): "GND",
    ("J3", "＋"): "N+", ("J3", "−"): "GND",
    ("J4", "＋"): "N+", ("J4", "−"): "GND",
}

# power-budget.md の直列順序から期待する節点（何がその節点にあるか）
EXPECTED_NETS = {
    "VIN": {"J1＋", "F1a"},
    "VPTC": {"F1b", "Q1D"},
    "N+": {"Q1S", "J2＋", "J3＋", "J4＋"},
    "GATE": {"Q1G", "R11"},
    "GND": {"J1−", "R12", "J2−", "J3−", "J4−"},
}

NET_LABEL = {
    "VIN": "入力の＋（PTC の手前）",
    "VPTC": "PTC の後（MOSFET の Drain）",
    "N+": "分岐点（MOSFET の Source の後の 5 V）",
    "GATE": "MOSFET の Gate（10 kΩ を通して GND へ）",
    "GND": "共通 GND 戻り",
}
NET_COLOR = {"VIN": "#d33", "VPTC": "#e80", "N+": "#c0c", "GATE": "#2a2", "GND": "#36c"}

# 基板から出る線（外部接続点）
EXTERNAL = [
    ("J1", "M-12001 の線（Micro-B の手前で切った導体。`EXP-012`）", "左の壁の後ろ寄りの穴から入る（enclosure-and-neck.md）",
     "＋と−の取り違えの確かめ方は power-budget.md の`通電前の配線確認手順`に従う"),
    ("J2", "servo の枝の線（`111641`）", "ふたの切り込みから servo へ（enclosure-and-neck.md の`線の通り道`）",
     "servo の plug とのつなぎ方は #405 で決める（`TBD`）。shunt（`MEAS-01`）を J2− と GND の間に入れるかは #17・#405"),
    ("J3", "`CABLE-PI-PWR-01`（`109314` を切った導体。被覆印字 `22AWG/2C`）", "右の壁の後ろ寄りの Pi の `PWR IN` へ",
     "＋と−の取り違えは Pi へ逆電圧を掛ける（power-budget.md の`逆接可否表`の rail → Pi の行）"),
    ("J4", "logic の枝（ESP32 の 5 V 入力・LCD の VCC の候補）", "B1（床の前寄り）へ",
     "ESP32 の給電経路と LCD の VCC の電圧は未決定（power-budget.md の`ESP32の給電経路`、#405）。決まるまで付けないでよい"),
]

# 寸法の出所（一次資料）
SOURCES = [
    ("`PROTO-02`（秋月 100517）", "外形 72 × 47.5、ランドピッチ 2.54、穴径 1、取付穴径 3.2、取付穴間隔 66 × 41.5、厚み 1.6",
     "hardware-bom.md の PROTO-02 行、秋月の商品ページ、寸法図 `p517-dimm.pdf`（sha256 `3421ec892958f9903e533233f1c583a97e137fdf059a6fe1126f5a344f0e0d12`）。穴の数（27 × 17、四隅が欠けて447穴）は寸法図の穴を数えた"),
    ("端子台 `114217`（Cixi Wanjie `WJEK254-2.54-02P-140-00A`）", "ピッチ 2.54、幅 2.54 + 3.00 = 5.54、奥行 6.20（足の列は中央、3.10）、高さ 8.50、足 0.80 角、足の長さ 3.50 ± 0.3、推奨穴径 1.00",
     "メーカーの図面 `WJEK254-2.54-XXP-1YY-00A.pdf`（sha256 `9355c49497ebb449d8fc3633752b87a2a2d8df5798b12cb58bcfc32f8016d748`）と秋月の商品ページ"),
    ("`MF-R135`（Bourns）", "幅 A 最大 8.9、高さ B 最大 18.9、足の間隔 C 5.1 ± 0.7、足の長さ D 最小 7.6、厚み E 最大 3.0、Style 1、足の径 0.51",
     "MF-R Series datasheet `mfr.pdf`（sha256 `7d980de97abdadc53465454d48a39c6926f1287a2fcfca77d0520a7512245fe4`）の Product Dimensions"),
    ("`2SJ334`（東芝）", "足の間隔 2.54 ± 0.25、足の幅 0.75 ± 0.15・厚み 0.75 ± 0.15（付け根 1.1、付け根の長さ最大 5.6）、足の長さ 13.0 以上、本体 10 ± 0.3 × 15 ± 0.3、厚み 4.5 ± 0.2、pin 1 G・2 D・3 S（刻印面から見て左から）",
     "datasheet `2sj334.pdf`（2006-11-14、sha256 `c01da93643cb1cb11075d9137897655c556989de647c38368d5c487274d5da46`。power-budget.md に記録済みの sha256 と一致）"),
    ("10 kΩ（秋月 125103）", "炭素皮膜 1/4 W。**本体の寸法は商品ページに無い**", "秋月の商品ページ"),
]

# 資料に無く、測る所（測るのは人間。現物は改変しない）
MEASURE = [
    ("M1", "`2SJ334` の足が `PROTO-02` の穴（φ1.0）に入るか",
     "datasheet の足の断面は 0.75 ± 0.15 角で、角の対角が公称で約 1.06 になり、穴径 1.0 を超えうる。挿して止まらずに入るかを、力を掛けずに当てて見る（曲げない・削らない）"),
    ("M2", "`2SJ334` を挿したときの、基板の面から本体の上端までの高さ",
     "付け根の幅 1.1 の部分の長さは datasheet に最大 5.6 しか無く、本体が止まる高さが決まらない（上端は最大で約 20.9）。胴の中で左の壁から内側へ出る高さ（実機側の PM が照らす）に効く"),
    ("M3", "10 kΩ（125103）の本体の長さと直径",
     "商品ページに寸法が無い。立てて挿す（足の間隔 1 穴）ので、直径が隣の穴（Q1 の G との間）に掛からないかを見る"),
    ("M4", "端子台 `114217` の電線の口の、基板の面からの高さ",
     "図面の SECTION A-A に口の高さの寸法が無い。基板を壁の溝にはめるとき、口が溝の中に入らないか（実機側の PM が照らす）に効く"),
    ("M5", "`PROTO-02` の、端の穴の中心から基板の縁までの距離（長辺・短辺）",
     "寸法図は取付穴の位置だけを数値で示し、穴の格子の位置は数値で示さない。図の座標から読んだ値（長辺側 約 3.0、短辺側 約 3.45）を使った。壁の溝の寸法に効く"),
]

NOT_DRAWN = [
    "`PSU-INGRESS-01`（Micro-B の変換基板）：2026-09-09 の案B で入力の経路から外れた（hardware-bom.md の `Not used`）。入力は J1 の端子台で受ける。",
    "ヒューズホルダ `110521`：2026-08-26 に方式を PTC へ変えたため使わない（`PROT-OC-01` は「ガラス管ヒューズ＋ホルダは採らない」）。PTC（F1）は基板へ直接はんだ付けする。",
    "shunt（`MEAS-01`）と `ADC-SHUNT`（GPIO32）の線：入れるかは #17・#405 で決める。入れる場合は W4a（(16,8)→(16,9)）の代わりに入れる（power-budget.md の`GND topology`の、servo の戻りだけが通る位置）。",
    "C1（servo の枝の bulk capacitor）：値と品が決まっていない（enclosure-and-neck.md の`決めていないこと`）。場所だけを空けた。",
]


# ---------------------------------------------------------------- 検査

def cells(path):
    """折れ点の並びから、通る穴をすべて列挙する（縦・横の直線だけ）。"""
    out = []
    for (x1, y1), (x2, y2) in zip(path, path[1:]):
        if x1 != x2 and y1 != y2:
            raise ValueError(f"斜めの導体: {path}")
        dx = (x2 > x1) - (x2 < x1)
        dy = (y2 > y1) - (y2 < y1)
        x, y = x1, y1
        while True:
            if (x, y) not in out:
                out.append((x, y))
            if (x, y) == (x2, y2):
                break
            x, y = x + dx, y + dy
    return out


def run_checks():
    errors = []
    pin_at = {}
    for pid, _, pins, *_ in PARTS:
        for name, xy in pins.items():
            if not hole_exists(*xy):
                errors.append(f"{pid}.{name}: 穴 {xy} は基板に無い")
            if xy in pin_at:
                errors.append(f"{pid}.{name}: 穴 {xy} を {pin_at[xy]} と共有")
            pin_at[xy] = f"{pid}.{name}"
            if (pid, name) not in PIN_NET:
                errors.append(f"{pid}.{name}: 節点が決まっていない")
    cond_net = {}
    for cid, net, path, _ in CONDUCTORS:
        for xy in cells(path):
            if not hole_exists(*xy):
                errors.append(f"{cid}: 穴 {xy} は基板に無い")
            if xy in cond_net and cond_net[xy] != net:
                errors.append(f"{cid}: 穴 {xy} で節点 {cond_net[xy]} と {net} が交わる")
            cond_net[xy] = net
            if xy in pin_at:
                p = pin_at[xy]
                pid, name = p.split(".")
                if PIN_NET[(pid, name)] != net:
                    errors.append(f"{cid}: 穴 {xy} の {p} は節点 {PIN_NET[(pid, name)]} で、導体は {net}")
    # 足から組み立てた節点
    got = {}
    for (pid, name), net in PIN_NET.items():
        got.setdefault(net, set()).add(f"{pid}{name}")
    for net, members in got.items():
        if EXPECTED_NETS.get(net) != members:
            errors.append(f"節点 {net}: {sorted(members)} が期待 {sorted(EXPECTED_NETS.get(net, []))} と違う")
    # 各足は、その節点の導体に触れている（はんだ面でつながっている）
    for (pid, name), net in PIN_NET.items():
        xy = dict((p[0], p[2]) for p in PARTS)[pid][name]
        if cond_net.get(xy) != net:
            errors.append(f"{pid}.{name}: 穴 {xy} に節点 {net} の導体が来ていない")
    # Q1 の足の並び：刻印面から見て左から G・D・S
    q1 = dict((p[0], p[2]) for p in PARTS)["Q1"]
    lx, ly = LEFT_OF_VIEWER[Q1_MARKING]
    order = sorted(q1, key=lambda n: -(q1[n][0] * lx + q1[n][1] * ly))
    if order != ["G", "D", "S"]:
        errors.append(f"Q1: 刻印面 {Q1_MARKING} から見た左からの並びが {order}（datasheet は G・D・S）")
    phrase = {"+x": "刻印面を右", "-x": "刻印面を左", "+y": "刻印面を下", "-y": "刻印面を上"}
    q1_text = [p[4] for p in PARTS if p[0] == "Q1"][0]
    if phrase[Q1_MARKING] not in q1_text or any(v in q1_text for k, v in phrase.items() if k != Q1_MARKING):
        errors.append("Q1: 向きの指示の文と Q1_MARKING が合わない")
    # 外形
    rects = []
    for pid, _, _, (cx, cy, w, h), *_ in PARTS:
        mx, my = hole_mm(cx, cy)
        r = (mx - w / 2, my - h / 2, mx + w / 2, my + h / 2)
        if r[0] < 0 or r[1] < 0 or r[2] > BOARD_W or r[3] > BOARD_H:
            errors.append(f"{pid}: 外形が基板の外に出る {tuple(round(v, 2) for v in r)}")
        rects.append((pid, r))
    for i, (a, ra) in enumerate(rects):
        for b, rb in rects[i + 1:]:
            if ra[0] < rb[2] and rb[0] < ra[2] and ra[1] < rb[3] and rb[1] < ra[3]:
                errors.append(f"{a} と {b} の外形が重なる")
    for rid, _, (x1, y1, x2, y2) in RESERVED:
        ax, ay = hole_mm(x1, y1)
        bx, by = hole_mm(x2, y2)
        for pid, r in rects:
            if ax < r[2] and r[0] < bx and ay < r[3] and r[1] < by:
                errors.append(f"{rid}（予約）と {pid} の外形が重なる")
    for pid, r in rects:
        for mx, my in MOUNT:
            if r[0] - 1.6 < mx < r[2] + 1.6 and r[1] - 1.6 < my < r[3] + 1.6:
                errors.append(f"{pid}: 取付穴 ({mx}, {my}) に掛かる")
    return errors


# ---------------------------------------------------------------- 生成（表）

def render_md():
    L = ["<!-- generate.py が生成する。手で直さない。 -->",
         "# `PROTO-02` の実装の計画（未実装）",
         "",
         "**これは実装の計画であり、未実装である。完成図ではない。**#405 のはんだ付けはこの計画に従い、付けた後に現物と照合して完成図（`layout.svg`）にする（[hardware/README.md](../README.md)）。この表は通電の許可ではない。通電の条件は [power-budget.md](../../docs/hardware/power-budget.md) が持つ。",
         "",
         "図は [`plan.svg`](plan.svg)。上が部品面、下がはんだ面（基板を左右に裏返した向き）である。穴は（列, 行）で呼ぶ。列は部品面から見て左（胴の後ろ）から 1〜27、行は上（ふたの側）から 1〜17 である。",
         "",
         "## 基板の向き",
         "",
         "- 胴の左の壁の内側に立てる（[enclosure-and-neck.md](../../docs/hardware/enclosure-and-neck.md)の`物理的な完成形`）。長辺（72）を前後に、短辺（47.5）を上下にする。",
         "- 部品面を胴の内側へ、はんだ面を壁の側へ向ける。部品面から見て左が胴の後ろ、右が前になる。",
         "- 基板は四隅とも同じ形なので、どちらを上にしても穴の並びは同じである。上にした辺を、はんだ付けの前に油性ペンで印す（部品面の上の縁）。",
         "",
         "## 部品と向き",
         "",
         "| ID | 品 | 足（穴） | 向き | 外形の出所 |",
         "|---|---|---|---|---|"]
    for pid, what, pins, _, orient, src in PARTS:
        p = "、".join(f"{n} ({x},{y})" for n, (x, y) in pins.items())
        L.append(f"| {pid} | {what} | {p} | {orient} | {src} |")
    for rid, what, (x1, y1, x2, y2) in RESERVED:
        L.append(f"| {rid} | {what} | 場所だけ空ける（列 {x1:g}〜{x2:g}、行 {y1:g}〜{y2:g}） | — | — |")
    L += ["",
          "## はんだ面の導体",
          "",
          "大電流の経路（W1〜W4）は `111641` の芯線（被覆を剥いた区間）を穴の上に沿わせ、通る穴ごとにはんだ付けする。**はんだだけで穴と穴を橋渡ししない。**分岐の節点の導体は `111641` が決める（power-budget.md の`HW-TBD-022を閉じるために残っていること`の2）。W5 は電流が流れない Gate の線で、同じ芯線でよい。",
          "",
          "| ID | 節点 | 通る穴 | 何 |",
          "|---|---|---|---|"]
    for cid, net, path, what in CONDUCTORS:
        c = cells(path)
        L.append(f"| {cid} | {net} | " + " → ".join(f"({x},{y})" for x, y in [path[0]] + path[1:]) + f"（{len(c)}穴） | {what} |")
    L += ["",
          "## 節点（電気的に同じ点）",
          "",
          "| 節点 | 何 | その点にある足 |",
          "|---|---|---|"]
    got = {}
    for (pid, name), net in PIN_NET.items():
        got.setdefault(net, []).append(f"{pid} {name}")
    for net in EXPECTED_NETS:
        L.append(f"| {net} | {NET_LABEL[net]} | {'、'.join(got[net])} |")
    L += ["",
          "電源経路は J1＋ → W1 → F1（PTC）→ W2 → Q1 の D → Q1 の S → W3（分岐点）→ J2・J3・J4 の＋である。power-budget.md の`実装方法を決定した（2026-09-03）`の直列順序（端子台 → `PROT-OC-01` → `PROT-RP-01`（Drain を PTC 側、Source を分岐点側）→ 分岐点）をそのまま写した。Gate は R1（10 kΩ）を直列に通して GND へつなぐ（同節）。",
          "",
          "分岐の順は、入力に近い方から servo（J2）、Pi（J3）、logic（J4）にした。servo の戻りの電流が、W4 の上で Pi と logic の取り出し口（列 21・26）の区間を通らないためである（power-budget.md の`GND topology`の、servo の電流で logic の GND の基準を動かさない、という条件）。**star point は（16,9）の節点とする。**servo の電流は W4 の（2,9）〜（16,9）だけを流れ、Pi と logic の取り出し口（21,9）（26,9）との間を流れない。shunt（`MEAS-01`）を入れる場合は、W4a（(16,8)→(16,9)）の代わりに入れる。logic GND と servo GND の間に shunt が入っていないことは、#405 の受け入れ条件の電源 off の導通 check で確かめる。",
          "",
          "## 基板から出る線",
          "",
          "| 端子台 | 線 | 行き先 | 注意 |",
          "|---|---|---|---|"]
    for j, w, to, note in EXTERNAL:
        L.append(f"| {j} | {w} | {to} | {note} |")
    L += ["",
          "## 描かないもの",
          ""]
    L += [f"- {t}" for t in NOT_DRAWN]
    L += ["",
          "## 寸法の出所",
          "",
          "| 品 | 使った寸法（mm） | 出所 |",
          "|---|---|---|"]
    for a, b, c in SOURCES:
        L.append(f"| {a} | {b} | {c} |")
    L += ["",
          "## 測る所（資料に寸法が無い）",
          "",
          "測るのは人間である。現物は改変しない（[Hardware Safety Policy](../../docs/governance/hardware-safety-policy.md)の`測定は現物を改変しない`）。結果で穴の割り当てが変わる場合は、このファイルのデータを直して生成し直す。",
          "",
          "| ID | 測る所 | 理由 |",
          "|---|---|---|"]
    for a, b, c in MEASURE:
        L.append(f"| {a} | {b} | {c} |")
    L.append("")
    return "\n".join(L)


# ---------------------------------------------------------------- 生成（図）

S = 9.0  # px / mm
MARGIN = 40
PANEL_H = BOARD_H * S


def esc(t):
    return t.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def render_svg():
    W = int(BOARD_W * S + 2 * MARGIN)
    H = int(2 * PANEL_H + 4 * MARGIN + 60)
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="sans-serif">',
         f'<rect width="{W}" height="{H}" fill="#fff"/>',
         f'<text x="{MARGIN}" y="24" font-size="16" font-weight="bold" fill="#b00">PROTO-02 実装の計画（未実装）— 完成図ではない。#405 で付けた後に現物と照合する</text>']

    def panel(top, mirror, title):
        def P(xmm, ymm):
            if mirror:
                xmm = BOARD_W - xmm
            return MARGIN + xmm * S, top + ymm * S
        o.append(f'<text x="{MARGIN}" y="{top - 8}" font-size="13">{esc(title)}</text>')
        x0, y0 = P(0, 0)
        x1, y1 = P(BOARD_W, BOARD_H)
        o.append(f'<rect x="{min(x0, x1)}" y="{y0}" width="{abs(x1 - x0)}" height="{y1 - y0}" fill="#efe6c8" stroke="#555"/>')
        for mx, my in MOUNT:
            cx, cy = P(mx, my)
            o.append(f'<circle cx="{cx}" cy="{cy}" r="{1.6 * S}" fill="#fff" stroke="#555"/>')
        for x in range(1, COLS + 1):
            for y in range(1, ROWS + 1):
                if hole_exists(x, y):
                    cx, cy = P(*hole_mm(x, y))
                    o.append(f'<circle cx="{cx}" cy="{cy}" r="{0.9 * S}" fill="#d9b55e" stroke="#a88"/><circle cx="{cx}" cy="{cy}" r="{0.5 * S}" fill="#fff"/>')
        for x in (1, 5, 10, 15, 20, 25, 27):
            cx, cy = P(*hole_mm(x, 1))
            o.append(f'<text x="{cx}" y="{top - 22 if False else cy - 14}" font-size="9" text-anchor="middle" fill="#666">{x}</text>')
        for y in (1, 5, 10, 15, 17):
            cx, cy = P(*hole_mm(1, y))
            o.append(f'<text x="{cx - 14 if not mirror else cx + 14}" y="{cy + 3}" font-size="9" text-anchor="middle" fill="#666">{y}</text>')
        return P

    # 部品面
    top1 = MARGIN + 30
    P = panel(top1, False, "部品面（胴の内側から見る。左＝胴の後ろ、右＝前、上＝ふたの側）")
    for rid, what, (x1, y1, x2, y2) in RESERVED:
        ax, ay = P(*hole_mm(x1, y1))
        bx, by = P(*hole_mm(x2, y2))
        o.append(f'<rect x="{ax}" y="{ay}" width="{bx - ax}" height="{by - ay}" fill="none" stroke="#888" stroke-dasharray="4 3"/>')
        o.append(f'<text x="{(ax + bx) / 2}" y="{ay - 3}" font-size="10" text-anchor="middle" fill="#666">{rid} 予約</text>')
    for pid, _, pins, (cx, cy, w, h), *_ in PARTS:
        mx, my = hole_mm(cx, cy)
        ax, ay = P(mx - w / 2, my - h / 2)
        dash = ' stroke-dasharray="5 3"' if pid == "J4" else ""
        o.append(f'<rect x="{ax}" y="{ay}" width="{w * S}" height="{h * S}" fill="#fff8" stroke="#000" stroke-width="1.5"{dash}/>')
        o.append(f'<text x="{ax + w * S / 2}" y="{ay - 4}" font-size="11" font-weight="bold" text-anchor="middle">{pid}</text>')
        for name, (x, y) in pins.items():
            px, py = P(*hole_mm(x, y))
            o.append(f'<circle cx="{px}" cy="{py}" r="{0.6 * S}" fill="{NET_COLOR[PIN_NET[(pid, name)]]}"/>')
            o.append(f'<text x="{px + 7}" y="{py + 4}" font-size="9">{esc(name)}</text>')
    # 端子台の電線の口と、Q1 の刻印面
    for pid, side in (("J1", -1), ("J2", 1), ("J3", 1), ("J4", 1)):
        _, _, _, (cx, cy, w, h), *_ = [p for p in PARTS if p[0] == pid][0]
        mx, my = hole_mm(cx, cy)
        ex = mx + side * w / 2
        sx, sy = P(ex, my)
        tx, _ = P(ex + side * 2.5, my)
        o.append(f'<line x1="{sx}" y1="{sy}" x2="{tx}" y2="{sy}" stroke="#000" stroke-width="2" marker-end="url(#a)"/>')
    _, _, _, (cx, cy, w, h), *_ = [p for p in PARTS if p[0] == "Q1"][0]
    mx, my = hole_mm(cx, cy)
    ax, ay = P(mx - w / 2, my - h / 2)
    o.append(f'<line x1="{ax + w * S}" y1="{ay}" x2="{ax + w * S}" y2="{ay + h * S}" stroke="#b00" stroke-width="3"/>')
    o.append(f'<text x="{ax + w * S + 3}" y="{ay + h * S + 12}" font-size="9" fill="#b00">刻印面</text>')
    o.insert(1, '<defs><marker id="a" markerWidth="8" markerHeight="8" refX="6" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 z"/></marker></defs>')

    # はんだ面
    top2 = top1 + PANEL_H + 2 * MARGIN
    P = panel(top2, True, "はんだ面（基板を左右に裏返した向き。左＝胴の前、右＝後ろ）。太線＝111641 の芯線")
    for cid, net, path, _ in CONDUCTORS:
        pts = " ".join(f"{P(*hole_mm(x, y))[0]},{P(*hole_mm(x, y))[1]}" for x, y in path)
        o.append(f'<polyline points="{pts}" fill="none" stroke="{NET_COLOR[net]}" stroke-width="{0.9 * S}" stroke-linecap="round" stroke-linejoin="round" opacity="0.85"/>')
    for pid, _, pins, *_ in PARTS:
        for name, (x, y) in pins.items():
            px, py = P(*hole_mm(x, y))
            o.append(f'<circle cx="{px}" cy="{py}" r="{0.35 * S}" fill="#000"/>')
            o.append(f'<text x="{px + 6}" y="{py - 5}" font-size="8">{pid}{esc(name)}</text>')

    # 凡例
    ly = top2 + PANEL_H + 26
    x = MARGIN
    for net in EXPECTED_NETS:
        o.append(f'<rect x="{x}" y="{ly - 9}" width="14" height="10" fill="{NET_COLOR[net]}"/>')
        o.append(f'<text x="{x + 18}" y="{ly}" font-size="10">{net}: {esc(NET_LABEL[net])}</text>')
        x += 18 + 11 * len(NET_LABEL[net]) + 40
        if x > W - 200:
            x = MARGIN
            ly += 16
    o.append(f'<text x="{MARGIN}" y="{ly + 18}" font-size="10" fill="#444">表・寸法の出所・測る所は plan.md。J4 は付けるかを #405 で決める（破線）。C1 は値と品が未決で場所だけ空ける。</text>')
    o.append("</svg>")
    return "\n".join(o) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="検査し、生成物が file と一致するか確かめる")
    a = ap.parse_args()
    errors = run_checks()
    if errors:
        for e in errors:
            print("NG:", e, file=sys.stderr)
        return 1
    outputs = {HERE / "plan.md": render_md(), HERE / "plan.svg": render_svg()}
    if a.check:
        bad = [p.name for p, t in outputs.items() if not p.exists() or p.read_text(encoding="utf-8") != t]
        if bad:
            print("生成物が file と一致しない:", ", ".join(bad), file=sys.stderr)
            return 1
        print("OK")
        return 0
    for p, t in outputs.items():
        p.write_text(t, encoding="utf-8")
    print("wrote", ", ".join(p.name for p in outputs))
    return 0


if __name__ == "__main__":
    sys.exit(main())
