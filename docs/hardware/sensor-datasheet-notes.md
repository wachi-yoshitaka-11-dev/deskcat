# Sensor and Display Datasheet Notes

> 状態: Blocked — 正確なmoduleの現物確認が必要
> 正本とする情報: Driverに必要な、データシート由来のdevice動作
>
> **公開specから確定する欄は2026-08-10に埋めた。**残る`TBD`は次のいずれかである。
> (a) 現物確認が要るもの（jumper状態、address、backlight回路）。**touch controller型番は2026-08-13に`XPT2046`と確定した**、
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
| 解像度 | **現物silkは`2.8 TFT SPI 240X320 V1.2`である**（2026-08-13確認。パネルは`HSD028309 A2`）。[hardware-bom.md](hardware-bom.md) DISP-01の`240×320`と一致する。**一方メーカーdatasheetの表記は`320×240`で向きが逆である。**向きの取り方の違いと思われるが、どちらをdriverの基準にするかは現物のbench試験（単色fill、四隅の座標pattern、rotation）で確定する |
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

出典: 上記と同じdatasheet。**controllerの型番はdatasheetに記載が無い**ため、
**型番は2026-08-13に現物chip刻印で`XPT2046`と確定した**（追跡は[tbd-register HW-TBD-003](tbd-register.md)）。

**下表の`TBD`は、型番が決まったことで「現物確認待ち」から「`XPT2046`のdatasheetを当てる作業」へ移った。**
まだ当てていないため値は入れない（[AGENTS.md](../../AGENTS.md) 推測禁止）。

| 項目 | 値 |
|---|---|
| Module／controller識別情報 | Moduleは MSP2807（DISP-01と同一）。**controller ICは`XPT2046`（確定）。**現物裏面`U2`（TSSOP-16）の刻印を2026-08-13に読み取った（`XPT`ロゴ、`XPT2046`、ロット`ABDEAB`）。**メーカーdatasheetには型番の記載が無いため、刻印が唯一の根拠である** |
| Touch方式 | TBD（`XPT2046`のdatasheetで確認する） |
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
| [Analog Devices ADXL345 Data Sheet](https://www.analog.com/media/en/technical-documentation/data-sheets/adxl345.pdf) | **2026-08-12にブラウザで取得した。Rev. G。**PDF metadataは`/Author: Analog Devices, Inc.`、`/Title: ADXL345 (Rev. G)`、`/Subject: 3-Axis, ±2 g/±4 g/±8 g/±16 g Digital Accelerometer`、作成日2022-05-26。36page。sha256は`87ae2212498c35a6759d8732adee0ec9b9d8d60fa95688bc5904f1f07ceb8ff6`。**これを正の出典とする。****ただしCLIからは取得できない**（下行）。取得したPDFは[machine-profiles.md](../toolchains/machine-profiles.md)に従いrepositoryへ置いていない |
| 同上（CLIからの取得可否） | **curlでは取得できない。**2026-08-10・08-11・08-12に小文字／大文字の両pathをWebFetch／curl／PowerShellで試し、HTTP/2では`stream 0 was not closed cleanly: INTERNAL_ERROR`、`--http1.1`ではbrowser UAを付けても45秒でtimeoutした。**DNSは解決する。****旧記載は「本作業環境から開けない」「egressの問題である」としていたが、ブラウザからは取得できたため誤りである**（2026-08-12訂正。取得した実物のPDFを確認した）。**CLIとブラウザで結果が分かれる理由も、両者の経路が同一かどうかも特定していないため書かない** |
| [Analog Devices ADXL345製品ページ](https://www.analog.com/en/products/adxl345.html) | curlは上行と同じく失敗する。**このURL自体をブラウザで開いたことは確認していない**（`analog.com`のブラウザ経由の取得はdatasheet PDFで確認した）。[hardware-bom.md](hardware-bom.md) `ACCEL-01`が公式文書欄に載せているURLと同じである |
| 秋月 M-06724 商品ページ | **404**（`gM-06724`／`g106724`とも） |
| [Octopartがhostする ADXL345 Data Sheet](https://datasheet.octopart.com/ADXL345BCCZ.-Analog-Devices-datasheet-43345133.pdf) | **2026-08-12にcurlで取得した。Rev. E**（`/Author: Analog Devices, Inc.`、作成日2015-05-28、40page）。**公式Rev. Gとの差分照合に用いた。電気的特性はRev. Gと一致した**（Rev. G の`REVISION HISTORY`によれば`5/2022—Rev. F to Rev. G`の変更はpackage情報と推奨はんだ付けprofileのみである） |
| [SparkFunがhostするADXL345 Data Sheet](https://www.sparkfun.com/datasheets/Sensors/Accelerometer/ADXL345.pdf) | **Rev. 0**（作成日2009-05-29、24page）。2026-08-12に開けた。**Revision 4の時点ではこれが唯一開けた版であり、下表の値の出典であった。現在は公式Rev. Gを正とする**（差は下記のとおり） |
| Mouser mirror（`mouser.com/datasheet/2/609/ADXL345-1517570.pdf`） | **取得できない。**curlはHTTP 200を返すが、実体はPDFではなく`Access to this page has been denied.`のHTMLである（2026-08-12確認） |

**下表のICの値は、公式のRev. Gで確認した範囲である。**page番号はRev. Gのもので、
PDFのpage indexと印字page（`Rev. G | N of 36`）は一致する。

**Rev. 0（Revision 4までの出典）とRev. Gには差がある。**

| 項目 | Rev. 0 | Rev. G |
|---|---|---|
| 絶対最大定格 `VS`／`VDD I/O` | −0.3 V to +3.6 V | **−0.3 V to +3.9 V** |
| `Interface Voltage Range (VDD I/O)` | `VS ≤ 2.5 V`／`VS ≥ 2.5 V`の条件分岐 | 1.7 V–`VS`の単一表記 |
| `Supply Current` | 145 µA（>100 Hz）／40 µA（<10 Hz） | 140 µA（ODR ≥ 100 Hz）／30 µA（ODR < 10 Hz） |
| `Standby Mode Leakage Current` | 0.1 µA typ／2 µA max | 0.1 µA typ（max欄なし） |
| `Device Weight` | 20 mg | 30 mg |

Rev. Gの`REVISION HISTORY`に`4/10—Rev. 0 to Rev. A … Changes to Table 2 and Table 3`があり、
**絶対最大定格はRev. 0→Aで改訂されている。Rev. 0の3.6 Vはsupersededである。**

module boardの値は、これとは別に現物でしか決まらない（[AGENTS.md](../../AGENTS.md) 推測禁止）。

### ICの値（Analog Devices ADXL345）

| 項目 | 値 |
|---|---|
| IC識別情報 | Analog Devices ADXL345 |
| 供給電圧（動作範囲） | `VS` 2.0–3.6 V（typ 2.5 V）。interface用の電源は別系統の`VDD I/O`で1.7 V–`VS`（typ 1.8 V）である。**記号は`VS`と`VDD I/O`である**（pin 6＝`VS` Supply Voltage、pin 1＝`VDD I/O` Digital Interface Supply Voltage）。Table 1、page 4。**これは動作範囲であって絶対最大定格ではない。**両者は別物であり、下行と混同しない |
| 絶対最大定格 | `VS`／`VDD I/O`とも**−0.3 V to +3.9 V**。Digital Pinsは`VDD I/O` + 0.3 Vか3.9 Vの**小さい方**、All Other Pinsは−0.3 V to +3.9 V。Acceleration 10,000 g（Unpowered／Poweredとも）。Output Short-Circuit Duration（Any Pin to Ground）は`Indefinite`。Temperature RangeはPowered／Storageとも−40°C to +105°C。Table 2、page 5。**電流の上限はこの表に記載が無い**（`HW-TBD-025`(b)、Issue [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)。**無いことを2026-08-12に確認した**） |
| 測定range | ±2 g／±4 g／±8 g／±16 g 選択式 |
| Interface（ICの対応） | I2C／SPI（3線式・4線式）の両対応（`SPI (3- and 4-wire) and I2C digital interfaces`） |
| Filter／FIFO機能 | **32段のFIFOを内蔵**（`embedded 32-level FIFO`）。modeは`bypass`／`FIFO`／`stream`／`trigger`の4種で、`FIFO_CTL` registerの`FIFO_MODE` bitsで選ぶ。**各modeの詳細な挙動と、driverでどれを使うかはTBD** |
| Device ID register／value | `DEVID`（address `0x00`、Read Only）。**reset値`11100101`＝`0xE5`**（`The DEVID register holds a fixed device ID code of 0xE5 (345 octal)`）。Table 19 page 23、Register 0x00節 page 24。**実機で読み出して一致を確認するのは[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)の受け入れ条件である** |
| Sensitivity変換 | 全g-rangeのfull resolutionで**256 LSB/g typ**（min 230／max 282）、Scale Factorは**3.9 mg/LSB typ**（min 3.5／max 4.3）。Table 1、page 3。**10-bit固定分解能で使う場合はg-rangeごとに異なる**（同表）。**実測offsetとnoiseの確認は[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)** |
| Output data rate | ICは**0.1–3200 Hz**に対応する（Table 1 page 3）。`BW_RATE` register（address `0x2C`）のrate codeで選び、reset値は`00001010`＝100 Hzである（Table 19 page 23）。data rateごとの消費電流はTable 7 page 13。**DeskCatがどのODRを使うかは設計判断であり[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)で決める** |
| Interrupt pinと動作 | INT1（pin 8）／INT2（pin 9）の2本。**駆動形式はpush-pull固定である**（`Both interrupt pins are push-pull, low impedance pins`、page 19）。**設定で切り替えられない。**polarityは`DATA_FORMAT` register（address `0x31`）の`INT_INVERT` bitで選び、**同registerのreset値が`00000000`であるため既定はactive-highである**（Table 19 page 23、page 27）。**どちらのpinを使うかは[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)で決める**（割り当ては[gpio-assignment.md](gpio-assignment.md)） |
| 起動／reset sequence | 電源投入時は**standby mode**であり、measurement modeへ入るcommandを待つ（Table 6 `Power Sequencing`、page 12）。`VS`がonで`VDD I/O`がoffの状態（`Bus Disabled`）は通信busへ競合を起こすため、**power-up時のこの状態を最短にする**と定めている。**register map（Table 19、page 23）にsoftware reset registerは無い。****bring-up手順の確定は[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)** |
| I2Cアドレスの選択方式 | pin 12 `SDO/ALT ADDRESS`で選ぶ。**7-bit addressは`0x1D`（同pinをhigh）／`0x53`（同pinをGNDへ）**。**ただしどちらになるかはmodule board上の実装で決まる**ため、現物確認まで確定しない（下表`実装済みI2C address`と[tbd-register HW-TBD-004](tbd-register.md)） |
| Calibration要件 | offset補正用に`OFSX`／`OFSY`／`OFSZ` register（address `0x1E`–`0x20`、reset値`00000000`）を持つ。**scale factorは15.6 mg/LSBで、選択したg-rangeに依存しない**（page 24）。手順は`OFFSET CALIBRATION`節（page 30）。**実施要否と実測値は[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)** |

### module boardの値（秋月 M-06724）

**商品ページが404で資料が無いため、現物確認が唯一の根拠である**（[tbd-register HW-TBD-004](tbd-register.md)）。
**2026-08-13に現物を確認し、下表の一部が確定した。**

| 項目 | 値 |
|---|---|
| Module識別情報 | ADXL345モジュール（秋月 M-06724）。**IC刻印は`345B` / `#727` / `750B` / `PHIL`**（2026-08-13読了）。`345B`はAnalog DevicesのADXL345の刻印であり、**ICの根拠がsilkと購入履歴からIC自身の刻印になった** |
| pin列 | 上段 `CS` `Vs` `GND` `VDD`、下段 `INT1` `INT2` `SDO` `SDA` `SCL`（2026-08-13確認）。**`Vs`と`VDD`が別pinとして表記されている。**ただし**これはheaderのsilkラベルであって、board上の配線ではない。**各pinがICの`VS`／`VDD I/O`へ直結しているか、直列抵抗やlevel shiftが入るか、2系統を独立に給電してよいかは、**パターンを追っていないため未解決である**（導通確認またはPCBパターンの追跡が要る） |
| 実装されているinterface（jumper設定） | TBD。**裏面の半田ジャンパ2箇所はいずれもオープンである**（2026-08-13確認）。ただし各ジャンパが何を選ぶかはboard資料が無く、パターンも追っていないため不明 |
| 実装済みI2C address | TBD。**`SDO`はheaderへ出ており、基板上で固定されていない。**したがってaddressは配線時に決まる |
| board上のregulatorの有無 | **非搭載（確定）。**2026-08-13の現物確認で、IC＋`C1`×2＋抵抗のみであり3端子部品が無いことを確認した。**旧記載は根拠資料（秋月 商品ページ。現在404）を失ってTBDへ戻していたが、現物で確定した** |
| moduleへ入れてよい電圧 | TBD。**ICの動作上限3.6 Vも絶対最大定格3.9 Vも、boardの許容値と同一視しない**（regulatorやlevel shiftの有無で変わる）。**[power-budget.md](power-budget.md)と[hardware-bom.md](hardware-bom.md)が置く「3.3 Vで給電する」は、この行が埋まるまで確定しない。****5 V直結の禁止と、この行は別の主張である。**5 V禁止は「moduleがregulatorを持たなければ5 VがICへ直接掛かる」ことを否定できないための**確認前の安全規則**であって、IC定格からmoduleの許容入力電圧を導いたものではない。3.3 Vをmoduleが受け入れる根拠も同様に別に要る（[tbd-register HW-TBD-004](tbd-register.md)） |
| Module搭載pull-up | **`01C`（EIA-96で10 kΩ、1%）が4個**（2026-08-13確認）。ほかに`R2`＝`0`（0 Ωジャンパ）が1個。**どのpinへ付くかはパターンを追っていないため未確定であり、I2Cの実効pull-upを計算する前に配線を確認する**（[gpio-assignment.md](gpio-assignment.md)の競合checklist） |

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
| 供給／logic電圧（動作範囲） | VDD 1.71–3.6 V、VDDIO 1.2–3.6 V。**module上でVDDとVDDIOは接続済み**のため実効は1.71–3.6 V。**これは動作範囲であって絶対最大定格ではない。**両者は別物であり、下行と混同しない |
| 絶対最大定格 | `Voltage at any supply pin`（VDDとVDDIO）は**−0.3 V to +4.25 V**、`Voltage at any interface pin`は−0.3 V to `VDDIO` + 0.3 V。Storage temperature（≤65 % RH）は−45 °C to +85 °C、Pressureは0 to 20,000 hPa、ESDはHBM ±2 kV／CDM ±500 V／Machine model ±200 V。Bosch datasheet Revision 1.24のTable 5、page 13。**電流の上限はこの表に記載が無い**（`HW-TBD-025`(b)、Issue [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)。**無いことを2026-08-12に確認した**）。**この行は2026-08-12に追加した。**それまで[hardware-bom.md](hardware-bom.md)と[tbd-register.md](tbd-register.md)がこの確認結果を引用しながら、datasheet由来値の正であるこの文書に元の記録が無かった |
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

| jumper | 意味 | **現物の実装状態（2026-08-22に実測）** |
|---|---|---|
| `J1` | I2C時の SDA 用プルアップ（R1 4.7 kΩ）の選択 | **開放** |
| `J2` | I2C時の SCL 用プルアップ（R2 4.7 kΩ）の選択 | **開放** |
| `J3` | **I2C時にはんだでジャンパする**（`CSB`→VDD） | **開放** |

**SPI 4線式／3線式で使う場合は`J1`〜`J3`をすべてオープンにする。**

##### 現物の実装状態を実測で確定させた（2026-08-22）

**3つとも開放である。**説明書が示す工場出荷状態と一致する。
**2026-08-13の接写では光学的に判別できず**（パッドが錫めっきで鏡面のため、
半円を分ける溝が半田で埋まっているかを写真で決められなかった）、**導通確認で確定させた。**

**方法**: モジュール単体、**電源を接続しない状態**で、pin間の抵抗を測った
（DT830Bの`Ω`の`20k`レンジ、最小単位10 Ω。`MEAS-03`）。
**判定は抵抗値で行う。**`J1`／`J2`が閉なら4.7 kΩのプルアップが繋がり、`J3`が閉なら`CSB`が`VDD`へ直結する。

| 測定 | 読み | 判定 |
|---|---|---|
| ゼロ点（プローブ同士を接触） | `0` | 測定が有効であることの確認 |
| `VDD`(pin1) ↔ `SDI`(pin4) | **開放** | **`J1`は開放** |
| `VDD`(pin1) ↔ `SCK`(pin6) | **開放** | **`J2`は開放** |
| `VDD`(pin1) ↔ `CSB`(pin3) | **開放** | **`J3`は開放** |
| `VDD`(pin1) ↔ `GND`(pin2) | 開放 | 電源間の短絡が無いことの確認 |
| `SDO`(pin5) ↔ `VDD`(pin1) | 開放 | **`SDO`は基板上でどこにも固定されていない** |
| `SDO`(pin5) ↔ `GND`(pin2) | 開放 | 同上 |

**`SDO`が固定されていないことも実測で裏付けた。**したがってI2Cアドレスは配線時に決まり、
基板側に既定値は無い。

##### この状態から出る帰結（3点）

**(1) 現状のままではI2Cで動かない。**説明書は`J3`を「I2C時にはんだジャンパする」と定めており、
**`J3`が開放なら`CSB`が`VDD`へ繋がらない。**
**したがってI2Cで使うには`J3`のはんだ付けが要る。**これは実装作業であり、`TBD`ではない。

**(2) module搭載の4.7 kΩプルアップは繋がっていない。**`J1`／`J2`が開放のため、
**bus のpull-upはこのmoduleから供給されない。**
[gpio-assignment.md](gpio-assignment.md)の`I2C sensor bus`はbus全体の実効pull-upを扱うため、
**この事実を入力にする必要がある。**同じbusにあるADXL345モジュールは`01C`（10 kΩ）を搭載しており
（`Module搭載pull-up`）、**実効pull-upの計算はそちら側だけを数える形になる。**
`J1`／`J2`をはんだ付けするかは、その計算の結果で決める。**まだ決めていない。**

**(3) アドレスは配線で決める。**`SDO`→GNDで`0x76`、`SDO`→VDDで`0x77`。
**基板側に既定値が無いため、配線しないとアドレスが定まらない。**

##### この記録が主張しないこと

- **`J1`／`J2`をはんだ付けするかを決めていない。**bus全体の実効pull-upの計算が先である。
- **`J3`のはんだ付けをまだ行っていない。**
- **通電していない。**moduleへの給電は一度も行っていない。
- **抵抗値そのものを測っていない。**開放か導通かの判定であり、`4.7 kΩ`の実測値は得ていない。

pin配列: 1=`VDD`, 2=`GND`, 3=`CSB`, 4=`SDI`, 5=`SDO`, 6=`SCK`。

必要なベンチ試験の根拠:

- Device ID
- Raw値と変換値
- Reference instrumentとの比較
- 安定した反復sampling
- Timeout／disconnect動作
- サーボ動作中の動作

## Local decoupling

**この節がdatasheet由来のdecoupling指定の正である**（[tbd-register HW-TBD-029](tbd-register.md)。同行は2026-08-12にcloseした）。
**module搭載分で足りるか外付けが要るかという判断は、ここでは行わない。**
それは[power-budget.md](power-budget.md)の`local decouplingの外付け要否`節にある。

**ICへの指定と、module boardに実装されているものを分けて記録する。**根拠の種類が違う
（ICはdatasheet、module boardはboard資料か現物）。ADXL345のIC値とmodule値を分けたのと同じ扱いである。

| device | datasheetの指定 | 出典 | module boardの実装 |
|---|---|---|---|
| **BME280（IC）** | `C1`／`C2`の推奨値は**100 nF**。`C1`はVDD–GND間、`C2`はVDDIO–GND間 | Bosch Revision 1.24のFigure 17（I2C）／Figure 18（4-wire SPI）／Figure 19（3-wire SPI）のNote。**3つのconnection diagramすべてに同じNoteがある** | **AE-BME280は`C1` 0.1 µF（VDD用）／`C2` 0.1 µF（VDDIO用）を実装済み。**説明書の部品表は「ピンヘッダ以外は、基板にすべて実装済みです」と述べる（[AE-BME280説明書](https://akizukidenshi.com/goodsaffix/AE-BME280_manu_v1.1.pdf) v1.1の◆部品表）。**推奨値100 nFと一致する** |
| **ADXL345（IC）** | `CS` **1 µF tantalum**を`VS`へ、`CI/O` **0.1 µF ceramic**を`VDD I/O`へ、**supply pinの近くに**置くことを推奨する。追加のdecouplingが要る場合は**100 Ω以下**の抵抗またはferrite beadを`VS`と直列に入れるとよい。さらに`VS`のbypassを**10 µF tantalum ∥ 0.1 µF ceramic**へ増やすとノイズが改善しうる。**あわせて`VS`と`VDD I/O`を別電源にすることを推奨し、それができない場合は上記の追加filteringが要るとする。**ADXL345のGNDからpower supply GNDへの接続を**低impedance**に保つことも求めている（`noise transmitted through ground has an effect similar to noise transmitted through VS`） | Rev. Gの`POWER SUPPLY DECOUPLING`、page 29 | **TBD。**M-06724のboard資料が無く、実装済みの部品を現物でしか読めない（[tbd-register HW-TBD-004](tbd-register.md)の確認項目）。**`VS`と`VDD I/O`がboard上で結線されているかも未確認である** |
| **MSP2807（module）** | **記載が無いことを確認した。**[msp2807.pdf](https://akizukidenshi.com/goodsaffix/msp2807.pdf)の`Product Parameters`が持つ電気的仕様は`VCC power voltage`／`Logic IO port voltage`／`Power Consumption`（`TBD`と印字）の3つだけであり、decouplingにもbacklight駆動回路にも触れない | 同上（2026-08-12確認） | **TBD。**現物確認は[tbd-register HW-TBD-002](tbd-register.md)の確認項目であり、`HW-TBD-024`のbacklight回路確認と同じ機会に行える |

**`CS`は1 µFと10 µFのどちらを採るか**（Table 1 page 3の測定条件は`CS = 10 µF tantalum, CI/O = 0.1 µF`
であり、page 29の推奨値1 µFと違う）。

**推奨値の1 µFを採る。**理由は次のとおりである。

- Table 1の`CS = 10 µF`は**datasheetの電気的特性を再現するための測定条件**であって、設計要件として
  提示されたものではない
- page 29自身が10 µF ∥ 0.1 µFを「**ノイズが改善しうる**」追加策として位置づけており、
  1 µFを基本の推奨値としている
- DeskCatが要求するノイズ性能は、tap判定のしきい値が未決である以上まだ定まっていない
  （[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)）。**先に推奨値を採り、
  実機で不足した場合に10 µF ∥ 0.1 µFへ上げる**

**ただしこれはICへの指定に対する選択であって、外付け部品を発注する決定ではない。**
M-06724に何が実装済みかが未確認である以上、外付けの要否そのものが決まっていない
（`HW-TBD-004`）。**ICへの推奨値を、そのまま購入待ちリストへ載せない。**

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
| 2026-08-12 | 5 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)と[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)。**メーカー公式のRev. Gを入手し、Revision 4までの前提2つが誤りであったと判明した。**(a) **`analog.com`へ到達できないという判定が誤りであった。**ブラウザからは取得できる。到達できないのはCLI client（WebFetch／curl）であり、「本作業環境から開けない」「egressの問題である」という従来の記述を撤回した。**原因は特定していないため書いていない。**(b) **Revision 4が記録したRev. 0の絶対最大定格3.6 Vはsupersededであった。**Rev. Gは`VS`／`VDD I/O`とも**−0.3 V to +3.9 V**とし、Rev. Gの`REVISION HISTORY`は`4/10—Rev. 0 to Rev. A`で`Table 2`を改訂したと記す。あわせて`Supply Current`（145/40→140/30 µA）、`VDD I/O`の下限表記、`Device Weight`（20→30 mg）も差がある。**動作範囲3.6 Vと絶対最大定格3.9 Vは別物である**ため、供給電圧の行を動作範囲と絶対最大定格に分けた。**`HW-TBD-025`(b)の答えとして、絶対最大定格に電流の上限が記載されていないことを記録した。**「あるはず」と仮定せずに確認した結果である。あわせてRev. Gから`Device ID`（`0x00 DEVID`＝`0xE5`）、`Sensitivity`（256 LSB/g）、`Output data rate`（0.1–3200 Hz）、`起動／reset sequence`（Table 6。software reset registerは無い）、`Calibration要件`（`OFSX`/`OFSY`/`OFSZ`、15.6 mg/LSB）を埋め、**driver設計判断と実機検証に属する部分は[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)を追跡先として`TBD`のまま残した。**(c) **`Interrupt pinと動作`の駆動形式は、`gpio-assignment.md`が「push-pull／open-drainを設定可能」と書いていたが誤りである。**Rev. Gは`Both interrupt pins are push-pull, low impedance pins`と定める。polarityの既定active-highは正しく、`DATA_FORMAT`のreset値から確定に改めた。**module boardの値（秋月 M-06724）は1欄も変更していない。****ICの3.9 Vをboardの許容入力電圧と同一視しない**（`HW-TBD-004`）。**5 V直結の禁止も変えていない。**3.9 Vであっても5 Vはこれを超える |
| 2026-08-12 | 6 | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)。(a) **`Local decoupling`節を新設した**（`HW-TBD-029`）。この文書はdatasheet由来値の正でありながら、decouplingの記載が1件も無かった。BME280は`C1`／`C2`とも推奨100 nF（Bosch Revision 1.24のFigure 17／18／19のNote）で、**AE-BME280は各0.1 µFを実装済みである**（説明書v1.1の部品表。「ピンヘッダ以外は基板にすべて実装済み」）。ADXL345は`CS` 1 µF tantalum＠`VS`／`CI/O` 0.1 µF ceramic＠`VDD I/O`をsupply pin近傍へ、追加が要れば100 Ω以下の抵抗かferrite beadを`VS`と直列に、さらに`VS`のbypassを10 µF tantalum ∥ 0.1 µF ceramicへ増やす（Rev. G page 29）。**同節が`VS`と`VDD I/O`を別電源にすることも推奨している点もあわせて記録した。**MSP2807は**decouplingの記載が無いことを確認した**（`Product Parameters`の電気的仕様は3項目のみ）。**`CS`は推奨値1 µFを採ることを根拠付きで決めた**（Table 1 page 3の`CS = 10 µF`は測定条件であって設計要件ではなく、page 29自身が10 µFを追加策として位置づけている）。**ICへの指定とmodule boardの実装を分けて書き、外付けの要否はこの文書で判断していない**（判断は[power-budget.md](power-budget.md)の`local decouplingの外付け要否`）。(b) **Environmental sensorへ`絶対最大定格`行を追加した。**`Voltage at any supply pin`は−0.3 V to +4.25 V、`Voltage at any interface pin`は−0.3 V to `VDDIO` + 0.3 Vである（Revision 1.24 Table 5 page 13）。**電流の上限が無いことも記録した。****[hardware-bom.md](hardware-bom.md)と[tbd-register.md](tbd-register.md)は2026-08-12にこの確認結果を引用していたが、datasheet由来値の正であるこの文書に元の記録が無かった。**ADXL345には同じ行があり、BME280だけ欠けていた。あわせて既存の供給／logic電圧の行を`（動作範囲）`と明示し、絶対最大定格と混同しない書き分けをADXL345と揃えた。**module boardの値（秋月 M-06724、AE-BME280）は1欄も変更していない** |
| 2026-08-15 | 7 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)。**現物写真の読み取り結果を反映した。**(a) Touch controllerを**`XPT2046`と確定**した（現物`U2`の刻印。`XPT`ロゴ、ロット`ABDEAB`）。同節の`TBD`は「現物確認待ち」から「`XPT2046`のdatasheetを当てる作業」へ移った。(b) LCD moduleの解像度欄へ、**現物silkが`2.8 TFT SPI 240X320 V1.2`であり`hardware-bom.md`の`240×320`と一致する**ことを追記した。(c) ADXL345のmodule board表を更新し、**IC刻印`345B`によるIC確定**、**regulator非搭載の確定**、**搭載pull-upが`01C`＝10 kΩ×4**であること、`Vs`と`VDD`が別pinに出ていること、裏面の半田ジャンパ2箇所がオープンであること、`SDO`が基板上で固定されておらずaddressが配線時に決まることを記録した。**pull-upがどのpinへ付くかはパターンを追っていないため未確定のままとした** |
| 2026-08-15 | 8 | [PR #122](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/122)のレビュー指摘を反映。(a) `Touch方式`欄に`resistiveと推定`が残っていたため削除した。**同節は「型番が決まったのでdatasheetを当てる段階へ移った」と書きながら推定値を残しており、矛盾していた。**(b) 文書冒頭の状態行が`touch controller型番`を現物確認待ちに挙げたままだったため、`XPT2046`確定を反映した |
| 2026-08-15 | 9 | [PR #122](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/122)のレビュー指摘を反映。ADXL345 module boardの`pin列`欄が「**moduleはICの2系統電源を外部から個別に受ける**」と断定していた。**これはheaderのsilkラベルから導けない。**観測できた事実を「`Vs`と`VDD`が別pinとして表記されている」に限定し、ICへの接続・直列抵抗・level shift・独立給電の可否は導通確認またはPCBパターンの追跡まで未解決とした。**`tbd-register.md`の`HW-TBD-004`は同じ指摘で先に直しており、この文書だけが取り残されていた** |
| 2026-08-22 | 10 | **`HW-TBD-005`の残件を実測で確定させた。**AE-BME280の`J1`／`J2`／`J3`は**3つとも開放**である。2026-08-13の接写では光学的に判別できず（パッドが錫めっきで鏡面のため、半円を分ける溝が半田で埋まっているかを写真で決められなかった）、**モジュール単体・電源を接続しない状態でpin間の抵抗を測って確定させた。****ゼロ点を確認したうえで測った。**説明書が示す工場出荷状態と一致する。**`SDO`が基板上でどこにも固定されていないことも実測で裏付けた。****この状態から帰結が3点出る。**(1) `J3`が開放のため**現状のままではI2Cで動かない**（説明書は`J3`を「I2C時にはんだジャンパする」と定める）。**はんだ付けが要る。**(2) `J1`／`J2`が開放のため**module搭載の4.7 kΩプルアップはbusへ繋がっていない。**実効pull-upの計算にこれを入れない。**はんだ付けするかは計算の結果で決める。まだ決めていない。**(3) アドレスは配線で決める。**通電していない。抵抗値そのものも測っていない**（開放か導通かの判定である）（[#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)） |
