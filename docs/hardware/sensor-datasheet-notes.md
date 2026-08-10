# Sensor and Display Datasheet Notes

> 状態: Blocked — 正確なmoduleの現物確認が必要
> 正本とする情報: Driverに必要な、データシート由来のdevice動作
>
> **公開specから確定する欄は2026-08-10に埋めた。**残る`TBD`は次のいずれかである。
> (a) 現物確認が要るもの（jumper状態、address、touch controller型番、backlight回路）、
> (b) メーカーが公開していないもの（MSP2807のSPI mode／max clock、初期化sequence、消費電流）、
> (c) bench試験で決めるもの。**推定で埋めない**（[AGENTS.md](../../AGENTS.md) 推測禁止）。

## 使用方法

物理moduleごとに一つのsectionを完成させる。ICのデータシートとmodule board文書の両方を記録する。候補例をもとにsectionを埋めない。

各文書について次を記録する。

- メーカー
- 正確なmodelとsuffix
- 文書名
- Revision／date
- 公式URL
- 関連page／table
- Module回路図／pinout
- 現物識別の根拠

## LCD module

出典: メーカーdatasheet [`msp2807.pdf`](https://akizukidenshi.com/goodsaffix/msp2807.pdf)（[秋月商品ページ](https://akizukidenshi.com/catalog/g/g116265/) 添付、revision表記なし）、
[LCD Wiki MSP2807](http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807)。2026-08-10取得。

| 項目 | 値 |
|---|---|
| Module識別情報 | MSP2807（touch付。touch無しは MSP2806）。PCB 50.0×86.0 mm、AA 43.2×57.6 mm（datasheet記載のまま） |
| Controller IC | `ILI9341` |
| 解像度 | **datasheetの表記は`320×240`。**一方[hardware-bom.md](hardware-bom.md) DISP-01は`240×320`と記載している。**向きの取り方が違うだけと思われるが確認していない。**どちらをdriverの基準にするかは現物のbench試験（単色fill、四隅の座標pattern、rotation）で確定する |
| Color format／order | RGB 65K color。byte orderはTBD（**現物のbench試験で確認する**） |
| Interface | 4-wire SPI（14pin。LCDとtouchでbusを共有し、CSを分ける） |
| 供給電圧 | VCC 3.3–5 V |
| Logic電圧 | **3.3 V TTL**。`hardware-bom.md` DISP-01のとおり、5 V給電時の出力levelがメーカー資料で不明なため3.3 V給電とする |
| SPI mode／max clock | TBD（datasheetに記載なし。**現物のbench試験で確認する**） |
| Reset polarity／timing | polarityは**low activeでreset**（`RESET` pin）。timingはTBD |
| Chip-select timing | `CS`は**low activeで有効**。timingはTBD |
| Data／command動作 | `DC/RS` pin。**high＝register、low＝data** |
| Backlight回路／電流／polarity | polarityは**highで点灯**（制御しない場合は3.3 V直結で常時点灯）。**回路と電流は未公開。**datasheetの`Power Consumption`欄は`TBD`と印字されている。追跡は[tbd-register HW-TBD-024](tbd-register.md) |
| 初期化sequence | TBD（datasheetに記載なし） |
| 対応orientation command | TBD |
| Readback機能 | `SDO(MISO)`あり。**read機能が不要なら未接続でよい**とdatasheetが明記 |
| 電源投入後に必要なdelay | TBD |
| 動作／保存温度 | 動作 -20〜60 ℃、保存 -30〜70 ℃ |
| Driver／library候補 | TBD |

### pin定義（datasheet記載、14pin）

| No. | ラベル | 説明 |
|---|---|---|
| 1 | `VCC` | 5V／3.3V power input |
| 2 | `GND` | Ground |
| 3 | `CS` | LCD chip select（low有効） |
| 4 | `RESET` | LCD reset（low有効） |
| 5 | `DC/RS` | register（high）／data（low）選択 |
| 6 | `SDI(MOSI)` | SPI write |
| 7 | `SCK` | SPI clock |
| 8 | `LED` | backlight制御（high点灯） |
| 9 | `SDO(MISO)` | SPI read（不要なら未接続可） |
| 10 | `T_CLK` | touch SPI clock |
| 11 | `T_CS` | touch chip select（low有効） |
| 12 | `T_DIN` | touch SPI input |
| 13 | `T_DO` | touch SPI output |
| 14 | `T_IRQ` | touch割り込み（touch検出時low） |

必要なベンチ試験の根拠:

- 単色fill
- Color-order pattern
- 四隅の座標pattern
- Rotation
- 全体更新と部分更新の時間
- Touchとserialがactiveな状態での動作

## Touch controller

出典: 上記と同じdatasheet。**controllerの型番はdatasheetに記載が無い**ため、多くの欄が現物確認待ちである
（追跡は[tbd-register HW-TBD-003](tbd-register.md)）。

| 項目 | 値 |
|---|---|
| Module／controller識別情報 | Moduleは MSP2807（DISP-01と同一）。**controller ICの型番はメーカー未公開。**`XPT2046`系と推定されるが**未確認**であり、現物chip刻印で確定する |
| Touch方式 | TBD（controller型番の確定待ち。resistiveと推定） |
| Interface | SPI（LCDとbusを共有し、`T_CS`で分ける）。信号は`T_CLK`／`T_CS`／`T_DIN`／`T_DO`／`T_IRQ` |
| 供給／logic電圧 | DISP-01と同一（VCC 3.3–5 V、logic 3.3 V TTL） |
| AddressまたはSPI mode | TBD |
| 検証済み最大bus速度 | TBD |
| IRQ polarity／type | **touch検出時にlow**（datasheet記載）。edge／levelの別はTBD |
| Raw出力 | TBD |
| 座標／強度の動作 | TBD |
| Reset／起動sequence | TBD |
| Module pull-up | TBD |
| Calibration方法 | TBD |

必要なベンチ試験の根拠:

- Idle時のraw noise
- Press／release動作
- 座標がある場合は四隅と中央
- Rotation変換
- 撫でgestureのsample
- LCD通信中の動作

## Accelerometer

出典: [Analog Devices ADXL345 Data Sheet](https://www.analog.com/media/en/technical-documentation/data-sheets/adxl345.pdf)。
**`analog.com`へは本作業環境から接続できず（2026-08-10、複数手段でECONNRESET／timeout）、datasheetの内容を未検証である。**
また**秋月 M-06724の商品ページは404**であり、module board側の資料も入手できていない。
したがって下表はほぼ全欄が未確定であり、**IC datasheetから推定して埋めない**（[AGENTS.md](../../AGENTS.md) 推測禁止）。

| 項目 | 値 |
|---|---|
| Module／IC識別情報 | Module: ADXL345モジュール（秋月 M-06724）。IC: Analog Devices ADXL345 |
| Interface | I2C または SPI（3線式／4線式）、選択式。**現物のjumper設定で確定する**（[tbd-register HW-TBD-004](tbd-register.md)） |
| 供給／logic電圧 | VDD 2.0–3.6 V（VDD I/Oは別系統）。**M-06724はregulator非搭載とされるが、根拠資料へ現在アクセスできないため現物で確認する** |
| Address／select pin | TBD（現物のjumper／pin設定確認要） |
| Device ID register／value | TBD（datasheet未検証のため記載しない） |
| 測定range | ±2 g／±4 g／±8 g／±16 g 選択式（datasheetの表題に含まれる範囲） |
| Sensitivity変換 | TBD |
| Filter／FIFO機能 | FIFOを内蔵（Analog Devicesの製品説明による。**深さと動作はdatasheet未検証のためTBD**） |
| Output data rate | TBD |
| Interrupt pinと動作 | INT1／INT2。polarityと駆動形式はTBD |
| 起動／reset sequence | TBD |
| Module pull-up | TBD（module搭載pull-upの有無と値を現物で確認する） |
| Calibration要件 | TBD |

必要なベンチ試験の根拠:

- Device ID
- 複数の静止姿勢におけるraw XYZ
- Offsetとnoise分布
- 軽打sample
- 机の振動sample
- サーボ動作時sample
- Eventのfalse-positive／false-negative data

## Environmental sensor

出典: [Bosch BME280 Data Sheet](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf)
（`BST-BME280-DS001-24`、Revision 1.24、2024年2月）、
[AE-BME280 製品説明書](https://akizukidenshi.com/goodsaffix/AE-BME280_manu_v1.1.pdf)（v1.1、2015-06-02）。2026-08-10取得。

| 項目 | 値 |
|---|---|
| Module／IC識別情報 | Module: AE-BME280（秋月 K-09421）。IC: Bosch BME280。6pin SIP（2.54 mmピッチ）、基板16×10 mm |
| 測定量 | 温度、湿度、気圧 |
| Interface | I2C（最大3.4 MHz）または SPI 4線式／3線式（最大10 MHz）、選択式。**現物の`J1`／`J2`／`J3`で確定する**（[tbd-register HW-TBD-005](tbd-register.md)） |
| 供給／logic電圧 | VDD 1.71–3.6 V、VDDIO 1.2–3.6 V。**module上でVDDとVDDIOは接続済み**のため実効は1.71–3.6 V |
| Address／select pin | I2C: `0x76`（`SDO`→GND、既定）／`0x77`（`SDO`→VDD）。I2C選択時は`J3`をはんだジャンパする |
| Device ID | register `0xD0`、reset値 `0x60` |
| 起動時間 | `t_startup` 2 ms（VDD > 1.58 V かつ VDDIO > 0.65 V を満たしてから最初の通信まで） |
| 測定／変換時間 | oversampling設定に依存。詳細はBosch datasheet |
| 測定range | 温度 -40〜+85 ℃、湿度 0〜100 %、気圧 300〜1100 hPa |
| 精度と分解能 | 精度: 温度 ±1 ℃、湿度 ±3 %、気圧 ±1 hPa。分解能: 温度 0.01 ℃、湿度 0.008 %、気圧 0.18 Pa |
| Calibration coefficientの処理 | `0x88`–`0xA1` と `0xE1`–`0xF0` の calibration data を読み出して補償演算に使う |
| CRC／data integrity動作 | TBD |
| Heaterがある場合の動作／電流 | Heaterなし |
| 推奨測定rate | TBD（normal modeの`t_standby`設定で決まる） |
| Module pull-up | I2C用 4.7 kΩ×2（SDA用R1、SCL用R2）を搭載。**`J1`／`J2`のはんだジャンパで接続を選択する。**現物の実装状態を確認し、bus全体の実効pull-upを計算する |
| 消費電流 | sleep 0.1 µA（typ）／0.3 µA（max）、standby 0.2／0.5 µA、湿度測定 340 µA（85 ℃でのmax）、気圧測定 714 µA（-40 ℃でのmax）、温度測定 350 µA（85 ℃でのmax）。**いずれも動作値であって絶対最大定格ではない** |

### jumper（AE-BME280）

| jumper | 意味 |
|---|---|
| `J1` | I2C時の SDA 用プルアップ（R1 4.7 kΩ）の選択 |
| `J2` | I2C時の SCL 用プルアップ（R2 4.7 kΩ）の選択 |
| `J3` | **I2C時にはんだでジャンパする**（`CSB`→VDD） |

**SPI 4線式／3線式で使う場合は`J1`〜`J3`をすべてオープンにする。**

pin配列: 1=`VDD`, 2=`GND`, 3=`CSB`, 4=`SDI`, 5=`SDO`, 6=`SCK`。

必要なベンチ試験の根拠:

- Device ID
- Raw値と変換値
- Reference instrumentとの比較
- 安定した反復sampling
- Timeout／disconnect動作
- サーボ動作中の動作

## 共有I2Cのreview

正確なdevice選定後、次を確認する。

- [ ] すべてのaddressが一意、または競合解決方法が文書化されている
- [ ] すべてのlogic電圧に互換性がある
- [ ] 各moduleのpull-up値と接続先が明確である
- [ ] 並列合成後の実効pull-upを計算した
- [ ] 選定した速度にすべてのdeviceが対応している
- [ ] Clock stretchingとrepeated-start要件に対応している
- [ ] Timeoutとbus recovery動作を定義した
- [ ] 組み立て後の配線でwaveformを取得した

## 共有SPIのreview

正確なdevice選定後、次を確認する。

- [ ] 各deviceに個別のchip selectがある
- [ ] Deviceごとにmode、word size、bit order、速度が明確である
- [ ] Reset、data／command、backlight、IRQ信号をinventoryへ記録した
- [ ] 非選択時のMISO動作が明確である
- [ ] LCD転送scheduleがtouchをstarveさせない
- [ ] 組み立て後のハードウェアを基準に速度を検証した

## Revision履歴

| 日付 | Revision | 変更 |
|---|---|---|
| 2026-07-27 | 0 | 必要なデータシート項目とベンチ試験根拠を作成 |
| 2026-08-10 | 1 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)。**一次資料を特定し、公開specから確定する欄を埋めた。**LCD module（[msp2807.pdf](https://akizukidenshi.com/goodsaffix/msp2807.pdf)）は14pin定義・`ILI9341`・320×240・VCC 3.3–5V・logic 3.3V TTL・各信号のpolarityを確定し、pin定義表を追加した。Touch controllerは**datasheetに型番の記載が無い**ことを記録した。Environmental sensor（[Bosch Rev 1.24](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf)＋[AE-BME280説明書 v1.1](https://akizukidenshi.com/goodsaffix/AE-BME280_manu_v1.1.pdf)）は電圧範囲・address・Device ID・起動時間・測定range・精度・消費電流・module pull-upを確定し、jumper表とpin配列を追加した。Accelerometerは**`analog.com`へ接続できず、秋月 M-06724の商品ページも404**のため、確定できた欄だけを埋め、残りは推定せず`TBD`のまま残した。**各moduleの絶対最大定格は`HW-TBD-025`(b)（Issue [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)）の範囲であり、この改訂では扱っていない** |
