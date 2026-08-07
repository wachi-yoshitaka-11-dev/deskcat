# ESP32開発端末Setup

> 状態: Draft。DeskCatではcommand未検証
> 適用範囲: ESP32 Build profileとESP32 Flash / HIL profile
> 前提OS: 実機のLinux（[ADR-0005](../decisions/0005-standard-development-os.md)）。flashは実機Linuxに限る

## 目的

将来の開発端末へ、classic ESP32 の ESP-IDF Rust build 環境を再現可能な形で導入する。この手順は文書確認専用端末では実行しない。

候補版と理由は [ESP32 Rust Toolchain](../toolchains/esp32-rust-toolchain.md) を参照する。

## 開始条件

- [ ] 対象端末を ESP32 Build または ESP32 Flash / HIL profile として使用する
- [ ] ツール導入が対象 Issue の範囲に含まれる
- [ ] 大きな外部 dependency の導入について人間が確認した
- [ ] OS update と再起動が許容される
- [ ] 必要な storage と network access がある
- [ ] 既存の Rust、Python、ESP-IDF、`IDF_PATH` を調査した
- [ ] flash する場合は exact board、USB-UART、電源、安全条件を確認した

## 1. Host事前要件

### Linux

ESP-IDF Programming Guide が対象 distribution に指定する package を導入する。一般に Git、Python、pip、venv、CMake、Ninja、C compiler、build utilities、libffi、OpenSSL、libusb が必要になるが、package 名は distribution ごとに公式手順を使う。

Flash profile では `espflash` に必要な `libudev` development package と USB permission も確認する。

build-only 検証は Docker 上の Linux で行ってよい。flash と serial monitor は実機 Linux で実行する。判断基準は [Machine Profiles](../toolchains/machine-profiles.md) の標準OS節を参照する。

### Windows（対象外）

[ADR-0005](../decisions/0005-standard-development-os.md)により、Windows は support 対象外である。この節の手順は実行しない。

2026-07-27 時点では MSVC host 環境と Visual Studio Build Tools を候補として記載していたが、いずれの端末でも実行しておらず、検証根拠は存在しない。Windows 上の VM および Docker は USB passthrough を提供しないため、flash も成立しない。

## 2. Rust host環境

[rustup.rs](https://rustup.rs/) の公式 installer を使う。第三者配布の Rust package と混在させない。

導入後、次を記録する。

```bash
rustup -V
rustc -Vv
cargo -V
rustup show
```

`rustfmt` と Clippy が不足している場合だけ、選択した host toolchain へ追加する。

```bash
rustup component add rustfmt clippy
```

## 3. Rust on ESP tool

公式 template が要求する Cargo tool を導入し、各 version を記録する。

```bash
cargo install cargo-generate
cargo install ldproxy
cargo install espup
```

Flash / HIL profile の端末だけ追加する。

```bash
cargo install espflash
```

`cargo-espflash` は初期必須ツールに含めない。必要性が生じた時点で `espflash` との差を review する。

実行前に公式 document で tool 名と install 方法を再確認する。再現性のため、検証が成功した後は実際に使用した version の固定方法を決める。

## 4. Espressif Rust toolchain

```bash
espup install
```

`espup` が出力した export file を、新しい terminal ごとに読み込む。

導入後に次を記録する。

```bash
espup --version
rustup show
```

`IDF_PATH` が既に設定されている場合は停止し、意図した外部 SDK か確認する。設定済み `IDF_PATH` は template で選ぶ ESP-IDF version を上書きする。

## 5. Review済みtemplateから生成

template の moving branch を直接信頼せず、[記録済み commit](../toolchains/esp32-rust-toolchain.md#調査した公式構成) を checkout した local copy を使う。

```bash
git clone https://github.com/esp-rs/esp-idf-template.git <reviewed-template-directory>
git -C <reviewed-template-directory> checkout 08115a069d167a5ee37363e84f168a565f17bbca
cargo generate --path <reviewed-template-directory>/cargo
```

`<reviewed-template-directory>` は実際の一時 path に置き換える。command option は、導入した `cargo-generate` の `cargo generate --help` でも確認する。

対話入力は [生成条件の候補](../toolchains/esp32-rust-toolchain.md#生成条件の候補) に合わせる。生成先は `firmware/esp32` とし、既存 README や未コミット変更を上書きしない。安全のため、空の staging directory で生成して差分を review してから移す方法を推奨する。

## 6. 生成物のreview

最低限、次を review する。

- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `.cargo/config.toml`
- `build.rs`
- `sdkconfig.defaults`
- build target、linker、runner
- crate source、version、features、license
- startup 時に未確認 GPIO や peripheral を初期化しないこと

application の `Cargo.lock` を追跡する。template 由来の `.gitignore` が `/Cargo.lock` を除外している場合は、その行を外す。

## 7. Build-only検証

最小 source が未確認 GPIO、LCD、sensor、servo を初期化しないことを確認してから実行する。

```bash
cargo fmt --all -- --check
cargo build
```

Clippy は ESP-IDF target での対応を実行時点で確認し、成功した正式 command だけをリポジトリへ記録する。未確認 command を受け入れ条件にしない。

この段階では flash しない。

## 8. 証拠

[Version Record Template](../toolchains/version-record-template.md) を使い、次を保存する。

- template commit
- tool versions
- environment override の有無
- generated configuration
- dependency lockfile
- clean build log
- build duration
- artifact path と size
- warning と workaround

個人 path や credential を取り除いてから公開リポジトリへ追加する。

## 9. Flashへの移行

flash と serial monitor は #6 で行う。次を満たすまで実行しない。

- 実機 Linux 端末である（VM と container では USB が見えない）
- exact board と USB-UART を確認済み
- unknown output を駆動しない firmware
- servo 電源を切り離している
- 対象 port を確認した
- 人間が監視している
- rollback または再flash 手順がある

## 失敗時

- version と完全な error を保存する。
- SDK や tool を無計画に upgrade / downgrade しない。
- `IDF_PATH`、Python environment、host linker、target、lockfile を分けて調べる。
- 一度成功した cache を根拠にせず、必要に応じて clean environment で再現する。
- tool の削除や system-wide 設定変更は、対象を確認して別操作として扱う。

## 公式資料

- [The Rust on ESP Book: Toolchain Installation](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html)
- [esp-rs/esp-idf-template](https://github.com/esp-rs/esp-idf-template)
- [ESP-IDF Get Started](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/get-started/)
- [Rustup installation](https://rustup.rs/)
