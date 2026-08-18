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
HEADER_LEN = 24

# micros() は uint32 で約71.6分で wrap する。
US_MODULO = 1 << 32
# 1 wrap ぶんの秒数。これを超える取得では us_delta() が残余だけを返し、
# Arduino時計基準のrateとCSVの時刻が誤りになる。余裕を見て下の上限で弾く。
US_WRAP_SECONDS = US_MODULO / 1e6
MAX_CAPTURE_SECONDS = 4000.0


class Block:
    """1 block ぶんの header と sample。"""

    __slots__ = (
        "seq",
        "taken",
        "dropped",
        "mark_us",
        "mark_taken",
        "nsamples",
        "pending",
        "adps",
        "nch",
        "raw",
        "t_recv",
    )

    def __init__(self, seq, taken, dropped, mark_us, mark_taken, nsamples, pending,
                 adps, nch, raw):
        self.seq = seq
        self.taken = taken
        self.dropped = dropped
        self.mark_us = mark_us
        self.mark_taken = mark_taken
        self.nsamples = nsamples
        # pending は snapshot 時に ring に滞留していた（未送信の）sample数。
        # `taken` はこれを含むため、収支を閉じるには差し引く必要がある。
        self.pending = pending
        self.adps = adps
        self.nch = nch
        self.raw = raw  # tuple[int]。bit 0-9 が値、bit 10 が channel
        self.t_recv = None  # PC側で受け取った時刻（time.monotonic）。measure() が入れる

    def values(self):
        """(channel, value) を順に返す。"""
        for v in self.raw:
            yield (v >> 10) & 0x01, v & 0x03FF


class ParseStats:
    """parser が読み捨てた量と framing の破れの件数。"""

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
        """byte列を追加し、その時点で完成した Block の list を返す。"""
        self._buf.extend(chunk)
        return self._drain()

    def _drain(self):
        """buffer から取り出せる Block をすべて取り出す。"""
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

            (seq, taken, dropped, mark_us, mark_taken, nsamples, pending,
             cfg) = struct.unpack_from("<HIIIIBBB", header, 2)
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
                    pending=pending,
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
    """指定秒数ぶん streaming を受け、Block の list と受信時刻を返す。"""
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
            for b in got:
                # 同じ read で複数 block が来た場合も同じ時刻を入れる。
                # rate は block 間の差分で出すため、この粒度で足りる。
                b.t_recv = now
            blocks.extend(got)

    return blocks, parser.stats, t_start, t_end


def summarize(blocks, stats, t_start, t_end, discard_blocks: int):
    """rate・取りこぼし・channel別統計を集計する。

    差分で出す量（rate、収支）は、分子と分母を同じ区間へ揃える。
    """
    if len(blocks) <= discard_blocks + 1:
        raise SystemExit(
            "block が足りない（%d 件）。配線・baud・sketchの動作を確認する。" % len(blocks)
        )

    # 起動直後の過渡を捨てる。捨てた件数は報告する。
    used = blocks[discard_blocks:]
    first, last = used[0], used[-1]

    delivered = sum(b.nsamples for b in used)
    taken_delta = (last.taken - first.taken) % (1 << 32)
    dropped_delta = (last.dropped - first.dropped) % (1 << 32)

    # taken / dropped / pending は「差」なので、比べる相手も同じ区間に揃える。
    # ここで区間が2種類あり、含む block が違う。**block ごとの sample 数が一定でない
    # 場合に値がずれるため、同じ数として扱わない。**
    #   - header の snapshot 区間: 各 header は「自分の sample を ring から取り出す前」に
    #     採られるので、snapshot first と last の間に取り出されたのは first..last-1
    #   - 受信時刻の区間: first の受信から last の受信までに届いたのは first+1..last
    delivered_between_snapshots = delivered - last.nsamples
    delivered_between_arrivals = delivered - first.nsamples

    # 区間内の収支。`taken` は ring に滞留していて未送信の sample も含むため、
    # pending の増減を差し引かないと収支が閉じない。
    #   taken の増分 = 届いた数 + ISRが捨てた数 + ring滞留の増減 + 回線上で失った数
    pending_delta = last.pending - first.pending
    unaccounted = (taken_delta - delivered_between_snapshots - dropped_delta
                   - pending_delta)

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

    # 壁時計の区間も、分子と同じ区間に揃える。捨てた block の時間を分母へ入れると
    # rate が blocks_discarded / blocks_total のぶん低く出る。
    # 受信時刻が無い場合は**区間の揃わない値を出さず**、rate を None にする。
    if first.t_recv is not None and last.t_recv is not None:
        wall_s = last.t_recv - first.t_recv
    else:
        wall_s = 0.0
    # 取得の総窓（捨てた block を含む）。rate には使わず、参考として報告する。
    window_s = (t_end - t_start) if (t_start is not None and t_end is not None) else 0.0

    # Arduino自身の時計による rate。mark は「取得したsample数」に対して打たれている。
    # **micros() の wrap を跨いだ取得では出さない。**us_delta() は残余しか返せないため、
    # span_taken が全区間を覆う一方で span_us が短く出て、rate が過大になる。
    # 判定にはPCの単調時計を使う（wrap しない）。
    wrap_risk = wall_s >= MAX_CAPTURE_SECONDS
    ard_rate = None
    mark_span_taken = (last.mark_taken - first.mark_taken) % (1 << 32)
    if (not wrap_risk) and mark_span_taken > 0 and last.mark_taken != 0 \
            and first.mark_taken != 0:
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
        "delivered_between_snapshots": delivered_between_snapshots,
        "delivered_between_arrivals": delivered_between_arrivals,
        "taken_delta": taken_delta,
        "dropped_delta": dropped_delta,
        "pending_delta": pending_delta,
        "unaccounted": unaccounted,
        "dropped_total_since_boot": last.dropped,
        "seq_gaps": seq_gaps,
        "lost_blocks": lost_blocks,
        "wall_s": wall_s,
        "window_s": window_s,
        "wrap_risk": wrap_risk,
        # 分子は受信時刻の区間に届いた数。分母の wall_s と同じ区間である。
        "wall_rate_total": (delivered_between_arrivals / wall_s) if wall_s > 0 else None,
        "arduino_rate_taken": ard_rate,
        "per_ch": per_ch,
        "stats": stats,
        "used": used,
    }


def print_report(r, baud: int):
    """summarize() の結果を人が読む形で出す。"""
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
    print("  測定時間(壁時計)    : %.3f s（採用区間。総窓は %.3f s）"
          % (r["wall_s"], r["window_s"]))
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

    if r.get("wrap_risk"):
        print("  **micros() の wrap 周期に近い取得である。Arduino時計基準のrateを出さない。**")

    print("=== 取りこぼし ===")
    print("  ISRが捨てたsample   : %d （測定区間）/ %d （boot以降の累計）"
          % (r["dropped_delta"], r["dropped_total_since_boot"]))
    print("  block欠番           : %d 箇所 / 失った block %d 件"
          % (r["seq_gaps"], r["lost_blocks"]))
    # 収支は同じ区間どうしで比べる。先頭 block の sample は first.taken の時点で
    # 既に数え終わっているため、届いた数から先頭 block ぶんを除く。
    print("  区間収支            : 取得 %d = 届いた %d + 捨てた %d + ring滞留増減 %d + 未説明 %d"
          % (r["taken_delta"], r["delivered_between_snapshots"], r["dropped_delta"],
             r["pending_delta"], r["unaccounted"]))
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
    # 回線上で block を失った場合も index と時刻の対応が崩れるため、両方を見る。
    if (r["dropped_delta"] or r["lost_blocks"]) and not allow_drops:
        raise SystemExit(
            "取りこぼしが %d 件、回線上で失った block が %d 件ある。\n"
            "時刻の復元が曖昧になるため CSV を書かない。\n"
            "承知のうえで書くなら --allow-drops を付ける（時刻は近似になる）。"
            % (r["dropped_delta"], r["lost_blocks"])
        )

    first, last = used[0], used[-1]
    span_taken = (last.mark_taken - first.mark_taken) % (1 << 32)
    if span_taken > 0:
        period_us = us_delta(last.mark_us, first.mark_us) / span_taken
    else:
        period_us = 0.0
    if period_us <= 0:
        raise SystemExit("micros() mark から周期を出せない。測定時間を延ばす。")

    if r.get("wrap_risk"):
        raise SystemExit(
            "取得が micros() の wrap 周期（約 %.0f s）に近い。時刻を復元できないため "
            "CSV を書かない。--seconds を短くする。" % US_WRAP_SECONDS
        )

    if first.mark_taken == 0:
        # mark がまだ打たれていない block を anchor にすると時刻の原点がずれる。
        raise SystemExit(
            "先頭 block に micros() mark が無い。--discard-blocks を増やす。"
        )

    nch = first.nch
    base_us = first.mark_us
    base_idx = first.mark_taken

    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        if nch > 1:
            fh.write("時刻[us],ch,生ADC値\n")
        else:
            fh.write("時刻[us],生ADC値\n")
        for b in used:
            # index は block ごとに header から引き直す。単調加算だけにすると、
            # 回線上で block を失ったときに以降のすべての sample へずれが残り、累積する。
            # header の pending は「この block の sample を取り出す前の ring 滞留数」なので、
            # 先頭 sample の取得 index は taken - pending になる。
            # **これが厳密なのは取りこぼしが無いときだけである。**ISR が捨てた sample は
            # ring へ入らないため、捨てた区間を跨ぐと取得 index が連続しない。
            # 取りこぼしがある場合は上の guard で既定では書かない。
            idx = b.taken - b.pending
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
    """command line entry point。"""
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
    if args.seconds >= MAX_CAPTURE_SECONDS:
        p.error(
            "--seconds は %.0f 未満にする。micros() は約 %.0f s で wrap し、"
            "それを跨ぐと Arduino時計基準のrateとCSVの時刻を復元できない。"
            % (MAX_CAPTURE_SECONDS, US_WRAP_SECONDS)
        )

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
