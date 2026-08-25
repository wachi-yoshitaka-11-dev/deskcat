# Hardware Safety Policy

> 状態: Active
> 適用範囲: DeskCatの電子回路、firmware出力、ベンチ試験、機械動作

## 1. 安全原則

いかなるfirmware機能も、損傷、予期しない動作、過熱、診断根拠の喪失を防ぐことより優先しない。

必要な電気的または機械的情報が不明な場合は、次を行う。

1. `TBD`と記録する。
2. 影響する出力を無効のままにする。
3. 必要な公式文書または測定を特定する。
4. 結果をハードウェアの正本文書へ記録した後にのみ再開する。

### 安全要件の5項目

**次の5項目は、外したときの帰結が部品またはboardの破損である。**
一次資料または実測を要求し続ける。**この水準を緩めない。**

**この5項目は列挙であり、導出ではない。網羅性は証明していない。**
追加は安全側への変更であり、[ADR-0014](../decisions/0014-safety-requirements-and-general-values.md)の
改訂を要さない。**削除は要する。**

| 項目 | 外したときに起きること | 値と状態の正本 |
|---|---|---|
| **短絡** | **電源の供給能力が、経路の最弱部品の定格を上回る。****過電流保護部品（`PROT-OC-01`）が入るまで、故障電流を制限するものが経路に無い。**connectorと線材の発熱は秒から分のorderで進むため、人が読み取って電源を落とすのでは止まらない | [過電流保護（段階Cのgate）](../hardware/power-budget.md#過電流保護段階cのgate)、[power-budget](../hardware/power-budget.md)の`配線・保護表`。`HW-TBD-021`／`HW-TBD-022` |
| **逆極性** | 5 Vを逆に入れると、Pi・ESP32・moduleが同時に死ぬ | [逆極性保護のdesign review](../hardware/power-budget.md#逆極性保護のdesign-review)。`HW-TBD-030` |
| **給電経路の重複** | 電源系統は排他であり、2つ以上から同時に給電するとboardまたは電源が破損する | [ESP32の給電経路](../hardware/power-budget.md#esp32の給電経路案aで確定実測待ち)。[§3 電源](#3-電源)のbackfeed確認と合わせて読む |
| **電圧違いのpinへ挿す** | 3.3 Vのpinへ5 Vを入れると、その部品が死ぬ | [§4 Logic信号とGPIO](#4-logic信号とgpio)。[GPIO割り当て](../hardware/gpio-assignment.md)。servoへ入れてよい電圧は`HW-TBD-035` |
| **servoの持続的拘束** | 開放loopのhobby servoは、機構に押し付けられても最大torqueを出し続け、静かに過熱してギアボックスが焼ける | [拘束（stall）と過負荷](../hardware/servo-safety-limits.md#拘束stallと過負荷)。`HW-TBD-020` |

**値と状態をここへ再掲しない。**正本が持つ。この表が持つのは「どれが安全要件か」だけである。
未確定項目の状態は[TBD台帳](../hardware/tbd-register.md)が正本である。

**この5項目は、gateの開閉とは別の話である。**どれが安全要件かを定めることは、
[サーボ出力を有効化してよい条件](../hardware/servo-safety-limits.md#サーボ出力を有効化してよい条件)や
技術ガイドのゲートを開くことではない。

### 5項目以外の扱い

**5項目に効かないものは、一般的な値で始めて、動かして観察してよい。**
対象は、外した場合の帰結が**「動かない」または「作り直す」に留まるもの**である。

**判断の分かれ目は「外したら壊れるか」であって「根拠が一次資料か」ではない。**

[AGENTS.md](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/AGENTS.md)の
「推測禁止」が挙げる8分類は、この基準で次のように分かれる。**この対応表がその正本である。**

| 推測禁止の分類 | 扱い | 理由 |
|---|---|---|
| 部品型番 | **一次資料または実測** | 型番が電圧・電流・pinoutのすべてを決める。外すと5項目のすべてに効く |
| GPIO | **一次資料または実測** | 挿し先を間違えると電圧違いのpinへ入る。起動時状態も含む |
| 供給電圧とロジック電圧 | **一次資料または実測** | 電圧違いと逆極性に直結する |
| サーボPWM、可動域、速度、加速度 | **一次資料または実測** | 可動域を外すと機構へ押し付け、持続的拘束になる |
| 電源・抵抗・コンデンサ値のうち、**故障電流を決めるか制限するもの**と**耐圧** | **一次資料または実測** | 短絡の帰結を決める。耐圧を超えたcapacitorは破裂しうる |
| I2Cアドレス | 一般値で開始してよい | 外れると応答しない。壊れない |
| I2C／SPIの速度とmode | 一般値で開始してよい | 外れると通信しないか化ける。壊れない |
| タッチ、加速度のしきい値 | 一般値で開始してよい | 外れると誤検知する。壊れない |
| 電源・抵抗・コンデンサ値のうち、**pull-upとdecouplingの値** | 一般値で開始してよい | 外れると通信または安定性が悪くなる。壊れない。**耐圧は上の行で扱う** |

**「電源・抵抗・コンデンサ値」は1つの分類として扱えない。**同じ分類の中に、短絡の帰結を
決めるものと、外しても動かないだけのものが混在する。**分類名で判断しない。**

**一般値で開始する場合も、記録は落とさない。**採った値、それが暫定であること、
確定させる手段を記録する。**追跡可能性は緩めていない。**緩めたのは、着手前に一次資料を
そろえる要求だけである。

**この対応表は、現在の構成（LCD、touch、環境sensor、servo）に対するものである。**
**出力を持つdeviceや、別の電圧domainを足したら、表を見直す。**

先例がある。§5のSPIは「検証済みの保守的な速度から開始し、根拠がある場合だけ速度を上げる」と
既に定めている。**速度について、この文書はもともと「一般値で開始して観察する」を採っていた。**
この節はその考え方を、5項目に効かない範囲へ明示的に広げたものである。

**迷ったら一次資料の側に置く。**

## 2. 必須の根拠

deviceを接続する前に、次を確認する。

- 正確なメーカー名とmodel suffix
- 搭載ICだけではなくmodule boardの識別情報
- 供給電圧
- logic電圧
- 絶対最大定格
- 通常電流とpeak電流
- pinoutとconnector方向
- 起動時状態
- 必要な外付け部品
- bus address、mode、対応速度

販売業者の掲載情報や、外観が似ているmoduleは十分な根拠にならない。

**この10項目のうち、接続前に一次資料または実測を要するものと、一般値で開始してよいものの区別は、上の「[5項目以外の扱い](#5項目以外の扱い)」の対応表による。**
現在の対応表では、**一般値で開始してよいのは`bus address、mode、対応速度`だけである。**
残る9項目は接続前に確認する。

## 3. 電源

- サーボをESP32のGPIO、3.3 V rail、board regulatorから給電しない。
- 容量を適切に設計した独立5 Vサーボ電源を使用する。
- ESP32、サーボ電源、周辺deviceのGNDを意図的に共通化する。
- USBと外部電源の経路にbackfeedがないか確認する。
- 電源予算には、通常電流と同時動作時のpeak電流を含める。
- regulator、wire、connector、保護deviceの定格を確認する。
- 必要なdecoupling部品を各deviceの近くへ配置する。
- 大電流のサーボ経路と、長いsensor return pathを共有させない。
- サーボ過渡動作中の5 V、3.3 V、reset動作を測定する。

一般的なsampleをコピーしてcapacitorや電源容量を決定しない。初期値を計算し、実負荷のwaveformで検証する。

## 4. Logic信号とGPIO

- 正確なboard文書に別の記載がない限り、ESP32信号を3.3 V domainとして扱う。
- 不明な出力または5 V出力をESP32入力へ直接接続しない。
- module上のpull-upと、その接続先電圧を確認する。
- 一方のdeviceだけに給電されている場合の動作を確認する。
- bootstrap、flash、debug、UART、board予約pinを確認する。
- pinをoutput modeへ切り替える前に、安全な初期出力を定義する。
- reset、LCD backlight、chip select、サーボPWMの起動時glitchを確認する。

GPIO割り当ては`docs/hardware/gpio-assignment.md`を正本とする。

## 5. I2CとSPI

### I2C

- addressの衝突を確認する。
- pull-up先の電圧と、並列合成後の実効抵抗を確認する。
- すべてのtransactionにtimeout上限を設ける。
- NACK、timeout、CRC、not-ready errorを区別する。
- 組み立てた配線で信号のrise timeとnoiseを確認する。
- サーボ動作中にもtestする。

### SPI

- 共有bus上の各deviceに個別のchip selectを割り当てる。
- 同時にactiveになるchip selectを一つだけにする。
- deviceを切り替えるたびに、そのdeviceの正しいmodeと速度を適用する。
- 長いLCD転送がタッチ入力をstarveさせないことを確認する。
- 検証済みの保守的な速度から開始し、根拠がある場合だけ速度を上げる。

## 6. サーボ

最初のPWM出力前に次を行う。

- 正確なサーボmodelを確認する。
- 電圧、電流、PWM周期、pulse width要件を確認する。
- 可能であればサーボhornまたは機械負荷を外す。
- 電流制限を設定でき、十分な定格を持つ電源を使用する。
- 狭いpulse範囲と可動域を設定する。
- 直ちに操作できる電源遮断手段を用意する。
- 手やcableを動作範囲の外へ置く。

Firmwareでは次を強制する。

- calibration済み最小位置
- neutral位置
- calibration済み最大位置
- 最大角速度
- 最大角加速度
- 最大command範囲
- 単位時間あたりに受理するmotion command数
- 最大連続動作時間とduty cycle
- 拘束または過負荷を検知した場合の停止
- 通信断時の定義済み動作
- resetまたはdriver故障時の定義済み動作

debug commandでもこれらの制限を迂回してはならない。

**この段落は受理判定にだけ適用する。**上限を超えたcommandは、無言で破棄せず理由を返す。

計数の境界（設定値がNのとき何番目から拒否するか）は
[Servo Safety Limits](../hardware/servo-safety-limits.md)を正本とする。
ここでは規則を再掲しない。同じ規範を2箇所に置くと、片方だけが更新されて食い違う。

**実行中の動作に対する安全停止はこれとは別である。**最大連続動作時間、duty cycle、
拘束・過負荷は、理由を返すだけでは足りず、trajectoryの中止またはPWM disableと
fault eventを要する。理由を返して動作を続ける実装にしない。
停止の要件は[Servo Safety Limits](../hardware/servo-safety-limits.md)を正本とする。

正本の分担は次のとおり。

| 対象 | 正本 |
|---|---|
| 安全要件（何を満たすか、検知時に何をするか）と計数の規則 | [Servo Safety Limits](../hardware/servo-safety-limits.md) |
| しきい値・時間・窓・上限の**実測値** | [TBD台帳](../hardware/tbd-register.md) |
| stale commandの拒否条件 | Protocolの`PROTO-TBD-013` |

可動域は小さいstepで拡大し、電流、電圧、noise、干渉、機械的clearanceを記録する。

次の場合は試験を停止する。

- 予期しない方向への動作
- 衝突または拘束
- 異音
- 過熱
- resetまたはbrownoutの反復
- 過電流
- commandまたは緊急停止応答の喪失

## 7. 人間の監視が必要な操作

次の操作では、人間が立ち会い、電源を遮断できる状態にする。

- 新しい配線revisionの初回通電
- 新しい電源の初回使用
- 初回サーボPWM
- 機構へ接続したサーボの初回動作
- 可動域の拡張
- GPIOまたは電源変更後の試験
- 意図的なbrownoutまたはfault injection
- 長時間の機械動作試験

AIエージェントはcommandとchecklistを準備できるが、人間の観察または取得済み測定値がなければ、物理的結果を確認済みと表現してはならない。

## 8. 通信のfail-safe

ESP32は次を検証する。

- message length
- JSON構造
- protocol version
- command名
- field type
- 列挙値
- 数値範囲
- commandの識別と重複
- 該当する場合はcommand age

通信断時、ESP32は承認済みのサーボfail-safe動作に従う。上限のない動作sequenceを継続したり、再接続後に古いrelative commandを再実行したりしてはならない。

この要求が成立する前提として、ESP32が通信断を自力で検知できなければならない。Heartbeat source、loss timeout、fail-safe sequenceが未確定の間は、サーボ出力を有効にしない。追跡は[HW-TBD-017／018](../hardware/tbd-register.md)で行う。

ただしこれは有効化条件の一部である。条件の全体は[Servo Safety Limits](../hardware/servo-safety-limits.md#サーボ出力を有効化してよい条件)を正本とする。ここに挙げた項目だけを満たしても、出力を有効化してよいことにはならない。

また、送信側の再起動によってmessage IDが振り直されるため、duplicate判定はsession境界を考慮しなければならない。詳細は[Protocol](../protocol/esp32-pi-protocol.md)を参照する。

## 9. Interruptとwatchdogの安全

- ISRを最小限に保つ。
- queueに上限を設け、overflowを記録する。
- logまたはserial出力によってsensor処理やwatchdogの進行をblockしない。
- watchdog feedは、重要taskが実際に進行したことを表さなければならない。
- 再起動後にreset reasonを記録する。
- watchdog、brownout、panic、external resetを区別できるfault contextを保存する。

## 10. ベンチ試験記録

次の項目を含む記録を使用する。

```text
Test ID:
Date:
Operator:
Hardware revision:
Exact components:
Wiring revision:
Power supply and current limit:
Firmware commit/profile:
Configuration:
Measurement equipment:
Procedure:
Expected result:
Measured result:
Faults:
Conclusion:
Next safe step:
```

## 11. 緊急時の対応

危険な動作が発生した場合は、次の手順を取る。

1. actuator電源を遮断する。
2. 同じcommandを繰り返し再実行しない。
3. logとreset reasonを保存する。
4. 安全に実行できる場合は、変更前に配線を撮影または記録する。
5. actuatorを外して電源railを測定する。
6. 焦点を絞ったIssueまたは実験記録を作成する。
7. 妥当な原因候補と安全な試験を定義した後にのみ、より小さい独立試験から再開する。
