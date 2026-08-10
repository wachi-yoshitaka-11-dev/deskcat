# deskcatd

`deskcatd`は、Raspberry Pi Zero W（V1.1）**向けに設計する**Rustサービスである。
**実機での動作は未検証である。**検証状態の正本は
[Raspberry Pi Rust Toolchain](../../docs/toolchains/raspberry-pi-rust-toolchain.md)であり、
同文書は状態を「調査済み。Raspberry Pi実機は未検証」としている。

責務:

- ESP32とのシリアル接続と状態同期
- 感情・性格状態の管理
- 行動のスケジューリング
- コマンド生成
- アイドル時の独り言
- 設定、API、ストレージの統合

toolchainと最初のIssueが準備できた時点でpackageを作成する。このREADMEでは、空のmanifestを先行作成せずに責務境界だけを定義する。
