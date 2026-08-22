# Version Record: ESP32 flash と初回起動記録（実機 Linux）

[Issue #6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6) の実施記録である。
[Version Record Template](../version-record-template.md) の書式に従う。

**この記録は flash と実機起動を主張する。**build-only の記録
（[2026-08-15](2026-08-15-esp32-build-native-linux.md)）とは別である。

```text
Record ID: 2026-08-20-esp32-flash-boot-native
Date: 2026-08-20 (JST)
Machine profile: ESP32 Flash / HIL
Operator role: firmware
Repository commit: 3aa9fe4ba270be59687d87d31ed23d79a80d2dcd
Working tree clean: no

OS name: Ubuntu
OS version: 24.04.4 LTS
CPU architecture: x86_64
Userspace bitness: 64-bit
Container / VM / native: native（実機）。systemd-detect-virt: none

Rustup version: rustup 1.29.0 (28d1352db 2026-03-05)
Rust channel: esp-1.95.0.0（firmware/esp32/rust-toolchain.toml）
Rust compiler version: rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
Rust host: x86_64-unknown-linux-gnu
Installed Rust targets: x86_64-unknown-linux-gnu（xtensa-esp32-espidf は esp channel が持つ）
Cargo version: cargo 1.95.0-nightly (f2d3ce0bd 2026-03-21) (1.95.0.0)
rustfmt version: rustfmt 1.9.0-nightly (95e5bda868 2026-04-15)
Clippy version: clippy 0.1.95 (95e5bda868 2026-04-15)
Linker identity and version: ldproxy 0.3.5

ESP32 only:
  Physical board: ESP-WROOM-32D 開発ボード（秋月 M-13628）
  Module marking: この記録では未確認（既に #122 で確認済み。HW-TBD-001 は close）
  Board revision: 同上
  Rust target: xtensa-esp32-espidf
  espup version: espup 0.17.1
  cargo-generate version: cargo-generate 0.23.14
  ldproxy version: ldproxy 0.3.5
  espflash version: espflash 4.5.0
  ESP-IDF version: v5.5.3
  ESP-IDF source/commit: 2c211b236707889e8400c4dc5644dd5c4ee071e0
  ESP-IDF tools location mode: workspace
  IDF_PATH present: no
  IDF_TOOLS_PATH present: no
  Template repository: 該当なし（既存の firmware/esp32 を変更した）
  Template commit: 該当なし
  sdkconfig/defaults identity: sha256 4907245b9e964d0c7643f48c…（sdkconfig.defaults。内容は未変更）
  USB-UART identity: Silicon Labs CP2102N（USB id 10c4:ea60、driver cp210x）

Commands run:
  . "$HOME/export-esp.sh"
  cargo fmt --all -- --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo build --locked
  cargo build --locked --release
  cargo install espflash --version 4.5.0 --locked
  ESPTOOL=firmware/esp32/.embuild/espressif/python_env/idf5.5_py3.12_env/bin/esptool.py
  IDFPY=firmware/esp32/.embuild/espressif/python_env/idf5.5_py3.12_env/bin/python
  "$IDFPY" "$ESPTOOL" --port <port> chip_id
  "$IDFPY" "$ESPTOOL" --chip esp32 elf2image --output <out> <elf>
  espflash flash --monitor --port <port> --chip esp32 target/xtensa-esp32-espidf/release/deskcat-esp32

Expected result:
  format・lint・build が通り、flash 後の起動出力に firmware build identity、
  board-configuration ID、reset reason が出る。servo と未知の output を drive しない。
  esptool が報告する chip 名が ESP32-D0WD または ESP32-D0WD-V3 である。

Actual result: 期待どおり。詳細は下記。
Build duration: debug 18.46 s、release 6 m 01 s（いずれも増分 build。clippy は 7.86 s）
Peak memory if measured: 未測定
Storage delta if measured: 未測定
Generated artifact identity:
  debug   ELF 13,748,908 bytes / flash image 557,968 bytes
  release ELF    654,944 bytes / flash image 381,344 bytes
Log or evidence path: 本文書に未加工の出力を転記した。log file は repository へ入れていない
Known differences from documented profile: 下記「文書との差」を参照
Conclusion: Verified（flash と起動記録について）
Next action: 下記「この記録が主張しないこと」に挙げた項目
```

## chip 識別（`HW-TBD-031`）

**満たし方の正本は [ESP32 Rust Toolchain](../esp32-rust-toolchain.md) の `chip 識別の満たし方` である。
ここへ規則を再掲しない。**記録するのは結果と未加工の出力だけである。

`esptool` が報告した行を未加工で残す。

```text
esptool.py v4.12.0
Serial port /dev/ttyUSB0
Connecting.......
Detecting chip type... ESP32
Chip is ESP32-D0WD (revision v1.0)
Features: WiFi, BT, Dual Core, 240MHz, VRef calibration in efuse, Coding Scheme None
WARNING: Detected crystal freq 41.01MHz is quite different to normalized freq 40MHz. Unsupported crystal in use?
Crystal is 40MHz
```

**chip 名は `ESP32-D0WD` であり、期待値に一致する。**
`esptool` の版は **4.12.0**（pin 済み ESP-IDF v5.5.3 の tool set に含まれる。追加導入はしていない）。

**`esptool.py` は PATH では解決できない。**`. "$HOME/export-esp.sh"` を読んでも
`command -v esptool.py` は何も返さない（同 script が設定するのは `LIBCLANG_PATH` と
Xtensa compiler の `PATH` である）。`python3` も system の `/usr/bin/python3` が選ばれる。
**したがって ESP-IDF の Python と `esptool.py` を path で明示して起動する。**
上の `Commands run` はその形で記録した。`ESP_IDF_TOOLS_INSTALL_DIR=workspace` のため、
両者は `firmware/esp32/.embuild/` 配下にある。

**MAC address は記録しない**（template の「記録してはいけないもの」に該当する）。

## `espflash` では満たせないことの実測

`espflash` の出力を未加工で残す。**family 名しか出ない**ことが実測で確認できた。

```text
Chip type:         esp32 (revision v1.0)
Crystal frequency: 40 MHz
Flash size:        4MB
Features:          WiFi, BT, Dual Core, 240MHz, VRef calibration in efuse, Coding Scheme None
App/part. size:    381,344/4,128,768 bytes, 9.24%
```

## 起動出力

`espflash flash --monitor` で採った、firmware が出した行である。

```text
I (354) main_task: Calling app_main()
I (354) deskcat_esp32: firmware=deskcat-esp32 version=0.1.0 profile=release
I (354) deskcat_esp32: board=esp32
I (354) deskcat_esp32: reset_reason=power_on raw=PowerOn
I (364) deskcat_esp32: peripherals=untouched servo=not_driven
I (364) main_task: Returned from app_main()
```

**Wi-Fi も peripheral も初期化されていない。**`Peripherals::take()` を呼ばない実装であり、
起動出力にも peripheral 初期化の行が無い。

## 起動の再現

**4 回とも同じ結果になった。**ROM 側と firmware 側が一致している。

| 回 | ROM の reset 表示 | firmware の出力 |
|---|---|---|
| 1 | `rst:0x1 (POWERON_RESET)` | `reset_reason=power_on raw=PowerOn` |
| 2 | 同 | 同 |
| 3 | 同 | 同 |
| 4 | 同 | 同 |

**USB の抜き差しによる本物の電源再投入も行った**（device node が再作成されたことで確認）。
**ただしその起動出力は採れていない。**host 側の serial port は USB enumerate 後にしか
存在せず、その時点で ESP32 の起動出力は既に終わっているためである。**構造的な限界であり、
再試行では解決しない。**採れた 4 回はいずれも `espflash` が行う reset によるものである。

## 文書との差

1. **`Chip type:` という label は `esptool` の `chip_id` では出ない。**`esptool` は `Chip is`、
   `espflash` が `Chip type:` を使う。正本は「行全体を文字列一致で照合しない。求めるのは chip 名」
   と定めているため**判定は成立する**が、label の想定と実際が違う点を記録する
2. **crystal について警告が出た。**`esptool` は `Detected crystal freq 41.01MHz is quite different
   to normalized freq 40MHz. Unsupported crystal in use?` と出し、`espflash` は
   `Crystal frequency: 40 MHz` と出す。**同じ個体に対して両者の表示が違う。**
   原因は未特定であり、**推測を書かない。**影響の有無もこの記録では判定していない
3. **image に `App version: 3aa9fe4-dirty` が入っている。**未 commit の変更がある状態で
   build したためである（`Working tree clean: no`）。**commit 後に採り直した値ではない**

## この記録が主張しないこと

- **flash が通ったことを「board が正常である」と読み替えない。**確認したのは
  起動出力と chip 名だけである
- **「brownout も reset も起きなかった」を電圧が正常だった根拠にしない。**
  `HW-TBD-028` の (b)(c)(d) は合否判定が `Blocked` である
- **周辺回路を何も確認していない。**LCD、touch、sensor、servo には触れていない
- **`HW-TBD-034` の未解決項目は動いていない**（基準電圧の校正、GND topology）
- 本物の電源再投入の起動出力は採れていない（上記のとおり）。**したがって「電源再投入後の
  起動を再現できる」は、`espflash` の reset による 4 回で示したにとどまる。USB 抜き差しでの再現は未検証である**
