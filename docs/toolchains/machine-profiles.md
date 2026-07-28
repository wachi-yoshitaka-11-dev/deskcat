# Machine Profiles

> 状態: Accepted policy
> 日付: 2026-07-27

DeskCat は複数端末で作業する。すべての端末へ同じツールを導入せず、担当する作業に必要な最小環境だけを用意する。

## Profile一覧

| Profile | 用途 | 必須要件 | 不要なもの |
|---|---|---|---|
| Docs / Review | 文書、Issue、差分、設計レビュー | Git、Markdown を扱えるエディタ | Rust、ESP-IDF、USB ドライバ、実機 |
| Host Rust Development | 純粋ロジック、Pi 側 crate、シミュレータ | Git、Rust stable、Cargo、linker、rustfmt、Clippy | ESP32 Xtensa toolchain、実機 |
| ESP32 Build | firmware 生成、format、lint、build | Host Rust 要件、`espup`、`ldproxy`、ESP-IDF の前提ツール | USB 接続は build-only なら不要 |
| ESP32 Flash / HIL | flash、serial monitor、実機検証 | ESP32 Build 要件、`espflash`、USB ドライバ、対象ボード、人間の監視 | 未確認の周辺回路やサーボ出力 |
| Raspberry Pi Runtime | `deskcatd` の実行、実機ログ | Pi Zero WH、対応 Raspberry Pi OS、Rust 実行物、Git または配布物 | ESP32 toolchain |
| Raspberry Pi Direct Build | Pi 上での最小 build 検証 | Runtime 要件、Rust stable、Cargo、native linker、空き容量 | Cross compiler |
| CI | 再現可能な自動検証 | 固定 runner、pin 済み action、秘密情報を使わない build | flash と無人実機駆動 |

一台の端末が複数 profile を兼ねてもよい。ただし、その場合も各 profile の検証結果を区別する。

## 作業開始時の判断

1. 今回の作業に必要な profile を選ぶ。
2. 端末がその profile を意図的に担うか確認する。
3. [Version Record Template](version-record-template.md) で既存環境を調べる。
4. 不足ツールの導入が必要なら、対象 Issue と人間の承認を確認する。
5. 文書だけで完了できる場合は、ツールを導入しない。

## Profile間で共有するもの

Git で共有する。

- toolchain 方針と採用版
- `rust-toolchain.toml` または toolchain 指定
- Cargo manifest とアプリケーションの lockfile
- ESP-IDF 選択と生成条件
- format、lint、test、build の正式コマンド
- 非秘密の設定例
- 実機試験手順と匿名化した証拠

Git で共有しない。

- `.env`
- access token、秘密鍵、Wi-Fi 認証情報
- 個人の絶対パス
- IDE の個人設定
- USB ポート名の固定値
- 端末固有キャッシュ
- 再配布権が不明な SDK、PDF、バイナリ

## 検証の移送

ある端末で成功したという記録だけでは、別端末の profile を検証済みにしない。別端末では最低限、次を再確認する。

- OS と CPU architecture
- toolchain と target
- linker と SDK
- repository commit
- lockfile が変更されていないこと
- clean build の結果

flash または実機試験を行う端末では、USB 接続、board identity、電源、安全制限も別途確認する。
