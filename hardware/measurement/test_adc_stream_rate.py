#!/usr/bin/env python3
"""adc_stream_rate.py の parser を board 無しで検証する。

framing、再同期、checksum、予約bit、block欠番、取りこぼしの集計を、
合成した byte 列に対して確認する。**実機を占有せずに回せる。**

Python 3 の標準ライブラリだけを使う（ADR-0006）。

    python3 hardware/measurement/test_adc_stream_rate.py
"""

from __future__ import annotations

import io
import os
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import adc_stream_rate as asr  # noqa: E402


def build_block(seq, taken, dropped, mark_us, mark_taken, values, nch=2, adps=7,
                pending=None):
    """sketch と同じ並びで1 block を組む。値は (ch, adc) の列で渡す。

    pending を省略した場合は「ring 滞留がこの block ぶんだけ」という素直な状態にする。
    """
    if pending is None:
        pending = len(values)
    cfg = (adps & 0x07) | (0x08 if nch == 2 else 0x00)
    header = bytearray(asr.MAGIC)
    header += struct.pack(
        "<HIIIIBBB", seq, taken, dropped, mark_us, mark_taken, len(values), pending, cfg
    )
    x = 0
    for b in header:
        x ^= b
    header.append(x)
    assert len(header) == asr.HEADER_LEN, len(header)

    payload = bytearray()
    for ch, adc in values:
        payload += struct.pack("<H", (adc & 0x03FF) | ((ch & 0x01) << 10))
    return bytes(header + payload)


def alt_values(n, v0=675, v1=0):
    """A0=3V3 相当と A1=GND 相当を交互に並べる。"""
    out = []
    for i in range(n):
        out.append((0, v0) if i % 2 == 0 else (1, v1))
    return out


def two_blocks(dropped_second=0, seq_second=1):
    """1 sample = 100 us になる 2 block を作る。"""
    n = 128
    b0 = build_block(0, n, 0, 1_000_000, n, alt_values(n))
    # 2 block目: 取得は 2n、mark は n 進んで 1 sample 100 us の想定にする。
    b1 = build_block(
        seq_second,
        2 * n + dropped_second,
        dropped_second,
        1_000_000 + n * 100,
        2 * n,
        alt_values(n),
    )
    p = asr.BlockParser()
    return p.feed(b0 + b1), p.stats


class TestParser(unittest.TestCase):
    """block の切り出しと framing の破れの扱い。"""

    def test_parses_single_block(self):
        """header の全 field と payload を正しく取り出せること。"""
        blk = build_block(7, 1000, 0, 123456, 768, alt_values(8))
        p = asr.BlockParser()
        got = p.feed(blk)
        self.assertEqual(len(got), 1)
        b = got[0]
        self.assertEqual(b.seq, 7)
        self.assertEqual(b.taken, 1000)
        self.assertEqual(b.dropped, 0)
        self.assertEqual(b.mark_us, 123456)
        self.assertEqual(b.mark_taken, 768)
        self.assertEqual(b.nsamples, 8)
        self.assertEqual(b.pending, 8)
        self.assertEqual(b.nch, 2)
        self.assertEqual(b.adps, 7)
        self.assertEqual(list(b.values()), alt_values(8))
        self.assertEqual(p.stats.header_xor_errors, 0)
        self.assertEqual(p.stats.reserved_bit_errors, 0)
        self.assertEqual(p.stats.resync_bytes, 0)

    def test_resync_after_leading_garbage(self):
        """起動直後の banner のような先行 text を読み飛ばせること。"""
        junk = b"# deskcat-transient-logger baud=1000000\r\n"
        blk = build_block(1, 128, 0, 10, 128, alt_values(4))
        p = asr.BlockParser()
        got = p.feed(junk + blk)
        self.assertEqual(len(got), 1)
        self.assertEqual(p.stats.resync_bytes, len(junk))

    def test_header_xor_error_is_counted_and_recovered(self):
        """header の XOR が壊れた block を落とし、後続は拾えること。"""
        good = build_block(2, 256, 0, 20, 256, alt_values(4))
        bad = bytearray(build_block(3, 384, 0, 30, 384, alt_values(4)))
        bad[asr.HEADER_LEN - 1] ^= 0xFF  # header 末尾のXOR byteを壊す
        p = asr.BlockParser()
        got = p.feed(bytes(bad) + good)
        # 壊れた block は落ちるが、後続の正しい block は拾える。
        self.assertEqual([b.seq for b in got], [2])
        self.assertGreaterEqual(p.stats.header_xor_errors, 1)

    def test_reserved_bit_error_detected(self):
        """sample の予約bitの破れを framing のずれとして検出すること。"""
        blk = bytearray(build_block(4, 512, 0, 40, 512, alt_values(4)))
        # payload 先頭 sample の bit 11 を立てる（予約bitの破れ）。
        off = asr.HEADER_LEN
        v = struct.unpack_from("<H", blk, off)[0] | 0x0800
        struct.pack_into("<H", blk, off, v)
        p = asr.BlockParser()
        got = p.feed(bytes(blk))
        self.assertEqual(got, [])
        self.assertEqual(p.stats.reserved_bit_errors, 1)

    def test_split_across_chunks_byte_by_byte(self):
        """1 byte ずつ届いても block を組み立てられること。"""
        blk = build_block(5, 640, 0, 50, 640, alt_values(6))
        p = asr.BlockParser()
        got = []
        for i in range(len(blk)):
            got.extend(p.feed(blk[i : i + 1]))
        self.assertEqual(len(got), 1)
        self.assertEqual(got[0].seq, 5)

    def test_partial_magic_at_tail_is_kept(self):
        """末尾に magic の1 byte目だけ来ても捨てないこと。"""
        p = asr.BlockParser()
        self.assertEqual(p.feed(b"\xa5"), [])
        blk = build_block(6, 768, 0, 60, 768, alt_values(4))
        got = p.feed(blk[1:])
        self.assertEqual(len(got), 1)
        self.assertEqual(p.stats.resync_bytes, 0)


class TestSummary(unittest.TestCase):
    """rate・取りこぼし・欠番の集計。"""

    def test_rate_from_arduino_clock(self):
        blocks, stats = two_blocks()
        blocks[0].t_recv, blocks[1].t_recv = 0.0, 2.0
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=2.0, discard_blocks=0)
        # mark 間 128 sample を 12800 us で進んだので 1 sample = 100 us -> 10000 Sample/s
        self.assertAlmostEqual(r["arduino_rate_taken"], 10000.0, places=3)
        # 壁時計も同じ区間で出す。2 block の受信間隔 2.0 s に届いたのは
        # 後続 block の 128 sample だけなので 64 Sample/s になる。
        self.assertAlmostEqual(r["wall_rate_total"], 64.0, places=6)
        self.assertEqual(r["nch"], 2)
        self.assertEqual(r["delivered"], 256)
        self.assertEqual(r["dropped_delta"], 0)
        self.assertEqual(r["seq_gaps"], 0)
        self.assertEqual(r["lost_blocks"], 0)

    def test_drop_accounting(self):
        """ISR が捨てた数を区間収支へ正しく載せること。"""
        blocks, stats = two_blocks(dropped_second=5)
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=1.0, discard_blocks=0)
        self.assertEqual(r["dropped_delta"], 5)
        self.assertEqual(r["dropped_total_since_boot"], 5)
        # 区間収支が閉じること。先頭 block の sample は first.taken の時点で
        # 既に数え終わっているため、届いた数から先頭 block ぶんを除いて比べる。
        self.assertEqual(r["delivered"], 256)
        # 収支に使うのは snapshot 区間（first..last-1）のぶん。
        self.assertEqual(r["delivered_between_snapshots"], 128)
        self.assertEqual(r["taken_delta"], 133)
        self.assertEqual(r["unaccounted"], 0)

    def test_unaccounted_matches_wire_loss(self):
        """回線上で block を失うと、未説明ぶんがその sample 数に一致する。"""
        n = 128
        b0 = build_block(0, n, 0, 1_000_000, n, alt_values(n))
        # seq を 2 に飛ばし、失った 1 block ぶん (n) も taken に含める。
        b1 = build_block(2, 3 * n, 0, 1_000_000 + 2 * n * 100, 3 * n, alt_values(n))
        p = asr.BlockParser()
        blocks = p.feed(b0 + b1)
        r = asr.summarize(blocks, p.stats, t_start=0.0, t_end=1.0, discard_blocks=0)
        self.assertEqual(r["lost_blocks"], 1)
        self.assertEqual(r["dropped_delta"], 0)
        self.assertEqual(r["unaccounted"], n)

    def test_two_intervals_differ_when_block_size_varies(self):
        """block の sample 数が不均一なとき、2つの区間を同じ数として扱わないこと。"""
        b0 = build_block(0, 100, 0, 1_000_000, 100, alt_values(100), pending=100)
        b1 = build_block(1, 150, 0, 1_000_000 + 5000, 150, alt_values(50), pending=50)
        p = asr.BlockParser()
        blocks = p.feed(b0 + b1)
        blocks[0].t_recv, blocks[1].t_recv = 0.0, 1.0
        r = asr.summarize(blocks, p.stats, 0.0, 1.0, discard_blocks=0)
        self.assertEqual(r["delivered"], 150)
        # snapshot 区間は last を除く -> 100。受信区間は first を除く -> 50。
        self.assertEqual(r["delivered_between_snapshots"], 100)
        self.assertEqual(r["delivered_between_arrivals"], 50)
        self.assertEqual(r["unaccounted"], 0)
        self.assertAlmostEqual(r["wall_rate_total"], 50.0, places=6)

    def test_wall_rate_is_none_without_recv_times(self):
        """受信時刻が無いときは、区間の揃わないrateを出さないこと。"""
        blocks, stats = two_blocks()
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=2.0, discard_blocks=0)
        self.assertIsNone(r["wall_rate_total"])
        self.assertAlmostEqual(r["window_s"], 2.0, places=6)

    def test_block_seq_gap_detected(self):
        """block sequence の欠番と失った件数を数えること。"""
        blocks, stats = two_blocks(seq_second=3)
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=1.0, discard_blocks=0)
        self.assertEqual(r["seq_gaps"], 1)
        self.assertEqual(r["lost_blocks"], 2)

    def test_per_channel_split(self):
        """channel ごとに生値を分けて集計すること。"""
        blocks, stats = two_blocks()
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=1.0, discard_blocks=0)
        self.assertEqual(set(r["per_ch"]), {0, 1})
        self.assertEqual(r["per_ch"][0]["min"], 675)
        self.assertEqual(r["per_ch"][0]["max"], 675)
        self.assertEqual(r["per_ch"][1]["min"], 0)
        self.assertEqual(r["per_ch"][1]["max"], 0)


class TestIntervalConsistency(unittest.TestCase):
    """壁時計 rate の分子と分母が同じ区間であることを確かめる。"""

    @staticmethod
    def _uniform(n_blocks, n=128, period_us=100, dt=0.1):
        """等間隔に受信した n_blocks 件の block を作る。"""
        blocks = []
        p = asr.BlockParser()
        raw = b""
        for k in range(n_blocks):
            taken = (k + 1) * n
            raw += build_block(k, taken, 0, 1_000_000 + k * n * period_us, taken,
                               alt_values(n))
        blocks = p.feed(raw)
        for k, b in enumerate(blocks):
            b.t_recv = k * dt
        return blocks, p.stats

    def test_discarding_blocks_does_not_change_rate(self):
        """捨てた block の時間が分母へ残っていれば rate が下がってしまう。"""
        blocks, stats = self._uniform(20)
        r0 = asr.summarize(blocks, stats, 0.0, 1.9, discard_blocks=0)
        blocks2, stats2 = self._uniform(20)
        r4 = asr.summarize(blocks2, stats2, 0.0, 1.9, discard_blocks=4)
        self.assertAlmostEqual(r0["wall_rate_total"], r4["wall_rate_total"], places=6)
        # 128 sample / 0.1 s = 1280 Sample/s
        self.assertAlmostEqual(r0["wall_rate_total"], 1280.0, places=6)

    def test_unaccounted_ignores_ring_residue(self):
        """ring 滞留が増減しても収支は閉じる（pending を差し引くため）。"""
        n = 128
        b0 = build_block(0, n, 0, 1_000_000, n, alt_values(n), pending=n)
        # 2 block目は ring に 10 sample 余分に溜まった状態。taken もそのぶん増える。
        b1 = build_block(1, 2 * n + 10, 0, 1_000_000 + n * 100, 2 * n,
                         alt_values(n), pending=n + 10)
        p = asr.BlockParser()
        blocks = p.feed(b0 + b1)
        r = asr.summarize(blocks, p.stats, 0.0, 1.0, discard_blocks=0)
        self.assertEqual(r["pending_delta"], 10)
        self.assertEqual(r["unaccounted"], 0)


class TestCsv(unittest.TestCase):
    """CSV 出力の時刻復元。"""

    def test_index_recovers_after_wire_loss(self):
        """回線上で block を失っても、時刻のずれが以降へ累積しないこと。"""
        n, period = 128, 100
        specs = [(0, 1 * n), (1, 2 * n), (3, 4 * n)]  # seq 2 は回線上で失われた
        raw = b""
        for seq, taken in specs:
            raw += build_block(seq, taken, 0, 1_000_000 + (taken - n) * period, taken,
                               alt_values(n), pending=n)
        p = asr.BlockParser()
        blocks = p.feed(raw)
        for k, b in enumerate(blocks):
            b.t_recv = k * 0.1
        r = asr.summarize(blocks, p.stats, 0.0, 0.2, discard_blocks=0)
        self.assertEqual(r["lost_blocks"], 1)

        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "out.csv")
            # 欠落があるので既定では書かない。
            with self.assertRaises(SystemExit):
                asr.write_csv(path, r, allow_drops=False)
            asr.write_csv(path, r, allow_drops=True)
            with io.open(path, encoding="utf-8") as fh:
                lines = fh.read().splitlines()
            rows = [line.strip().split(",") for line in lines[1:]]

        self.assertEqual(len(rows), 3 * n)
        base_us, base_idx = blocks[0].mark_us, blocks[0].mark_taken
        # 3番目の block（seq=3）の先頭 sample。取得 index は 3n であり、
        # 届いた順の index（2n）ではない。単調加算だけだと n*period だけ早くなる。
        expect = base_us + (3 * n - base_idx) * period
        self.assertAlmostEqual(float(rows[2 * n][0]), expect, places=1)
        wrong = base_us + (2 * n - base_idx) * period
        self.assertNotAlmostEqual(float(rows[2 * n][0]), wrong, places=1)


class TestWireFormat(unittest.TestCase):
    """wire format の幅に関する回帰。"""

    def test_dropped_is_32bit(self):
        """uint16 を超える取りこぼしを桁落ちなく運べること。

        baud 115200 の実測では10秒で約4万件に達した。uint16 では約16秒で wrap し、
        境界で dropped_delta が0に見えて CSV guard を誤って通す。
        """
        big = 300000  # uint16 では表せない
        blk = build_block(0, big + 128, big, 1_000_000, big + 128, alt_values(8))
        p = asr.BlockParser()
        got = p.feed(blk)
        self.assertEqual(len(got), 1)
        self.assertEqual(got[0].dropped, big)

    def test_header_len_matches_sketch(self):
        """parser の HEADER_LEN が sketch の #define と一致すること。"""
        import re
        path = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                            "arduino-transient-logger",
                            "arduino-transient-logger.ino")
        with io.open(path, encoding="utf-8") as fh:
            ino = fh.read()
        m = re.search(r"#define HEADER_LEN (\d+)", ino)
        self.assertIsNotNone(m)
        self.assertEqual(int(m.group(1)), asr.HEADER_LEN)


class TestWrapGuard(unittest.TestCase):
    """micros() の wrap 周期を跨ぐ取得を弾くこと。"""

    def _two(self, wall_s):
        n = 128
        b0 = build_block(0, n, 0, 1_000_000, n, alt_values(n))
        b1 = build_block(1, 2 * n, 0, 1_000_000 + n * 100, 2 * n, alt_values(n))
        p = asr.BlockParser()
        blocks = p.feed(b0 + b1)
        blocks[0].t_recv, blocks[1].t_recv = 0.0, wall_s
        return asr.summarize(blocks, p.stats, 0.0, wall_s, discard_blocks=0)

    def test_short_capture_reports_arduino_rate(self):
        r = self._two(1.0)
        self.assertFalse(r["wrap_risk"])
        self.assertIsNotNone(r["arduino_rate_taken"])

    def test_long_capture_suppresses_arduino_rate(self):
        r = self._two(asr.MAX_CAPTURE_SECONDS + 1.0)
        self.assertTrue(r["wrap_risk"])
        self.assertIsNone(r["arduino_rate_taken"])

    def test_unassessable_is_treated_as_risky(self):
        """受信時刻が無いときは、判定不能として危険側へ倒すこと。"""
        blocks, stats = two_blocks()
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=2.0, discard_blocks=0)
        self.assertFalse(r["wrap_assessable"])
        self.assertTrue(r["wrap_risk"])
        self.assertIsNone(r["arduino_rate_taken"])
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(SystemExit):
                asr.write_csv(os.path.join(d, "x.csv"), r, allow_drops=True)

    def test_long_capture_refuses_csv(self):
        r = self._two(asr.MAX_CAPTURE_SECONDS + 1.0)
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(SystemExit):
                asr.write_csv(os.path.join(d, "x.csv"), r, allow_drops=True)


class TestUsDelta(unittest.TestCase):
    """micros() の差の取り方。"""

    def test_wrap(self):
        """uint32 の wrap を跨いでも正の差になること。"""
        self.assertEqual(asr.us_delta(5, 1), 4)
        # micros() の uint32 wrap を跨いでも正の差になる。
        self.assertEqual(asr.us_delta(3, (1 << 32) - 2), 5)


if __name__ == "__main__":
    unittest.main(verbosity=2)
