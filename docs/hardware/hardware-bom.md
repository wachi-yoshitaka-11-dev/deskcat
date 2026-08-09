# Hardware Bill of Materials

> 状態: Draft — 正確なmoduleは現物確認が必要
> 正本とする情報: 部品識別情報と一次資料

## 規則

- 正確なメーカー、model、suffix、module boardの識別情報を記録する。
- 候補例を採用済み部品として扱わない。
- メーカーのデータシートとmodule回路図を個別にリンクする。
- deviceの識別情報が`TBD`の間はdriver開発を開始しない。
- 部品識別に使用した現物の表示または写真referenceを記録する。
- 部品を交換する場合は、既存行を黙って変更せず新しいBOM revisionとして記録する。

## 状態ラベル

| 状態 | 意味 |
|---|---|
| `Selected` | 初期DeskCat製作で使用する予定 |
| `Required` | 機能は必要だが、正確な部品は未選定 |
| `Candidate` | 評価中 |
| `Deferred` | 初期MVPには含めない |
| `Not used` | 初期製作の対象外と明示済み |

## 計算機とstorage

| Ref | 機能 | 部品／module | 状態 | 正確なメーカー／suffix | 公式文書 | 電源 | Peak電流 | 根拠／残作業 |
|---|---|---|---|---|---|---|---|---|
| MCU-01 | Real-time I/O controller | ESP-WROOM-32D開発ボード（秋月電子 M-13628。Espressif公式ESP32-DevKitCではない） | Selected | モジュール: ESP-WROOM-32D。基板silkscreen: `ESP32_DevkitC_V4`（Espressif ESP32-DevKitC V4リファレンスデザイン、38pin／wide版） | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g113628/)（データシート添付）、[Espressif ESP32-DevKitC V4公式guide](https://docs.espressif.com/projects/esp-idf/en/v5.1/esp32/hw-reference/esp32/get-started-devkitc.html)（pinout参照用） | USB Micro-B 5 V入力、board上regulatorで3.3V生成。定格はTBD | TBD（現物測定予定） | 現物写真のpin表記（`D0`–`D3`／`CMD`／`CLK`相当がheaderに露出）、購入履歴、および基板裏面silkscreen「`ESP32_DevkitC_V4`」から機種確定。旧記載の「ESP32-DevKitC-32E」は誤りのため訂正。Flash予約pinの露出をGPIO割当時に要注意 |
| SBC-01 | 高水準controller | Raspberry Pi Zero W（ヘッダなし版。ピンヘッダを別途ハンダ付け） | Selected | Board revision: `V1.1`（基板裏面silkscreen「Raspberry Pi Zero W V1.1 © Raspberry Pi 2017」で確認） | [Raspberry Pi公式製品ページ](https://www.raspberrypi.com/products/raspberry-pi-zero-w/) | 5 V入力（micro USB）。connectorと電流予算はTBD | TBD | 購入履歴で品名「RPI-ZERO-W」（`WH`ではない）と基板裏面のrevision表示を確認。写真のheaderは別購入の細pin headerを自分でハンダ付けしたものと推定。旧記載の「WH」は訂正 |
| SD-01 | Piのbootとstorage | Samsung EVO Plus microSDHC 32GB | Selected | Samsung。Speed Class: UHS Speed Class 1（`U1`表示） | メーカー公式ページはTBD | SBCから供給 | TBD | カード本体のprint（`SAMSUNG EVO Plus 32 microSDHC U1`）で確認済み。Piのslotは空だったため、この手持ちcardを新たに使用する。使用前の健全性（health）check未実施 |

## Display、入力、sensor

| Ref | 機能 | 部品／module | 状態 | 正確なIC／module | Interface | Address／mode | 電源／logic | 公式文書 | 残作業 |
|---|---|---|---|---|---|---|---|---|---|
| DISP-01 | LCDの顔 | MSP2807（2.8インチSPI TFT、ILI9341、タッチパネル付） | Selected | ILI9341（LCD controller）。240×320。裏面にmicroSDスロット搭載 | 4-wire SPI | TBD | VCC 3.3–5V、**logic IOは3.3V TTL**。5V給電時の出力levelがメーカー資料で不明なため、安全側に倒して3.3V給電とする（`power-budget.md`参照） | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g116265/)（[datasheet](https://akizukidenshi.com/goodsaffix/msp2807.pdf)）、[LCD Wiki](http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807) | **入手済み（2026-08-08着荷）**。現物のpin配列とtouch controller型番、backlight回路、VCC pin表記を確認して受け入れchecklistを完了させる |
| TOUCH-01 | 撫で操作／選択入力 | MSP2807のタッチパネル部（DISP-01と同一module） | Selected | Touch controller型番は未公開（一般に`XPT2046`系resistive touchとされるが未確認） | 4-wire SPI（LCDと共有、CSは別） | TBD | DISP-01と同一（3.3V給電） | 同上 | DISP-01と一体のため同時購入。**入手済み（2026-08-08着荷）**。撫で動作は連続touch座標(x,y)の変化として検出する想定。Touch controller型番は現物chip刻印で確認する |
| ACCEL-01 | 軽打／持ち上げ／姿勢 | ADXL345モジュール（秋月 M-06724） | Selected | Analog Devices ADXL345。I2C／SPI選択式、tap／double-tap／free-fall検出をhardware内蔵 | I2CまたはSPI（3線式／4線式）、選択式 | Address等はTBD（現物のjumper／pin設定確認要） | VDD 2.0–3.6V（別途VDD_IO）。**M-06724はregulator非搭載のため3.3V直結必須、Logic 5V railへは直結しない**（ESP32 board上の3V3 pinから給電。`power-budget.md`参照） | [ADXL345解説](https://www.digikey.jp/ja/product-highlight/a/analog-devices/adxl345-3-axis-digital-accelerometer) | DeskCatの軽打／持ち上げ判定にtap／free-fall検出hardwareが適合するため採用決定。KXR94-2050（同時購入、秋月）は不採用・spare保管。Interface選択jumperと実装済みaddressは現物確認が必要 |
| ENV-01 | 温度／湿度／気圧 | BME280使用温湿度・気圧センサモジュールキット（秋月 K-09421） | Selected | Bosch BME280 | I2C（最大3.4MHz）またはSPI 3線式／4線式（最大10MHz）、選択式 | 選択jumperの実装状態はTBD（現物確認要） | DC1.71V～3.6V。**5V直結不可**、ESP32 board上の3V3 pinから給電する（`power-budget.md`参照） | 現物付属の製品説明書（写真確認済み） | 説明書記載値を使用。I2C/SPI選択jumperの実装状態と実装済みaddressは現物確認が必要 |
| COLOR-01 | 環境色 | I2C対応デジタルカラーセンサモジュール S11059-02DT（秋月 K-08316） | Deferred | Hamamatsu S11059-02DT | I2C | TBD | TBD | [Hamamatsu公式](https://www.hamamatsu.com/us/en/product/type/S11059-02DT/index.html) | 初期MVPでは選定しない。将来featureで再検討する際の識別情報として記録 |

部品候補は、正確な現物識別と公式文書の確認後に追加する。候補例や類似部品を採用済みとして扱わない。

## 駆動系と電源

| Ref | 機能 | 部品／module | 状態 | 正確なmodel | 定格電圧 | Peak／stall電流 | 公式文書 | 残作業 |
|---|---|---|---|---|---|---|---|---|
| SERVO-01 | 首振り | TowerPro Micro servo 9g SG90 | Selected | TowerPro SG90 | 4.8–6V（データシート値。外部5 V系はproject方針） | データシート値0.5–2A（負荷依存）。実測値はTBD | [TowerPro公式](https://towerpro.com.tw/product/sg90-7/)、[datasheet](https://www.mouser.com/catalog/specsheets/Soldered_101246.pdf) | ラベル現物確認済み。Peak／stall電流と機械的可動域は`power-budget.md`の測定計画に従い実測が必要（[tbd-register HW-TBD-010／011](tbd-register.md)） |
| PSU-PI-01 | Piとlogic電源 | スイッチングACアダプター(USB ACアダプター) MicroBオス 5V3A（秋月 M-12001）— logic／servo共通の単一入力源 | Selected | 秋月 M-12001 | 5V／3A（15W） | TBD（実測でmargin確認） | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g112001/) | この1個を`power-budget.md`の電源rail構成案における単一入力源とし、breadboard上でlogic railとservo railの2本に分岐する（単一入力・内部で分岐、複数ACアダプターは使わない）。分岐後の各railの配線・保護は`power-budget.md`で確定する。手持ちのTA7805S（5V1A×5）はこの構成では不要（adapterが直接5Vを出力するため） |
| PSU-SERVO-01 | サーボ電源 | PSU-PI-01と同一のACアダプターから分岐したservo rail | Selected（入力源は確定。分岐後の部品はTBD） | 入力はPSU-PI-01と共通（M-12001）。Servo直近のbulk capacitorは候補として電解コンデンサ470μF／16V（秋月 g108426、ルビコンWXA）を想定するが、最終容量は実測待ち（`power-budget.md`配線・保護表） | 5V（共通入力からの分岐） | TBD | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g112001/) | Servo起動時の過渡電流を吸収するbulk capacitorの容量選定と、logic railへの影響評価が必要（`power-budget.md`測定計画、[tbd-register HW-TBD-007](tbd-register.md)） |
| PSU-INGRESS-01 | ACアダプターのplugをbreadboardへ引き込む物理変換 | Micro-Bメスreceptacleの2.54mm変換基板（DIP化キット） | Required | 未購入。候補は秋月 g110972（¥130、電源専用、定格1ピン1.5A） | 5V | ingress全体を通る電流。上限は最弱部品定格の80%（候補品なら1.2A） | [候補の商品ページ](https://akizukidenshi.com/catalog/g/g110972/) | **合成給電（段階C）を始める前に必要。**これが無いとアダプターとPiの間にテスターを直列に入れられず、ingressの電流を測れない。到着までは段階A（Pi単体起動。**GPIOへは何も接続しない**）と段階B（ESP32をPCのUSBから給電）に留める（[power-budget.md](power-budget.md)の`5 V ingress`節）。品の確定はservoの実測peakが出てからで、定格不足による買い直しを避ける |
| CABLE-PI-PWR-01 | breadboard railからPiへの給電 | Micro-Bオスcable | Required | 未購入 | 5V | Piの消費電流 | — | **servo試験の直前までは不要**（それまではM-12001を直挿しするため）。PSU-INGRESS-01と同時に購入する |
| CABLE-PI-LINK-01 | Pi–ESP32間のUSB serial link | USB OTG cableまたはMicro-B ⇔ Micro-B OTG変換（PiのUSB OTG port ⇔ ESP32のMicro USB） | Required | 未購入 | — | 案Aを採る場合、このcableがESP32への給電経路も兼ねる | — | **FND-001〜003には不要。**M2（protocol実装）で必要になる。それまでESP32はPCからUSBで給電・flashingする。transportの確定内容は[gpio-assignment.md](gpio-assignment.md)、給電を兼ねるか否かは[power-budget.md](power-budget.md)の`ESP32の給電経路（未決定）`節 |
| PROTO-01 | 試作用配線 | ブレッドボード(秋月 EIC-3901、6穴版)、ミニブレッドボードBB-601(スケルトン)×2、クリップ付コード5色45cm×10本、細いピンヘッダ20P×5(Pi header用に一部使用済み) | Selected | 手持ち品。個別の許容電流はメーカー資料未確認 | 回路に従う | 回路に従う（ジャンパー線・ブレッドボード接点の許容電流は一般に小電流用途。Servo peak電流経路には別途太い線を使う想定） | TBD | 現物写真・購入履歴で確認済み。Servo電源のようなpeak電流が流れる経路は、breadboard接点やジャンパー線でなく、より太い専用線を使うことを`power-budget.md`の配線・保護表で検討する |
| MEAS-01 | 電流波形測定（Oscilloscope代替） | セメント抵抗5W0.1Ω（秋月 g117836、SQP5WJ0R1B、¥30）、および分圧用カーボン抵抗10kΩ | Required | 秋月 SQP5WJ0R1B | — | shuntはservo rail全電流を通す。5W定格に対し0.1Ω×2A²＝0.4Wで余裕あり | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g117836/) | **入手済み（2026-08-08着荷）**。servo rail低側へ挿入する。挿入位置とGND topologyの制約は[power-budget.md](power-budget.md)の`GND topology`節、ADC pinは[gpio-assignment.md](gpio-assignment.md) |

## 購入待ちリスト

**この表が発注時の唯一の参照先である。**部品が必要と判明したら、判明した時点でここへ行を足す。
各部品行の`残作業`列に「未購入」と書くだけで終わらせない。**書いた本人以外には発注時に見えないため、
過去に実際の発注漏れを起こしている**（2026-08-08、下記3点が発注に載らなかった）。

発注は送料がかかるため一度にまとめる。**発注前に、repository全体を
`未購入`／`Required`／`未選定`で機械的に洗い出し、この表と突き合わせる。**
記憶や前回の会話に頼らない。

| 部品 | 用途 | 必要になる時期 | 状態 |
|---|---|---|---|
| `PSU-INGRESS-01` Micro-Bメスreceptacle変換基板 | M-12001のplugをbreadboard railへ引き込む | **合成給電（段階C）を始める前。**これが無いとアダプターとPiの間にテスターを直列に入れられず、ingressの電流を測れない | 未購入。**品の確定はservoの実測peak待ち**（必要な定格が決まらないため） |
| `CABLE-PI-PWR-01` Micro-Bオスcable | breadboard rail → Piの`PWR IN` | 同上 | 未購入 |
| `CABLE-PI-LINK-01` USB OTG変換／cable | Pi ↔ ESP32のUSB serial link | M2（protocol実装）から。ESP32単体のflashingはPCのUSBで足りる | 未購入 |

品の確定に実測が要るものは、**実測できる状態になってから選ぶ**。定格不足による買い直しを避けるためだが、
そのぶん発注が後ろへずれる。ずれる場合は、この表に「何を待っているか」を書く。

## 初期製作の明示的な対象外

| 部品 | 状態 | 理由 |
|---|---|---|
| Arduino Uno | Not used | 初期のreal-time controllerはESP32とする |
| Raspberry Pi Pico | Not used | 初期のreal-time controllerはESP32とする |
| Camera | Not used | Projectのprivacyとscopeに関する決定 |
| Microphone | Not used | Projectのprivacyとscopeに関する決定 |
| Audio output | Not used | 静かなdesktop動作を維持する |
| LCDキャラクターディスプレイモジュール（HD44780系、手持ち） | Not used | 文字専用でgraphic表示不可のため、DISP-01の要件（「顔」としての画像表示）を満たさない |

## 部品受け入れchecklist

各部品を**受け入れ済み**として扱う前に、次を確認する。

**このchecklistは`状態`列の`Selected`のgateではない。**`Selected`は上の状態ラベル定義の
とおり「初期製作で使用する予定」を表すだけであり、部品が特定できた時点で付与する。
このchecklistは、**GPIO割り当てと電源budgetを承認するためのgate**である。

> **現状（2026-08-08時点）**: 下のchecklistは**どの部品についても未完了**である。
> `状態`列の`Selected`は「初期製作で使用する予定」（上の状態ラベル定義）を表しており、
> **受け入れ完了を意味しない**。特に次の点が未了である。
>
> - MSP2807（DISP-01／TOUCH-01）とM-12001（PSU-PI-01／PSU-SERVO-01）は**2026-08-08に着荷したが、
>   現物の表示・pin配列・電流をまだ確認していない**。
> - 全部品でpeak電流が未実測である（`power-budget.md`の測定計画）。
> - ADXL345とBME280のinterface選択jumper、実装済みaddressが現物未確認である。
>
> このchecklistが部品ごとに完了するまで、GPIO割り当てと電源budgetを承認済みとして扱わない。

- [ ] 現物に記載された正確な表示を読んだ
- [ ] Module boardの識別情報が明確である
- [ ] Revision／dateを含む公式データシートへリンクした
- [ ] Module回路図または検証済みpinoutへリンクした
- [ ] 供給電圧とlogic電圧が明確である
- [ ] 通常電流とpeak電流が明確、または測定予定がある
- [ ] Connector方向とpin 1が明確である
- [ ] Bus設定と起動sequenceが明確である
- [ ] 必要なpull-up、capacitor、level shiftingが明確である
- [ ] 入手性と代替リスクを記録した
- [ ] `gpio-assignment.md`と`power-budget.md`を更新した

## Revision履歴

| 日付 | Revision | 変更 | 根拠 |
|---|---|---|---|
| 2026-07-27 | 0 | 初期project方針からinventoryを作成。周辺部品の正確なmodelは引き続きTBD | [DeskCat マイコン開発技術ガイド](../DeskCat_Microcontroller_Development_Guide.md)、ADR-0001 |
| 2026-08-04 | 1 | MCU-01／SBC-01／ENV-01／SERVO-01／COLOR-01の識別情報を確定、ACCEL-01を2候補（ADXL345／KXR94-2050）に絞り込み。旧「ESP32-DevKitC-32E」「Raspberry Pi Zero WH」表記を訂正。DISP-01／TOUCH-01／PSU-PI-01／PSU-SERVO-01は現物未所持のまま | 現物写真＋購入履歴（秋月電子）を、Akizuki商品ページ／TowerPro公式／Hamamatsu公式情報と照合 |
| 2026-08-04 | 2 | ACCEL-01をADXL345に確定（KXR94-2050は不採用・spare）。SD-01をSamsung EVO Plus microSDHC 32GBに確定（Pi挿入済みcardは無く、別の手持ちcardを使用） | カード本体print（Samsung EVO Plus 32 microSDHC U1）の写真、ADXL345採用のユーザー決定 |
| 2026-08-04 | 3 | MCU-01の基板revisionを`ESP32_DevkitC_V4`（Espressif DevKitC V4リファレンスデザイン）、SBC-01のBoard revisionを`V1.1`（2017年製）に確定 | 基板裏面silkscreenの現物確認 |
| 2026-08-04 | 4 | DISP-01／TOUCH-01を秋月MSP2807（ILI9341 SPI TFT＋タッチパネル）に確定（未購入）。手持ちのcharacter LCDモジュールをNot usedとして記録 | ユーザーの購入方針決定、秋月商品ページ |
| 2026-08-05 | 5 | PSU-PI-01／PSU-SERVO-01を秋月 M-12001（5V3A）1個の単一入力源＋breadboard上での2rail分岐に確定。当初提案した「ACアダプター2個を別々に用意する」設計は誤りのため訂正し、`power-budget.md`のTBD wired source構成（単一入力→logic／servoへ分岐）に合わせた | ユーザーからの指摘、`power-budget.md`の電源rail構成案 |
| 2026-08-05 | 6 | PROTO-01を手持ちのbreadboard／mini breadboard／クリップ付コード／ピンヘッダに確定 | 現物写真・購入履歴 |
| 2026-08-05 | 7 | 自己レビューで検出: ACCEL-01(ADXL345, M-06724)とENV-01(BME280)はどちらも定格上限3.6Vのregulator非搭載moduleであるため、`power-budget.md`のLogic 5V railへ直結しないことを明記。ESP32 board上の3.3V出力(3V3 pin)から給電する | Akizuki M-06724商品情報、BME280モジュール付属説明書の電源電圧記載 |
| 2026-08-05 | 8 | `power-budget.md`と`gpio-assignment.md`のレビュー対応で必要と判明した部品4点をBOMへ追加した。いずれも未購入である。`PSU-INGRESS-01`（Micro-Bメスreceptacle変換基板。M-12001のplugをbreadboardへ引き込む手段が無かった）、`CABLE-PI-PWR-01`（Piの`PWR IN`への給電cable）、`CABLE-PI-LINK-01`（Pi–ESP32のUSB serial link用OTG cable。案Aではこれが給電も兼ねる）、`MEAS-01`（電流測定用のshunt抵抗と分圧抵抗） | [PR #55レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55)、[power-budget.md](power-budget.md)の`5 V ingress`節、[gpio-assignment.md](gpio-assignment.md)のtransport節 |
| 2026-08-05 | 9 | 自己レビューで検出: DISP-01／TOUCH-01(MSP2807)の電源欄が「3.3–5V」だけの記載で、logic IOが3.3V TTLである点が抜けていた。5V給電時にmodule出力が5Vになる場合ESP32 GPIOを破損しうるが、level shiftの有無はメーカー資料でも不明なため、安全側に倒して3.3V給電とすることを明記 | [LCD Wiki MSP2807](http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807)の「Logic IO port voltage: 3.3V(TTL)」記載 |
| 2026-08-05 | 10 | 自己レビューで検出: 本文書の規則は「部品受け入れchecklistを確認してから`Selected`にする」と定めているが、Revision 1〜8で未購入・未実測の部品も`Selected`にしており、checklistは全部品で未完了のままだった。`Selected`が受け入れ完了を意味しないこと、および未了項目をchecklist冒頭に明記した | 自己レビュー |
| 2026-08-05 | 11 | 自己レビューで検出: PSU-PI-01が`power-budget.md`の既に存在しない文言「TBD wired source」を引用していたため実際の節名へ修正。PSU-SERVO-01のbulk capacitorを「型番・容量はTBD」としていたが、`power-budget.md`では候補（ルビコンWXA 470μF16V）が確定済みで記述がずれていたため揃えた | 自己レビュー |
| 2026-08-08 | 12 | Revision 8で追加した未購入部品3点（`PSU-INGRESS-01`／`CABLE-PI-PWR-01`／`CABLE-PI-LINK-01`）について、「これが無いと配線を開始できない」という記述が過大だったため訂正した。前2者はservo試験の直前まで不要（それまではM-12001をPiの`PWR IN`へ直挿しする）、`CABLE-PI-LINK-01`はM2のprotocol実装まで不要である。`PSU-INGRESS-01`には候補品（秋月 g110972、定格1ピン1.5 A）を記載した | ユーザーからの指摘（購入済み5点にこれらが含まれていない）、[power-budget.md](power-budget.md)の`5 V ingress`節の段階表 |
| 2026-08-08 | 13 | レビュー指摘2件を反映。(a) 受け入れchecklistのsnapshot日付が`2026-08-05`のままで、同じ節に記載した2026-08-08の着荷と食い違っていたため`2026-08-08時点`へ更新した。(b) Revision履歴でRevision 8と9が重複し、2026-08-08の行が古い行より前に挿入されていた。番号を一意にし、日付順へ並べ直した | [PR #57レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/57) |
| 2026-08-09 | 14 | 昇格PR [#61](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/61)のレビュー指摘を反映。受け入れchecklistの見出しが「各行を`Selected`へ変更する前に確認する」となっており、状態ラベル定義（`Selected`＝「使用する予定」）および「Selectedは受け入れ完了を意味しない」という注記と矛盾していた。この矛盾はrevision 0から存在し、Revision 8の注記追加で表面化した。checklistのgateを`Selected`の付与ではなく**受け入れ（GPIO割り当てと電源budgetの承認）**へ変更した | [PR #61レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/61) |
| 2026-08-09 | 15 | **`購入待ちリスト`節を新設した。**必要と判明した部品を各行の`残作業`列へ「未購入」と書くだけでは発注時に見えず、2026-08-08に実際の発注漏れを起こしている（`PSU-INGRESS-01`とcable 2点）。発注時の唯一の参照先として1箇所へ集約し、発注前にrepository全体を機械的に洗い出して突き合わせる手順も明記した。あわせて[PR #64](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)のレビューで判明した「ingress測定用の2本目のshunt」を同リストへ追加した | 発注漏れの実例、[PR #64レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64) |
| 2026-08-09 | 16 | [PR #64](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)のレビュー指摘を反映。`PSU-INGRESS-01`行と購入待ちリストが、廃止した合成給電手順（Piの5V GPIO pinからrailを作る）を残しており、`power-budget.md`の段階A／B／Cと矛盾していた。両方を段階の規則へ揃えた。あわせて、ingress測定用に一度追加した「2本目のshunt」を購入待ちリストから削除した。ingressの判定量を定常値へ改めた結果、ESP32 ADCによるpeak captureが不要になったためである（負電圧で測定できない問題も同時に解消した） | [PR #64レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64) |
