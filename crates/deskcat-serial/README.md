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

含まないもの:

- **実portを開いての確認**（下記）
- **domain動作。**感情、性格、行動判断、独り言は入らない。公開するのは
  connection stateとcounterだけである
- **session遷移の確定、duplicate履歴、受理budget、遷移cooldown。**受信側のstateを
  要するため[Issue #12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)の範囲である

## 実deviceのbackend（`SerialDevice`）

`serial2`でportを開く。crateの選定は[Development Workflow](../../docs/governance/development-workflow.md)の
依存追加の手順に従い、`Cargo.toml`のcommentへ8項目（必要性・公式性・保守状況・target・
license・security・build負荷・代替）を記録した。**`serialport`は採らなかった。**
MPL-2.0であり、`nix`／`bitflags`／`unescaper`とCの`libudev`を引く。
Pi Zero W（使用可能memory 426 MiB、依存付きbuildは未評価）へ持ち込む量を最小にするため、
Linuxでの推移依存が`cfg-if`と`libc`の2つだけである`serial2`を採った。

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

## 実機に残っていること

**このcrateの検証はhost（VM）上である。**testは`SerialPort::pair()`の擬似端末を使い、
実のfile descriptor越しに行の復元、切断（`EIO`）、idle、送出を確認している。
**`SerialDevice::open()`はtestで呼んでいない。**openこそがhardware無しに検証できない
部分であり、通したことにしない。

[Issue #11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11)の後半に残るもの:

- `/dev/ttyUSB*`のdevice名の確定
- 実portでのread／write、切断、reconnect、partial I/Oの確認
- `CLOCAL`をdriverが受け付けること（受け付けなければopenが失敗する）
- **`HUPCL`の判断。**既定ではcloseでDTRが落ちる。ESP32の開発boardはDTR／RTSに自動reset
  回路を持つものが多く、その場合**再接続のたびにESP32が再起動する。**`boot`／`hello`の
  handshakeに効くprotocol側の判断であり、**このcrateでは触っていない**
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

repository rootで実行する。

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
```

lint levelはroot `Cargo.toml`の`[workspace.lints]`が持つため、`-D warnings`は要らない。
`cargo fmt`は`--locked`を受け付けない。

`Cargo.toml`の`[lints] workspace = true`を消さない。workspace lintはmemberへ
自動継承されず、消すと`unsafe_code = "forbid"`、`missing_docs = "warn"`、
`clippy::all = deny`がすべて無効になる。**しかもbuildは通るため気づかない。**
