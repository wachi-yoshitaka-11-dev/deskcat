# ESP32 Rust Toolchain

> 状態: build検証済み（Linux x86_64 の ESP32 Build profile 端末、および CI の `ubuntu-24.04`）。**実機確認は未実施**
> 調査日: 2026-07-27
> build 検証日: 2026-08-06（初回）／2026-08-10（現行 tree に対する最新の検証、および CI での再現）
> 証拠: [開発端末](version-records/2026-08-06-esp32-build-linux.md)／[CI](version-records/2026-08-10-esp32-build-ci.md)
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
| board | ESP-WROOM-32D 開発ボード（秋月電子 M-13628）。基板裏面 silkscreen は **`ESP32_DevKitc_V4`**（2026-08-15 に大文字小文字を訂正。旧記載 `ESP32_DevkitC_V4`）で、Espressif ESP32-DevKitC V4 のリファレンスデザインに基づく | [秋月商品ページ M-13628](https://akizukidenshi.com/catalog/g/g113628/)（**添付はモジュールとチップの datasheet のみ**）、[ESP32-DevKitC V4 公式回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)。確定根拠は [hardware-bom.md](../hardware/hardware-bom.md) の MCU-01 |
| module | ESP-WROOM-32D。中核は `ESP32-D0WD`。Xtensa dual-core 32-bit LX6 | [ESP32-WROOM-32D & ESP32-WROOM-32U Datasheet v2.7](https://documentation.espressif.com/esp32-wroom-32d_esp32-wroom-32u_datasheet_en.pdf)（2026-08-08 取得） |
| target | MCU `esp32` → arch `xtensa` / rust target `xtensa-esp32-espidf` / gcc target `xtensa-esp32-elf` | [esp-idf-template `08115a06`](https://github.com/esp-rs/esp-idf-template/blob/08115a069d167a5ee37363e84f168a565f17bbca/cargo/pre-script.rhai) の `pre-script.rhai`（2026-08-06 取得） |

同 template は MCU `esp32` の Wokwi board を `board-esp32-devkit-c-v4` と定義しており、対象 board と MCU 選択の対応を裏づける。

本 board は silkscreen が示すとおり V4 リファレンスデザインに基づき、搭載 module も classic ESP32 系である。したがって MCU 選択と target は同一になる。

ESP-WROOM-32D の datasheet v2.7 には **PSRAM を内蔵する variant の記載が無い**。同 datasheet が挙げる variant の差は antenna（PCB antenna の 32D と外部 connector の 32U）だけである。したがって PSRAM の有無による構成差は本 board では生じない。

現物確認により、module 種別と、基板に revision 表示が無いことは確定している。**module 種別の根拠は 2026-08-13 に module 自身の刻印（`ESPRESSIF` / `ESP32-WROOM-32D`）になった。**購入履歴と基板裏面 silkscreen **`ESP32_DevKitc_V4`** もこれと整合する。

[HW-TBD-001](../hardware/tbd-register.md) の**照合は 2026-08-13 に完了し、一致した。**38pin ヘッダ両側の silk が公式 `J2`／`J3` と 19pin×2 列すべてで一致した（GND の位置を含む）。**これにより、この文書の確定条件のうち「公式 pin 表と現物 pin 表記を照合した」は満たされる見込みである**（close 判定は台帳の解決手順 1–8 に従う）。

なお、**旧記載の「Espressif 公式の ESP32-DevKitC ではない／秋月電子の独自基板である」は 2026-08-10 に削除した。**この断定を支持する資料が存在せず、秋月商品ページ自身がメーカーを `Espressif Systems`、型番を `ESP32-DevKitC-32D` と表示していたためである。詳細は [hardware-bom.md](../hardware/hardware-bom.md) の Revision 29。**照合が必要である点は変わらない**（理由が変わっただけである）。

これとは別に、**chip の刻印は非破壊では読めないことが 2026-08-15 に確定した。**中核 chip `ESP32-D0WD` は ESP-WROOM-32D の半田付けされた金属シールドの内側にあり、シールドを外さない限りどのような撮影手段でも見えない。**旧記載の「現物写真が反射で判読不能」は、撮影条件の問題として書いていたが誤りである。**撮り直しても解決しない。

**module の刻印は 2026-08-13 に読了した**（斜光＋接写）。シールド上面に `ESPRESSIF` と `ESP32-WROOM-32D` が刻印されており、**module 種別の根拠が購入履歴と silkscreen に加えて module 自身の刻印になった。**ただし**これは module の品番であって chip の品番ではない。**中核 chip が `ESP32-D0WD` であるという記載の出典は、上表のとおり ESP-WROOM-32D datasheet v2.7 のままである。

追跡は [`HW-TBD-031`](../hardware/tbd-register.md)（2026-08-11 の全数照合で登録し、2026-08-15 に要件を書き換えた）。**本文書の状態を `Verified` にする条件のうち chip 刻印の項は、非破壊で満たせないため 2026-08-15 に満たし方を再定義した。****`esptool` が報告する chip 名で満たす。**一次資料で、`esptool` が品番相当の文字列を返すことを確認した。**`espflash` の出力では満たせない**（family 名しか返さない）。実施は実機 Linux 限定の flash（[#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)）まで進まないため、**この項目は未達のままである**（[chip 識別の満たし方](#chip-識別の満たし方)）。

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
| Rust channel | `esp` | `esp-1.95.0.0`（Xtensa Rust 1.95.0.0） |
| ESP-IDF | `v5.5.3` が default | `v5.5.3`（commit `2c211b236707889e8400c4dc5644dd5c4ee071e0`） |
| Rust edition | `2021` | `2021` |
| Minimum Rust | `1.82` | manifest の下限は `1.82`。build に使用した compiler は Xtensa Rust 1.95.0.0 |
| `esp-idf-svc` | `0.52.1` | `0.52.1` |
| `embuild` | `0.33` | `0.33.3` |
| linker | `ldproxy` | `ldproxy` 0.3.5 → `xtensa-esp-elf-gcc` 15.2.0 |
| runner | `espflash flash --monitor` | **2026-08-20 に検証した**（`espflash` 4.5.0）。`--port` と `--chip esp32` を明示し、artifact の path を渡して実行した。**非対話 shell では monitor が `Failed to initialize input reader` で落ちるため pty を割り当てる。**記録は [Version Record](version-records/2026-08-20-esp32-flash-boot-native.md) |
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
- `esptool` は flash を行う端末だけで必須。**chip 識別の確認に要る**（[chip 識別の満たし方](#chip-識別の満たし方)）。**`espflash` では代替できない。****2026-08-20 に、pin 済み ESP-IDF v5.5.3 の tool 一式へ含まれることを確認した**（`esptool` 4.12.0。追加導入は不要）。記録は [Version Record](version-records/2026-08-20-esp32-flash-boot-native.md)
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

### compiler 版の固定

`rust-toolchain.toml` の `channel` へ、版を含む toolchain 名を指定する。

```toml
[toolchain]
channel = "esp-1.95.0.0"
```

この名前は `espup install --toolchain-version 1.95.0.0 --targets esp32 --name esp-1.95.0.0` が作る。
`--name` の既定は `esp` で、名前に版が入らない。名前だけを固定した場合、その名前へどの版が入るかは
`espup` 任せになり、`--toolchain-version` を付け忘れた端末が別の Xtensa Rust で build できてしまう。
版付きの名前にすると、その toolchain が無い環境では rustup が compile 前に停止する。

これで、再現性を左右する 3 つの入力がいずれも実行時に強制される。

| 入力 | 強制する仕組み |
|---|---|
| dependency 解決 | `--locked`（`Cargo.lock` から逸脱したら失敗） |
| ESP-IDF 版 | `.cargo/config.toml` の `[env]` と `force = true`（環境変数で上書きされない） |
| Rust compiler 版 | `rust-toolchain.toml` の版付き channel 名（未導入なら compile 前に失敗） |

**これは toolchain 名の一致を強制するものであり、その名前の中身が本当に 1.95.0.0 かは検査しない。**
想定している事故は `--toolchain-version` の付け忘れであり、その場合は既定名 `esp` が生成されて
名前が食い違うため停止する。

残る穴は、**同じ名前へ異なる版を導入した場合**である。悪意を要しない。版上げの際に
`--toolchain-version` だけを変えて `--name` を据え置けば、名前は一致したまま中身が入れ替わり、
build はそのまま成功する。この取り違えは runbook の版上げ手順で注意喚起するにとどまり、
仕組みでは止まらない。

採らなかった案は次である。

| 案 | 採らなかった理由 |
|---|---|
| `espup` 側で版を固定する | `espup` 0.17.1 に設定 file は無く、版の指定は `--toolchain-version` という起動 option だけである（`espup install --help` で確認、2026-08-10）。付け忘れを止める手段が `espup` 側に無い |
| `build.rs` で実際の compiler 版を検査する | template 由来 file へ独自コードを足すことになり、版を上げるたびに `rust-toolchain.toml` と `build.rs` の両方を直す必要が生じる。塞げる穴が「意図的な偽装」に限られ、費用に見合わない |
| CI で版を照合する | 手元 build を止められない。[#42](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/42) で CI を導入する際に、多重の検査として別途判断する |

`build.rs` での実版検査は、必要になれば別 Issue で改めて検討する。

実証は [Version Record](version-records/2026-08-06-esp32-build-linux.md) の #74 追記にある。

## 確定条件

次を満たすまで状態を`Verified`または`Accepted`に変更しない。

- [x] 物理基板の機種と搭載 module を確認した（ESP-WROOM-32D 開発ボード／秋月電子 M-13628。基板に revision 表示は無い）
- [x] **module 刻印を読み取った**（`ESPRESSIF` / `ESP32-WROOM-32D`。2026-08-13、斜光＋接写）
- [x] **chip の識別を確認した**（2026-08-20。`esptool` 4.12.0 が `ESP32-D0WD` を報告した）。**旧項目「chip 刻印を読み取った」は非破壊で満たせないため 2026-08-15 に書き換え、同日に満たし方を再定義した**（chip はシールド内側にある）。**満たし方は [chip 識別の満たし方](#chip-識別の満たし方) に定める。**未加工の出力と実行した command は [Version Record](version-records/2026-08-20-esp32-flash-boot-native.md) にある
- [x] **公式 pin 表と現物 pin 表記を照合した**（2026-08-13、19pin×2 列すべて一致。[HW-TBD-001](../hardware/tbd-register.md) は close 済み）
- [x] 開発端末の profile と version record を作成した
- [x] レビュー済み template commit から最小 project を生成した
- [x] 環境変数による意図しない SDK override がない
- [x] 記録済みの toolchain 名が未導入の端末では build が compile 前に停止することを実証した（[#74](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/74)。同じ名前へ異なる版が入っている場合は検出しない。[compiler 版の固定](#compiler-版の固定)を参照）
- [x] clean `cargo build` が成功した
- [x] dependency と lockfile を review した
- [x] 正式な format、lint、build command を `AGENTS.md` と root README へ反映した
- [x] 別の開発端末または clean environment で再現した（CI の `ubuntu-24.04` runner。[#42](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/42)、[Version Record](version-records/2026-08-10-esp32-build-ci.md)）

**未達は 1 項目になった。**この文書の状態は `検証済み`（`Verified`）ではなく **`build検証済み`** にとどめる。語の定義は [状態ラベル](README.md#状態ラベル) を参照する。

- **chip の識別**: **非破壊では刻印を読めないことが 2026-08-15 に確定した**（シールド内側）。**したがって「撮り直せば満たせる項目」ではない。**module 刻印は読了しており、搭載 module は module 自身の刻印で確定した。その datasheet が中核 chip を示すため、build への影響は無い。**満たし方は 2026-08-15 に再定義した**（[chip 識別の満たし方](#chip-識別の満たし方)、[HW-TBD-031](../hardware/tbd-register.md)）。
- **公式 pin 表と現物 pin 表記の照合**: **2026-08-13 に完了し、一致した**（[HW-TBD-001](../hardware/tbd-register.md) は close）。**この項目は未達ではなくなった。**#6 の flash 前提条件も、この照合については満たされる。

**別端末での再現は満たした。**根拠と、そう判断してよい理由を次に示す。

CI（`ubuntu-24.04`）で clean build し、`cargo fmt`、`cargo clippy --locked`、
`cargo build --locked` がすべて成功した（[Version Record](version-records/2026-08-10-esp32-build-ci.md)）。
**3 回実行しており、うち 2 回は commit `663a486`、1 回は `8889281` である**
（両者の差分は文書 file だけで、`firmware/` 配下と workflow は同一）。
[Machine Profiles](machine-profiles.md) の「検証の移送」が別端末へ求める 7 項目
（OS、architecture、container／VM／実機の別、toolchain と target、linker と SDK、commit、
lockfile、clean build）をすべて記録している。

**CI runner を再現対象に含めてよいと判断した理由**は次の 2 点である。

- 同文書は「ESP32 の build-only 検証」について **Docker 上の Linux を使ってよい**と明記している。
  「container で得た結果を実機の根拠として扱わない」という制限は、**flash と実機試験に掛かる**ものである
- 本記録は build-only であり、flash、serial monitor、実機起動を一切主張しない。それらは #6 の範囲である

標準OSは [ADR-0005](../decisions/0005-standard-development-os.md) により実機 Linux であり、
Windows は対象外のため再現対象に含めない。**CI は Windows ではなく Linux であり、この除外に当たらない。**

**それでも状態は `Verified` へ上げない。**残る 1 項目（chip の識別）は CI では代替できない。**「CI が通ったから昇格できる」とは結論しない。**

**うち chip の識別は、満たし方を 2026-08-15 に再定義した。**旧条件は非破壊で成立せず、
満たし方が未定のままだった。**再定義したのは満たし方であって、項目が満たされたわけではない。**
**2026-08-20 に #6 で実施し、この項目を満たした**（`esptool` 4.12.0 が `ESP32-D0WD` を報告した。[chip 識別の満たし方](#chip-識別の満たし方)）。記録は [Version Record](version-records/2026-08-20-esp32-flash-boot-native.md)。

flash、serial monitor、実機起動は #6 の範囲であり、この文書の build-only 確定条件には含めない。

### chip 識別の満たし方

**2026-08-15 に決定した。**旧項目「chip 刻印を読み取った」は非破壊で成立しない
（chip はシールド内側にある）ため、次のとおり再定義した。**この節がこの条件の正本である。**
[HW-TBD-031](../hardware/tbd-register.md) へ内容を再掲しない。

| 項目 | 内容 |
|---|---|
| 満たし方 | **実機へ接続したときに `esptool` が報告する chip 名**を記録し、`ESP32-D0WD` または `ESP32-D0WD-V3` であることを確認する |
| 期待値が 2 つある理由 | `esptool` は major revision 3 の個体へ `-V3` を付ける。**どちらであるかは現物を読むまで決まらない**ため、両方を満たしたものとして扱う |
| **`espflash` では満たせない** | `espflash` が印字するのは family 名 `esp32` であり、品番を含まない（根拠は下表）。**本文書は runner を `espflash flash --monitor` と記録しているため、その出力を見ただけではこの項目を満たさない** |
| 使う command | **2026-08-20 に確定した。**`esptool.py --port <port> chip_id` を使う。**この command の出力の label は `Chip is` であり、`Chip type:` ではない**（`Chip type:` を出すのは `espflash` だが、そちらは family 名しか含まない）。**照合するのは chip 名だけ**であるため判定は成立する。実行した command と `esptool` の版は [Version Record](version-records/2026-08-20-esp32-flash-boot-native.md) にある |
| datasheet の扱い | 中核 chip を `ESP32-D0WD` とする [ESP-WROOM-32D datasheet v2.7](https://documentation.espressif.com/esp32-wroom-32d_esp32-wroom-32u_datasheet_en.pdf) の記載は根拠として維持する。**ただし datasheet の記載だけではこの項目を満たさない** |
| 期待値と食い違った場合 | `ESP32-D0WD` 系以外が返ったら、datasheet 由来の記載が現物と食い違う。**その訂正**（[hardware-bom.md](../hardware/hardware-bom.md) の `MCU-01` と本文書の [board から target を決めた根拠](#board-から-target-を決めた根拠)）**は別 Issue で行う。**この決定はその訂正を含まない |
| 実施時期 | [#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)。**この項目のチェックが埋まるのは #6 のあとである** |

**この決定が担保しないもの**を明示する。

- **efuse は package／variant を示すものであって刻印ではない。**現物の chip に印字された文字を読んだことにはならない
- 読み出した値が Espressif 出荷時のものであることは検査していない
- `esptool` が持つ package 値から品番への対応表そのものの正しさは検証していない。**依拠しているのは Espressif 自身の実装である**

根拠は次の一次資料である。

| 主張 | 一次資料 |
|---|---|
| `esptool` は品番相当の文字列を報告する。公式 documentation の例は `Chip type:          ESP32-D0WD (revision 1)` である | [esptool: Flashing Firmware](https://docs.espressif.com/projects/esptool/en/latest/esp32/esptool/flashing-firmware.html)（2026-08-15 取得） |
| その文字列は efuse の package 値、single core を示す bit、major revision から導かれる。対応表は `ESP32-D0WD`／`ESP32-D0WD-V3`／`ESP32-D0WDQ6`／`ESP32-D2WD`／`ESP32-U4WDH`／`ESP32-PICO-D4` などを含む | [esptool v5.3.1 の `esptool/targets/esp32.py`](https://github.com/espressif/esptool/blob/v5.3.1/esptool/targets/esp32.py) にある `get_chip_description()`（2026-08-15 取得） |
| `espflash` は family 名しか印字しない。`print_board_info` は `Chip type: {info.chip}` を出力し、`Chip` enum は `strum(serialize_all = "lowercase")` により `esp32` になる。package variant 名はこの crate に存在しない | [espflash v4.5.0 の `espflash/src/cli/mod.rs`](https://github.com/esp-rs/espflash/blob/v4.5.0/espflash/src/cli/mod.rs) と [同 `espflash/src/target/mod.rs`](https://github.com/esp-rs/espflash/blob/v4.5.0/espflash/src/target/mod.rs)（2026-08-15 取得） |

**照合するのは chip 名だけである。**上表の 1 行目と 2 行目は revision の書式が食い違っている
（documentation の例は `(revision 1)`、v5.3.1 の実装は `(revision v<major>.<minor>)`）。
**行全体を文字列一致で照合しない。**この項目が求めるのは chip 名であって revision の書式ではない。

**版へ固定したのは実装の引用だけである。**上表 2 行目の `esptool` は v5.3.1、3 行目の `espflash` は
v4.5.0 であり、既定 branch は変わるため tag へ固定した path で確認した。**documentation の link は
固定していない。**上表 1 行目は `latest` を指しており、**内容が変わりうる**ため取得日を併記した。
**引用時点では採用版として確定していなかった。****2026-08-20 に [#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6) で決めた。**
`espflash` は **4.5.0** を採用し、導入 command は [ESP32 開発端末セットアップ](../runbooks/esp32-development-machine-setup.md) が持つ。
`esptool` は **pin 済み ESP-IDF v5.5.3 の tool set に含まれる 4.12.0** を使い、追加導入しない。
**実際に使った版と command は [Version Record](version-records/2026-08-20-esp32-flash-boot-native.md) にある**（引用に使った版とは別である。上表 2 行目の `esptool` v5.3.1 は実装の引用であり、**実行した版は 4.12.0 である**）。

**この再定義とあわせて日付を 1 件訂正した。**上の確定条件は旧記載で「非破壊で満たせないため
**2026-08-13 に**書き換えた」としていたが、書き換えたのは **2026-08-15** である。08-13 は
現物写真を撮った observation の日であって、文書を変更した日ではない。**observation の日付と
文書変更の日付を同じ日として書いていた**ため、2026-08-15 に訂正した。

## 公式資料

- [The Rust on ESP Book: Getting Started](https://docs.espressif.com/projects/rust/book/getting-started/index.html)
- [The Rust on ESP Book: Toolchain Installation](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html)
- [esp-rs/esp-idf-template](https://github.com/esp-rs/esp-idf-template)
- [ESP-IDF Programming Guide: Get Started](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/get-started/)
- [Rustup installation](https://rustup.rs/)
