#!/usr/bin/env python3
"""adc_stream_rate.py の parser を board 無しで検証する。

framing、再同期、checksum、予約bit、block欠番、取りこぼしの集計を、
合成した byte 列に対して確認する。**実機を占有せずに回せる。**

Python 3 の標準ライブラリだけを使う（ADR-0006）。

    python3 hardware/measurement/test_adc_stream_rate.py
"""

from __future__ import annotations

import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import adc_stream_rate as asr  # noqa: E402


def build_block(seq, taken, dropped, mark_us, mark_taken, values, nch=2, adps=7):
    """sketch と同じ並びで1 block を組む。値は (ch, adc) の列で渡す。"""
    cfg = (adps & 0x07) | (0x08 if nch == 2 else 0x00)
    header = bytearray(asr.MAGIC)
    header += struct.pack(
        "<HIHIIBB", seq, taken, dropped, mark_us, mark_taken, len(values), cfg
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


class TestParser(unittest.TestCase):
    def test_parses_single_block(self):
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
        good = build_block(2, 256, 0, 20, 256, alt_values(4))
        bad = bytearray(build_block(3, 384, 0, 30, 384, alt_values(4)))
        bad[asr.HEADER_LEN - 1] ^= 0xFF  # header 末尾のXOR byteを壊す
        p = asr.BlockParser()
        got = p.feed(bytes(bad) + good)
        # 壊れた block は落ちるが、後続の正しい block は拾える。
        self.assertEqual([b.seq for b in got], [2])
        self.assertGreaterEqual(p.stats.header_xor_errors, 1)

    def test_reserved_bit_error_detected(self):
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
    def _two_blocks(self, dropped_second=0, seq_second=1):
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

    def test_rate_from_arduino_clock(self):
        blocks, stats = self._two_blocks()
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=2.0, discard_blocks=0)
        # mark 間 128 sample を 12800 us で進んだので 1 sample = 100 us -> 10000 Sample/s
        self.assertAlmostEqual(r["arduino_rate_taken"], 10000.0, places=3)
        # 壁時計は 256 sample / 2.0 s = 128 Sample/s
        self.assertAlmostEqual(r["wall_rate_total"], 128.0, places=6)
        self.assertEqual(r["nch"], 2)
        self.assertEqual(r["delivered"], 256)
        self.assertEqual(r["dropped_delta"], 0)
        self.assertEqual(r["seq_gaps"], 0)
        self.assertEqual(r["lost_blocks"], 0)

    def test_drop_accounting(self):
        blocks, stats = self._two_blocks(dropped_second=5)
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=1.0, discard_blocks=0)
        self.assertEqual(r["dropped_delta"], 5)
        self.assertEqual(r["dropped_total_since_boot"], 5)
        # 区間収支が閉じること。先頭 block の sample は first.taken の時点で
        # 既に数え終わっているため、届いた数から先頭 block ぶんを除いて比べる。
        self.assertEqual(r["delivered"], 256)
        self.assertEqual(r["delivered_in_interval"], 128)
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

    def test_block_seq_gap_detected(self):
        blocks, stats = self._two_blocks(seq_second=3)
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=1.0, discard_blocks=0)
        self.assertEqual(r["seq_gaps"], 1)
        self.assertEqual(r["lost_blocks"], 2)

    def test_per_channel_split(self):
        blocks, stats = self._two_blocks()
        r = asr.summarize(blocks, stats, t_start=0.0, t_end=1.0, discard_blocks=0)
        self.assertEqual(set(r["per_ch"]), {0, 1})
        self.assertEqual(r["per_ch"][0]["min"], 675)
        self.assertEqual(r["per_ch"][0]["max"], 675)
        self.assertEqual(r["per_ch"][1]["min"], 0)
        self.assertEqual(r["per_ch"][1]["max"], 0)


class TestUsDelta(unittest.TestCase):
    def test_wrap(self):
        self.assertEqual(asr.us_delta(5, 1), 4)
        # micros() の uint32 wrap を跨いでも正の差になる。
        self.assertEqual(asr.us_delta(3, (1 << 32) - 2), 5)


if __name__ == "__main__":
    unittest.main(verbosity=2)
