#!/usr/bin/env python3
"""`PROTO-02` の基板ファイルの GND の star point の規則を確かめる。

規則（docs/hardware/power-budget.md の`GND topology`と、`実装方法を決定した（2026-09-03）`の
「Gate は共通 GND 戻り（star point）へ抵抗を介してつなぐ」）：

- servo の戻りの電流は、J6− から J5− までの GND の導体を流れる（servo の経路）。
- star point は、servo の経路の上の (16,9) の穴である。
- Gate 抵抗 R18 の GND 側と、Pi（J7−）・logic（J8−）の GND は、J5− へ向かう途中で
  servo の経路に最初に触れる点が star point でなければならない（servo の電流が流れる区間の
  途中に付けない）。

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
STAR_HOLE = (16, 9)
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
        if at.group(3) and float(at.group(3)) != 0:
            print(f"NG: {ref} は回転しており、この検査は読めない", file=sys.stderr)
            return 1
        fx, fy = float(at.group(1)), float(at.group(2))
        for pad in blocks(fp, "pad "):
            num = re.match(r'\(pad "([^"]*)"', pad).group(1)
            px, py = map(float, re.search(r'\(at ([-\d.]+) ([-\d.]+)', pad).groups())
            pads[(ref, num)] = (round(fx + px, 3), round(fy + py, 3))
    segs = []
    for seg in blocks(text, "segment"):
        if f"(net {gnd[0]})" not in seg or '"B.Cu"' not in seg:
            continue
        sx, sy = map(float, re.search(r"\(start ([-\d.]+) ([-\d.]+)\)", seg).groups())
        ex, ey = map(float, re.search(r"\(end ([-\d.]+) ([-\d.]+)\)", seg).groups())
        segs.append(((round(sx, 3), round(sy, 3)), (round(ex, 3), round(ey, 3))))
    star = hole_mm(*STAR_HOLE)
    points = {p for s in segs for p in s} | set(pads.values()) | {star}

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
    for e in errors:
        print("NG:", e, file=sys.stderr)
    if errors:
        return 1
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
