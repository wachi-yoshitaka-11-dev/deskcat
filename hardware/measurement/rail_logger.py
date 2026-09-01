#!/usr/bin/env python3
"""Piのrail電圧と低電圧flagを、再起動をまたいで残すためのpolling logger。

Issue #247 の測定計画3節に対応する。標準ライブラリだけを使う。

**この logger が答えられること**: Pi の software が動いていた区間の
`get_throttled` と `measure_volts` の推移。
**答えられないこと**: Pi が完全に応答停止した後の状態。および
polling 間隔より短い過渡（間隔未満の spike は取り落とす）。

各行ごとに flush と os.fsync を行う。buffer に溜めたまま応答不能になると
直前の値が disk へ落ちないため。**SD への書き込みが毎行発生する。**
測定窓の外で回し続けない。

usage:
  rail_logger.py <出力file> [--interval 秒] [--duration 秒]

`--duration` を省くと SIGTERM/SIGINT まで走る。
"""

import os
import signal
import subprocess
import sys
import time

FIELDS = (
    "iso8601",
    "monotonic_s",
    "boot_id",
    "uptime_s",
    "throttled_raw",
    "throttled_hex",
    "volts_core",
    "volts_sdram_c",
    "temp_c",
    "meminfo_available_kB",
    "note",
)

# get_throttled の bit。Raspberry Pi の公式 documentation が定める割り当てである。
# **この logger は bit を解釈して合否を出さない。**raw 値をそのまま残し、
# 解釈は読む側へ委ねる（`HW-TBD-023` が未確定であり、判定基準が無い）。
THROTTLED_BITS = {
    0: "under-voltage detected",
    1: "arm frequency capped",
    2: "currently throttled",
    3: "soft temperature limit active",
    16: "under-voltage has occurred",
    17: "arm frequency capping has occurred",
    18: "throttling has occurred",
    19: "soft temperature limit has occurred",
}


def vcgencmd(*args):
    """vcgencmd を1回呼ぶ。失敗しても例外を投げず、印を返す。"""
    try:
        out = subprocess.run(
            ("vcgencmd",) + args,
            capture_output=True, text=True, timeout=5,
        )
        if out.returncode != 0:
            return "ERR:rc=%d" % out.returncode
        return out.stdout.strip()
    except FileNotFoundError:
        return "ERR:notfound"
    except subprocess.TimeoutExpired:
        return "ERR:timeout"
    except OSError as e:
        return "ERR:%s" % e.errno


def value_after_eq(text):
    """`throttled=0x0` や `volt=1.3500V` から右辺を取る。"""
    if text.startswith("ERR:"):
        return text
    _, _, rhs = text.partition("=")
    return rhs or text


def read_boot_id():
    try:
        with open("/proc/sys/kernel/random/boot_id", encoding="ascii") as f:
            return f.read().strip()
    except OSError:
        return "unknown"


def read_uptime():
    try:
        with open("/proc/uptime", encoding="ascii") as f:
            return f.read().split()[0]
    except OSError:
        return ""


def read_mem_available():
    try:
        with open("/proc/meminfo", encoding="ascii") as f:
            for line in f:
                if line.startswith("MemAvailable:"):
                    return line.split()[1]
    except OSError:
        pass
    return ""


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    path = sys.argv[1]
    interval = 1.0
    duration = None
    rest = sys.argv[2:]
    while rest:
        key = rest.pop(0)
        if key == "--interval":
            interval = float(rest.pop(0))
        elif key == "--duration":
            duration = float(rest.pop(0))
        else:
            print("不明な引数: %s" % key)
            return 2

    stopping = {"flag": False}

    def stop(signum, _frame):
        stopping["flag"] = True

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)

    boot_id = read_boot_id()
    new_file = not os.path.exists(path) or os.path.getsize(path) == 0

    # append で開く。**既存の記録を切り捨てない。**再起動後に同じ file へ
    # 続けて書くことで、boot をまたいだ推移が1本の file に残る。
    with open(path, "a", encoding="utf-8") as f:
        if new_file:
            f.write(",".join(FIELDS) + "\n")
        # 起動印。boot_id が変われば再起動を挟んだことが読める。
        f.write("# session start boot_id=%s interval=%s duration=%s\n"
                % (boot_id, interval, duration))
        f.flush()
        os.fsync(f.fileno())

        started = time.monotonic()
        tick = 0
        while not stopping["flag"]:
            if duration is not None and time.monotonic() - started >= duration:
                break
            thr = vcgencmd("get_throttled")
            row = (
                time.strftime("%Y-%m-%dT%H:%M:%S%z"),
                "%.3f" % (time.monotonic() - started),
                boot_id,
                read_uptime(),
                thr,
                value_after_eq(thr),
                value_after_eq(vcgencmd("measure_volts", "core")),
                value_after_eq(vcgencmd("measure_volts", "sdram_c")),
                value_after_eq(vcgencmd("measure_temp")),
                read_mem_available(),
                "",
            )
            f.write(",".join(row) + "\n")
            # **毎行 fsync する。**この logger の存在理由がここである。
            f.flush()
            os.fsync(f.fileno())
            # **絶対時刻でscheduleする。**`sleep(interval)`だと vcgencmd 4回と
            # fsync の所要が毎周期上乗せされ、標本間隔が単調に伸びる（実測 1.075 s）。
            tick += 1
            slack = (started + tick * interval) - time.monotonic()
            if slack > 0:
                time.sleep(slack)

        f.write("# session end boot_id=%s reason=%s\n"
                % (boot_id, "signal" if stopping["flag"] else "duration"))
        f.flush()
        os.fsync(f.fileno())

    # directory entry も落とす（file を新規作成した場合に要る）
    dirfd = os.open(os.path.dirname(os.path.abspath(path)) or ".", os.O_RDONLY)
    try:
        os.fsync(dirfd)
    finally:
        os.close(dirfd)
    return 0


if __name__ == "__main__":
    sys.exit(main())
