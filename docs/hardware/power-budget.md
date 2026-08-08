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
├─ Logic/Pi rail 5V（breadboard上で分岐、追加regulatorなし）
│  ├─ Raspberry Pi Zero W
│  ├─ ESP-WROOM-32D開発ボード（秋月 M-13628）
│  │  └─ board上regulatorが3.3Vを生成し、board上の3V3 pinから出力
│  │     ├─ ADXL345（VDD 2.0–3.6V。M-06724はregulator非搭載、3.3V直結が必須）
│  │     ├─ BME280（電源電圧DC1.71～3.6V。5V直結不可）
│  │     └─ MSP2807（LCD＋touch。VCC 3.3–5V対応だがlogic IOは3.3V TTL。
│  │        下記理由により3.3Vで給電する）
└─ Servo rail 5V（breadboard上で分岐、直近にbulk capacitor）
   └─ SERVO-01（TowerPro SG90）

周辺moduleはすべて3.3Vで給電し、5V railへ直結しない。理由は次のとおり。

- ADXL345（M-06724）とBME280は定格上限3.6V。5V直結は定格超過となる。
- MSP2807はVCC 3.3–5V対応だがlogic IOは3.3V TTLである。5Vで給電した場合に
  module側の出力（MISO等）が5Vになるか3.3Vに留まるかは、メーカー資料でも明示されていない。
  5V出力になるとESP32のGPIOが定格超過となり破損しうる。level shiftの有無を現物の
  回路で確認するまでは、安全側に倒して3.3V給電とする。

3V3 railに接続する3moduleの合計消費電流は、ADXL345とBME280が文献値で数mA未満と小さい一方、
**MSP2807のbacklightを含む消費電流は未確認**（負荷表参照）である。加えて
**この基板の3V3 pinが外部へ供給できる電流の定格も未確認**（`hardware-bom.md` MCU-01で
「定格はTBD」）である。`測定計画`で3V3 railの供給能力とMSP2807の実消費電流を測るまで、
この給電構成が成立することを確定としない。MSP2807の実測値が3V3 pinの供給能力を超える場合は、
別途3.3V regulatorを追加する構成へ変更する。

すべての信号domainで、意図的に基準GNDを共有する。
接続前に、USBと外部電源間のbackfeed経路をreviewする。
```

この図はarchitecture案であり、最終配線図ではない。単一のACアダプターを入力源とし、
複数のACアダプターを並列に用意する構成は採用しない（Piの電圧低下riskを避けるための
rail分離は、adapter本体を分けるのではなくbreadboard上のrail分岐とservo直近のbulk capacitorで行う）。

## 負荷表

| Rail | 負荷 | 数量 | Typical電流 | 最大連続電流 | Transient／peak | 根拠 | 確度 |
|---|---|---:|---:|---:|---:|---|---|
| Logic | ESP-WROOM-32D board | 1 | 約80〜100mA（WiFi idle） | 約240mA（WiFi TX時、文献値） | 短時間で最大約500mA相当のspikeが報告例あり | [ESP32技術資料](https://www.espressif.com/sites/default/files/documentation/esp32_datasheet_en.pdf)を含む複数の技術資料（[参考](https://lastminuteengineers.com/esp32-sleep-modes-power-consumption/)） | **文献値。実測前** |
| Logic | Raspberry Pi Zero W | 1 | 約140mA（公式spec） | 実測未定 | Stress時最大約350mAの報告例あり | [Raspberry Pi公式spec](https://www.raspberrypi.com/products/raspberry-pi-zero-w/) | **文献値。実測前** |
| ESP32 3V3出力 | MSP2807（LCD＋backlight＋touch） | 1 | TBD（メーカー未公開） | TBD | TBD | 秋月商品ページに電流記載なし。logic IOが3.3V TTLのため3.3V給電とする（`電源rail構成案`参照）。backlight込みの電流次第では3V3 pinの供給能力を超える可能性があり、その場合は別途3.3V regulatorが必要 | Blocked（未購入、実測必須） |
| ESP32 3V3出力 | ADXL345（accelerometer） | 1 | 数百µA程度（測定mode時） | 無視できるほど小さい想定 | 無視できるほど小さい想定 | [ADXL345解説](https://www.digikey.jp/ja/product-highlight/a/analog-devices/adxl345-3-axis-digital-accelerometer)。M-06724はregulator非搭載のため3.3V直結必須（Logic 5V railへは直結しない） | **文献値。実測前** |
| ESP32 3V3出力 | BME280（environment sensor） | 1 | 数µA〜1mA未満（測定mode時） | 無視できるほど小さい想定 | 無視できるほど小さい想定 | Bosch公式BME280データシート（一般値）。現物付属説明書の電源電圧DC1.71～3.6Vのため5V直結不可（Logic 5V railへは直結しない） | **文献値。実測前** |
| Servo | TowerPro SG90 | 1 | 数十〜数百mA（動作時、負荷依存） | TBD | データシート値0.5〜2A（負荷依存の広い範囲） | [SG90 datasheet](https://www.mouser.com/catalog/specsheets/Soldered_101246.pdf) | **文献値。実測必須（`tbd-register.md` HW-TBD-006）** |

**重要**: 上表の大半は「文献値」であり、この文書の目的である実測値ではない。特にLogic railの同時最大peak
（ESP32 TX spike＋Pi stress＋LCD backlight）とServoのstall電流が重なる最悪caseは、単一電源(M-12001、5V/3A)の
margin不足リスクがある。`測定計画`のESP32＋shunt抵抗によるADC loggingで実際の同時peakを確認するまで、
この表の値を最終受け入れの根拠にしない。

## 容量計算

**rail間の従属関係**: 負荷表の`ESP32 3V3出力` railは独立した電源ではなく、ESP32 board上の
regulatorが`Logic` railの5Vから作っている。したがって入力電源（M-12001）の容量を求めるときは、
3V3 railの負荷も5V rail側の消費に含める。regulatorの変換効率と自己消費があるため、
3V3側で消費した電力に対して5V側の消費はそれ以上になる。3V3 railを独立した予算として扱い、
入力電源の合計から除外しない。

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
| 入力電源 | 電圧、連続電流、peak電流 | スイッチングACアダプター MicroBオス 5V／3A（秋月 M-12001） | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g112001/) | Selected（実測でmargin確認要） |
| Logic regulator／経路 | Pi／ESP32／周辺deviceの要件 | 追加regulatorなし。M-12001の5Vをbreadboard rail経由でそのまま供給するのは**PiとESP32 boardのみ**。周辺module3点（MSP2807、ADXL345、BME280）は5V railへ直結せず、ESP32 board上の3V3 pinから給電する（理由は`電源rail構成案`参照） | `hardware-bom.md` PSU-PI-01、DISP-01、TOUCH-01、ACCEL-01、ENV-01 | Selected（同時peak margin、および3V3 pinの供給能力の実測待ち） |
| Servo regulator／経路 | 正確なservo要件 | 追加regulatorなし。M-12001の5Vをbreadboard上で別railに分岐し、直近にbulk capacitorを配置 | `hardware-bom.md` PSU-SERVO-01 | Selected（bulk capacitor容量は実測待ち） |
| Backfeed防止 | USB／外部電源の共存 | TBD | 回路図review | Blocked |
| Servo bulk capacitor | 測定した過渡電流への対応 | 候補: 電解コンデンサ470μF／16V（秋月、ルビコンWXA、¥10）×2〜3個 | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g108426/)。最終容量はESP32＋shunt抵抗によるADC loggingで確定 | Candidate（実測前） |
| 電流測定用shunt抵抗 | 波形測定の手段（Oscilloscope代替） | セメント抵抗5W0.1Ω（秋月、SQP5WJ0R1B、¥30）×1〜2個。ESP32 ADCで電圧降下をsamplingし、電流波形を近似する | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g117836/) | Selected（Oscilloscope未所持のため、その購入を避けて低costで対応。**低側に挿入すること**。理由は`測定計画`参照） |
| Local decoupling | 各deviceのデータシートに従う | TBD | データシート | Blocked |
| Wire gauge／許容電流 | Peak電流と長さ | TBD | 製品資料／計算 | Blocked |
| Connector定格 | Peak電流と誤接続防止 | TBD | 製品資料 | Blocked |
| 過電流保護 | 故障電流の制限 | TBD | Design review | Blocked |
| 逆極性保護 | 配線リスク | TBD | Design review | Blocked |

## 測定計画

**測定手段**: Oscilloscopeは所有していないため使用しない。既定手段は、セメント抵抗0.1Ω（shunt）を
**GND戻り経路側（低側／low-side）**に挿入し、その両端電圧をESP32のADCで高速sampling（数十kHz程度）して
PCへserial出力する方法（「ESP32＋shunt抵抗によるADC logging」）とする。定常電流はデジタルテスター
（MAS830L）でも確認できるが、サーボ起動時のms単位の過渡変化にはADC loggingを使う。

**ADC入力範囲の制約（必ず守る）**: ESP32のADC入力はおおむね0〜3.3Vであり、これを超える電圧を
直接加えるとpinを破損する。したがって次を守る。

- shuntは必ず低側に置く。高側（5V側）に置くと両端が5V付近になりESP32 ADCで測定できない。
- 5V railそのものの電圧を観測する場合は、直接ADCへ入れず**分圧抵抗で3.3V未満へ落としてから**入力する。
  分圧比と使用する抵抗値は、実施前に記録する。
- 5V系とESP32のGNDは共通化済みである前提に立つ（`確定している制約`参照）。共通GNDでない状態で
  低側shunt測定を行わない。

### サーボ接続前

- [ ] 電源offで導通と想定した絶縁を確認する
- [ ] Connectorの極性を確認する
- [ ] 確認済み部品に適した電流制限を設定する
- [ ] 無負荷の各railを測定する
- [ ] ESP32 board上3V3 pinの外部供給可能電流の定格を確認し、周辺module3点（MSP2807、ADXL345、BME280）を接続した状態で3V3 rail電圧と電流を実測する。3V3 pinの供給能力を超える場合は別途3.3V regulatorを追加する
- [ ] サーボなしでlogicへ給電し、電流を記録する
- [ ] UndervoltageなしでESP32とPiがbootすることを確認する
- [ ] 外部電源とUSB間のbackfeed動作を確認する

### サーボ試験

- [ ] 可能であれば機械負荷を外す
- [ ] サーボ5 VとESP32 3.3 Vを同時にcaptureする（5V側は分圧してからADCへ入れる。上記「ADC入力範囲の制約」に従う）
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

| 日付 | Revision | 変更 | 根拠 |
|---|---|---|---|
| 2026-07-27 | 0 | 初期architectureと測定計画を作成。部品値は引き続きTBD | — |
| 2026-08-05 | 1 | 単一入力源（秋月 M-12001、5V3A）＋breadboard上2rail分岐の構成に確定。負荷表にESP-WROOM-32D／Pi Zero W／ADXL345／BME280／SG90の文献値（実測前の参考値）を記載。Servo bulk capacitor候補（470μF50V×2〜3個）を記載。DISP-01(MSP2807)は未購入のため電流値Blockedのまま | `hardware-bom.md`のRevision履歴4〜6、ESP32／Raspberry Pi公式資料、各部品datasheet |
| 2026-08-05 | 2 | Oscilloscope未所持のため、測定手段をESP32＋shunt抵抗(セメント抵抗0.1Ω)によるADC loggingに変更。GitHub Issue #3の受け入れ条件も同様に修正（Oscilloscopeを必須から任意に変更） | ユーザーが測定機材を所持していないとの指摘 |
| 2026-08-05 | 3 | Servo rail bulk capacitorを、売り切れだった日本ケミコンLXJ 470μF50V(g107766)からルビコンWXA 470μF16V(g108426、¥10、在庫あり)へ変更 | ユーザーからの売り切れ報告 |
| 2026-08-05 | 4 | 自己レビューで検出: ADXL345(M-06724)とBME280は定格上限3.6Vのregulator非搭載moduleであり、Logic railの5Vへ直結すると定格超過となるため、ESP32 board上の3.3V出力(3V3 pin)から給電する構成に訂正（旧版はこの2部品も5V直結としていた）。typo「coost」を修正。Shunt抵抗のlinkをcategoryページから商品ページ(g117836)へ訂正。`gpio-assignment.md`の存在しないRef ID「LCD-01」を「DISP-01」に訂正。「ESP32board」の表記漏れspaceと「秋月M-」の表記揺れ（秋月 M-へ統一）を全fileで修正 | Akizuki M-06724商品情報、`hardware-bom.md`記載のBME280電源電圧(DC1.71～3.6V)、自己レビュー |
| 2026-08-05 | 5 | 自己レビューで検出: `の spike`の余分なspace、Revision4の記述誤り(発生していなかった修正を記載していた)を訂正。配線・保護表の状態列で和文「選定済み」を英語`Selected`に統一（`hardware-bom.md`の状態label語彙に合わせた）。このRevision履歴表の列数不整合（header 3列に対し追加行が4列）をheader側を4列（`根拠`列を追加、`hardware-bom.md`と同形式）に揃えて解消 | 自己レビュー |
| 2026-08-05 | 6 | 自己レビューで検出: 3V3 pinからのsensor給電について「余裕は十分にある」と根拠なく断定していたのを訂正。この基板の3V3 pin外部供給定格は`hardware-bom.md` MCU-01で未確認(TBD)であり、実測するまで構成の成立を確定としない旨に変更し、測定計画へ3V3 rail実測の項目を追加。合計電流の記述(1mA未満)が負荷表の上限値と矛盾していたため数mA未満に訂正 | `hardware-bom.md` MCU-01の「定格はTBD」記載、自己レビュー |
| 2026-08-05 | 7 | 自己レビューで検出: ESP32のADC入力上限(約3.3V)に関する制約が測定計画に欠落していた。shuntを高側に挿入するとESP32 ADCで測定できずpin破損riskがあるため、**低側(GND戻り経路)への挿入**を明記。5V rail電圧を観測する場合は分圧してからADCへ入れる旨と、共通GNDが前提である旨を追記 | ESP32のADC入力範囲、[servo-safety-limits](servo-safety-limits.md#拘束stallと過負荷)の「低側shunt抵抗とESP32 ADC」記載、自己レビュー |
| 2026-08-05 | 8 | 自己レビューで検出: MSP2807をLogic 5V railへ接続する構成にしていたが、同moduleのlogic IOは3.3V TTLであり、5V給電時に出力が5VになるとESP32 GPIOを破損しうる。level shiftの有無はメーカー資料でも不明なため、安全側に倒して3.3V給電へ変更。あわせて、3V3 railの負荷にMSP2807(backlight込みで電流未確認)が加わったため、3V3 pinの供給能力を超える場合は別途3.3V regulatorを追加する旨を明記 | [LCD Wiki MSP2807](http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807)の「Logic IO port voltage: 3.3V(TTL)」記載、自己レビュー |
| 2026-08-05 | 9 | 自己レビューで検出: 容量計算で`ESP32 3V3出力` railを独立電源のように扱うと入力電源の容量を過小評価するため、3V3 railが5V railから作られている従属関係と、5V側の消費に含める旨を明記。あわせてrail構成図のtree枝記号の誤り（同階層に`└─`が2つ）と冗長行を修正し、「3系統合計」という不正確な表現を「3V3 railに接続する3moduleの合計」へ訂正 | 自己レビュー |
| 2026-08-05 | 10 | 自己レビューで検出: Revision 8でMSP2807を3.3V給電へ変更した際、配線・保護表の`Logic regulator／経路`行と測定計画の3V3実測項目が5V給電のままで、文書内に矛盾が残っていた。両者を3.3V給電（周辺module3点）に統一 | 自己レビュー |
