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
controllerの挙動は`ILI9341 Datasheet V1.13`（Ilitek、2011-08-05）による。2026-08-12取得。
**Ilitek公式の配布先は見つかっていないため、[Adafruitがhostする版](https://cdn-shop.adafruit.com/datasheets/ILI9341.pdf)を開いた**
（PDF metadataのTitleが`ILI9341_DS_V1.13_20110805.doc`。245page）。**mirrorであり、公式配布物との同一性は確認していない。**

> **module datasheetとcontroller datasheetが矛盾する箇所がある**（`DC/RS`の極性）。
> **その1箇所については後者を正とする。**理由は下表の`Data／command動作`欄に記す。

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
| Data／command動作 | `DC/RS` pin。**`low`＝command、`high`＝data。**`ILI9341 Datasheet V1.13`が`When DCX = '1', data is selected. When DCX = '0', command is selected.`と定め、4-line serial interfaceの節も`If the D/CX bit is "low", the transmission byte is interpreted as a command byte`と一致する。**MSP2807のメーカーdatasheetは`high level: register, low level: data`と逆に記載しているが、この値は採らない。**`DC/RS`はcontrollerの`D/CX`へ直結し、送出byteの解釈を決めるのはILI9341であるためである（この表の`Controller IC`欄のとおり、本moduleのcontrollerはILI9341である）。**現物での確認はLCD bring-up（[#13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)）のbench試験で行う** |
| Backlight回路／電流／polarity | polarityは**highで点灯**（datasheet記載）。**回路と電流は未公開**であり、datasheetの`Power Consumption`欄は`TBD`と印字されている。**配線方法はここに書かない。**`HW-TBD-024`（このmoduleが耐えられる電流の上限）が未解決の間は、`LED` pinへ電源を直結してよいかを判定できない。配線規則の正本は[power-budget.md](power-budget.md)であり、現物のbacklight回路と安全な電流上限を確認した後に定める。追跡は[tbd-register HW-TBD-024](tbd-register.md) |
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
| 5 | `DC/RS` | command（`low`）／data（`high`）選択。**メーカーdatasheetの14pin表は逆に記載している**（上表`Data／command動作`欄を参照） |
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
- **`DC/RS`の極性**（module datasheetとcontroller datasheetで記載が逆であるため）

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

**ICの値とmodule boardの値を分けて記録する。**根拠の種類が違うためである。
ICの値はメーカーdatasheetで決まるが、module boardの値（jumper構成、実装済みaddress、
搭載pull-up、regulatorの有無）は**そのboardの資料か現物でしか決まらない**。

到達状況は次のとおりである。

| 資料 | 状況 |
|---|---|
| [Analog Devices ADXL345 Data Sheet](https://www.analog.com/media/en/technical-documentation/data-sheets/adxl345.pdf) | **本作業環境から開けない。**2026-08-10と08-11に`analog.com`（小文字／大文字の両path）とMouser mirrorをWebFetch／curl／PowerShellで試し、いずれもECONNRESETまたはtimeoutであった |
| [Analog Devices ADXL345製品ページ](https://www.analog.com/en/products/adxl345.html) | **本作業環境から開けない。**2026-08-12に再試行し、45秒でtimeoutした。[hardware-bom.md](hardware-bom.md) `ACCEL-01`が公式文書欄に載せているURLと同じである |
| 秋月 M-06724 商品ページ | **404**（`gM-06724`／`g106724`とも） |
| [SparkFunがhostするADXL345 Data Sheet](https://www.sparkfun.com/datasheets/Sensors/Accelerometer/ADXL345.pdf) | **2026-08-12に開けた。**PDF metadataは`/Author: Analog Devices, Inc.`、`/Title: ADXL345 (Rev. 0)`、`/Category: Data Sheet`、作成日2009-05-29。24page。**mirrorであり公式配布物との同一性は確認していない。またRev. 0が現行revisionである保証はない**（`analog.com`へ到達できないため最新版と照合できていない） |

**下表のICの値は、上記のSparkFun版（Rev. 0）で確認した範囲に限る。**revisionが確認できない以上、
**この文書の値をもって最新版の内容とはしない。**現行revisionとの照合は`analog.com`へ到達できる環境で行う。
module boardの値は、これとは別に現物でしか決まらない（[AGENTS.md](../../AGENTS.md) 推測禁止）。

### ICの値（Analog Devices ADXL345）

| 項目 | 値 |
|---|---|
| IC識別情報 | Analog Devices ADXL345 |
| 供給電圧 | `VS` 2.0–3.6 V（`Supply voltage range: 2.0 V to 3.6 V`）。interface用の電源は別系統の`VDD I/O`で、`VS ≤ 2.5 V`のとき1.7 V–`VS`、`VS ≥ 2.5 V`のとき2.0 V–`VS`である。**記号は`VS`と`VDD I/O`である**（pin 6＝`VS` Supply Voltage、pin 1＝`VDD I/O` Digital Interface Supply Voltage）。**絶対最大定格はこの行では扱わない。**`HW-TBD-025`(b)（Issue [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)）の範囲である |
| 測定range | ±2 g／±4 g／±8 g／±16 g 選択式 |
| Interface（ICの対応） | I2C／SPI（3線式・4線式）の両対応（`SPI (3- and 4-wire) and I2C digital interfaces`） |
| Filter／FIFO機能 | **32段のFIFOを内蔵**（`embedded 32-level FIFO`）。modeは`bypass`／`FIFO`／`stream`／`trigger`の4種で、`FIFO_CTL` registerの`FIFO_MODE` bitsで選ぶ。**各modeの詳細な挙動と、driverでどれを使うかはTBD** |
| Device ID register／value | TBD |
| Sensitivity変換 | TBD |
| Output data rate | TBD |
| Interrupt pinと動作 | INT1（pin 8）／INT2（pin 9）の2本。polarityと駆動形式はTBD |
| 起動／reset sequence | TBD |
| I2Cアドレスの選択方式 | pin 12 `SDO/ALT ADDRESS`で選ぶ。**7-bit addressは`0x1D`（同pinをhigh）／`0x53`（同pinをGNDへ）**。**ただしどちらになるかはmodule board上の実装で決まる**ため、現物確認まで確定しない（下表`実装済みI2C address`と[tbd-register HW-TBD-004](tbd-register.md)） |
| Calibration要件 | TBD |

### module boardの値（秋月 M-06724）

**すべて現物確認が要る**（[tbd-register HW-TBD-004](tbd-register.md)）。商品ページが404で資料が無い。

| 項目 | 値 |
|---|---|
| Module識別情報 | ADXL345モジュール（秋月 M-06724） |
| 実装されているinterface（jumper設定） | TBD |
| 実装済みI2C address | TBD |
| board上のregulatorの有無 | TBD。**旧記載の「regulator非搭載」は根拠資料（秋月 商品ページ。現在404）を失ったため、2026-08-11に現物確認待ちへ改めた**（[hardware-bom.md](hardware-bom.md) Revision 31）。**載っていないことを前提にしない** |
| moduleへ入れてよい電圧 | TBD。**ICの上限3.6 Vをboardの許容値と同一視しない**（regulatorやlevel shiftの有無で変わる）。**[power-budget.md](power-budget.md)と[hardware-bom.md](hardware-bom.md)が置く「3.3 Vで給電する」は、この行が埋まるまで確定しない。****5 V直結の禁止と、この行は別の主張である。**5 V禁止は「moduleがregulatorを持たなければ5 VがICへ直接掛かる」ことを否定できないための**確認前の安全規則**であって、IC定格からmoduleの許容入力電圧を導いたものではない。3.3 Vをmoduleが受け入れる根拠も同様に別に要る（[tbd-register HW-TBD-004](tbd-register.md)） |
| Module搭載pull-up | TBD（有無と値） |

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
| 2026-08-11 | 2 | [PR #82](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/82)のレビュー指摘2件を反映。(a) LCD moduleのbacklight欄に「制御しない場合は3.3 V直結」と**配線方法を書いていた**。`HW-TBD-024`（このmoduleが耐えられる電流の上限）が未解決の間は直結してよいか判定できず、配線規則の正本は`power-budget.md`である。polarityの記録だけに戻し、配線は現物確認後に定めるとした。(b) Accelerometer節が**ICの値とmodule boardの値を1つの表に混ぜていた**。根拠の種類が違う（ICはdatasheet、moduleはboard資料か現物）ため2つの表へ分けた。あわせて`analog.com`とMouser mirrorへ到達できなかった経緯を記録し、**開いていない文書のrevisionとpage番号は記録しない**方針を明示した |
| 2026-08-11 | 3 | [PR #82](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/82)の自己レビューで検出。**Revision 2が「datasheetを開けていない」と書きながら、そのdatasheet由来の記号名を書いていた。**Accelerometer節で供給電圧の記号を`VS`と表記し、I2Cアドレス選択pinを`ALT ADDRESS`と名指ししていたが、どちらも一次資料で確認していない。記号名とpin名を落とし、値の範囲（2.0–3.6 V）と`hardware-bom.md`の既存表記だけを残した |
| 2026-08-12 | 4 | 昇格PR[#109](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/109)のレビュー指摘2件を反映。(a) **`DC/RS`の極性が、controllerの実挙動と逆であった。**module datasheetの14pin表は`high level: register, low level: data`と記載しているが、`ILI9341 Datasheet V1.13`は`When DCX = '1', data is selected. When DCX = '0', command is selected.`と定める。`DC/RS`はcontrollerの`D/CX`へ直結するため**ILI9341を正**とし、両者が食い違う事実と理由を記載した。**module datasheetの記載を消していない**（出典との差を辿れなくなるため）。あわせてbench試験の項目へ極性確認を追加した。(b) **ADXL345のICの値に、開いた出典が無かった。**`analog.com`は2026-08-12にも到達できない（45秒timeout）ままだが、**SparkFunがhostするRev. 0版を開けた**ため到達状況表へ追加し、既記載の値（供給電圧、測定range、interface）をその版で確認した。あわせて`VS`／`VDD I/O`の記号名、FIFOの32段と4 mode、`SDO/ALT ADDRESS`による`0x1D`／`0x53`の選択を記録した。**Revision 3で落とした`VS`と`ALT ADDRESS`は、いずれも結果として正しかった。**ただし当時は開いていない資料の内容であり、落とした判断は当時の根拠に照らして正しい。**mirrorであり公式配布物との同一性も現行revisionであることも確認できていない**ため、その旨を明記した。**絶対最大定格は引き続き扱っていない**（`HW-TBD-025`(b)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)） |
