# 開発Toolchain

このディレクトリは、DeskCatを複数端末で開発するための環境要件、採用候補、検証結果のSingle Source of Truthである。

## 現在の状態

> 調査日: 2026-07-27
> 標準OS判断: 2026-08-06（[ADR-0005](../decisions/0005-standard-development-os.md)）
> ESP32 build検証日: 2026-08-06（初回）／2026-08-11（現行treeに対する最新の検証。開発端末は`2026-08-11T11:13:22Z`、CIは`2026-08-10T06:26:26Z`）

- 開発環境の標準OSは実機のLinuxで、Windowsは対象外。distributionとarchitectureは未固定
- USBを必要としない作業ではDocker上のLinuxも使う。flashと実機試験は実機Linuxに限る
- ESP32とRaspberry Piの候補toolchainは公式資料に基づいて整理済み
- **ESP32の最小buildは、Linux x86_64のESP32 Build profile端末で成功した**（[Version Record](version-records/2026-08-06-esp32-build-linux.md)）。version、生成条件、lockfileを固定済み
- **別環境での再現は実施済み。**GitHub Actionsの`ubuntu-24.04` runnerがclean環境でbuildを再現した（[CIのVersion Record](version-records/2026-08-10-esp32-build-ci.md)、#42／PR #86）。**開発端末以外の実機での再現は未実施である**
- **ESP32の実機確認（物理基板・module・revision）は未実施。**CIでの再現はこれを代替しない
- Windowsは[ADR-0005](../decisions/0005-standard-development-os.md)により対象外。support対象ではないため「未検証」ではない
- **Raspberry Pi Zero W実機での最小Rust programのdirect buildは成功した**（[Version Record](version-records/2026-08-17-pi-direct-build-native.md)。判定は`Partial`）。候補target`arm-unknown-linux-gnueabihf`はこのとき確定した。**ただし成功したのは依存0件の最小programだけである。****このrepositoryのcrateとworkspaceのbuild、および依存を持つbuildはPi上で未測定**であり、host workspaceの検証済みcommandがPiで通るかは不明である。cross compilationは保留を維持している
- 実行結果が得られるまで、記載したversionを「検証済み」または「確定」と扱わない

## 文書

| 文書 | 役割 |
|---|---|
| [Machine Profiles](machine-profiles.md) | 複数端末の役割と、役割ごとの最小要件 |
| [ESP32 Rust Toolchain](esp32-rust-toolchain.md) | ESP32 向けの公式情報、採用候補、確定条件 |
| [Raspberry Pi Rust Toolchain](raspberry-pi-rust-toolchain.md) | Pi Zero W 向けの候補 target と検証方針 |
| [Version Record Template](version-record-template.md) | 開発端末ごとの再現可能な環境記録 |
| [Version Records](version-records/README.md) | 実際の端末で確認した環境記録 |

実際の作業手順は次を参照する。

- [ESP32 開発端末セットアップ](../runbooks/esp32-development-machine-setup.md)
- [Raspberry Pi 開発端末セットアップ](../runbooks/raspberry-pi-development-machine-setup.md)

## 状態ラベル

| 状態 | 意味 |
|---|---|
| 調査済み | 公式資料または公式リポジトリで存在と用途を確認した |
| 採用候補 | DeskCat へ適合すると判断したが、対象端末で未検証 |
| build検証済み | 記録された一台の端末で build-only のコマンドが成功した。実機確認または別端末での再現が未了 |
| 検証済み | 記録された端末と手順で、必要なコマンドが成功した |
| 確定 | 検証結果をレビューし、リポジトリの設定と lockfile に反映した |
| TBD | 追加の公式情報、実機確認、または判断が必要 |

`build検証済み` は `採用候補` と `検証済み` の間に置く。build は通るが、実機または別端末の裏づけが無い状態を、両者と混同しないために分ける。

調査時点の最新版と、DeskCat の採用版は同じ意味ではない。最新版を自動的に採用せず、互換性と再現性を確認する。

## 更新規則

- 開発端末へツールを導入する前に、その端末が担う [Machine Profile](machine-profiles.md) を決める。
- 文書確認専用端末へ Rust、ESP-IDF、USB ドライバを導入する必要はない。
- 実行した端末では [Version Record Template](version-record-template.md) を埋める。
- ESP32 workspace 生成後はアプリケーションの `Cargo.lock` を追跡する。
- `IDF_PATH` などの環境変数が選択した SDK を上書きしていないか記録する。
- ツールチェーン変更は独立した Issue とし、生成設定、lockfile、ビルド結果を同時にレビューする。
- 個人名、端末名、ユーザーディレクトリ、資格情報、シリアル番号は公開文書へ記録しない。

## 公式情報

- [The Rust on ESP Book](https://docs.espressif.com/projects/rust/book/)
- [esp-rs/esp-idf-template](https://github.com/esp-rs/esp-idf-template)
- [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
- [rustup](https://rustup.rs/)
- [Raspberry Pi documentation](https://www.raspberrypi.com/documentation/)
