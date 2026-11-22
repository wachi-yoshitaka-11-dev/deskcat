#!/usr/bin/env python3
"""`PROTO-02` の基板ファイルの GND の star point の規則を確かめる。

規則（docs/hardware/power-budget.md の`GND topology`と、`実装方法を決定した（2026-09-03）`の
「Gate は共通 GND 戻り（star point）へ抵抗を介してつなぐ」）：

- servo の戻りの電流は、J6− から J5− までの GND の導体を流れる（servo の経路）。
- star point は、servo の経路の上の (5,16) の穴である（R18 の足2 の穴そのもの。J6− の足、servo の経路、Pi・logic の母線が集まる点）。
- Gate 抵抗 R18 の GND 側と、Pi（J7−）・logic（J8−）の GND は、J5− へ向かう途中で
  servo の経路に最初に触れる点が star point でなければならない（servo の電流が流れる区間の
  途中に付けない）。star point を除いても servo の経路とつながる導体（迂回）があってはならない。
- R18 の足2 は、star point へ導体で直接つなぐ（途中の節点を通らず、Pi・logic の戻りの導体を共有しない）。
  Gate は star point の電位を基準にするため、Pi・logic の戻りの電流が流れる区間に付けない。
- 導体の端と折れ曲がり、通る穴は、基板に実在する穴（`Dwgs.User` の穴の図）の上にある。
  ユニバーサル基板の四隅の穴は無い（例：(2,17)）。

基板ファイル（`hardware/deskcat.kicad_pcb`）の `B.Cu` の GND の配線と、部品の pad の位置だけを読む。
KiCad は使わず、標準ライブラリだけで動く（ADR-0006）。

使い方: `python3 -I hardware/universal-board/check_star_point.py`。規則を満たせば `OK` を出して 0 で終わる。
"""

import re
import sys
from collections import deque
from pathlib import Path

PCB = Path(__file__).resolve().parent.parent / "deskcat.kicad_pcb"
ORIGIN = (100.0, 100.0)          # 基板の左上の角（基板ファイルの座標）
X0, Y0, PITCH = 3.0, 3.45, 2.54  # 穴の格子（plan.md の`寸法の出所`）
STAR_HOLE = (5, 16)
SERVO = ("J6", "-")
RETURN = ("J5", "-")
MUST_JOIN_AT_STAR = [("R18", "2"), ("J7", "-"), ("J8", "-")]


def hole_mm(x, y):
    return (round(ORIGIN[0] + X0 + (x - 1) * PITCH, 3), round(ORIGIN[1] + Y0 + (y - 1) * PITCH, 3))


def blocks(text, head):
    """`(head ...` で始まる括弧の塊を順に返す。"""
    i = 0
    while True:
        j = text.find("(" + head, i)
        if j < 0:
            return
        depth, k = 0, j
        while True:
            c = text[k]
            if c == '"':
                k = text.index('"', k + 1)
            elif c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0:
                    break
            k += 1
        yield text[j:k + 1]
        i = k + 1


def main():
    text = PCB.read_text(encoding="utf-8")
    nets = dict((n, name) for n, name in re.findall(r'^\t\(net (\d+) "([^"]*)"\)', text, re.M))
    gnd = [n for n, name in nets.items() if name == "/GND"]
    if len(gnd) != 1:
        print("NG: GND の net が1つでない", file=sys.stderr)
        return 1
    pads = {}
    for fp in blocks(text, "footprint "):
        ref = re.search(r'\(property "Reference" "([^"]+)"', fp).group(1)
        at = re.search(r'\n\t\t\(at ([-\d.]+) ([-\d.]+)( [-\d.]+)?\)', fp)
        rot = float(at.group(3)) if at.group(3) else 0.0
        if rot not in (0.0, 180.0):
            print(f"NG: {ref} は 0 度と 180 度以外に回転しており、この検査は読めない", file=sys.stderr)
            return 1
        sign = -1 if rot == 180.0 else 1  # 180 度回すと、足の相対位置の符号が逆になる
        fx, fy = float(at.group(1)), float(at.group(2))
        for pad in blocks(fp, "pad "):
            num = re.match(r'\(pad "([^"]*)"', pad).group(1)
            px, py = map(float, re.search(r'\(at ([-\d.]+) ([-\d.]+)', pad).groups())
            pads[(ref, num)] = (round(fx + sign * px, 3), round(fy + sign * py, 3))
    segs = []
    for seg in blocks(text, "segment"):
        if f"(net {gnd[0]})" not in seg or '"B.Cu"' not in seg:
            continue
        sx, sy = map(float, re.search(r"\(start ([-\d.]+) ([-\d.]+)\)", seg).groups())
        ex, ey = map(float, re.search(r"\(end ([-\d.]+) ([-\d.]+)\)", seg).groups())
        segs.append(((round(sx, 3), round(sy, 3)), (round(ex, 3), round(ey, 3))))
    star = hole_mm(*STAR_HOLE)
    # 基板の穴（Dwgs.User の円。取付穴は footprint なのでここには入らない）
    holes = set()
    for c in blocks(text, "gr_circle"):
        if '"Dwgs.User"' not in c:
            continue
        cx, cy = map(float, re.search(r"\(center ([-\d.]+) ([-\d.]+)\)", c).groups())
        holes.add((round(cx, 2), round(cy, 2)))
    def is_hole(p):
        return (round(p[0], 2), round(p[1], 2)) in holes
    all_segs = []
    for seg in blocks(text, "segment"):
        sx, sy = map(float, re.search(r"\(start ([-\d.]+) ([-\d.]+)\)", seg).groups())
        ex, ey = map(float, re.search(r"\(end ([-\d.]+) ([-\d.]+)\)", seg).groups())
        all_segs.append(((sx, sy), (ex, ey)))
    off = set()
    for (sx, sy), (ex, ey) in all_segs:
        n = max(round(abs(ex - sx) / PITCH), round(abs(ey - sy) / PITCH))
        for i in range(n + 1):
            q = (sx + (ex - sx) * i / max(n, 1), sy + (ey - sy) * i / max(n, 1))
            if not is_hole(q):
                off.add((round((q[0] - ORIGIN[0] - X0) / PITCH + 1), round((q[1] - ORIGIN[1] - Y0) / PITCH + 1)))
    if not holes:
        print("NG: 基板の穴（Dwgs.User の円）が読めない", file=sys.stderr)
        return 1
    # 2本の芯線が互いの内側で交差する点（どちらの端点でもない穴）も節点にする。同じ穴を通る芯線は導通する
    cross = set()
    for i, (a1, a2) in enumerate(segs):
        for b1, b2 in segs[i + 1:]:
            d = (a2[0] - a1[0]) * (b2[1] - b1[1]) - (a2[1] - a1[1]) * (b2[0] - b1[0])
            if abs(d) < 1e-9:
                continue
            u = ((b1[0] - a1[0]) * (b2[1] - b1[1]) - (b1[1] - a1[1]) * (b2[0] - b1[0])) / d
            v = ((b1[0] - a1[0]) * (a2[1] - a1[1]) - (b1[1] - a1[1]) * (a2[0] - a1[0])) / d
            if -1e-9 <= u <= 1 + 1e-9 and -1e-9 <= v <= 1 + 1e-9:
                cross.add((round(a1[0] + u * (a2[0] - a1[0]), 3), round(a1[1] + u * (a2[1] - a1[1]), 3)))
    points = {p for s in segs for p in s} | set(pads.values()) | {star} | cross

    def on(p, a, b):
        cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        return abs(cross) < 1e-6 and min(a[0], b[0]) - 1e-6 <= p[0] <= max(a[0], b[0]) + 1e-6 \
            and min(a[1], b[1]) - 1e-6 <= p[1] <= max(a[1], b[1]) + 1e-6

    graph = {p: set() for p in points}
    for a, b in segs:
        pts = sorted((p for p in points if on(p, a, b)), key=lambda p: (p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2)
        for u, v in zip(pts, pts[1:]):
            graph[u].add(v)
            graph[v].add(u)

    def path(src, dst):
        prev, q = {src: None}, deque([src])
        while q:
            u = q.popleft()
            if u == dst:
                out = []
                while u is not None:
                    out.append(u)
                    u = prev[u]
                return out[::-1]
            for v in graph[u]:
                if v not in prev:
                    prev[v] = u
                    q.append(v)
        return None

    errors = []
    servo = path(pads[SERVO], pads[RETURN])
    if servo is None:
        errors.append("J6− から J5− まで GND の導体がつながっていない")
    elif star not in servo:
        errors.append(f"star point {STAR_HOLE} が servo の経路の上に無い")
    else:
        servo_set = set(servo)
        for p in MUST_JOIN_AT_STAR:
            route = path(pads[p], pads[RETURN])
            if route is None:
                errors.append(f"{p[0]} の {p[1]} から J5− まで GND の導体がつながっていない")
                continue
            first = next(u for u in route if u in servo_set)
            if first != star:
                errors.append(f"{p[0]} の {p[1]} は servo の経路に {first} で付く（star point {star} ではない）")
            if pads[p] == star:
                continue  # 足の穴が star point そのもの（迂回の経路は無い）
            # 最短の経路だけでは、star point を迂回する別の導体を見逃す。star point を除いても
            # servo の経路とつながるなら、star point を通らない戻りがある
            reach, q = {pads[p]}, deque([pads[p]])
            while q:
                u = q.popleft()
                for v in graph[u]:
                    if v != star and v not in reach:
                        reach.add(v)
                        q.append(v)
            bypass = reach & (servo_set - {star})
            if bypass:
                errors.append(f"{p[0]} の {p[1]} は star point {star} を通らずに servo の経路（{min(bypass)}）へつながる導体がある")
    for h in sorted(off):
        errors.append(f"導体が基板に無い穴 {h}（列, 行）を通る")
    # R18 の足2 は star point へ直接つなぐ：最初の1歩が star point で、Pi・logic の戻りの経路と star point 以外を共有しない
    if servo is not None and star in servo:
        r18 = path(pads[("R18", "2")], pads[RETURN])
        if r18 is None or not (r18[0] == star or (len(r18) > 1 and r18[1] == star)):
            errors.append(f"R18 の足2 が star point {STAR_HOLE} へ直接つながっていない（経路 {r18[:3] if r18 else None}）")
        else:
            shared = set()
            for p in (("J7", "-"), ("J8", "-")):
                route = path(pads[p], pads[RETURN])
                if route:
                    shared |= (set(route[:route.index(star) + 1]) & set(r18[:2])) - {star}
            if shared:
                errors.append(f"R18 の足2 の導体が Pi・logic の戻りの経路と {sorted(shared)} で共有している")
    for e in errors:
        print("NG:", e, file=sys.stderr)
    if errors:
        return 1
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
