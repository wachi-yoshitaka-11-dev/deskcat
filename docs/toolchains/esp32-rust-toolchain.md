# ESP32 Rust Toolchain

> 状態: 調査済み。採用候補はbuild未検証
> 調査日: 2026-07-27
> 対象family: classic ESP32／Xtensa

## 結論

DeskCat の ESP32 firmware は、`std` を利用できる ESP-IDF ベースの Rust 構成を採用候補とする。classic ESP32 は Xtensa であるため、upstream Rust の通常 target だけではなく、`espup` が導入する Espressif Rust toolchain を使用する。

現時点の candidate target は次である。

```text
xtensa-esp32-espidf
```

リポジトリでは対象ボードを ESP32-DevKitC-32E としている。ただし、物理基板の module、board revision、USB-UART 構成は未確認である。firmware を生成する前に実物とメーカー資料で一致を確認する。

## 調査した公式構成

2026-07-27 時点の [esp-idf-template](https://github.com/esp-rs/esp-idf-template) を次の commit で確認した。

```text
08115a069d167a5ee37363e84f168a565f17bbca
```

[確認した commit](https://github.com/esp-rs/esp-idf-template/tree/08115a069d167a5ee37363e84f168a565f17bbca) の template には次が記載されている。

| 項目 | Template の値 | DeskCat での扱い |
|---|---|---|
| MCU | `esp32` を選択可能 | 採用候補 |
| Rust target | `xtensa-esp32-espidf` | 採用候補 |
| Rust channel | `esp` | 採用候補 |
| ESP-IDF | `v5.5.3` が default | 採用候補 |
| Rust edition | `2021` | 生成時に再確認 |
| Minimum Rust | `1.82` | crate manifest の下限。実際の compiler 版とは別に記録 |
| `esp-idf-svc` | `0.52.1` | Template の候補。build 後に lock |
| `embuild` | `0.33` | Template の候補。build 後に lock |
| linker | `ldproxy` | 必須候補 |
| runner | `espflash flash --monitor` | flash Issue で検証 |
| ESP-IDF tools location | workspace | 再現性のため採用候補 |

これは調査時点の template 内容であり、DeskCat の確定済み dependency 一覧ではない。

## 生成条件の候補

最初の生成では、対話項目を次のように明示する。

| 質問 | 候補 |
|---|---|
| MCU | `esp32` |
| Advanced options | `true` |
| ESP-IDF version | `v5.5.3` |
| Git crates | `false` |
| ESP-IDF tools install directory | `workspace` |
| Wokwi | `false` |
| Dev Container | `false` |
| CI | `false` |

理由:

- 最初の Issue では変数を減らし、公式 release の crate を使う。
- SDK を workspace 管理にして、端末全体の外部 `IDF_PATH` 依存を減らす。
- simulator、container、CI は最小 build の成立後に独立して判断する。
- `master` の ESP-IDF は再現性と公式サポートの観点から選ばない。

## 必要なツール

### Host 共通

- Git
- Python と venv
- C/C++ build prerequisites
- Rustup と Cargo

### Rust on ESP

- `cargo-generate`
- `espup`
- `ldproxy`
- `espflash` は flash、monitor を行う端末だけで必須
- `cargo-espflash` は任意。初期手順では command surface を増やさない

Windows では Rust の MSVC host toolchain と Visual Studio C++ Build Tools を候補とする。Unix 系では、ESP-IDF が公式に列挙する OS package と、`espup` が出力する environment export が必要になる。

詳細は [ESP32 開発端末セットアップ](../runbooks/esp32-development-machine-setup.md) を参照する。

## 再現性の方針

- Template は branch 名ではなく、レビューした commit を記録して生成する。
- 生成後の application `Cargo.lock` は追跡する。
- 生成された `rust-toolchain.toml`、`.cargo/config.toml`、`sdkconfig.defaults` をレビューする。
- Template が `Cargo.lock` を ignore する場合、application 方針に合わせて ignore を外す。
- `IDF_PATH` が設定されていると template で選んだ ESP-IDF より優先されるため、build 記録へ有無を残す。
- `IDF_TOOLS_PATH` と `ESP_IDF_TOOLS_INSTALL_DIR` の意味を混同しない。
- `cargo install` した補助ツールは `--version` の出力を保存する。
- CI action の version は、CI を導入する Issue で別途 review して commit SHA に pin する。

## 確定条件

次を満たすまで状態を`Verified`または`Accepted`に変更しない。

- [ ] 物理基板が ESP32-DevKitC-32E であり、搭載 module と revision を確認した
- [ ] 開発端末の profile と version record を作成した
- [ ] レビュー済み template commit から最小 project を生成した
- [ ] 環境変数による意図しない SDK override がない
- [ ] clean `cargo build` が成功した
- [ ] dependency と lockfile を review した
- [ ] 正式な format、lint、build command を `AGENTS.md` と root README へ反映した
- [ ] 別の開発端末または clean environment で再現した

flash、serial monitor、実機起動は #6 の範囲であり、この文書の build-only 確定条件には含めない。

## 公式資料

- [The Rust on ESP Book: Getting Started](https://docs.espressif.com/projects/rust/book/getting-started/index.html)
- [The Rust on ESP Book: Toolchain Installation](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html)
- [esp-rs/esp-idf-template](https://github.com/esp-rs/esp-idf-template)
- [ESP-IDF Programming Guide: Get Started](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/get-started/)
- [Rustup installation](https://rustup.rs/)
