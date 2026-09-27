# Version Record: ESP32 Build（既定構成 + `bringup-display-13`、実機Linux x86_64）

様式は [Version Record Template](../version-record-template.md) に従う。

**この記録は`bringup-display-13`構成のVersion Recordを含む。**[検証済みコマンド](../verified-commands.md)が
明記するとおり、`bringup-display-13`構成はまだ正式なVersion Recordを持たない。本記録がそれを兼ねる
（「相当の記録」として、[power-budget.md](../../hardware/power-budget.md)の`DISP-01`追加接続のbring-upの手順・条件(2)が求める記録）。
**検証済みcommandへ格上げする記録ではなく、この1回の実施記録である。**[EXP-016](../../hardware/experiment-log.md#exp-016-disp-01msp2807esp323v3-pin追加接続のbring-upcontroller識別単色fill四隅pattern)が参照する。

## 記録

```text
Record ID: 2026-09-26-esp32-build-bringup-display-13-native-linux
Date: 2026-09-26
Machine profile: ESP32 Build
Operator role: AI agent作業（build-onlyのみ。flash・monitor・USBシリアルポートは開いていない）
Repository commit: 9fdaee417e7803df9c928b54d8b390bc7dc3f9a0（origin/develop）
Working tree clean: yes（`git archive origin/develop`でscratchpadへ展開した未改変コピーを使用。
  worktree・main checkoutいずれのgit stateも変更していない）

OS name: Ubuntu
OS version: 24.04.4 LTS
Kernel: 7.0.0-28-generic
CPU architecture: x86_64
Userspace bitness: 64-bit
Container / VM / native: native（実機）。systemd-detect-virt: none

Rustup version: rustup 1.29.0 (28d1352db 2026-03-05)
Rust channel: esp-1.95.0.0（firmware/esp32/rust-toolchain.tomlが固定）
Rust compiler version: rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
Rust host: x86_64-unknown-linux-gnu
Installed Rust targets: xtensa-esp32-espidf（esp toolchain同梱）
Cargo version: cargo 1.95.0-nightly (f2d3ce0bd 2026-03-21) (1.95.0.0)
rustfmt version: esp toolchain同梱版（rustfmt 1.9.0-nightly (95e5bda868 2026-04-15)）
Clippy version: esp toolchain同梱版（clippy 0.1.95 (95e5bda868 2026-04-15)）
Linker identity and version: cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0（host側）。
  Xtensa側はesp toolchain同梱のxtensa-esp-elf-gcc (crosstool-NG esp-15.2.0_20250920) 15.2.0

ESP32 only:
  Physical board: 未接続（build-only。この記録ではboardを一切使っていない）
  Module marking: この検証では未確認（board未接続のため）
  Board revision: 同上
  Rust target: xtensa-esp32-espidf
  espup version: 0.17.1
  cargo-generate version: 0.23.14
  ldproxy version: 0.3.5
  espflash version: 導入済み（4.5.0）だが本記録では未使用（flashを行っていない）
  ESP-IDF version: v5.5.3
  ESP-IDF source/commit: 2c211b236707889e8400c4dc5644dd5c4ee071e0
  ESP-IDF tools location mode: workspace外の既存.embuildをsymlinkで共有
    （main checkoutのfirmware/esp32/.embuildを、scratchpad上の
    origin/develop展開先`firmware/esp32/.embuild`へsymlinkし、再ダウンロード・再構築を避けた）
  IDF_PATH present: no（現在のshellでは未export。embuild crateが内部的に上記.embuild配下を解決する）
  IDF_TOOLS_PATH present: no
  Template repository: https://github.com/esp-rs/esp-idf-template
  Template commit: 未確認（本記録では確認していない）
  sdkconfig/defaults identity: 追跡中のfirmware/esp32/sdkconfig.defaultsを使用。本作業で変更していない
  USB-UART identity: 未確認（board未接続）

Commands run:
  . "$HOME/export-esp.sh"
  cargo fmt --all -- --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo build --locked
  cargo clippy --all-targets --locked --features bringup-display-13 -- -D warnings
  cargo build --locked --features bringup-display-13

Expected result: すべて成功する。warningを出さない。

Actual result:
  cargo fmt --all -- --check（既定構成）                                成功。差分なし
  cargo clippy --all-targets --locked -- -D warnings（既定構成）        成功。warning 0件
  cargo build --locked（既定構成）                                      成功
  cargo clippy --all-targets --locked --features bringup-display-13    成功。warning 0件
  cargo build --locked --features bringup-display-13                   成功

Build duration:
  cargo clippy（既定構成、ESP-IDF本体のcompileを含む）  約6分06秒
  cargo build（既定構成、clippy後）                     約49秒
  cargo clippy（+bringup-display-13、増分）             約3秒
  cargo build（+bringup-display-13、増分）              約7秒
Peak memory if measured: 未測定
Storage delta if measured:
  scratchpad上のtarget/xtensa-esp32-espidf: 未計測（既存.embuildをsymlinkしたため.embuild分は増加なし）
  build前後の`df -h /` Avail: 69G → 68G（1G未満の変動）
Generated artifact identity:
  target/xtensa-esp32-espidf/debug/deskcat-esp32（bringup-display-13構成、15,524,004 bytes）
  既定構成のartifactは同じtarget配下で上書きされたため個別のbyte数は記録していない
Log or evidence path: この記録本文

Known differences from documented profile:
  - **worktreeからは実行できない。**既存記録
    （[2026-08-15-esp32-build-native-linux.md](2026-08-15-esp32-build-native-linux.md)）と同じ理由
    （root Cargo.tomlのexcludeがworktree pathと不一致になりcargoがworkspaceを誤認する）。
    本記録では main checkout も使わず、`git archive origin/develop`で
    scratchpad（repo外）へ展開した独立コピーを使い、そこへ既存`.embuild`をsymlinkして実行した。
    main checkoutの作業branch（`develop`、origin/developより古い）を汚さないための選択である。
  - `pi-protocol-mode` featureとの排他構成（`compile_error!`）は本記録の対象外
    （[検証済みコマンド](../verified-commands.md)が定めるとおり意図した挙動であり検証済みcommandの対象外）。
  - `bringup-display-13`構成はflash・monitor・実機起動を一切含まない。
    [power-budget.md](../../hardware/power-budget.md)の`DISP-01`追加接続のbring-upの手順・条件(2)が要求するのは
    「firmwareがこのfeature付きでbuild済みであること」までであり、本記録はその範囲を満たす。

Conclusion: Partial。**build-onlyの範囲（既定構成＋`bringup-display-13`構成のfmt/clippy/build）では
  未実行の項目が無く、すべて成功した。**
  **`Partial`とするのは、flashと実機起動を行っていないためである。**board未接続のため
  `Physical board`／`Module marking`／`Board revision`／`USB-UART identity`は
  この検証では確認していない。ESP32へのUSB接続は一切行っていない
  （試験当日に人間が行う範囲であり、本記録はそれに先立つbuild確認に留まる）。

Next action: flashと実機起動は[power-budget.md](../../hardware/power-budget.md)の`DISP-01`追加接続のbring-upの手順に従い、
  ESP32 Flash / HIL profileの端末で人間の監視のもと実施した（結果は[EXP-016](../../hardware/experiment-log.md#exp-016-disp-01msp2807esp323v3-pin追加接続のbring-upcontroller識別単色fill四隅pattern)を参照）。
```
