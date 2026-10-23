#!/usr/bin/env python3
"""ブレッドボードを組み直す配置（#512）の図と結線表を、1つのデータから作る。

配置の正本はこの file のデータである。`layout.svg` と `wiring-table.md` はここから生成し、
手で直さない。生成の前に次を機械で確かめる（どれかが崩れると生成せずに止まる）。

- 1つの穴に挿すものは1つだけである。ESP32 の本体とアンテナの下の穴は使わない。
- 板の上に寝かせる抵抗・LED と硬いジャンパーは、縦か横の直線である。両端の間の穴は空いている
  （ほかの足の上を通らない）。互いに交差しない。
- 論理接続が意図どおりである。穴の列（つながっている5〜6穴）と、ジャンパー・柔らかい線で
  つながる範囲ごとに、ピン・板の外の端・抵抗とLEDの端が期待する信号名が1つにそろう（混ざらない）。
  逆に、同じ信号名の端はすべて1つの導体につながる（断線しない）。

使い方: `python3 hardware/breadboard-rebuild/generate.py`（標準ライブラリだけを使う。ADR-0006）。
`--check` を付けると、検査だけを行い、生成した内容が file と一致するかを確かめる。
"""

import argparse
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# ---------------------------------------------------------------- 板

# 行の縦位置（ピッチ単位）。溝の幅と電源列までの距離は写真からの目安であり、TBD である。
B1_ROWS = {r: i for i, r in enumerate("ABCDEF")}
B1_ROWS.update({r: 8 + i for i, r in enumerate("GHIJKL")})
B1_ROWS.update({"-": 16, "+": 17})
BB601_ROWS = {r: i for i, r in enumerate("abcde")}
BB601_ROWS.update({r: 7 + i for i, r in enumerate("fghij")})

RAIL_COLS = [c for g in ((1, 6), (9, 14), (17, 22), (25, 30)) for c in range(g[0], g[1] + 1)]

BOARDS = {
    "B1": {
        "title": "主板 EIC-3901（6穴版）",
        "cols": 30,
        "rows": B1_ROWS,
        "top": "ABCDEF",
        "bottom": "GHIJKL",
        "rails": "-+",
    },
    "B2": {
        "title": "LCD 板 BB-601（1枚目）",
        "cols": 17,
        "rows": BB601_ROWS,
        "top": "abcde",
        "bottom": "fghij",
        "rails": "",
    },
    "B3": {
        "title": "センサー板 BB-601（2枚目）",
        "cols": 17,
        "rows": BB601_ROWS,
        "top": "abcde",
        "bottom": "fghij",
        "rails": "",
    },
}


def parse_hole(text):
    """`B1:L12`、`B1:-25`、`B2:e3` を (board, row, col) にする。"""
    board, rest = text.split(":")
    row, col = rest[0], int(rest[1:])
    spec = BOARDS[board]
    if row not in spec["rows"]:
        raise ValueError(f"行が無い: {text}")
    if not 1 <= col <= spec["cols"]:
        raise ValueError(f"列が無い: {text}")
    if row in "-+" and col not in RAIL_COLS:
        raise ValueError(f"電源列のその列に穴は無い: {text}")
    return board, row, col


def strip_of(hole):
    board, row, col = hole
    spec = BOARDS[board]
    if row in spec["rails"]:
        # 赤（＋）の列は14と17の間で分かれる構造で、写真では赤い部品でつないである。つないだまま使う。
        return (board, row)
    half = "T" if row in spec["top"] else "B"
    return (board, half, col)


def hole_name(hole):
    board, row, col = hole
    shown = {"-": "−", "+": "＋"}.get(row, row)
    return f"{board} {shown}{col}"


# ---------------------------------------------------------------- ESP32（DevKitC V4）

# 並びは docs/hardware/gpio-assignment.md の `Board識別情報`（公式回路図 `ESP32_DevKitc_V4`）。
J2 = "3V3 EN VP VN 34 35 32 33 25 26 27 14 12 GND 13 D2 D3 CMD 5V".split()
J3 = "GND 23 22 TX RX 21 GND 19 18 5 17 16 4 0 2 15 D1 D0 CLK".split()
ESP_FIRST_COL = 12  # J2-1・J3-1 の列。USB は30列の側（板の端）にくる
ESP_J2_ROW, ESP_J3_ROW = "J", "B"
# 本体はB〜J行、アンテナは1番ピンの先へ約6 mm（Espressifの寸法図）。9〜11列は使わない。
KEEP_OUT_COLS = (9, 10, 11)

PIN_NET = {
    "3V3": "3V3", "GND": "GND", "5V": "5V",
    "VP": "GPIO36", "34": "GPIO34", "32": "GPIO32", "33": "GPIO33", "25": "GPIO25",
    "26": "GPIO26", "27": "GPIO27", "14": "GPIO14", "13": "GPIO13", "23": "GPIO23",
    "22": "GPIO22", "21": "GPIO21", "19": "GPIO19", "18": "GPIO18", "5": "GPIO5",
    "17": "GPIO17", "16": "GPIO16", "4": "GPIO4", "2": "GPIO2",
}


def esp_pins():
    pins = []
    for header, names, row in (("J2", J2, ESP_J2_ROW), ("J3", J3, ESP_J3_ROW)):
        for n, name in enumerate(names, start=1):
            col = ESP_FIRST_COL + n - 1
            net = PIN_NET.get(name, f"使わない:{header}-{n}")
            pins.append({"hole": ("B1", row, col), "label": f"{header}-{n} {name}", "net": net})
    return pins


# ---------------------------------------------------------------- 信号名の説明

NET_NOTE = {
    "3V3": "J2-1 から", "GND": "GND", "5V": "J2-19 の 5V pin",
    "GPIO36": "ADC-3V3", "GPIO34": "TOUCH-IRQ", "GPIO33": "ADC-5V", "GPIO32": "ADC-SHUNT",
    "GPIO25": "I2C SDA", "GPIO26": "I2C SCL", "GPIO27": "SERVO-PWM", "GPIO14": "PI-UART-RX",
    "GPIO13": "PI-UART-TX", "GPIO23": "LCD-MOSI", "GPIO22": "LCD-CS", "GPIO21": "TOUCH-CS",
    "GPIO19": "LCD-MISO", "GPIO18": "LCD-SCLK", "GPIO5": "LED-REACT", "GPIO17": "LCD-DC",
    "GPIO16": "LCD-RST", "GPIO4": "LCD-BL", "GPIO2": "LED-COMM",
    "PI_TXD": "Pi pin 8 TXD 側", "PI_RXD": "Pi pin 10 RXD 側",
    "SERVO_SIG": "サーボの黄の線の側", "LED_W_A": "白LEDのアノード", "LED_R_A": "赤LEDのアノード",
}

# ---------------------------------------------------------------- 部品

# (id, 値, 品, 端1, 端1の信号, 端2, 端2の信号, 用途)
RESISTORS = [
    ("R1", "4.7 kΩ", "RES-PULL-01", "B1:L20", "GPIO25", "B1:+20", "3V3", "I2C SDA の pull-up"),
    ("R2", "4.7 kΩ", "RES-PULL-01", "B1:L21", "GPIO26", "B1:+21", "3V3", "I2C SCL の pull-up"),
    ("R3", "4.7 kΩ", "RES-PULL-01", "B1:L22", "GPIO27", "B1:-22", "GND", "SERVO-PWM の pull-down"),
    ("R4", "4.7 kΩ", "RES-PULL-01", "B1:A1", "GPIO14", "B1:A5", "PI_TXD", "UART 直列（Pi→ESP32）"),
    ("R5", "4.7 kΩ", "RES-PULL-01", "B1:C2", "GPIO13", "B1:C6", "PI_RXD", "UART 直列（ESP32→Pi）"),
    ("R6", "330 Ω", "CF25J330RB", "B1:E3", "GPIO27", "B1:E7", "SERVO_SIG", "サーボ信号の直列（EXP-028）"),
    ("R7", "330 Ω", "CF25J330RB", "B1:H5", "GPIO2", "B1:H1", "LED_W_A", "白LEDの電流制限"),
    ("R8", "330 Ω", "CF25J330RB", "B1:I2", "GPIO5", "B1:I6", "LED_R_A", "赤LEDの電流制限"),
    ("R9", "10 kΩ", "RES-PULL-01", "B1:L5", "GPIO2", "B1:-5", "GND", "LED-COMM の pull-down"),
    ("R10", "10 kΩ", "RES-PULL-01", "B1:L2", "GPIO5", "B1:-2", "GND", "LED-REACT の pull-down"),
    ("R11", "10 kΩ", "MEAS-01 分圧用", "B1:K8", "5V", "B1:K4", "GPIO33", "ADC-5V 分圧の上"),
    ("R12", "10 kΩ", "MEAS-01 分圧用", "B1:L4", "GPIO33", "B1:-4", "GND", "ADC-5V 分圧の下"),
    ("R13", "10 kΩ", "RES-PULL-01", "B2:e3", "GPIO22", "B2:f3", "3V3", "LCD-CS の pull-up"),
    ("R14", "10 kΩ", "RES-PULL-01", "B2:e4", "GPIO16", "B2:f4", "3V3", "LCD-RST の pull-up"),
    ("R15", "10 kΩ", "RES-PULL-01", "B2:e10", "GPIO21", "B2:f10", "3V3", "TOUCH-CS の pull-up"),
    ("R16", "4.7 kΩ", "RES-PULL-01", "B2:e8", "GPIO4", "B2:f8", "GND", "LCD-BL の pull-down"),
    ("R17", "10 kΩ", "MEAS-01 分圧用", "B3:g10", "3V3", "B3:g14", "GPIO36", "ADC-3V3 分圧の上"),
    ("R18", "10 kΩ", "MEAS-01 分圧用", "B3:h14", "GPIO36", "B3:h11", "GND", "ADC-3V3 分圧の下"),
]

# (id, 品, アノード, カソード)
LEDS = [
    ("D1", "白 LED-COMM-01（OSPW5111B-QR）", "B1:L1", "B1:-1", "LED_W_A"),
    ("D2", "赤 LED-REACT-01（OSHR5161A-QR）", "B1:L6", "B1:-6", "LED_R_A"),
]

# 硬いジャンパー（被覆あり、1方向に曲がったもの）。(id, 端1, 端2, 信号)
HARD = [
    ("H1", "B1:L12", "B1:+12", "3V3"),
    ("H2", "B1:L25", "B1:-25", "GND"),
    ("H3", "B2:e1", "B2:f1", "3V3"),
    ("H4", "B2:e2", "B2:f2", "GND"),
    ("H5", "B2:j1", "B2:j3", "3V3"),
    ("H6", "B2:i3", "B2:i4", "3V3"),
    ("H7", "B2:h4", "B2:h10", "3V3"),
    ("H8", "B2:g2", "B2:g8", "GND"),
    ("H9", "B3:e10", "B3:f10", "3V3"),
    ("H10", "B3:e11", "B3:f11", "GND"),
    ("H11", "B3:c11", "B3:c14", "GND"),
]

# 柔らかい線（両端オスのジャンパー線）。端が板の外のものは文字列で書く。(id, 端1, 端2, 信号, 状態)
# 状態: 空文字は「挿す」。それ以外は組み直した直後に抜いておく端と理由。
SOFT = [
    ("S1", "B1:+26", "B2:b1", "3V3", ""),
    ("S2", "B1:-26", "B2:b2", "GND", ""),
    ("S3", "B1:+27", "B3:b10", "3V3", ""),
    ("S4", "B1:-27", "B3:b11", "GND", ""),
    ("S5", "B1:A14", "B2:b3", "GPIO22", ""),
    ("S6", "B1:A23", "B2:b4", "GPIO16", ""),
    ("S7", "B1:A22", "B2:b5", "GPIO17", ""),
    ("S8", "B1:A13", "B2:c6", "GPIO23", ""),
    ("S9", "B1:A20", "B2:c7", "GPIO18", ""),
    ("S10", "B1:A24", "B2:b8", "GPIO4", ""),
    ("S11", "B1:A19", "B2:c9", "GPIO19", ""),
    ("S12", "B1:A17", "B2:b10", "GPIO21", ""),
    ("S13", "B1:K16", "B2:b11", "GPIO34", ""),
    ("S14", "B1:K20", "B3:g5", "GPIO25", ""),
    ("S15", "B1:K21", "B3:g6", "GPIO26", ""),
    ("S16", "B1:K14", "B3:f14", "GPIO36", ""),
    ("S17", "B1:K19", "B1:G4", "GPIO33", ""),
    ("S18", "B1:K30", "B1:G8", "5V", "G8 側を先に挿し、K30 側を後に挿す（5 V の端を最後にする）"),
    ("S19", "B1:A26", "B1:G5", "GPIO2", "A26 側を抜いておく（EXP-026 で ESP32 側を抜いた状態）"),
    ("S20", "B1:A21", "B1:G2", "GPIO5", "A21 側を抜いておく（EXP-026 で ESP32 側を抜いた状態）"),
    ("S21", "B1:K23", "B1:F1", "GPIO14",
     "K23 側を抜いておく。挿すのは gpio-assignment.md の `信号線をつないでよい条件` を満たすときだけ（ESP32 側の端で抜き差しする）"),
    ("S22", "B1:K26", "B1:F2", "GPIO13", "K26 側を抜いておく。条件は S21 と同じ"),
    ("S23", "B1:K22", "B1:F3", "GPIO27", ""),
    ("S24", "B3:h5", "B3:b13", "GPIO25", ""),
    ("S25", "B3:h6", "B3:b15", "GPIO26", ""),
    ("S26", "B3:c10", "ACCEL:Vs", "3V3", ""),
    ("S27", "B3:d11", "ACCEL:GND", "GND", ""),
    ("S28", "ACCEL:CS", "ACCEL:VDD", "3V3", ""),
    ("S29", "B3:g4", "B3:i11", "GND", ""),
]

# 板の外へ出る線。(id, 板の端, 板の端の信号, 板の外の端, 状態)
OFFBOARD = [
    ("X1", "B1:D5", "PI_TXD", "Pi の header pin 8（GPIO14 `TXD`）",
     "挿す（抜き差しは S21 の K23 側で行う。Pi の header 側では抜き差ししない）"),
    ("X2", "B1:D6", "PI_RXD", "Pi の header pin 10（GPIO15 `RXD`）",
     "挿す（抜き差しは S22 の K26 側で行う。Pi の header 側では抜き差ししない）"),
    ("X3", "B1:A18", "GND", "Pi の header pin 39（GND）",
     "A18 側を抜いておく。挿す前に、gpio-assignment.md の `信号線をつないでよい条件` の6の開放側の値を読む"),
    ("X4", "B1:D7", "SERVO_SIG", "サーボの plug の黄の穴",
     "D7 側を抜いておく。挿すのは #17 のサーボの試験の手順と承認に従うときだけ"),
    ("X5", "B1:-30", "GND", "端子台S の −（ESP32 の GND をサーボの GND と共通にする線。EXP-034 の Wiring revision、hardware/breadboard/layout.svg の基準線）",
     "−30 側を抜いておく。条件は X4 と同じ"),
]

# 首を通る14本（LCD 側は MSP2807 の pin 番号。#34 の決定）。線は両端メス（enclosure-and-neck.md の仮筐体）。
# 胴側は、細ピンヘッダ（PROTO-01）を1本ずつ下の穴に立てて受ける（VM 側の PM の決定、2026-10-23）。
NECK = [
    (1, "VCC", "B2:a1", "3V3"), (2, "GND", "B2:a2", "GND"), (3, "CS", "B2:a3", "GPIO22"),
    (4, "RESET", "B2:a4", "GPIO16"), (5, "DC/RS", "B2:a5", "GPIO17"), (6, "SDI", "B2:a6", "GPIO23"),
    (7, "SCK", "B2:a7", "GPIO18"), (8, "LED", "B2:a8", "GPIO4"), (9, "SDO", "B2:a9", "GPIO19"),
    (10, "T_CLK", "B2:b7", "GPIO18"), (11, "T_CS", "B2:a10", "GPIO21"), (12, "T_DIN", "B2:b6", "GPIO23"),
    (13, "T_DO", "B2:b9", "GPIO19"), (14, "T_IRQ", "B2:a11", "GPIO34"),
]

# module のピン（板の上）。ENV-01 は1列6本、ACCEL-01 は下段5本（上段4本は穴が TBD）。
MODULE_PINS = [
    ("ENV-01", "VDD", "B3:a10", "3V3"), ("ENV-01", "GND", "B3:a11", "GND"),
    ("ENV-01", "CSB", "B3:a12", "使わない:ENV-CSB"), ("ENV-01", "SDI", "B3:a13", "GPIO25"),
    ("ENV-01", "SDO", "B3:a14", "GND"), ("ENV-01", "SCK", "B3:a15", "GPIO26"),
    ("ACCEL-01", "INT1", "B3:f2", "使わない:ACCEL-INT1"), ("ACCEL-01", "INT2", "B3:f3", "使わない:ACCEL-INT2"),
    ("ACCEL-01", "SDO", "B3:f4", "GND"), ("ACCEL-01", "SDA", "B3:f5", "GPIO25"),
    ("ACCEL-01", "SCL", "B3:f6", "GPIO26"),
]
# 上段4本（CS Vs GND VDD）。列はユーザーの写真（2026-10-23）で CS が INT1 の上、VDD が SDA の上に見えるので
# 2〜5列とする（写真の読み。挿すときに確かめる）。行は2列の間隔が記録にもメーカーの資料にも無く TBD。
# 間隔が3〜6ピッチなら上段は上半分の e〜b 行に来て、上に空き穴が1つ以上残る。各ピンの列の空き穴へ線を挿す。
ACCEL_UPPER = [("CS", "3V3"), ("Vs", "3V3"), ("GND", "GND"), ("VDD", "3V3")]
# module の中でつながっている pin。ACCEL-01 の Vs と VDD は、2026-09-05 の非通電の導通測定で同じ net
# （sensor-datasheet-notes.md の `module boardの値（秋月 M-06724）`）。
# ESP32 の GND の3本（J2-14、J3-1、J3-7）は、公式回路図（gpio-assignment.md の `Board識別情報`）で同じ GND。
# 現物の導通は測っていない。
MODULE_INTERNAL = [
    ("ACCEL:Vs", "ACCEL:VDD"),
    (f"B1:{ESP_J2_ROW}{ESP_FIRST_COL + 13}", f"B1:{ESP_J3_ROW}{ESP_FIRST_COL}"),
    (f"B1:{ESP_J2_ROW}{ESP_FIRST_COL + 13}", f"B1:{ESP_J3_ROW}{ESP_FIRST_COL + 6}"),
]


# ---------------------------------------------------------------- 検査


class Check:
    def __init__(self):
        self.errors = []

    def fail(self, msg):
        self.errors.append(msg)


def is_hole(text):
    return text[:2] in BOARDS and ":" in text


def endpoints():
    """穴を使う端をすべて列挙する: (穴, 何の端か)。"""
    out = []
    for p in esp_pins():
        out.append((p["hole"], f"ESP32 {p['label']}"))
    for mod, pin, h, _ in MODULE_PINS:
        out.append((parse_hole(h), f"{mod} {pin}"))
    for rid, *_r in RESISTORS:
        out.append((parse_hole(_r[2]), f"{rid} の端"))
        out.append((parse_hole(_r[4]), f"{rid} の端"))
    for did, _p, a, k, _n in LEDS:
        out.append((parse_hole(a), f"{did} アノード"))
        out.append((parse_hole(k), f"{did} カソード"))
    for hid, a, b, _n in HARD:
        out.append((parse_hole(a), f"{hid} の端"))
        out.append((parse_hole(b), f"{hid} の端"))
    for sid, a, b, _n, _s in SOFT:
        for e in (a, b):
            if is_hole(e):
                out.append((parse_hole(e), f"{sid} の端"))
    for xid, h, *_x in OFFBOARD:
        out.append((parse_hole(h), f"{xid} の端"))
    for n, _name, h, _net in NECK:
        out.append((parse_hole(h), f"首の線 {n}"))
    return out


def covered_holes():
    """ESP32 の本体の下と、アンテナの近くの使わない穴。"""
    cov = set()
    for col in range(ESP_FIRST_COL, ESP_FIRST_COL + 19):
        for row in "CDEFGHI":
            cov.add(("B1", row, col))
    for col in KEEP_OUT_COLS:
        for row in "ABCDEFGHIJKL":
            cov.add(("B1", row, col))
    return cov


def flat_parts():
    """板の上に寝かせる部品と硬いジャンパー: (id, 端1, 端2, 裸の金属か)。"""
    parts = []
    for rid, _v, _p, a, _na, b, _nb, _u in RESISTORS:
        parts.append((rid, parse_hole(a), parse_hole(b)))
    for did, _p, a, k, _n in LEDS:
        parts.append((did, parse_hole(a), parse_hole(k)))
    for hid, a, b, _n in HARD:
        parts.append((hid, parse_hole(a), parse_hole(b)))
    return parts


def y_of(hole):
    board, row, _col = hole
    return BOARDS[board]["rows"][row]


def between_holes(a, b):
    """縦か横の直線で a と b の間にある穴（a、b を除く）。"""
    board = a[0]
    spec = BOARDS[board]
    if a[1] == b[1]:  # 同じ行（横）
        lo, hi = sorted((a[2], b[2]))
        cols = range(lo + 1, hi)
        if a[1] in "-+":
            cols = [c for c in cols if c in RAIL_COLS]
        return [(board, a[1], c) for c in cols]
    if a[2] == b[2]:  # 同じ列（縦）
        ya, yb = sorted((y_of(a), y_of(b)))
        rows = [r for r, y in spec["rows"].items() if ya < y < yb]
        out = []
        for r in rows:
            if r in "-+" and a[2] not in RAIL_COLS:
                continue
            out.append((board, r, a[2]))
        return out
    return None


def segments_cross(p1, p2, q1, q2):
    def orient(a, b, c):
        v = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        return (v > 0) - (v < 0)

    o1, o2 = orient(p1, p2, q1), orient(p1, p2, q2)
    o3, o4 = orient(q1, q2, p1), orient(q1, q2, p2)
    return o1 * o2 < 0 and o3 * o4 < 0


def pitch_length(a, b):
    if a[1] == b[1]:
        return abs(a[2] - b[2])
    return abs(y_of(a) - y_of(b))


def run_checks():
    ck = Check()
    used = {}
    for hole, what in endpoints():
        if hole in used:
            ck.fail(f"{hole_name(hole)} に2つ挿している: {used[hole]} と {what}")
        used[hole] = what
    cov = covered_holes()
    for hole, what in used.items():
        if hole in cov and not what.startswith("ESP32"):
            ck.fail(f"{hole_name(hole)} は ESP32 の本体・アンテナの下か近くで使わない穴: {what}")

    parts = flat_parts()
    for pid, a, b in parts:
        if a[0] != b[0]:
            ck.fail(f"{pid} の両端が別の板にある")
            continue
        mid = between_holes(a, b)
        if mid is None:
            ck.fail(f"{pid} が縦横の直線でない（{hole_name(a)}–{hole_name(b)}）")
            continue
        for h in mid:
            if h in used:
                ck.fail(f"{pid} が {hole_name(h)}（{used[h]}）の上を通る")
            if h in cov:
                ck.fail(f"{pid} が ESP32 の下・近くの {hole_name(h)} を通る")
        if strip_of(a) == strip_of(b):
            ck.fail(f"{pid} の両端が同じ列（つながった穴）にある")
    for i, (pa, a1, a2) in enumerate(parts):
        for pb, b1, b2 in parts[i + 1:]:
            if a1[0] != b1[0]:
                continue
            p = [(h[2], y_of(h)) for h in (a1, a2, b1, b2)]
            if segments_cross(p[0], p[1], p[2], p[3]):
                ck.fail(f"{pa} と {pb} が交差する")

    # 論理接続
    parent = {}

    def find(x):
        parent.setdefault(x, x)
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a, b):
        parent[find(a)] = find(b)

    def node(text):
        return ("S",) + strip_of(parse_hole(text)) if is_hole(text) else ("M", text)

    expect = {}

    def want(n, net, why):
        expect.setdefault(find(n), []).append((net, why))

    for _hid, a, b, _n in HARD:
        union(node(a), node(b))
    for _sid, a, b, _n, _s in SOFT:
        union(node(a), node(b))
    for a, b in MODULE_INTERNAL:
        union(node(a), node(b))
    # 期待（union の後に集める）
    labels = []
    for p in esp_pins():
        labels.append((("S",) + strip_of(p["hole"]), p["net"], f"ESP32 {p['label']}"))
    for mod, pin, h, net in MODULE_PINS:
        labels.append((node(h), net, f"{mod} {pin}"))
    for pin, net in ACCEL_UPPER:
        labels.append((("M", f"ACCEL:{pin}"), net, f"ACCEL-01 {pin}（上段）"))
    for rid, _v, _p, a, na, b, nb, _u in RESISTORS:
        labels.append((node(a), na, f"{rid}"))
        labels.append((node(b), nb, f"{rid}"))
    for did, _p, a, k, na in LEDS:
        labels.append((node(a), na, f"{did} アノード"))
        labels.append((node(k), "GND", f"{did} カソード"))
    for hid, a, b, net in HARD:
        labels.append((node(a), net, hid))
        labels.append((node(b), net, hid))
    for sid, a, b, net, _s in SOFT:
        labels.append((node(a), net, sid))
        labels.append((node(b), net, sid))
    for xid, h, net, _far, _s in OFFBOARD:
        labels.append((node(h), net, xid))
    for n, _name, h, net in NECK:
        labels.append((node(h), net, f"首の線 {n}"))
    for n, net, why in labels:
        want(n, net, why)
    for _root, items in expect.items():
        nets = sorted({net for net, _ in items})
        if len(nets) != 1:
            ck.fail("1つの導体に別の信号が混ざる: " + "、".join(f"{w}={n}" for n, w in items))
    # 断線: 同じ信号名の端は、すべて1つの導体につながっている（つながない pin は除く）
    roots = {}
    for n, net, why in labels:
        if not net.startswith("使わない:"):
            roots.setdefault(net, {}).setdefault(find(n), []).append(why)
    for net, parts_of in roots.items():
        if len(parts_of) > 1:
            ck.fail(f"{net} が {len(parts_of)} つの導体に分かれている（断線）: "
                    + " ／ ".join("、".join(w) for w in parts_of.values()))
    for rid, _v, _p, a, _na, b, _nb, _u in RESISTORS:
        if find(node(a)) == find(node(b)):
            ck.fail(f"{rid} の両端が同じ導体（短絡）")
    for did, _p, a, k, _n in LEDS:
        if find(node(a)) == find(node(k)):
            ck.fail(f"{did} の両端が同じ導体（短絡）")
    # 抵抗は寝かせられる長さか（1/4 W の本体を寝かせるには、3ピッチ以上を目安にする）
    for rid, _v, _p, a, _na, b, _nb, _u in RESISTORS:
        if pitch_length(parse_hole(a), parse_hole(b)) < 3:
            ck.fail(f"{rid} の両端の間が3ピッチ未満で寝かせられない")
    check_5v(ck, find, node, used, parts)
    return ck.errors


# 板の外へ出してよい信号。サーボの電源（＋）と Pi の給電はブレッドボードを通さないので、ここに無い。
OFFBOARD_ALLOWED = {"PI_TXD", "PI_RXD", "GND", "SERVO_SIG"}


def check_5v(ck, find, node, used, parts):
    """5 V の安全（VM 側の PM の指定、2026-10-23）を検査する。

    1. GPIO33 へは 5V → 10 kΩ → 分圧点（GPIO33）→ 10 kΩ → GND の順でだけ届く。
    2. 5 V の列の両隣の列は空いている（ESP32 自身のピンを除く）。5 V の列と両隣の穴の上を、
       ほかの部品・硬いジャンパーが通らない。
    3. 板の外へ出る線にサーボの電源と Pi の給電が無い。
    """
    five = find(node("B1:K30"))
    pin33 = find(("S",) + strip_of(("B1", ESP_J2_ROW, ESP_FIRST_COL + J2.index("33"))))
    gnd = find(("S",) + strip_of(("B1", ESP_J2_ROW, ESP_FIRST_COL + J2.index("GND"))))
    if pin33 == five:
        ck.fail("5V と GPIO33 が同じ導体（分圧器を通らない経路）")

    def touching(root):
        out = []
        for rid, _v, _p, a, _na, b, _nb, _u in RESISTORS:
            ra, rb = find(node(a)), find(node(b))
            if root in (ra, rb):
                out.append((rid, rb if ra == root else ra))
        for did, _p, a, k, _n in LEDS:
            ra, rk = find(node(a)), find(node(k))
            if root in (ra, rk):
                out.append((did, rk if ra == root else ra))
        return out

    t5 = touching(five)
    if [rid for rid, _o in t5] != ["R11"] or t5[0][1] != pin33:
        ck.fail(f"5V に触れる部品が R11（先は GPIO33）だけでない: {t5}")
    t33 = sorted(touching(pin33))
    if t33 != sorted([("R11", five), ("R12", gnd)]):
        ck.fail(f"GPIO33 に触れる部品が R11（5V）と R12（GND）だけでない: {t33}")

    five_strips = {strip_of(h) for h in used if find(("S",) + strip_of(h)) == five}
    watch = set(five_strips)
    for s in five_strips:
        if len(s) != 3:
            ck.fail(f"5V が電源列にある: {s}")
            continue
        board, half, col = s
        for nb in (col - 1, col + 1):
            ns = (board, half, nb)
            watch.add(ns)
            for h, what in used.items():
                if strip_of(h) == ns and not what.startswith("ESP32"):
                    ck.fail(f"5V の列 {board} {col} 列の隣の列に {what}（{hole_name(h)}）がある")
    for pid, a, b in parts:
        if pid == "R11":
            continue
        mid = between_holes(a, b) or []
        for h in mid + [a, b]:
            if strip_of(h) in watch:
                ck.fail(f"{pid} が 5V の列かその隣の列（{hole_name(h)}）にかかる")

    for xid, _h, net, _far, _s in OFFBOARD:
        if net not in OFFBOARD_ALLOWED:
            ck.fail(f"{xid} の信号 {net} は板の外へ出してよい信号でない")


# ---------------------------------------------------------------- 図

P = 22  # 1ピッチの px
BOARD_POS = {"B1": (60, 560), "B2": (60, 130), "B3": (900, 560)}
SVG_W, SVG_H = 1700, 1560

NET_COLOR = {"3V3": "#d62728", "GND": "#111111", "5V": "#e377c2"}


def hole_xy(hole):
    board, row, col = hole
    ox, oy = BOARD_POS[board]
    return ox + 40 + (col - 1) * P, oy + 50 + BOARDS[board]["rows"][row] * P


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def draw_board(out, board):
    spec = BOARDS[board]
    ox, oy = BOARD_POS[board]
    w = 40 + (spec["cols"] - 1) * P + 40
    rows = spec["rows"]
    h = 50 + max(rows.values()) * P + 40
    out.append(f'<rect x="{ox}" y="{oy}" width="{w}" height="{h}" rx="10" fill="#f4f4f0" stroke="#888"/>')
    out.append(f'<text x="{ox}" y="{oy - 10}" font-size="17" font-weight="bold">{esc(board + " " + spec["title"])}</text>')
    for col in range(1, spec["cols"] + 1):
        x, _ = hole_xy((board, next(iter(rows)), col))
        out.append(f'<text x="{x}" y="{oy + 30}" text-anchor="middle" font-size="10" fill="#666">{col}</text>')
    for row, y_i in rows.items():
        _, y = hole_xy((board, row, 1))
        shown = {"-": "−", "+": "＋"}.get(row, row)
        color = {"-": "#1f5fbf", "+": "#d62728"}.get(row, "#666")
        out.append(f'<text x="{ox + 18}" y="{y + 4}" text-anchor="middle" font-size="11" fill="{color}">{shown}</text>')
        if row in "-+":
            x1, _ = hole_xy((board, row, 1))
            x2, _ = hole_xy((board, row, spec["cols"]))
            out.append(f'<line x1="{x1 - 8}" y1="{y + (8 if row == "+" else -8)}" x2="{x2 + 8}" y2="{y + (8 if row == "+" else -8)}" stroke="{color}" stroke-width="2"/>')
        for col in range(1, spec["cols"] + 1):
            if row in "-+" and col not in RAIL_COLS:
                continue
            x, _ = hole_xy((board, row, col))
            out.append(f'<rect x="{x - 3}" y="{y - 3}" width="6" height="6" fill="#bbb"/>')
    if board == "B1":
        x14, y = hole_xy(("B1", "+", 14))
        x17, _ = hole_xy(("B1", "+", 17))
        out.append(f'<rect x="{x14 + 4}" y="{y - 3}" width="{x17 - x14 - 8}" height="6" fill="#d62728"/>')
        out.append(f'<text x="{(x14 + x17) / 2}" y="{y + 20}" text-anchor="middle" font-size="10" fill="#d62728">赤のつなぎ（外さない）</text>')


def draw_esp(out):
    x1, y1 = hole_xy(("B1", "B", ESP_FIRST_COL))
    x2, y2 = hole_xy(("B1", "J", ESP_FIRST_COL + 18))
    ax, _ = hole_xy(("B1", "B", 9))
    out.append(f'<rect x="{ax + 14}" y="{y1 - 8}" width="{x1 - ax - 14 + 4}" height="{y2 - y1 + 16}" fill="#555" opacity="0.55"/>')
    out.append(f'<text x="{(ax + x1) / 2 + 8}" y="{(y1 + y2) / 2}" text-anchor="middle" font-size="10" fill="#fff" transform="rotate(-90 {(ax + x1) / 2 + 8} {(y1 + y2) / 2})">アンテナ（9〜11列は使わない）</text>')
    out.append(f'<rect x="{x1 - 10}" y="{y1 - 8}" width="{x2 - x1 + 20}" height="{y2 - y1 + 16}" rx="4" fill="#222" opacity="0.82"/>')
    out.append(f'<text x="{(x1 + x2) / 2}" y="{(y1 + y2) / 2 - 6}" text-anchor="middle" font-size="14" fill="#fff">ESP32 DevKitC V4（MCU-01）</text>')
    out.append(f'<text x="{(x1 + x2) / 2}" y="{(y1 + y2) / 2 + 14}" text-anchor="middle" font-size="11" fill="#ddd">上の列＝J3（B行）、下の列＝J2（J行）。USB は右（30列の外）</text>')
    for p in esp_pins():
        x, y = hole_xy(p["hole"])
        name = p["label"].split()[1]
        out.append(f'<circle cx="{x}" cy="{y}" r="4" fill="#caa53d"/>')
        if p["hole"][1] == ESP_J3_ROW:
            ty = y + 15
        else:
            ty = y - 8
        out.append(f'<text x="{x}" y="{ty}" text-anchor="middle" font-size="8.5" fill="#fff">{esc(name)}</text>')


def draw_parts(out):
    for rid, val, _p, a, _na, b, _nb, _u in RESISTORS:
        (x1, y1), (x2, y2) = hole_xy(parse_hole(a)), hole_xy(parse_hole(b))
        out.append(f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#7a6a50" stroke-width="2"/>')
        mx, my = (x1 + x2) / 2, (y1 + y2) / 2
        if y1 == y2:
            out.append(f'<rect x="{mx - 16}" y="{my - 5}" width="32" height="10" rx="4" fill="#e9d3a5" stroke="#7a6a50"/>')
        else:
            out.append(f'<rect x="{mx - 5}" y="{my - 14}" width="10" height="28" rx="4" fill="#e9d3a5" stroke="#7a6a50"/>')
        if y1 == y2:
            out.append(f'<text x="{mx}" y="{my - 8}" text-anchor="middle" font-size="9" fill="#5a4a30">{rid} {esc(val)}</text>')
        else:  # 縦の抵抗は隣と近いので ID だけを書く（値は結線表）
            out.append(f'<text x="{mx}" y="{my + 3}" text-anchor="middle" font-size="7" fill="#5a4a30" transform="rotate(-90 {mx} {my})">{rid}</text>')
    for did, part, a, k, _n in LEDS:
        (xa, ya), (xk, yk) = hole_xy(parse_hole(a)), hole_xy(parse_hole(k))
        color = "#ffffff" if part.startswith("白") else "#ff4040"
        out.append(f'<line x1="{xa}" y1="{ya}" x2="{xk}" y2="{yk}" stroke="#888" stroke-width="2"/>')
        out.append(f'<circle cx="{xa}" cy="{(ya + yk) / 2}" r="9" fill="{color}" stroke="#333"/>')
        out.append(f'<text x="{xa - 12}" y="{ya + 3}" text-anchor="end" font-size="9">A</text>')
        out.append(f'<text x="{xk - 12}" y="{yk + 3}" text-anchor="end" font-size="9">K</text>')
        out.append(f'<text x="{xa + 12}" y="{(ya + yk) / 2 + 3}" font-size="9">{did}</text>')
    for hid, a, b, net in HARD:
        (x1, y1), (x2, y2) = hole_xy(parse_hole(a)), hole_xy(parse_hole(b))
        color = NET_COLOR.get(net, "#2a9d8f")
        out.append(f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{color}" stroke-width="5" stroke-linecap="round" opacity="0.85"/>')
        out.append(f'<text x="{(x1 + x2) / 2 + 6}" y="{(y1 + y2) / 2 - 5}" font-size="9" fill="#0b5">{hid}</text>')


def soft_end_xy(text):
    if is_hole(text):
        return hole_xy(parse_hole(text))
    # ACCEL の上段（穴は TBD）。図では B3 の上半分の上に仮の位置で描く。
    pins = [p for p, _n in ACCEL_UPPER]
    pin = text.split(":")[1]
    x, _ = hole_xy(("B3", "a", 2 + pins.index(pin)))
    _, y = hole_xy(("B3", "c", 1))
    return x, y


def draw_soft(out):
    for sid, a, b, net, state in SOFT:
        (x1, y1), (x2, y2) = soft_end_xy(a), soft_end_xy(b)
        color = NET_COLOR.get(net, "#2f6fb0")
        dash = ' stroke-dasharray="6 4"' if "抜いておく" in state else ""
        cx, cy = (x1 + x2) / 2, min(y1, y2) - 40
        out.append(f'<path d="M{x1} {y1} Q{cx} {cy} {x2} {y2}" stroke="{color}" stroke-width="1.6" fill="none" opacity="0.55"{dash}/>')
        for x, y in ((x1, y1), (x2, y2)):
            out.append(f'<circle cx="{x}" cy="{y}" r="4.5" fill="#fff" stroke="{color}" stroke-width="2"/>')
        out.append(f'<text x="{x2 + 6}" y="{y2 - 6}" font-size="9" fill="{color}">{sid}</text>')
        out.append(f'<text x="{x1 + 6}" y="{y1 - 6}" font-size="9" fill="{color}">{sid}</text>')
    for xid, h, _net, far, state in OFFBOARD:
        x, y = hole_xy(parse_hole(h))
        if "抜いておく" in state:  # 板の端を抜いておく線は、白抜きの点線の丸
            out.append(f'<circle cx="{x}" cy="{y}" r="5" fill="#fff" stroke="#a07800" stroke-width="2" stroke-dasharray="2 2"/>')
        else:
            out.append(f'<circle cx="{x}" cy="{y}" r="5" fill="#ffd54d" stroke="#a07800" stroke-width="2"/>')
        out.append(f'<text x="{x + 7}" y="{y + 12}" font-size="9" fill="#806000">{xid}</text>')
    for n, name, h, _net in NECK:
        x, y = hole_xy(parse_hole(h))
        out.append(f'<circle cx="{x}" cy="{y}" r="5.5" fill="#9b59b6" stroke="#4a235a" stroke-width="1.5"/>')
        out.append(f'<text x="{x}" y="{y + 3}" text-anchor="middle" font-size="7.5" fill="#fff">{n}</text>')


def draw_modules(out):
    # ENV-01: 1列6本。本体の向きと大きさは TBD（図は仮）。
    x1, y = hole_xy(("B3", "a", 10))
    x2, _ = hole_xy(("B3", "a", 15))
    out.append(f'<rect x="{x1 - 12}" y="{y - 70}" width="{x2 - x1 + 24}" height="60" rx="6" fill="#b9e3c6" stroke="#333" stroke-dasharray="5 3"/>')
    out.append(f'<text x="{(x1 + x2) / 2}" y="{y - 46}" text-anchor="middle" font-size="12" font-weight="bold">ENV-01（AE-BME280）</text>')
    out.append(f'<text x="{(x1 + x2) / 2}" y="{y - 30}" text-anchor="middle" font-size="9.5">本体は板の外へ出す向き（形は TBD）</text>')
    for _mod, pin, h, _n in [m for m in MODULE_PINS if m[0] == "ENV-01"]:
        x, yy = hole_xy(parse_hole(h))
        out.append(f'<circle cx="{x}" cy="{yy}" r="4.5" fill="#caa53d"/>')
        out.append(f'<text x="{x}" y="{yy - 8}" text-anchor="middle" font-size="8.5">{pin}</text>')
    # ACCEL-01: 下段5本を f2〜f6。上段4本は2列の間隔が記録に無く TBD。
    xl, yl = hole_xy(("B3", "f", 2))
    xr, _ = hole_xy(("B3", "f", 6))
    _, yt = hole_xy(("B3", "c", 1))
    out.append(f'<rect x="{xl - 14}" y="{yt - 14}" width="{xr - xl + 28}" height="{yl - yt + 28}" rx="6" fill="#f2b8cc" opacity="0.7" stroke="#333" stroke-dasharray="5 3"/>')
    out.append(f'<text x="{(xl + xr) / 2}" y="{(yt + yl) / 2 + 4}" text-anchor="middle" font-size="11" font-weight="bold">ACCEL-01</text>')
    for _mod, pin, h, _n in [m for m in MODULE_PINS if m[0] == "ACCEL-01"]:
        x, yy = hole_xy(parse_hole(h))
        out.append(f'<circle cx="{x}" cy="{yy}" r="4.5" fill="#caa53d"/>')
        out.append(f'<text x="{x}" y="{yy + 16}" text-anchor="middle" font-size="8">{pin}</text>')
    for i, (pin, _n) in enumerate(ACCEL_UPPER):
        x, _ = hole_xy(("B3", "a", 2 + i))
        out.append(f'<circle cx="{x}" cy="{yt}" r="4.5" fill="none" stroke="#a00" stroke-width="1.5" stroke-dasharray="2 2"/>')
        out.append(f'<text x="{x}" y="{yt + 16}" text-anchor="middle" font-size="8" fill="#a00">{pin}</text>')
    out.append(f'<text x="{xl - 14}" y="{yt - 20}" font-size="9.5" fill="#a00">上段4本は2〜5列（写真の読み）。行は TBD（点線は仮の位置）</text>')


NOTES = [
    "この図は組み直しの設計（#512）。まだ組んでいない。今の配線の記録は hardware/breadboard/layout.svg。",
    "結線表と出所・TBD は同じ場所の wiring-table.md と README.md。図と表は generate.py から生成する。",
    "灰の四角＝穴。金の丸＝ピン。ベージュ＝抵抗（板に沿わせて寝かせる）。太い線＝硬いジャンパー（被覆あり）。",
    "白丸と曲線＝柔らかい線（S番号、両端に同じ番号）。点線の曲線＝組み直した直後は ESP32 側を抜いておく線。",
    "黄の丸＝板の外へ出る線（X番号）。白抜きの点線の丸＝そのうち組み直した直後は板の端を抜いておく線。紫の丸＝首を通る LCD の線（数字は LCD の pin 番号）。",
    "＋の列＝3V3、−の列＝GND（主板の下側の1組だけ）。5 V の線は S18 だけで、ADC-5V の分圧へ行く。",
    "サーボの電源・Pi の給電はブレッドボードを通さない（power-budget.md、#34）。",
    "溝の幅、L 行から電源列までの距離は写真からの目安。部品の足の長さと硬いジャンパーの長さは現物で合わせる。",
    "図面を電源・安全の実機確認に使わない。通電前の照合は実機側が行う。",
]


def text_width(text, size):
    """文字の幅の見積もり（全角を 1 em、それ以外を 0.55 em とする。描画して測った値ではない）。"""
    return sum(size if ord(ch) > 0x2E7F else size * 0.55 for ch in text)


def check_text_fits(lines):
    """図の下の文字が図の幅に収まり、同じ高さで重ならないことを確かめる。"""
    for x, y, size, text, _bold in lines:
        if x + text_width(text, size) > SVG_W - 10:
            raise SystemExit(f"NG: 図の幅に収まらない文字（y={y}）: {text}")
    ys = [y for _x, y, _s, _t, _b in lines]
    if len(ys) != len(set(ys)):
        raise SystemExit("NG: 図の下の文字が同じ高さに2行ある")
    if max(ys) > SVG_H - 10:
        raise SystemExit("NG: 図の下の文字が図の高さに収まらない")


def build_svg():
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {SVG_W} {SVG_H}" '
        'font-family="Noto Sans CJK JP, Hiragino Sans, Meiryo, sans-serif" font-size="13">',
        f'<rect width="{SVG_W}" height="{SVG_H}" fill="#fff"/>',
        '<text x="30" y="36" font-size="22" font-weight="bold">ブレッドボードの組み直し配置（設計。#512）</text>',
        '<text x="30" y="62" fill="#444">生成元: hardware/breadboard-rebuild/generate.py。組み直した後に hardware/breadboard/layout.svg の最終版になる。</text>',
    ]
    for b in BOARDS:
        draw_board(out, b)
    draw_esp(out)
    draw_modules(out)
    draw_parts(out)
    draw_soft(out)
    # 板（B1・B3 の下端は y=1024）より下に、凡例と板の外へ出る線を縦に並べる（横に並べると長い行が重なる）
    lines = [(60, 1070, 15, "凡例と注意", True)]
    lines += [(60, 1096 + i * 22, 12.5, line, False) for i, line in enumerate(NOTES)]
    y2 = 1096 + len(NOTES) * 22 + 20
    lines.append((60, y2, 15, "板の外へ出る線", True))
    for i, (xid, h, _net, far, state) in enumerate(OFFBOARD):
        mark = "（板の端を抜いておく）" if "抜いておく" in state else ""
        lines.append((60, y2 + 26 + i * 22, 12,
                      f'{xid}: {hole_name(parse_hole(h))} → {far.replace("`", "")}{mark}', False))
    yy = y2 + 26 + len(OFFBOARD) * 22 + 10
    lines.append((60, yy, 12, "首の14本: LCD 側は MSP2807 の pin 番号。線は両端メス。B2 の a・b 行に立てた細ピンヘッダで受ける。", False))
    lines.append((60, yy + 22, 12, "SCK と T_CLK、SDI と T_DIN、SDO と T_DO は同じ列に挿して共有する（#34）。", False))
    check_text_fits(lines)
    for x, y, size, text, bold in lines:
        weight = ' font-weight="bold"' if bold else ""
        out.append(f'<text x="{x}" y="{y}" font-size="{size}"{weight}>{esc(text)}</text>')
    out.append("</svg>")
    return "\n".join(out) + "\n"


# ---------------------------------------------------------------- 結線表


def net_text(net):
    if net.startswith("使わない:"):
        return "つながない"
    note = NET_NOTE.get(net)
    return f"{net}（{note}）" if note and note != net else net


def end_text(e):
    if is_hole(e):
        return hole_name(parse_hole(e))
    pin = e.split(':')[1]
    col = 2 + [p for p, _n in ACCEL_UPPER].index(pin)
    return f"B3 の{col}列（ACCEL-01 上段の {pin}）の、上段より上の空き穴（行は TBD）"


def build_table():
    L = []
    L.append("<!-- generate.py が生成する。手で直さない。 -->")
    L.append("# 組み直しの結線表（#512）")
    L.append("")
    L.append("穴の書き方: `B1 L12` は主板の L 行・12列、`B1 −25` は主板の − の列の25列の下の穴、`B2 e3` は LCD 板の e 行・3列。")
    L.append("配置の考え方、値と並びの出所、`TBD` は [README.md](README.md) にある。")
    L.append("")
    L.append("## 板と module")
    L.append("")
    L.append("| 何 | どこへ | 向き |")
    L.append("|---|---|---|")
    L.append(f"| ESP32 DevKitC V4（`MCU-01`） | B1。J3（GND 23 22 … CLK）を B 行の12〜30列、J2（3V3 EN VP … 5V）を J 行の12〜30列 | USB を30列の側（板の端）、アンテナを12列の側にする。B 行の上に1穴（A）、J 行の下に2穴（K・L）が残る |")
    L.append("| `ENV-01`（AE-BME280） | B3 の a 行。VDD 10、GND 11、CSB 12、SDI 13、SDO 14、SCK 15 | 本体を板の外（上の辺の外）へ出す向き。ピンの名前の順が逆になる向きしか取れないときは止める（TBD） |")
    L.append("| `ACCEL-01`（M-06724） | 下段5本を B3 の f 行。INT1 2、INT2 3、SDO 4、SDA 5、SCL 6。上段4本（CS Vs GND VDD）は上半分の2〜5列（CS 2、Vs 3、GND 4、VDD 5。写真の読み）の、間隔に合う行 | 2列が溝をまたぐ向き。上段の行は TBD。間隔が7ピッチ以上で上段が a 行に来る（上に空き穴が残らない）ときは組めないので止める |")
    L.append("")
    L.append("## 抵抗（足は切る）")
    L.append("")
    L.append("| ID | 値 | 品 | 端1 | 端2 | 用途 |")
    L.append("|---|---|---|---|---|---|")
    for rid, val, part, a, na, b, nb, use in RESISTORS:
        L.append(f"| {rid} | {val} | `{part}` | {hole_name(parse_hole(a))} — {net_text(na)} | {hole_name(parse_hole(b))} — {net_text(nb)} | {use} |")
    L.append("")
    L.append("## LED（足は切る。向きをこの表で残す）")
    L.append("")
    L.append("| ID | 品 | アノード（長い足） | カソード（短い足） |")
    L.append("|---|---|---|---|")
    for did, part, a, k, _n in LEDS:
        L.append(f"| {did} | {part} | {hole_name(parse_hole(a))} | {hole_name(parse_hole(k))} — GND |")
    L.append("")
    L.append("## 硬いジャンパー（被覆あり）")
    L.append("")
    L.append("長さは両端の穴の間隔（ピッチ＝2.54 mm の数）。溝をまたぐものと電源列へ下りるものは、写真からの目安で TBD（現物で合う長さを選ぶ）。")
    L.append("")
    L.append("| ID | 端1 | 端2 | 長さ（ピッチ） | 信号 |")
    L.append("|---|---|---|---|---|")
    for hid, a, b, net in HARD:
        ha, hb = parse_hole(a), parse_hole(b)
        n = pitch_length(ha, hb)
        tbd = "（目安、TBD）" if ha[1] != hb[1] else ""
        L.append(f"| {hid} | {hole_name(ha)} | {hole_name(hb)} | {n}{tbd} | {net_text(net)} |")
    L.append("")
    L.append("## 柔らかい線（両端オス）")
    L.append("")
    L.append("| ID | 端1 | 端2 | 信号 | 組み直した直後 |")
    L.append("|---|---|---|---|---|")
    for sid, a, b, net, state in SOFT:
        L.append(f"| {sid} | {end_text(a)} | {end_text(b)} | {net_text(net)} | {state or '挿す'} |")
    L.append("")
    L.append("## 板の外へ出る線（柔らかい線）")
    L.append("")
    L.append("| ID | 板の端 | 板の外の端 | 信号 | 組み直した直後 |")
    L.append("|---|---|---|---|---|")
    for xid, h, net, far, state in OFFBOARD:
        L.append(f"| {xid} | {hole_name(parse_hole(h))} | {far} | {net_text(net)} | {state} |")
    L.append("")
    L.append("## 首を通る LCD の14本（両端メスの線）")
    L.append("")
    L.append("細ピンヘッダ（`PROTO-01`）を1本ずつに切り離して下の穴に立て、線の胴側のメスの端をそこへ挿す。LCD 側のメスの端は module のピンに挿す。")
    L.append("")
    L.append("| LCD pin | 表記 | 挿す穴 | 信号 |")
    L.append("|---:|---|---|---|")
    for n, name, h, net in NECK:
        L.append(f"| {n} | {name} | {hole_name(parse_hole(h))} | {net_text(net)} |")
    L.append("")
    L.append("## 足を切る部品・切らない部品")
    L.append("")
    L.append(f"- 切る: 抵抗 {len(RESISTORS)} 本（R1〜R{len(RESISTORS)}）、LED 2 本（D1・D2）。穴の奥まで入る長さを残し、板の上に出る金属を短くする。")
    L.append("- LED は切る前に長い足（アノード）を確かめ、上の表の穴に挿してから、もう一度向きを表と照らす。")
    L.append("- 切らない: `PROTO-02` へ移す電源部の部品（`PROT-OC-01` の PTC、`WIRE-PWR-01` の端子台と線、`MEAS-01` の shunt）。どれもブレッドボードに載せない。")
    L.append("- module（ESP32、`ENV-01`、`ACCEL-01`）のピンは切らない。")
    L.append("")
    L.append("## 載せないもの")
    L.append("")
    L.append("- `MEAS-02`（Arduino）の線（A0・GND）と、EXP-030 の Arduino 用の 10 kΩ 分圧（a10–a12、b12–b14）。測定器であり、必要なら実機側が試験のときに足す。")
    L.append("- `MEAS-01` の shunt と GPIO32（`ADC-SHUNT`）の線。shunt はサーボの電流を通すのでブレッドボードに載せない。GPIO32 の線は今の配線に無く、足すかどうかは #17・#405 で決める（J2-7 の列 B1 K18・L18 を空けてある）。")
    L.append("")
    return "\n".join(L) + "\n"


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
        stale = [p.name for p, text in outputs.items() if not p.exists() or p.read_text(encoding="utf-8") != text]
        if stale:
            print("生成物が古い: " + ", ".join(stale), file=sys.stderr)
            return 1
        print("OK: 検査を通り、生成物は最新")
        return 0
    for p, text in outputs.items():
        p.write_text(text, encoding="utf-8")
    print("OK: 検査を通り、生成した")
    return 0


if __name__ == "__main__":
    sys.exit(main())
