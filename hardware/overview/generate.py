#!/usr/bin/env python3
"""最終形の全体の配線図（ブレッドボード B1・B2・B3、PROTO-02、Pi、サーボ、LCD、M-12001）を作る。

つながりの正本は KiCad の回路図 `hardware/deskcat.kicad_sch` である。この図は、回路図から書き出した
netlist（`deskcat-netlist.xml`）を入力にして生成し、手で直さない。図に描く線は次の2つから取る。

- 板どうしの線（B1–B2、B1–B3）と首の14本（B2–LCD）、板の外へ出る線（X1〜X5）は、
  `hardware/breadboard-rebuild/generate.py` のデータを読み込む（ここに写さない）。
- PROTO-02 の端子台から出る線（M-12001、サーボの赤・茶、Pi の PWR IN）は、下の `POWER_WIRES` に書く。
  netlist はどの端子台へ挿すかを持たない（同じ節点の端子が複数ある）ためである。

生成の前に、次を機械で確かめる（崩れると生成せずに止まる）。
- netlist が今の回路図から書き出したものである（回路図の sha256 が `SCHEMATIC_SHA256` と一致する）。
- 図の各線の節点が netlist にあり、線の両端に部品の pin を書いた場合、その pin がその節点にある。
- netlist の各節点について、その節点の pin を持つ部品のまとまり（板・Pi・サーボなど）が、
  図の同じ節点の線だけでつながっている（線の抜け・余分の検出）。決まっていない所（`TBD`）は除く。

使い方: `python3 -I hardware/overview/generate.py`（標準ライブラリだけ。ADR-0006）。
`--check` を付けると、検査し、生成した内容が file と一致するかを確かめる。
netlist の書き出し（回路図を変えたら行う）:
`kicad-cli sch export netlist --format kicadxml -o hardware/overview/deskcat-netlist.xml hardware/deskcat.kicad_sch`
の後、`<source>` を `hardware/deskcat.kicad_sch` に、library の `<uri>` を `${KIPRJMOD}/DeskCat.kicad_sym` に置き換え（書き出した端末の絶対 path を残さない）、`SCHEMATIC_SHA256` を回路図の sha256 に直す。
"""

import argparse
import hashlib
import importlib.util
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCHEMATIC = HERE.parent / "deskcat.kicad_sch"
NETLIST = HERE / "deskcat-netlist.xml"
SCHEMATIC_SHA256 = "83f76795601a79b49cdacfa3bc4d894ef5ac415e1f323e8e71881dafaa3b56da"

# ---------------------------------------------------------------- まとまり（ブロック）

# 回路図の参照番号 → どこに載るか。抵抗の板は breadboard-rebuild/generate.py の RESISTORS と同じ。
BLOCK_OF = {
    "U1": "B1", "D1": "B1", "D2": "B1",
    "R4": "B1", "R5": "B1", "R7": "B1", "R8": "B1", "R9": "B1", "R12": "B1", "R13": "B1",
    "R14": "B1", "R15": "B1", "R16": "B1", "R17": "B1", "R19": "B1",
    "R1": "B2", "R2": "B2", "R3": "B2", "R6": "B2",
    "R10": "B3", "R11": "B3", "J2": "B3", "J3": "B3",
    "J1": "LCD", "J4": "Pi", "J9": "Pi", "M1": "SERVO", "PS1": "M-12001",
    "J5": "PROTO-02", "J6": "PROTO-02", "J7": "PROTO-02", "J8": "PROTO-02",
    "F1": "PROTO-02", "Q1": "PROTO-02", "R18": "PROTO-02",
}

BLOCKS = {
    # 名前: (題, 置き場所（enclosure-and-neck.md の物理的な完成形）, 図の上の矩形 x, y, w, h)
    "LCD": ("LCD（DISP-01）", "頭。背板の前の面", 260, 40, 200, 60),
    "SERVO": ("サーボ（SERVO-01）", "ふたの上の天板。軸は胴の真ん中", 520, 230, 140, 60),
    "PROTO-02": ("電源の基板 PROTO-02", "左の壁の内側に立てる", 60, 170, 130, 300),
    "M-12001": ("AC アダプター M-12001", "左の壁の後ろ寄りの穴から線が入る", -150, 160, 150, 50),
    "Pi": ("Raspberry Pi Zero W", "右の壁の内側の後ろ寄りに立てる", 860, 170, 120, 180),
    "B2": ("B2（BB-601。LCD の14本を受ける）", "床の B1 の後ろ", 260, 380, 230, 80),
    "B1": ("B1（EIC-3901。ESP32）", "床の前寄り", 260, 520, 330, 110),
    "B3": ("B3（BB-601。ENV-01・ACCEL-01）", "前の壁の内側に立てる", 640, 690, 260, 60),
}

# breadboard-rebuild の信号名 → netlist の節点
BB_NET = {
    "3V3": "/3V3", "GND": "/GND", "GPIO22": "/LCD-CS", "GPIO16": "/LCD-RST", "GPIO17": "/LCD-DC",
    "GPIO23": "/LCD-MOSI", "GPIO18": "/LCD-SCLK", "GPIO4": "/LCD-BL", "GPIO19": "/LCD-MISO",
    "GPIO21": "/TOUCH-CS", "GPIO34": "/TOUCH-IRQ", "GPIO25": "/I2C-SDA", "GPIO26": "/I2C-SCL",
    "GPIO36": "/ADC-3V3", "PI_TXD": "/PI-UART-TX-Pi", "PI_RXD": "/PI-UART-RX-Pi", "SERVO_SIG": "/SERVO-SIG",
}
# 板の外へ出る線（X1〜X5）の、板の外の端の部品 pin。X5 の PROTO-02 の側の端子は #405 で決める。
OFFBOARD_END = {"X1": ("Pi", "J4", "8"), "X2": ("Pi", "J4", "10"), "X3": ("Pi", "J4", "39"),
                "X4": ("SERVO", "M1", "yellow"), "X5": ("PROTO-02", None, None)}

# PROTO-02 の端子台から出る線。(id, 節点, 端1 (block, ref, pin), 端2, 線, 色)
POWER_WIRES = [
    ("P1", "/5V-IN", ("M-12001", "PS1", "+"), ("PROTO-02", "J5", "+"), "M-12001 の線（Micro-B の手前で切った導体）", "#d62728"),
    ("P2", "/GND", ("M-12001", "PS1", "-"), ("PROTO-02", "J5", "-"), "M-12001 の線（同上）", "#111111"),
    ("P3", "/5V-NODE", ("PROTO-02", "J6", "+"), ("SERVO", "M1", "red"), "サーボの赤（`111641` で延ばす。plug とのつなぎ方は #405）", "#d62728"),
    ("P4", "/GND", ("PROTO-02", "J6", "-"), ("SERVO", "M1", "brown"), "サーボの茶（同上）", "#8c564b"),
    ("P5", "/5V-NODE", ("PROTO-02", "J7", "+"), ("Pi", "J9", "VBUS"), "`CABLE-PI-PWR-01`（`109314` を切った導体）→ Pi の `PWR IN`", "#d62728"),
    ("P6", "/GND", ("PROTO-02", "J7", "-"), ("Pi", "J9", "GND"), "`CABLE-PI-PWR-01`（同上）", "#111111"),
]

# 決まっていない所。図に破線と `TBD` で描き、線の検査からは除く。
TBD = [
    ("T1", "PROTO-02", "B1", "J8（logic の枝）→ ESP32 の 5 V 入力：使うかと経路は #405（power-budget.md の`ESP32の給電経路`）"),
    ("T2", "B2", "LCD", "LCD の VCC（pin 1）の電圧：今は 3V3。最終は #405（`HW-TBD-024`）"),
    ("T3", "PROTO-02", "SERVO", "servo の枝の bulk capacitor C1 の値と品、shunt（`MEAS-01`）を入れるか：#405・#17"),
    ("T4", "B1", "PROTO-02", "X5 を PROTO-02 のどの端子へつなぐか：#405"),
]

# 線の通り道（まとまりの組 → 途中の点）。ほかの枠の上を横切らないように曲げる。
ROUTES = {
    ("B1", "SERVO"): [(590, 520)],
    ("PROTO-02", "Pi"): [(125, 160), (920, 160)],
    ("B1", "PROTO-02"): [(240, 600)],
}

SERVO_COLOR = {"yellow": "#f2c500", "red": "#d62728", "brown": "#8c564b"}


# ---------------------------------------------------------------- 入力

def load_breadboard():
    path = HERE.parent / "breadboard-rebuild" / "generate.py"
    spec = importlib.util.spec_from_file_location("breadboard_rebuild", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def load_netlist():
    root = ET.parse(NETLIST).getroot()
    pin_net, net_nodes = {}, {}
    for n in root.iter("net"):
        name = n.get("name")
        nodes = [(x.get("ref"), x.get("pin")) for x in n.iter("node")]
        net_nodes[name] = nodes
        for node in nodes:
            pin_net[node] = name
    refs = {c.get("ref") for c in root.iter("comp")}
    return pin_net, net_nodes, refs


def board_of(hole):
    return hole.split(":")[0] if ":" in hole else None


def collect_wires(bb):
    """図に描く線。(id, 節点, (block, ref, pin), (block, ref, pin), 説明, 色)"""
    wires = []
    for wid, a, b, sig, _state in bb.SOFT:
        ba, bb_ = board_of(a), board_of(b)
        if ba in BLOCKS and bb_ in BLOCKS and ba != bb_:
            wires.append((wid, BB_NET[sig], (ba, None, None), (bb_, None, None), f"柔らかい線 {wid}（{a} – {b}）", "#555555"))
    for n, name, hole, sig in bb.NECK:
        wires.append((f"N{n}", BB_NET[sig], (board_of(hole), None, None), ("LCD", "J1", str(n)),
                      f"首の線 LCD pin {n} {name}（{hole}）", "#9467bd"))
    for xid, hole, sig, far, _state in bb.OFFBOARD:
        blk, ref, pin = OFFBOARD_END[xid]
        color = SERVO_COLOR["yellow"] if xid == "X4" else "#555555"
        wires.append((xid, BB_NET[sig], (board_of(hole), None, None), (blk, ref, pin), f"{xid}（{hole} → {far}）", color))
    wires += POWER_WIRES
    return wires


# ---------------------------------------------------------------- 検査

def run_checks(wires, pin_net, net_nodes, refs):
    errors = []
    sha = hashlib.sha256(SCHEMATIC.read_bytes()).hexdigest()
    if sha != SCHEMATIC_SHA256:
        errors.append(f"netlist が今の回路図から書き出したものでない（回路図の sha256 {sha}）。書き出し直して SCHEMATIC_SHA256 を直す")
    for ref in refs:
        if ref not in BLOCK_OF:
            errors.append(f"{ref}: どのまとまりに載るかが BLOCK_OF に無い")
    for wid, net, a, b, *_ in wires:
        if net not in net_nodes:
            errors.append(f"{wid}: 節点 {net} が netlist に無い")
            continue
        for blk, ref, pin in (a, b):
            if blk not in BLOCKS:
                errors.append(f"{wid}: まとまり {blk} が無い")
            if ref is not None:
                if pin_net.get((ref, pin)) != net:
                    errors.append(f"{wid}: {ref} pin {pin} は節点 {pin_net.get((ref, pin))} で、線の節点 {net} と違う")
                if BLOCK_OF.get(ref) != blk:
                    errors.append(f"{wid}: {ref} は {BLOCK_OF.get(ref)} に載り、{blk} ではない")
    for net, nodes in net_nodes.items():
        blocks = {BLOCK_OF.get(ref) for ref, _ in nodes}
        if len(blocks) < 2:
            continue
        adj = {b: set() for b in BLOCKS}
        for _wid, wnet, a, b, *_ in wires:
            if wnet == net:
                adj[a[0]].add(b[0])
                adj[b[0]].add(a[0])
        start = next(iter(blocks))
        seen, stack = {start}, [start]
        while stack:
            u = stack.pop()
            for v in adj[u] - seen:
                seen.add(v)
                stack.append(v)
        missing = blocks - seen
        if missing:
            errors.append(f"節点 {net}: {sorted(blocks)} が図の線でつながらない（{sorted(missing)} に届かない）")
    used = {w[1] for w in wires}
    for net in used:
        if net in net_nodes and len({BLOCK_OF.get(r) for r, _ in net_nodes[net]}) < 2:
            errors.append(f"節点 {net}: 1つのまとまりの中だけの節点に、まとまりの間の線がある")
    return errors


# ---------------------------------------------------------------- 生成

def esc(t):
    return str(t).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def center(blk):
    _t, _w, x, y, w, h = BLOCKS[blk]
    return x + w / 2, y + h / 2


def anchor(blk, toward):
    """矩形の縁の上で、相手の中心へ向かう点。"""
    _t, _w, x, y, w, h = BLOCKS[blk]
    cx, cy = x + w / 2, y + h / 2
    tx, ty = toward
    dx, dy = tx - cx, ty - cy
    if dx == 0 and dy == 0:
        return cx, cy
    sx = (w / 2) / abs(dx) if dx else float("inf")
    sy = (h / 2) / abs(dy) if dy else float("inf")
    s = min(sx, sy)
    return cx + dx * s, cy + dy * s


def bundles(wires):
    out = {}
    for w in wires:
        key = tuple(sorted((w[2][0], w[3][0])))
        out.setdefault(key, []).append(w)
    return out


def render_svg(wires):
    OX, OY, W, H = 200, 20, 1220, 880
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="sans-serif">',
         f'<rect width="{W}" height="{H}" fill="#fff"/>',
         '<text x="16" y="22" font-size="15" font-weight="bold">DeskCat 全体の配線（最終形のつながり。#512）</text>',
         '<text x="16" y="40" font-size="11" fill="#444">位置は胴の中の位置関係（上＝後ろ、下＝前、左＝左の壁。enclosure-and-neck.md の物理的な完成形）だけを表し、寸法は表さない。'
         '線は netlist から生成。電源の基板は未実装の計画（#405）。</text>']

    def P(x, y):
        return x + OX, y + OY + 40

    bx0, by0 = P(40, 150)
    bx1, by1 = P(1000, 770)
    o.append(f'<rect x="{bx0}" y="{by0}" width="{bx1 - bx0}" height="{by1 - by0}" fill="#fafafa" stroke="#999" stroke-dasharray="6 4"/>')
    o.append(f'<text x="{bx0 + 6}" y="{by0 + 14}" font-size="11" fill="#777">胴（内側。上の辺＝後ろの壁、下の辺＝前の壁）</text>')
    def route(a, b, shift=0.0):
        key = (a, b) if (a, b) in ROUTES else (b, a)
        mids = ROUTES.get(key, [])
        if key != (a, b):
            mids = mids[::-1]
        pts = [anchor(a, mids[0] if mids else center(b))] + mids + [anchor(b, mids[-1] if mids else center(a))]
        out = []
        for i, (x, y) in enumerate(pts):
            j = min(i, len(pts) - 2)
            dx, dy = pts[j + 1][0] - pts[j][0], pts[j + 1][1] - pts[j][1]
            L = (dx * dx + dy * dy) ** 0.5 or 1
            out.append(P(x - dy / L * shift, y + dx / L * shift))
        return out

    def poly(pts, color, width, extra=""):
        p = " ".join(f"{x:.1f},{y:.1f}" for x, y in pts)
        o.append(f'<polyline points="{p}" fill="none" stroke="{color}" stroke-width="{width}"{extra}/>')

    def mid(pts):
        k = len(pts) // 2
        (x1, y1), (x2, y2) = pts[k - 1], pts[k]
        return (x1 + x2) / 2, (y1 + y2) / 2

    for (a, b), ws in sorted(bundles(wires).items()):
        if "SERVO" in (a, b) or len(ws) <= 3:
            n = len(ws)
            for i, w in enumerate(ws):
                pts = route(a, b, (i - (n - 1) / 2) * 7)
                poly(pts, w[5], 2.4)
            label = "・".join(w[0] for w in ws)
        else:
            pts = route(a, b)
            poly(pts, ws[0][5], 7, ' opacity="0.55"')
            label = f"{len(ws)}本"
        mx, my = mid(route(a, b))
        o.append(f'<text x="{mx + 6:.1f}" y="{my - 5:.1f}" font-size="10" fill="#222">{esc(label)}</text>')
    for k, (tid, a, b, what) in enumerate(TBD):
        pts = route(a, b, 14 + 9 * k)
        poly(pts, "#c00", 1.5, ' stroke-dasharray="5 4"')
        mx, my = mid(pts)
        o.append(f'<text x="{mx + 6:.1f}" y="{my + 12:.1f}" font-size="10" fill="#c00">{tid}</text>')
    for blk, (title, place, x, y, w, h) in BLOCKS.items():
        X, Y = P(x, y)
        dash = ' stroke-dasharray="5 3"' if blk in ("SERVO", "LCD") else ""
        o.append(f'<rect x="{X}" y="{Y}" width="{w}" height="{h}" rx="6" fill="#eef4ff" stroke="#335"{dash}/>')
        o.append(f'<text x="{X + 6}" y="{Y + 16}" font-size="11" font-weight="bold">{esc(title)}</text>')
        o.append(f'<text x="{X + 6}" y="{Y + 30}" font-size="9" fill="#555">{esc(place)}</text>')
        refs = sorted([r for r, bl in BLOCK_OF.items() if bl == blk], key=lambda r: (r[0], int(r[1:]) if r[1:].isdigit() else 0))
        o.append(f'<text x="{X + 6}" y="{Y + 43}" font-size="9" fill="#555">{esc(" ".join(refs))}</text>')
    ly = 790
    o.append(f'<text x="16" y="{ly}" font-size="10" fill="#444">細い線＝1本ずつ（ID。サーボは黄・赤・茶）。太い線＝束（本数）。各線の節点と端は wiring-table.md。赤の破線＝TBD。'
             '描かない物：#18 の試験だけの物（端子台S、PSU、試験用の負荷、shunt）。</text>')
    for i, (tid, _a, _b, what) in enumerate(TBD):
        o.append(f'<text x="16" y="{ly + 14 * (i + 1)}" font-size="10" fill="#c00">{tid}: {esc(what)}</text>')
    o.append("</svg>")
    return "\n".join(o) + "\n"


def render_md(wires):
    L = ["<!-- generate.py が生成する。手で直さない。 -->",
         "# 全体の配線（最終形のつながり）",
         "",
         "ブレッドボード（B1・B2・B3）、電源の基板 `PROTO-02`、Pi、サーボ、LCD、AC アダプター M-12001 の、**最終形のつながり**である。"
         "つながりの正本は回路図 [`../deskcat.kicad_sch`](../deskcat.kicad_sch) で、この表と [`layout.svg`](layout.svg) は、回路図から書き出した netlist（[`deskcat-netlist.xml`](deskcat-netlist.xml)）から生成する。"
         "位置は胴の中の位置関係（[enclosure-and-neck.md](../../docs/hardware/enclosure-and-neck.md) の`物理的な完成形`）だけで、寸法は表さない。"
         "電源の基板は未実装の計画である（[universal-board/plan.md](../universal-board/plan.md)、#405）。この表は配線・通電の許可ではない。",
         "",
         "**描かない物：**#18 の試験の間だけの物（端子台S、`PSU-BENCH-01`、試験用の負荷、shunt）。それらは [`../bench-18/`](../bench-18/wiring-table.md) にある。",
         "",
         "## まとまりと置き場所",
         "",
         "| まとまり | 何 | 置き場所 | 回路図の参照番号 |",
         "|---|---|---|---|"]
    for blk, (title, place, *_r) in BLOCKS.items():
        refs = sorted([r for r, bl in BLOCK_OF.items() if bl == blk], key=lambda r: (r[0], int(r[1:]) if r[1:].isdigit() else 0))
        L.append(f"| {blk} | {title} | {place} | {'、'.join(refs)} |")
    L += ["", "## 線（まとまりの間）", "", "| ID | 節点 | 端1 | 端2 | 線 |", "|---|---|---|---|---|"]

    def end(e):
        blk, ref, pin = e
        return f"{blk}" + (f"（{ref} pin {pin}）" if ref else "")

    for wid, net, a, b, what, _c in wires:
        L.append(f"| {wid} | `{net.lstrip('/')}` | {end(a)} | {end(b)} | {what} |")
    L += ["", "サーボの3本：黄（信号）は B1 から X4、赤（5 V）と茶（GND）は PROTO-02 の J6 から（P3・P4）。",
          "", "## 決まっていない所（TBD）", "", "| ID | 間 | 何 |", "|---|---|---|"]
    for tid, a, b, what in TBD:
        L.append(f"| {tid} | {a} – {b} | {what} |")
    L.append("")
    return "\n".join(L)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="検査し、生成物が file と一致するか確かめる")
    a = ap.parse_args()
    bb = load_breadboard()
    pin_net, net_nodes, refs = load_netlist()
    wires = collect_wires(bb)
    errors = run_checks(wires, pin_net, net_nodes, refs)
    if errors:
        for e in errors:
            print("NG:", e, file=sys.stderr)
        return 1
    outputs = {HERE / "layout.svg": render_svg(wires), HERE / "wiring-table.md": render_md(wires)}
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
