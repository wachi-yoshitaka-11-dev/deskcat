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

## 現在の実装

[Issue #491](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/491)の段階2bとして、libraryを作成した
（`src/`。2b-i）。接触eventから表情を決めてESP32へ送り、sessionのやり直しをprocessの終了で行う。
終了と再開の規則は`src/daemon.rs`のmodule docが持つ。

binary（`src/main.rs`）、stderrのlogger、引数、起動からの期限（watchdog）は2b-iiで足した。**引数は既定値を持たない**
（`--help`を参照）。

systemdのunitは`deploy/systemd/deskcatd.service`にある。値は暫定で、根拠は無い。Pi実機で未確認である。
