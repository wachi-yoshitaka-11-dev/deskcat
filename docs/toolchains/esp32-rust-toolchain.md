# ESP32 Rust Toolchain

> 状態: build検証済み（Linux x86_64 の ESP32 Build profile 端末）。実機確認と別端末での再現は未実施
> 調査日: 2026-07-27
> build 検証日: 2026-08-06（初回）／2026-08-08（現行 tree に対する最新の検証）
> 証拠: [Version Record](version-records/2026-08-06-esp32-build-linux.md)
> 対象family: classic ESP32／Xtensa

## 結論

DeskCat の ESP32 firmware は、`std` を利用できる ESP-IDF ベースの Rust 構成を採用する。classic ESP32 は Xtensa であるため、upstream Rust の通常 target だけではなく、`espup` が導入する Espressif Rust toolchain を使用する。

採用する target は次である。

```text
xtensa-esp32-espidf
```

### board から target を決めた根拠

| 段階 | 内容 | 出典 |
|---|---|---|
| board | ESP-WROOM-32D 開発ボード（秋月電子 M-13628）。基板裏面 silkscreen は `ESP32_DevkitC_V4` で、Espressif ESP32-DevKitC V4 のリファレンスデザインに基づく秋月独自基板である。Espressif 公式の ESP32-DevKitC ではない | [秋月商品ページ M-13628](https://akizukidenshi.com/catalog/g/g113628/)（データシート添付）。確定根拠は [hardware-bom.md](../hardware/hardware-bom.md) の MCU-01 |
| module | ESP-WROOM-32D。中核は `ESP32-D0WD`。Xtensa dual-core 32-bit LX6 | [ESP32-WROOM-32D & ESP32-WROOM-32U Datasheet v2.7](https://documentation.espressif.com/esp32-wroom-32d_esp32-wroom-32u_datasheet_en.pdf)（2026-08-08 取得） |
| target | MCU `esp32` → arch `xtensa` / rust target `xtensa-esp32-espidf` / gcc target `xtensa-esp32-elf` | [esp-idf-template `08115a06`](https://github.com/esp-rs/esp-idf-template/blob/08115a069d167a5ee37363e84f168a565f17bbca/cargo/pre-script.rhai) の `pre-script.rhai`（2026-08-06 取得） |

同 template は MCU `esp32` の Wokwi board を `board-esp32-devkit-c-v4` と定義しており、対象 board と MCU 選択の対応を裏づける。

本 board は Espressif 公式の ESP32-DevKitC ではないが、silkscreen が示すとおり同じ V4 リファレンスデザインに基づき、搭載 module も classic ESP32 系である。したがって MCU 選択と target は同一になる。

ESP-WROOM-32D の datasheet v2.7 には **PSRAM を内蔵する variant の記載が無い**。同 datasheet が挙げる variant の差は antenna（PCB antenna の 32D と外部 connector の 32U）だけである。したがって PSRAM の有無による構成差は本 board では生じない。

現物確認により、module 種別（購入履歴と基板裏面 silkscreen `ESP32_DevkitC_V4`）と、基板に revision 表示が無いことは確定している。

[HW-TBD-001](../hardware/tbd-register.md) は範囲が縮小し、残る追跡対象は **board 回路図と現物 pin 表記の照合**だけである。**pin 配列が Espressif ESP32-DevKitC V4 と完全に一致する保証は無い**ため、秋月電子の独自基板である本 board では GPIO 割り当ての前に照合する。

これとは別に、**chip の刻印は未読である**（現物写真が反射で判読不能）。搭載 module が ESP-WROOM-32D であることは購入履歴と silkscreen で確定しており、その datasheet が示す中核 chip は `ESP32-D0WD` である。刻印の読み取りは TBD 台帳の追跡対象にはなっていない。

いずれも build には影響しないが、flash 後の実機動作には影響する。

## 調査した公式構成

2026-07-27 時点の [esp-idf-template](https://github.com/esp-rs/esp-idf-template) を次の commit で確認した。

```text
08115a069d167a5ee37363e84f168a565f17bbca
```

[確認した commit](https://github.com/esp-rs/esp-idf-template/tree/08115a069d167a5ee37363e84f168a565f17bbca) の template には次が記載されている。

| 項目 | Template の値 | DeskCat での確定版 |
|---|---|---|
| MCU | `esp32` を選択可能 | `esp32` |
| Rust target | `xtensa-esp32-espidf` | `xtensa-esp32-espidf` |
| Rust channel | `esp` | `esp`（Xtensa Rust 1.95.0.0） |
| ESP-IDF | `v5.5.3` が default | `v5.5.3`（commit `2c211b236707889e8400c4dc5644dd5c4ee071e0`） |
| Rust edition | `2021` | `2021` |
| Minimum Rust | `1.82` | manifest の下限は `1.82`。build に使用した compiler は Xtensa Rust 1.95.0.0 |
| `esp-idf-svc` | `0.52.1` | `0.52.1` |
| `embuild` | `0.33` | `0.33.3` |
| linker | `ldproxy` | `ldproxy` 0.3.5 → `xtensa-esp-elf-gcc` 15.2.0 |
| runner | `espflash flash --monitor` | 未検証。flash Issue（#6）で確認する |
| ESP-IDF tools location | workspace | workspace（`firmware/esp32/.embuild`） |

`esp-idf-hal` 0.46.2、`esp-idf-sys` 0.37.2、`embassy-time` 0.5.1 を含む **184 個の依存**の確定版は `firmware/esp32/Cargo.lock` にあり、主要なものは [Version Record](version-records/2026-08-06-esp32-build-linux.md) にも記載している。`Cargo.lock` の `[[package]]` は、root crate `deskcat-esp32` 自身を含むため 185 entry になる。

`runner` の `espflash flash --monitor` だけは build-only 検証の対象外であり、検証済み command として扱わない。

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

2026-08-06 の生成では、この表のとおりの入力で `firmware/esp32` を作成し、clean build まで確認した。以後の再生成でも同じ入力を使う。

理由:

- 最初の Issue では変数を減らし、公式 release の crate を使う。
- SDK を workspace 管理にして、端末全体の外部 `IDF_PATH` 依存を減らす。
- simulator、container、CI は最小 build の成立後に独立して判断する。
- `master` の ESP-IDF は再現性と公式サポートの観点から選ばない。

なお、この表の値は上記 template commit における各項目の default と一致する。2026-08-06 時点の [ESP-IDF stable ドキュメント](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/get-started/index.html)は v6.0.2 を指していたが、review 済み commit が提示する `v5.5.3` を採用した。最新版を自動的に採用しない方針（[更新規則](README.md#更新規則)）に従う。この stable の値は時間とともに変わるため、比較する場合は再取得して日付とともに記録する。

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

host は [ADR-0005](../decisions/0005-standard-development-os.md) の標準OS である実機 Linux を対象とする。ESP-IDF が公式に列挙する OS package と、`espup` が出力する environment export が必要になる。

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

- [x] 物理基板の機種と搭載 module を確認した（ESP-WROOM-32D 開発ボード／秋月電子 M-13628。基板に revision 表示は無い）
- [ ] chip 刻印を読み取った（現物写真が反射で判読不能）
- [ ] 回路図と現物 pin 表記を照合した
- [x] 開発端末の profile と version record を作成した
- [x] レビュー済み template commit から最小 project を生成した
- [x] 環境変数による意図しない SDK override がない
- [x] clean `cargo build` が成功した
- [x] dependency と lockfile を review した
- [x] 正式な format、lint、build command を `AGENTS.md` と root README へ反映した
- [ ] 別の開発端末または clean environment で再現した

未達の 3 項目により、この文書の状態は `検証済み`（`Verified`）ではなく **`build検証済み`** にとどめる。語の定義は [状態ラベル](README.md#状態ラベル) を参照する。

- **chip 刻印の読み取り**: 現物写真が反射で判読不能。搭載 module は確定しており、その datasheet が中核 chip を示すため、build への影響は無い。
- **回路図と現物 pin 表記の照合**: [HW-TBD-001](../hardware/tbd-register.md) として追跡し、#6 の flash 前提条件でもある。物理基板の機種と搭載 module 自体は現物確認で確定済みである。
- **別端末での再現**: 本記録は Linux x86_64 の 1 台のみである。[Machine Profiles](machine-profiles.md) の「検証の移送」に従い、別端末では OS、toolchain、linker、commit、lockfile、clean build を再確認する。標準OSは [ADR-0005](../decisions/0005-standard-development-os.md) により実機 Linux であり、Windows は対象外のため再現対象に含めない。

flash、serial monitor、実機起動は #6 の範囲であり、この文書の build-only 確定条件には含めない。

## 公式資料

- [The Rust on ESP Book: Getting Started](https://docs.espressif.com/projects/rust/book/getting-started/index.html)
- [The Rust on ESP Book: Toolchain Installation](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html)
- [esp-rs/esp-idf-template](https://github.com/esp-rs/esp-idf-template)
- [ESP-IDF Programming Guide: Get Started](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/get-started/)
- [Rustup installation](https://rustup.rs/)
