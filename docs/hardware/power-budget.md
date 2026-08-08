# Power Budget

> 状態: Blocked — 部品は概ね確定（`hardware-bom.md`参照）。測定済みpeak電流とDISP-01(MSP2807)の現物入手・実測が必要
> 正本とする情報: DeskCatの電源rail、負荷、margin、測定計画

## 確定している制約

- 初期prototypeには有線電源を使用する。
- サーボをESP32 GPIO、3.3 V、ESP32 board regulatorから給電しない。
- 外部のサーボ電源経路を使用する。
- ESP32とサーボ電源のGNDを意図的に共通化する。
- Raspberry Piのundervoltageはresetとstorage破損を引き起こす可能性がある。
- 負荷dataと測定結果が得られるまで、capacitor値と電源定格は`TBD`とする。

## 電源rail構成案

```text
スイッチングACアダプター(USB ACアダプター) MicroBオス 5V3A（秋月 M-12001）— 単一入力源
├─ Logic/Pi rail（breadboard上で分岐、追加regulatorなし）
│  ├─ Raspberry Pi Zero W
│  ├─ ESP-WROOM-32D開発ボード（秋月 M-13628）
│  └─ MSP2807（LCD＋touch）、ADXL345、BME280
└─ Servo rail（breadboard上で分岐、直近にbulk capacitor）
   └─ SERVO-01（TowerPro SG90）

すべての信号domainで、意図的に基準GNDを共有する。
接続前に、USBと外部電源間のbackfeed経路をreviewする。
```

この図はarchitecture案であり、最終配線図ではない。単一のACアダプターを入力源とし、
複数のACアダプターを並列に用意する構成は採用しない（Piの電圧低下riskを避けるための
rail分離は、adapter本体を分けるのではなくbreadboard上のrail分岐とservo直近のbulk capacitorで行う）。

## 負荷表

| Rail | 負荷 | 数量 | Typical電流 | 最大連続電流 | Transient／peak | 根拠 | 確度 |
|---|---|---:|---:|---:|---:|---|---|
| Logic | ESP-WROOM-32D board | 1 | 約80〜100mA（WiFi idle） | 約240mA（WiFi TX時、文献値） | 短時間で最大約500mA相当の spikeが報告例あり | [ESP32技術資料](https://www.espressif.com/sites/default/files/documentation/esp32_datasheet_en.pdf)を含む複数の技術資料（[参考](https://lastminuteengineers.com/esp32-sleep-modes-power-consumption/)） | **文献値。実測前** |
| Logic | Raspberry Pi Zero W | 1 | 約140mA（公式spec） | 実測未定 | Stress時最大約350mAの報告例あり | [Raspberry Pi公式spec](https://www.raspberrypi.com/products/raspberry-pi-zero-w/) | **文献値。実測前** |
| Logic | MSP2807（LCD＋backlight＋touch） | 1 | TBD（メーカー未公開） | TBD | TBD | 秋月商品ページに電流記載なし | Blocked（未購入、実測必須） |
| Logic | ADXL345（accelerometer） | 1 | 数百µA程度（測定mode時） | 無視できるほど小さい想定 | 無視できるほど小さい想定 | [ADXL345解説](https://www.digikey.jp/ja/product-highlight/a/analog-devices/adxl345-3-axis-digital-accelerometer) | **文献値。実測前** |
| Logic | BME280（environment sensor） | 1 | 数µA〜1mA未満（測定mode時） | 無視できるほど小さい想定 | 無視できるほど小さい想定 | Bosch公式BME280データシート（一般値） | **文献値。実測前** |
| Servo | TowerPro SG90 | 1 | 数十〜数百mA（動作時、負荷依存） | TBD | データシート値0.5〜2A（負荷依存の広い範囲） | [SG90 datasheet](https://www.mouser.com/catalog/specsheets/Soldered_101246.pdf) | **文献値。実測必須（`tbd-register.md` HW-TBD-006）** |

**重要**: 上表の大半は「文献値」であり、この文書の目的である実測値ではない。特にLogic railの同時最大peak
（ESP32 TX spike＋Pi stress＋LCD backlight）とServoのstall電流が重なる最悪caseは、単一電源(M-12001、5V/3A)の
margin不足リスクがある。`測定計画`のESP32＋shunt抵抗によるADC loggingで実際の同時peakを確認するまで、
この表の値を最終受け入れの根拠にしない。

## 容量計算

各railについて次を満たす。

```text
required_continuous_current
  >= sum(maximum simultaneous continuous loads) × design margin

required_transient_current
  >= sum(simultaneous credible transient loads)
```

採用したdesign marginとその理由を記録する。不明なpeak値を割合marginで隠さない。

一次近似によるcapacitor見積もり:

```text
ΔV ≈ I × Δt / C
```

最終値では、電源の応答、配線抵抗、capacitor ESR、測定したサーボwaveformを考慮する。

## 配線・保護表

| 項目 | 要件 | 選定値／部品 | 根拠 | 状態 |
|---|---|---|---|---|
| 入力電源 | 電圧、連続電流、peak電流 | スイッチングACアダプター MicroBオス 5V／3A（秋月M-12001） | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g112001/) | 選定済み（実測でmargin確認要） |
| Logic regulator／経路 | Pi／ESP32／周辺deviceの要件 | 追加regulatorなし。M-12001の5Vをbreadboard rail経由でそのまま供給 | `hardware-bom.md` PSU-PI-01 | 選定済み（同時peak margin実測待ち） |
| Servo regulator／経路 | 正確なservo要件 | 追加regulatorなし。M-12001の5Vをbreadboard上で別railに分岐し、直近にbulk capacitorを配置 | `hardware-bom.md` PSU-SERVO-01 | 選定済み（bulk capacitor容量は実測待ち） |
| Backfeed防止 | USB／外部電源の共存 | TBD | 回路図review | Blocked |
| Servo bulk capacitor | 測定した過渡電流への対応 | 候補: 電解コンデンサ470μF／16V（秋月、ルビコンWXA、¥10）×2〜3個 | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g108426/)。最終容量はESP32＋shunt抵抗によるADC loggingで確定 | Candidate（実測前） |
| 電流測定用shunt抵抗 | 波形測定の手段（Oscilloscope代替） | セメント抵抗5W0.1Ω（秋月、¥30程度）×1〜2個。ESP32 ADCで電圧降下をsamplingし、電流波形を近似する | [秋月商品ページ](https://akizukidenshi.com/catalog/c/ccementrg/)で該当型番を確認 | 選定済み（追加購入は不要な既存工具の代わりに低coostで対応） |
| Local decoupling | 各deviceのデータシートに従う | TBD | データシート | Blocked |
| Wire gauge／許容電流 | Peak電流と長さ | TBD | 製品資料／計算 | Blocked |
| Connector定格 | Peak電流と誤接続防止 | TBD | 製品資料 | Blocked |
| 過電流保護 | 故障電流の制限 | TBD | Design review | Blocked |
| 逆極性保護 | 配線リスク | TBD | Design review | Blocked |

## 測定計画

**測定手段**: Oscilloscopeは所有していないため使用しない。既定手段は、電流経路にセメント抵抗0.1Ω（shunt）を挿入し、
その両端電圧をESP32のADCで高速sampling（数十kHz程度）してPCへserial出力する方法（「ESP32＋shunt抵抗によるADC
logging」）とする。定常電流はデジタルテスター（MAS830L）でも確認できるが、サーボ起動時のms単位の過渡変化には
ADC loggingを使う。

### サーボ接続前

- [ ] 電源offで導通と想定した絶縁を確認する
- [ ] Connectorの極性を確認する
- [ ] 確認済み部品に適した電流制限を設定する
- [ ] 無負荷の各railを測定する
- [ ] サーボなしでlogicへ給電し、電流を記録する
- [ ] UndervoltageなしでESP32とPiがbootすることを確認する
- [ ] 外部電源とUSB間のbackfeed動作を確認する

### サーボ試験

- [ ] 可能であれば機械負荷を外す
- [ ] サーボ5 VとESP32 3.3 Vを同時にcaptureする
- [ ] 起動時の電源電流を記録する
- [ ] 小さく低速な動作時の電流を記録する
- [ ] 承認範囲内で想定される最悪動作時の電流を記録する
- [ ] Brownoutまたはreset reasonを記録する
- [ ] LCDとsensor通信を動作させた状態でも反復する
- [ ] 正確なsetupと電流制限が承認されるまでstall testを行わない

## 受け入れ条件

数値制限は引き続き`TBD`とする。承認前に次を定義する。

- Pi入力で許容する最低電圧
- ESP32入力／3.3 Vで許容する最低電圧
- 最大定常ripple
- 最大transient droopと継続時間
- Connector／wireで許容する最大温度上昇
- 許容するbrownout／reset回数: 受け入れ試験では0回
- 電源・connector定格に対する最大電流

## Revision履歴

| 日付 | Revision | 変更 |
|---|---|---|
| 2026-07-27 | 0 | 初期architectureと測定計画を作成。部品値は引き続きTBD |
| 2026-08-05 | 1 | 単一入力源（秋月M-12001、5V3A）＋breadboard上2rail分岐の構成に確定。負荷表にESP-WROOM-32D／Pi Zero W／ADXL345／BME280／SG90の文献値（実測前の参考値）を記載。Servo bulk capacitor候補（470μF50V×2〜3個）を記載。DISP-01(MSP2807)は未購入のため電流値Blockedのまま | `hardware-bom.md`のRevision履歴4〜6、ESP32／Raspberry Pi公式資料、各部品datasheet |
| 2026-08-05 | 2 | Oscilloscope未所持のため、測定手段をESP32＋shunt抵抗(セメント抵抗0.1Ω)によるADC loggingに変更。GitHub Issue FND-003(#3)の受け入れ条件も同様に修正（Oscilloscopeを必須から任意に変更） | ユーザーが測定機材を所持していないとの指摘 |
