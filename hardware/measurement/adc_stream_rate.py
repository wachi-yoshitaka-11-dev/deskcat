#!/usr/bin/env python3
"""Arduino Uno R3 の連続streamingを受けて実効sample rateと取りこぼしを測る。

`HW-TBD-034` 方式1（独立した外部観測）の作業1で使う。対になる sketch は
`arduino-transient-logger/arduino-transient-logger.ino` である。

正本は docs/hardware/power-budget.md の `Sample rateとlog形式` と
`手持ち候補に固有の制約: SRAMと取得方式` であり、合格条件も要件値もここへ再掲しない。

Python 3 の標準ライブラリだけを使う（ADR-0006）。pyserial は導入しない。
"""

from __future__ import annotations

import argparse
import os
import select
import struct
import sys
import termios
import time

MAGIC = b"\xa5\x5a"
HEADER_LEN = 21

# micros() は uint32 で約71.6分で wrap する。
US_MODULO = 1 << 32


class Block:
    """1 block ぶんの header と sample。"""

    __slots__ = (
        "seq",
        "taken",
        "dropped",
        "mark_us",
        "mark_taken",
        "nsamples",
        "adps",
        "nch",
        "raw",
    )

    def __init__(self, seq, taken, dropped, mark_us, mark_taken, nsamples, adps, nch, raw):
        self.seq = seq
        self.taken = taken
        self.dropped = dropped
        self.mark_us = mark_us
        self.mark_taken = mark_taken
        self.nsamples = nsamples
        self.adps = adps
        self.nch = nch
        self.raw = raw  # tuple[int]。bit 0-9 が値、bit 10 が channel

    def values(self):
        """(channel, value) を順に返す。"""
        for v in self.raw:
            yield (v >> 10) & 0x01, v & 0x03FF


class ParseStats:
    def __init__(self):
        self.resync_bytes = 0
        self.header_xor_errors = 0
        self.reserved_bit_errors = 0
        self.blocks = 0


class BlockParser:
    """byte列を食わせると Block を返す。framingの破れは数えて読み捨てる。"""

    def __init__(self):
        self._buf = bytearray()
        self.stats = ParseStats()

    def feed(self, chunk: bytes):
        self._buf.extend(chunk)
        return self._drain()

    def _drain(self):
        out = []
        buf = self._buf
        while True:
            # magic を先頭へ持ってくる。
            if len(buf) < 2:
                break
            if buf[0:2] != MAGIC:
                idx = buf.find(MAGIC, 1)
                if idx < 0:
                    # magic の一部が末尾に残っている可能性があるので1 byteだけ残す。
                    self.stats.resync_bytes += max(0, len(buf) - 1)
                    del buf[: max(0, len(buf) - 1)]
                    break
                self.stats.resync_bytes += idx
                del buf[:idx]
                continue

            if len(buf) < HEADER_LEN:
                break

            header = bytes(buf[:HEADER_LEN])
            x = 0
            for b in header[:-1]:
                x ^= b
            if x != header[-1]:
                self.stats.header_xor_errors += 1
                del buf[:1]
                continue

            (seq, taken, dropped, mark_us, mark_taken, nsamples, cfg) = struct.unpack_from(
                "<HIHIIBB", header, 2
            )
            if nsamples == 0:
                self.stats.header_xor_errors += 1
                del buf[:1]
                continue

            need = HEADER_LEN + nsamples * 2
            if len(buf) < need:
                break

            raw = struct.unpack_from("<%dH" % nsamples, buf, HEADER_LEN)
            # bit 11-15 は sketch 側が 0 で予約している。破れていれば framing がずれている。
            if max(raw) >= 0x0800:
                self.stats.reserved_bit_errors += 1
                del buf[:1]
                continue

            del buf[:need]
            self.stats.blocks += 1
            out.append(
                Block(
                    seq=seq,
                    taken=taken,
                    dropped=dropped,
                    mark_us=mark_us,
                    mark_taken=mark_taken,
                    nsamples=nsamples,
                    adps=cfg & 0x07,
                    nch=2 if (cfg & 0x08) else 1,
                    raw=raw,
                )
            )
        return out


def open_serial(path: str, baud: int) -> int:
    """raw mode で開く。指定 baud に対応する termios 定数が無ければ例外を投げる。"""
    name = "B%d" % baud
    if not hasattr(termios, name):
        raise SystemExit("baud %d はこの環境の termios に定数が無い（%s）" % (baud, name))
    speed = getattr(termios, name)

    fd = os.open(path, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    attrs = termios.tcgetattr(fd)
    iflag, oflag, cflag, lflag, _ispeed, _ospeed, cc = attrs

    iflag = 0
    oflag = 0
    lflag = 0
    cflag = termios.CS8 | termios.CREAD | termios.CLOCAL
    cc = list(cc)
    cc[termios.VMIN] = 0
    cc[termios.VTIME] = 0

    termios.tcsetattr(
        fd, termios.TCSANOW, [iflag, oflag, cflag, lflag, speed, speed, cc]
    )
    return fd


def us_delta(later: int, earlier: int) -> int:
    """micros() の wrap を考慮した差。"""
    return (later - earlier) % US_MODULO


def measure(fd: int, seconds: float, settle: float, quiet_banner: bool):
    parser = BlockParser()

    # port を開くと DTR が立って Uno が reset する。bootloader が抜けるまで待ち、
    # その間に届いた byte（bannerを含む）は捨てる。
    settle_end = time.monotonic() + settle
    banner = bytearray()
    while time.monotonic() < settle_end:
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            data = os.read(fd, 65536)
            if data and len(banner) < 512:
                banner.extend(data)
    termios.tcflush(fd, termios.TCIFLUSH)

    if banner and not quiet_banner:
        text = bytes(banner).split(b"\n")[0].decode("ascii", "replace").strip()
        if text.startswith("#"):
            print("board banner: %s" % text)

    blocks = []
    t_start = None
    t_end = None
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        r, _, _ = select.select([fd], [], [], 0.1)
        if not r:
            continue
        data = os.read(fd, 1 << 16)
        if not data:
            continue
        now = time.monotonic()
        got = parser.feed(data)
        if got:
            if t_start is None:
                t_start = now
            t_end = now
            blocks.extend(got)

    return blocks, parser.stats, t_start, t_end


def summarize(blocks, stats, t_start, t_end, discard_blocks: int):
    if len(blocks) <= discard_blocks + 1:
        raise SystemExit(
            "block が足りない（%d 件）。配線・baud・sketchの動作を確認する。" % len(blocks)
        )

    # 起動直後の過渡を捨てる。捨てた件数は報告する。
    used = blocks[discard_blocks:]
    first, last = used[0], used[-1]

    delivered = sum(b.nsamples for b in used)
    taken_delta = (last.taken - first.taken) % (1 << 32)
    dropped_delta = (last.dropped - first.dropped) % (1 << 16)

    # taken / dropped は「差」なので、比べる相手も同じ区間に揃える。
    # first block 自身の sample は first.taken の時点で既に数え終わっているため、
    # 区間内に届いたのは先頭 block を除いたぶんである。ここを揃えないと
    # 先頭 block ぶんだけずれた比較になる。
    delivered_in_interval = delivered - first.nsamples

    # 区間内の収支。ISRが取得した数は、届いた数 + ISRが捨てた数 + 回線上で失った数。
    # 差が残るなら block が回線上で失われている（block欠番と符合するはず）。
    unaccounted = taken_delta - delivered_in_interval - dropped_delta

    # block seq の欠番。
    seq_gaps = 0
    lost_blocks = 0
    prev = first.seq
    for b in used[1:]:
        step = (b.seq - prev) % (1 << 16)
        if step != 1:
            seq_gaps += 1
            lost_blocks += max(0, step - 1)
        prev = b.seq

    wall_s = (t_end - t_start) if (t_start is not None and t_end is not None) else 0.0

    # Arduino自身の時計による rate。mark は「取得したsample数」に対して打たれている。
    ard_rate = None
    mark_span_taken = (last.mark_taken - first.mark_taken) % (1 << 32)
    if mark_span_taken > 0 and last.mark_taken != 0 and first.mark_taken != 0:
        span_us = us_delta(last.mark_us, first.mark_us)
        if span_us > 0:
            ard_rate = mark_span_taken * 1e6 / span_us

    per_ch = {}
    for b in used:
        for ch, v in b.values():
            s = per_ch.get(ch)
            if s is None:
                s = per_ch[ch] = {"n": 0, "min": 1 << 30, "max": -1, "sum": 0}
            s["n"] += 1
            s["sum"] += v
            if v < s["min"]:
                s["min"] = v
            if v > s["max"]:
                s["max"] = v

    return {
        "blocks_total": len(blocks),
        "blocks_used": len(used),
        "blocks_discarded": discard_blocks,
        "nch": first.nch,
        "adps": first.adps,
        "nsamples_per_block": first.nsamples,
        "delivered": delivered,
        "delivered_in_interval": delivered_in_interval,
        "taken_delta": taken_delta,
        "dropped_delta": dropped_delta,
        "unaccounted": unaccounted,
        "dropped_total_since_boot": last.dropped,
        "seq_gaps": seq_gaps,
        "lost_blocks": lost_blocks,
        "wall_s": wall_s,
        "wall_rate_total": (delivered / wall_s) if wall_s > 0 else None,
        "arduino_rate_taken": ard_rate,
        "per_ch": per_ch,
        "stats": stats,
        "used": used,
    }


def print_report(r, baud: int):
    adc_clock = 16_000_000 / (1 << r["adps"]) if r["adps"] else None
    print("")
    print("=== 設定 ===")
    print("  baud                : %d" % baud)
    print("  ADPS                : %d  (ADC clock = F_CPU/2^ADPS)" % r["adps"])
    if adc_clock:
        note = "50-200 kHz の規定内" if 50_000 <= adc_clock <= 200_000 else "**規定外（分解能を落とす取引）**"
        print("  ADC clock           : %.0f Hz  (%s)" % (adc_clock, note))
    print("  channel数           : %d" % r["nch"])
    print("  block当たりsample数 : %d" % r["nsamples_per_block"])

    print("=== 実測 ===")
    print("  測定時間(壁時計)    : %.3f s" % r["wall_s"])
    print("  block              : 受信 %d / 採用 %d / 起動直後を捨てた %d"
          % (r["blocks_total"], r["blocks_used"], r["blocks_discarded"]))
    print("  届いたsample数      : %d" % r["delivered"])
    if r["wall_rate_total"] is not None:
        print("  実効rate(合計)      : %.1f Sample/s   ← PC壁時計基準" % r["wall_rate_total"])
        if r["nch"] > 1:
            print("  実効rate(1 ch当たり): %.1f Sample/s"
                  % (r["wall_rate_total"] / r["nch"]))
    if r["arduino_rate_taken"] is not None:
        print("  取得rate(Arduino時計): %.1f Sample/s   ← micros() 基準・取得側"
              % r["arduino_rate_taken"])
        if r["nch"] > 1:
            print("  同 1 ch当たり        : %.1f Sample/s"
                  % (r["arduino_rate_taken"] / r["nch"]))

    print("=== 取りこぼし ===")
    print("  ISRが捨てたsample   : %d （測定区間）/ %d （boot以降の累計）"
          % (r["dropped_delta"], r["dropped_total_since_boot"]))
    print("  block欠番           : %d 箇所 / 失った block %d 件"
          % (r["seq_gaps"], r["lost_blocks"]))
    # 収支は同じ区間どうしで比べる。先頭 block の sample は first.taken の時点で
    # 既に数え終わっているため、届いた数から先頭 block ぶんを除く。
    print("  区間収支            : 取得 %d = 届いた %d + 捨てた %d + 未説明 %d"
          % (r["taken_delta"], r["delivered_in_interval"], r["dropped_delta"],
             r["unaccounted"]))
    expected_lost = r["lost_blocks"] * r["nsamples_per_block"]
    if r["unaccounted"] == expected_lost:
        print("                        未説明ぶんは block欠番 %d 件と符合する（整合）"
              % r["lost_blocks"])
    else:
        print("                        **未説明ぶんが block欠番から出る %d と合わない。"
              "framingか集計を疑う**" % expected_lost)
    s = r["stats"]
    print("  header XOR不一致    : %d" % s.header_xor_errors)
    print("  予約bit破れ         : %d" % s.reserved_bit_errors)
    print("  再同期で捨てたbyte  : %d" % s.resync_bytes)

    print("=== channel別の生ADC値 ===")
    for ch in sorted(r["per_ch"]):
        c = r["per_ch"][ch]
        print("  ch%d : n=%-8d min=%-5d max=%-5d mean=%.1f"
              % (ch, c["n"], c["min"], c["max"], c["sum"] / c["n"]))
    if len(r["per_ch"]) == 2:
        m0 = r["per_ch"][0]["sum"] / r["per_ch"][0]["n"]
        m1 = r["per_ch"][1]["sum"] / r["per_ch"][1]["n"]
        print("")
        print("  channel順の確認: A0 に 3V3、A1 に GND を入れた場合、ch0 > ch1 を期待する。")
        print("  実測 ch0=%.1f, ch1=%.1f -> %s" % (
            m0, m1,
            "期待どおり" if m0 > m1 else
            "**逆である。DS40002061B §24.5.1 の1変換遅れの段数がずれている疑い**"))
    print("")


def write_csv(path: str, r, allow_drops: bool):
    """log形式は power-budget.md の `Sample rateとlog形式` に合わせる。

    同節は `時刻[us],生ADC値` を定める。2 channelのときだけ ch 列を足す。
    時刻は block header の micros() mark を anchor に、sample index から線形に復元する。
    """
    used = r["used"]
    if r["dropped_delta"] and not allow_drops:
        raise SystemExit(
            "取りこぼしが %d 件ある。時刻の復元が曖昧になるため CSV を書かない。\n"
            "承知のうえで書くなら --allow-drops を付ける（時刻は近似になる）。"
            % r["dropped_delta"]
        )

    first, last = used[0], used[-1]
    span_taken = (last.mark_taken - first.mark_taken) % (1 << 32)
    if span_taken > 0:
        period_us = us_delta(last.mark_us, first.mark_us) / span_taken
    else:
        period_us = 0.0
    if period_us <= 0:
        raise SystemExit("micros() mark から周期を出せない。測定時間を延ばす。")

    nch = first.nch
    base_us = first.mark_us
    base_idx = first.mark_taken
    idx = first.taken - first.nsamples  # この block の先頭sampleの取得index（近似）

    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        if nch > 1:
            fh.write("時刻[us],ch,生ADC値\n")
        else:
            fh.write("時刻[us],生ADC値\n")
        for b in used:
            for ch, v in b.values():
                t = base_us + (idx - base_idx) * period_us
                if nch > 1:
                    fh.write("%.1f,%d,%d\n" % (t, ch, v))
                else:
                    fh.write("%.1f,%d\n" % (t, v))
                idx += 1

    print("CSV を書いた: %s" % path)
    print("  復元に使った周期: %.4f us/sample（mark 区間 %d sample から算出）"
          % (period_us, span_taken))
    print("  **時刻は mark 間の線形復元であり、sample毎の実測時刻ではない。**")


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--port", default="/dev/ttyACM0", help="既定 /dev/ttyACM0")
    p.add_argument("--baud", type=int, default=1000000)
    p.add_argument("--seconds", type=float, default=10.0, help="測定時間。既定 10 s")
    p.add_argument("--settle", type=float, default=2.5,
                   help="open直後のreset待ち。既定 2.5 s")
    p.add_argument("--discard-blocks", type=int, default=4,
                   help="起動直後に捨てる block 数。既定 4")
    p.add_argument("--csv", help="生値を CSV へ書く")
    p.add_argument("--allow-drops", action="store_true",
                   help="取りこぼしがあっても CSV を書く（時刻は近似になる）")
    p.add_argument("--quiet-banner", action="store_true")
    args = p.parse_args(argv)

    fd = open_serial(args.port, args.baud)
    try:
        blocks, stats, t0, t1 = measure(fd, args.seconds, args.settle, args.quiet_banner)
    finally:
        os.close(fd)

    r = summarize(blocks, stats, t0, t1, args.discard_blocks)
    print_report(r, args.baud)
    if args.csv:
        write_csv(args.csv, r, args.allow_drops)
    return 0


if __name__ == "__main__":
    sys.exit(main())
