#!/usr/bin/env python3
"""`hardware/overview/generate.py` の板の中継の検査の回帰test（#512）。

板（B1〜B3）は pin を持たない節点の線を中継できるが、中継を認めるのは、その板の上の線の端が
すべて同じ strip にある場合だけである。`/LCD-CS` の余分な線を B1 から B3 へ2本足し、
B3 の上でつながらない strip に落ちたとき、検査が NG にすることを確かめる。
"""

import importlib.util
import sys
import unittest
from pathlib import Path

PATH = Path(__file__).resolve().parent.parent / "hardware" / "overview" / "generate.py"
spec = importlib.util.spec_from_file_location("overview_generate", PATH)
g = importlib.util.module_from_spec(spec)
sys.modules["overview_generate"] = g
spec.loader.exec_module(g)


class RelayTest(unittest.TestCase):
    def errors(self, extra):
        bb = g.load_breadboard()
        bb.SOFT = list(bb.SOFT) + extra
        pin_net, net_nodes, refs = g.load_netlist()
        return g.run_checks(g.collect_wires(bb), pin_net, net_nodes, refs)

    def test_current_data_is_ok(self):
        self.assertEqual(self.errors([]), [])

    def test_two_extra_wires_into_unconnected_strips_are_rejected(self):
        extra = [("S24", "B1:A14", "B3:b3", "GPIO22", ""), ("S25", "B1:A14", "B3:b8", "GPIO22", "")]
        self.assertTrue(any("B3" in e and "strip" in e for e in self.errors(extra)))


if __name__ == "__main__":
    unittest.main()
