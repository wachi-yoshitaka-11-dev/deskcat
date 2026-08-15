# Version Record: ESP32 Build (実機 Linux x86_64)

様式は [Version Record Template](../version-record-template.md) に従う。

- Record ID: `2026-08-15-esp32-build-native-linux`
- 判定: `Partial`
- 初回検証日: 2026-08-15
- 最終有効な検証日時: 2026-08-15

**この記録は[2026-08-06-esp32-build-linux.md](2026-08-06-esp32-build-linux.md)を置き換えない。**
同記録は VM 上で取得した。本記録は**実機**である。`Container / VM / native:` が異なるため、
[README](README.md) の「一つの記録は、一台の端末と一つの profile に対応させる」に従い別記録とする。

**同じ端末の Host Rust Development profile は
[2026-08-15-host-rust-native-linux.md](2026-08-15-host-rust-native-linux.md) が持つ。**
profile が違うため記録を分けている。

## 記録

```text
Record ID: 2026-08-15-esp32-build-native-linux
Date: 2026-08-15
Machine profile: ESP32 Build
Operator role: 開発者（human）の監督下でのAI agent作業。tool導入はhumanの確認を得た
Repository commit: f69be9ec512891319f2d6ceeda60c7f7fba6f83c
Working tree clean: yes（firmware/esp32 は本作業で変更していない）

OS name: Ubuntu
OS version: 24.04.4 LTS
Kernel: 7.0.0-28-generic
CPU architecture: x86_64
Userspace bitness: 64-bit
Container / VM / native: native（実機）。systemd-detect-virt: none

Rustup version: rustup 1.29.0 (28d1352db 2026-03-05)
Rust channel: esp-1.95.0.0（firmware/esp32/rust-toolchain.toml が固定）
Rust compiler version: rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
Rust host: x86_64-unknown-linux-gnu
Installed Rust targets: xtensa-esp32-espidf（esp toolchain が同梱）
Cargo version: cargo 1.95.0-nightly (f2d3ce0bd 2026-03-21) (1.95.0.0)
rustfmt version: esp toolchain 同梱版
Clippy version: esp toolchain 同梱版
Linker identity and version: cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
  （host 側。Xtensa 側は esp toolchain と xtensa-esp-elf GCC を使う）

ESP32 only:
  Physical board: 未接続（build-only のため、この検証では board を使っていない）
  Module marking: この検証では未確認（board 未接続のため）。
    現物の識別自体は [#122](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/122) で
    確定しており、`HW-TBD-001` は close 済みである。
  Board revision: 同上
  Rust target: xtensa-esp32-espidf
  espup version: 0.17.1
  cargo-generate version: 0.23.14
  ldproxy version: 0.3.5（`cargo install ldproxy --version 0.3.5 --locked` で導入）
  espflash version: 未導入（ESP32 Build profile では不要）
  ESP-IDF version: v5.5.3
  ESP-IDF source/commit: 2c211b236707889e8400c4dc5644dd5c4ee071e0
  ESP-IDF tools location mode: workspace（firmware/esp32/.embuild 配下）
  IDF_PATH present: no
  IDF_TOOLS_PATH present: no
  Template repository: https://github.com/esp-rs/esp-idf-template
  Template commit: 08115a069d167a5ee37363e84f168a565f17bbca
  sdkconfig/defaults identity: 追跡中の firmware/esp32/sdkconfig.defaults を使用。本作業で変更していない
  USB-UART identity: 未確認（build-only のため board 未接続）

Commands run:
  . "$HOME/export-esp.sh"
  cargo fmt --all -- --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo build --locked

Expected result: すべて成功する。warning を出さない。

Actual result:
  cargo fmt --all -- --check                      成功。差分なし
  cargo clippy --all-targets --locked -D warnings 成功。warning 0 件
  cargo build --locked                            成功

Build duration:
  cargo clippy（ESP-IDF 本体の compile を含む初回）  16m27s
  cargo build（clippy 後）                           1m01s
Peak memory if measured: 未測定
Storage delta if measured:
  firmware/esp32/target: 1.4 GB
  firmware/esp32/.embuild: 4.4 GB（ESP-IDF と managed tool 一式。git 管理外）
Generated artifact identity:
  target/xtensa-esp32-espidf/debug/deskcat-esp32（13,660,584 bytes）
Log or evidence path: この記録本文

Known differences from documented profile:
  - **worktree からは実行できない。**この端末の作業は git worktree
    （repository root 配下）で行っているが、そこから `firmware/esp32` を build すると
    `cargo` が workspace の解決に失敗する。**本記録の測定は main checkout で行った。**
    詳細は下の「worktree から実行できない理由」。
  - この端末は Host Rust Development profile も兼ねる。そちらは
    [2026-08-15-host-rust-native-linux.md](2026-08-15-host-rust-native-linux.md) が持つ。
  - `espup` と Xtensa toolchain は本作業で新規導入した。human の確認を得ている
    （AGENTS.md「ツール導入は、対象 Issue、端末 profile、人間の確認が揃った
    開発端末だけで行う」）。

Conclusion: Partial。**build-only の範囲では未実行の項目が無く、format、lint、build が成功した。**
  ESP-IDF v5.5.3 の pin も効いている。

  **`Partial` とするのは、flash と実機起動を行っていないためである。**
  board を接続していないため、`Physical board`／`Module marking`／`Board revision`／
  `USB-UART identity` を**この検証では確認していない。**
  **ただしこれは board の識別が未確定という意味ではない。**現物の識別は
  [#122](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/122) で確定し、
  `HW-TBD-001` は close 済みである。**本記録が確認していないのは、
  この build に board を使っていないという事実にとどまる。**
  **CLAUDE.md が「build-only であり、flash と実機起動は主張しない」と定めるとおりである。**

Next action: flash と実機起動は ESP32 Flash / HIL profile の範囲であり、別記録が必要である。
```

## 補足

### 版が既存記録と一致する

`espup` 0.17.1、Xtensa Rust 1.95.0.0、ESP-IDF v5.5.3（commit `2c211b23…`）はいずれも
[2026-08-06-esp32-build-linux.md](2026-08-06-esp32-build-linux.md) と同じ値である。
`firmware/esp32/rust-toolchain.toml` と `.cargo/config.toml` が版を固定しているため、
**別端末でも同じ版が入ることを実機で確認したことになる。**

### worktree から実行できない理由

`firmware/esp32` を git worktree の中から build すると `cargo` が失敗する。

```text
current package believes it's in a workspace when it's not:
current:   <repo>/.claude/worktrees/<name>/firmware/esp32/Cargo.toml
workspace: <repo>/Cargo.toml
```

root `Cargo.toml` の `exclude = ["firmware/esp32"]` は**その manifest からの相対 path** である。
worktree が repository root の配下に置かれると、`cargo` は上位へ探索して
**main checkout の `Cargo.toml`** を workspace root と判定するが、`exclude` の値は
worktree 側の path に一致しない。`firmware/esp32/Cargo.toml` は `[workspace]` 節を
持たないため、自分自身を workspace root にすることもできない。

**したがって ESP32 の build 検証は main checkout で実行する。**
host workspace 側は worktree からでも問題なく実行できる。
