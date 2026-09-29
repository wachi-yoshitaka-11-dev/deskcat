# Version Record: ESP32 Build（`deskcat-servo`の共有、3構成、実機Linux x86_64）

様式は [Version Record Template](../version-record-template.md) に従う。

- Record ID: `2026-09-29-esp32-build-servo-limiter-native-linux`
- 判定: `Partial`
- 初回検証日: 2026-09-29
- 最終有効な検証日時: 2026-09-30T00:39 JST（2026-09-30T00:30 JSTに開始した下の実行）

**この記録は3つのことを示す。**(0) host workspaceの検証済みcommand（stable）が同じtreeで通ること、(1) ESP toolchainのrustc（`esp-1.95.0.0`）で`crates/deskcat-servo`と
`crates/deskcat-protocol`のtestとlintが通ること（`deskcat-servo`の`rust-version = "1.95"`の根拠）、
(2) `firmware/esp32`が`deskcat-servo`をpath dependencyで使う状態で、既定構成・`pi-protocol-mode`・
`bringup-display-13`の3構成のformat、lint、buildが通ること（[Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)、
[ADR-0008](../../decisions/0008-firmware-protocol-crate-reuse.md)の追記）。
**検証済みcommandへ格上げする記録ではなく、この1回の実施記録である**（`pi-protocol-mode`と
`bringup-display-13`の扱いは[検証済みコマンド](../verified-commands.md)が持つ。ここで変えない）。

## 記録

```text
Record ID: 2026-09-29-esp32-build-servo-limiter-native-linux
Date: 2026-09-29（初回）。下の値は最終有効な検証（2026-09-30T00:30〜00:39 JST）のもの
Machine profile: ESP32 Build
Operator role: AI agent作業（build-onlyのみ。flash・monitor・USBシリアルポートは開いていない）
Repository commit: ed048289bf61b77d01aa13efd0ec02694ce505b4（#19の作業branchのcommit。
  origin/develop 52a8142c793a54aa4627bca5c3a32f8debf951d8 を取り込み済み）。
  **このcommitより後に同branchへ入れるcommitは、この記録とVersion Recordsの一覧だけを変える。**
  code、manifest、lockfileは変えない。squash mergeの後はこのSHAがdevelopから辿れなくなるため、
  そのときはPull Requestのcommit一覧から辿る
Working tree clean: yes（`git archive ed04828`でscratchpadへ展開した未改変コピーを使用。
  worktree・main checkoutいずれのgit stateも変更していない）

OS name: Ubuntu
OS version: 24.04.4 LTS
Kernel: 7.0.0-28-generic
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
  cargo +esp-1.95.0.0 test --locked -p deskcat-servo -p deskcat-protocol
  cargo +esp-1.95.0.0 clippy --locked -p deskcat-servo -p deskcat-protocol --all-targets
  (2) 展開先の`firmware/esp32`で
  . "$HOME/export-esp.sh"
  cargo fmt --all -- --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo build --locked
  cargo clippy --all-targets --locked --features pi-protocol-mode -- -D warnings
  cargo build --locked --features pi-protocol-mode
  cargo clippy --all-targets --locked --features bringup-display-13 -- -D warnings
  cargo build --locked --features bringup-display-13

Expected result: すべて成功する。warningを出さない。(1)は`--ignore-rust-version`を付けずに通る
  （`deskcat-servo`が`rust-version = "1.95"`を宣言しているため）。

Actual result:
  (0) cargo fmt／clippy／test（host workspace）                        成功。warning 0件。243 tests、失敗 0
      （test target 237件: deskcat-config 0＋4＋4＋2、deskcat-protocol 52＋11＋3＋5＋9、
      deskcat-serial 50＋65、deskcat-servo 0＋32。doctest 6件: deskcat-protocol 3、deskcat-serial 2、deskcat-servo 1）
  (1) cargo +esp-1.95.0.0 test（2 crate）                            成功。deskcat-protocol 80 tests
      （unit 52、conformance 11、error_codes 3、framing 5、limits 9）、deskcat-servo 32 tests
      （trajectory 32、unit 0）、doctest 4（deskcat-protocol 3、deskcat-servo 1）。失敗 0
  (1) cargo +esp-1.95.0.0 clippy（2 crate、--all-targets）            成功。warning 0件
  (2) cargo fmt --all -- --check                                     成功。差分なし
  (2) cargo clippy --all-targets --locked -- -D warnings（既定構成）  成功。warning 0件
  (2) cargo build --locked（既定構成）                                成功
  (2) cargo clippy ... --features pi-protocol-mode                    成功。warning 0件
  (2) cargo build --locked --features pi-protocol-mode                成功
  (2) cargo clippy ... --features bringup-display-13                  成功。warning 0件
  (2) cargo build --locked --features bringup-display-13              成功

Build duration:
  (0) fmt 約1秒、clippy 約14秒、test 約16秒
  (1) test 約25秒、clippy 約10秒
  cargo clippy（既定構成、ESP-IDF本体のcompileを含む）  約6分24秒
  cargo build（既定構成、clippy後）                     約50秒
  cargo clippy／build（+pi-protocol-mode、増分）        約3秒／約7秒
  cargo clippy／build（+bringup-display-13、増分）      約2秒／約8秒
Peak memory if measured: 未測定
Storage delta if measured:
  build前後の`df -h /` Used: 53G → 55G（scratchpad上のtarget。既存.embuildをsymlinkしたため.embuild分は増加なし）
Generated artifact identity（いずれも target/xtensa-esp32-espidf/debug/deskcat-esp32。同じpathを構成ごとに上書きしたため、各build直後に取得した）:
  既定構成            14,821,608 bytes  sha256 cb4db40ed6b022f5bd042a4f338c79440ff799dbd23e8ba77f3ff5e2d008c265
  pi-protocol-mode    16,774,860 bytes  sha256 9fac38b7d7cdf4d2c955da69e15c529c1b5cdb122316a9b4abe4c46adc516f5b
  bringup-display-13  15,477,080 bytes  sha256 f0c0b2abb8b7f0e60b6d25a7236ea9c92edea22d72d384659c2ffc64f2409dec
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
    `pi-protocol-mode`との排他構成（`compile_error!`）も対象外である。
  - 既定buildの`main()`はservoを駆動しない。**`deskcat-servo`はfirmwareのbuildでcrateとして
    compileされるが、`main()`から呼ぶ経路が無いため、link後の実行物に入っているかは確かめていない。
    実機では一度も実行していない。**

Conclusion: Partial。**build-onlyの範囲（(0)のhost workspace、(1)の2 crateのtestとlint、(2)の3構成のfmt/clippy/build）では
  未実行の項目が無く、すべて成功した。**
  **`Partial`とするのは、flashと実機起動を行っていないためである。**board未接続のため
  `Physical board`／`Module marking`／`Board revision`／`USB-UART identity`は
  この検証では確認していない。ESP32へのUSB接続は一切行っていない。

Next action: limiterを通る実動作の監視下試験は#19の受け入れ条件7であり、#18の実測値と
  [servo-safety-limits.md](../../hardware/servo-safety-limits.md)の`承認の状態`の項目6の条件(a)〜(c)がそろってから行う。
  ESP toolchainの版を上げるときは(1)をやり直す（[ADR-0008](../../decisions/0008-firmware-protocol-crate-reuse.md)）。
```
