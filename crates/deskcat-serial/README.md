# deskcat-serial

Host側のserial session。上限付きのread／write、切断の観測、再接続の上限とrate limit、
上限のある送信queueを持つ。

wire仕様の正本は[ESP32–Pi Protocol](../../docs/protocol/esp32-pi-protocol.md)である。
message型、検証、上限付きline受信は[`deskcat-protocol`](../deskcat-protocol/README.md)が持ち、
**このcrateは再実装しない。**

## 範囲

含むもの:

- port名とbaudを設定として扱う型（`SerialConfig`）
- byte列を運ぶ層の境界（`Transport`）とI/O errorの分類（`IoDisposition`）
- 上限のある送信queue（`Outbox`）
- 送信側の`id`採番（`IdAllocator`）
- 接続stateとcounter（`Session`）
- 実serial portの上で`Transport`を満たす型（`SerialDevice`）。`serial2`でportを開き、
  切断のerrnoと読みのtimeoutを契約どおりに正規化する（下記）
- ESP32 peer sessionの状態（`PeerSession`、`src/peer.rs`）。`boot`のsession遷移、
  duplicate履歴、`hello`／`boot`以外の`stale_session`判定、Piが送った`ping`／
  `get_status`／`set_expression`への応答の相関（[Issue #12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)）
- 現在sessionのduplicate履歴（`DuplicateHistory`）。保持件数と保持期間
  （`PROTO-TBD-005`）は`DuplicatePolicy`として呼び出し側から受け取り、値を持たない。
  **定義は`crates/deskcat-protocol/src/duplicate.rs`にある**（hostとfirmwareで共用するために
  [Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)で移した）。
  このcrateは`deskcat_serial::duplicate`ごとre-exportし、以前のpathを保つ
- 受信frameの振り分け（`handle_frame`、`src/coordinator.rs`）。`ack`の相関、`status`の受理、
  現在sessionのeventの受け渡し、ESP32→Piで定義されていないtypeの計上

含まないもの:

- **実portを開いての確認**（下記）
- **domain動作。**感情、性格、行動判断、独り言は入らない。公開するのは
  connection stateとcounterだけである
- **単位時間あたりの受理上限、session遷移budget、cooldown。**`PROTO-TBD-012`が
  未確定であり、値を推測しない。`PeerSession`はこれらのbudgetに依存しない部分だけを
  扱う（`src/peer.rs`のmodule doc参照）

## 実deviceのbackend（`SerialDevice`）

`serial2`でportを開く。crateの選定は[Development Workflow](../../docs/governance/development-workflow.md)の
依存追加の手順に従い、`Cargo.toml`のcommentへ8項目（必要性・公式性・保守状況・target・
license・security・build負荷・代替）を記録した。**`serialport`は採らなかった。**
MPL-2.0であり、`nix`／`bitflags`／`unescaper`とCの`libudev`を引く。
Pi Zero W（公称512 MB、実測の使用可能memoryは**426 MiB**。**依存を伴うbuildがmemory不足で
安定しないかは未評価のまま残っている**——
[Raspberry Pi Rust toolchain](../../docs/toolchains/raspberry-pi-rust-toolchain.md)、
[Version Record](../../docs/toolchains/version-records/2026-08-17-pi-direct-build-native.md)）へ
持ち込む量を最小にするため、Linuxでの推移依存が`cfg-if`と`libc`の2つだけである
`serial2`を採った。

`serial2::SerialPort`は`Read + Write`を実装するため、`transport.rs`のblanket implで
**そのままでも既に`Transport`である。**それでもnewtypeを置くのは、**下層のerrorを
2点だけ正規化するため**である。

| 正規化 | なぜ要るか |
|---|---|
| `EIO`／`ENXIO`／`ENODEV` → `BrokenPipe` | LinuxでUSB serialを抜くとこのerrnoが出るが、standard libraryは対応する`ErrorKind`を持たない。`IoDisposition::classify`は知らないkindを`Fatal`にするため、**正規化しないと切断が再接続の経路へ入らない** |
| `read`の`TimedOut` → `WouldBlock` | `serial2`は`poll`満了を`TimedOut`で表すが、`Transport`の契約は「今はdataが無い」を`WouldBlock`と定めている。移さないとidleのあいだ`counters.timeouts`が増え続ける。**`write`側は移さない**（送信bufferが詰まったままなのは実際に異常である） |

`EBADF`と`ENOTTY`は**写さない。**再接続では直らないため`Fatal`が正しい。

**`open()`のerrorを`IoDisposition::classify`へ渡さない。**同関数が分類するのは確立済みの
linkの上で起きたerrorである。openの`ENOENT`／`EACCES`／`EBUSY`はUSBの再列挙中に起きる
一時的なものだが、`classify`はこれらを`Fatal`にする。再接続のloopは、openの失敗を
`Session::begin_reconnect()`が`None`を返すまで単純に再試行する。

## 動かす（`serial_link` example）

`SerialDevice`を呼ぶ実行体を1つ持つ。**このcrateはloopを持たない**（`Session`は
transportを所有せず、pumpの引数で受け取る）ため、呼び出し側の形をここに置く。

```bash
cargo run --example serial_link -- --port <path> --baud <rate> --duplicate-capacity <n> --duplicate-retention-ms <ms> [--seconds <n>] [--ping-count <n>] [--verbose]
```

`--port`と`--baud`は**どちらも必須である。既定値を持たせない。**device名は未確認であり
（確定は[Issue #11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11)の後半）、
baudの正本は`PROTO-TBD-001`でいずれも`Candidate`である。渡した値は記録にそのまま残る。
`--duplicate-capacity`と`--duplicate-retention-ms`（`boot`のduplicate履歴の保持件数と保持期間）も
同じ理由で必須である。正本は`PROTO-TBD-005`で未確定である。

**出力にdevice名を書かない。**`Version Record Template`の禁止項目であり、出力を
そのまま記録へ貼れるようにしてある。

`--verbose`を付けない限り`Info`までを出す。`Debug`にするとread timeoutごとに1行出て
（既定50 msなので毎秒20行）、長時間の観察では本当のeventが埋まる。

この実行体は、受信したframeを種類を問わず`handle_frame`で`PeerSession`
（[Issue #12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)）へ渡し、送った`hello`の`id`を
`note_hello_sent`で記録し、pumpの1周ごとに`retry_due_requests`でACK timeoutした`get_status`を送り直す。
`retry_due_requests`は、確立の直後にqueueへ入れられなかった`get_status`も送る。
接続のたびに`hello`を1件送る（`reason`は初回が`Startup`、再接続が`PortReopen`。仕様§5.1）。
`--ping-count <n>`（1以上）を付けたときだけ、ESP32 sessionの確立のたびに`ping`を1件ずつ最大n件送る。ACKの相関と、ACK timeoutでの同じ`id`の再送は、`get_status`と同じ経路である。周期は持たない（`PROTO-TBD-010`）。省けば`ping`を送らない。
根拠の読み方: 終了時の`counters: pings_sent=… ping_acks=… ping_rejected=… ping_retries=… ping_gave_up=…`は通算である。`pings_sent`と`ping_acks`が等しくn以上なら、送った`ping`のすべてに`ok`のACKを相関できた（n未満なら、まだ送り終えていない。等しくなければ、拒否・取り下げ・未決のまま終了・ESP32 sessionの切替で捨てた`ping`がある。`peer: session_switches`は最初の確立でも1になり、2以上なら切替があった）。これは「`ping`へのACKを相関した」根拠であり、`boot`→ACK→`get_status`→`status`の往復の成立とは別に記録する。ESP32側のlogと突き合わせる。

**ESP32が`boot`→ACKより先の往復に応えるのは、`firmware/esp32`を
[Issue #487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)のPR B2
（[PR #529](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/529)）以降のsourceから、Pi linkを持つbuild（`bench-servo-test-17`以外）で書き込んだ場合である。**
PR B2で、ESP32は`hello`／`ping`／`get_status`にACKを返し、`get_status`にはACKの直後に`status`を返す。
初回の接続で`boot`より先に`hello`のACKが届けば、`hello`のACK（ESP32の`sid`を未承認のため
`unapproved_hello_acks`へ数える。仕様§6）→`boot`とそのACK→`get_status`のACKと`status`、の順になる
はずである。届く順序とESP32の状態によって`hello`のACKの数え方は変わる（正本は`PeerSession`の
`pending_hello`のdocと`tests/simulator.rs`）。
往復の順と、成り立つ条件は`examples/serial_link.rs`のmodule docにある。
**実機ではまだ確かめていない。**ESP32側の振る舞いはsourceを読んで導いたものである。
`PeerSession`のsession logicは、simulator test（`tests/simulator.rs`）と、この実行体を擬似端末で
走らせた下の表の範囲までで確かめている。
**ESP32側がprotocolを話すとは限らない。**記録では**「行が通った」と「protocolが成立した」を
書き分ける。**

### host（VM）で確認済みの挙動

**擬似端末を相手に実走させた。実serial portではない。**下の表の「実行A」「実行B」「実行C1」「実行C2」は、このcrateの`serial_link`（#12の受信の配線を入れた版。実行C1・C2は`--ping-count`を入れた版）を、fixtureの行で応える偽のESP32（repositoryに置いていないscript）と擬似端末で繋いだ別々の実行である。

| 確認 | 結果 |
|---|---|
| 存在しないportを指定 | openの失敗が`Fatal`にならず、backoffが100→200→400→800→1600 msと伸びて上限5回で`Stopped(ReconnectExhausted)` |
| 行の往復 | `hello`を123 byte書き出して相手が受信。相手の`ping`行を受信して`sid`／`id`／型まで復元 |
| idle | 4秒で`retries=80`、**`timeouts=0`**。「dataが無いだけ」をtimeoutとして数えていない |
| 相手を落とす | 切断を観測（`disconnects=1`）し、再接続へ入って上限で停止 |
| 未承認の`sid`の`hello`のACK（実行A。相手は`hello`を受けると、`boot`と同じ`sid`でACKを返す。`boot`はまだ送っていない） | `hello`の結果として受理せず、`unapproved_hello_acks=1`、`unmatched_acks=0` |
| `boot`を受信（実行A。相手は上のACKの0.2秒後に`crates/deskcat-protocol/tests/fixtures/valid.json`の`boot_minimal`を送り、`get_status`を受けるとACKと同じfileの`status_snapshot`を続けて返す） | `Established`へ移り、`boot`のACKと`get_status`を送出。`get_status`のACKを相関させ（logの`ackを相関した: request=GetStatus`）、続く`status`を応答として扱う（`solicited=true`） |
| `get_status`にACKが来ない（実行B。相手は実行Aと同じだが、`get_status`に応答しない） | ACK timeout（`SerialConfig::new`の既定の`RetryPolicy::provisional`。500 ms、`max_retries`は1）の後に同じ`id`で1回送り直し（偽のESP32が`id=3`の`get_status`を2回受信した）、予算を使い切って取り下げる（logの`応答待ちを取り下げた`） |
| `--ping-count 3`（実行C1。相手は実行Aと同じだが、`ping`にもACKを返す） | `pings_sent=3 ping_acks=3 ping_rejected=0 ping_retries=0 ping_gave_up=0`。logの順は、`ping`をqueueへ入れる→そのACKを相関→次の`ping`、の繰り返し。偽のESP32は`ping`の`id`4・5・6を各1回受信した |
| `--ping-count 3`（実行C2。相手は実行C1と同じだが、1件目の`ping`にだけACKを返す） | `pings_sent=3 ping_acks=1 ping_rejected=0 ping_retries=2 ping_gave_up=2`。偽のESP32が`ping`の`id`5・6を各2回受信し（同じ`id`で1回再送）、予算を使い切って取り下げる（logの`応答待ちを取り下げた`）。ESP32側のlogとの突き合わせは、偽のESP32のため行っていない |

## 実機に残っていること

**このcrateの検証はhost（VM）上である。**testは`SerialPort::pair()`の擬似端末を使い、
実のfile descriptor越しに行の復元、切断（`EIO`）、idle、送出を確認している。
**`SerialDevice::open()`はtestで呼んでいない。**openこそがhardware無しに検証できない
部分であり、通したことにしない。

[Issue #11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11)の後半に残るもの:

- 実機でのdevice名の確定（Pi linkはGPIOのUARTであり、Pi側は`/dev/serial0`を使う。この名前はRaspberry Pi公式文書から導いた。Piでは`/dev/serial0`が`ttyAMA0`を指すことを確かめた（[experiment-log.md](../../docs/hardware/experiment-log.md)の`EXP-020`）。開けることとUARTでの通信は確かめていない。正は[gpio-assignment.md](../../docs/hardware/gpio-assignment.md)の`Pi側の設定`）
- 実portでのread／write、切断、reconnect、partial I/Oの確認
- `CLOCAL`をdriverが受け付けること（受け付けなければopenが失敗する）
- **`HUPCL`の判断。**既定ではcloseでDTRが落ちる。**本projectのESP32 boardでDTR／RTSが
  自動resetへ繋がっているかは確認していない。**繋がっていれば再接続のたびにESP32が
  再起動することになり、`boot`／`hello`のhandshakeに効く。現物の確認と判断はprotocol側の
  話であり、**このcrateでは触っていない**。**Pi linkはGPIOのUART（TX／RX／GNDだけ。[gpio-assignment.md](../../docs/hardware/gpio-assignment.md)の
  `Pi–ESP32間のtransport`節）であり、PiのDTRはESP32へつながらないため、Pi linkではこの再起動は起きない。**
  当てはまるのは、hostがESP32 board上のUSB-UARTブリッジ（USB）を開く場合だけである（Pi linkの試験中に、
  PCのserial monitorでESP32のdebug logを読む場合もこれに当たる。再起動が起きるかは確かめていない）
- Pi上でこのcrateをbuildできるか（memory）

## 既定値は暫定である

再接続の回数上限とbackoff、送信queueの容量は**設定parameterとして受け取る。**
`ReconnectPolicy::provisional()`と`SerialConfig::DEFAULT_OUTBOX_CAPACITY`が返す値は
**暫定であり、確定値ではない。**

正本は`PROTO-TBD-012`（単位時間あたりの受理上限、送出上限、cooldown）と
`PROTO-TBD-017`（再送契約のparameter）であり、**いずれも負荷試験・reconnect試験待ち**である。
これらが確定したら、暫定値を置き換えたうえでこの節を更新する。

`SerialConfig`は`Default`を実装していない。device名とbaudに既定値を持たせると、
確認していない値が設定の既定として固定される。§2の`115200`も`Candidate`であり、
確定値は`PROTO-TBD-001`である。

**不正な設定はpanicではなく`ConfigError`で返す。**port名が空、baudが0、queue容量が0、
backoffの初期値が0、backoffの上下が逆、のそれぞれに変種がある。これらは呼び出し側から
渡る値であり、`assert!`で落とすとhost processごと終わる。分類して返し、初期化側が
logとcounterへ落とせるようにする。**とくにbackoffの初期値0は、`backoff()`が常に0を返して
rate limitを実質的に無効化するため受け付けない。**

## `id`の上限に達したときの動作

仕様§3の`PROTO-TBD-003`をそのまま実装する。

- `id`をwrapさせない
- 次に割り当てる新規`id`が`u32`の上限値そのものになった時点で、その上限値を
  終端報告のために予約し、それ以外の新しい`(sid, id)`を要する送出をすべて止める
- 予約と払い出しは`&mut self`で直列化する。採番点を複数持たない
- 上限値そのものは正当な`id`である。受信側に予約値の判定を足さない
- 既に送出したmessageの再送は同じ`(sid, id)`で行うため、上限到達後も実行できる

**`sid`は自動で選び直さない。**復帰はprocessの再起動、または運用者の明示的な
session resetによる（§3.1）。

## 設定を将来どこへ移すか

`deskcat-config`（型付き設定と検証）は未作成である。`SerialConfig`は当面ここに置くが、
同crateを作る時点で移す前提とする。移すときも「**既定値を持たないfieldがどれか**」という
性質は保つ。

## 検証

**commandの正本は[検証済みコマンド](../../docs/toolchains/verified-commands.md)である。**
repository rootで実行する。**ここへcommandを写さない**（[ADR-0018](../../docs/decisions/0018-instruction-file-structure.md)）。

lint levelはroot `Cargo.toml`の`[workspace.lints]`が持つため、`-D warnings`は要らない。

`Cargo.toml`の`[lints] workspace = true`を消さない。workspace lintはmemberへ
自動継承されず、消すと`unsafe_code = "forbid"`、`missing_docs = "warn"`、
`clippy::all = deny`がすべて無効になる。**しかもbuildは通るため気づかない。**
