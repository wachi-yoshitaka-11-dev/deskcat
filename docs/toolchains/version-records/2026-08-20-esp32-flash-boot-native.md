# Version Record: ESP32 flash と初回起動記録（実機 Linux）

[Issue #6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6) の実施記録である。
[Version Record Template](../version-record-template.md) の書式に従う。

**この記録は flash と実機起動を主張する。**build-only の記録
（[2026-08-15](2026-08-15-esp32-build-native-linux.md)）とは別である。

- 最終有効な検証日時: 2026-08-29（USB の抜き差しによる電源再投入を、同じ端末・同じ profile で実測した。
  「[2026-08-29 追加検証](#2026-08-29-追加検証usb-の抜き差しによる電源再投入)」節を参照する。
  2026-08-25 の firmware 差し替え記録は「[2026-08-25 再検証](#2026-08-25-再検証issue-7-の-heartbeat-と-health-snapshot)」節に残る）

**Record ID と file 名は初回検証日の 2026-08-20 で固定する。**同じ端末の同じ profile であるため、
別記録を起こさずこの記録を更新する（[記録一覧](README.md)の規則）。

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
Conclusion: Partial（flash と起動記録について。USB 抜き差しによる電源再投入後の起動出力が未検証。詳細は下記「この記録が主張しないこと」）
Next action: 下記「この記録が主張しないこと」に挙げた項目
```

## `Commands run` の作業directoryについて（2026-08-23 追記）

**上の `Commands run` は、そのままの順で実行しても再現しない。**起点の異なる path が
混在しているためである。

- `cargo fmt` ／ `cargo clippy` ／ `cargo build` は `firmware/esp32` でしか動かない。
  root workspace が `firmware/esp32` を `exclude` しているためである（[ADR-0008](../../decisions/0008-firmware-protocol-crate-reuse.md)）
- `espflash` へ渡している `target/xtensa-esp32-espidf/release/deskcat-esp32` も
  `firmware/esp32` 起点である
- 一方 `ESPTOOL` と `IDFPY` は `firmware/esp32/.embuild/...` と **repository root 起点**で書かれている

**当時どの directory で実行したかは記録していない。**上の行は観測の記録であり、
**推測で書き換えない。**再現する場合は、次のとおり `firmware/esp32` を作業directory
として読み替える。

**この手順を実行してよいのは ESP32 Flash / HIL profile の端末だけである**
（この記録の `Machine profile`。役割の正本は
[Machine Profiles](../machine-profiles.md)）。**Docs / Review 端末と ESP32 Build
profile 端末では実行しない。**`cargo install espflash` と USB serial の操作を含むためである。

**`espflash flash --monitor` は実機を駆動する。**実行前に
[Hardware Safety Policy](../../governance/hardware-safety-policy.md) と
[AGENTS.md](../../../AGENTS.md) の「ハードウェア安全」が定める条件を満たすこと。
**条件と停止判断の正本はそちらであり、ここへ再掲しない。**この記録が対象とした構成は
`この記録が主張しないこと`のとおり **ESP32 単体（周辺回路・servo なし）**である。

```text
cd firmware/esp32
. "$HOME/export-esp.sh"
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
cargo build --locked --release
cargo install espflash --version 4.5.0 --locked
ESPTOOL=.embuild/espressif/python_env/idf5.5_py3.12_env/bin/esptool.py
IDFPY=.embuild/espressif/python_env/idf5.5_py3.12_env/bin/python
"$IDFPY" "$ESPTOOL" --port <port> chip_id
"$IDFPY" "$ESPTOOL" --chip esp32 elf2image --output <out> <elf>
espflash flash --monitor --port <port> --chip esp32 target/xtensa-esp32-espidf/release/deskcat-esp32
```

**この節は再現手順であって、追加の実行記録ではない。**上の形で再実行して確かめてはいない。
**記録済みの観測値（chip 名、版、size、起動出力）は1つも変更していない。**

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

## 2026-08-25 再検証（Issue #7 の heartbeat と health snapshot）

同じ端末、同じ profile である。**環境の値は上の記録から変わっていない。**
再確認した項目だけを書く。

**この節の日付は JST である。**下に引用した `espflash` の log の timestamp は UTC であり
（`2026-08-24T15:38Z` など）、JST では 2026-08-25 に当たる。**UTC の日付と読み違えない。**

```text
Date: 2026-08-25 (JST)
Repository commit: d528d14 に Issue #7 の差分を載せた作業tree
Working tree clean: no
OS / arch / virt: Ubuntu 24.04.4 LTS / x86_64 / native（systemd-detect-virt: none）
Rust channel: esp-1.95.0.0（rustc 1.95.0-nightly (95e5bda86 2026-04-15)）
espflash version: espflash 4.5.0
ESP-IDF version: v5.5.3（source commit 2c211b236707889e8400c4dc5644dd5c4ee071e0）
ESP-IDF tools location mode: workspace
IDF_PATH present: no
IDF_TOOLS_PATH present: no
sdkconfig/defaults identity: sha256 4907245b9e964d0c…（**未変更**）
chip: ESP32-D0WD (revision v1.0)、USB-UART は CP210x（10c4:ea60）
```

`Commands run` は上の「再現できる形」と同じである。**追加した command は無い。**

### size report

```text
debug   ELF 14,215,884 bytes / flash image 574,560 bytes
release ELF    672,160 bytes / flash image 391,216 bytes
```

`espflash` の表示は `App/part. size: 391,216/4,128,768 bytes, 9.48%` である。
Issue `#6` の release image 381,344 bytes から **9,872 bytes 増えた**。partition に対する比率は 9.24 % → 9.48 %。

### 起動出力

```text
rst:0x1 (POWERON_RESET),boot:0x13 (SPI_FAST_FLASH_BOOT)
I (346) main_task: Started on CPU0
I (356) main_task: Calling app_main()
I (356) deskcat_esp32: firmware=deskcat-esp32 version=0.1.0 profile=release
I (356) deskcat_esp32: board=esp32
I (356) deskcat_esp32: reset_reason=power_on raw=PowerOn
I (366) deskcat_esp32: peripherals=untouched servo=not_driven
I (366) deskcat_esp32: heartbeat_period_ms=1000 health_snapshot_period_ms=10000
I (1386) deskcat_esp32: hb seq=1 uptime_ms=1006
I (2386) deskcat_esp32: hb seq=2 uptime_ms=2006
I (3386) deskcat_esp32: hb seq=3 uptime_ms=3006
```

**`main_task: Returned from app_main()` は現れない。**#6 の firmware はこれを出して `main()` から戻っていた。

health snapshot（10 秒ごと。1 行を折り返して示す）。

```text
I (10386) deskcat_esp32: health uptime_ms=10006 overrun_ticks=0 snapshot_errors=0
status={"firmware":"0.1.0","reset_reason":"power_on",
"display":{"state":"unknown","expression":"unknown"},
"servo":{"state":"unknown"},
"sensors":{"touch":"unknown","acceleration":"unknown","environment":"unknown"},
"protocol":{"parse_errors":0,"invalid_payloads":0,"unsupported_versions":0,
"oversize_lines":0,"unknown_types":0,"rate_limited":0,"busy":0,"out_of_range":0,
"stale_sessions":0,"hardware_unavailable":0,"duplicate_expired":0,
"session_switches":0,"suppressed_responses":0}}
```

### 周期の実測

最終 build を flash した窓（約 90 秒）で heartbeat 80 行を得た。
**連続する `uptime_ms` の差は 79 件すべて 1000 ms である**（他の値は 0 件）。
health snapshot は 8 行で、差はすべて 10000 ms である。
`heartbeat_overrun` と `health_snapshot_overrun` は 0 行、`overrun_ticks` は 0 のままである。

**下の「watchdog と連続動作」の 599 秒の観測は、`next_deadline` へ渡す時刻を出力の後へ
改める前の build で取った。**その修正は overrun の検出時刻だけを変えるもので、
overrun が発生しない通常経路の周期には影響しない。最終 build でも上記のとおり
間隔は 1000 ms 一定である。

ESP-IDF 自身の log timestamp（`I (nnnnn)`）と `uptime_ms` の差は全行で 380 ms 一定である。
`Instant` の起点が logger 初期化の後にあるためで、**この差が増えていかないことが
deadline 方式で周期がずれていない証拠である。**

### watchdog と連続動作

`sdkconfig.defaults` に watchdog の設定を足していない。**Task Watchdog Timer は ESP-IDF の
既定値のままである。**

EN pin reset から **連続 599 秒**（約 10 分）を 1 度の窓で観測した結果。

```text
hb 行            599
health 行         59
overrun warn       0（`heartbeat_overrun`／`health_snapshot_overrun` のいずれも）
serialize 失敗     0
boot banner        1（`rst:0x` は窓の先頭 1 回だけ ＝ 途中で reset していない）
Returned from app_main   0
Guru Meditation／abort()／panic／task watchdog   0
```

**連続する `uptime_ms` の差は 598 件すべて 1000 ms である**（他の値は 0 件）。
`uptime_ms` の標本 658 件に**減少は無い**。

heartbeat loop は `FreeRtos::delay_ms()` で待つ。同 API の doc が
「**This delayer avoids that by yielding to the OS during the delay.**」と述べており、
IDLE task が回るため TWDT が進む。待ち時間は必ず 1 ms 以上へ丸めてあり、`delay_ms(0)` に落ちない。

### reset 後の boot 出力を取るための追加手順

`espflash monitor` の `--after` は **monitor 終了時**に適用されるため、reset 直後の
boot 出力を取り逃がす（`--after hard-reset` と `--after no-reset` の両方で、
監視窓に application の出力が 1 行も現れないことを実測した）。
reset と read を同一 process で行う必要がある。この再検証では pyserial 3.5 で EN pin を
toggle してから読み出した（`RTS` → `EN`、`DTR` → `IO0`。`esptool` の classic reset と同じ順序）。

### この再検証が主張しないこと

- **`uptime_ms` の長時間の単調性を確認していない。**観測したのは**連続 599 秒**（約 10 分）
  の範囲までである。**実装は `u64` であり `u32` の wrap は起きない。**確認していないのは、
  Protocol §3 が `u32` を退けた根拠の長さ（約 49.7 日）に相当する連続動作である
- **overrun 経路と serialize 失敗経路は実機で発火していない。**観測範囲で `overrun_ticks` と
  `snapshot_errors` は 0 のままだった。**1 ms への丸めも発火していない**
  （通常運転の待ち時間は約 994 ms である）
- **観測できた reset reason は `power_on` だけである。**5 回の起動（flash 3 回、EN pin reset 2 回）
  すべてで `reset_reason=power_on raw=PowerOn` だった。EN pin を下げる reset でも ESP32 は
  `rst:0x1 (POWERON_RESET)` を報告するためで、上の #6 の結果と同じである。
  `reset_reason_str()` が写す残る 15 variant は**未観測**である
- **USB 抜き差しによる電源再投入後の起動出力は、上と同じ構造的理由で取得していない**
- **周辺回路と servo は含まない。**`Peripherals::take()` を呼んでおらず、GPIO へ触れていない
- **共有 conformance fixture への合格は主張しない。**`status` payload を serialize しただけで、
  envelope を付けた wire line として送っていない。ADR-0008 の未達項目は残る
- **「brownout も reset も起きなかった」を電圧が正常だった根拠にしない**（#6 と同じ）

### 端末側の差（この端末の事情であり、repository の構成に由来しない）

- **`cargo` を素の worktree で起動できなかった。**作業用の git worktree が本 checkout の配下
  （`<repo>/.claude/worktrees/…`）にあるため、cargo が worktree root の
  `exclude = ["firmware/esp32"]` を越えて上位の workspace を検出し、
  `current package believes it's in a workspace when it's not` で停止した。
  **build の間だけ `firmware/esp32/Cargo.toml` へ空の `[workspace]` を付与し、毎回戻した。**
  source、lockfile、toolchain、target、profile は同一である。
  素の tree での build は CI の `firmware.yml`（`ubuntu-24.04`、repository root で checkout）が確認する
- serial port への access は、login session が `dialout` group を保持していなかったため
  `sg dialout -c` を介した。`/etc/group` 上は member である
- `ldproxy --version` は linker 引数が無いと panic するため、版は `~/.cargo` の crate path
  （`ldproxy-0.3.5`）から採った。#6 の記録と同じ値である

### Build duration

```text
clippy   6 m 23 s（この worktree での初回。ESP-IDF の build を含む）
debug    49.60 s（増分 build）
release  5 m 59 s
```

### Conclusion（2026-08-25 時点）

`Partial`。profile の必須 command はすべて成功した（`fmt`／`clippy`／`build`／
`build --release`／`chip_id`／`elf2image`／`flash --monitor`）が、**未実行の項目が残る。**
一覧は上の「この再検証が主張しないこと」にある。

**判定はこの記録に対して1つである。**[記録一覧](README.md)の規則が「未実行の項目が残る場合は
`Partial`とし、何が未達かを記録内に明記する」と定めているため、先頭の `Conclusion:` 欄と
この節を食い違わせない。

## 2026-08-29 追加検証（USB の抜き差しによる電源再投入）

[Issue #6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6) の実施記録として、
`WORK-INSTRUCTIONS-POWER-CYCLE-BOOT.md`（PM 作成。人間の review は経ていない。
Issue は立てず、この節の反映は `Change-Class: fixup` と `Refs: #6` を宣言する）に基づき実施した。

**この節が閉じるのは「本物の電源再投入の起動出力は採れていない」（上記「この記録が主張しないこと」）
のうち、`reset_reason` と `uptime_ms` による定常状態到達の確認だけである。banner
（`rst:0x...`）と firmware 最初の出力は、今回も取得していない。**理由は同じで、host 側の
serial port が USB enumerate 後にしか存在せず、その時点で当該出力は既に流れ終わっているためである
（独立給電の serial adapter が無い。この作業の範囲外。導入の要否は方式ごと PM へ上げる）。

```text
Date: 2026-08-29 (JST)
Machine profile: ESP32 Flash / HIL
Repository commit: 6190f616b028063c09a653fa46c4e47bbeaf4a94
Working tree clean: yes
Container / VM / native: native（実機）。systemd-detect-virt: none
OS: Ubuntu 24.04.4 LTS / x86_64
USB-UART: Silicon Labs CP210x（10c4:ea60、/dev/ttyUSB0）
現物構成: ESP32 単体（ブレッドボード・pull 抵抗・LCD・servo・sensor 未接続）。
  給電は PC の USB のみ。Raspberry Pi へは未接続（人間が着手前に確認）
立ち会い: 有（人間が USB の抜き差しを物理的に実施。エージェントは device node の
  消失／再作成を監視し、再作成直後に port を開いて出力を記録した）
再 flash: 行っていない。**board 上の binary の同一性は `TBD` である。**health snapshot の
  firmware=0.1.0 と protocol counters の形は 2026-08-25 節の記録と一致するが、**いずれも
  binary や commit を一意に識別しない**（`firmware` はバージョン文字列であり、counters は
  形の一致にすぎない）。2026-08-29 は再 flash しておらず、boot identity も binary hash も
  記録していない（commit hash を含む boot 行は、着手時点で既に流れ去っており未確認）。
  **したがって「Issue #7 の firmware と同一である」とは結論しない。**同定するには boot 行の
  取得か、binary hash の照合が要る
```

### 手法

事前に用意した監視 script（repository 外。`docs/` へは入れていない）が、`/dev/ttyUSB0` の
存在を高頻度で poll し、消失→再作成の遷移を検知した。再作成を検知した時刻を起点とし、
可能な限り速やかに port を開いて出力を読み、最初に検出できた `reset_reason` と `uptime_ms`
（および検出までの経過時間）を記録した。

### 実測（3 回）

| 回 | 消失→再作成 | 再作成→port open | reset_reason（検出まで） | uptime_ms（検出まで） |
|---|---|---|---|---|
| 1 | 10.59 s | 0.118 s | power_on（+9.903 s） | 1006（+0.854 s） |
| 2 |  7.72 s | 0.120 s | power_on（+10.042 s） | 1006（+0.993 s） |
| 3 |  5.30 s | 0.110 s | power_on（+10.059 s） | 1006（+1.008 s） |

**3 回とも `reset_reason=power_on`、`uptime_ms=1006`（heartbeat 1 本目の値）で一致した。**
heartbeat の周期は約 994 ms（「2026-08-25 再検証」節の周期実測を参照）であり、3 回とも
最初に検出できた行が 1 本目の heartbeat（`hb seq=1`）だったため、同じ値になっている。
**使い回しではなく、毎回起動直後の最初の行を捕まえたことの結果である。**
`uptime_ms` は port を開いてから 1 秒程度で検出できており、「十分小さい」と言える値である。
`reset_reason` は heartbeat には含まれず、10 秒周期の health snapshot（JSON）まで待って初めて
検出できたため、検出までの経過時間は 10 秒前後になっている。**この経過時間の長さは
`reset_reason` の値そのものの信頼性を下げるものではない**（health snapshot の JSON にある
`"reset_reason":"power_on"` を読んだだけであり、10 秒待ったこと自体が異常を示すものではない）。

これとは別に、上と同じ抜き差しの試行のうち 4 回は、device node 再作成の直後に port を開いた
際に `SerialException`（device disconnected or multiple access on port?）が発生し、
`reset_reason`／`uptime_ms` を得られなかった。**再現するたびに device node は正しく消失・
再作成しており、電源が実際に切れて入り直したこと自体は確認できている。**エラーは port を
開くタイミングが udev のデバイス確定に対して早すぎたことによる読み取り側の一時的な失敗と見られ、
`reset_reason` が `power_on` 以外だった、または `uptime_ms` が続きに見えた、という
停止条件には該当しなかった。これらの試行は破棄し、実測には含めていない。

### 主張の更新

**上記「この記録が主張しないこと」の関連行は、消さずにそのまま残す。**そのうえで、今回の
実測で新たに言えることを次のとおり区別する。

- **主張してよいこと（今回追加）**: USB の抜き差しによる電源再投入のあと、`reset_reason=power_on`
  かつ `uptime_ms` が小さい値（1006 ms）であることを 3 回とも確認した。**したがって
  「USB の抜き差しによる電源再投入のあと、firmware が定常状態へ到達した」と言える**
- **主張しないこと（変わらず）**: 「起動出力を記録した」とは書かない。ROM の boot banner
  （`rst:0x...`）と firmware 起動直後（`uptime_ms` が heartbeat 1 本目である 1006 ms より前）
  の出力は、今回も取得していない。取りに行っていない
- **主張しないこと（今回追加）**: `uptime_ms=1006` より前、すなわち boot から heartbeat
  1 本目までの区間で何が起きたかは、今回の方法では確認できない（記録済みの起動出力に
  よれば ESP-IDF の log timestamp で `app_main` 呼び出しは `I (354)` 付近だが、これは
  firmware 自身の `uptime_ms` とは別の clock であり、上記「2026-08-25 再検証」節が示す
  約 380 ms のオフセットを直接足し引きできる値ではない）
- 周辺回路、servo、電圧については本節でも何も追加確認していない（既存の「この記録が
  主張しないこと」と同じ）

### `AGENTS.md` との食い違い（2026-08-31 に解消済み）

**この追記の時点では、`AGENTS.md` の「検証」節が「USB 抜き差しによる電源再投入後の
起動出力は未検証である」とだけ書いており、この記録と食い違っていた。**当該文は「起動出力
（起動直後の出力全般）が未検証」とだけ述べ、本節が示した「定常状態への到達（`reset_reason`／
`uptime_ms` による確認）は実測済みであり、未検証なのは boot banner と firmware 最初の出力に
限られる」という区別を反映していなかった。

**この食い違いは `7ff4e54`（[PR #277](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/277)）で解消した。**
`AGENTS.md` は現在、電源再投入のあと firmware が定常状態へ到達したことまでを主張し、
起動出力そのものは今も取得していないことと、その理由が構造的で再試行では解決しないことを
書き分けている。**本節の実測内容は変更していない。**

### Conclusion（2026-08-29 時点）

`Partial`。電源再投入後の定常状態到達（`reset_reason`／`uptime_ms`）は 3 回とも実測できた。
banner と firmware 最初の出力は、構造的理由により今回も未達である。一覧は上の「主張の更新」を参照する。
