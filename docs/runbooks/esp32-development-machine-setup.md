# ESP32開発端末Setup

> 状態: Linux x86_64 で検証済み（初回 2026-08-06、現行 tree に対する最新の検証 2026-08-10）。Flash / HIL 手順は未検証
> 適用範囲: ESP32 Build profileとESP32 Flash / HIL profile
> 前提OS: 実機のLinux（[ADR-0005](../decisions/0005-standard-development-os.md)）。flashは実機Linuxに限る

## 目的

開発端末へ、classic ESP32 の ESP-IDF Rust build 環境を再現可能な形で導入する。この手順は文書確認専用端末では実行しない。

採用版と理由は [ESP32 Rust Toolchain](../toolchains/esp32-rust-toolchain.md) を参照する。

## 検証状況

| 範囲 | 状態 |
|---|---|
| Linux x86_64 の build-only 経路（1、3〜8節） | 検証済み。[Version Record](../toolchains/version-records/2026-08-06-esp32-build-linux.md) |
| 2節の `rustup component add` | 未実行。検証時は installer の `--profile default` で `rustfmt` と Clippy を導入したため、この command 自体は通していない |
| 3節の `cargo install espflash` | 未実行。Flash / HIL profile 専用であり、ESP32 Build profile では導入しない |
| Windows | 対象外。[ADR-0005](../decisions/0005-standard-development-os.md) により support しない |
| 9節（Flash への移行） | 未検証。#6 の範囲 |

code block はすべて Linux で実行する形式で記載している。

一台で成功した記録は別端末を検証済みにしない。別端末では [Machine Profiles](../toolchains/machine-profiles.md) の「検証の移送」に従って再確認する。

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

ESP-IDF Programming Guide が対象 distribution に指定する package を導入する。package 名は distribution ごとに公式手順を使う。

Ubuntu 22.04 では、[ESP-IDF v5.5.3 の公式手順](https://docs.espressif.com/projects/esp-idf/en/v5.5.3/esp32/get-started/linux-macos-setup.html)（2026-08-06 取得）が列挙する次の package で検証した。

```bash
sudo apt-get install git wget flex bison gperf python3 python3-pip python3-venv \
  cmake ninja-build ccache libffi-dev libssl-dev dfu-util libusb-1.0-0 \
  pkg-config
```

最後の `pkg-config` だけは ESP-IDF の公式一覧に含まれない。3 節の `cargo install` が crate を
source build する際に必要になるため、同じ command でまとめて導入する。
2026-08-06 の検証もこの構成で行った。

補足:

- `python3-venv` は必須である。`esp-idf-sys` は build 中に ESP-IDF の `idf_tools.py install-python-env` を呼び、その内部で `python -m venv` を実行する。`ensurepip` が無い環境では build が成立しない。
- CMake は 3.16 以上が必要である。検証時は distribution 提供の 3.22.1 を使用した。
- `dfu-util` と `libusb-1.0-0` は build-only では使わない。Flash / HIL profile 用である。

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

`rustfmt` と Clippy は installer の既定 profile に含まれる。不足している場合だけ、選択した host toolchain へ追加する。

```bash
rustup component add rustfmt clippy
```

検証時は `--profile default` で導入したため、この `rustup component add` は実行していない。

## 3. Rust on ESP tool

公式 template が要求する Cargo tool を導入し、各 version を記録する。

```bash
cargo install cargo-generate --version 0.23.14 --locked
cargo install ldproxy --version 0.3.5 --locked
cargo install espup --version 0.17.1 --locked
```

`--locked` は、install する package 自身の lockfile どおりに依存を解決させる option であり、
**package の release 版は固定しない**。再現性のために `--version` で版を明示する。

記載した版は検証した組み合わせである。最新の検証日時は [Version Record](../toolchains/version-records/2026-08-06-esp32-build-linux.md) の `最終有効な検証日時` を参照する。
更新する場合は、実際に build が成立した版へ書き換える。

Flash / HIL profile の端末だけ追加する。

```bash
cargo install espflash
```

`cargo-espflash` は初期必須ツールに含めない。必要性が生じた時点で `espflash` との差を review する。

`ldproxy` は `--version` を実装しない。版は次で確認する。

```bash
cargo install --list
```

実行前に公式 document で tool 名と install 方法を再確認する。

## 4. Espressif Rust toolchain

対象 chip と compiler 版を明示して導入する。`--targets` の既定は `all` であり、DeskCat で使わない chip の toolchain まで取得する。

```bash
espup install --toolchain-version 1.95.0.0 --targets esp32 --name esp-1.95.0.0
```

`--toolchain-version` は導入する compiler 版を、`--name` はその toolchain へ付ける名前を指定する。
`--name` の既定は `esp` であり、名前に版が入らない。名前だけを固定しても、その名前へどの版が
入るかは `espup` 任せになり、`--toolchain-version` を付け忘れた端末が別の Xtensa Rust で
build できてしまう。

`firmware/esp32/rust-toolchain.toml` は、この版付きの名前を要求する。

```toml
[toolchain]
channel = "esp-1.95.0.0"
```

そのため上記の `--name` を省略すると、build は compile 前に次で停止する。

```text
error: custom toolchain 'esp-1.95.0.0' specified in override file '.../rust-toolchain.toml' is not installed
```

これは toolchain 名の一致を強制するものであり、その名前の中身が本当に 1.95.0.0 かまでは
検査しない。想定している事故は `--toolchain-version` の付け忘れであり、それは既定名 `esp` が
生成されることで名前が食い違い、上記のとおり停止する。

版を上げるときは、この `--toolchain-version` と `--name`、および
`firmware/esp32/rust-toolchain.toml` の `channel` を**同時に**変える。

**`--toolchain-version` だけを変えて `--name` を据え置くと、この guard は働かない。**
名前は一致したまま中身だけが別の版に入れ替わるため、build はそのまま成功する。
これは本節が防ごうとしている状態そのものであり、版上げの際に最も起こりやすい取り違えでもある。
`--name` は必ず `--toolchain-version` と同じ版を含む名前にする。

`channel` だけを変えて導入を忘れた場合は、その名前の toolchain が無いため停止する。
逆に導入だけを行って `channel` を据え置いた場合は、旧 toolchain が残っている限り
旧版で build が続く。**版上げは 3 か所すべてを変えて完了する。**

`espup install` は environment export file を生成し、その path を出力する。新しい terminal ごとに、その export file を shell へ読み込む。読み込まないと `espup` が設定した環境変数が反映されない。

[espup の公式 README](https://github.com/esp-rs/espup) は、export file を home directory へ生成し、Unix shell では次で読み込むと記載している。2026-08-06 の検証でも同じ path に生成された。

```bash
. "$HOME/export-esp.sh"
```

path を変える場合は `espup install` の `-f, --export-file <EXPORT_FILE>` を使う。`--export-file` は `espup install` の option であり、`espup` 直下の option ではない。

この export file は `LIBCLANG_PATH` と、`xtensa-esp-elf` を含む `PATH` を設定する。読み込まずに build すると失敗する。

導入後に次を記録する。

```bash
espup --version
rustup show
rustup run esp-1.95.0.0 rustc -Vv
xtensa-esp32-elf-gcc --version
```

実行ファイル名は `xtensa-esp32-elf-gcc` だが、`--version` の出力は
`xtensa-esp-elf-gcc` と自己申告する。esp toolchain は chip 別名と統合名の両方を提供しており、
どちらも同じ toolchain を指す。下表が記録しているのは出力側の名前である。

検証した組み合わせでは、次の値になる。異なる場合は
[Version Record](../toolchains/version-records/2026-08-06-esp32-build-linux.md)と照合し、
build 前に差異の理由を確認する。

| 項目 | 検証済みの値 |
|---|---|
| Xtensa Rust | `rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)` |
| GCC | `xtensa-esp-elf-gcc (crosstool-NG esp-15.2.0_20250920) 15.2.0` |
| Xtensa LLVM | esp-clang `esp-20.1.1_20250829` |
| `espup` | 0.17.1 |
| `ldproxy` | 0.3.5 |
| `cargo-generate` | 0.23.14 |

`IDF_PATH` が既に設定されている場合は停止し、意図した外部 SDK か確認する。設定済み `IDF_PATH` は template で選ぶ ESP-IDF version を上書きする。

`firmware/esp32/.cargo/config.toml` の `[env]` は `MCU`、`ESP_IDF_VERSION`、
`ESP_IDF_TOOLS_INSTALL_DIR` に `force = true` を付けており、これら 3 つは shell の
環境変数より優先される。**`IDF_PATH` はこの表に含まれず、`[env]` では保護できない。**
managed install を前提とするため `IDF_PATH` を固定値にできないからである。

**代わりに `firmware/esp32/build.rs` が build を止める。**`IDF_PATH` が設定されていれば
build script が panic し、意図した外部 SDK である場合だけ `DESKCAT_ALLOW_EXTERNAL_IDF_PATH=1`
を設定して通す。**値は厳密に `1` である。**存在するだけで通すと `=0` や `=false` でも
通ってしまい、「無効にしたつもり」の設定が外部 SDK での build を許す。
通した場合は `cargo:warning` が出るので、その値を Version Record の
`IDF_PATH present` へ記録する。**手作業の確認に依存させない。**

## 5. Review済みtemplateから生成

template の moving branch を直接信頼せず、[記録済み commit](../toolchains/esp32-rust-toolchain.md#調査した公式構成) を checkout した local copy を使う。

```bash
git clone https://github.com/esp-rs/esp-idf-template.git <reviewed-template-directory>
git -C <reviewed-template-directory> checkout 08115a069d167a5ee37363e84f168a565f17bbca
cargo generate --path <reviewed-template-directory>/cargo
```

`<reviewed-template-directory>` は実際の一時 path に置き換える。command option は、導入した `cargo-generate` の `cargo generate --help` でも確認する。

対話入力は [生成条件の候補](../toolchains/esp32-rust-toolchain.md#生成条件の候補) に合わせる。同じ入力を非対話で与える場合は次を使う。

```bash
cargo generate --path <reviewed-template-directory>/cargo --name deskcat-esp32 --silent \
  --destination <empty-staging-directory> \
  -d mcu=esp32 -d advanced=true -d espidfver=v5.5.3 -d git=false \
  -d installdir=workspace -d devcontainer=false -d wokwi=false -d ci=false
```

`cargo generate` は生成先へ `--name` と同名の directory を作る。上の例では
`<empty-staging-directory>/deskcat-esp32` が出力される。`--destination` を省略すると
現在の作業 directory 直下へ出力されるため、意図しない場所へ生成しないよう常に明示する。

**この command は `firmware/esp32` へ直接生成しない。** 出力は 1 階層深い
`deskcat-esp32/` に入るため、`--destination firmware/esp32` としても
`firmware/esp32/deskcat-esp32/` になってしまう。空の staging directory で生成し、
差分を review してから中身だけを `firmware/esp32` へ移す。既存 README や
未コミット変更を上書きしない。

`cargo generate` は生成先に新しい `.git` を作る。リポジトリへ移す前に削除する。空の `.vscode` も同様に削除する。

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

生成された `Cargo.toml` の `authors` には、`cargo generate` が git 設定の氏名とメールを埋め込む。公開リポジトリへ個人の連絡先を新たに残さないため、edition 2021 では任意項目であるこの field を削除する。

## 7. Build-only検証

最小 source が未確認 GPIO、LCD、sensor、servo を初期化しないことを確認してから実行する。`firmware/esp32` で実行する。

```bash
. "$HOME/export-esp.sh"
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
```

Clippy は ESP-IDF target でも動作することを確認済みである。実行日時は Version Record を参照する。

`--locked` は、追跡している `Cargo.lock` から解決結果が逸脱した場合に、lockfile を更新せず失敗させる。
再現可能な build のために付ける。`cargo fmt` はこの option を受け付けない
（`error: unexpected argument '--locked' found` になる）。

初回 build は ESP-IDF 本体と managed tool を取得するため長い。検証時は 4 分 33 秒、`firmware/esp32/.embuild` は 4.4 GB になった。`.embuild` と `target` は生成 `.gitignore` で除外されている。

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

- 実機 Linux 端末である。VM と container 上での flash は [ADR-0005](../decisions/0005-standard-development-os.md) で対象外とした
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
