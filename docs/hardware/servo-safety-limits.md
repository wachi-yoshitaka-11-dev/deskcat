# Servo Safety Limits

> 状態: Blocked — 正確なservoとmechanical assemblyが必要
> 正本とする情報: サーボの電気的制限、機械的制限、動作制限、fail-safe動作

## 確定しているproject規則

- サーボにはESP32の電源pinではなく、外部電源経路を使用する。
- ESP32とサーボ電源のGNDを意図的に共通化する。
- ESP32はRaspberry Piから独立して、hard motion limitを強制する。
- AIが生成したcommandやdebug commandでもhard limitを迂回できない。
- 初回動作では負荷を外すか、意図的に狭い安全範囲を使用する。

## サーボ識別情報

| 項目 | 値 | 根拠 |
|---|---|---|
| メーカー | TBD | 現物表示 |
| 正確なmodel／suffix | TBD | 現物表示 |
| データシートrevision | TBD | メーカー文書 |
| 定格電圧範囲 | TBD | データシート |
| 無負荷電流 | TBD | データシート／測定 |
| 動作電流 | TBD | データシート／測定 |
| Stall／peak電流 | TBD | データシート／測定 |
| 制御logic要件 | TBD | データシート |
| PWM周期／rate | TBD | データシート |
| 許容最小pulse | TBD | データシートと無負荷試験 |
| Neutral pulse | TBD | Calibration |
| 許容最大pulse | TBD | データシートと無負荷試験 |

一般的なhobby servoの値を、この表の確定値として使用しない。

## 機械組み立て

| 項目 | 値 | 根拠 |
|---|---|---|
| Horn typeと取付index | TBD | 組み立て記録 |
| 機械的neutral姿勢 | TBD | Calibration |
| 左方向の物理的障害 | TBD | 低速確認 |
| 右方向の物理的障害 | TBD | 低速確認 |
| Cableで制限される範囲 | TBD | 組み立て確認 |
| 安全なsoftware最小値 | TBD | Marginを含むcalibration |
| 安全なsoftware最大値 | TBD | Marginを含むcalibration |
| PWM停止時の重力による動作 | TBD | 監視下試験 |

Software可動域は、明示的なmarginを設けて機械的可動域の内側に収める。

## 動作制限

| 制限 | 承認値 | 設定可能なhard bound | 根拠 |
|---|---:|---:|---|
| Neutral位置 | TBD | TBD | Calibration |
| 最小位置 | TBD | TBD | Calibration |
| 最大位置 | TBD | TBD | Calibration |
| 最大速度 | TBD | TBD | 動作／電流試験 |
| 最大加速度 | TBD | TBD | 動作／電流試験 |
| 単一commandの最大変化量 | TBD | TBD | 動作・安全試験 |
| 最大連続動作時間 | TBD | TBD | 温度／電流試験 |
| Command timeout | TBD | TBD | Protocol／fail-safe試験 |

Runtime設定では、動作をより保守的にしてよい。Firmwareへcompileするか安全にprovisionしたhard configurable boundを超えてはならない。

## Command処理

```text
received command
  → protocol validation
  → motion-name/target validation
  → hard range clamp or rejection
  → velocity and acceleration limiting
  → calibrated pulse conversion
  → hardware PWM
  → state and clamp-counter report
```

構造的に不正または明らかに危険なcommandは、clampよりrejectを優先する。有効なtargetを保守的にclampした場合は、machine-readableなstatusまたはeventで報告する。

## 起動時動作

無負荷試験後に次を決定する。

- PWM driver初期化前のGPIO state
- PWMをdisabledで開始するか、calibration済みneutralで開始するか
- Actuator enableまでのdelay
- Pi未接続時の動作
- Watchdog、panic、brownout reset後の動作

承認されるまで、安全状態は「未検証の動作出力を行わない」とする。

## 通信断時動作

機械試験後に、次のいずれかを選択する。

- 短時間保持し、低速でneutralへ移動してからdisableにする
- 現在位置を保持する
- 直ちにPWMをdisableにする

PWMをdisableにしたときに首が落下したり予期せず動いたりするかを考慮して選択する。

次を記録する。

| 項目 | 値 |
|---|---|
| Heartbeat source | TBD |
| Loss timeout | TBD |
| 選択したfail-safe sequence | TBD |
| Recovery／reconnect動作 | TBD |
| Stale command rejection | TBD |

再接続後に古いrelative-motion commandを再実行してはならない。

## 緊急停止

ベンチ環境には次を用意する。

- 人間が直ちに操作できるactuator電源遮断手段
- hard limitを弱めず停止を要求するfirmware command
- 新しいtrajectoryを停止するtimeoutまたはfault path
- 文書化された再起動手順

最終的な緊急操作はactuator電源の遮断である。初回試験ではserial commandだけでは不十分である。

## Calibration手順

1. 正確なサーボと電源dataを確認する。
2. 可能であればhornを外すか無負荷にする。
3. 安全な電流制限と電源遮断手段を設定する。
4. サーボ未接続でPWM waveformを確認する。
5. 無負荷のサーボを接続する。
6. 検証済みneutral候補の周辺に狭いpulse範囲を設定して開始する。
7. 動作方向を確認する。
8. 承認済みneutral姿勢で機構へ取り付ける。
9. 一方向ずつ低速・小stepで範囲を広げる。
10. 接触または電流の急増前に停止する。
11. 物理的境界を記録する。
12. 反対側でも繰り返す。
13. Safety marginを差し引く。
14. 承認範囲内で速度と加速度を試験する。
15. 通信断、reset、緊急停止を試験する。

## 受け入れchecklist

- [ ] 正確なサーボとデータシートを記録した
- [ ] Peak電流から電源容量を決定した
- [ ] サーボ接続前にPWMを測定した
- [ ] 機械的neutralを記録した
- [ ] 両方向の物理的境界を記録した
- [ ] Software可動域にsafety marginを含めた
- [ ] 速度と加速度制限を試験した
- [ ] 起動時に危険なglitchがない
- [ ] 通信断時動作を試験した
- [ ] 緊急電源遮断を試験した
- [ ] 受け入れ動作試験中にESP32 brownoutがない
- [ ] 動作中もLCD、sensor、serialが機能する

## Revision履歴

| 日付 | Revision | 変更 |
|---|---|---|
| 2026-07-27 | 0 | 安全とcalibrationの構造を作成。device固有の制限はすべて引き続きTBD |
