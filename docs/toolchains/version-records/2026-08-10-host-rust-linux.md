# Version Record: Host Rust Development (Linux x86_64)

様式は [Version Record Template](../version-record-template.md) に従う。

- Record ID: `2026-08-10-host-rust-linux`
- 判定: `Verified`
- 初回検証日: 2026-08-10
- 最終有効な検証日時: 2026-08-10

## 記録

```text
Record ID: 2026-08-10-host-rust-linux
Date: 2026-08-10
Machine profile: Host Rust Development
Operator role: 開発者（human）の監督下でのAI agent作業
Repository commit: e9bf2b29d6240c46146a857088709e693d5c399f
Working tree clean: no（本Issueの追加分を含む）

OS name: Ubuntu
OS version: 22.04.5 LTS (Jammy Jellyfish)
CPU architecture: x86_64
Userspace bitness: 64-bit
Container / VM / native: VM（systemd-detect-virt: microsoft）。container ではない

Rustup version: 1.29.0 (28d1352db 2026-03-05)
Rust channel: stable（root に rust-toolchain.toml は置いていない）
Rust compiler version: rustc 1.97.1 (8bab26f4f 2026-07-14)
Rust host: x86_64-unknown-linux-gnu
Installed Rust targets: x86_64-unknown-linux-gnu
Cargo version: cargo 1.97.1 (c980f4866 2026-06-30)
rustfmt version: rustfmt 1.9.0-stable (8bab26f4f6 2026-07-14)
Clippy version: clippy 0.1.97 (8bab26f4f6 2026-07-14)
Linker identity and version: cc (Ubuntu 11.4.0-1ubuntu1~22.04.3) 11.4.0

Commands run:
  cargo clean
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --locked
  cargo test --workspace --locked
  cargo metadata --format-version 1 --no-deps
  cargo metadata --manifest-path firmware/esp32/Cargo.toml --format-version 1 --no-deps

Expected result: すべて成功する。warning を出さない。firmware が host workspace の
  member にならない。

Actual result:
  cargo fmt --all -- --check        成功。差分なし
  cargo clippy ...                  成功。warning 0 件
  cargo test --workspace --locked   成功。12 件合格（0 件失敗）
    tests/conformance.rs  6 件
    tests/limits.rs       4 件
    doc-test              2 件
  cargo metadata (root)             workspace_root=<repo>、members=[deskcat-protocol]
  cargo metadata (firmware)         workspace_root=<repo>/firmware/esp32、
                                    members=[deskcat-esp32]

Build duration:
  cargo clippy（clean から）  5.7 秒
  cargo test（clippy 後）      3.7 秒
Peak memory if measured: 未測定
Storage delta if measured: clean 時に target/ から 175.5 MiB を削除
Generated artifact identity: なし（library crate と test binary のみ）
Log or evidence path: この記録本文

Known differences from documented profile:
  - Machine Profiles は標準OSを実機の Linux としている。この記録は VM 上の Linux で
    取得した。Machine Profiles は「USB を必要としない作業」に container / VM を
    認めており、host Rust の build・test・lint はこれに当たる。flash と実機試験は
    この端末で行わない。
  - この端末は ESP32 Build profile も兼ねる（esp toolchain が導入済み）。ただし本記録は
    host workspace だけを対象とし、ESP32 の build 結果は
    2026-08-06-esp32-build-linux.md を正とする。

Conclusion: Verified。host workspace の format、lint、unit test、integration test が
  clean build から成功した。firmware workspace の分離も確認した。

Next action: 別端末での再現は未実施。CI での自動実行は #42 の範囲。
```

## 補足

`cargo fmt` は `--locked` を受け付けないため、この option を付けていない。
lint の水準は root `Cargo.toml` の `[workspace.lints]` が持つため、
`cargo clippy` に `-D warnings` を付けていない。

`firmware/esp32` に対する `cargo metadata` は repository root を作業ディレクトリとして
実行したため、`firmware/esp32/rust-toolchain.toml` の override は適用されていない。
この確認が示すのは manifest の workspace 帰属だけであり、**ESP32 の build を再検証したものではない**。
ESP32 の build 検証は [2026-08-06-esp32-build-linux.md](2026-08-06-esp32-build-linux.md) を正とする。
