# Version Record: ESP32 Build (Linux x86_64) — 2026-08-06

> 判定: `Partial`
> 対象Issue: [#5](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/5)

[Version Record Template](../version-record-template.md)の形式で、実際に確認した環境を記録する。

> **`Conclusion`の「別端末での再現が未実施」は本記録の作成時点の状態である。**
> その後、CIの`ubuntu-24.04` runnerで再現した（#42、[CIのVersion Record](2026-08-10-esp32-build-ci.md)）。
> 詳細は「[未達の確定条件](#未達の確定条件)」の追記を参照する。**実機確認（flash、起動）は依然として未実施である。**
>
> **boardの識別情報は本記録の作成後に訂正された。**
> 以降の本文と log は**実施時点（2026-08-06〜08）の記録**であり、遡って書き換えていない。
> `Physical board`／`Module marking`／`Board revision` を「未確認」とする記載、および
> 「未達の確定条件」節の物理基板に関する記述は、その時点の状態である。
> 現在の正しい識別情報と、確定・未確定の切り分けは
> 「[boardの識別に関する訂正](#boardの識別に関する訂正2026-08-08-追記)」節を参照する。

`Verified`にしない理由は「[未達の確定条件](#未達の確定条件)」を参照する。

## 記録

```text
Record ID: VR-2026-08-06-ESP32-BUILD-01
Date: 2026-08-06（初回検証）
最終有効な検証日時: 2026-08-11T11:13:22Z
  現行のsource tree fb4425a8 に対する実行日である（#102。下の「`IDF_PATH` guard の検証log」節）。
  前回は2026-08-10T02:47:29Z、tree cf3fcdb4 であった。
  reviewを受けた修正と、developへのrebaseでtreeが変わるたびに再検証している。
  Record IDとfile名は初回検証日のまま維持し、追跡の連続性を保つ。
Machine profile: ESP32 Build
Operator role: AIエージェント（ツール導入について人間の事前確認あり）
Repository commit: e20c2a2744903ea6ced9f469fbb74116419455de（最新検証時点のbase。developのtipと同一）
  前回は831f4b17fa151ee731eed01abd981fe8feaaffc0、初回は109f1a90d9688542f9381fb95d4269ef336e23ddであった。
Build対象のsource tree: fb4425a82a0ceedca9cd6beb5016551d7a0ef8fa
  `firmware/esp32`のtree objectである（`git rev-parse HEAD:firmware/esp32`）。
  この記録は`docs/`配下にあり同じtreeへ含まれないため、記録自体を更新しても値は変わらず、
  commitのhashを書く場合のような循環が起きない。
  **この値が識別するのは追跡下のsourceだけであり、build入力の全体ではない。**
  workspaceへ展開した`.embuild`配下のESP-IDFとmanaged tool、toolchainの実体、
  生成物のいずれもこのtreeに含まれない。それらは本記録の
  `ESP-IDF version`／`ESP-IDF source/commit`／`Rust compiler version`／
  `Linker identity and version`／`Generated artifact identity`が別々に識別する。
  この値の一致は「追跡下のsourceが同一」までしか意味せず、build入力全体の同一性を意味しない。
  またこのtreeは`firmware/esp32/README.md`も含むため、buildに影響しないREADME編集でも値は変わる。
  安全側に倒した近似であり、一致しなければ検証をやり直すという使い方をする。
  再現時は`git rev-parse HEAD:firmware/esp32`をこの値と突き合わせ、
  あわせて上記のtoolchain識別子も照合する。
Working tree clean: no
  2026-08-11の検証時点では、**追跡下のfileに内容差分は無く**、untrackedの作業指示file 1件
  （git管理外）だけが残っていた。前回（2026-08-10）は、firmware/esp32配下の生成fileと
  この記録自体が未commitであり、tree hashは生成fileをcommitした後のbranchで採取していた。
  2026-08-11に`git status`が示した44件の`M`は**全件が`mode change 100644 => 100755`**であり、
  共有mount由来のfile modeの見え方の差で、内容は同一であった
  （local設定の`core.fileMode`で切り分け、検証後に元へ戻している）。

OS name: Ubuntu
OS version: 22.04.5 LTS (Jammy Jellyfish) / kernel 5.15.0-187-generic
CPU architecture: x86_64
Userspace bitness: 64-bit
Container / VM / native: VM（systemd-detect-virt = microsoft）
  [Machine Profiles](../machine-profiles.md)の標準OSは実機のLinuxであり、
  ESP32のbuild-only検証はDocker上のLinuxでも認められている。本記録はVM上の実行である。
  同文書は「containerで得た結果を、実機の根拠として扱わない」と定めており、
  VMも同じ扱いとする。**本記録は実機の根拠にならない。**
  このことは確定条件「別の開発端末またはclean environmentで再現した」が
  未達である理由の一つでもある。flashと実機試験は実機Linuxに限られる（#6）。

Rustup version: 1.29.0 (28d1352db 2026-03-05)
Rust channel: esp（firmware build用。rust-toolchain.tomlで固定）／host側はstable
  **この値は初回検証（2026-08-06）時点の記録である。**#74により、現行treeの`channel`は
  版付きの`esp-1.95.0.0`へ変更されている。compilerの実体（1.95.0.0）は同一であり、
  変わったのはtoolchainの名前と、その名前が実行時に強制されることである。
  「[compiler版の固定](#compiler版の固定74-追記)」節を参照する。
Rust compiler version:
  esp:  rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0) / LLVM 21.1.3
  host: rustc 1.97.1 (8bab26f4f 2026-07-14) / LLVM 22.1.6
Rust host: x86_64-unknown-linux-gnu
Installed Rust targets: x86_64-unknown-linux-gnu（host）。
  xtensa-esp32-espidfはespチャネルのbuild-stdで供給され、rustupのtarget一覧には現れない
Cargo version: 1.97.1 (c980f4866 2026-06-30)
rustfmt version: 1.9.0-stable (8bab26f4f6 2026-07-14)
Clippy version: 0.1.97 (8bab26f4f6 2026-07-14)
Linker identity and version:
  ldproxy 0.3.5 → xtensa-esp-elf-gcc (crosstool-NG esp-15.2.0_20250920) 15.2.0

ESP32 only:
  Physical board: 未確認（HW-TBD-001。build-onlyのため実機なし）
  Module marking: 未確認（HW-TBD-001）
  Board revision: 未確認（HW-TBD-001）
  Rust target: xtensa-esp32-espidf
  espup version: 0.17.1
  cargo-generate version: 0.23.14
  ldproxy version: 0.3.5
  espflash version: 未導入（ESP32 Build profileでは不要。#6 の範囲）
  ESP-IDF version: v5.5.3
  ESP-IDF source/commit: 2c211b236707889e8400c4dc5644dd5c4ee071e0
  ESP-IDF tools location mode: workspace（firmware/esp32/.embuild配下）
    値の出所は`firmware/esp32/.cargo/config.toml`の`[env]`であり、shellの環境変数ではない。
    build logが出力する`ESP_IDF_TOOLS_INSTALL_DIR=<unset>`はshell側の確認結果であって、
    実効値が未設定という意味ではない。両者は矛盾しない。
    なおCargoの`[env]`は既定では既存の環境変数を上書きしないため、
    review後に`force = true`を付けて実効性を担保した（「生成条件」節を参照）。
  IDF_PATH present: no（shell環境）
    **2026-08-11に実測へ置き換えた（#102）。**`~/export-esp.sh`の中身を読み、
    `LIBCLANG_PATH`と`PATH`の2行だけで`IDF_PATH`を設定しないことを確認した。
    従来この`no`は推測であり、`export-esp.sh`が設定していれば
    `build.rs`のguardが全buildを止めるという未確認のriskがあった。
    **「上書きしうる」も推測ではなくなった。**`IDF_PATH`を設定してbuildすると
    `esp-idf-sys`が`Ignoring configuration setting ESP_IDF_VERSION (Tag v5.5.3):
    custom esp-idf repository detected via $IDF_PATH`を出し、pinを実際に破棄する。
  IDF_TOOLS_PATH present: no（shell環境）
  Template repository: https://github.com/esp-rs/esp-idf-template
  Template commit: 08115a069d167a5ee37363e84f168a565f17bbca
  sdkconfig/defaults identity: template既定のstack size調整のみ。
    CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192 / CONFIG_ESP_SYSTEM_EVENT_TASK_STACK_SIZE=4096 /
    CONFIG_FREERTOS_IDLE_TASK_STACKSIZE=4096 / CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT=4096
    peripheral、GPIO、電源に関する設定は含まない
  USB-UART identity: 未確認（build-onlyのためboard未接続。#6 の範囲）

Commands run:
  初回のbuild log採取時:
    espup install --targets esp32
    cargo fmt --all -- --check
    cargo build
    cargo clippy --all-targets -- -D warnings
  review指摘を受けた再検証時（正式なcommandはこちら）:
    cargo install cargo-generate --version 0.23.14 --locked
    cargo install ldproxy --version 0.3.5 --locked
    cargo install espup --version 0.17.1 --locked
    espup install --toolchain-version 1.95.0.0 --targets esp32
    cargo fmt --all -- --check
    cargo clippy --all-targets --locked -- -D warnings
    cargo build --locked
Expected result: 全commandがrc 0で終了し、Xtensa向けELFが生成される
Actual result: 全commandがrc 0。warningなし
  生成fileをcommitした後、feature/m1-001-esp32-toolchain上で再検証した。
  `--locked`付きのclippyとbuildがrc 0で、Cargo.lockに変化が無いことを確認した。
  `espup install --toolchain-version 1.95.0.0`は記録済みのXtensa Rust 1.95.0.0を解決した。
  `cargo fmt`は`--locked`を受け付けない（`error: unexpected argument '--locked' found`）ため、
  fmtだけはoptionなしのままとする。
Build duration:
  初回（ESP-IDF本体のcompileを含む）: cargo build 4m33s（real 4m34.393s）／cargo clippy 10.5s
  最終検証（cargo clean後。.embuild内のESP-IDFは再利用）:
    cargo clippy --locked 2m35s（real 2m34.985s）／cargo build --locked 21s（real 0m21.261s）
    clippyが先に全crateをcheckするため、後続のbuildが短い
Peak memory if measured: 未計測
Storage delta if measured:
  espup install: 約 +1.8 GB（9.2G→11G）
  firmware/esp32/.embuild: 4.4 GB（ESP-IDFとmanaged tool一式。git管理外）
Generated artifact identity:
  現行のsource tree fb4425a8 に対応する成果物は次の1つである。
  path: firmware/esp32/target/xtensa-esp32-espidf/debug/deskcat-esp32
  type: ELF 32-bit LSB executable, Tensilica Xtensa, version 1 (SYSV), statically linked, with debug_info, not stripped
  size: 13,660,584 bytes
  sha256: 54a2adec71f0af045f880af867a36c9cfcf732196cbf7765f65aa02f3ffef375
  実行日時: 2026-08-11T11:13:22Z（#102の検証後、`IDF_PATH`未設定で復帰buildしたもの）
    sizeは前回と同一で、sha256だけが異なる。source treeが変わっているうえ、
    **入力を揃えてもsha256は一致しないことを下の「再現性実験log」で測定済み**であり矛盾しない。
    cache利用のincremental buildである（`cargo clean`からのfull buildは実施していない）。

  過去のtreeでの実行結果（履歴。現行treeの成果物ではない）:
    tree cf3fcdb4 / #101前:            0f9d4918526c3e4153d101943ee20c5d9a4ac3d0f08fce6f60569b503e2877f7
    tree cdcff41c / #74の自己review前:  11deb6d07a8bdec9e41f51464ba460eb4a4417f9757bf8dd1641bfb01b729bf9
    tree 815c903f / 表記統一後:      1e9623de74b79b33c85510c41c755093f70a4bf85969f40b90fa47ce7ae69d78
    tree b0120755 / board識別訂正後: 626707014889c5dc83ce0dc453383ee8139fa2d8feded452380fcfed21a96d97
    tree 79351378 / review反映後:    817d55948479097c0389db4bba08140aa7cca2c974c7a04258724ec3cadca442
    tree 9e48f340 / rebase後:        678cc201e60bb7376c38c19534d2e4819bdc40e68e20713bcb789b04d3376c24
    tree 58ea4794 / force=true追加後: 6a541ff8de3c66c18856397933f87ce26d4b95bae0d2ceb01b8dc361e9bd5c47
    tree 10bad8e / 最終検証:     b1cb851bdf43ff2449ad394ac234b27a2abb8c59cf37f57837c802ef00180363
    tree 10bad8e / 再現性実験:   3f0ea82c6539464df2081a57f938f750f640dee55ea35a446386433598c56843
    初回（未pin・未`--locked`）: sha256 prefix 91dd34e9fec0a228
  sizeは、#74でtoolchain名を`esp`から`esp-1.95.0.0`へ変えるまでの8回の実行すべてで
  13,654,676 bytesであった。#74後の2回はいずれも13,660,584 bytes（+5,908）である。
  この2回はtreeが違う（cdcff41c と cf3fcdb4）が、差分は`rust-toolchain.toml`の
  comment行だけであり、sizeは一致してsha256は異なった。
  成果物の文字列には`toolchains/esp-1.95.0.0`を含むものが371件あり、
  toolchain名が9文字伸びたことによる増分は371×9=3,339 bytesとなる。
  増分の向きと桁は説明できるが、**残る2,569 bytesは説明できていない。**
  debug_infoのstring tableやsectionのalignmentが関与しうるが切り分けておらず、
  ここでも原因は断定しない。
  **記録した入力を揃えてもsha256は一致しなかった（実測）。**
  再現性実験では、source tree、toolchain、gcc、path、`IDF_PATH`の有無、command列を
  最終検証と同一にしたうえで`cargo clean`から再buildしたが、sha256は異なった。
  比較対象は最終検証と再現性実験の2回（いずれもtree 10bad8e）である。
  **これは「全入力が同一」の証明ではない。** 揃えたのは上に列挙した記録済みの項目だけで、
  `.embuild`配下のESP-IDFとmanaged toolの実体、Cargoが渡す環境変数の全体、
  build時刻などは照合していない。したがって
  「同一入力なら成果物が一致する／しない」を結論づける根拠にはならない。
  記録できるのは、これらの項目を揃えても一致しなかったという事実だけである。
  **この差分の原因は特定していない。** 断定に足る切り分けを行っていないため、
  ここでは測定事実だけを記録する。
  実務上の帰結として、sha256は特定の実行を識別する値であり、
  sourceやbuild入力の同一性判定には使えない。
  sourceの同一性は`Build対象のsource tree`で判定する。
Log or evidence path: この文書へ全文を掲載する。
  各logがどのsource treeで実行されたかを併記する。treeが異なるlogを同一条件の
  証拠として読まないこと。
  「Build log」節: 初回実行。**source treeは未確立**。歴史的証拠
  「Clippy log」節: 初回実行。**source treeは未確立**
  「最終検証log」節: 正式なcommandによる実行。tree 10bad8e
  「再現性実験log」節: 最終検証と同一入力での再実行。tree 10bad8e

  初回のBuild logとClippy logにtree hashを割り当てない。これらは未commitの作業treeで
  実行しており、`Cargo.lock`はその実行中に生成された。commit後に採取したtree 10bad8e が
  build開始時のfileと1 byteも違わないことを示す証拠は無い。
  記録できるのはrepository commit 109f1a90... と「working treeがcleanでなかった」事実だけである。
  「`force = true` の実効性検証log」節: 環境変数に対する優先の確認。tree 58ea4794
  「`force = true` 追加後のフル再検証log」節: 正式なcommand実行。tree 58ea4794
  「rebase後のフル再検証log」節: 正式なcommand実行。tree 9e48f340
  「review反映後のフル再検証log」節: 正式なcommand実行。tree 79351378
  「board識別訂正後のフル再検証log」節: 正式なcommand実行。tree b0120755
  「表記統一後のフル再検証log」節: 正式なcommand実行。tree 815c903f
  「版付きtoolchainの導入log（#74 追記）」節: `espup`による導入と`export-esp.sh`の
    再生成確認。treeに依存しない
  「compiler版の固定後のフル再検証log（#74 追記）」節: 現行treeでの正式なcommand実行。
    tree cf3fcdb4。同節の後半に、記録と異なるtoolchainを要求した場合の停止も含む
  記録本体が挙げるsha256 0f9d4918... は最後の節の実行結果である。
Known differences from documented profile:
  - 本記録はLinux（bash）で実行した。runbookのWindows節はADR-0005により対象外となり、
    PowerShell表記のcommand例も残っていない
  - espupは既定の`--targets all`ではなく`--targets esp32`に絞った（ADR-0002の最小環境方針）。
    runbookもこの形を正式なcommandとして記載している
  - ldproxyは`--version`を実装しないため、版は`cargo install --list`で確認した
  - 初回のbuild log採取は`--locked`と版pinを付けずに実行した。review指摘を受けて
    正式なcommandを`--locked`付き・版pin付きへ改めた。
    「Build log」節は初回採取時のもので、command行が現在の正式なcommandと異なるため、
    ESP-IDF全体をcompileした唯一のclean buildの記録として歴史的証拠に位置づける。
    正式なcommandに対応する証拠は「最終検証log」節にある
Conclusion: Partial。build-onlyの検証項目は全て成功したが、実機確認と別端末での再現が未実施
Next action: HW-TBD-001（物理基板・module・revision）の確認と、別のESP32 Build端末での再現
```

## 確定した依存版（lockfile由来）

`firmware/esp32/Cargo.lock`に固定した**184個の依存**のうち、主要なものは次である。
build logの`Locking 184 packages`はこの依存数を指す。`Cargo.lock`の`[[package]]`は、
root crateの`deskcat-esp32`自身を含むため185 entryになる。両者は矛盾しない。

| Crate | 確定版 |
|---|---|
| `esp-idf-svc` | 0.52.1 |
| `esp-idf-hal` | 0.46.2 |
| `esp-idf-sys` | 0.37.2 |
| `embedded-svc` | 0.29.0 |
| `embuild`（build-dependency） | 0.33.3 |
| `embassy-time` | 0.5.1 |
| `log` | 0.4.33 |

## 生成条件

[esp-idf-template](https://github.com/esp-rs/esp-idf-template)のreview済みcommitから、次の入力で生成した。

| 質問 | 指定値 |
|---|---|
| MCU | `esp32` |
| Advanced options | `true` |
| ESP-IDF version | `v5.5.3` |
| Git crates | `false` |
| ESP-IDF tools install directory | `workspace` |
| Wokwi | `false` |
| Dev Container | `false` |
| CI | `false` |

生成後のreviewで加えた変更は次の4点である。

1. `Cargo.toml`の`authors`を削除した。生成時にgit設定の個人メールが埋め込まれるためで、edition 2021では任意項目である。あわせて`license = "MIT"`と`publish = false`を明示した。
2. `.gitignore`から`/Cargo.lock`の除外を外した。applicationのlockfileは追跡する方針である。
3. `cargo generate`が作成したnestedな`.git`と、空の`.vscode`を削除した。
4. `.cargo/config.toml`の`[env]`へ`force = true`を付けた。Cargoは既定では
   「既に環境に存在する変数を上書きしない」（[公式仕様](https://doc.rust-lang.org/cargo/reference/config.html)）ため、
   templateの記法のままでは、開発者のshellに`ESP_IDF_VERSION`や
   `ESP_IDF_TOOLS_INSTALL_DIR`が設定されていると固定値が黙って無視される。
   環境変数によるSDK差し替えを防ぐための変更である。

## 未達の確定条件

[ESP32 Rust Toolchain](../esp32-rust-toolchain.md)の確定条件のうち、次は本記録で満たしていない。

**この節は実施時点の状態である。** 物理基板に関する項目はその後
[#55](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55)で確定し、
未確定の範囲はchip刻印の読み取りと、回路図・現物pin表記の照合へ縮小した。
現在の状態は末尾の「[boardの識別に関する訂正](#boardの識別に関する訂正2026-08-08-追記)」節を参照する。
確定条件のcheckbox自体は`esp32-rust-toolchain.md`が正本であり、そちらは更新済みである。

- **物理基板がESP32-DevKitC-32Eであり、搭載moduleとrevisionを確認した** — 実機が無いため未実施。[HW-TBD-001](../../hardware/tbd-register.md)として追跡中であり、#6 のflash前提条件でもある。
  公式datasheetによればESP32-WROOM-32Eの中核は`ESP32-D0WD-V3`または`ESP32-D0WDR2-V3`であり、後者は2 MB PSRAMを内蔵する。どちらが実装されているかで利用可能なメモリ構成が変わるため、実機確認まで確定しない。
- **別の開発端末またはclean environmentで再現した** — 本記録は単一端末の結果である。[Machine Profiles](../machine-profiles.md)の「検証の移送」に従い、別端末では再確認が必要である。
  **（2026-08-11 追記: この条件はその後満たされた。**CIの`ubuntu-24.04` runnerでclean buildし、
  `cargo fmt`・`cargo clippy --locked`・`cargo build --locked`がすべて成功している（#42、
  [CIのVersion Record](2026-08-10-esp32-build-ci.md)）。**build-onlyであり、flashと実機起動は
  依然として主張しない。**上の本文は実施時点の記録として残す。確定条件のcheckboxは
  [ESP32 Rust Toolchain](../esp32-rust-toolchain.md)が正本であり、そちらは`[x]`である。**）**

## Build log

**初回採取分（歴史的証拠）。** ESP-IDF本体をcompileした唯一のclean buildの記録である。
`--locked`と版pinを付ける前のcommandで実行したため、command行は現在の正式なcommandと異なる。
現行treeの正式なcommandに対応する証拠は「[review反映後のフル再検証log](#review反映後のフル再検証log)」節にある。

`cargo fmt --all -- --check`と`cargo build`の全出力である。個人pathは`<home>`へ置換した。

```text
########## DeskCat M1-001 clean build ##########
date: 2026-08-06T05:13:49Z
repo commit: 109f1a90d9688542f9381fb95d4269ef336e23dd
--- SDK override check ---
IDF_PATH=<unset>
IDF_TOOLS_PATH=<unset>
ESP_IDF_TOOLS_INSTALL_DIR=<unset>
--- toolchain ---
esp (overridden by '<home>/deskcat/firmware/esp32/rust-toolchain.toml')
--- cargo fmt --all -- --check ---
FMT_OK
--- cargo build ---
    Updating crates.io index
     Locking 184 packages to latest compatible versions
 Downloading crates ...
  Downloaded cfg_aliases v0.2.2
  Downloaded build-time v0.1.3
  Downloaded bitflags v2.13.1
  Downloaded embedded-io-async v0.6.1
  Downloaded embassy-time-driver v0.2.2
  Downloaded embedded-io-async v0.7.0
  Downloaded embedded-hal-async v1.0.0
  Downloaded embedded-hal-nb v1.0.0
  Downloaded embassy-time-queue-utils v0.3.2
  Downloaded strum v0.24.1
  Downloaded either v1.17.0
  Downloaded uncased v0.9.10
  Downloaded void v1.0.2
  Downloaded which v4.4.2
  Downloaded strum v0.27.2
  Downloaded rustc-hash v2.1.3
  Downloaded zmij v1.0.23
  Downloaded thiserror-impl v2.0.19
  Downloaded strum_macros v0.27.2
  Downloaded rustversion v1.0.23
  Downloaded camino v1.2.5
  Downloaded thiserror v2.0.19
  Downloaded num_enum_derive v0.7.6
  Downloaded cc v1.4.0
  Downloaded toml_parser v1.1.3+spec-1.1.0
  Downloaded serde_core v1.0.229
  Downloaded toml_edit v0.25.13+spec-1.1.0
  Downloaded minimal-lexical v0.2.1
  Downloaded nom v7.1.3
  Downloaded heapless v0.9.3
  Downloaded itertools v0.13.0
  Downloaded winnow v1.0.4
  Downloaded serde_json v1.0.151
  Downloaded chrono v0.4.45
  Downloaded syn v1.0.109
  Downloaded bindgen v0.71.1
  Downloaded konst v0.2.20
  Downloaded esp-idf-svc v0.52.1
  Downloaded syn v3.0.3
  Downloaded esp-idf-hal v0.46.2
  Downloaded esp-idf-sys v0.37.2
  Downloaded serde v1.0.229
  Downloaded rustix v0.38.44
  Downloaded prettyplease v0.2.37
  Downloaded ignore v0.4.33
  Downloaded darling_core v0.21.3
  Downloaded serde_derive v1.0.229
  Downloaded proc-macro2 v1.0.107
  Downloaded fastrand v2.5.0
  Downloaded quote v1.0.47
  Downloaded glob v0.3.4
  Downloaded enumset v1.1.14
  Downloaded num_enum v0.7.6
  Downloaded globwalk v0.8.1
  Downloaded regex-automata v0.4.18
  Downloaded enumset_derive v0.15.0
  Downloaded libloading v0.8.9
  Downloaded iana-time-zone v0.1.65
  Downloaded embuild v0.33.3
  Downloaded globset v0.4.20
  Downloaded embedded-hal v1.0.0
  Downloaded critical-section v1.2.0
  Downloaded strum_macros v0.24.3
  Downloaded remove_dir_all v0.8.4
  Downloaded proc-macro-crate v3.5.0
  Downloaded futures-core v0.3.33
  Downloaded unicode-xid v0.2.6
  Downloaded libc v0.2.189
  Downloaded konst_macro_rules v0.2.19
  Downloaded futures-io v0.3.33
  Downloaded heck v0.4.1
  Downloaded envy v0.4.2
  Downloaded clang-sys v1.9.1
  Downloaded nb v0.1.3
  Downloaded embedded-io v0.7.1
  Downloaded nb v1.1.0
  Downloaded ident_case v1.0.1
  Downloaded futures-sink v0.3.33
  Downloaded embedded-svc v0.29.0
  Downloaded embedded-io v0.6.1
  Downloaded embedded-hal v0.2.7
  Downloaded embassy-time v0.5.1
  Downloaded darling v0.21.3
  Downloaded cargo_metadata v0.18.1
  Downloaded embedded-can v0.4.1
  Downloaded cargo-platform v0.1.9
  Downloaded aho-corasick v1.1.5
  Downloaded embassy-sync v0.7.2
  Downloaded embassy-futures v0.1.2
  Downloaded darling_macro v0.21.3
  Downloaded const_format_proc_macros v0.2.34
  Downloaded const_format v0.2.36
  Downloaded anyhow v1.0.104
  Downloaded embassy-executor-timer-queue v0.1.0
  Downloaded cexpr v0.6.0
  Downloaded linux-raw-sys v0.4.15
    Updating crates.io index
 Downloading crates ...
  Downloaded foldhash v0.2.0
  Downloaded rustc-literal-escaper v0.0.7
  Downloaded memchr v2.7.6
  Downloaded getopts v0.2.24
  Downloaded hashbrown v0.16.1
  Downloaded libc v0.2.178
   Compiling compiler_builtins v0.1.160 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/compiler-builtins/compiler-builtins)
   Compiling core v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/core)
   Compiling libc v0.2.178
   Compiling object v0.37.3
   Compiling std v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/std)
   Compiling proc-macro2 v1.0.107
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.47
   Compiling memchr v2.8.3
   Compiling libc v0.2.189
   Compiling cfg-if v1.0.4
   Compiling serde_core v1.0.229
   Compiling bitflags v2.13.1
   Compiling crossbeam-utils v0.8.22
   Compiling syn v2.0.119
   Compiling syn v3.0.3
   Compiling aho-corasick v1.1.5
   Compiling regex-syntax v0.8.11
   Compiling cfg_aliases v0.2.2
   Compiling glob v0.3.4
   Compiling rustversion v1.0.23
   Compiling crossbeam-epoch v0.9.20
   Compiling regex-automata v0.4.18
   Compiling serde v1.0.229
   Compiling clang-sys v1.9.1
   Compiling nix v0.29.0
   Compiling serde_derive v1.0.229
   Compiling syn v1.0.109
   Compiling log v0.4.33
   Compiling prettyplease v0.2.37
   Compiling crossbeam-deque v0.8.7
   Compiling zmij v1.0.23
   Compiling bstr v1.13.0
   Compiling rustix v0.38.44
   Compiling either v1.17.0
   Compiling minimal-lexical v0.2.1
   Compiling rustix v1.1.4
   Compiling same-file v1.0.6
   Compiling thiserror v1.0.69
   Compiling serde_json v1.0.151
   Compiling getrandom v0.4.3
   Compiling walkdir v2.5.0
   Compiling nom v7.1.3
   Compiling globset v0.4.20
   Compiling thiserror-impl v1.0.69
   Compiling libloading v0.8.9
   Compiling cvt v0.1.2
   Compiling find-msvc-tools v0.1.9
   Compiling heck v0.4.1
   Compiling itoa v1.0.18
   Compiling once_cell v1.21.4
   Compiling shlex v2.0.1
   Compiling anyhow v1.0.104
   Compiling linux-raw-sys v0.12.1
   Compiling bindgen v0.71.1
   Compiling linux-raw-sys v0.4.15
   Compiling fs_at v0.2.1
   Compiling cc v1.4.0
   Compiling cexpr v0.6.0
   Compiling ignore v0.4.33
   Compiling itertools v0.13.0
   Compiling regex v1.13.1
   Compiling strum_macros v0.24.3
   Compiling home v0.5.12
   Compiling fastrand v2.5.0
   Compiling bitflags v1.3.2
   Compiling rustc-hash v2.1.3
   Compiling shlex v1.3.0
   Compiling normpath v1.5.1
   Compiling remove_dir_all v0.8.4
   Compiling globwalk v0.8.1
   Compiling tempfile v3.27.0
   Compiling strum v0.24.1
   Compiling which v4.4.2
   Compiling cmake v0.1.58
   Compiling filetime v0.2.29
   Compiling camino v1.2.5
   Compiling cargo-platform v0.1.9
   Compiling semver v1.0.28
   Compiling envy v0.4.2
   Compiling autocfg v1.5.1
   Compiling ident_case v1.0.1
   Compiling fnv v1.0.7
   Compiling darling_core v0.21.3
   Compiling cargo_metadata v0.18.1
   Compiling num-traits v0.2.19
   Compiling heapless v0.9.3
   Compiling embassy-time-queue-utils v0.3.2
   Compiling embassy-time-driver v0.2.2
   Compiling embedded-hal-async v1.0.0
   Compiling iana-time-zone v0.1.65
   Compiling embedded-io-async v0.6.1
   Compiling heapless v0.8.0
   Compiling unicode-xid v0.2.6
   Compiling const_format_proc_macros v0.2.34
   Compiling embuild v0.33.3
   Compiling chrono v0.4.45
   Compiling litrs v1.0.0
   Compiling darling_macro v0.21.3
   Compiling embassy-sync v0.7.2
   Compiling version_check v0.9.5
   Compiling darling v0.21.3
   Compiling enumset_derive v0.15.0
   Compiling build-time v0.1.3
   Compiling uncased v0.9.10
   Compiling document-features v0.2.12
   Compiling num_enum_derive v0.7.6
   Compiling esp-idf-sys v0.37.2
   Compiling esp-idf-hal v0.46.2
   Compiling esp-idf-svc v0.52.1
   Compiling deskcat-esp32 v0.1.0 (<home>/deskcat/firmware/esp32)
   Compiling rustc-std-workspace-core v1.99.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/rustc-std-workspace-core)
   Compiling alloc v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/alloc)
   Compiling unwind v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/unwind)
   Compiling adler2 v2.0.1
   Compiling memchr v2.7.6
   Compiling rustc-demangle v0.1.27
   Compiling panic_abort v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/panic_abort)
   Compiling rustc-literal-escaper v0.0.7
   Compiling rustc-std-workspace-alloc v1.99.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/rustc-std-workspace-alloc)
   Compiling panic_unwind v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/panic_unwind)
   Compiling gimli v0.32.3
   Compiling miniz_oxide v0.8.9
   Compiling hashbrown v0.16.1
   Compiling std_detect v0.1.5 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/std_detect)
   Compiling addr2line v0.25.1
   Compiling proc_macro v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/proc_macro)
   Compiling byteorder v1.5.0
   Compiling stable_deref_trait v1.2.1
   Compiling nb v1.1.0
   Compiling hash32 v0.3.1
   Compiling embedded-hal v1.0.0
   Compiling konst_macro_rules v0.2.19
   Compiling konst v0.2.20
   Compiling nb v0.1.3
   Compiling embedded-io v0.7.1
   Compiling embedded-io v0.6.1
   Compiling futures-core v0.3.33
   Compiling critical-section v1.2.0
   Compiling void v1.0.2
   Compiling embedded-hal v0.2.7
   Compiling embedded-io-async v0.7.0
   Compiling const_format v0.2.36
   Compiling enumset v1.1.14
   Compiling futures-sink v0.3.33
   Compiling embassy-executor-timer-queue v0.1.0
   Compiling embedded-hal-nb v1.0.0
   Compiling embedded-can v0.4.1
   Compiling atomic-waker v1.1.2
   Compiling embassy-futures v0.1.2
   Compiling futures-io v0.3.33
   Compiling num_enum v0.7.6
   Compiling embassy-time v0.5.1
   Compiling embedded-svc v0.29.0
    Finished `dev` profile [optimized + debuginfo] target(s) in 4m 33s

real	4m34.393s
user	10m2.848s
sys	1m21.059s
BUILD_RC=0
```

## Clippy log

ESP-IDF targetでClippyが動作することを実測した結果である。

```text
--- cargo clippy --all-targets -- -D warnings ---
    Checking byteorder v1.5.0
    Checking stable_deref_trait v1.2.1
    Checking nb v1.1.0
    Checking embedded-hal v1.0.0
    Checking konst_macro_rules v0.2.19
    Checking critical-section v1.2.0
    Checking futures-core v0.3.33
    Checking nb v0.1.3
    Checking void v1.0.2
    Checking cfg-if v1.0.4
    Checking embedded-io v0.7.1
    Checking embedded-io v0.6.1
    Checking embedded-hal v0.2.7
    Checking konst v0.2.20
    Checking embedded-hal-async v1.0.0
    Checking embedded-io-async v0.6.1
    Checking hash32 v0.3.1
    Checking embedded-io-async v0.7.0
    Checking enumset v1.1.14
    Checking heapless v0.9.3
    Checking heapless v0.8.0
    Checking serde_core v1.0.229
    Checking const_format v0.2.36
    Checking libc v0.2.189
    Checking futures-sink v0.3.33
    Checking embassy-executor-timer-queue v0.1.0
    Checking esp-idf-sys v0.37.2
    Checking embedded-hal-nb v1.0.0
    Checking embassy-time-driver v0.2.2
    Checking embedded-can v0.4.1
    Checking atomic-waker v1.1.2
    Checking embassy-sync v0.7.2
    Checking log v0.4.33
    Checking num_enum v0.7.6
    Checking uncased v0.9.10
   Compiling deskcat-esp32 v0.1.0 (<home>/deskcat/firmware/esp32)
    Checking embassy-futures v0.1.2
    Checking embassy-time-queue-utils v0.3.2
    Checking futures-io v0.3.33
    Checking embassy-time v0.5.1
    Checking serde v1.0.229
    Checking embedded-svc v0.29.0
    Checking esp-idf-hal v0.46.2
    Checking esp-idf-svc v0.52.1
    Finished `dev` profile [optimized + debuginfo] target(s) in 10.51s

real	0m10.603s
user	0m19.504s
sys	0m3.175s
CLIPPY_RC=0
```

## 最終検証log

`--locked`と版pinを付けた正式なcommandによる実行である。`cargo clean`後に再buildし、
成果物のsha256、lockfileの状態、各commandのexit code、source treeを同一transcriptへ収めた。

**この節は履歴である。** tree 10bad8e に対する実行であり、節名の「最終」は採取当時の
呼称にすぎない。現行treeの証拠は「[review反映後のフル再検証log](#review反映後のフル再検証log)」節である。

個人pathは`<home>`へ置換した。

```text
########## DeskCat M1-001 最終検証（pin + --locked） ##########
date: 2026-08-06T08:37:50Z
source tree (git rev-parse HEAD:firmware/esp32): 10bad8e38f8fb8305eaee381d49a4c7e8bc19192
--- SDK override check ---
IDF_PATH=<unset>
IDF_TOOLS_PATH=<unset>
--- toolchain identity ---
esp (overridden by '<home>/deskcat/firmware/esp32/rust-toolchain.toml')
rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
binary: rustc
xtensa-esp-elf-gcc (crosstool-NG esp-15.2.0_20250920) 15.2.0
--- cargo clean ---
     Removed 3667 files, 1.7GiB total
clean rc=0
--- cargo fmt --all -- --check ---
fmt rc=0
--- cargo clippy --all-targets --locked -- -D warnings ---
   Compiling compiler_builtins v0.1.160 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/compiler-builtins/compiler-builtins)
   Compiling core v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/core)
   Compiling libc v0.2.178
   Compiling object v0.37.3
   Compiling std v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/std)
   Compiling proc-macro2 v1.0.107
   Compiling quote v1.0.47
   Compiling unicode-ident v1.0.24
   Compiling memchr v2.8.3
   Compiling cfg-if v1.0.4
   Compiling libc v0.2.189
   Compiling serde_core v1.0.229
   Compiling bitflags v2.13.1
   Compiling crossbeam-utils v0.8.22
   Compiling aho-corasick v1.1.5
   Compiling regex-syntax v0.8.11
   Compiling syn v2.0.119
   Compiling syn v3.0.3
   Compiling regex-automata v0.4.18
   Compiling cfg_aliases v0.2.2
   Compiling serde v1.0.229
   Compiling rustversion v1.0.23
   Compiling crossbeam-epoch v0.9.20
   Compiling glob v0.3.4
   Compiling nix v0.29.0
   Compiling clang-sys v1.9.1
   Compiling log v0.4.33
   Compiling zmij v1.0.23
   Compiling crossbeam-deque v0.8.7
   Compiling syn v1.0.109
   Compiling prettyplease v0.2.37
   Compiling serde_derive v1.0.229
   Compiling bstr v1.13.0
   Compiling minimal-lexical v0.2.1
   Compiling getrandom v0.4.3
   Compiling thiserror v1.0.69
   Compiling same-file v1.0.6
   Compiling rustix v1.1.4
   Compiling serde_json v1.0.151
   Compiling rustix v0.38.44
   Compiling either v1.17.0
   Compiling walkdir v2.5.0
   Compiling nom v7.1.3
   Compiling globset v0.4.20
   Compiling libloading v0.8.9
   Compiling cvt v0.1.2
   Compiling shlex v2.0.1
   Compiling bindgen v0.71.1
   Compiling linux-raw-sys v0.12.1
   Compiling thiserror-impl v1.0.69
   Compiling heck v0.4.1
   Compiling find-msvc-tools v0.1.9
   Compiling itoa v1.0.18
   Compiling linux-raw-sys v0.4.15
   Compiling anyhow v1.0.104
   Compiling once_cell v1.21.4
   Compiling cc v1.4.0
   Compiling fs_at v0.2.1
   Compiling strum_macros v0.24.3
   Compiling cexpr v0.6.0
   Compiling ignore v0.4.33
   Compiling itertools v0.13.0
   Compiling regex v1.13.1
   Compiling home v0.5.12
   Compiling shlex v1.3.0
   Compiling bitflags v1.3.2
   Compiling fastrand v2.5.0
   Compiling normpath v1.5.1
   Compiling rustc-hash v2.1.3
   Compiling remove_dir_all v0.8.4
   Compiling tempfile v3.27.0
   Compiling globwalk v0.8.1
   Compiling which v4.4.2
   Compiling strum v0.24.1
   Compiling cmake v0.1.58
   Compiling filetime v0.2.29
   Compiling camino v1.2.5
   Compiling cargo-platform v0.1.9
   Compiling semver v1.0.28
   Compiling envy v0.4.2
   Compiling ident_case v1.0.1
   Compiling fnv v1.0.7
   Compiling autocfg v1.5.1
   Compiling darling_core v0.21.3
   Compiling cargo_metadata v0.18.1
   Compiling num-traits v0.2.19
   Compiling heapless v0.9.3
   Compiling embassy-time-queue-utils v0.3.2
   Compiling embassy-time-driver v0.2.2
   Compiling heapless v0.8.0
   Compiling iana-time-zone v0.1.65
   Compiling embedded-io-async v0.6.1
   Compiling embedded-hal-async v1.0.0
   Compiling unicode-xid v0.2.6
   Compiling const_format_proc_macros v0.2.34
   Compiling chrono v0.4.45
   Compiling embuild v0.33.3
   Compiling embassy-sync v0.7.2
   Compiling version_check v0.9.5
   Compiling litrs v1.0.0
   Compiling darling_macro v0.21.3
   Compiling build-time v0.1.3
   Compiling uncased v0.9.10
   Compiling darling v0.21.3
   Compiling enumset_derive v0.15.0
   Compiling document-features v0.2.12
   Compiling num_enum_derive v0.7.6
   Compiling esp-idf-sys v0.37.2
   Compiling esp-idf-hal v0.46.2
   Compiling esp-idf-svc v0.52.1
   Compiling deskcat-esp32 v0.1.0 (<home>/deskcat/firmware/esp32)
   Compiling rustc-std-workspace-core v1.99.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/rustc-std-workspace-core)
   Compiling alloc v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/alloc)
   Compiling memchr v2.7.6
   Compiling unwind v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/unwind)
   Compiling adler2 v2.0.1
   Compiling panic_abort v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/panic_abort)
   Compiling rustc-demangle v0.1.27
   Compiling rustc-literal-escaper v0.0.7
   Compiling rustc-std-workspace-alloc v1.99.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/rustc-std-workspace-alloc)
   Compiling panic_unwind v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/panic_unwind)
   Compiling gimli v0.32.3
   Compiling miniz_oxide v0.8.9
   Compiling std_detect v0.1.5 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/std_detect)
   Compiling hashbrown v0.16.1
   Compiling addr2line v0.25.1
   Compiling proc_macro v0.0.0 (<home>/.rustup/toolchains/esp/lib/rustlib/src/rust/library/proc_macro)
    Checking byteorder v1.5.0
    Checking stable_deref_trait v1.2.1
    Checking nb v1.1.0
    Checking konst_macro_rules v0.2.19
    Checking embedded-hal v1.0.0
    Checking futures-core v0.3.33
    Checking nb v0.1.3
    Checking critical-section v1.2.0
    Checking konst v0.2.20
    Checking embedded-io v0.6.1
    Checking void v1.0.2
    Checking embedded-io v0.7.1
    Checking hash32 v0.3.1
    Checking embedded-hal v0.2.7
    Checking const_format v0.2.36
    Checking embedded-io-async v0.7.0
    Checking embassy-executor-timer-queue v0.1.0
    Checking enumset v1.1.14
    Checking futures-sink v0.3.33
    Checking embedded-hal-nb v1.0.0
    Checking embedded-can v0.4.1
    Checking atomic-waker v1.1.2
    Checking num_enum v0.7.6
    Checking futures-io v0.3.33
    Checking embassy-futures v0.1.2
    Checking embassy-time v0.5.1
    Checking embedded-svc v0.29.0
    Finished `dev` profile [optimized + debuginfo] target(s) in 2m 34s

real	2m34.985s
user	9m3.039s
sys	1m34.831s
clippy rc=0
--- cargo build --locked ---
   Compiling byteorder v1.5.0
   Compiling stable_deref_trait v1.2.1
   Compiling nb v1.1.0
   Compiling konst_macro_rules v0.2.19
   Compiling embedded-hal v1.0.0
   Compiling embedded-io v0.7.1
   Compiling embedded-io v0.6.1
   Compiling nb v0.1.3
   Compiling void v1.0.2
   Compiling konst v0.2.20
   Compiling critical-section v1.2.0
   Compiling cfg-if v1.0.4
   Compiling futures-core v0.3.33
   Compiling hash32 v0.3.1
   Compiling embedded-io-async v0.7.0
   Compiling embedded-hal v0.2.7
   Compiling embedded-hal-async v1.0.0
   Compiling heapless v0.9.3
   Compiling heapless v0.8.0
   Compiling const_format v0.2.36
   Compiling embedded-io-async v0.6.1
   Compiling enumset v1.1.14
   Compiling libc v0.2.189
   Compiling serde_core v1.0.229
   Compiling embassy-executor-timer-queue v0.1.0
   Compiling futures-sink v0.3.33
   Compiling esp-idf-sys v0.37.2
   Compiling embassy-sync v0.7.2
   Compiling embassy-time-driver v0.2.2
   Compiling embedded-hal-nb v1.0.0
   Compiling embedded-can v0.4.1
   Compiling atomic-waker v1.1.2
   Compiling log v0.4.33
   Compiling embassy-time-queue-utils v0.3.2
   Compiling num_enum v0.7.6
   Compiling uncased v0.9.10
   Compiling deskcat-esp32 v0.1.0 (<home>/deskcat/firmware/esp32)
   Compiling futures-io v0.3.33
   Compiling embassy-futures v0.1.2
   Compiling embassy-time v0.5.1
   Compiling serde v1.0.229
   Compiling embedded-svc v0.29.0
   Compiling esp-idf-hal v0.46.2
   Compiling esp-idf-svc v0.52.1
    Finished `dev` profile [optimized + debuginfo] target(s) in 20.69s

real	0m21.261s
user	0m42.065s
sys	0m5.270s
build rc=0
--- artifact ---
size_bytes=13654676
target/xtensa-esp32-espidf/debug/deskcat-esp32: ELF 32-bit LSB executable, Tensilica Xtensa, version 1 (SYSV), statically linked, with debug_info, not stripped
b1cb851bdf43ff2449ad394ac234b27a2abb8c59cf37f57837c802ef00180363  target/xtensa-esp32-espidf/debug/deskcat-esp32
--- lockfile state ---
Cargo.lock unchanged
```

## 再現性実験log

成果物のsha256が実行ごとに異なる件について、入力を揃えて比較した記録である。
source tree、toolchain、gcc、path、`IDF_PATH`の有無、command列を「最終検証log」と
同一にしたうえで、`cargo clean`から再buildした。個人pathは`<home>`へ置換した。

```text
########## 再現性実験: 同一入力で2回目 ##########
date: 2026-08-06T12:20:52Z
source tree: 10bad8e38f8fb8305eaee381d49a4c7e8bc19192
toolchain  : rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
gcc        : xtensa-esp-elf-gcc (crosstool-NG esp-15.2.0_20250920) 15.2.0
path       : <home>/deskcat/firmware/esp32
IDF_PATH   : <unset>
--- 最終検証と同一のcommand列 ---
     Removed 3648 files, 1.7GiB total
clean rc=0
fmt rc=0
clippy rc=0
build rc=0
--- 結果 ---
size_bytes=13654676
sha256=3f0ea82c6539464df2081a57f938f750f640dee55ea35a446386433598c56843
Cargo.lock unchanged
```

結果として、sizeは同一のまま sha256 だけが異なった。
**差分の原因は特定していない。**入力を揃えても一致しないという測定事実のみを記録する。

## `force = true` の実効性検証log

Cargoの`[env]`は既定では既存の環境変数を上書きしない。`force = true`を付けた効果を、
設定値の確認と実際のbuild挙動の両方で検証した記録である。個人pathは`<home>`へ置換した。

```text
=== A) cargo config get（unstable option付き） ===
env.ESP_IDF_TOOLS_INSTALL_DIR.force = true
env.ESP_IDF_TOOLS_INSTALL_DIR.value = "workspace"
env.ESP_IDF_VERSION.force = true
env.ESP_IDF_VERSION.value = "v5.5.3"
env.MCU.force = true
env.MCU.value = "esp32"

=== B) 機能的検証: shellに存在しないESP-IDF版を設定してbuild ===
設定: ESP_IDF_VERSION=v9.9.9（存在しないtag）
   Compiling deskcat-esp32 v0.1.0 (<home>/deskcat/firmware/esp32)
    Finished `dev` profile [optimized + debuginfo] target(s) in 6.58s
build rc=0

=== 判定 ===
buildが成功 → config.tomlのv5.5.3が勝っている（force有効）
v9.9.9を取りに行って失敗 → forceが効いていない
```

**一次的な根拠は A) の`cargo config get env`である。** `force = true`と`value = "v5.5.3"`が
解決済み設定として返っており、Cargoがこの値を適用することを示す。

B) のbuildは補助的な観察にとどめる。shellへ存在しない`ESP_IDF_VERSION=v9.9.9`を設定し、
`build.rs`をtouchしてbuild scriptを再実行させたうえでbuildしrc 0で完了したが、
**これは単独では証明にならない。** `.embuild`配下にv5.5.3が既に展開済みであり、
cacheを再利用しただけでも同じ結果になりうる。build scriptが実際に読んだ環境変数の値を
出力させていないため、この実行だけでは環境変数が上書きされたと断定できない。

より強い証拠が必要な場合は、`.embuild`を空にした状態で同じ実験を行うか、
build scriptに環境変数の実効値を出力させて確認する。本記録の範囲では実施していない。

## `force = true` 追加後のフル再検証log

`force = true`によりsource treeが`58ea47942b4b190b840b6ad2d2a2e82442519a05`へ変わったため、
`cargo clean`から正式なcommandを再実行した記録である。

**この節は履歴である。** 生成したsha256 `6a541ff8...`は当時のtree 58ea4794 に対応する。
その後 tree は複数回変わっている。現行 tree に対応する成果物は記録本体の
`Generated artifact identity` を参照する。この節の sha256 は当時の実行だけを識別する。

個人pathは`<home>`へ置換した。

```text
########## force=true 追加後のフル再検証 ##########
date: 2026-08-07T01:50:18Z
--- force=true の実効性確認 ---
shellへ意図的に誤った値を設定して、config.tomlが上書きするか見る
shell側: ESP_IDF_VERSION=v9.9.9 ESP_IDF_TOOLS_INSTALL_DIR=global
cargoが解決する実効値: 
--- 通常環境で再検証 ---
IDF_PATH=<unset>
esp (overridden by '<home>/deskcat/firmware/esp32/rust-toolchain.toml')
     Removed 3648 files, 1.7GiB total
clean rc=0
fmt rc=0
clippy rc=0
build rc=0
--- 結果 ---
size_bytes=13654676
sha256=6a541ff8de3c66c18856397933f87ce26d4b95bae0d2ceb01b8dc361e9bd5c47
Cargo.lock unchanged
```

## rebase後のフル再検証log

`develop`（#44／#45／#46 反映後）へ rebase したことで source tree が
`9e48f340bec4b618225776247bb71dc72943d153` へ変わったため、`cargo clean` から
正式な command を再実行した記録である。

**この節は履歴である。** 生成した sha256 `678cc201...` は当時の tree 9e48f340 に対応する。
その後 tree は複数回変わっている。現行 tree に対応する成果物は記録本体の
`Generated artifact identity` を参照する。この節の sha256 は当時の実行だけを識別する。

個人 path は `<home>` へ置換した。

```text
########## rebase後のフル再検証 ##########
date: 2026-08-07T06:48:31Z
base: 9e886e1 開発環境の標準OSを記録し、runbookのWindows節を整理する (#46)
source tree: 9e48f340bec4b618225776247bb71dc72943d153
IDF_PATH=<unset>
esp (overridden by '<home>/deskcat/firmware/esp32/rust-toolchain.toml')
rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
xtensa-esp-elf-gcc (crosstool-NG esp-15.2.0_20250920) 15.2.0
--- 正式なcommand ---
     Removed 3668 files, 1.7GiB total
clean rc=0
fmt rc=0
clippy rc=0
build rc=0
--- 結果 ---
size_bytes=13654676
sha256=678cc201e60bb7376c38c19534d2e4819bdc40e68e20713bcb789b04d3376c24
Cargo.lock unchanged
```

## review反映後のフル再検証log

CodeRabbit の指摘反映で `firmware/esp32/README.md` が変わり、source tree が
`793513787144ddf7ba3d966c8fd82018b081ef11` へ変わったため、`cargo clean` から
正式な command を再実行した記録である。

**この節は履歴である。** この実行の成果物は sha256 `817d5594...` である。
現行 tree の成果物は記録本体の `Generated artifact identity` を参照する。

個人 path は `<home>` へ置換した。

```text
########## review反映後のフル再検証 ##########
date: 2026-08-08T02:00:59Z
base: 9e886e1 開発環境の標準OSを記録し、runbookのWindows節を整理する (#46)
source tree(staged): 793513787144ddf7ba3d966c8fd82018b081ef11
IDF_PATH=<unset>
esp (overridden by '<home>/deskcat/firmware/esp32/rust-toolchain.toml')
rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
xtensa-esp-elf-gcc (crosstool-NG esp-15.2.0_20250920) 15.2.0
--- 正式なcommand ---
     Removed 3648 files, 1.7GiB total
clean rc=0
fmt rc=0
clippy rc=0
build rc=0
--- 結果 ---
size_bytes=13654676
sha256=817d55948479097c0389db4bba08140aa7cca2c974c7a04258724ec3cadca442
Cargo.lock unchanged
```

## boardの識別に関する訂正（2026-08-08 追記）

本記録が前提としていた board の識別情報は誤りであった。訂正は
[#55](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55) で行われ、正本は
[hardware-bom.md](../../hardware/hardware-bom.md) の MCU-01 である。

| | 本記録の記載（誤） | 訂正後（正） |
|---|---|---|
| board | ESP32-DevKitC V4 | ESP-WROOM-32D 開発ボード（秋月電子 M-13628）。基板裏面 silkscreen `ESP32_DevkitC_V4` |
| module | ESP32-WROOM-32E | ESP-WROOM-32D |
| 中核 chip | `ESP32-D0WD-V3` または `ESP32-D0WDR2-V3` | `ESP32-D0WD` |

**上記より前の本文と log は、実施時点の記録として書き換えていない。**
記録は実行時の事実を保存する文書であり、後から得た知見で遡って改変しない。
現在の正しい識別情報は上表と正本文書を参照する。

build 結果への影響は無い。どちらも classic ESP32（Xtensa dual-core 32-bit LX6）であり、
Rust target は `xtensa-esp32-espidf` のまま変わらない。lockfile、toolchain、成果物の
いずれも再取得を要しない。

PSRAM に関する本記録の懸念も解消した。ESP-WROOM-32D の
[datasheet v2.7](https://documentation.espressif.com/esp32-wroom-32d_esp32-wroom-32u_datasheet_en.pdf)
には PSRAM を内蔵する variant の記載が無く、variant の差は antenna だけである（2026-08-08 取得）。

`Physical board` / `Module marking` / `Board revision` の各 field は「未確認」と記載しているが、
PR #55 の現物確認により機種と module は確定し、基板に revision 表示が無いことも確認された。
未確定として残るのは、回路図と現物 pin 表記の照合、および chip 刻印の読み取りである。

## board識別訂正後のフル再検証log

boardの識別情報を訂正して source tree が `b012075572c47fd622cd254003b87fa7329740d5` へ
変わったため、`cargo clean` から正式な command を再実行した記録である。
この実行の成果物は sha256 `62670701...` である。現行 tree の成果物は記録本体の
`Generated artifact identity` を参照する。
個人 path は `<home>` へ置換した。

```text
########## board識別訂正後のフル再検証 ##########
date: 2026-08-08T15:39:53Z
base: 3c53afe 手持ちhardwareを識別し、GPIO割り当てと電源budgetを下書きする (#55)
source tree(staged): b012075572c47fd622cd254003b87fa7329740d5
IDF_PATH=<unset>
esp (overridden by '<home>/deskcat/firmware/esp32/rust-toolchain.toml')
     Removed 3648 files, 1.7GiB total
clean rc=0
fmt rc=0
clippy rc=0
build rc=0
size_bytes=13654676
sha256=626707014889c5dc83ce0dc453383ee8139fa2d8feded452380fcfed21a96d97
Cargo.lock unchanged
```

## 表記統一後のフル再検証log

board名とRaspberry Piの表記を正本へ揃えて source tree が
`815c903fdbe53cd66a7afbdbd1a277d29efc2a37` へ変わったため、`cargo clean` から
正式な command を再実行した記録である。この実行の成果物は sha256 `1e9623de...` である。
現行 tree の成果物は記録本体の `Generated artifact identity` を参照する。
個人 path は `<home>` へ置換した。

```text
########## 表記統一後のフル再検証 ##########
date: 2026-08-08T17:29:16Z
base: 993ef75 board識別の訂正をdocs/hardware以外へ波及させる (#58)
source tree(staged): 815c903fdbe53cd66a7afbdbd1a277d29efc2a37
IDF_PATH=<unset>
esp (overridden by '<home>/deskcat/firmware/esp32/rust-toolchain.toml')
     Removed 3648 files, 1.7GiB total
clean rc=0
fmt rc=0
clippy rc=0
build rc=0
size_bytes=13654676
sha256=1e9623de74b79b33c85510c41c755093f70a4bf85969f40b90fa47ce7ae69d78
Cargo.lock unchanged
```

## compiler版の固定（#74 追記）

初回検証の時点では `rust-toolchain.toml` が固定していたのは custom channel 名 `esp` だけで、
その名前へどの Xtensa Rust 版が入るかは `espup` が決めていた。
[#74](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/74) でこれを版付きの名前へ変更した。

```toml
[toolchain]
channel = "esp-1.95.0.0"
```

導入は `espup install --toolchain-version 1.95.0.0 --targets esp32 --name esp-1.95.0.0` である。
`--name` の既定は `esp` であり、付け忘れた端末では名前が食い違うため build が compile 前に止まる。

**compiler の実体は変わっていない。** 本記録の `Rust compiler version` が挙げる
`rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)` は変更の前後で同一である。
変わったのは toolchain の名前と、その名前が実行時に強制されるようになったことである。

**これは名前の一致の強制であり、その名前の中身の版検査ではない。**
採否の理由は [ESP32 Rust Toolchain](../esp32-rust-toolchain.md#compiler-版の固定) に記載する。

`espup install --name` は `~/export-esp.sh` を再生成し、`PATH` と `LIBCLANG_PATH` を
新しい toolchain 名の path へ書き換えた（下の log の「導入」節）。既存の `esp` は削除していない。

## 版付きtoolchainの導入log（#74 追記）

`espup install --name` が `~/export-esp.sh` を正しく再生成するかを、導入の前後で比較した記録である。
個人 path は `<home>` へ置換した。

```text
########## 版付きtoolchainの導入 ##########
date: 2026-08-10T02:27:34Z
--- 導入前 ---
stable-x86_64-unknown-linux-gnu (active, default)
esp
export-esp.sh(前):
export PATH="<home>/.rustup/toolchains/esp/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin:$PATH"
export LIBCLANG_PATH="<home>/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-20.1.1_20250829/esp-clang/lib"
--- espup install --toolchain-version 1.95.0.0 --targets esp32 --name esp-1.95.0.0 ---
[info]: Installing the Espressif Rust ecosystem
[info]: Checking Rust installation
[info]: Installing Xtensa Rust 1.95.0.0 toolchain
[info]: Installing GCC (xtensa-esp-elf)
[info]: Installing Xtensa LLVM
[info]: Creating symlink between '<home>/.rustup/toolchains/esp-1.95.0.0/xtensa-esp32-elf-clang/esp-20.1.1_20250829/esp-clang/lib' and '<home>/.espup/esp-clang'
[info]: Installing 'rust' component for Xtensa Rust toolchain
[info]: All downloads complete
[info]: Installing 'rust-src' component for Xtensa Rust toolchain
[info]: Installation successfully completed!
real	1m22.496s
ESPUP_RC=0
--- 導入後 ---
stable-x86_64-unknown-linux-gnu (active, default)
esp
esp-1.95.0.0
export-esp.sh(後):
export LIBCLANG_PATH="<home>/.rustup/toolchains/esp-1.95.0.0/xtensa-esp32-elf-clang/esp-20.1.1_20250829/esp-clang/lib"
export PATH="<home>/.rustup/toolchains/esp-1.95.0.0/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin:$PATH"
--- 版の確認 ---
rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
```

`export-esp.sh` の 2 行はいずれも `toolchains/esp-1.95.0.0/` を指す path へ再生成された。
`gcc`（`esp-15.2.0_20250920`）と `clang`（`esp-20.1.1_20250829`）の版は導入前と同一である。
実行時間 1分22秒は、既存の download 成果物が再利用されたことによる。

`espup --version` は 0.17.1 である。この版に設定 file は無く、版の指定は
`--toolchain-version` という起動 option だけである（`espup install --help`、2026-08-10 確認）。

## compiler版の固定後のフル再検証log（#74 追記）

`rust-toolchain.toml` の `channel` を版付きへ変えて source tree が
`cf3fcdb4cd4057cbd70e333676cc547632c14bd6` へ変わったため、`cargo clean` から
正式な command を再実行した記録である。記録本体の `Generated artifact identity` が
挙げる sha256 `0f9d4918...` はこの実行の結果である。
同じ実行の後半で、記録と異なる toolchain を要求した場合に compile 前へ停止することも確認している。
個人 path は `<home>` へ置換した。

```text
########## #74 版付きchannelでのフル再検証 ##########
date: 2026-08-10T02:47:29Z
base: 831f4b1 未解決review threadを残したmergeを防ぐ確認手順を定める (#76)
source tree(staged): cf3fcdb4cd4057cbd70e333676cc547632c14bd6
IDF_PATH=<unset>
esp-1.95.0.0 (overridden by '<home>/deskcat/firmware/esp32/rust-toolchain.toml')
     Removed 3648 files, 1.7GiB total
clean rc=0
fmt rc=0
clippy rc=0
build rc=0
size_bytes=13660584
sha256=0f9d4918526c3e4153d101943ee20c5d9a4ac3d0f08fce6f60569b503e2877f7
Cargo.lock unchanged

--- 強制の実証: 記録と異なるtoolchainを要求した場合 ---
channel = "esp-9.9.9.9"
error: custom toolchain 'esp-9.9.9.9' specified in override file '<home>/deskcat/firmware/esp32/rust-toolchain.toml' is not installed
build rc=1
--- 復元 ---
channel = "esp-1.95.0.0"
file type: ELF 32-bit LSB executable, Tensilica Xtensa, version 1 (SYSV), statically linked, with debug_info, not stripped
```

`build rc=1` の行が受け入れ条件の中核である。`cargo build` は crate を1つも compile せず、
rustup が toolchain の解決段階で停止している。runbook どおりに
`--name esp-1.95.0.0` を付けずに `espup install` した端末では、これと同じ停止が起きる。

**この `esp-9.9.9.9` の case が示すのは、`rust-toolchain.toml` が要求する toolchain 名が
その端末に未導入である、という状態の検出だけである。** 同じ名前へ異なる compiler 版が
導入されている状態は再現しておらず、**この log は「compiler 版が違えば失敗する」ことの
証拠にはならない。** 名前が一致していれば中身の版は検査されないため、そもそも失敗しない。
この区別は [ESP32 Rust Toolchain](../esp32-rust-toolchain.md#compiler-版の固定) の
「残る穴」に対応する。

## `IDF_PATH` guard の検証log（#102 追記）

`firmware/esp32/build.rs`の`IDF_PATH` guard（[PR #101](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/101)）が
実際に動くかを確認した記録である。**このguardは導入時点で一度も実行されていなかった。**
Windows側の作業環境にRust toolchainが無く、CI runnerは`IDF_PATH present: no`で
発火しない経路だけを通るためである。

- 実行日時: 2026-08-11T11:08:57Z 〜 11:13:22Z
- Repository commit: `e20c2a2`、source tree `fb4425a8`
- Container / VM / native: **VM**（`systemd-detect-virt` = `microsoft`）
- 対象Issue: [#102](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/102)

**この検証はVM上で行った。**[Machine Profiles](../machine-profiles.md)は`ESP32 の build-only 検証`に
VMの使用を認めているが、**flashと実機試験の根拠にはならない。**実機Linux（bare metal）での
再現は実施していない。

### 結論

| # | 論点 | 結果 |
|---|---|---|
| 1 | guardが通常のbuildを壊すか | **壊さない。**`IDF_PATH`未設定のbuildは`exit=0` |
| 2 | `~/export-esp.sh`が`IDF_PATH`を設定するか | **設定しない**（実測） |
| 3 | override判定（`1`完全一致） | **設計どおり。**`0`／`false`は通さない |
| 4 | guardは実buildで発火するか | **発火する。**ただし`IDF_PATH`が**実在するESP-IDF**を指す場合に限る |
| 5 | 空文字を未設定として扱う配慮 | **実buildでは到達しない** |

### 依存のbuild scriptが先に走る

`esp-idf-sys`は`deskcat-esp32`の依存であり、**その build script が先に実行される。**
`esp-idf-sys`は`IDF_PATH`を検出すると、`.cargo/config.toml`がpinした`ESP_IDF_VERSION`を破棄する。

```text
warning: esp-idf-sys@0.37.2: Ignoring configuration setting `ESP_IDF_VERSION` (Tag v5.5.3): custom esp-idf repository detected via $IDF_PATH
```

したがって`IDF_PATH`の指す先で挙動が二分される。

| `IDF_PATH`の指す先 | 何が起きるか | guardは |
|---|---|---|
| 実在しないpath（`/tmp/fake-idf`、空文字） | `esp-idf-sys`が版を判定できず停止 | **実行されない** |
| 実在するESP-IDF | `esp-idf-sys`は成功し、`deskcat-esp32`のbuild scriptへ到達 | **発火する** |

**危険なのは後者である。**`esp-idf-sys`は記録に無いESP-IDFでもwarning 1行を出すだけで
buildを続ける。guardはこの経路を止める。前者はguardが無くてもbuildが止まるため、
差は診断messageの分かりやすさだけである。

**この順序はCargoが決めるため`build.rs`側では解決できない。**

### 実buildでの結果

`IDF_PATH=/tmp/fake-idf`（実在しない）を使った5 caseは、いずれも`exit=101`で停止したが
**停止させたのは`esp-idf-sys`であり、guardのpanic messageは全出力に存在しない。**
`IDF_PATH=`（空文字）も同様で、`esp-idf-sys`が`idf_path: Some("")`と解釈して失敗した。
**これらのcaseはguardを検証していない。**

guardの発火は、workspaceへ展開済みの実在するESP-IDFを指定して確認した。

| # | 条件 | 結果 |
|---|---|---|
| D1 | `IDF_PATH=<workspace>/.embuild/espressif/esp-idf/v5.5.3`、override無し | `exit=101`、`build.rs:26`のpanic。**guardが発火した** |
| D2 | 同上＋`DESKCAT_ALLOW_EXTERNAL_IDF_PATH=1` | `exit=0`、`cargo:warning=IDF_PATH=... overrides the pinned ESP-IDF.` を出力 |

```text
thread 'main' panicked at build.rs:26:9:
IDF_PATH is set to <home>/deskcat/firmware/esp32/.embuild/espressif/esp-idf/v5.5.3. It overrides the ESP_IDF_VERSION pinned in .cargo/config.toml, so this build would not match the recorded toolchain. Unset IDF_PATH, or set DESKCAT_ALLOW_EXTERNAL_IDF_PATH=1 (exactly "1") and record the override in a Version Record.
```

**D1／D2で指定したESP-IDFはpin済みと同じv5.5.3である。**別版のESP-IDFを指定した検証は
実施していない。**「記録と異なる版で build される事態を防げる」ことの証明にはなっていない。**

### build script 単体での分岐確認

実buildでguardへ到達しないcaseがあるため、生成済みのbuild scriptを直接起動して
6分岐すべてを確認した。

| 条件 | exit | 出力 |
|---|---|---|
| 未設定 | `0` | `cargo:rerun-if-env-changed=` 2行のみ |
| `/tmp/fake-idf`・override無し | `101` | `panicked at build.rs:26:9` ＋ guard message |
| `=1` | `0` | `cargo:warning=IDF_PATH=... overrides the pinned ESP-IDF.` |
| `=0` | `101` | `panicked at build.rs:26:9` ＋ guard message |
| `=false` | `101` | `panicked at build.rs:26:9` ＋ guard message |
| 空文字 | `0` | `cargo:rerun-if-env-changed=` 2行のみ（未設定と同じ。warningもpanicも無い） |

**6分岐すべてが`build.rs`の設計どおりである。**`cargo:rerun-if-env-changed`の2行は全caseで出ている。

### この検証で確認できていないこと

- **実機Linux（bare metal）での再現。**本記録はVM上である
- **別版のESP-IDFに対するguardの有効性。**用意していない
- `cargo clean`からのfull clean build（cache利用のincremental buildのみ）
- `cargo fmt`／`cargo clippy`（#102の対象外。CIが判定済み）
