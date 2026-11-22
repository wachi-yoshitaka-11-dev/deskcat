#!/usr/bin/env python3
"""`link_stress.py`の回帰test。port を開かず、送る byte 列の性質だけを確かめる（Issue #24）。"""

import json
import subprocess
import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))

import link_stress as ls  # noqa: E402


class LinkStressTest(unittest.TestCase):
    def test_corpus_is_every_invalid_fixture_case(self):
        lines = ls.corpus_lines()
        self.assertEqual(len(lines), len(ls.load_cases("invalid.json")))
        for line in lines:
            self.assertTrue(line.endswith(b"\n"))
            self.assertEqual(line.count(b"\n"), 1)

    def test_noise_is_deterministic_invalid_utf8_without_inner_newline(self):
        a, b = ls.noise_lines(50), ls.noise_lines(50)
        self.assertEqual(a, b)
        for line in a:
            self.assertEqual(line[0], 0xFF)
            self.assertEqual(line.count(b"\n"), 1)
            self.assertLessEqual(len(line), ls.MAX_LINE_BYTES)
            with self.assertRaises(UnicodeDecodeError):
                line.decode("utf-8")

    def test_oversize_lines_exceed_the_limit(self):
        for line in ls.oversize_lines(20):
            self.assertGreater(len(line), ls.MAX_LINE_BYTES)
            self.assertEqual(line.count(b"\n"), 1)

    def test_flood_lines_are_valid_get_status_with_distinct_ids(self):
        lines = ls.build_payload("flood", 30, 24_000_001)
        ids = set()
        for line in lines:
            obj = json.loads(line)
            self.assertEqual(obj["type"], "get_status")
            self.assertEqual(obj["sid"], 24_000_001)
            ids.add(obj["id"])
            self.assertLess(len(line), ls.MAX_LINE_BYTES)
        self.assertEqual(len(ids), 30)

    def test_hello_uses_the_given_sid(self):
        obj = json.loads(ls.hello_line(777))
        self.assertEqual((obj["type"], obj["sid"], obj["id"]), ("hello", 777, 1))

    def test_newline_free_mode_ends_with_one_newline(self):
        data = b"".join(ls.build_payload("newline-free", 3, 1))
        self.assertEqual(data.count(b"\n"), 1)
        self.assertTrue(data.endswith(b"\n"))

    def test_dump_runs_without_a_port(self):
        for mode in ls.MODES:
            out = subprocess.run(
                [sys.executable, str(SCRIPTS / "link_stress.py"), "--mode", mode, "--count", "3", "--dump"],
                capture_output=True,
                check=False,
            )
            self.assertEqual(out.returncode, 0, mode)
            self.assertTrue(out.stdout.startswith(b'{"v":1'), mode)

    def test_port_and_baud_are_required_without_dump(self):
        out = subprocess.run(
            [sys.executable, str(SCRIPTS / "link_stress.py"), "--mode", "corpus"],
            capture_output=True,
            check=False,
        )
        self.assertEqual(out.returncode, 2)


if __name__ == "__main__":
    unittest.main()
