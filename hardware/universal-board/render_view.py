#!/usr/bin/env python3
"""`PROTO-02` の「実際の見た目」の図（部品面・はんだ面）を、基板ファイルから作る（#512）。

入力は `hardware/deskcat.kicad_pcb` だけである。部品の位置・向き（footprint の `at`）、足（pad）、
外形（footprint の `F.Fab` の矩形）、はんだ面（`B.Cu`）の導体、基板の穴（`Dwgs.User` の円）、
基板の外形（`Edge.Cuts`）を読み、2 枚の SVG を書く。

- `view-front.svg`：部品面。部品を箱・円盤状の板などで、少し斜めから見た絵にする（高さは右上へずらして表す）。
- `view-back.svg`：はんだ面（左右を裏返した向き）。ランド、芯線（`111641`）、はんだの盛り。

**これは見た目の図で、寸法の正本ではない。**寸法・位置の正本は基板ファイルと
[plan.md](plan.md)、部品の資料は hardware-bom.md・power-budget.md である。
**資料に無い寸法・色は推測で決めず、破線で描き、`TBD` と注記する**（下の `TBD`）。

使い方: `python3 hardware/universal-board/render_view.py`（標準ライブラリだけを使う。ADR-0006）。
`--check` を付けると、図を書かずに、(1) 生成した内容が file と一致するか、(2) file の中の
`data-*` から読んだ部品・足・導体が基板ファイルと一致するかを確かめる。

寸法の出所（plan.md の`寸法の出所`と同じ）：
- 基板 `PROTO-02`（秋月 100517）：外形 72 × 47.5、板厚 1.6、ランドピッチ 2.54、穴径 1、取付穴径 3.2、
  片面（はんだ面だけにランド）、はんだメッキ仕上げ。出所は hardware-bom.md の `PROTO-02` 行。
- 端子台 `114217`（Cixi Wanjie `WJEK254-2.54-02P-140-00A`）：幅 5.54 × 奥行 6.20 × 高さ 8.50、足 0.80 角。
  出所はメーカー図面 `WJEK254-2.54-XXP-1YY-00A.pdf`。外形の幅と奥行は footprint の `F.Fab` と同じ。
- PTC `MF-R135`（Bourns）：幅 A 最大 8.9、高さ B 最大 18.9、厚み E 最大 3.0、足の径 0.51。出所は datasheet `mfr.pdf`。
  外見（黄色い円盤、ラジアルリード、刻印 `R135`）の出所は hardware-bom.md の Revision 89（2026-09-08、現物の照合）。
  円盤の径＝幅、円盤の厚み＝厚み、基板の面から円盤の上端まで＝高さ、として最大値で描く。
- `2SJ334`（東芝）：本体 10 × 15、厚み 4.5、足の幅・厚み 0.75、付け根の長さ最大 5.6、足の長さ 13 以上。
  出所は datasheet `2sj334.pdf`。
- 芯線 `111641`：導体外径 0.75（hardware-bom.md の `WIRE-PWR-01` 行。基板ファイルの導体の幅も 0.75）。

TBD（資料に無いので推測せず、破線または注記にした）：
- 基板の色、PTC 以外の部品の色（見た目のための仮の色。hardware-bom.md・experiment-log.md に現物の色の記録が無い）。
- ランドの外径（資料に無い。pad と同じ 1.8 で描く）。
- 端子台のねじの位置・径、電線の口の高さと幅（plan.md の M4）。
- `2SJ334` の本体の下端の高さ（plan.md の M2。図は付け根の長さの最大値 5.6 から描き、外形を破線にする）、金属板の形。
- `R18` の本体の長さ・直径・高さ（plan.md の M3）。
- はんだの盛りの大きさ（見た目）。
"""

import argparse
import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PCB = HERE.parent / "deskcat.kicad_pcb"
OUT = {"front": HERE / "view-front.svg", "back": HERE / "view-back.svg"}

S = 14.0                 # 1 mm の画素数
MARGIN = (5.0, 13.0, 5.0, 8.0)   # 左・上・右・下（mm）。上は部品の高さの分を空ける
KX, KY = 0.35, 0.55      # 高さ 1 mm が画面で右・上へずれる量（mm）
PITCH = 2.54

# 見た目の色（TBD：資料で確認していない仮の色）
C_BOARD = "#4f8a5b"
C_BOARD_DARK = "#3c6e47"
C_LAND = "#d6d9dd"
C_HOLE = "#1d2a22"
C_METAL = "#b8bcc2"
C_WIRE = "#cfd2d6"
C_PTC = "#e6c53c"   # 黄色（現物の記録：hardware-bom.md の Revision 89。仮の色ではない）


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


# ---------------------------------------------------------------- 描画の道具

PSI = math.radians(24.0)   # 部品面の図：板を左下から見るための回転
THETA = math.radians(52.0)  # 部品面の図：見下ろす角（90 度が真上）
BT = 1.6                    # 板厚（hardware-bom.md の `PROTO-02` 行）


class Cam:
    """部品面の図の正射影。基板の座標（mm）と高さ z（板の下面を 0）→ 画面（画素）。"""

    def __init__(self, edge):
        self.x0, self.y0, self.x1, self.y1 = edge
        self.cx, self.cy = (self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2
        us, vs = [], []
        for x, y in ((self.x0, self.y0), (self.x1, self.y0), (self.x0, self.y1), (self.x1, self.y1)):
            for z in (0.0, BT + 23.0):
                u, v = self.raw(x, y, z)
                us.append(u)
                vs.append(v)
        self.ox = MARGIN[0] - min(us)
        self.oy = MARGIN[1] - min(vs) + 4.0
        self.w = (max(us) - min(us) + MARGIN[0] + MARGIN[2]) * S
        self.h = (max(vs) - min(vs) + MARGIN[1] + MARGIN[3] + 4.0 + 9.0) * S
        self.board_bottom = max(self.p(x, y, 0.0)[1] for x, y in ((self.x0, self.y0), (self.x1, self.y0), (self.x0, self.y1), (self.x1, self.y1)))
        # カメラの向き（板の座標。面の法線との内積が正なら見える）
        self.c = (-math.cos(THETA) * math.sin(PSI), math.cos(THETA) * math.cos(PSI), math.sin(THETA))

    def raw(self, x, y, z):
        X, Y = x - self.cx, y - self.cy
        xp = X * math.cos(PSI) + Y * math.sin(PSI)
        yp = -X * math.sin(PSI) + Y * math.cos(PSI)
        return xp, yp * math.sin(THETA) - z * math.cos(THETA)

    def p(self, x, y, z=BT):
        u, v = self.raw(x, y, z)
        return ((u + self.ox) * S, (v + self.oy) * S)

    def depth(self, x, y, z=0.0):
        X, Y = x - self.cx, y - self.cy
        yp = -X * math.sin(PSI) + Y * math.cos(PSI)
        return yp * math.cos(THETA) + z * math.sin(THETA)

    def matrix(self, z=BT):
        o = self.p(0, 0, z)
        ex = self.p(1, 0, z)
        ey = self.p(0, 1, z)
        return (ex[0] - o[0], ex[1] - o[1], ey[0] - o[0], ey[1] - o[1], o[0], o[1])

    def pts(self, plist):
        return " ".join("%.1f,%.1f" % self.p(*q) for q in plist)

    def visible(self, n):
        return n[0] * self.c[0] + n[1] * self.c[1] + n[2] * self.c[2] > 1e-9


def f1(v):
    return ("%.2f" % v).rstrip("0").rstrip(".")


def face(cam, plist, n, fill, stroke="#000000a0", sw=0.8, dash=None, force=False):
    if not force and not cam.visible(n):
        return ""
    d = f' stroke-dasharray="{dash}"' if dash else ""
    return f'<polygon points="{cam.pts(plist)}" fill="{fill}" stroke="{stroke}" stroke-width="{sw}" stroke-linejoin="round"{d}/>'


def box(cam, x0, y0, x1, y1, z0, z1, top, front, left, dash=None):
    """直方体。見える面（上・y の大きい側・x の小さい側）だけを描く。"""
    stroke = "#000000a0"
    return (face(cam, [(x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1)], (0, 1, 0), front, stroke, 0.8, dash)
            + face(cam, [(x0, y0, z0), (x0, y1, z0), (x0, y1, z1), (x0, y0, z1)], (-1, 0, 0), left, stroke, 0.8, dash)
            + face(cam, [(x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)], (0, 0, 1), top, stroke, 0.8, dash))


def ellipse_pts(cx, cy, z, r, n=20):
    return [(cx + r * math.cos(2 * math.pi * i / n), cy + r * math.sin(2 * math.pi * i / n), z) for i in range(n)]


def text_at(cam, x, y, z, s, size=1.2, fill="#ffffff", halo="#000000"):
    u, v = cam.p(x, y, z)
    return (f'<text x="{u:.1f}" y="{v:.1f}" font-size="{size * S:.1f}" fill="{fill}" text-anchor="middle" '
            f'stroke="{halo}" stroke-width="{0.28 * S:.1f}" paint-order="stroke" stroke-linejoin="round" '
            f'font-family="sans-serif">{s}</text>')


def line3(cam, a, b, stroke, width_mm, dash=None):
    u1, v1 = cam.p(*a)
    u2, v2 = cam.p(*b)
    d = f' stroke-dasharray="{dash}"' if dash else ""
    return (f'<line x1="{u1:.1f}" y1="{v1:.1f}" x2="{u2:.1f}" y2="{v2:.1f}" stroke="{stroke}" '
            f'stroke-width="{width_mm * S:.1f}" stroke-linecap="round"{d}/>')


def stadium_profile(cx, base, width, height, n=14):
    """x-z 面の外形：幅 width、高さ height、頭が半円の板（頭の丸みは TBD）。左下から時計回り。"""
    r = width / 2
    pts = [(cx - r, base), (cx - r, base + height - r)]
    for i in range(1, n):
        a = math.pi * i / n
        pts.append((cx - r * math.cos(a), base + height - r + r * math.sin(a)))
    pts += [(cx + r, base + height - r), (cx + r, base)]
    return pts


# 図の部品の名前（出所：plan.md の「部品と向き」の表の「品」）。側は、引き出し線を上（top）か下（bottom）へ出す
LABELS = {
    "F1": ("F1 PTC（過電流の保護）", "top"),
    "Q1": ("Q1 MOSFET 2SJ334（逆接の保護）", "top"),
    "R18": ("R18 10 kΩ（Gate の抵抗）", "bottom"),
    "J5": ("J5 入力の端子台", "bottom"),
    "J6": ("J6 servo の端子台", "bottom"),
    "J7": ("J7 Pi の端子台", "bottom"),
    "J8": ("J8 logic の端子台", "top"),
}
LABEL_SIZE = 1.05  # mm


def label_width(txt, size_px):
    return sum((1.0 if ord(c) > 0x2E80 else 0.58) * size_px for c in txt)


def place_labels(items, width, y_top0, y_bot0, line_h, mirror_order=False):
    """引き出し線つきの部品名。items は (text, side, ax, ay)（anchor は画面の座標）。
    上の側：左の部品ほど上に置く。下の側：右の部品ほど上に置く。こうすると、引き出し線が他の名前の文字を横切らない。"""
    size = LABEL_SIZE * S
    out = []
    tops = sorted([i for i in items if i[1] == "top"], key=lambda i: i[2])
    bots = sorted([i for i in items if i[1] == "bottom"], key=lambda i: -i[2])
    for group, y0, sign in ((tops, y_top0, 1), (bots, y_bot0, 1)):
        for k, (txt, side, ax, ay) in enumerate(group):
            y = y0 + sign * k * line_h
            w = label_width(txt, size)
            x = min(max(ax - 6, 6), width - w - 8)
            out.append(f'<line x1="{ax:.1f}" y1="{ay:.1f}" x2="{ax:.1f}" y2="{y + (4 if side == "top" else -size * 0.9):.1f}" '
                       f'stroke="#33322e" stroke-width="1" stroke-dasharray="4 3"/>')
            out.append(f'<circle cx="{ax:.1f}" cy="{ay:.1f}" r="2.6" fill="#33322e"/>')
            out.append(f'<text x="{x:.1f}" y="{y:.1f}" font-size="{size:.1f}" fill="#1c1c1c" stroke="#f3f1ea" '
                       f'stroke-width="{0.3 * S:.1f}" paint-order="stroke" stroke-linejoin="round">{txt}</text>')
    return "".join(out)


# ---------------------------------------------------------------- 部品の絵

def tb_mouth(lib):
    """端子台の footprint の名前から、電線の口の向き（基板の座標の単位ベクトル）。"""
    return {"WireLid": (0, 1), "WireFloor": (0, -1), "WireLeft": (-1, 0)}[lib.rsplit("_", 1)[-1]]


def fab_rect(fp):
    """footprint の F.Fab の矩形を、向きを掛けて基板の座標にする（x0, y0, x1, y1）。"""
    a = rot(fp["fab"][0], fp["fab"][1], fp["rot"])
    b = rot(fp["fab"][2], fp["fab"][3], fp["rot"])
    return (fp["x"] + min(a[0], b[0]), fp["y"] + min(a[1], b[1]), fp["x"] + max(a[0], b[0]), fp["y"] + max(a[1], b[1]))


def draw_tb(cam, fp):
    x0, y0, x1, y1 = fab_rect(fp)
    H = 8.5                                    # 高さ（メーカー図面）
    z0, z1 = BT, BT + H
    o = box(cam, x0, y0, x1, y1, z0, z1, "#3f6fb0", "#2f5a96", "#264a7c")
    mx, my = tb_mouth(fp["lib"])
    pads = [p for p in fp["pads"] if p["name"] in ("+", "-")]
    cx = sum(p["x"] for p in pads) / len(pads)
    cy = sum(p["y"] for p in pads) / len(pads)
    zo0, zo1 = z0 + 1.5, z0 + 5.5              # 口の高さ（M4：資料に無い。TBD）
    if (mx, my) == (0, 1):
        hx0, hx1 = max(cx - 3.0, x0 + 0.4), min(cx + 3.0, x1 - 0.4)
        o += face(cam, [(hx0, y1, zo0), (hx1, y1, zo0), (hx1, y1, zo1), (hx0, y1, zo1)], (0, 1, 0), "#10182a", "#000", 0.8, "3 2")
    elif (mx, my) == (-1, 0):
        hy0, hy1 = max(cy - 3.0, y0 + 0.4), min(cy + 3.0, y1 - 0.4)
        o += face(cam, [(x0, hy0, zo0), (x0, hy1, zo0), (x0, hy1, zo1), (x0, hy0, zo1)], (-1, 0, 0), "#10182a", "#000", 0.8, "3 2")
    else:
        # 口が見えない側（奥）を向く：上面に矢印で示す
        o += line3(cam, (cx, y0 + 3.0, z1), (cx, y0 + 0.5, z1), "#ffffff", 0.22, "3 2")
    for p in pads:                              # ねじ（位置・径は TBD。破線）
        o += f'<polygon points="{cam.pts(ellipse_pts(p["x"], p["y"], z1, 1.0))}" fill="#c9ccd0" stroke="#222" stroke-width="0.8" stroke-dasharray="3 2"/>'
        o += line3(cam, (p["x"] - 0.8, p["y"], z1), (p["x"] + 0.8, p["y"], z1), "#333", 0.1)
    return o


def draw_ptc(cam, fp):
    """PTC `MF-R135`：黄色いラジアルリードの円盤（現物の記録は hardware-bom.md の Revision 89。2026-09-08）。
    円盤の径＝幅 A（8.9）、厚み＝E（3.0）、基板の面から円盤の上端まで＝高さ B（18.9）。いずれも Bourns の
    datasheet（`mfr.pdf`）の最大値で、その最大寸法で描く。足の径 0.51 も datasheet。"""
    x0, y0, x1, y1 = fab_rect(fp)
    cx, cy = (x0 + x1) / 2, fp["y"]
    A, B, E = 8.9, 18.9, 3.0
    r = A / 2
    cz = BT + B - r
    yb, yf = cy - E / 2, cy + E / 2
    n = 36
    prof = [(cx + r * math.cos(2 * math.pi * i / n), cz + r * math.sin(2 * math.pi * i / n)) for i in range(n)]
    o = ""
    for p in fp["pads"]:                                   # 足：基板から円盤の縁まで
        zt = cz - math.sqrt(max(r * r - (p["x"] - cx) ** 2, 0.0))
        o += line3(cam, (p["x"], p["y"], BT), (p["x"], p["y"], zt), C_METAL, 0.51)
    for i in range(n):                                     # 円盤の縁（厚み）
        (xa, za), (xb, zb) = prof[i], prof[(i + 1) % n]
        dx, dz = xb - xa, zb - za
        o += face(cam, [(xa, yb, za), (xb, yb, zb), (xb, yf, zb), (xa, yf, za)], (dz, 0, -dx), "#b99a22", "#b99a22", 0.6)
    o += face(cam, [(x, yf, z) for x, z in prof], (0, 1, 0), C_PTC, "#000000a0", 0.8)
    o += text_at(cam, cx, yf, cz, "R135", 1.0, "#5a4608", "#e6c53c")  # 現物の刻印（Revision 89）
    return o


def draw_mosfet(cam, fp):
    X0, Y0, X1, Y1 = fab_rect(fp)               # 本体：厚み 4.5（x）× 長さ 10（y）。回した向きも反映する
    zb = BT + 5.6                              # 付け根の長さの最大値（datasheet）。本体の下端の高さは TBD（M2）
    o = ""
    for p in fp["pads"]:
        o += line3(cam, (p["x"], p["y"], BT), (p["x"], p["y"], zb), C_METAL, 0.75)
    o += box(cam, X0, Y0, X1, Y1, zb, zb + 15.0, "#555a62", "#3c4047", "#2c2f34", "4 3")
    # 刻印面は footprint の `MARK` の側（180 度回した向きでは x の小さい側）。金属板の形・大きさは TBD
    o += face(cam, [(X0, Y0 + 1.0, zb + 9.5), (X0, Y1 - 1.0, zb + 9.5), (X0, Y1 - 1.0, zb + 14.0), (X0, Y0 + 1.0, zb + 14.0)],
              (-1, 0, 0), "#9aa0a8", "#222", 0.8, "3 2")
    o += text_at(cam, X0, (Y0 + Y1) / 2, zb + 6.5, "2SJ334", 0.9, "#e8e8e8")
    return o


def draw_resistor(cam, fp):
    # R18：本体の寸法・高さは TBD（plan.md の M3）。footprint の仮の本体（2.5 × 2.5）を破線で描く
    X0, Y0, X1, Y1 = fab_rect(fp)
    return box(cam, X0, Y0, X1, Y1, BT, BT + 6.0, "#e6d6a8", "#d9c58e", "#bfa96f", "3 2")


def draw_part(cam, fp):
    lib = fp["lib"]
    if lib.startswith("TB_114217"):
        return draw_tb(cam, fp)
    if lib.startswith("PTC_MF-R135"):
        return draw_ptc(cam, fp)
    if lib.startswith("2SJ334"):
        return draw_mosfet(cam, fp)
    if lib.startswith("R_Vertical"):
        return draw_resistor(cam, fp)
    raise SystemExit(f"NG: footprint {lib} の絵が render_view.py に無い（{fp['ref']}）")


def is_mount_hole(fp):
    return fp["lib"].startswith("MountingHole")


# ---------------------------------------------------------------- 図の組み立て

def hole_rc(board, x, y):
    return (round((x - 103.0) / PITCH + 1), round((y - 103.45) / PITCH + 1))


def svg_head(w, h, title, desc):
    return [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0f}" height="{h:.0f}" '
        f'viewBox="0 0 {w:.0f} {h:.0f}" font-family="sans-serif">',
        f"<title>{title}</title><desc>{desc}</desc>",
        '<defs>'
        '<radialGradient id="solder" cx="40%" cy="35%" r="70%"><stop offset="0" stop-color="#f4f5f7"/>'
        '<stop offset="0.6" stop-color="#b9bec6"/><stop offset="1" stop-color="#7c828b"/></radialGradient>'
        '</defs>',
        '<rect width="100%" height="100%" fill="#f3f1ea"/>',
    ]


def front_svg(board):
    cam = Cam(board["edge"])
    x0, y0, x1, y1 = board["edge"]
    o = svg_head(cam.w, cam.h, "PROTO-02 部品面（実際の見た目）",
                 "基板ファイルから生成。見た目の図で、寸法の正本ではない。資料に無い寸法は破線（TBD）。")
    # 板：側面（板厚）と上面
    o.append(face(cam, [(x0, y1, 0), (x1, y1, 0), (x1, y1, BT), (x0, y1, BT)], (0, 1, 0), "#2f5a3a", "#1d3b26", 1.0))
    o.append(face(cam, [(x0, y0, 0), (x0, y1, 0), (x0, y1, BT), (x0, y0, BT)], (-1, 0, 0), "#27482f", "#1d3b26", 1.0))
    o.append(face(cam, [(x0, y0, BT), (x1, y0, BT), (x1, y1, BT), (x0, y1, BT)], (0, 0, 1), C_BOARD, "#244a30", 1.2))
    # 部品面は片面基板の素の面：穴だけ（ランドははんだ面にある）。穴と取付穴は板の面の上の円（平面の変換で楕円になる）
    m = cam.matrix(BT)
    o.append(f'<g transform="matrix({m[0]:.4f} {m[1]:.4f} {m[2]:.4f} {m[3]:.4f} {m[4]:.2f} {m[5]:.2f})">')
    for (hx, hy, r) in board["holes"]:
        o.append(f'<circle cx="{hx:.2f}" cy="{hy:.2f}" r="{r:.2f}" fill="{C_HOLE}"/>')
    for fp in board["fps"]:
        if is_mount_hole(fp):
            for p in fp["pads"]:
                o.append(f'<circle cx="{p["x"]:.2f}" cy="{p["y"]:.2f}" r="{p["size"] / 2:.2f}" fill="#f3f1ea" stroke="#244a30" stroke-width="0.12"/>')
    o.append("</g>")
    parts = [fp for fp in board["fps"] if not is_mount_hole(fp)]
    parts.sort(key=lambda f: (cam.depth(f["x"], f["y"]), f["ref"]))
    for fp in parts:
        pads = ";".join(f'{p["name"]}@{f1(p["x"])},{f1(p["y"])}={p["net"]}' for p in fp["pads"])
        o.append(f'<g data-ref="{fp["ref"]}" data-lib="{fp["lib"]}" data-at="{f1(fp["x"])},{f1(fp["y"])}" '
                 f'data-rot="{f1(fp["rot"])}" data-pads="{pads}">')
        o.append(draw_part(cam, fp))
        o.append("</g>")
    items = []
    for fp in parts:                                    # 部品名：引き出し線の先は、上の側は部品の上端、下の側は手前の板の上
        r = fab_rect(fp)
        cxm = (r[0] + r[2]) / 2
        txt, side = LABELS[fp["ref"]]
        if side == "top":
            ztop = {"PTC": BT + 18.9, "2SJ": BT + 5.6 + 15.0}.get(fp["lib"][:3], BT + 8.5)
            ax, ay = cam.p(cxm, (r[1] + r[3]) / 2 if not fp["lib"].startswith("PTC") else fp["y"], ztop)
        else:
            ax, ay = cam.p(cxm, max(r[3], max(p["y"] for p in fp["pads"])) + 1.2, BT)
        items.append((txt, side, ax, ay))
    o.append(place_labels(items, cam.w, 7.8 * S, cam.board_bottom + 3.2 * S, 1.8 * S))
    o.append(legend(cam.w, "部品面", [("破線", "資料に無い寸法・位置（TBD）：端子台の口とねじ（M4）、2SJ334 の下端の高さ（M2）と金属板、R18 の本体（M3）"),
                                     ("色", "F1 の黄色い円盤は現物の記録（hardware-bom.md の Revision 89）。ほかは見た目のための仮の色（資料で確認していない）"),
                                     ("向き", "板の左を手前に、少し斜めから見下ろす。J8 の口は奥（上面の破線の矢印）")]))
    o.append("</svg>")
    return "\n".join(o) + "\n"


class Flat:
    """はんだ面の図：真上から見る。左右を裏返す（部品面と同じ穴が、鏡の位置に出る）。"""

    def __init__(self, edge):
        self.x0, self.y0, self.x1, self.y1 = edge
        self.w = (self.x1 - self.x0 + MARGIN[0] + MARGIN[2]) * S
        self.h = (self.y1 - self.y0 + 3.0 + MARGIN[3] + 20.0) * S

    def p(self, x, y):
        return ((self.x1 - x + MARGIN[0]) * S, (y - self.y0 + 20.0) * S)


def back_svg(board):
    view = Flat(board["edge"])
    o = svg_head(view.w, view.h, "PROTO-02 はんだ面（実際の見た目。左右を裏返した向き）",
                 "基板ファイルから生成。見た目の図で、寸法の正本ではない。")
    x0, y0, x1, y1 = board["edge"]
    corners = [view.p(x0, y0), view.p(x1, y0), view.p(x1, y1), view.p(x0, y1)]
    o.append('<polygon points="' + " ".join("%.1f,%.1f" % c for c in corners) + f'" fill="{C_BOARD_DARK}" stroke="#1d3b26" stroke-width="1.5"/>')
    for (hx, hy, r) in board["holes"]:
        u, v = view.p(hx, hy)
        # ランド（外径は資料に無い。pad と同じ 1.8 で描く：TBD）
        o.append(f'<circle cx="{u:.1f}" cy="{v:.1f}" r="{0.9 * S:.1f}" fill="{C_LAND}" stroke="#8d9299" stroke-width="0.6"/>')
        o.append(f'<circle cx="{u:.1f}" cy="{v:.1f}" r="{r * S:.1f}" fill="{C_HOLE}"/>')
    for fp in board["fps"]:
        if is_mount_hole(fp):
            for p in fp["pads"]:
                u, v = view.p(p["x"], p["y"])
                o.append(f'<circle cx="{u:.1f}" cy="{v:.1f}" r="{p["size"] / 2 * S:.1f}" fill="#f3f1ea" stroke="#1d3b26" stroke-width="1.5"/>')
    # 芯線（被覆を剥いた区間。導体外径 0.75）
    for t in board["tracks"]:
        a = view.p(t["x1"], t["y1"])
        b = view.p(t["x2"], t["y2"])
        o.append(f'<g data-track="{t["net"]}:{f1(t["x1"])},{f1(t["y1"])}-{f1(t["x2"])},{f1(t["y2"])}" data-layer="{t["layer"]}">')
        o.append(f'<line x1="{a[0]:.1f}" y1="{a[1]:.1f}" x2="{b[0]:.1f}" y2="{b[1]:.1f}" stroke="#00000055" '
                 f'stroke-width="{t["w"] * S + 3:.1f}" stroke-linecap="round" transform="translate(1.5 2)"/>')
        o.append(f'<line x1="{a[0]:.1f}" y1="{a[1]:.1f}" x2="{b[0]:.1f}" y2="{b[1]:.1f}" stroke="{C_WIRE}" '
                 f'stroke-width="{t["w"] * S:.1f}" stroke-linecap="round"/>')
        o.append(f'<line x1="{a[0]:.1f}" y1="{a[1]:.1f}" x2="{b[0]:.1f}" y2="{b[1]:.1f}" stroke="#ffffff" '
                 f'stroke-width="{t["w"] * S * 0.25:.1f}" stroke-linecap="round" stroke-opacity="0.8"/>')
        o.append("</g>")
    # はんだの盛り：導体が通る穴と、部品の足の穴（通る穴ごとにはんだ付けする。plan.md）
    solder = set()
    for t in board["tracks"]:
        n = int(round(max(abs(t["x2"] - t["x1"]), abs(t["y2"] - t["y1"])) / PITCH))
        for i in range(n + 1):
            solder.add(hole_rc(board, t["x1"] + (t["x2"] - t["x1"]) * i / max(n, 1),
                               t["y1"] + (t["y2"] - t["y1"]) * i / max(n, 1)))
    legs = {}
    for fp in board["fps"]:
        for p in fp["pads"]:
            if p["type"] == "thru_hole":
                solder.add(hole_rc(board, p["x"], p["y"]))
                legs[hole_rc(board, p["x"], p["y"])] = f'{fp["ref"]}:{p["name"]}'
    for (c, r) in sorted(solder):
        u, v = view.p(103.0 + (c - 1) * PITCH, 103.45 + (r - 1) * PITCH)
        tag = f' data-leg="{legs[(c, r)]}"' if (c, r) in legs else ""
        o.append(f'<g data-solder="{c},{r}"{tag}><circle cx="{u:.1f}" cy="{v:.1f}" r="{1.15 * S:.1f}" fill="url(#solder)" '
                 f'stroke="#5d636b" stroke-width="0.6"/>')
        if tag:
            o.append(f'<circle cx="{u:.1f}" cy="{v:.1f}" r="{0.3 * S:.1f}" fill="#4a4e54"/>')
        o.append("</g>")
    items = []
    for fp in board["fps"]:
        if is_mount_hole(fp):
            continue
        px = sum(p["x"] for p in fp["pads"]) / len(fp["pads"])
        py = min(p["y"] for p in fp["pads"])
        ax, ay = view.p(px, py)
        items.append((LABELS[fp["ref"]][0], "top", ax, ay))
    o.append(place_labels(items, view.w, 7.2 * S, 0, 1.7 * S))
    o.append(legend(view.w, "はんだ面（左右を裏返した向き）",
                    [("ランド", "外径は資料に無い（TBD）。pad と同じ 1.8 mm で描く"),
                     ("はんだ", "盛りの大きさは見た目（TBD）。足の穴は中央の暗い点で示す。芯線は被覆を剥いた区間（導体外径 0.75）")]))
    o.append("</svg>")
    return "\n".join(o) + "\n"


def legend(width, name, lines):
    out = [f'<g font-size="{0.95 * S:.1f}" fill="#33322e">',
           f'<text x="{0.6 * S:.1f}" y="{1.4 * S:.1f}" font-size="{1.3 * S:.1f}" font-weight="bold">PROTO-02 {name}</text>',
           f'<text x="{0.6 * S:.1f}" y="{2.8 * S:.1f}">見た目の図。寸法の正本は基板ファイルと plan.md。生成: render_view.py</text>']
    for i, (k, v) in enumerate(lines):
        out.append(f'<text x="{0.6 * S:.1f}" y="{(4.1 + 1.25 * i) * S:.1f}">{k}：{v}</text>')
    out.append("</g>")
    return "".join(out)


# ---------------------------------------------------------------- 検査

def read_data(svg_front, svg_back):
    fps = {}
    for m in re.finditer(r'data-ref="([^"]+)" data-lib="([^"]+)" data-at="([^"]+)" data-rot="([^"]+)" data-pads="([^"]*)"', svg_front):
        fps[m.group(1)] = (m.group(2), m.group(3), m.group(4), m.group(5))
    tracks = re.findall(r'data-track="([^"]*)" data-layer="([^"]+)"', svg_back)
    solder = set(re.findall(r'data-solder="(\d+,\d+)"', svg_back))
    return fps, tracks, solder


def check_against_board(board, front, back):
    errs = []
    fps, tracks, solder = read_data(front, back)
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
    gt = sorted(t[0] for t in tracks)
    if et != gt:
        errs.append(f"はんだ面の導体が基板ファイルと違う: 図に無い {sorted(set(et) - set(gt))} / 基板に無い {sorted(set(gt) - set(et))}")
    legs = {hole_rc(board, p["x"], p["y"]) for fp in board["fps"] for p in fp["pads"] if p["type"] == "thru_hole"}
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
        # file の中の data-* が基板ファイルと一致するか（file を直接読む）
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
