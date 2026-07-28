# Power Budget

> 状態: Blocked — 正確な部品と測定済みpeak電流が必要
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
TBD wired source
├─ Logic/Pi regulated path
│  ├─ Raspberry Pi Zero WH
│  ├─ ESP32-DevKitC-32E
│  └─ LCD and sensors as their voltage requirements permit
└─ Servo regulated 5 V path
   └─ SERVO-01

すべての信号domainで、意図的に基準GNDを共有する。
接続前に、USBと外部電源間のbackfeed経路をreviewする。
```

この図はarchitecture案であり、最終配線図ではない。

## 負荷表

| Rail | 負荷 | 数量 | Typical電流 | 最大連続電流 | Transient／peak | 根拠 | 確度 |
|---|---|---:|---:|---:|---:|---|---|
| Logic TBD | ESP32 board | 1 | TBD | TBD | TBD | Board文書＋測定 | TBD |
| 5 V logic TBD | Raspberry Pi Zero WH | 1 | TBD | TBD | TBD | 公式文書＋測定 | TBD |
| TBD | LCDとbacklight | 1 | TBD | TBD | TBD | 正確なmodule文書 | Blocked |
| TBD | Touch controller | 1 | TBD | TBD | TBD | 正確なmodule文書 | Blocked |
| TBD | Accelerometer | 1 | TBD | TBD | TBD | 正確なmodule文書 | Blocked |
| TBD | Environment sensor | 1 | TBD | TBD | TBD | 正確なmodule文書 | Blocked |
| Servo 5 V候補 | Servo | 1 | TBD | TBD | Stall／peakはTBD | 正確なservo文書＋測定 | Blocked |

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
| 入力電源 | 電圧、連続電流、peak電流 | TBD | TBD | Blocked |
| Logic regulator／経路 | Pi／ESP32／周辺deviceの要件 | TBD | TBD | Blocked |
| Servo regulator／経路 | 正確なservo要件 | TBD | TBD | Blocked |
| Backfeed防止 | USB／外部電源の共存 | TBD | 回路図review | Blocked |
| Servo bulk capacitor | 測定した過渡電流への対応 | TBD | Oscilloscope capture | Blocked |
| Local decoupling | 各deviceのデータシートに従う | TBD | データシート | Blocked |
| Wire gauge／許容電流 | Peak電流と長さ | TBD | 製品資料／計算 | Blocked |
| Connector定格 | Peak電流と誤接続防止 | TBD | 製品資料 | Blocked |
| 過電流保護 | 故障電流の制限 | TBD | Design review | Blocked |
| 逆極性保護 | 配線リスク | TBD | Design review | Blocked |

## 測定計画

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
