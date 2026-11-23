#!/usr/bin/env python3
"""`PROTO-02` の基板ファイルの GND の star point と shunt の規則を確かめる。

規則（docs/hardware/power-budget.md の`GND topology`と、`実装方法を決定した（2026-09-03）`の
「Gate は共通 GND 戻り（star point）へ抵抗を介してつなぐ」）。基板の net は次のとおり：
`/ADC-SHUNT`（shunt `R20` の servo の側の端 A の node。J6−、`W1`）と `/GND`（shunt の
star の側の端 B から先。J5−、J7−、J8−、R18 の足2）。

1. shunt `R20` の足1 が `/ADC-SHUNT`（servo の側 A）、足2（B）が `/GND` にある（shunt は servo の戻りだけにある）。
2. `/ADC-SHUNT` の導体は、J6−、`W1`、R20 の足1 だけをつなぐ。**C1・C2 の−は A の node に無い**（servo の過渡電流が
   J6− → A → shunt → B → C− と流れ、shunt を必ず通る）。Pi・logic・ingress の GND（J5−・J7−・J8−・R18 の足2）も含まない（shunt を通らずに servo の戻りが GND へ流れる道が無い）。
3. **star point は B（shunt の足2）の node である。**J5−（5 V ingress の GND）は B に短い導体（2 穴以内）で直接つく。
   servo の戻りは J6− → A → shunt → B と流れ、B で初めてほかの GND に合流する。
4. **J7−（Pi）・J8−（ESP32）・R18 の足2 は、それぞれ別の導体で star point の node（B と J5− の間）へ直接つく。**
   3 本の導体は互いに穴を共有せず、star point の node を除くと互いにつながらない（Pi と ESP32 の GND の間に、
   servo の電流が流れる導体も、共有する導体も無い）。
   したがって J5−・J7−・J8−・R18 と B の間の導体に servo の電流は流れない（B と J5− の間の短い導体だけが、
   全部の戻りを一緒に運ぶ）。**R18 の足2 の導体は B の穴に直接つく**（B と J5− の間の導体の途中につかない）。C1・C2 の−も、J7−・J8−・R18 と導体を共有しない別の導体で node へつく（C1 と C2 の間は共有してよい）。
5. 導体の端と折れ曲がり、通る穴は、基板に実在する穴（`Dwgs.User` の円）の上にある。四隅の穴は無い。

基板ファイル（`hardware/deskcat.kicad_pcb`）の `B.Cu` の導体と、部品の pad の位置・net だけを読む。
KiCad は使わず、標準ライブラリだけで動く（ADR-0006）。90 度の倍数の回転を読む（KiCad の向き：画面で反時計回り）。

使い方: `python3 -I hardware/universal-board/check_star_point.py`。規則を満たせば `OK` を出して 0 で終わる。
"""

import re
import sys
from collections import deque
from pathlib import Path

PCB = Path(__file__).resolve().parent.parent / "deskcat.kicad_pcb"
ORIGIN = (100.0, 100.0)          # 基板の左上の角（基板ファイルの座標）
X0, Y0, PITCH = 3.0, 3.45, 2.54  # 穴の格子（plan.md の`寸法の出所`）
SHUNT = "R20"                    # 足1＝servo の側 A、足2＝star の側 B
J5M, J6M, J7M, J8M, R18_2 = ("J5", "-"), ("J6", "-"), ("J7", "-"), ("J8", "-"), ("R18", "2")


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


def _report(errors):
    for e in errors:
        print("NG:", e, file=sys.stderr)
    if errors:
        return 1
    print("OK")
    return 0


def main():
    text = PCB.read_text(encoding="utf-8")
    nets = dict((n, name) for n, name in re.findall(r'^\t\(net (\d+) "([^"]*)"\)', text, re.M))
    pads, padnet = {}, {}
    for fp in blocks(text, "footprint "):
        ref = re.search(r'\(property "Reference" "([^"]+)"', fp).group(1)
        at = re.search(r'\n\t\t\(at ([-\d.]+) ([-\d.]+)( [-\d.]+)?\)', fp)
        rot = float(at.group(3)) if at.group(3) else 0.0
        if rot % 90.0 != 0.0:
            print(f"NG: {ref} は 90 度の倍数以外に回転しており、この検査は読めない", file=sys.stderr)
            return 1
        cs, sn = {0: (1, 0), 90: (0, 1), 180: (-1, 0), 270: (0, -1)}[int(rot) % 360]
        fx, fy = float(at.group(1)), float(at.group(2))
        for pad in blocks(fp, "pad "):
            num = re.match(r'\(pad "([^"]*)"', pad).group(1)
            px, py = map(float, re.search(r'\(at ([-\d.]+) ([-\d.]+)', pad).groups())
            pads[(ref, num)] = (round(fx + px * cs + py * sn, 3), round(fy - px * sn + py * cs, 3))
            m = re.search(r'\(net (\d+) "([^"]*)"\)', pad)
            padnet[(ref, num)] = m.group(2) if m else ""
    holes = set()
    for c in blocks(text, "gr_circle"):
        if '"Dwgs.User"' not in c:
            continue
        cx, cy = map(float, re.search(r"\(center ([-\d.]+) ([-\d.]+)\)", c).groups())
        holes.add((round(cx, 2), round(cy, 2)))
    if not holes:
        print("NG: 基板の穴（Dwgs.User の円）が読めない", file=sys.stderr)
        return 1
    errors = []
    segs_by_net = {}
    off = set()
    for seg in blocks(text, "segment"):
        sx, sy = map(float, re.search(r"\(start ([-\d.]+) ([-\d.]+)\)", seg).groups())
        ex, ey = map(float, re.search(r"\(end ([-\d.]+) ([-\d.]+)\)", seg).groups())
        netid = re.search(r"\(net (\d+)\)", seg).group(1)
        segs_by_net.setdefault(nets[netid], []).append(((round(sx, 3), round(sy, 3)), (round(ex, 3), round(ey, 3))))
        n = max(round(abs(ex - sx) / PITCH), round(abs(ey - sy) / PITCH))
        for i in range(n + 1):
            q = (sx + (ex - sx) * i / max(n, 1), sy + (ey - sy) * i / max(n, 1))
            if (round(q[0], 2), round(q[1], 2)) not in holes:
                off.add((round((q[0] - ORIGIN[0] - X0) / PITCH + 1), round((q[1] - ORIGIN[1] - Y0) / PITCH + 1)))
    for h in sorted(off):
        errors.append(f"導体が基板に無い穴 {h}（列, 行）を通る")

    def graph(netname):
        segs = segs_by_net.get(netname, [])
        points = {p for s in segs for p in s} | {pads[k] for k, v in padnet.items() if v == netname}
        # 同じ net の 2 本の芯線が互いの内側で交わる点も節点にする
        for i, (a1, a2) in enumerate(segs):
            for b1, b2 in segs[i + 1:]:
                d = (a2[0] - a1[0]) * (b2[1] - b1[1]) - (a2[1] - a1[1]) * (b2[0] - b1[0])
                if abs(d) < 1e-9:
                    continue
                u = ((b1[0] - a1[0]) * (b2[1] - b1[1]) - (b1[1] - a1[1]) * (b2[0] - b1[0])) / d
                v = ((b1[0] - a1[0]) * (a2[1] - a1[1]) - (b1[1] - a1[1]) * (a2[0] - a1[0])) / d
                if -1e-9 <= u <= 1 + 1e-9 and -1e-9 <= v <= 1 + 1e-9:
                    points.add((round(a1[0] + u * (a2[0] - a1[0]), 3), round(a1[1] + u * (a2[1] - a1[1]), 3)))

        def on(p, a, b):
            cr = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
            return abs(cr) < 1e-6 and min(a[0], b[0]) - 1e-6 <= p[0] <= max(a[0], b[0]) + 1e-6 \
                and min(a[1], b[1]) - 1e-6 <= p[1] <= max(a[1], b[1]) + 1e-6
        g = {p: set() for p in points}
        for a, b in segs:
            pts = sorted((p for p in points if on(p, a, b)), key=lambda p: (p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2)
            for u_, v_ in zip(pts, pts[1:]):
                g[u_].add(v_)
                g[v_].add(u_)
        return g

    def path(g, src, dst, skip=()):
        if src not in g or dst not in g:
            return None
        prev, q = {src: None}, deque([src])
        while q:
            u_ = q.popleft()
            if u_ == dst:
                out = []
                while u_ is not None:
                    out.append(u_)
                    u_ = prev[u_]
                return out[::-1]
            for v_ in g[u_]:
                if v_ not in prev and v_ not in skip:
                    prev[v_] = u_
                    q.append(v_)
        return None

    for k in ((SHUNT, "1"), (SHUNT, "2"), J5M, J6M, J7M, J8M, R18_2):
        if k not in pads:
            errors.append(f"部品 {k[0]} の足 {k[1]} が基板ファイルに無い")
    if errors:
        for e in errors:
            print("NG:", e, file=sys.stderr)
        return 1
    # 1. shunt は ADC-SHUNT（servo の側）と GND（star の側）の間だけにある
    if padnet[(SHUNT, "1")] != "/ADC-SHUNT" or padnet[(SHUNT, "2")] != "/GND":
        errors.append(f"{SHUNT} の足1 が /ADC-SHUNT、足2 が /GND でない（{padnet[(SHUNT, '1')]}、{padnet[(SHUNT, '2')]}）")
    # 2. ADC-SHUNT の導体の net に、Pi・logic・ingress の GND の足が無い
    for k in (J5M, J7M, J8M, R18_2):
        if padnet[k] != "/GND":
            errors.append(f"{k[0]} の {k[1]} が /GND の net でない（{padnet[k]}）")
    for k in (J6M,):
        if padnet[k] != "/ADC-SHUNT":
            errors.append(f"{k[0]} の {k[1]} が /ADC-SHUNT の net でない（{padnet[k]}）")
    gnd = graph("/GND")
    A = graph("/ADC-SHUNT")
    B, J5, J7, J8, R2 = pads[(SHUNT, "2")], pads[J5M], pads[J7M], pads[J8M], pads[R18_2]
    for name, p in (("R20 の足2（B）", B), ("J5−", J5), ("J7−", J7), ("J8−", J8), ("R18 の足2", R2)):
        if p not in gnd:
            errors.append(f"{name} が /GND の導体につながっていない")
    if not errors:
        # 3. star point の node ＝ B から J5− への最短の導体。長さは 2 穴（2 ピッチ）以内
        node = path(gnd, B, J5)
        if node is None:
            errors.append("B から J5− まで /GND の導体がつながっていない")
        else:
            length = sum(((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2) ** 0.5 for a, b in zip(node, node[1:]))
            if length > 2 * PITCH + 1e-6:
                errors.append(f"B と J5− の間の導体が {length:.1f} mm で、2 穴（{2 * PITCH:.2f} mm）より長い（servo の電流が長い導体を流れる）")
            nset = set(node)
            # 4. J7−・J8−・R18 の足2 は、それぞれ別の導体で node へ。node を除くと互いにつながらない
            terms = {"J7−": J7, "J8−": J8, "R18 の足2": R2}
            for cref in ("C1", "C2"):
                if (cref, "-") in pads:
                    terms[cref + "−"] = pads[(cref, "-")]
            branch = {}
            for nm, p in terms.items():
                # node の穴のどれかに最初に着く経路（B でも J5− の穴でも、node の穴であればよい）
                if p not in gnd:
                    errors.append(f"{nm} が /GND の導体につながっていない")
                    continue
                hit = None
                prev, qd = {p: None}, deque([p])
                while qd:
                    u_ = qd.popleft()
                    if u_ in nset:
                        hit = u_
                        break
                    for v_ in gnd[u_]:
                        if v_ not in prev:
                            prev[v_] = u_
                            qd.append(v_)
                if hit is None:
                    errors.append(f"{nm} が star point の node（B と J5− の間）につながっていない")
                    continue
                if nm == "R18 の足2" and hit != B:
                    errors.append("R18 の足2 の導体が B の穴に直接つかず、B と J5− の間の導体の途中につく（Gate の基準が servo の戻りの導体を経由する）")
                cells, u_ = set(), hit
                while u_ is not None:
                    cells.add(u_)
                    u_ = prev[u_]
                branch[nm] = cells - nset
            names = list(branch)
            for a_ in range(len(names)):
                for b_ in range(a_ + 1, len(names)):
                    if {names[a_], names[b_]} == {"C1−", "C2−"}:
                        continue
                    if branch[names[a_]] & branch[names[b_]]:
                        errors.append(f"{names[a_]} と {names[b_]} の導体が穴を共有している（戻りが node の手前で合流する）")
            # node を除くと互いにつながらない（node を迂回する導体が無い）
            for a_ in range(len(names)):
                for b_ in range(len(names)):
                    if a_ != b_ and {names[a_], names[b_]} != {"C1−", "C2−"} and path(gnd, terms[names[a_]], terms[names[b_]], skip=nset) is not None:
                        errors.append(f"{names[a_]} は node を通らずに {names[b_]} へつながる導体がある（迂回）")
            # J5−・J7−・J8−・R18 の足が、B と J5− の間の導体の上にない
            for nm, p in terms.items():
                if p in nset:
                    errors.append(f"{nm} の足が B と J5− の間の導体の上にある（servo の電流が流れる穴）")
    for cref in ("C1", "C2"):
        if (cref, "-") in pads:
            if padnet[(cref, "-")] != "/GND":
                errors.append(f"{cref} の − が /GND の net でない（{padnet[(cref, '-')]}。servo の過渡電流が shunt を通らない）")
            if pads[(cref, "-")] in A:
                errors.append(f"{cref} の − が /ADC-SHUNT の導体（A の node）の上にある")
    # 2'. ADC-SHUNT の導体は、Pi・logic・ingress の足を含まない
    for k in (J5M, J7M, J8M, R18_2):
        if pads[k] in A:
            errors.append(f"{k[0]} の足が /ADC-SHUNT の導体の上にある")
    return _report(errors)


if __name__ == "__main__":
    sys.exit(main())
