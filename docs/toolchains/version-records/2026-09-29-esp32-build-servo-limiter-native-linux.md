# Version Record: ESP32 Build（`deskcat-servo`の共有、実機Linux x86_64）

様式は [Version Record Template](../version-record-template.md) に従う。

- Record ID: `2026-09-29-esp32-build-servo-limiter-native-linux`
- 判定: `Partial`
- 初回検証日: 2026-09-29
- 最終有効な検証日時: 2026-10-03T10:08 JST（2026-10-03T09:59 JSTに開始した下の実行）

**この記録は3つのことを示す。**(0) host workspaceの検証済みcommand（stable）が同じtreeで通ること、(1) ESP toolchainのrustc（`esp-1.95.0.0`）で`crates/deskcat-servo`と
`crates/deskcat-protocol`のtestとlintが通ること（`deskcat-servo`の`rust-version = "1.95"`の根拠）、
(2) `firmware/esp32`が`deskcat-servo`をpath dependencyで使う状態で、既定構成と`bringup-display-13`の
format、lint、buildが通ること（[Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)、
[ADR-0008](../../decisions/0008-firmware-protocol-crate-reuse.md)の追記）。
**検証済みcommandへ格上げする記録ではなく、この1回の実施記録である**（`bringup-display-13`の扱いは
[検証済みコマンド](../verified-commands.md)が持つ。ここで変えない）。

**2026-10-03の再検証で、構成が変わった。**`pi-protocol-mode`は[#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)
（PR B1）で廃止され、Pi linkは既定構成に入った。そのため(2)は既定と`bringup-display-13`の2構成である。
2026-09-30までの実行は、`pi-protocol-mode`を含む3構成だった。あわせて、Issue #19の段2で
`DuplicateHistory`を`crates/deskcat-serial`から`crates/deskcat-protocol`へ移し、`crates/deskcat-servo`に
`MotionDedup`を足したtreeで、(1)をやり直した。

## 記録

```text
Record ID: 2026-09-29-esp32-build-servo-limiter-native-linux
Date: 2026-09-29（初回）。下の値は最終有効な検証（2026-10-03T09:59〜10:08 JST）のもの
Machine profile: ESP32 Build
Operator role: AI agent作業（build-onlyのみ。flash・monitor・USBシリアルポートは開いていない）
Repository commit: 0bb0d6cfc962d774e5c609885685680b96a43c24（#19の段2の作業branchのcommit。
  親は origin/develop 43189b83f66042b22c3804dccf4e20fb3a1258fb）。
  **このcommitより後に同branchへ入れるcommitは、文書（この記録、Version Recordsの一覧、
  `docs/toolchains/verified-commands.md`）だけを変える。code、manifest、lockfileは変えない。**squash mergeの後はこのSHAがdevelopから辿れなくなるため、
  そのときはPull Requestのcommit一覧から辿る
Working tree clean: yes（`git archive 0bb0d6c`でscratchpadへ展開した未改変コピーを使用。
  worktree・main checkoutいずれのgit stateも変更していない）

OS name: Ubuntu
OS version: 24.04.4 LTS
Kernel: 7.0.0-34-generic（2026-09-30までの実行は7.0.0-28-generic）
CPU architecture: x86_64
Userspace bitness: 64-bit
Container / VM / native: native（実機）。systemd-detect-virt: none

Rustup version: rustup 1.29.0 (28d1352db 2026-03-05)
Rust channel: esp-1.95.0.0（firmware/esp32/rust-toolchain.tomlが固定。
  (1)のcrate検証では`cargo +esp-1.95.0.0`で明示した）
Rust compiler version: rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
Rust host: x86_64-unknown-linux-gnu
Installed Rust targets: xtensa-esp32-espidf（esp toolchain同梱）
Cargo version: cargo 1.95.0-nightly (f2d3ce0bd 2026-03-21) (1.95.0.0)
rustfmt version: esp toolchain同梱版（rustfmt 1.9.0-nightly (95e5bda868 2026-04-15)）
Clippy version: esp toolchain同梱版（clippy 0.1.95 (95e5bda868 2026-04-15)）
Host stable toolchain（(0)で使用）: rustc 1.97.1 (8bab26f4f 2026-07-14)、cargo 1.97.1 (c980f4866 2026-06-30)、
  clippy 0.1.97 (8bab26f4f6 2026-07-14)、rustfmt 1.9.0-stable (8bab26f4f6 2026-07-14)
Linker identity and version: cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0（host側）。
  Xtensa側はesp toolchain同梱のxtensa-esp-elf（esp-15.2.0_20250920）

ESP32 only:
  Physical board: 未接続（build-only。この記録ではboardを一切使っていない）
  Module marking: この検証では未確認（board未接続のため）
  Board revision: 同上
  Rust target: xtensa-esp32-espidf
  espup version: 0.17.1
  cargo-generate version: 0.23.14
  ldproxy version: 0.3.5
  espflash version: 導入済み（4.5.0）だが本記録では未使用（flashを行っていない）
  ESP-IDF version: v5.5.3
  ESP-IDF source/commit: 2c211b236707889e8400c4dc5644dd5c4ee071e0
  ESP-IDF tools location mode: workspace外の既存.embuildをsymlinkで共有
    （main checkoutのfirmware/esp32/.embuildを、scratchpad上の展開先
    `firmware/esp32/.embuild`へsymlinkし、再ダウンロード・再構築を避けた）
  IDF_PATH present: no
  IDF_TOOLS_PATH present: no
  Template repository: https://github.com/esp-rs/esp-idf-template
  Template commit: 未確認（本記録では確認していない）
  sdkconfig/defaults identity: 追跡中のfirmware/esp32/sdkconfig.defaultsを使用。本作業で変更していない
  USB-UART identity: 未確認（board未接続）

Commands run:
  (0) 展開先のrootで（host workspace、上のHost stable toolchain。CARGO_TARGET_DIRはscratchpad）
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --locked
  cargo test --workspace --locked
  (1) 展開先のrootで（host workspace。CARGO_TARGET_DIRはscratchpad）
  cargo +esp-1.95.0.0 test --locked -p deskcat-protocol -p deskcat-servo
  cargo +esp-1.95.0.0 clippy --locked -p deskcat-protocol -p deskcat-servo --all-targets
  (2) 展開先の`firmware/esp32`で
  . "$HOME/export-esp.sh"
  cargo fmt --all -- --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo build --locked
  cargo clippy --all-targets --locked --features bringup-display-13 -- -D warnings
  cargo build --locked --features bringup-display-13

Expected result: すべて成功する。warningを出さない。(1)は`--ignore-rust-version`を付けずに通る
  （`deskcat-servo`が`rust-version = "1.95"`を宣言しているため）。

Actual result:
  (0) cargo fmt／clippy／test（host workspace）                        成功。warning 0件。269 tests、失敗 0
      （test target 263件: deskcat-config 0＋4＋4＋2、deskcat-domain 0＋13、deskcat-protocol 58＋11＋3＋5＋12、
      deskcat-serial 45＋65、deskcat-servo 0＋9＋32。doctest 6件: deskcat-protocol 3、deskcat-serial 2、deskcat-servo 1）
  (1) cargo +esp-1.95.0.0 test（2 crate）                            成功。deskcat-protocol 89 tests
      （unit 58、conformance 11、error_codes 3、framing 5、limits 12）、deskcat-servo 41 tests
      （dedup 9、trajectory 32、unit 0）、doctest 4（deskcat-protocol 3、deskcat-servo 1）。失敗 0
  (1) cargo +esp-1.95.0.0 clippy（2 crate、--all-targets）            成功。warning 0件
  (2) cargo fmt --all -- --check                                     成功。差分なし
  (2) cargo clippy --all-targets --locked -- -D warnings（既定構成）  成功。warning 0件
  (2) cargo build --locked（既定構成）                                成功
  (2) cargo clippy ... --features bringup-display-13                  成功。warning 0件
  (2) cargo build --locked --features bringup-display-13              成功

Build duration:
  (0) fmt 1秒未満、clippy 約15秒、test 約19秒
  (1) test 約26秒、clippy 約11秒
  cargo clippy（既定構成、ESP-IDF本体のcompileを含む）  約6分16秒
  cargo build（既定構成、clippy後）                     約52秒
  cargo clippy／build（+bringup-display-13、増分）      約3秒／約10秒
Peak memory if measured: 未測定
Storage delta if measured:
  build前後の`df -h /` Used: 44G → 46G（scratchpad上のtarget。既存.embuildをsymlinkしたため.embuild分は増加なし）
Generated artifact identity（いずれも target/xtensa-esp32-espidf/debug/deskcat-esp32。同じpathを構成ごとに上書きしたため、各build直後に取得した）:
  既定構成            17,316,424 bytes  sha256 2cadca05da7d18b9e7d9e724891dfbaaf7a05b7c9facedf98881617162c20a19
  bringup-display-13  17,962,088 bytes  sha256 6c2a826010d9328d9acd4254deac291202e30bdc21fc393b7e199a7286aaa5e1
Log or evidence path: この記録本文

Known differences from documented profile:
  - **worktreeからは実行できない。**既存記録
    （[2026-08-15-esp32-build-native-linux.md](2026-08-15-esp32-build-native-linux.md)）と同じ理由
    （root Cargo.tomlのexcludeがworktree pathと不一致になりcargoがworkspaceを誤認する）。
    [2026-09-26の記録](2026-09-26-esp32-build-bringup-display-13-native-linux.md)と同じく、
    `git archive`でscratchpad（repo外）へ展開した独立コピーを使い、そこへ既存`.embuild`をsymlinkして実行した。
  - (1)は`--workspace`ではなく`-p`で2 crateに絞った。`deskcat-config`と`deskcat-serial`はworkspaceの
    `rust-version`（1.97）を継承しており、1.95のrustcではcompile前に止まるためである。
    **root workspaceのcrateのうち、firmwareが使うのはこの2つだけである**（`firmware/esp32/Cargo.toml`の`[dependencies]`）。
  - (1)のtestはhost target（`x86_64-unknown-linux-gnu`）で実行した。**ESP toolchainのrustcで
    compileできることの確認であり、xtensaの上でtestを走らせたのではない。**
  - `bench-servo-test-17`構成は本記録の対象外（#474の`compile_error!`で意図的に止まる。
    扱いは[servo-safety-limits.md](../../hardware/servo-safety-limits.md)の`承認の状態`が持つ）。
  - 既定buildの`main()`はservoを駆動しない。**`deskcat-servo`はfirmwareのbuildでcrateとして
    compileされるが、`main()`から呼ぶ経路が無いため、link後の実行物に入っているかは確かめていない。
    実機では一度も実行していない。**

Conclusion: Partial。**build-onlyの範囲（(0)のhost workspace、(1)の2 crateのtestとlint、(2)の2構成のfmt/clippy/build）では
  未実行の項目が無く、すべて成功した。**
  **`Partial`とするのは、flashと実機起動を行っていないためである。**board未接続のため
  `Physical board`／`Module marking`／`Board revision`／`USB-UART identity`は
  この検証では確認していない。ESP32へのUSB接続は一切行っていない。

Next action: limiterを通る実動作の監視下試験は#19の受け入れ条件7であり、#18の実測値と
  [servo-safety-limits.md](../../hardware/servo-safety-limits.md)の`承認の状態`の項目6の条件(a)〜(c)がそろってから行う。
  ESP toolchainの版を上げるときは(1)をやり直す（[ADR-0008](../../decisions/0008-firmware-protocol-crate-reuse.md)）。
```
