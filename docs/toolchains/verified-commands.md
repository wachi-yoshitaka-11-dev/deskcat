# 検証済みコマンド

> 状態: Active
> 適用範囲: build、lint、test、flash の実行コマンドと、その来歴

**この文書が検証済みコマンドの正本である。**以前は[AGENTS.md](../../AGENTS.md)が
コマンドと来歴の両方を持っていた。**`AGENTS.md`は毎sessionのcontextへ全文が入るため、
firmwareを触らないsessionもESP32のbuild手順を読み込んでいた**（[ADR-0018](../decisions/0018-instruction-file-structure.md)）。

**ここへ来歴を集約し、コマンドの実行時参照は`.claude/rules/`が持つ。**
`.claude/rules/`はpath限定で読み込まれ、この文書へリンクする。**コマンドを両方へ書かない。**

## 実行の順序

利用可能な範囲で次を実行する。

1. format
2. lint
3. unit test
4. host integration test
5. ESP32 build
6. 実機単体試験
7. 統合・回帰試験

**実機試験が必要な変更を、PCテストだけで完了扱いにしない。**

## host workspace

repository root で実行する。**ESP32 toolchain は要らない。**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
```

**`-D warnings` は付けない。**lint の水準は root `Cargo.toml` の `[workspace.lints]` が持つ。
`unsafe_code = "forbid"` もそこで強制している。

Linux x86_64、Rust stable 1.97.1 で検証した。初回は 2026-08-10 で、これは VM 上の記録である
（[Version Record](version-records/2026-08-10-host-rust-linux.md)）。実機 Linux での検証は
2026-08-15 であり、上の block の command がすべて成功している
（[Version Record](version-records/2026-08-15-host-rust-native-linux.md)）。別端末での再現は
CI の `ubuntu-24.04` runner で満たした（#129。[Version Record](version-records/2026-08-15-host-rust-ci.md)）。
**CI が実行するのは host workspace だけであり、Raspberry Pi 上での build と実行は主張しない。**

## ESP32 firmware

ESP32 Build profile の端末で、`firmware/esp32` にて実行する。

```bash
. "$HOME/export-esp.sh"
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
```

`--locked` は追跡している `Cargo.lock` からの逸脱を失敗として扱う。
**`cargo fmt` はこの option を受け付けない。**

Linux x86_64 で検証した。初回は 2026-08-06 で、これは VM 上の初回環境記録である
（[Version Record](version-records/2026-08-06-esp32-build-linux.md)）。実機 Linux での最初の検証は
2026-08-15 である（[Version Record](version-records/2026-08-15-esp32-build-native-linux.md)）。最新の検証は
2026-10-04 であり、Version Record の `Repository commit` の tree（#487 の PR B1 の後の構成。Pi link を含む既定構成）で、`deskcat-servo` を共有した状態で実機 Linux で取得した（[Version Record](version-records/2026-09-29-esp32-build-servo-limiter-native-linux.md)。
初回検証日は 2026-09-29、最終有効な検証日時は 2026-10-04）。
別端末での再現は CI の `ubuntu-24.04` runner で満たした
（#42。[Version Record](version-records/2026-08-10-esp32-build-ci.md)）。
**build-only であり、flash と実機起動は主張しない。**

**`pi-protocol-mode` featureは[#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)で廃止した。**Pi linkは既定構成（製品build）に入った。**上の「Linux x86_64 で検証した」以下の記録のうち、2026-08-15 までの記録は #487 の PR B1 より前の tree についてのものである。2026-09-29 の記録は、2026-10-04 に #487 の PR B1 の後の構成（Pi link を含む既定構成）で再検証した。**2026-09-29の記録のうち、既定構成の分は上の検証記録である。それ以外の構成の分は実施記録であり、検証済みcommandへは格上げしていない。#487のPR B1より前は、`pi-protocol-mode`を有効にした構成も`cargo clippy`と`cargo build`へ`--features pi-protocol-mode`を足して通していた。

**`bringup-display-13` feature（[#451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)）を有効にする場合は、既定構成に加えて`cargo clippy`と`cargo build`（`cargo fmt`は対象外）へ`--features bringup-display-13`を足したものも通す。**#487から、このfeatureは既定構成へLCDの試験モードを加えるだけであり、他のfeatureと排他ではない。**この構成の実施記録（[Version Records](version-records/README.md)）は、検証済みcommandへ格上げする記録ではない。2026-09-29の記録だけは、2026-10-04に#487のPR B1の後の構成（Pi linkを含む）でこの構成も再検証した。他の記録は、どれも#487のPR B1より前のtreeのものである。**そのため、ここでもcommand blockを写さない（`flash と serial monitor`節（下記）がcommand blockを持たない理由とは別である）。

**`bringup-led-514` feature（[#514](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514)）を有効にする場合は、既定構成に加えて`cargo clippy`と`cargo build`へ`--features bringup-led-514`を足したものも通す。**`bringup-display-13`とも組み合わせられるため、`--features bringup-display-13,bringup-led-514`も通す。**これらの構成のVersion Recordは無い。**

**`face-21` feature（[#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)）を有効にする場合は、既定構成に加えて`cargo clippy`と`cargo build`へ`--features face-21`を足したものも通す。**`bringup-display-13`とも組み合わせられるため、`--features face-21,bringup-display-13`も通す。**これらの構成のVersion Recordは無い。**

**`tap-21` feature（[#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)のF1）を有効にする場合は、既定構成に加えて`cargo clippy`と`cargo build`へ`--features tap-21`を足したものも通す。**`face-21`・`bringup-display-13`とも組み合わせられるため、`--features tap-21,face-21,bringup-display-13`も通す。**これらの構成のVersion Recordは無い。**

### workspace との関係

`firmware/esp32` は root workspace から `exclude` している。firmware の manifest は
`[workspace]` 節を持たないため、**exclude を外すと firmware の build が壊れる。**

firmware は `crates/deskcat-protocol`、`crates/deskcat-servo`、`crates/deskcat-face`、`crates/deskcat-tap` を path dependency で使う
（[ADR-0008](../decisions/0008-firmware-protocol-crate-reuse.md)。`deskcat-servo`、`deskcat-face`、`deskcat-tap` は同 ADR の追記）。
**4 crate の `rust-version` は host と ESP toolchain の両方を満たす下限にしてある。**
上げると firmware の build が compile 前に停止する。
**`crates/deskcat-protocol/`、`crates/deskcat-servo/`、`crates/deskcat-face/`、`crates/deskcat-tap/` を変更したら、host だけでなく ESP32 build も回す。**
`deskcat-servo` の下限の根拠は [Version Record](version-records/2026-09-29-esp32-build-servo-limiter-native-linux.md) にある。

### flash と serial monitor

**2026-08-20 に検証した**（[#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)。`espflash` 4.5.0）。
artifact の path を渡し、`--port` と `--chip esp32` を明示する。
**非対話 shell では monitor が落ちるため pty を割り当てる。**
**chip 識別は `esptool` で行い、`espflash` では代替できない**（family 名しか返さない）。
実行した command と版は [Version Record](version-records/2026-08-20-esp32-flash-boot-native.md) にある。
**主張するのは flash と起動記録までであり、周辺回路と servo は含まない。**

**この節は意図して command block を持たない。**Version Record が載せている手順は、
同記録自身が「**再現手順であって、追加の実行記録ではない。上の形で再実行して確かめてはいない**」と
断っているものである。**ここへ写すと、再現手順を検証済み command へ格上げすることになる。**
実行する場合は Version Record を開き、**そこに書かれた条件**（ESP32 Flash / HIL profile の端末に限る、
実機を駆動するため[Hardware Safety Policy](../governance/hardware-safety-policy.md)の条件を満たす）
**ごと読む。**

**「非対話 shell では monitor が落ちるため pty を割り当てる」の出所は`AGENTS.md`の旧記述であり、
Version Record に該当する観測は無い**（2026-09-10 に走査して確認）。**実行前に自分で確かめる。**

**USB の抜き差しによる電源再投入は 2026-08-29 に検証した。**3 回とも `reset_reason=power_on`
かつ `uptime_ms` が小さい値であり、**「電源再投入のあと firmware が定常状態へ到達した」まで
主張できる。****ただし起動出力そのものは今も取得していない**（ROM の boot banner と、
heartbeat 1 本目より前の出力。**host 側の serial port が USB enumerate 後にしか存在しないためであり、
再試行では解決しない**）。

## Raspberry Pi

**2026-08-26 に検証した**（[#11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11) の前半）。
Raspberry Pi Direct Build profile の端末で、source tree の root にて実行する。
**ESP32 toolchain は要らない。**

```bash
cargo build --locked -p deskcat-serial
cargo fmt --all -- --check
cargo clippy --locked -p deskcat-serial --all-targets
cargo test --locked -p deskcat-serial
cargo test --locked -p deskcat-protocol
```

**検証したのはこの 2 crate である。**他の crate で通ることは主張しない。
**`-p` で 1 crate ずつ絞る。**Pi Zero W の使用可能 memory は実測 426 MiB であり、
**`--workspace` を一度に回した場合は未検証である。**実行した版と実測値
（依存 16 crate を含む clean build 22 分 24 秒、peak 単一 process RSS 247364 kB、OOM なし、
138 tests passed）は [Version Record](version-records/2026-08-17-pi-direct-build-native.md) の
2026-08-26 再検証節にある。
**主張するのは build と lint と test までであり、実 serial port と ESP32 との通信は含まない。**

## まだコマンドが無いもの

**Raspberry Pi の実機試験（実 serial port、ESP32 との通信）と HIL には、まだ正式なコマンドが無い。**
[ツールチェーン一覧](README.md) と未検証の runbook 手順を、検証済みコマンドとして扱わない。
**clean build の成功ごとにこの文書を更新する。**
