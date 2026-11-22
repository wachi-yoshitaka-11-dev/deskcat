#!/usr/bin/env python3
"""`PROTO-02` の「実際の見た目」の図（部品面・はんだ面）を、基板ファイルから作る（#512）。

入力は `hardware/deskcat.kicad_pcb` だけである。部品の位置・向き（footprint の `at`）、足（pad）、
外形（footprint の `F.Fab` の矩形）、はんだ面（`B.Cu`）の導体、基板の穴（`Dwgs.User` の円）、
基板の外形（`Edge.Cuts`）を読み、2 枚の SVG を書く。

- `view-front.svg`：部品面。**真上から見る。**板・穴・部品・文字・凡例の描き方、色、線の太さ、文字の大きさは
  ブレッドボードの図（`hardware/breadboard-rebuild/generate.py`）に合わせた（1 ピッチの画素 `P`、
  色 `COLOR_HEX`、`esc`、`text_width` はその file から読み込んで同じものを使う）。はんだ面の芯線
  （`111641`）を、部品面の図の上に色の付いた線で重ねて描く（赤＝＋、黒＝GND、橙＝Gate）。
  端子台の口の向きと、つながる先は、ブレッドボードの図の「板の外へ出る線」と同じ描き方にした。
- `view-back.svg`：はんだ面（真上から。左右を裏返した向き）。ランド、芯線、はんだの盛り。

**これは見た目の図で、寸法の正本ではない。**寸法・位置の正本は基板ファイルと
[plan.md](plan.md)、部品の資料は hardware-bom.md・power-budget.md である。
**資料に無い寸法・色は推測で決めず、破線で描き、`TBD` と注記する**（下の `TBD`）。

使い方: `python3 hardware/universal-board/render_view.py`（標準ライブラリだけを使う。ADR-0006）。
`--check` を付けると、図を書かずに、(1) 生成した内容が file と一致するか、(2) file の中の
`data-*` から読んだ部品・足・導体が基板ファイルと一致するかを確かめる。

寸法の出所（plan.md の`寸法の出所`と同じ）：
- 基板 `PROTO-02`（秋月 100517）：外形 72 × 47.5、板厚 1.6、ランドピッチ 2.54、穴径 1、取付穴径 3.2、
  片面（はんだ面だけにランド）、はんだメッキ仕上げ。出所は hardware-bom.md の `PROTO-02` 行。
- 端子台 `114217`（Cixi Wanjie `WJEK254-2.54-02P-140-00A`）：幅 5.54 × 奥行 6.20、足 0.80 角。
  出所はメーカー図面 `WJEK254-2.54-XXP-1YY-00A.pdf`。外形の幅と奥行は footprint の `F.Fab` と同じ。
- PTC `MF-R135`（Bourns）：幅 A 最大 8.9、厚み E 最大 3.0（真上から見た形は 幅 8.9 × 厚み 3.0）。
  出所は datasheet `mfr.pdf`。外見（黄色い円盤、ラジアルリード、刻印 `R135`）の出所は hardware-bom.md の
  Revision 89（2026-09-08、現物の照合）。
- `2SJ334`（東芝）：本体 厚み 4.5 × 長さ 10（真上から見た形）。出所は datasheet `2sj334.pdf`。
- 芯線 `111641`：導体外径 0.75（hardware-bom.md の `WIRE-PWR-01` 行。基板ファイルの導体の幅も 0.75）。
- 部品の名前の出所は plan.md の「部品と向き」の表の「品」。端子台の行き先は plan.md の「基板から出る線」。
- 凡例の用途と部品の役目の出所：hardware-bom.md の `PROT-OC-01`・`PROT-RP-01`・`WIRE-PWR-01` の行（R18 は
  plan.md の部品表の行。BOM に Gate の抵抗の行が無いことは plan.md の「BOM に無いもの」）、tbd-register.md の `HW-TBD-030`、Issue #405（段階Cの実装）・#525（段階Cの初回通電の前提）の題。

TBD（資料に無いので推測せず、破線または注記にした）：
- 基板の色、PTC 以外の部品の色（hardware-bom.md・experiment-log.md に現物の色の記録が無い）。
- ランドの外径（資料に無い。pad と同じ 1.8 で描く）。
- 端子台のねじの位置・径、電線の口の幅（plan.md の M7。破線。真上からの図に口の高さ M4 は出ない）。
- `R18` の本体の大きさ（plan.md の M3。footprint の仮の 2.5 × 2.5）。
- はんだの盛りの大きさ（見た目）。
"""

import argparse
import importlib.util
import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PCB = HERE.parent / "deskcat.kicad_pcb"
OUT = {"front": HERE / "view-front.svg", "back": HERE / "view-back.svg"}
PITCH = 2.54


def _load_breadboard():
    """ブレッドボードの図の generate.py を、定数と関数を共有するために読み込む（実行はしない）。"""
    path = HERE.parent / "breadboard-rebuild" / "generate.py"
    spec = importlib.util.spec_from_file_location("breadboard_generate", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


BB = _load_breadboard()
P = BB.P                      # 1 ピッチの画素（ブレッドボードの図と同じ）
COLOR_HEX = BB.COLOR_HEX      # 手持ちの色（赤 黒 …）
esc = BB.esc
text_width = BB.text_width
K = P / PITCH                 # 1 mm の画素
FONT = "Noto Sans CJK JP, Hiragino Sans, Meiryo, sans-serif"   # generate.py の build_svg と同じ

# 線の色（ブレッドボードの図の決まり：赤＝3V3／＋、黒＝GND。Gate は手持ちの橙）
NET_COLOR = {
    "/5V-IN": COLOR_HEX["赤"], "/5V-PTC": COLOR_HEX["赤"], "/5V-NODE": COLOR_HEX["赤"],
    "/GND": COLOR_HEX["黒"], "/Q1-GATE": COLOR_HEX["橙"], "/ADC-SHUNT": COLOR_HEX["紫"],
}
NET_SHORT = {"/5V-IN": "5V-IN", "/5V-PTC": "5V-PTC", "/5V-NODE": "5V-NODE", "/GND": "GND", "/Q1-GATE": "Q1-GATE", "/ADC-SHUNT": "ADC-SHUNT"}
C_PTC = "#e6c53c"             # 黄色（現物の記録：hardware-bom.md の Revision 89）
C_BEIGE = "#e9d3a5"           # 抵抗の色（generate.py の draw_parts と同じ）
C_BEIGE_LINE = "#7a6a50"
C_PIN = "#caa53d"             # 足の丸（generate.py の ESP32 のピンと同じ）

# 図の部品の名前（出所：plan.md の「部品と向き」の表の「品」）
LABELS = {
    "F1": "F1 PTC（過電流の保護）",
    "Q1": "Q1 MOSFET 2SJ334（逆接の保護）",
    "R18": "R18 10 kΩ（Gate の抵抗）",
    "J5": "J5 入力の端子台",
    "J6": "J6 servo の端子台",
    "J7": "J7 Pi の端子台",
    "J8": "J8 logic の端子台",
    "R20": "R20 shunt 0.1 Ω 5 W（servo の電流を測る）",
    "C1": "C1 servo の bulk capacitor",
    "C2": "C2 servo の bulk capacitor",
    "W1": "W1 ADC-SHUNT の線の出口",
}
# 端子台の行き先（出所：plan.md の「基板から出る線」）
FAR = {
    "J5": "M-12001（5 V の入力。赤＝＋、黒＝GND）",
    "J6": "servo（赤＝5 V、茶＝GND。ふたの切り込みから。plug とのつなぎ方は #405）",
    "J7": "Pi（PWR IN。CABLE-PI-PWR-01）",
    "J8": "B1（ESP32 の 5V pin へ。J8− は ESP32 の GND の戻り）",
    "W1": "B1 の GPIO32（ADC-SHUNT。線は床の側へ）",
}


# ---------------------------------------------------------------- S 式の読み込み

def parse(text):
    """KiCad の S 式を、入れ子の list にする。文字列は引用符を外した str、数は str のまま返す。"""
    tokens = re.findall(r'\(|\)|"(?:[^"\\]|\\.)*"|[^\s()"]+', text)
    stack, cur = [], []
    for t in tokens:
        if t == "(":
            stack.append(cur)
            cur = []
        elif t == ")":
            done = cur
            cur = stack.pop()
            cur.append(done)
        elif t.startswith('"'):
            cur.append(t[1:-1])
        else:
            cur.append(t)
    return cur[0]


def kids(node, head):
    return [c for c in node if isinstance(c, list) and c and c[0] == head]


def kid(node, head):
    r = kids(node, head)
    return r[0] if r else None


def floats(node, head):
    k = kid(node, head)
    return [float(v) for v in k[1:] if re.fullmatch(r"-?[\d.]+", v)] if k else None


def rot(x, y, deg):
    """KiCad の向き（画面で反時計回りが正。y は下向き）で回す。"""
    r = math.radians(deg)
    return (x * math.cos(r) + y * math.sin(r), -x * math.sin(r) + y * math.cos(r))


def load_board():
    tree = parse(PCB.read_text(encoding="utf-8"))
    board = {"holes": [], "fps": [], "tracks": [], "edge": None}
    for n in kids(tree, "gr_rect"):
        if kid(n, "layer")[1] == "Edge.Cuts":
            s, e = floats(n, "start"), floats(n, "end")
            board["edge"] = (min(s[0], e[0]), min(s[1], e[1]), max(s[0], e[0]), max(s[1], e[1]))
    for n in kids(tree, "gr_circle"):
        if kid(n, "layer")[1] == "Dwgs.User":
            c, e = floats(n, "center"), floats(n, "end")
            board["holes"].append((c[0], c[1], math.hypot(e[0] - c[0], e[1] - c[1])))
    for n in kids(tree, "footprint"):
        at = floats(n, "at")
        deg = at[2] if len(at) > 2 else 0.0
        ref = next(p[2] for p in kids(n, "property") if p[1] == "Reference")
        fab = None
        for r in kids(n, "fp_rect"):
            if kid(r, "layer")[1] == "F.Fab":
                s, e = floats(r, "start"), floats(r, "end")
                fab = (min(s[0], e[0]), min(s[1], e[1]), max(s[0], e[0]), max(s[1], e[1]))
        for r in kids(n, "fp_circle"):
            if kid(r, "layer")[1] == "F.Fab":
                c, e = floats(r, "center"), floats(r, "end")
                rad = math.hypot(e[0] - c[0], e[1] - c[1])
                fab = (c[0] - rad, c[1] - rad, c[0] + rad, c[1] + rad)
        pads = []
        for p in kids(n, "pad"):
            px, py = floats(p, "at")[:2]
            dx, dy = rot(px, py, deg)
            size = floats(p, "size")[0]
            net = kid(p, "net")
            pads.append({"name": p[1], "type": p[2], "x": round(at[0] + dx, 3), "y": round(at[1] + dy, 3),
                         "size": size, "net": net[2] if net and len(net) > 2 else ""})
        board["fps"].append({"ref": ref, "lib": n[1].split(":")[-1], "x": at[0], "y": at[1], "rot": deg,
                             "fab": fab, "pads": pads})
    names = {n[1]: n[2] for n in kids(tree, "net") if len(n) > 2}
    for n in kids(tree, "segment"):
        s, e = floats(n, "start"), floats(n, "end")
        board["tracks"].append({"x1": s[0], "y1": s[1], "x2": e[0], "y2": e[1],
                                "w": floats(n, "width")[0], "layer": kid(n, "layer")[1],
                                "net": names.get(kid(n, "net")[1], "")})
    return board


def f1(v):
    return ("%.2f" % v).rstrip("0").rstrip(".")


def tb_mouth(lib):
    """端子台の footprint の名前から、電線の口の向き（基板の座標の単位ベクトル）。"""
    return {"WireLid": (0, 1), "WireFloor": (0, -1), "WireLeft": (-1, 0)}[lib.rsplit("_", 1)[-1]]


def fab_rect(fp):
    """footprint の F.Fab の矩形を、向きを掛けて基板の座標にする（x0, y0, x1, y1）。"""
    a = rot(fp["fab"][0], fp["fab"][1], fp["rot"])
    b = rot(fp["fab"][2], fp["fab"][3], fp["rot"])
    return (fp["x"] + min(a[0], b[0]), fp["y"] + min(a[1], b[1]), fp["x"] + max(a[0], b[0]), fp["y"] + max(a[1], b[1]))


def is_mount_hole(fp):
    return fp["lib"].startswith("MountingHole")


def hole_rc(x, y):
    return (round((x - 103.0) / PITCH + 1), round((y - 103.45) / PITCH + 1))


# ---------------------------------------------------------------- 図の座標

class Frame:
    """基板の座標（mm）→ 画面（画素）。部品面は x をそのまま、はんだ面は左右を裏返す。"""

    def __init__(self, edge, bx, by, mirror=False):
        self.x0, self.y0, self.x1, self.y1 = edge
        self.bx, self.by, self.mirror = bx, by, mirror
        self.w = (self.x1 - self.x0) * K
        self.h = (self.y1 - self.y0) * K

    def p(self, x, y):
        u = (self.x1 - x) if self.mirror else (x - self.x0)
        return (self.bx + u * K, self.by + (y - self.y0) * K)


def svg_open(w, h, title, sub):
    return [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" font-family="{FONT}" font-size="13">',
        f'<rect width="{w}" height="{h}" fill="#fff"/>',
        f'<text x="30" y="36" font-size="22" font-weight="bold">{esc(title)}</text>',
        f'<text x="30" y="62" fill="#444">{esc(sub)}</text>',
    ]


def draw_board(out, fr, board, back):
    """板：ブレッドボードの図と同じ（角丸の矩形 #f4f4f0、灰の四角の穴、列と行の番号）。"""
    out.append(f'<rect x="{fr.bx:.1f}" y="{fr.by:.1f}" width="{fr.w:.1f}" height="{fr.h:.1f}" rx="10" fill="#f4f4f0" stroke="#888"/>')
    ncols, nrows = 27, 17
    for c in range(1, ncols + 1):
        x, _ = fr.p(103.0 + (c - 1) * PITCH, 103.45)
        out.append(f'<text x="{x:.1f}" y="{fr.by + 14:.1f}" text-anchor="middle" font-size="10" fill="#666">{c}</text>')
    for r in range(1, nrows + 1):
        _, y = fr.p(103.0, 103.45 + (r - 1) * PITCH)
        xl = fr.bx + 13 if not back else fr.bx + fr.w - 13
        out.append(f'<text x="{xl:.1f}" y="{y + 4:.1f}" text-anchor="middle" font-size="11" fill="#666">{r}</text>')
    for (hx, hy, _r) in board["holes"]:
        x, y = fr.p(hx, hy)
        if back:   # はんだ面：ランド（外径は資料に無い。pad と同じ 1.8 で描く：TBD）
            out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{0.9 * K:.1f}" fill="#cfd3d8" stroke="#999" stroke-width="0.8"/>')
            out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{0.5 * K:.1f}" fill="#555"/>')
        else:      # 部品面：片面基板は穴だけ。ブレッドボードの図と同じ灰の四角
            out.append(f'<rect x="{x - 3:.1f}" y="{y - 3:.1f}" width="6" height="6" fill="#bbb"/>')
    for fp in board["fps"]:
        if is_mount_hole(fp):
            for p in fp["pads"]:
                x, y = fr.p(p["x"], p["y"])
                out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{p["size"] / 2 * K:.1f}" fill="#fff" stroke="#888"/>')


def draw_tracks(out, fr, board, tag):
    """はんだ面の芯線を、色の付いた線で描く（ブレッドボードの硬いジャンパーと同じ太さ・形）。"""
    for t in board["tracks"]:
        (x1, y1), (x2, y2) = fr.p(t["x1"], t["y1"]), fr.p(t["x2"], t["y2"])
        color = NET_COLOR.get(t["net"], "#9e9e9e")
        out.append(f'<g data-{tag}="{t["net"]}:{f1(t["x1"])},{f1(t["y1"])}-{f1(t["x2"])},{f1(t["y2"])}" data-layer="{t["layer"]}">'
                   f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{color}" stroke-width="5" '
                   f'stroke-linecap="round" opacity="0.85"/></g>')


def net_labels(out, fr, board, offsets, skip=()):
    """線の名前（net）を、各 net の一番長い線の脇に小さく書く（ブレッドボードの線の ID と同じ 9 の文字）。"""
    best = {}
    for t in board["tracks"]:
        length = abs(t["x2"] - t["x1"]) + abs(t["y2"] - t["y1"])
        if t["net"] not in best or length > best[t["net"]][0]:
            best[t["net"]] = (length, t)
    for net, (_l, t) in sorted(best.items()):
        if net in skip:
            continue
        (x1, y1), (x2, y2) = fr.p(t["x1"], t["y1"]), fr.p(t["x2"], t["y2"])
        dx, dy = offsets.get(net, (0, -8))
        mx, my = (x1 + x2) / 2 + dx, (y1 + y2) / 2 + dy
        out.append(f'<text x="{mx:.1f}" y="{my:.1f}" text-anchor="middle" font-size="9" fill="{NET_COLOR.get(net, "#333")}" '
                   f'stroke="#fff" stroke-width="0.4">{NET_SHORT.get(net, net)}</text>')


# ---------------------------------------------------------------- 部品（真上から）

def draw_tb(out, fr, fp):
    x0, y0, x1, y1 = fab_rect(fp)
    (ux, uy), (vx, vy) = fr.p(x0, y0), fr.p(x1, y1)
    out.append(f'<rect x="{ux:.1f}" y="{uy:.1f}" width="{vx - ux:.1f}" height="{vy - uy:.1f}" rx="3" fill="#dde6f5" stroke="#335" stroke-width="1.2"/>')
    mx, my = tb_mouth(fp["lib"])
    pads = [p for p in fp["pads"] if p["name"] in ("+", "-")]
    cx = sum(p["x"] for p in pads) / len(pads)
    cy = sum(p["y"] for p in pads) / len(pads)
    half, depth = 2.3, 1.3         # 電線の口の幅と奥行（幅は TBD：破線）
    if my:
        edge_y = y1 if my > 0 else y0
        a, b = fr.p(cx - half, edge_y - depth if my > 0 else edge_y), fr.p(cx + half, edge_y if my > 0 else edge_y + depth)
    else:
        a, b = fr.p(x0, cy - half), fr.p(x0 + depth, cy + half)
    rx, ry, rw, rh = min(a[0], b[0]), min(a[1], b[1]), abs(b[0] - a[0]), abs(b[1] - a[1])
    out.append(f'<rect x="{rx:.1f}" y="{ry:.1f}" width="{rw:.1f}" height="{rh:.1f}" fill="#334" opacity="0.8" stroke="#000" stroke-dasharray="3 2"/>')
    for p in pads:                                           # ねじ（位置・径は TBD：破線）
        x, y = fr.p(p["x"], p["y"])
        out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{1.0 * K:.1f}" fill="#c9ccd0" stroke="#222" stroke-dasharray="3 2"/>')


def draw_ptc(out, fr, fp):
    x0, y0, x1, y1 = fab_rect(fp)
    cx, cy = (x0 + x1) / 2, fp["y"]
    (ux, uy), (vx, vy) = fr.p(cx - 4.45, cy - 1.5), fr.p(cx + 4.45, cy + 1.5)    # 幅 8.9 × 厚み 3.0（datasheet の最大値）
    out.append(f'<rect x="{ux:.1f}" y="{uy:.1f}" width="{vx - ux:.1f}" height="{vy - uy:.1f}" rx="{1.5 * K:.1f}" fill="{C_PTC}" stroke="#8a7414" stroke-width="1.2"/>')


def draw_mosfet(out, fr, fp):
    x0, y0, x1, y1 = fab_rect(fp)
    (ux, uy), (vx, vy) = fr.p(x0, y0), fr.p(x1, y1)
    out.append(f'<rect x="{ux:.1f}" y="{uy:.1f}" width="{vx - ux:.1f}" height="{vy - uy:.1f}" rx="2" fill="#555a62" stroke="#222" stroke-width="1.2"/>')
    mark_x = ux if fp["rot"] == 180 else vx      # 刻印面の側（footprint の `MARK`。180 度回した向きでは x の小さい側）
    out.append(f'<line x1="{mark_x:.1f}" y1="{uy + 2:.1f}" x2="{mark_x:.1f}" y2="{vy - 2:.1f}" stroke="#d8d8d8" stroke-width="3"/>')


def draw_resistor(out, fr, fp):
    """R18：立てて挿し、片方の足を本体の横に沿わせて下ろす（足の間隔 2 穴）。足は generate.py の抵抗と同じ茶の線、
    本体は同じベージュ。本体の大きさは TBD（M3）なので、footprint の仮の 2.5 × 2.5 を破線で描く。"""
    x0, y0, x1, y1 = fab_rect(fp)
    p1, p2 = sorted(fp["pads"], key=lambda p: p["name"])
    (ax, ay), (bx, by) = fr.p(p1["x"], p1["y"]), fr.p(p2["x"], p2["y"])
    out.append(f'<line x1="{ax:.1f}" y1="{ay:.1f}" x2="{bx:.1f}" y2="{by:.1f}" stroke="{C_BEIGE_LINE}" stroke-width="3"/>')   # 足 2 は本体の横を通って 2 つ目の穴へ
    (ux, uy), (vx, vy) = fr.p(x0, y0), fr.p(x1, y1)
    out.append(f'<rect x="{ux:.1f}" y="{uy:.1f}" width="{vx - ux:.1f}" height="{vy - uy:.1f}" rx="4" fill="{C_BEIGE}" stroke="{C_BEIGE_LINE}" stroke-dasharray="3 2"/>')


def draw_shunt(out, fr, fp):
    """R20（MEAS-01 の shunt、セメント抵抗 SQP5WJ0R1B）：本体 22 × 10 mm（datasheet）。寝かせる。足の間隔 27.94 mm（11 穴）は慣行の値で暫定、
    本体の高さ H は資料に無い（TBD）。足は本体の端から足の穴まで、茶の線で描く。"""
    x0, y0, x1, y1 = fab_rect(fp)
    p1, p2 = sorted(fp["pads"], key=lambda p: p["name"])
    for p in (p1, p2):
        (ax, ay), (bx, by) = fr.p(p["x"], p["y"]), fr.p(min(max(p["x"], x0), x1), min(max(p["y"], y0), y1))
        out.append(f'<line x1="{ax:.1f}" y1="{ay:.1f}" x2="{bx:.1f}" y2="{by:.1f}" stroke="{C_BEIGE_LINE}" stroke-width="3"/>')
    (ux, uy), (vx, vy) = fr.p(x0, y0), fr.p(x1, y1)
    out.append(f'<rect x="{ux:.1f}" y="{uy:.1f}" width="{vx - ux:.1f}" height="{vy - uy:.1f}" rx="4" fill="#f3efe6" stroke="{C_BEIGE_LINE}" stroke-width="1.2"/>')
    cx, cy = (ux + vx) / 2, (uy + vy) / 2
    if (x1 - x0) >= (y1 - y0):
        out.append(f'<text x="{cx:.1f}" y="{cy + 4:.1f}" text-anchor="middle" font-size="11" fill="#5a4a30">0.1Ω 5W</text>')
    else:
        out.append(f'<text x="{cx:.1f}" y="{cy - 4:.1f}" text-anchor="middle" font-size="10" fill="#5a4a30" transform="rotate(-90 {cx:.1f} {cy - 4:.1f})">0.1Ω 5W</text>')


def draw_cap(out, fr, fp):
    """C1・C2（秋月 g108426、Rubycon 16WXA470MEFC8X9）：径 8 mm（datasheet j_wxa.pdf）。足の間隔は 3.5 を 5.08 へ広げる。色は仮。"""
    x0, y0, x1, y1 = fab_rect(fp)
    cx, cy = fr.p((x0 + x1) / 2, (y0 + y1) / 2)
    out.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{(x1 - x0) / 2 * K:.1f}" fill="#33415c" stroke="#111" stroke-width="1.2"/>')
    neg = next(p for p in fp["pads"] if p["name"] == "-")
    nx, ny = fr.p(neg["x"], neg["y"])
    rr = (x1 - x0) / 2 * K
    dx, dy = nx - cx, ny - cy
    dn = (dx * dx + dy * dy) ** 0.5 or 1.0
    dx, dy = dx / dn, dy / dn
    sx, sy = cx - dy * rr, cy + dx * rr           # 向きを +90°回した点から、− の足の側へふくらむ半円
    out.append(f'<path d="M{sx:.1f} {sy:.1f} A{rr:.1f} {rr:.1f} 0 0 0 {2 * cx - sx:.1f} {2 * cy - sy:.1f} Z" fill="#9aa4b8" opacity="0.7"/>')


def draw_exit(out, fr, fp):
    p = fp["pads"][0]
    x, y = fr.p(p["x"], p["y"])
    out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{1.3 * K:.1f}" fill="#fff" stroke="{COLOR_HEX["紫"]}" stroke-width="2.5"/>')


def draw_part(out, fr, fp):
    lib = fp["lib"]
    if lib.startswith("TB_114217"):
        draw_tb(out, fr, fp)
    elif lib.startswith("PTC_MF-R135"):
        draw_ptc(out, fr, fp)
    elif lib.startswith("2SJ334"):
        draw_mosfet(out, fr, fp)
    elif lib.startswith("R_Vertical"):
        draw_resistor(out, fr, fp)
    elif lib.startswith("SHUNT_"):
        draw_shunt(out, fr, fp)
    elif lib.startswith("C_Radial"):
        draw_cap(out, fr, fp)
    elif lib.startswith("WireExit"):
        draw_exit(out, fr, fp)
    else:
        raise SystemExit(f"NG: footprint {lib} の絵が render_view.py に無い（{fp['ref']}）")


def draw_pins(out, fr, fp):
    """足の丸（ブレッドボードの図の ESP32 のピンと同じ金色）と、足の名前（8.5 の文字）。"""
    for p in fp["pads"]:
        x, y = fr.p(p["x"], p["y"])
        out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="4" fill="{C_PIN}" stroke="#4a3b10" stroke-width="0.8"/>')
        name = {"+": "＋", "-": "−"}.get(p["name"], p["name"])
        if fp["ref"] == "R20":
            name = {"1": "A", "2": "B"}[p["name"]]
        out.append(f'<text x="{x:.1f}" y="{y - 9:.1f}" text-anchor="middle" font-size="8.5" fill="#222" stroke="#fff" stroke-width="0.4">{esc(name)}</text>')


# ---------------------------------------------------------------- 部品面

def lines_fit(lines, width, height):
    for x, y, size, text, _b in lines:
        if x + text_width(text, size) > width - 10:
            raise SystemExit(f"NG: 図の幅に収まらない文字（y={y}）: {text}")
    ys = [y for _x, y, _s, _t, _b in lines]
    if len(ys) != len(set(ys)):
        raise SystemExit("NG: 図の下の文字が同じ高さに2行ある")
    if max(ys) > height - 10:
        raise SystemExit("NG: 図の下の文字が図の高さに収まらない")


FRONT_NOTES = [
    "この基板は、完成形の電源（段階C）で、M-12001 の 5 V を servo・Pi・B1 へ分ける基板。まだはんだ付けしていない（未実装）。",
    "はんだ付けは #405、初回通電は #525。この図は PROTO-02 の実装の計画で、基板ファイル hardware/deskcat.kicad_pcb から render_view.py が生成する。",
    "F1 PTC＝過電流の保護（PROT-OC-01、Bourns MF-R135）。Q1 MOSFET 2SJ334＝電源の逆接の保護（PROT-RP-01。方式の判断は tbd-register.md の HW-TBD-030）。",
    "R18＝Q1 の Gate の抵抗 10 kΩ（plan.md の部品表の R18 の行）。J5〜J8＝端子台 114217（WIRE-PWR-01）：J5＝M-12001 の入力、J6＝servo、J7＝Pi、J8＝logic（B1 の ESP32 の 5V pin へ。案B）。",
    "R20＝servo の戻りだけに入れる shunt（MEAS-01。0.1 Ω 5 W。power-budget.md の GND topology）。W1＝ADC-SHUNT の線の出口。",
    "C1・C2＝servo の bulk capacitor の穴（470 µF／16 V。2 個分。3 個目が要るかは実測で決める）。",
    "寸法・位置の正本は基板ファイルと plan.md。この図は見た目で、寸法の正本ではない。図を電源・安全の実機確認に使わない。",
    "線は、はんだ面の芯線（111641、導体外径 0.75）を部品面の図の上に重ねて描いたもの。色はブレッドボードの図の決まりに合わせる。",
    "赤＝＋（5 V の経路：J5 → F1 → Q1 → 分岐点 → J6・J7・J8・C1・C2）、黒＝GND、橙＝Gate（Q1 の G → R18）、紫＝ADC-SHUNT（shunt の servo の側の node。J6−・C1・C2 の−・W1）。",
    "GND：J7−（Pi）・J8−（ESP32）・R18 は、別々の導体で shunt の B の node（B と J5− を結ぶ短い導体）へ直接つく。",
    "servo の戻りは J6− → A → R20 → B と流れ、B で初めてほかの GND に合流する（check_star_point.py が確かめる）。",
    "金の丸＝足の穴（＋・−・a・b・G・D・S・1・2・A・B は足の名前）。灰の四角＝穴。太い線＝はんだ面の芯線。R18 は縦の2つの穴を使い、茶の線が足。",
    "F1（黄色い円盤）：真上からは幅 8.9 × 厚み 3.0。現物の外見は hardware-bom.md の Revision 89。Q1：太い白線は刻印面の側（後ろ向き）。R20 は横に寝かせた本体 22 × 10 mm。C1・C2 は径 8 mm。",
    "破線＝資料に無い寸法（TBD）：端子台のねじの位置・径と電線の口の幅（plan.md の M7）。慣行の値で暫定：R18 の本体。R20 の足の間隔 11 穴は設計の値で確定（足を曲げる側が選ぶ）。R20 の高さ H は TBD。",
    "色は PTC（黄）以外、見た目のための仮の色（資料で確認していない）。",
    "端子台の口の向きは、口の帯とそこから出る曲線で示す。曲線の行き先が板の外（M-12001・servo・Pi・B1）。ブレッドボードの図の「板の外へ出る線」と同じ描き方。W1 の破線は、床の側へ出る線。",
]


def front_svg(board):
    edge = board["edge"]
    BX, BY = 360.0, 240.0
    fr = Frame(edge, BX, BY)
    bottom = BY + fr.h
    W = 1360
    notes_y0 = bottom + 190
    lines = [(60, notes_y0, 15, "凡例と注意", True)]
    lines += [(60, notes_y0 + 26 + i * 22, 12.5, t, False) for i, t in enumerate(FRONT_NOTES)]
    H = int(notes_y0 + 26 + len(FRONT_NOTES) * 22 + 30)
    lines_fit(lines, W, H)
    o = svg_open(W, H, "PROTO-02 の実装の計画（部品面。真上から見る）",
                 "生成元: hardware/universal-board/render_view.py（基板ファイルから）。描き方はブレッドボードの図（hardware/breadboard-rebuild/generate.py）と同じ。")
    draw_board(o, fr, board, back=False)
    o.append(f'<text x="{BX:.1f}" y="{BY - 12:.1f}" font-size="17" font-weight="bold">PROTO-02 部品面</text>')
    parts = [fp for fp in board["fps"] if not is_mount_hole(fp)]
    # 板の外へ出る線（端子台の口 → つながる先）：generate.py の draw_soft と同じ。曲線の先は、板の外の破線の枠の縁で終わる
    box_top_down = {"J6": bottom + 50, "J7": bottom + 100}      # 下へ出る線の枠の上辺
    for fp in parts:
        if not fp["lib"].startswith("TB_114217"):
            continue
        pads = {p["name"]: p for p in fp["pads"]}
        mx, my = tb_mouth(fp["lib"])
        ref = fp["ref"]
        title = "板の外（" + FAR[ref].split("（")[0] + "）"
        sxs = {n: fr.p(p["x"], p["y"]) for n, p in pads.items()}
        if my:
            txt = f'{LABELS[ref]} → {FAR[ref]}'
            w = max(text_width(txt, 9), text_width(title, 10)) + 16
            bx0 = min(x for x, _y in sxs.values()) - 12
            if my > 0:
                top = box_top_down[ref]
                edge = top
            else:
                top = BY - 76
                edge = top + 36
            o.append(f'<rect x="{bx0:.1f}" y="{top:.1f}" width="{w + 12:.1f}" height="36" rx="6" fill="#f4f4f4" stroke="#555" stroke-dasharray="5 3" opacity="0.5"/>')
            o.append(f'<text x="{bx0 + 8:.1f}" y="{top + 14:.1f}" font-size="10" font-weight="bold" fill="#333">{esc(title)}</text>')
            o.append(f'<text x="{bx0 + 8:.1f}" y="{top + 28:.1f}" font-size="9" fill="#333">{esc(txt)}</text>')
            for name, color in (("+", COLOR_HEX["赤"]), ("-", COLOR_HEX["黒"])):
                sx, sy = sxs[name]
                o.append(f'<path d="M{sx:.1f} {sy:.1f} Q{sx:.1f} {(sy + edge) / 2:.1f} {sx:.1f} {edge:.1f}" stroke="{color}" stroke-width="3" fill="none"/>')
        else:
            t1, t2 = LABELS[ref], "→ " + FAR[ref]
            w = max(text_width(t1, 9), text_width(t2, 9), text_width(title, 10)) + 16
            right = BX - 74
            ys = [sxs["+"][1], sxs["-"][1]]
            top = min(ys) - 40
            hgt = max(ys) - min(ys) + 64          # 2 本の曲線の先が、枠の右の縁の上に載る高さ
            o.append(f'<rect x="{right - w:.1f}" y="{top:.1f}" width="{w:.1f}" height="{hgt:.1f}" rx="6" fill="#f4f4f4" stroke="#555" stroke-dasharray="5 3" opacity="0.5"/>')
            o.append(f'<text x="{right - w + 8:.1f}" y="{top + 16:.1f}" font-size="10" font-weight="bold" fill="#333">{esc(title)}</text>')
            o.append(f'<text x="{right - 8:.1f}" y="{top + 36:.1f}" text-anchor="end" font-size="9" fill="#333">{esc(t1)}</text>')
            o.append(f'<text x="{right - 8:.1f}" y="{top + 50:.1f}" text-anchor="end" font-size="9" fill="#333">{esc(t2)}</text>')
            for name, color in (("+", COLOR_HEX["赤"]), ("-", COLOR_HEX["黒"])):
                sx, sy2 = sxs[name]
                o.append(f'<path d="M{sx:.1f} {sy2:.1f} Q{(sx + right) / 2:.1f} {sy2 + 14:.1f} {right:.1f} {sy2:.1f}" stroke="{color}" stroke-width="3" fill="none"/>')
    for fp in sorted(parts, key=lambda f: (f["y"], f["x"], f["ref"])):
        pads = ";".join(f'{p["name"]}@{f1(p["x"])},{f1(p["y"])}={p["net"]}' for p in fp["pads"])
        o.append(f'<g data-ref="{fp["ref"]}" data-lib="{fp["lib"]}" data-at="{f1(fp["x"])},{f1(fp["y"])}" '
                 f'data-rot="{f1(fp["rot"])}" data-pads="{pads}">')
        draw_part(o, fr, fp)
        o.append("</g>")
    draw_tracks(o, fr, board, "ftrack")
    net_labels(o, fr, board, {"/5V-NODE": (0, -8), "/GND": (0, 17), "/Q1-GATE": (0, -8)}, skip=("/5V-IN", "/5V-PTC"))
    for fp in parts:
        draw_pins(o, fr, fp)
    # ADC-SHUNT の線の出口 W1 → 板の外（B1 の GPIO32）：床の側（図の上）へ出る破線の線。generate.py の「板の外へ出る線」と同じ描き方
    w1 = next(f for f in parts if f["ref"] == "W1")
    wx, wy = fr.p(w1["pads"][0]["x"], w1["pads"][0]["y"])
    ttl = "板の外（B1 の GPIO32）"
    wtxt = f'{LABELS["W1"]} → {FAR["W1"]}'
    ww = max(text_width(wtxt, 9), text_width(ttl, 10)) + 16
    wtop = BY - 134
    wbx = wx + 44
    o.append(f'<rect x="{wbx:.1f}" y="{wtop:.1f}" width="{ww + 12:.1f}" height="36" rx="6" fill="#f4f4f4" stroke="#555" stroke-dasharray="5 3" opacity="0.5"/>')
    o.append(f'<text x="{wbx + 8:.1f}" y="{wtop + 14:.1f}" font-size="10" font-weight="bold" fill="#333">{esc(ttl)}</text>')
    o.append(f'<text x="{wbx + 8:.1f}" y="{wtop + 28:.1f}" font-size="9" fill="#333">{esc(wtxt)}</text>')
    o.append(f'<path d="M{wx:.1f} {wy:.1f} C{wx + 70:.1f} {wy:.1f} {wbx + 20:.1f} {(wy + wtop) / 2:.1f} {wbx + 20:.1f} {wtop + 36:.1f}" stroke="{COLOR_HEX["紫"]}" stroke-width="3" fill="none" stroke-dasharray="7 4"/>')
    # 部品の名前：板の下の空き（列 2〜12、行 3〜8）に左揃えで積み、各部品へ引き出し線を引く
    def anchor_of(ref, where):
        f = next(q for q in parts if q["ref"] == ref)
        r = fab_rect(f)
        return {"top": fr.p((r[0] + r[2]) / 2, r[1]), "left": fr.p(r[0], (r[1] + r[3]) / 2), "right": fr.p(r[2], (r[1] + r[3]) / 2), "bottom": fr.p((r[0] + r[2]) / 2, r[3]),
                "pad": fr.p(f["pads"][0]["x"], f["pads"][0]["y"])}[where]
    callouts = [
        ("F1 PTC（過電流の保護）", [anchor_of("F1", "top")]),
        ("Q1 MOSFET 2SJ334（逆接の保護）", [anchor_of("Q1", "top")]),
        ("R20 shunt 0.1 Ω 5 W（servo の電流を測る）", [anchor_of("R20", "top")]),
        ("R18 10 kΩ（Gate の抵抗）", [anchor_of("R18", "right")]),
        ("C1・C2 servo の bulk capacitor（470 µF。2 個分の穴）", [anchor_of(c, "right") for c in ("C1", "C2")]),
        ("W1 ADC-SHUNT の線の出口（B1 の GPIO32 へ）", [anchor_of("W1", "pad")]),
    ]
    for k, (txt, anchors) in enumerate(callouts):
        lx, ly = fr.p(103.0 + (22.6 - 1) * PITCH, 103.45 + (3.2 + k * 1.15 - 1) * PITCH)
        w = text_width(txt, 10)
        o.append(f'<text x="{lx:.1f}" y="{ly:.1f}" font-size="10" fill="#222" stroke="#f4f4f0" stroke-width="3" paint-order="stroke">{esc(txt)}</text>')
        for ax, ay in anchors:
            o.append(f'<line x1="{lx - 4:.1f}" y1="{ly - 3:.1f}" x2="{ax:.1f}" y2="{ay:.1f}" stroke="#666" stroke-width="1" stroke-dasharray="4 3"/>')
            o.append(f'<circle cx="{ax:.1f}" cy="{ay:.1f}" r="2.5" fill="#666"/>')
    for x, y, size, text, bold in lines:
        weight = ' font-weight="bold"' if bold else ""
        o.append(f'<text x="{x}" y="{y}" font-size="{size}"{weight}>{esc(text)}</text>')
    o.append("</svg>")
    return "\n".join(o) + "\n"


# ---------------------------------------------------------------- はんだ面

BACK_NOTES = [
    "この基板は、完成形の電源（段階C）で、M-12001 の 5 V を servo・Pi・B1 へ分ける基板（未実装）。",
    "はんだ付けは #405、初回通電は #525。まだはんだ付けしていない。部品の役目は部品面の図の凡例。",
    "はんだ面（左右を裏返した向き。部品面の図と同じ穴が、鏡の位置に出る）。基板ファイルの B.Cu の導体と、足の穴を描く。",
    "線の色は部品面の図と同じ：赤＝＋、黒＝GND、橙＝Gate、紫＝ADC-SHUNT。太い線＝芯線（111641、被覆を剥いた区間。導体外径 0.75）。",
    "銀の丸＝はんだの盛り（導体が通る穴と足の穴。通る穴ごとにはんだ付けする）。灰の円＝ランド、その中の黒い点＝穴（使わない穴も同じ）。",
    "銀の丸の中の小さい濃い点＝部品の足が入る穴。",
    "TBD（資料に無い）：ランドの外径（pad と同じ 1.8 mm で描く）、はんだの盛りの大きさ（見た目）、基板の色。",
]


def back_svg(board):
    edge = board["edge"]
    BX, BY = 60.0, 340.0
    fr = Frame(edge, BX, BY, mirror=True)
    bottom = BY + fr.h
    W = 900
    lines = [(60, bottom + 50, 15, "凡例と注意", True)]
    lines += [(60, bottom + 76 + i * 22, 12.5, t, False) for i, t in enumerate(BACK_NOTES)]
    H = int(bottom + 76 + len(BACK_NOTES) * 22 + 30)
    lines_fit(lines, W, H)
    o = svg_open(W, H, "PROTO-02 の実装の計画（はんだ面。真上から、左右を裏返した向き）",
                 "生成元: hardware/universal-board/render_view.py（基板ファイルから）。描き方はブレッドボードの図と同じ。")
    draw_board(o, fr, board, back=True)
    o.append(f'<text x="{BX:.1f}" y="{bottom + 22:.1f}" font-size="17" font-weight="bold">PROTO-02 はんだ面</text>')
    solder = set()
    for t in board["tracks"]:
        n = int(round(max(abs(t["x2"] - t["x1"]), abs(t["y2"] - t["y1"])) / PITCH))
        for i in range(n + 1):
            solder.add(hole_rc(t["x1"] + (t["x2"] - t["x1"]) * i / max(n, 1), t["y1"] + (t["y2"] - t["y1"]) * i / max(n, 1)))
    legs = {}
    for fp in board["fps"]:
        for p in fp["pads"]:
            if p["type"] == "thru_hole":
                solder.add(hole_rc(p["x"], p["y"]))
                legs[hole_rc(p["x"], p["y"])] = f'{fp["ref"]}:{p["name"]}'
    for (c, r) in sorted(solder):
        x, y = fr.p(103.0 + (c - 1) * PITCH, 103.45 + (r - 1) * PITCH)
        tag = f' data-leg="{legs[(c, r)]}"' if (c, r) in legs else ""
        o.append(f'<g data-solder="{c},{r}"{tag}><circle cx="{x:.1f}" cy="{y:.1f}" r="{1.05 * K:.1f}" fill="#e3e6ea" '
                 f'stroke="#6b7078" stroke-width="1"/>')
        if tag:
            o.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="2.6" fill="#555"/>')
        o.append("</g>")
    draw_tracks(o, fr, board, "track")      # 芯線ははんだの盛りの上に通る
    # 部品名：板の上の余白（板の上辺より上）に、引き出し線つきで置く。左の部品ほど上
    items = []
    for fp in board["fps"]:
        if is_mount_hole(fp):
            continue
        px = sum(p["x"] for p in fp["pads"]) / len(fp["pads"])
        py = min(p["y"] for p in fp["pads"])
        ax, ay = fr.p(px, py)
        items.append((LABELS[fp["ref"]], ax, ay))
    items.sort(key=lambda i: i[1])
    for k, (txt, ax, ay) in enumerate(items):
        ly = 96 + k * 21
        w = text_width(txt, 11)
        x = min(max(ax - 6, 6), W - w - 8)
        o.append(f'<line x1="{ax:.1f}" y1="{ay:.1f}" x2="{ax:.1f}" y2="{ly + 4:.1f}" stroke="#666" stroke-width="1" stroke-dasharray="4 3"/>')
        o.append(f'<circle cx="{ax:.1f}" cy="{ay:.1f}" r="2.5" fill="#666"/>')
        o.append(f'<text x="{x:.1f}" y="{ly:.1f}" font-size="11" fill="#222">{esc(txt)}</text>')
    for x, y, size, text, bold in lines:
        weight = ' font-weight="bold"' if bold else ""
        o.append(f'<text x="{x}" y="{y}" font-size="{size}"{weight}>{esc(text)}</text>')
    o.append("</svg>")
    return "\n".join(o) + "\n"


# ---------------------------------------------------------------- 検査

def read_data(svg_front, svg_back):
    fps = {}
    for m in re.finditer(r'data-ref="([^"]+)" data-lib="([^"]+)" data-at="([^"]+)" data-rot="([^"]+)" data-pads="([^"]*)"', svg_front):
        fps[m.group(1)] = (m.group(2), m.group(3), m.group(4), m.group(5))
    ftracks = re.findall(r'data-ftrack="([^"]*)" data-layer="([^"]+)"', svg_front)
    tracks = re.findall(r'data-track="([^"]*)" data-layer="([^"]+)"', svg_back)
    solder = set(re.findall(r'data-solder="(\d+,\d+)"', svg_back))
    return fps, ftracks, tracks, solder


def check_against_board(board, front, back):
    errs = []
    fps, ftracks, tracks, solder = read_data(front, back)
    exp = {}
    for fp in board["fps"]:
        if is_mount_hole(fp):
            continue
        pads = ";".join(f'{p["name"]}@{f1(p["x"])},{f1(p["y"])}={p["net"]}' for p in fp["pads"])
        exp[fp["ref"]] = (fp["lib"], f'{f1(fp["x"])},{f1(fp["y"])}', f1(fp["rot"]), pads)
    for ref in sorted(set(exp) | set(fps)):
        if ref not in fps:
            errs.append(f"部品 {ref} が部品面の図に無い")
        elif ref not in exp:
            errs.append(f"部品面の図の {ref} が基板ファイルに無い")
        elif fps[ref] != exp[ref]:
            errs.append(f"{ref} の位置・向き・足が基板ファイルと違う: 図 {fps[ref]} / 基板 {exp[ref]}")
    et = sorted(f'{t["net"]}:{f1(t["x1"])},{f1(t["y1"])}-{f1(t["x2"])},{f1(t["y2"])}' for t in board["tracks"])
    for name, got in (("部品面", sorted(t[0] for t in ftracks)), ("はんだ面", sorted(t[0] for t in tracks))):
        if et != got:
            errs.append(f"{name}の図の導体が基板ファイルと違う: 図に無い {sorted(set(et) - set(got))} / 基板に無い {sorted(set(got) - set(et))}")
    legs = {hole_rc(p["x"], p["y"]) for fp in board["fps"] for p in fp["pads"] if p["type"] == "thru_hole"}
    sold = {tuple(map(int, s.split(","))) for s in solder}
    if not legs <= sold:
        errs.append(f"足の穴にはんだの盛りが無い: {sorted(legs - sold)}")
    return errs


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="図を書かずに、生成物が最新か・基板ファイルと合っているかを確かめる")
    args = ap.parse_args()
    board = load_board()
    if board["edge"] is None or not board["holes"]:
        print("NG: 基板の外形または穴が読めない", file=sys.stderr)
        return 1
    front, back = front_svg(board), back_svg(board)
    errs = check_against_board(board, front, back)
    if args.check:
        for name, content in (("front", front), ("back", back)):
            if not OUT[name].exists() or OUT[name].read_text(encoding="utf-8") != content:
                errs.append(f"{OUT[name].name} が最新でない（基板ファイルまたはこの script から生成し直す）")
        if OUT["front"].exists() and OUT["back"].exists():
            errs += ["file: " + e for e in check_against_board(
                board, OUT["front"].read_text(encoding="utf-8"), OUT["back"].read_text(encoding="utf-8"))]
        for e in errs:
            print("NG:", e, file=sys.stderr)
        if errs:
            return 1
        print("OK: 検査を通り、生成物は最新")
        return 0
    if errs:
        for e in errs:
            print("NG:", e, file=sys.stderr)
        return 1
    for name, content in (("front", front), ("back", back)):
        OUT[name].write_text(content, encoding="utf-8")
    print("OK: view-front.svg、view-back.svg を書いた")
    return 0


if __name__ == "__main__":
    sys.exit(main())
