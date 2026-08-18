# 測定治具: 電源過渡の独立観測（`HW-TBD-034` 方式1）

`HW-TBD-034` は 2026-08-16 に**方式1（独立した外部観測）で決定済み**である。ここに置くのは、
その方式を成立させるための測定治具の source である。

**これは測定治具であって DeskCat firmware ではない。**Arduino は測定器であり開発端末ではないため、
[machine-profiles.md](../../docs/toolchains/machine-profiles.md) の profile は要さない
（デジタルテスターと同じ扱い）。`crates/` と `firmware/` には何も足していない。

**要件値・合格条件・測定結果はここへ書かない。**正本は次である。

- [power-budget.md](../../docs/hardware/power-budget.md) の `Sample rateとlog形式`、
  `手持ち候補に固有の制約: SRAMと取得方式`、`手持ち候補の現物識別`、
  `基準電圧が未解決である（設計上の論点）`、`5 V railをADCへ直結する条件`、
  `GND topology（測定前に必ず確定させる）`
- [hardware-bom.md](../../docs/hardware/hardware-bom.md) の `MEAS-01`、`MEAS-02`
- [tbd-register.md](../../docs/hardware/tbd-register.md) の `HW-TBD-034`

## 構成

| file | 役割 |
|---|---|
| `arduino-transient-logger/arduino-transient-logger.ino` | Arduino Uno R3 側。ADC を free running で回し、連続 streaming で PC へ流す |
| `arduino-vref-calibrate/arduino-vref-calibrate.ino` | Arduino Uno R3 側。基準電圧の校正用。AV<sub>CC</sub> 基準で内蔵 1.1 V を読む |
| `adc_stream_rate.py` | PC 側。実効 sample rate、取りこぼし、channel別の生値を集計する |
| `test_adc_stream_rate.py` | 合成した byte 列に対する parser の検証。**board を占有せずに回せる** |

`adc_stream_rate.py` は **Python 3 の標準ライブラリだけ**を使う
（[ADR-0006](../../docs/decisions/0006-validation-script-language.md)）。`pyserial` は導入しない。

## なぜ burst capture ではなく連続 streaming か

ATmega328P の SRAM は 2 KB であり、`Sample rateとlog形式` が定める burst capture の
buffer 長を満たせない。一方 **Arduino の USB は DeskCat の Protocol と無関係で測定に専有できる**ため、
ESP32 側が burst capture を採った理由がそもそも当てはまらない。詳細と帰結は
`手持ち候補に固有の制約: SRAMと取得方式` にある。

## 前提

- 実機 Linux（[ADR-0005](../../docs/decisions/0005-standard-development-os.md)）
- `arduino-cli` と `arduino:avr` core。導入した版は Version Record へ記録する
- serial device を開ける権限。`dialout` group へ追加したうえで、**再 login していない session では
  `sg dialout -c '...'` で包む**

```bash
sg dialout -c 'PATH="$HOME/.local/bin:$PATH" arduino-cli board list'
```

## 使い方

compile（board 不要）。

```bash
arduino-cli compile --fqbn arduino:avr:uno --warnings all hardware/measurement/arduino-transient-logger
```

parser の自己 test（board 不要）。

```bash
python3 hardware/measurement/test_adc_stream_rate.py
```

upload。**port は環境依存なので固定値を document へ書かない。**`arduino-cli board list` で確かめる。

```bash
sg dialout -c 'PATH="$HOME/.local/bin:$PATH" arduino-cli upload --fqbn arduino:avr:uno --port /dev/ttyACM0 hardware/measurement/arduino-transient-logger'
```

測定。

```bash
sg dialout -c 'python3 hardware/measurement/adc_stream_rate.py --port /dev/ttyACM0 --baud 1000000 --seconds 10'
```

### parameter を振る

sketch 側は compile 時定数である。`arduino-cli` から上書きする。

```bash
arduino-cli compile --fqbn arduino:avr:uno --build-property compiler.cpp.extra_flags="-DLOGGER_BAUD=500000 -DLOGGER_ADPS=6 -DLOGGER_NCH=1" hardware/measurement/arduino-transient-logger
```

| 定数 | 意味 |
|---|---|
| `LOGGER_BAUD` | UART の baud。**PC 側の `--baud` と一致させる** |
| `LOGGER_ADPS` | ADPS2:0。ADC clock = F_CPU / 2^ADPS。`DS40002061B` §24.4 は最大分解能に 50〜200 kHz を要求する |
| `LOGGER_NCH` | 1 または 2。2 のとき `A0` と `A1` を交互に読む |
| `LOGGER_BLOCK` | 1 block の sample 数。header 22 B に対する payload の割合を決める |

## 入力の配線（作業1）

`A0` → `3V3` pin、`A1` → `GND`。**抵抗は要らない。**どちらも入力範囲 0〜AV<sub>CC</sub> 内である。
`DeskCat` 側からは取らない（Arduino 自身の pin だけを使う）。

**2 channel の値を意図的に違えるのが要点である。**同電圧では、下に述べる channel 帰属の
1 変換遅れを取り違えても気付けない。

## block 形式

little endian。header は 22 B 固定。

| offset | size | field |
|---|---|---|
| 0 | 2 | magic `0xA5 0x5A` |
| 2 | 2 | block sequence（wrap する） |
| 4 | 4 | `taken`。ISR が取得した sample 数の累計（**捨てたぶんと、ring に滞留中のぶんを含む**） |
| 8 | 2 | `dropped`。ring が満杯で捨てた sample 数の累計 |
| 10 | 4 | `mark_us`。`mark_taken` の時点の `micros()` |
| 14 | 4 | `mark_taken` |
| 18 | 1 | この block の sample 数 |
| 19 | 1 | `pending`。**この block の sample を取り出す前の ring 滞留数** |
| 20 | 1 | cfg。bit0-2 = ADPS、bit3 = (channel 数 == 2)、bit4-7 は 0 で予約 |
| 21 | 1 | header 全 byte の XOR |

`pending` が要るのは、`taken` が ring 滞留分を含むためである。これが無いと
PC 側の収支（`taken の増分 = 届いた + 捨てた + ring滞留の増減 + 回線上の欠落`）が閉じず、
正常な測定に対して「不整合」を誤報しうる。CSV の時刻復元でも、
各 block の先頭 sample の取得 index を `taken - pending` として引き直すために使う。

payload は sample 数 × 2 B。各 sample は bit 0-9 が生 ADC 値、bit 10 が channel、
**bit 11-15 は 0 で予約**する。PC 側はこの予約 bit で framing の健全性を確認する。

## channel 帰属は 1 変換ぶん遅れる（`DS40002061B` §24.5.1）

Free Running mode では、ISR で `ADMUX` を書き換えても**次に上がる結果は「前の channel」のもの**である。

> Since the next conversion has already started automatically, the next result will reflect
> the previous channel selection. Subsequent conversions will reflect the new channel selection.

したがって sketch は「いま読んだ結果の channel」と「すでに飛行中の変換の channel」を別に保持し、
ISR で書く値はそのさらに次の変換に効く、として扱う。

**この段数が正しいかは実測で確かめる。**`A0` に `3V3`、`A1` に `GND` を入れたとき
`ch0 > ch1` になるはずで、逆なら段数が 1 つずれている。`adc_stream_rate.py` はこの判定を報告する。

## この治具が保証しないこと

- **`sample間` の未観測区間は残る。**連続 streaming にしても 0 にはならない。
  「電源過渡をすべて実測済み」とは扱わない（正本は
  `ESP32自身のADCは測定対象から独立していない`）
- **header の完全性検査は XOR であり CRC ではない。**同一 byte 位置で偶数回の bit 反転は見逃す。
  magic と予約 bit の検査を併用して補っているが、通信路の誤り検出を目的とした設計ではない
- **CSV の時刻は復元値である。**block header の `micros()` mark を anchor に、
  sample index から線形に補間している。**sample 毎の実測時刻ではない。**
  index は block ごとに `taken - pending` から引き直すため、回線上で block を失っても
  次の block で自己修復する。**ただし ISR が sample を捨てた場合は、
  取得 index と届いた sample の対応そのものが崩れる。**
  取りこぼしと回線上の block 欠落のどちらかがあれば、既定では CSV を書かない
- **基準電圧の校正は別作業である。**ADC は基準電圧に対する割合を返すので、
  生値のままでは絶対電圧の閾値と照合できない（正本は `基準電圧が未解決である（設計上の論点）`）。
  **電圧へ戻すときの分母は 1024 である。**§24.7 が `ADC = (V_IN * 1024) / V_REF` と定めている。
  慣習的に使われる 1023 ではない
- **DeskCat 側へ接続する場合は GND topology の確定が先である。**star point 構成を崩すと
  全 ADC 値がずれる。PC の GND が経路へ入る影響も別に評価する
