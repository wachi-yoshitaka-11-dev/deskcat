# Version Record: #15 ADXL345 sample firmware（実機 Linux x86_64、build only）

> 判定: `Partial`。host の test と ESP32 の clippy・build を記録する。**実機への書き込みと通電はこの記録の対象外で、`docs/hardware/experiment-log.md` の `EXP-033` が持つ**（flash と実機検証は `ESP32 Flash / HIL` の範囲で、この記録は profile を分けている）。

様式は [Version Record Template](../version-record-template.md) に従う。

```text
Record ID: VR-2026-10-17-ACCEL-15-NATIVE-LINUX
Date: 2026-10-17（初回。develop へ載せ替えた後の再実行は 2026-10-18 と 2026-10-20。最終有効な検証日時は 2026-10-20）
Machine profile: ESP32 Build（host の crate の検証を含む。flash・実機検証は含めない）
Operator role: 開発者（human）の監督下での AI agent 作業（tool の新規導入なし）
Repository commit: develop base は 0b5ca5a（#620 の merge 後）。載せ替え後の検査は、0b5ca5a の上の、この Version Record を含む1つの commit（#15 の変更の全部。commit 識別子は amend のたびに変わるため書かない）の内容で行った。最初の build と実機への書き込みは 045aab4552e88dc942373b63b08758a5e8efc2ca の上の未 commit 差分
Working tree clean: 最初の build と実機への書き込みは no（#15 の未 commit 差分）。載せ替え後の build・test・clippy は yes（変更を commit した tree。`git status --short` が空であることを確かめた。build 時だけ firmware/esp32/Cargo.toml へ足す [workspace] は、終えて戻した）。その後の amend は docs の変更だけで、build・test は再実行していない（`validate_doc_links.py`・`validate_table_columns.py` は再実行した）

OS name / version: Ubuntu 24.04.4 LTS
CPU architecture / userspace: x86_64 / 64 bit
Container / VM / native: native（systemd-detect-virt: none）

Rust channel / compiler: host stable 1.97.1（8bab26f4f 2026-07-14）、
  ESP esp-1.95.0.0 / rustc 1.95.0-nightly（95e5bda86 2026-04-15）
Rust host / target: x86_64-unknown-linux-gnu / xtensa-esp32-espidf
Cargo: host 1.97.1、ESP 1.95.0-nightly
rustfmt / Clippy: host 1.9.0-stable / 0.1.97、ESP 1.9.0-nightly / 0.1.95
Linker: ldproxy 使用、版はこの記録では独立確認していない

ESP32 only:
  Physical board / Module marking / Board revision: `MCU-01`（ESP32-DevKitC V4）。
    module の刻印と基板の revision は、この記録では確認していない
  espflash: この記録の対象外（書き込みは EXP-033 が持つ）
  espup / cargo-generate: この作業では版を確認していない
  ESP-IDF version: v5.5.3（firmware/esp32/.cargo/config.toml の ESP_IDF_VERSION）
  ESP-IDF source commit: この作業では確認していない
  ESP-IDF tools location mode: workspace（firmware/esp32/.embuild 配下）
  sdkconfig.defaults sha256: 4907245b9e964d0c7643f48c3e04fbe40c468e0d31ad7ee7fe3c0ee1ad9efc21
  USB-UART identity: 記録しない（書き込みは1回。port の識別子は個人環境の値）

Commands run / Actual result:
  Host: cargo test -p deskcat-accel: 成功（9件）
  Host: cargo fmt、cargo clippy（deskcat-accel）: 成功
  Firmware: cargo clippy --locked -- -D warnings を、--features accel-sample-15、
    既定、--features tap-21,face-21,bringup-display-13 の3構成で実行: 成功
  Firmware: cargo build --locked --features accel-sample-15: 成功
  scripts/validate_doc_links.py: BROKEN=0
  firmware の unit test は実行していない。
  上の結果は、前の会話での実行を #15 の comment 6233673175 に残した記録による。
  **0b5ca5a の上へ載せ替えた後（2026-10-20）に再実行した：**host の cargo test -p deskcat-accel（9件）、cargo fmt --all --check、cargo clippy -p deskcat-accel --all-targets -D warnings、firmware の cargo clippy --offline --all-targets --locked -D warnings（accel-sample-15、既定、tap-21,face-21,bringup-display-13）と cargo build --offline --locked --features accel-sample-15（いずれも成功。build 時だけ [workspace] を足して戻した）。
  **CodeRabbit の指摘（mg の換算）を直した後（2026-10-20）：**host の cargo test -p deskcat-accel（10件）、fmt、clippy（-D warnings）、firmware の clippy 3 構成と cargo build --features accel-sample-15 を再実行し、いずれも成功した（build 時だけ [workspace] を足して戻した）。
  **自己レビュー1巡目の指摘を直した後の差分で、次を再実行した（同じ端末・同じ tree）：**
  Host: cargo fmt --all -- --check、cargo clippy --locked -p deskcat-accel --all-targets、
    cargo test --locked -p deskcat-accel（9件）: 成功。validate_doc_links.py BROKEN=0、validate_table_columns.py MISMATCHES=0。
  Host（workspace 全体）: cargo clippy --workspace --all-targets --locked: 成功。cargo test --workspace --locked: 成功（468件、失敗0。50eb97f の上で実行。0b5ca5a の上では再実行していない）。
  Firmware: cargo fmt --all -- --check、cargo clippy --offline --all-targets --locked -- -D warnings を
    --features accel-sample-15、既定、--features tap-21,face-21,bringup-display-13 の3構成で実行: 成功。
    cargo build --offline --locked --features accel-sample-15: 成功。
    build 時だけ firmware/esp32/Cargo.toml の末尾へ [workspace] を足し、終えて元の byte 列へ戻した（cmp で一致を確認）。

Generated artifact identity:
  accel-sample-15 build の deskcat-esp32 ELF sha256
  d222579b09bddeaa7413162c1f19ea293e94c40d6cb48d4bb4d3807f350e99aa（overrun の確認を FIFO 読み出しの前へ移し、連続失敗の数え方を直し、mean_mg_typ の換算を mLSB から 1 回の丸めにした後の版。PR #622 の CodeRabbit の指摘への対応後の build。直前の fa391f90c54e33c1be6afc84eef744fc4b1dcaa6251342d4b80cb15ddae0e4cc は、mg の換算を直す前の版）
  git 管理外の target 配下。**実機へ書き込んでいない。**
  実機へ書き込んだのは、移す前の版（sha256 e287da22f5a7bf15e7a4aa7e4564fdf78fab098b9670729ca8e046eb9d87b0ba）で、
  その観察は EXP-033 にある（この記録の対象外）。

Known differences from documented profile:
  worktree の下では firmware の cargo が上位の workspace を拾うため、build 時だけ
  firmware/esp32/Cargo.toml の末尾へ [workspace] を足し、終えたら戻した。
  この確認はこの端末・この SDK・この tree の build / lint に限る。

Conclusion: Partial。host の test と ESP32 の clippy・build は、直した後の差分でも通過した。
  未達：CI での再現、firmware の unit test、offset、個体の感度、
  振動 sample、Rev. G との register 値の照合。
Next action: 実機の追加測定（直した版の書き込みを含む）は、別の通電の承認の下で行う。
```
