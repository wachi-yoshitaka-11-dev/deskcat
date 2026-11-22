#!/usr/bin/env python3
"""Pi–ESP32 link へ、不正・高 rate の行を流す試験用の送り手（Issue #24）。

**製品の動作を変えない。**`deskcatd` の option にせず、試験の入力だけをここで作る。標準ライブラリだけを使う
（ADR-0006）。実機の窓で、サーボを接続していない no-touch-487 の製品 build に対して使う。手順と合否は
`docs/runbooks/issue-24-fault-window.md` が持つ。

使う前に **`deskcatd` を止める**（同じ port を 2 つの process が開くと、行が混ざる）。

入力の出所のうち `corpus` と `hello`・`get_status` の行は `crates/deskcat-protocol/tests/fixtures/{valid,invalid}.json` で、
host の `crates/deskcat-protocol/tests/stress.rs` も同じ fixture を使う。`noise`・`oversize`・`newline-free` はこの script が
自分で作る（host test が作る byte 列とは別。同じ種類の入力の性質を host で確かめている）。

モード:

- `corpus`  : `invalid.json` の全 case を 1 回ずつ（分類済みの不正な行）。
- `noise`   : 無効な UTF-8 を含むランダムな行を `--count` 行。固定 seed。
- `oversize`: 1024 byte を超える行を `--count` 行。
- `newline-free`: 改行なしの byte を `--count` KiB 流してから改行を 1 つ送る。
- `flood`   : `hello` の後に `get_status` を `--count` 行、間を空けずに（baud が上限）送る。

どのモードも、最後に `get_status`（id=900000）を 1 件送り、その `ack: ok`（`reply_to` が一致）が `--probe-timeout-s` 以内に返れば復帰とみなす。`flood` は先に全応答を待つ（`--drain-timeout-s`。`drained=no` なら、確認の応答が滞留の後ろに並ぶだけの可能性があり、復帰の判定を保留する）
（終了 code 0）。返らなければ 1。**この復帰の確認は `hello` を前提にする**ため、`corpus` などでも先に `hello` を送る。

**ESP32 の `status.protocol` の counter は firmware が増やさない**（`firmware/esp32/src/pi_link.rs`）。
拒否の数は、ESP32 の UART0 の log（`pi_rx_decode_rejected` など）で数える。この script が表示する
`expected_rejections` と突き合わせる。

`--dump` は port を開かず、送る byte 列を標準出力へ書く（host で内容を確かめる用）。
"""

import argparse
import json
import os
import random
import select
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FIXTURES = REPO / "crates" / "deskcat-protocol" / "tests" / "fixtures"

# 仕様 §2。crates/deskcat-protocol/src/limits.rs の MAX_LINE_BYTES（改行を含む）。
MAX_LINE_BYTES = 1024
SEED = 0x24

# 復帰の確認に使う `get_status` の id。flood の id（100 から）と重ならない。
PROBE_ID = 900_000

MODES = ("corpus", "noise", "oversize", "newline-free", "flood")


def load_cases(name):
    doc = json.loads((FIXTURES / name).read_text(encoding="utf-8"))
    return {c["name"]: c for c in doc["cases"]}


def with_ids(line, sid, ident):
    """fixture の行の `sid` と `id` を差し替える。`ts_ms` は fixture のまま。"""
    obj = json.loads(line)
    obj["sid"] = sid
    obj["id"] = ident
    return (json.dumps(obj, separators=(",", ":")) + "\n").encode("utf-8")


def hello_line(sid):
    valid = load_cases("valid.json")
    return with_ids(valid["hello_startup"]["line"], sid, 1)


def get_status_line(sid, ident):
    valid = load_cases("valid.json")
    return with_ids(valid["get_status"]["line"], sid, ident)


def corpus_lines():
    return [c["line"].encode("utf-8") for c in load_cases("invalid.json").values()]


def noise_lines(count):
    rng = random.Random(SEED)
    out = []
    for _ in range(count):
        n = 1 + rng.randrange(MAX_LINE_BYTES - 2)
        body = bytearray(rng.randrange(256) for _ in range(n))
        body[0] = 0xFF  # UTF-8 に現れない byte。必ず拒否される。
        out.append(bytes(b if b != 0x0A else 0x20 for b in body) + b"\n")
    return out


def oversize_lines(count):
    rng = random.Random(SEED)
    return [b"x" * (MAX_LINE_BYTES + rng.randrange(1, 4096)) + b"\n" for _ in range(count)]


def build_payload(mode, count, sid):
    """送る byte 列のリスト（`hello` を除く）と、その数え方の説明を返す。"""
    if mode == "corpus":
        return corpus_lines()
    if mode == "noise":
        return noise_lines(count)
    if mode == "oversize":
        return oversize_lines(count)
    if mode == "newline-free":
        return [b"z" * 1024 for _ in range(count)] + [b"\n"]
    if mode == "flood":
        return [get_status_line(sid, 100 + i) for i in range(count)]
    raise ValueError(mode)


def expected_rejections(mode, payload):
    """ESP32 が拒否の行を出すと予想する数。wire 上の行ではなく、拒否の対象の数で数える。

    `newline-free` は 1 行（`line_too_long` が 1 度）、`flood` は有効な行だけなので 0。
    """
    return {"corpus": len(payload), "noise": len(payload), "oversize": len(payload),
            "newline-free": 1, "flood": 0}[mode]


def open_port(path, baud):
    import termios
    import tty

    if baud != 115200:
        raise SystemExit("baud は 115200 だけを扱う（termios の表に載せた値）。")
    fd = os.open(path, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    tty.setraw(fd)
    attrs = termios.tcgetattr(fd)
    attrs[4] = attrs[5] = termios.B115200
    attrs[2] |= termios.CLOCAL | termios.CREAD
    termios.tcsetattr(fd, termios.TCSANOW, attrs)
    termios.tcflush(fd, termios.TCIOFLUSH)
    return fd


class Reader:
    """ESP32 から届いた行を数える。"""

    def __init__(self, fd):
        self.fd = fd
        self.buf = b""
        self.counts = {}
        self.acks = {}
        self.status_seen = 0
        self.ok_reply_tos = set()
        self.unparsed = 0

    def pump(self, wait_s=0.0):
        r, _, _ = select.select([self.fd], [], [], wait_s)
        if not r:
            return
        try:
            data = os.read(self.fd, 4096)
        except BlockingIOError:
            return
        self.buf += data
        while b"\n" in self.buf:
            line, self.buf = self.buf.split(b"\n", 1)
            self.take(line)

    def take(self, line):
        try:
            obj = json.loads(line)
            kind = obj["type"]
        except (ValueError, KeyError, TypeError):
            self.unparsed += 1
            return
        self.counts[kind] = self.counts.get(kind, 0) + 1
        if kind == "status":
            self.status_seen += 1
        if kind == "ack":
            payload = obj.get("payload", {})
            key = payload.get("status", "?") + ":" + str(payload.get("code", "-"))
            self.acks[key] = self.acks.get(key, 0) + 1
            if payload.get("status") == "ok":
                self.ok_reply_tos.add(payload.get("reply_to"))


def write_all(fd, reader, data):
    view = memoryview(data)
    while view:
        _, w, _ = select.select([], [fd], [], 0.05)
        if w:
            try:
                n = os.write(fd, view)
            except BlockingIOError:
                n = 0
            view = view[n:]
        reader.pump(0.0)  # 書きながら読む。読まないと ESP32 の応答が詰まる。


def run(args):
    sid = args.sid
    payload = build_payload(args.mode, args.count, sid)
    if args.dump:
        sys.stdout.buffer.write(hello_line(sid))
        for item in payload:
            sys.stdout.buffer.write(item)
        return 0

    fd = open_port(args.port, args.baud)
    reader = Reader(fd)
    sent_bytes = 0
    started = time.monotonic()

    def send(data):
        nonlocal sent_bytes
        write_all(fd, reader, data)
        sent_bytes += len(data)

    send(hello_line(sid))
    reader.pump(1.0)
    for item in payload:
        send(item)
    # 流し終えた応答を待つ。flood では、ESP32 の応答（ack と status）が要求より大きく、同じ baud で返るため、
    # 全部の応答（送った id への ack: ok）が届くまで待つ。待てなかったら `drained=no` とし、復帰の判定を保留する。
    flood_ids = {100 + i for i in range(args.count)} if args.mode == "flood" else set()
    drained = True
    if flood_ids:
        end = time.monotonic() + args.drain_timeout_s
        while time.monotonic() < end and not flood_ids <= reader.ok_reply_tos:
            reader.pump(0.1)
        drained = flood_ids <= reader.ok_reply_tos
    else:
        end = time.monotonic() + 1.0
        while time.monotonic() < end:
            reader.pump(0.1)

    # 復帰は、確認の `get_status`（id=PROBE_ID）への `ack: ok` が届いたことで判定する。flood の残りの応答では成り立たない。
    send(get_status_line(sid, PROBE_ID))
    deadline = time.monotonic() + args.probe_timeout_s
    while time.monotonic() < deadline and PROBE_ID not in reader.ok_reply_tos:
        reader.pump(0.1)
    recovered = PROBE_ID in reader.ok_reply_tos

    elapsed = time.monotonic() - started
    print(
        f"mode={args.mode} sent_items={len(payload)} expected_rejections={expected_rejections(args.mode, payload)} "
        f"sent_bytes={sent_bytes} elapsed_s={elapsed:.1f}"
    )
    print(f"sid={sid}")
    print(f"received_types={json.dumps(reader.counts, sort_keys=True)}")
    print(f"received_acks={json.dumps(reader.acks, sort_keys=True)} unparsed_lines={reader.unparsed}")
    if flood_ids:
        print(f"flood_acked={len(flood_ids & reader.ok_reply_tos)}/{len(flood_ids)} drained={'yes' if drained else 'no'}")
    print(f"recovered={'yes' if recovered else 'no'}")
    os.close(fd)
    return 0 if recovered else 1


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--mode", choices=MODES, required=True)
    p.add_argument("--port", help="serial device（--dump では不要）")
    p.add_argument("--baud", type=int, help="--dump では不要。baud は PROTO-TBD-001 のため既定値を持たせない")
    p.add_argument("--count", type=int, default=100, help="行数（newline-free は KiB）")
    p.add_argument("--sid", type=int, help="hello の sid。省くと実行ごとにランダムに選ぶ（ESP32 は置き換えられた sid を retired として覚え、その sid の hello を stale_session で拒否するため）")
    p.add_argument("--probe-timeout-s", type=float, default=5.0)
    p.add_argument("--drain-timeout-s", type=float, default=300.0, help="flood の全応答を待つ時間（応答は要求より大きい）")
    p.add_argument("--dump", action="store_true")
    args = p.parse_args(argv)
    if not args.dump and not (args.port and args.baud):
        p.error("--port と --baud が要る（--dump を除く）")
    if args.sid is None:
        args.sid = random.SystemRandom().randrange(1, 0xFFFFFFFF)
    if not (0 < args.sid <= 0xFFFFFFFF):
        p.error("--sid は 1 以上 u32 以下")
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
