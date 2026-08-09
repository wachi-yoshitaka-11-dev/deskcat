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
│  ※plugをbreadboardへ引き込む変換部品が未購入（`5 V ingress`節）
├─ Logic/Pi rail 5V（breadboard上で分岐、追加regulatorなし）
│  ├─ Raspberry Pi Zero W（PWR IN portへ）
│  │  └─ USB OTG port ──[USB cable、Pi link]── ESP-WROOM-32D開発ボード（秋月電子 M-13628）
│  │        ※案Aではこのcableのみで給電する
│  └─ （案Bの場合のみ）ESP-WROOM-32Dの`5V` pinへ直接
│        ESP32の給電経路は案A／案Bのいずれか未決定（`ESP32の給電経路（未決定）`節）
│        board上regulatorが3.3Vを生成し、board上の3V3 pinから出力
│        ├─ ADXL345（VDD 2.0–3.6V。M-06724はregulator非搭載、3.3V直結が必須）
│        ├─ BME280（電源電圧DC1.71～3.6V。5V直結不可）
│        └─ MSP2807（LCD＋touch。VCC 3.3–5V対応だがlogic IOは3.3V TTL。
│              下記理由により3.3Vで給電する）
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

## 5 V ingress（物理的な引き込み経路）

上図の「単一入力源」から`breadboard上で分岐`までの間には、**まだ実在しない物理interface**がある。
M-12001はMicro-Bオスplugであり、breadboardへ直接挿せない。

ただし**この節が埋まるまで作業全体が止まるわけではない**。段階を分ける。

| 段階 | 引き込み方 | 追加部品 | 通せる電流 |
|---|---|---|---|
| bring-up前半（**現在ここ**） | **Piを単体で起動する。**M-12001のplugをPiの`PWR IN`へ直挿しする。**breadboard railを作らず、Piの5V GPIO pinへ何も接続しない** | **不要** | Pi単体のみ。gate不要（下記） |
| 合成給電以降 | 下表の変換基板でMicro-Bを受け、railへ引き出す。Piへは別途Micro-Bオスcableで給電する | Micro-Bメス変換基板、Micro-Bオスcable。**未購入**（`hardware-bom.md`の購入待ちリスト） | ingress定格まで（下記`ingressの電流制限`） |

**servo電流をPiのconnectorとPCB traceへ通してはならない。**servoを繋ぐ前に、必ず下段の構成へ移す。

### 合成給電を変換基板の到着まで行わない理由

当初は「bring-up前半ではPiの5V GPIO pinからbreadboard railを作り、ESP32とLCDとsensorを
合成給電する。合計1.0 Aを超えたら中止する」という段階を置いていた。**これは成立しない。**

そのgateが見るのはPiの`PWR IN`を通る合計電流だが、M-12001はMicro-Bオスplugの直結cableであり、
**アダプターとPiの間に測定器を挿入できない**。挿入するには変換基板が要る。
つまり**合成給電を許可するgateを、合成給電の構成では測れない**。測れないgateはgateではない。

したがって段階を次のように分ける。**gateが要る作業と、要らない作業を混ぜない。**

| 段階 | 内容 | 電流の扱い | 追加購入 |
|---|---|---|---|
| **A** | Piを単体でアダプターから起動する（`PWR IN`へ直挿し、GPIOへ何も繋がない） | gate不要。M-12001は5V/3AでPi用途として定格内であり、Pi単体は通常の使い方である | 不要 |
| **B** | ESP32を単体でPCのUSBから給電し、flashingとADC loggingを行う | gate不要。**PC hostのUSB portが持つ過電流保護（OCP）が制限として働く**。USB 2.0で公称0.5 A、多くのhostは1 A前後でtripする。ESP32単体の文献値（spike約500 mA）はこの範囲に収まる | 不要 |
| **C** | ESP32・LCD・sensorをPiの5V GPIOから合成給電する | **gate必要。変換基板が到着し、ingressで実測できるようになるまで実施しない** | 変換基板とcable |

AとBは通常の使い方であり、測れないgateに依存しない。#8（Pi Rust環境）と
#40（ESP32 toolchain）はここに収まる。Cだけを部品待ちにする。

servo試験以降で用いる構成を次に定める。

| 項目 | 内容 |
|---|---|
| 必要な部品 | Micro-Bメスreceptacleの2.54 mm変換基板（DIP化キット）。M-12001のplugを受け、そこからbreadboard railへ5 V／GNDを引き出す。**未購入**。候補は[秋月 g110972](https://akizukidenshi.com/catalog/g/g110972/)（¥130、電源専用、**定格1ピンあたり1.5 A**） |
| 分岐点 | receptacle変換基板の直後にbreadboard railへ入れ、そこからlogic railとservo railへ分ける。この構成では**Piの5 V GPIO pinを経由して他へ配らない**（下記の定格問題） |
| Piの5 V入力 | breadboard railからMicro-Bオスcableで`PWR IN`へ入れる。PiのUSB OTG portはPi link専用とし、給電に使わない |
| Servo railの5 V入力 | breadboard railから分岐し、直近にbulk capacitorを置く |
| **ESP32の5 V入力** | **未決定。**下記「ESP32の給電経路（未決定）」で選択する |

### ESP32の給電経路（未決定）

Espressifは電源3系統（Micro USB／`5V` pin／`3V3` pin）が**排他**であると明記している。
一方、Pi linkはUSB serialに確定しており（`gpio-assignment.md`のtransport節）、
**Pi linkを繋いだ時点でESP32のUSB VBUSは通電する**。したがって「`5V` pinから給電しつつ
USBでPiと繋ぐ」構成は、そのままでは排他制約に反する。次のどちらかを選ぶ。

| 案 | 内容 | 未解決の点 |
|---|---|---|
| A: USB VBUS単独給電 | ESP32はPiからのUSB cableだけで給電し、`5V` pinへは何も接続しない。配線が最も単純で排他制約にも反しない | PiのUSB OTG portが、ESP32のpeak（文献値で最大約500 mA spike）とMSP2807の3V3負荷を合わせた電流を供給できるか。Pi自身の入力電流もその分増える |
| B: `5V` pin給電＋USBはdata用 | ESP32を`5V` pinから給電し、USBはPi linkにのみ使う | VBUSと`5V` pinが同時に生きる。公式ESP32-DevKitC V4はVBUS保護のSchottky diodeを実装しているが、**この秋月基板が同じ保護を持つかは未確認**。逆流の有無を現物回路で確認するまで承認しない |

案Aを既定候補とする。PiのUSB port供給能力の実測（`測定計画`）で不足が判明した場合は
案Bへ切り替え、そのとき秋月基板のVBUS保護有無を回路で確認する。

**案Aと案Bはどちらも段階C（合成給電）の話であり、変換基板が到着するまで実施しない。**
それまでのESP32への通電は、段階Bの経路（PCのUSBから給電）で行う。3つを混同しない。

| 経路 | いつ | 電流制限 | 排他制約 |
|---|---|---|---|
| **段階B: PC → ESP32のMicro USB** | いま可能 | **PC hostのUSB portのOCP。**USB 2.0の公称は0.5 A、多くのhostは1 A前後でtripする。ESP32単体の文献値（spike約500 mA）はこの範囲に収まる | 満たす（USBのみ。`5V` pinへ何も繋がない） |
| 案A: Pi → ESP32のMicro USB | 段階Cから | **PiのUSB OTG portの供給能力。**値は未確認で、ingressでの実測とあわせて確かめる。実測するまで案Aを常用しない | 満たす（USBのみ） |
| 案B: breadboard rail → ESP32の`5V` pin | 段階Cから、かつ回路確認後 | ingressの上限（`ingressの電流制限`） | **満たさない恐れがある。**USBを繋ぐとVBUSと`5V` pinが同時に生きる |

**案Bは、秋月基板のVBUS保護diodeの有無を回路で確認するまで通電しない。**
確認前に`5V` pin給電とUSB接続を同時に行うと、保護が無い場合に逆流経路ができる。

段階Cでまず案Aを試し、PiのUSB port供給能力が不足した場合にだけ案Bへ進む。
その時点で回路確認を行う。いずれの通電も、人が電源を落とせる状態で監視して行う。

### ingressの電流制限（未解決）

**上限は経路上の全部品の定格のうち最小値で決まる。**connectorの一般定格ではない。
候補構成では変換基板の1ピンあたり1.5 Aが最小であり、Micro-B connectorの一般定格
（約1.8 A）を上限に使うと、1.5〜1.8 Aの測定値が「合格」になってしまう。

**derating: 最弱部品の定格の80%を上限とする。**候補構成なら`1.5 A × 0.8 = 1.2 A`である。

**評価するのは定常値（平均電流）であり、過渡peakではない。**connectorとPCB traceの
電流定格は`I²R`による発熱と放熱の釣り合いで決まる熱の制限であり、時定数は秒から分の
order である。µsからmsのspikeは発熱にほとんど寄与しないため、peakでこの上限を
評価すると過剰に厳しくなる。

**したがってingressの電流はデジタルテスター（MAS830L）を直列に入れて定常値で測る。**
ESP32 ADCによるpeak captureは、この上限の判定には使わない。

peakが問題になるのは別の現象であり、別の量で見る。

| 現象 | 見る量 | 測定点 |
|---|---|---|
| connectorとtraceの過熱 | **定常／平均電流** | ingressへテスターを直列 |
| peakによる電圧降下（Piのbrownout） | **電圧droop** | `ADC-5V`、`ADC-3V3`（`gpio-assignment.md`） |
| servoのstall電流 | peak電流 | `ADC-SHUNT`（同上） |

ingress低側へshuntを入れてESP32 ADCで測る案は採らない。star pointを基準にすると
adapter return側は`I × R`だけ**負**の電位になり、ESP32のADCでは測れない
（pin破損のriskもある）。high-side current sense ICで解決できるが、
上記のとおりこの上限の判定にpeakが要らないため、部品を追加しない。

80%とする理由は次のとおり。定格は連続通電の条件下で決まる値であり、本projectには
定格側にも測定側にも不確かさが残る。

- servoの連続動作電流は負荷依存で幅がある
- テスターの読みは平均値であり、動作条件によって変わる
- breadboard接点と細線の接触抵抗が加わり、部品単体の定格より条件が悪い

**判定は定常電流で行う。**負荷表の文献値でも、連続側の合計（ESP32 240 mA＋Pi 350 mA
＋LCD未確認＋servoの連続動作分）は候補構成の上限1.2 Aに迫る。したがって次のいずれかが要る。

- 実測でservoを含む**定常電流の合計**が**最弱部品定格の80%以下**に収まることを示す。
  収まらない場合は`servo-safety-limits.md`のtrajectory制限（可動域、速度、duty cycle）で
  servoの連続動作電流を下げ、再度実測する
- または、servo railのingressをMicro-Bを経由しない、より定格の高いconnectorへ変更する。
  その場合も上限は同じ規則（新しい最弱部品の80%）で決め直す

**peakをこの判定に使わない。**peakは`ADC-5V`／`ADC-3V3`による電圧droopと、
brownout・resetの有無を見るために使う。connector定格の判定には用いない。

### 変換基板に必要な定格の見積もり

ingressを実測できるのは段階C（変換基板が要る）である。したがって購入前の見積もりは、
**実測できるbranchは実測し、実測できないbranchは根拠のある保守的な上限を置いて**足す。
定常電流はこの足し方をしてよい（peakと違い、時間的に重なる前提を置く必要がない）。
足す値は、**同時に継続しうる動作条件での定常電流**とする。

| branch | 見積もりの根拠 | 値 |
|---|---|---|
| ESP32 board | 文献値。Wi-Fi TXを継続した場合 | 240 mA |
| Raspberry Pi Zero W | 公式specのstress時。上限側を採る | 350 mA |
| MSP2807（LCD＋backlight＋touch） | **未確認。段階B（ESP32をPCのUSBから給電）で実測できる。ここが唯一の未知数である** | **実測待ち** |
| SERVO-01（SG90） | **実測できない**（servoへ5 Vを供給する経路がingressしかないため）。連続動作時の電流として広く示される上限値`250 mA`を保守的な上限として置く。stall電流は採らない（connector定格は熱の制限であり、stallは過渡のため） | 250 mA |

servoの250 mAは**割り当てた予算**として扱う。実機がこれを継続して超えないことは、
`servo-safety-limits.md`のtrajectory制限（可動域、速度、duty cycle）で担保する。
実測でこれを超えた場合は、制限を締めるか、ingressの定格を上げるかを選ぶ。

未知数はMSP2807の1つだけである。段階Bで測れば合計が確定し、**変換基板の品を
ingressの実測を待たずに選べる**。選定後の実測で見積もりを超えていた場合は、
その時点で定格を見直す。

この制約は`受け入れ条件`にも数値として記載している。

## 負荷表

| Rail | 負荷 | 数量 | Typical電流 | 最大連続電流 | Transient／peak | 根拠 | 確度 |
|---|---|---:|---:|---:|---:|---|---|
| Logic | ESP-WROOM-32D board | 1 | 約80〜100mA（WiFi idle） | 約240mA（WiFi TX時、文献値） | 短時間で最大約500mA相当のspikeが報告例あり | [ESP32技術資料](https://www.espressif.com/sites/default/files/documentation/esp32_datasheet_en.pdf)を含む複数の技術資料（[参考](https://lastminuteengineers.com/esp32-sleep-modes-power-consumption/)） | **文献値。実測前** |
| Logic | Raspberry Pi Zero W | 1 | 約140mA（公式spec） | 実測未定 | Stress時最大約350mAの報告例あり | [Raspberry Pi公式spec](https://www.raspberrypi.com/products/raspberry-pi-zero-w/) | **文献値。実測前** |
| ESP32 3V3出力 | MSP2807（LCD＋backlight＋touch） | 1 | TBD（メーカー未公開） | TBD | TBD | 秋月商品ページに電流記載なし。logic IOが3.3V TTLのため3.3V給電とする（`電源rail構成案`参照）。backlight込みの電流次第では3V3 pinの供給能力を超える可能性があり、その場合は別途3.3V regulatorが必要 | Blocked（**入手済み・実測未実施**） |
| ESP32 3V3出力 | ADXL345（accelerometer） | 1 | 数百µA程度（測定mode時） | 無視できるほど小さい想定 | 無視できるほど小さい想定 | [ADXL345解説](https://www.digikey.jp/ja/product-highlight/a/analog-devices/adxl345-3-axis-digital-accelerometer)。M-06724はregulator非搭載のため3.3V直結必須（Logic 5V railへは直結しない） | **文献値。実測前** |
| ESP32 3V3出力 | BME280（environment sensor） | 1 | 数µA〜1mA未満（測定mode時） | 無視できるほど小さい想定 | 無視できるほど小さい想定 | Bosch公式BME280データシート（一般値）。現物付属説明書の電源電圧DC1.71～3.6Vのため5V直結不可（Logic 5V railへは直結しない） | **文献値。実測前** |
| Servo | TowerPro SG90 | 1 | 数十〜数百mA（動作時、負荷依存） | TBD | データシート値0.5〜2A（負荷依存の広い範囲） | [SG90 datasheet](https://www.mouser.com/catalog/specsheets/Soldered_101246.pdf) | **文献値。実測必須（`tbd-register.md` HW-TBD-010／011。model自体はHW-TBD-006で解決済み）** |

**重要**: 上表の大半は「文献値」であり、この文書の目的である実測値ではない。特にLogic railの同時最大peak
（ESP32 TX spike＋Pi stress＋LCD backlight）とServoのstall電流が重なる最悪caseは、単一電源(M-12001、5V/3A)の
margin不足リスクがある。実測するまで、この表の値を最終受け入れの根拠にしない。

**量ごとに見る対象が違う。**connector定格の判定は**定常電流**（テスター直列、
`ingressの電流制限`）、servoのpeakは`ADC-SHUNT`、peakによる電圧降下は
`ADC-5V`／`ADC-3V3`である。混同しない。

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
| **5 V ingress interface** | Micro-Bオスplugを受け、breadboard railへ5 V／GNDを引き出す物理変換 | **未購入**。候補はMicro-Bメスreceptacleの2.54 mm変換基板（秋月 g110972、定格1ピン1.5 A）。bring-up前半はM-12001をPiの`PWR IN`へ直挿しして代用する | `5 V ingress`節の段階表 | **Blocked（servo試験までに購入・実装が必要。前半はPi直挿しで進行可）** |
| ESP32の5 V入力経路 | 3系統（Micro USB／5V pin／3V3 pin）の排他制約を守る | **未決定。**案A（PiからのUSB VBUS単独給電、既定候補）と案B（`5V` pin給電＋USBはdata用）のいずれか | Espressif ESP32-DevKitC V4文書（3系統は排他）。`ESP32の給電経路（未決定）`節 | **Blocked**（案AはPiのUSB port供給能力が未実測、案Bはこの秋月基板のVBUS保護diode有無が未確認） |
| Piの5 V入力経路 | PWR IN portから給電し、USB OTG portはPi link専用とする | 段階C以降: breadboard railからMicro-Bオスcableで`PWR IN`へ。段階A（Pi単体起動）: M-12001を`PWR IN`へ直挿しし、**GPIOへは何も接続しない** | Raspberry Pi Zero W公式回路（PWR INはdata線未接続の給電専用） | Selected（cable未購入。段階Aは合成給電ではないため電流gateの対象外） |
| Logic regulator／経路 | Pi／ESP32／周辺deviceの要件 | 追加regulatorなし。M-12001の5Vをbreadboard rail経由でそのまま供給するのは**Piのみ確定**（ESP32は上行のとおり給電経路が未決定）。周辺module3点（MSP2807、ADXL345、BME280）は5V railへ直結せず、ESP32 board上の3V3 pinから給電する（理由は`電源rail構成案`参照） | `hardware-bom.md` PSU-PI-01、DISP-01、TOUCH-01、ACCEL-01、ENV-01 | Blocked（ESP32の給電経路が未決定。加えて同時peak marginと3V3 pinの供給能力が未実測） |
| Servo regulator／経路 | 正確なservo要件 | 追加regulatorなし。M-12001の5Vをbreadboard上で別railに分岐し、直近にbulk capacitorを配置 | `hardware-bom.md` PSU-SERVO-01 | Selected（bulk capacitor容量は実測待ち） |
| Backfeed防止 | USB／外部電源の共存 | TBD | 回路図review | Blocked |
| Servo bulk capacitor | 測定した過渡電流への対応 | 候補: 電解コンデンサ470μF／16V（秋月、ルビコンWXA、¥10）×2〜3個 | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g108426/)。最終容量はESP32＋shunt抵抗によるADC loggingで確定 | Candidate（実測前） |
| 電流測定用shunt抵抗 | 波形測定の手段（Oscilloscope代替） | セメント抵抗5W0.1Ω（秋月、SQP5WJ0R1B、¥30）×1〜2個。ESP32 ADCで電圧降下をsamplingし、電流波形を近似する | [秋月商品ページ](https://akizukidenshi.com/catalog/g/g117836/) | Selected（Oscilloscope未所持のため、その購入を避けて低costで対応。**低側に挿入すること**。理由は`測定計画`参照） |
| Local decoupling | 各deviceのデータシートに従う | TBD | データシート | Blocked |
| Wire gauge／許容電流 | Peak電流と長さ | TBD | 製品資料／計算 | Blocked |
| Connector定格 | **定常電流**と誤接続防止 | 上限は**経路上の最弱部品の定格の80%**。候補構成では変換基板の1ピン1.5 Aが最小のため1.2 A。判定はpeakではなく定常電流で行う（定格は熱の制限のため） | `5 V ingress`節の`ingressの電流制限` | **Blocked（実測で定常電流が上限内に収まることを示すか、servo railのingressを高定格connectorへ変更するまで承認しない）** |
| 過電流保護 | 故障電流の制限 | TBD | Design review | Blocked |
| 逆極性保護 | 配線リスク | TBD | Design review | Blocked |

## 測定計画

**測定手段**: Oscilloscopeは所有していないため使用しない。既定手段は、セメント抵抗0.1Ω（shunt）を
**servo railのGND戻り経路（低側／low-side）**に挿入し、その両端電圧をESP32のADCでsamplingする方法
（「ESP32＋shunt抵抗によるADC logging」）とする。定常電流はデジタルテスター（MAS830L）でも確認できるが、
サーボ起動時のms単位の過渡変化にはADC loggingを使う。使用するADC pinは`gpio-assignment.md`で予約済み
（`ADC-SHUNT`＝GPIO32、`ADC-5V`＝GPIO33、`ADC-3V3`＝GPIO36。すべてADC1）。

### GND topology（測定前に必ず確定させる）

低側shuntをGND戻り経路に入れると、shunt両端に電位差が生じる。この電位差がESP32のGND基準を
動かすか否かは、**どこにshuntを入れるかで決まる**。曖昧なまま測ると、ESP32側のGNDが浮いて
全ADC値がずれるか、あるいはservo電流がshuntを迂回して過小評価になる。次の一つに固定する。

```text
                        ┌─ Pi GND
                        ├─ ESP32 GND          ← logic戻り。shuntを通らない
                        └─ 周辺module GND
                        │
5V ingress GND ─────────┴── star point ──┬──[ shunt 0.1Ω ]── servo GND 端子
 (adapterのreturn。        （共通GND基準。 │                        ▲
  star pointと同一node）    ESP32 ADCの    │            servo戻りだけが通る
                            0 V基準）      │
                                           └ ADC-SHUNT(GPIO32)は
                                             servo GND端子側をこのnode基準で測る
```

- **shuntはservo戻り専用**とし、star pointとservo GND端子の間だけに入れる。
- **ESP32のGNDはstar pointへ直結**し、shuntを経由させない。これによりservo電流はESP32のGND基準を動かさない。
- ADCが測るのは、**servo GND端子側のnode（GPIO32）を、ESP32 GND＝star pointを基準とした単端測定**である。
  この電位差がそのまま `I_servo × 0.1Ω` になる。差動amplifierは使わない。
- 測定される極性が正になるよう、shuntの向き（どちら側をstar pointに繋ぐか）を配線時に記録する。
- **許容するGND offset**: shunt両端の電位差は最大で `2 A × 0.1Ω = 0.2 V`。この0.2 Vは
  **servo側にのみ現れ、logic側には現れない**構成であることを、電源off時の導通checkで確認する。
  logic GNDとservo GNDの間に0.1Ωが入ってしまっている配線は不可とする。

この構成が取れない場合（star pointを作れない、shuntを迂回する戻り経路が残る等）は、
**測定を実施しない**。誤ったtopologyでの実測値は、電源承認の根拠にしない。

### Sample rateとlog形式

`115200 8N1`のUSB serialは実効約11.5 kB/sであり、16 bit sampleを連続で流すと
framingを除いても約5.7 kSample/s、text encodeではさらに落ちる。**したがって連続streamingはしない。**

| 項目 | 方式 |
|---|---|
| 取得方式 | **burst capture**。ESP32のRAMへ一定期間ぶんを溜め、取得停止後にserialでdumpする。sample rateをserial帯域から切り離す |
| 必要な時間分解能 | servo起動の過渡はms order。これを追うため**1 kSample/s以上**を必須要件とする |
| 目標sample rate | 5〜10 kSample/s（ADC1・oneshot loop、Wi-Fi停止時）。**確定値は実機で測って実験記録へ残す**。sample rate自体は測定treatmentのparameterであり、`HW-TBD-014`が決めるlinkのbaudとは別のfieldである |
| dumpに使うbaud | **`HW-TBD-014`／`PROTO-TBD-001`で決めるlinkのbaudをそのまま使う。**測定用に別のbaudを勝手に選ばない。Pi–ESP32間のUSB serialは1本しかなく、Protocolがそのbaudを共有transport parameterとして持つ（[protocol](../protocol/esp32-pi-protocol.md)の§2）。別baudを使うなら、Protocolを停止する測定modeと両端の再設定手順、通常channelとの排他を先に定義する必要がある |
| Wi-Fiの扱い | capture中はWi-Fiを停止する。Wi-Fi動作中はADC sampling rateが大きく落ちる。ADC2はWi-Fi有効時に使用不可のため、そもそも測定へ割り当てていない |
| buffer長 | 1回のcaptureで最低200 ms（servo起動の突入を含む長さ）。必要RAMは`sample rate × 2 byte × 0.2 s`で見積もる |
| log形式 | dump時はCSV（`時刻[us],生ADC値`）とし、生値のまま出す。電流への換算は事後にPC側で行い、換算式と分圧比を実験記録へ残す |
| CSVを出してよい条件 | **Protocolが動作していない専用の測定modeでのみ出力する。**[Protocol](../protocol/esp32-pi-protocol.md)は「channelから送信するすべてのbyteは有効にframe化されたmessageの一部でなければならない」と定めており、生CSVをJSON Lines channelへ混ぜるとこれに反してPi側のparseを壊す。測定firmwareはProtocol sessionを確立せず、同じlinkでJSON LinesとCSVを同時に流さない。Protocol稼働中に採取したい場合はsampleをProtocol messageへ載せる必要があり、それはProtocol側の変更であってこの文書の範囲外である |

上表の「目標sample rate」は未実測の設計目標であり、受け入れの根拠にしない。

**実測が1 kSample/s未満だった場合、buffer長の短縮で代替しない。**buffer長は取得できる
「期間」を決めるだけで、sample間隔を詰めない。短くしても取り逃がしたpeakは戻らない。
その場合は次のいずれかを取る。

- ADC側を改善して1 kSample/s以上を達成する（Wi-Fi停止、oneshotからDMA continuousへ、
  channel数の削減など）。改善後に再測定する
- または、**必須要件そのものを見直す**。servo起動の過渡が実際には何msなのかを
  別手段（テスターの応答では足りないため、より低速な現象へ分解する等）で確認し、
  新しい根拠を添えて要件を書き換える

いずれも取れない場合は、**この測定手段では電源承認の根拠を作れない**と結論し、
測定機材の追加を検討する。要件を満たさない測定値を、満たしたものとして扱わない。
**linkのbaudを測定の都合で上げる場合は、`HW-TBD-014`／`PROTO-TBD-001`の決定そのものを
変える扱いとし、Protocol側と揃えて確定させる。**片側だけ変更しない。

**ADC入力範囲の制約（必ず守る）**: ESP32のADC入力はおおむね0〜3.3Vであり、これを超える電圧を
直接加えるとpinを破損する。したがって次を守る。

- shuntは必ず低側に置く。高側（5V側）に置くと両端が5V付近になりESP32 ADCで測定できない。
- **低電流側の精度に注意する。**0.1Ωのshuntでは、電流0.5 Aで50 mV、0.1 Aで10 mVにしかならない。
  ESP32のADCは入力0付近の直線性が悪く、減衰0 dBでも実用域はおおむね100 mV以上である。
  したがってこの構成が素直に測れるのは**約1 A以上**であり、typical電流帯（数十〜数百mA）の
  絶対値精度は期待しない。servo起動時のpeakとその継続時間を捉えることを主目的とし、
  小電流の定常値はデジタルテスター（MAS830L）側で確認する。
- 5V railそのものの電圧を観測する場合は、直接ADCへ入れず**分圧抵抗で3.3V未満へ落としてから**入力する。
  分圧比は1/2（10 kΩ／10 kΩ）を既定とし、`gpio-assignment.md`と一致させる。実施時に実測した抵抗値を記録する。
- 5V系とESP32のGNDは共通化済みである前提に立つ（`確定している制約`参照）。共通GNDでない状態で
  低側shunt測定を行わない。

### サーボ接続前

**この段階は変換基板（`PSU-INGRESS-01`）が到着してから実施する。**合成給電の電流を
ingressで測る必要があり、それが無いとアダプターとPiの間に測定器を挿入できない
（`合成給電を変換基板の到着まで行わない理由`）。**servoはまだ繋がない。**

変換基板の到着前にできるのは、`5 V ingress`節の段階Aと段階B（Pi単体の起動、
ESP32のPC USBからのflashing）だけである。これらは合成給電ではないため、
下記の測定を必要としない。

- [ ] 電源offで導通と想定した絶縁を確認する
- [ ] Connectorの極性を確認する
- [ ] 確認済み部品に適した電流制限を設定する
- [ ] 無負荷の各railを測定する
- [ ] **ingressを通る合計電流を段階的に測る。**変換基板の直後（アダプター側）に測定点を置き、
      各段階で記録する。**`ingressの電流制限`の上限（最弱部品定格の80%）を超えた時点で中止**する
  - [ ] 段階1: Piのみ（ESP32・周辺module未接続）
  - [ ] 段階2: ＋ESP32（Wi-Fi停止、idle）
  - [ ] 段階3: ＋ESP32のWi-Fi TXを動作させた状態
  - [ ] 段階4: ＋MSP2807（backlight点灯を含む）
  - [ ] 段階5: ＋ADXL345、BME280
- [ ] 各段階で**ingressの定常電流**をデジタルテスター（MAS830L）で記録する。
      これが`ingressの電流制限`の判定量である（connector定格は熱の制限のため）
- [ ] 各段階で**5 Vと3.3 Vの電圧droop**を`ADC-5V`／`ADC-3V3`で記録する。
      peakによる電圧降下はここで捉える。Piのbrownoutとreset reasonもあわせて確認する
- [ ] ESP32 board上3V3 pinの外部供給可能電流の定格を確認し、周辺module3点（MSP2807、ADXL345、BME280）を接続した状態で3V3 rail電圧と電流を実測する。3V3 pinの供給能力を超える場合は別途3.3V regulatorを追加する
- [ ] **ESP32の給電経路を確定させる**（`ESP32の給電経路（未決定）`節）。案A: PiのUSB OTG portからESP32＋3V3負荷を給電したときの電流とESP32入力電圧を実測し、undervoltageもPi側のbrownoutも起きないことを確認する。不足する場合は案Bへ切り替え、そのとき秋月基板のVBUS保護diodeの有無を回路で確認してから`5V` pinとUSBを同時接続する
- [ ] サーボなしでlogicへ給電し、電流を記録する（上記の段階測定の結果をそのまま用いる）
- [ ] UndervoltageなしでESP32とPiがbootすることを確認する
- [ ] 外部電源とUSB間のbackfeed動作を確認する

### サーボ試験

**この段階に入る前に、`5 V ingress`節の下段構成（Micro-Bメス変換基板＋Piへの給電cable）へ移す。**
Pi直挿しのままservoを繋ぐと、servo電流がPiのconnectorとPCB traceを通る。

- [ ] ingressを変換基板経由へ移し、Piの5 V GPIO pinから他へ配電していないことを確認する
- [ ] 可能であれば機械負荷を外す
- [ ] `GND topology`節のstar point構成が実配線で成立していることを、電源off時の導通checkで確認する（logic GNDとservo GNDの間にshuntが入っていないこと）
- [ ] ADC loggingが必須要件の1 kSample/s以上を満たすことを確認し、実測sample rateを記録する
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

次は数値制約として既に確定しており、承認時に実測値がこれを満たすことを確認する。

| 制約 | 値 | 根拠 |
|---|---|---|
| 5 V ingressを流れる最大電流 | 上行に同じ（最弱部品定格の80%）。**connector一般定格（約1.8 A）を上限に使わない** | 最弱部品が経路全体の上限を決める。1.5〜1.8 Aの測定値を「合格」としないため |
| **合成給電時にingressを通る定常電流** | **経路上の最弱部品の定格の80%以下。**候補構成なら`1.5 A × 0.8 = 1.2 A`。超えたら直ちに中止する | connector定格は熱の制限であり、評価量は定常値である。80%の根拠は`ingressの電流制限` |
| ADC入力へ加える最大電圧 | 3.3 V以下（5 V／3.3 V railは分圧比1/2を経由） | ESP32のADC入力範囲 |
| ADC loggingのsample rate | 1 kSample/s以上 | servo起動過渡がms orderであるため。`Sample rateとlog形式`節 |
| logic GNDとservo GND間に許容する直列抵抗 | 0Ω（star point直結。shuntを経由させない） | `GND topology`節 |

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
| 2026-08-05 | 11 | レビュー指摘3件を反映。(a) M-12001はMicro-Bオスplugでbreadboardへ直接挿せないが、そこからrailまでの物理interfaceが未定義だったため`5 V ingress`節を新設し、必要な変換基板（未購入）、ESP32の3系統排他制約、Piの給電port分離を明記。あわせてMicro-B connector定格（約1.8 A）に対し文献値の最悪同時peakが2 Aを超えうる問題を記載し、実測または高定格connectorへの変更まで電源経路を承認しない旨をConnector定格行へ反映。(b) `数十kHz程度`のsample rateが115200 baud（実効約11.5 kB/s、16 bit連続で約5.7 kSample/s）では到達不能だったため、連続streamingを止めてburst capture＋事後dump方式へ変更し、必須要件1 kSample/s・目標5〜10 kSample/s・buffer長・log形式を規定。(c) 低側shuntの挿入位置とADCの基準nodeが未定義で、共通GNDだとservo電流がESP32のGND基準を動かす問題があったため、`GND topology`節を新設しstar point構成（shuntはservo戻り専用、ESP32 GNDはstar point直結）を図示。許容GND offsetと、topologyが取れない場合は測定しない旨も明記 | [PR #55レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55)、Raspberry Pi Zero W公式回路（PWR INはdata線未接続）、Espressif ESP32-DevKitC V4文書（電源3系統は排他）、115200 8N1の実効throughput |
| 2026-08-05 | 12 | 自己レビューで検出: Revision 11で追加した`5 V ingress`節が、ESP32を`5V` pinから給電すると書きながら、同じ節で「USB接続と5V pin給電の同時使用を承認しない」とも書いており矛盾していた。Pi linkがUSB serialである以上、Pi接続時点でVBUSは通電するため、この2つは両立しない。`ESP32の給電経路（未決定）`節を新設して案A（USB VBUS単独給電、既定候補）と案B（`5V` pin給電）を並べ、未決定であることを明示。rail構成図、配線・保護表の2行、測定計画も同じ状態へ揃えた。あわせて0.1Ω shuntの低電流側の精度限界（実用域は約1 A以上）を測定計画へ追記し、`受け入れ条件`への記載を「記載する」から「記載している」へ訂正 | 自己レビュー、Espressif ESP32-DevKitC V4文書（電源3系統は排他）、ESP32 ADCの入力直線性 |
| 2026-08-05 | 13 | 自己レビューで検出: `Sample rateとlog形式`表が、ADC sample rateの確定を`HW-TBD-014`と対にすると書いていたが、`HW-TBD-014`はPi linkのbaudと最大line長であり測定treatmentのsample rateとは別物である。誤った対応付けを削除し、dumpに使うbaudは測定用に別途選んでよい旨へ訂正。`GND topology`の図が枝の接続関係を読み取りにくかったため描き直した。`tbd-register.md`側では、BOMで識別済みの`HW-TBD-015`（microSD）と`HW-TBD-016`（color sensor）が識別情報を未反映のまま残っていたため、確定部分と残作業を書き分けた | 自己レビュー、[tbd-register.md](tbd-register.md)、[hardware-bom.md](hardware-bom.md) SD-01／COLOR-01 |
| 2026-08-05 | 14 | レビュー指摘1件を反映。Revision 13で「dumpに使うbaudは測定用に別途選んでよい」と書いたのは誤りだった。Pi–ESP32間のUSB serialは1本しかなく、Protocolがそのbaudを共有transport parameterとして持つため、片側だけ別baudにはできない。dumpは`HW-TBD-014`／`PROTO-TBD-001`が決めるbaudをそのまま使うことにし、baudを変える場合はProtocol側と揃えて決定を変える扱いとした。あわせて、生CSVをJSON Lines channelへ流すとProtocolの「送信するすべてのbyteは有効にframe化されたmessageの一部でなければならない」要件に反するため、CSV出力はProtocolが動作していない専用の測定modeに限る条件を明記した | [PR #55レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55)、[protocol](../protocol/esp32-pi-protocol.md)§2のtransport表とframing要件 |
| 2026-08-08 | 15 | 5 V ingressの記述が「変換基板が無いと配線を開始できない」と過大だったため、段階表を追加して訂正した。bring-up前半はM-12001をPiの`PWR IN`へ直挿しし、Piの5 V GPIO pinからbreadboard railを作れば**追加購入なしで通電できる**（logic側のみ、servoは繋がない）。servo試験の直前に変換基板構成へ移す。候補変換基板（秋月 g110972）の定格が1ピン1.5 Aと、記載していた「Micro-B一般定格 約1.8 A」より低いことも反映した。購入は前半の実測後で足り、実電流に基づいて品を選べる | ユーザーからの指摘（購入済み5点にingress部品が含まれていない）、[秋月 g110972](https://akizukidenshi.com/catalog/g/g110972/)の定格、Raspberry Pi Zero W公式回路（PWR INと5 V GPIO pinは直結） |
| 2026-08-08 | 16 | レビュー指摘2件を反映。(a) Pi直挿しmodeを「文献値で合計0.5 A程度」と正当化していたが、これは根拠が無い。ESP32のspikeだけで約500 mA、Pi stressで約350 mA、MSP2807は未確認であり、文献値だけでも瞬時に0.85 Aを超えうる。`Pi直挿しmodeの電流gate`節を新設し、実測合計**1.0 Aを上限**として超えたら直ちに中止する条件、その1.0 AをMicro-B定格約1.8 Aに対する約1.8倍の余裕として選んだdesign marginの根拠、および段階的に負荷を足しながら測る手順を定めた。配線・保護表の`Piの5 V入力経路`行、測定計画の`サーボ接続前`、`受け入れ条件`の数値表にも同じgateを反映した。(b) Revision履歴で2026-08-08の行が古い行より前に挿入されていたため日付順へ並べ直した | [PR #57レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/57)、負荷表の文献値 |
| 2026-08-09 | 17 | 昇格PR [#61](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/61)のレビュー指摘4件を反映。(a) **ESP32給電の循環論法を解消。**「案A/Bが確定するまで通電しない」と書きながら、案を決める実測には通電が要る状態だった。通電可否を「案の確定」ではなく「どの経路で通電するか」で決める方式へ変更し、案Aの単一経路・電流制限・監視付きに限ったbring-upを許可した。案Bは秋月基板のVBUS保護diode確認まで通電しない。(b) **1.0 A gateの測定手段が無かった。**servo rail低側shuntはlogic側を通さず、テスターは過渡peakを取り逃がす。定常値と過渡peakで手段を分け、logic側の測定点が未確定であること、確定までgateは定常値しか保証しないことを明記した。(c) ingress上限を「Micro-B一般定格 約1.8 A」から**経路上の最弱部品の定格**（候補構成では変換基板の1ピン1.5 A）へ変更。1.5〜1.8 Aの測定値が合格になる穴を塞いだ。(d) sample rateが必須要件に届かない場合の対処から「buffer長の短縮」を削除。buffer長は時間分解能を改善しないため、ADC改善・要件の見直し・測定手段の追加のいずれかを取ることにした。あわせて見出し階層の飛び（MD001）を修正 | [PR #61レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/61) |
| 2026-08-09 | 18 | [PR #64](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)のレビュー指摘3件を反映。(a) **測れないgateを廃止した。**「Pi直挿しで合成給電し、合計1.0 Aを超えたら中止」としていたが、M-12001はMicro-Bオスplugの直結cableでアダプターとPiの間に測定器を挿入できず、**そのgateを測る手段が無かった**。段階をA（Pi単体起動）／B（ESP32をPCのUSBから給電）／C（合成給電）に分け、AとBはgate不要な通常の使い方、Cは変換基板の到着後とした。これにより「追加購入なしで合成給電できる」という前提が誤りだったことも訂正している。(b) 案Aの「電流制限」が未定義だったため、経路ごとに制限を明示した。段階BはPC hostのUSB port OCP、案AはPiのUSB OTG port供給能力（未確認、ingressで実測）、案Bはingress上限。(c) 「十分な余裕」が曖昧だったため、**最弱部品定格の80%**という数値のderatingを定めた（候補構成で1.2 A）。80%とする理由（servo peakの負荷依存、ADC loggingの取りこぼし、接触抵抗）も記載した。あわせて節名を`connector定格の制約`から`ingressの電流制限`へ変更し、配線・保護表・測定計画・受け入れ条件を揃えた | [PR #64レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64) |
| 2026-08-09 | 19 | [PR #64](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)のレビュー指摘を反映。ingress低側shuntをESP32 ADCで測る案が**電気的に成立しない**と判明した。star pointを基準にするとadapter return側は`I × R`だけ負の電位になり、ESP32のADCでは測れない。high-side current sense IC（例: INA219、秋月で¥2,300）で解決できるが、**そもそもpeakで判定する必要が無い**ため部品を追加しない。connectorとtraceの電流定格は`I²R`発熱と放熱の釣り合いで決まる熱の制限であり、時定数は秒から分のorderで、µs〜msのspikeは発熱にほとんど寄与しない。判定量を**定常値（テスター直列）**へ改め、peakによる電圧降下は既存の`ADC-5V`／`ADC-3V3`で捉えることにした。測定計画と受け入れ条件も揃えた | [PR #64レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)、connector定格の熱的性質 |
| 2026-08-09 | 20 | [PR #64](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)のレビュー指摘を反映。Revision 19で判定量を定常電流へ改めたが、「実測でservoを含む同時**peak**が80%以下に収まることを示す」という要件が残っており矛盾していた。測定不要としたpeakが、合成給電の許可と部品選定を阻む状態だった。判定を定常電流に統一し、peakは電圧droop・brownout・resetの判定にのみ使う旨を明記した。あわせて`変換基板に必要な定格の見積もり`節を追加し、定常電流はbranchごとに測って足せること（peakと違い時間的重なりを仮定しなくてよい）、ESP32＋LCD＋sensorのbranchは段階Bで測れることを示した。これで変換基板の品をingressの実測を待たずに選べる | [PR #64レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64) |
| 2026-08-09 | 21 | [PR #64](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)のレビュー指摘を反映。`変換基板に必要な定格の見積もり`が、servo branchの測定を段階C（変換基板が要る）に限定しており、**購入前に見積もれるという主張と循環していた**。実測できるbranchと、できないbranchを分けた。servoは供給経路がingressしかなく実測できないため、連続動作時の上限として広く示される250 mAを**割り当て予算**として置き、実機がこれを超えないことは`servo-safety-limits.md`のtrajectory制限で担保することにした。stall電流は採らない（connector定格は熱の制限のため）。これで未知数はMSP2807の1つだけになり、段階Bの実測で合計が確定する。足す値が「同時に継続しうる動作条件での定常電流」であることも明記した | [PR #64レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)、SG90の連続動作電流の一般値 |
