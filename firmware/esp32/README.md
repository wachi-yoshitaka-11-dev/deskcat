# ESP32 firmware

このディレクトリには、ESP32-DevKitC-32E用のRust firmwareを置く。

専用のESP-IDF/Xtensa toolchainを使用するため、rootのhost workspaceとは別のCargo workspaceとする。

責務:

- LCD
- touch、加速度、環境sensor
- サーボPWMと強制安全上限
- JSON Lines通信
- watchdog、reset reason、診断、fail-safe動作

現在の公式情報調査、target候補、version選定条件は次に記載する。

- [ESP32 Rust toolchain](../../docs/toolchains/esp32-rust-toolchain.md)
- [ESP32開発端末セットアップ](../../docs/runbooks/esp32-development-machine-setup.md)

これらの候補を検証済みの固定版として扱わない。manifestは、M1-001においてESP32 Build profile端末でのみ作成する。生成fileをreviewし、applicationの`Cargo.lock`を保持し、clean buildを記録する。
