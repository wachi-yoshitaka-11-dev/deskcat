# ESP32 firmware

このディレクトリには、ESP-WROOM-32D開発ボード（秋月電子 M-13628）用のRust firmwareを置く。基板裏面silkscreenは`ESP32_DevkitC_V4`である。

専用のESP-IDF/Xtensa toolchainを使用するため、rootのhost workspaceとは別のCargo workspaceとする。

責務:

- LCD
- touch、加速度、環境sensor
- サーボPWMと強制安全上限
- JSON Lines通信
- watchdog、reset reason、診断、fail-safe動作

## 現在の状態

Issue #5 でtoolchainを固定し、最小projectのclean buildを確認した。実装済みなのは`link_patches()`、logger初期化、起動logの出力だけであり、hardware driverは未実装である。

| 項目 | 確定版 |
|---|---|
| Crate | `deskcat-esp32` |
| Rust target | `xtensa-esp32-espidf` |
| Rust channel | `esp-1.95.0.0`（Xtensa Rust 1.95.0.0） |
| ESP-IDF | `v5.5.3`（tools install dirは`workspace`） |
| linker | `ldproxy` |

採用根拠と確定条件は[ESP32 Rust toolchain](../../docs/toolchains/esp32-rust-toolchain.md)、導入手順は[ESP32開発端末セットアップ](../../docs/runbooks/esp32-development-machine-setup.md)、実測環境は[Version Record](../../docs/toolchains/version-records/2026-08-06-esp32-build-linux.md)にある。

## Build

ESP32 Build profileの端末で、このディレクトリにて実行する。

```bash
. "$HOME/export-esp.sh"
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
```

`--locked`は、追跡している`Cargo.lock`から解決結果が逸脱した場合に失敗させる。`cargo fmt`はこのoptionを受け付けない。

`export-esp.sh`を読み込まないと失敗する。`ESP_IDF_TOOLS_INSTALL_DIR=workspace`のため、ESP-IDF本体とmanaged toolは`.embuild/`（約4.4 GB）へ展開される。`.embuild/`と`target/`は`.gitignore`で除外し、applicationの`Cargo.lock`は追跡する。

環境に`IDF_PATH`が設定されていると、`.cargo/config.toml`で選んだESP-IDF versionを上書きする。`[env]`の`force = true`はこの変数を保護できないため、**`build.rs`が設定を検出してbuildを止める。**意図した外部SDKを使う場合だけ`DESKCAT_ALLOW_EXTERNAL_IDF_PATH=1`（値は厳密に`1`。`0`や`false`では通らない）を設定して通し、出力される`cargo:warning`をVersion Recordの`IDF_PATH present`へ記録する。

## 未確定の前提

機種と搭載moduleは確定している（ESP-WROOM-32D開発ボード／秋月電子 M-13628。基板にrevision表示は無い）。根拠は[hardware-bom.md](../../docs/hardware/hardware-bom.md)のMCU-01である。

未確定なのは**回路図と現物pin表記の照合**である（[HW-TBD-001](../../docs/hardware/tbd-register.md)）。秋月独自基板のため、pin配列がEspressif ESP32-DevKitC V4と完全一致する保証が無い。GPIO割り当てを伴う変更は、この照合が済むまで入れない。chip刻印も未読である。

ESP-WROOM-32D datasheet v2.7にはPSRAM内蔵variantの記載が無いため、PSRAMを前提とする設定は不要である。

flash、serial monitor、実機起動は#6の範囲である。
