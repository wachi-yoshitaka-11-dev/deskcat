# Version Record: Host Rust Development (実機 Linux x86_64)

様式は [Version Record Template](../version-record-template.md) に従う。

- Record ID: `2026-08-15-host-rust-native-linux`
- 判定: `Verified`
- 初回検証日: 2026-08-15
- 最終有効な検証日時: 2026-08-15

**この記録は[2026-08-10-host-rust-linux.md](2026-08-10-host-rust-linux.md)を置き換えない。**
同記録は VM 上で取得したものであり、本記録は**実機**で取得した。
`Container / VM / native:` が異なるため、[README](README.md) の
「一つの記録は、一台の端末と一つの profile に対応させる」に従い別記録とする。

**実機 Linux で取得した version record は、profile を問わずこれが初めてである。**
既存 3 記録の `Container / VM / native:` はいずれも `VM` である
（[ESP32 Build](2026-08-06-esp32-build-linux.md)、[CI](2026-08-10-esp32-build-ci.md)、
[Host Rust](2026-08-10-host-rust-linux.md)）。
[ADR-0005](../../decisions/0005-standard-development-os.md) は実機 Linux を標準と定めているが、
**その標準環境で取得した記録が 1 件も無かった。**

## 記録

```text
Record ID: 2026-08-15-host-rust-native-linux
Date: 2026-08-15
Machine profile: Host Rust Development
Operator role: 開発者（human）の監督下でのAI agent作業。tool導入はhumanの確認を得た
Repository commit: f69be9ec512891319f2d6ceeda60c7f7fba6f83c
Working tree clean: no（本記録の追加分を含む）

OS name: Ubuntu
OS version: 24.04.4 LTS
Kernel: 7.0.0-28-generic
CPU architecture: x86_64
Userspace bitness: 64-bit
Container / VM / native: native（実機）。systemd-detect-virt: none。
  containerでもVMでもない

Rustup version: rustup 1.29.0 (28d1352db 2026-03-05)
Rust channel: stable（root に rust-toolchain.toml は置いていない）
Rust compiler version: rustc 1.97.1 (8bab26f4f 2026-07-14)
Rust host: x86_64-unknown-linux-gnu
Installed Rust targets: x86_64-unknown-linux-gnu
Cargo version: cargo 1.97.1 (c980f4866 2026-06-30)
rustfmt version: rustfmt 1.9.0-stable (8bab26f4f6 2026-07-14)
Clippy version: clippy 0.1.97 (8bab26f4f6 2026-07-14)
Linker identity and version: cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0

Commands run:
  cargo clean
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --locked
  cargo test --workspace --locked

Expected result: すべて成功する。warning を出さない。

Actual result:
  cargo fmt --all -- --check        成功。差分なし
  cargo clippy ...                  成功。warning 0 件
  cargo test --workspace --locked   成功。13 件合格（0 件失敗）
    unittests src/lib.rs   0 件
    tests/conformance.rs   6 件
    tests/limits.rs        5 件
    doc-test               2 件

Build duration:
  cargo clippy（clean から）  13.8 秒
  cargo test（clippy 後）      9.9 秒
Peak memory if measured: 未測定
Storage delta if measured: clean build 後の target/ は 133 MiB
Generated artifact identity: なし（library crate と test binary のみ）
Log or evidence path: この記録本文

Known differences from documented profile:
  - この端末は導入時点で Rust を持っていなかった。
    [#114](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/114) の作業中に
    human の確認を得て rustup 経由の stable と build-essential を導入した
    （AGENTS.md「ツール導入は、対象 Issue、端末 profile、人間の確認が揃った
    開発端末だけで行う」）。
  - **この端末は ESP32 Build profile も兼ねる**（`espup` と Xtensa toolchain
    `esp-1.95.0.0` を導入済み）。**ただし本記録は host workspace だけを対象とする。**
    ESP32 の検証は別記録が持つ。
  - この端末には内蔵 SDHCI card reader があり、
    [sd-health-check.md](../../hardware/sd-health-check.md) の実機試験を同じ端末で行った。
    **本記録はその試験の環境記録ではない。**profile が違うためである。

Conclusion: Verified。**この記録の対象は、この 1 台・Host Rust Development profile に
  限る。**その範囲では未実行の項目が無く、format、lint、unit test、integration test が
  clean build から成功した。

  **実機 Linux で取得した点が既存記録との違いである。**既存 3 記録
  （2026-08-06-esp32-build-linux、2026-08-10-esp32-build-ci、2026-08-10-host-rust-linux）
  はいずれも `Container / VM / native:` が `VM` である。

Next action: ESP32 Build profile の記録は別途必要である（本記録の対象外）。
```

## 補足

`cargo fmt` は `--locked` を受け付けないため、この option を付けていない。
lint の水準は root `Cargo.toml` の `[workspace.lints]` が持つため、
`cargo clippy` に `-D warnings` を付けていない。

### worktree からは ESP32 build を検証できない

**この端末で確認した制約である。**`firmware/esp32` を git worktree の中から build しようとすると、
`cargo` が workspace の解決に失敗する。

```text
current package believes it's in a workspace when it's not:
current:   <repo>/.claude/worktrees/<name>/firmware/esp32/Cargo.toml
workspace: <repo>/Cargo.toml
```

root `Cargo.toml` の `exclude = ["firmware/esp32"]` は**そのmanifestからの相対path**である。
worktree が repository root の配下に置かれると、`cargo` は上位へ探索して
**main checkout の `Cargo.toml`** を workspace root と判定するが、
`exclude` の値は worktree 側の path に一致しない。
`firmware/esp32/Cargo.toml` は `[workspace]` 節を持たないため、自分自身を
workspace root にすることもできない（この設計の理由は root `Cargo.toml` の注記にある）。

**したがって ESP32 の build 検証は main checkout で実行する。**
host workspace 側は worktree からでも問題なく実行できる（本記録はworktreeから取得した）。
