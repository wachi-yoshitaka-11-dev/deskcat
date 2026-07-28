# Sensor and Display Datasheet Notes

> 状態: Blocked — 正確なmoduleの現物確認が必要
> 正本とする情報: Driverに必要な、データシート由来のdevice動作

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

| 項目 | 値 |
|---|---|
| Module識別情報 | TBD |
| Controller IC | TBD |
| 解像度 | TBD |
| Color format／order | TBD |
| Interface | TBD |
| 供給電圧 | TBD |
| Logic電圧 | TBD |
| SPI mode／max clock | TBD |
| Reset polarity／timing | TBD |
| Chip-select timing | TBD |
| Data／command動作 | TBD |
| Backlight回路／電流／polarity | TBD |
| 初期化sequence | TBD |
| 対応orientation command | TBD |
| Readback機能 | TBD |
| 電源投入後に必要なdelay | TBD |
| Driver／library候補 | 識別後までTBD |

必要なベンチ試験の根拠:

- 単色fill
- Color-order pattern
- 四隅の座標pattern
- Rotation
- 全体更新と部分更新の時間
- Touchとserialがactiveな状態での動作

## Touch controller

| 項目 | 値 |
|---|---|
| Module／controller識別情報 | TBD |
| Touch方式 | TBD |
| Interface | TBD |
| 供給／logic電圧 | TBD |
| AddressまたはSPI mode | TBD |
| 検証済み最大bus速度 | TBD |
| IRQ polarity／type | TBD |
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

| 項目 | 値 |
|---|---|
| Module／IC識別情報 | TBD |
| Interface | TBD |
| 供給／logic電圧 | TBD |
| Address／select pin | TBD |
| Device ID register／value | TBD |
| 測定range | TBD |
| Sensitivity変換 | TBD |
| Output data rate | TBD |
| Filter／FIFO機能 | TBD |
| Interrupt pinと動作 | TBD |
| 起動／reset sequence | TBD |
| Module pull-up | TBD |
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

| 項目 | 値 |
|---|---|
| Module／IC識別情報 | TBD |
| 測定量 | TBD |
| Interface | TBD |
| 供給／logic電圧 | TBD |
| Address／select pin | TBD |
| Device ID | TBD |
| 起動時間 | TBD |
| 測定／変換時間 | TBD |
| 測定range | TBD |
| 精度と分解能 | TBD |
| Calibration coefficientの処理 | TBD |
| CRC／data integrity動作 | TBD |
| Heaterがある場合の動作／電流 | TBD |
| 推奨測定rate | TBD |
| Module pull-up | TBD |

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
