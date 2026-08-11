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

部品の識別情報の**正本は[hardware-bom.md](hardware-bom.md)のSERVO-01**である。
この表はそこから引いた値を、安全確認の作業者がここだけで読めるように再掲したものであり、
値をこの文書で確定させない。BOM側を更新したらこの表も合わせる。

| 項目 | 値 | 根拠 |
|---|---|---|
| メーカー | TowerPro | 現物ラベル（`TOWER PRO Micro servo 9g SG90`）。[HW-TBD-006](tbd-register.md)で解決済み |
| 正確なmodel／suffix | SG90 | 同上 |
| データシートrevision | [TowerPro公式](https://towerpro.com.tw/product/sg90-7/)、[datasheet](https://www.mouser.com/catalog/specsheets/Soldered_101246.pdf)。revision表記なし | メーカー文書 |
| 定格電圧範囲 | 4.8–6 V | データシート |
| 無負荷電流 | **TBD** | データシート／測定 |
| 動作電流 | **TBD**（データシート値は0.5–2 Aと負荷依存で幅が広い） | データシート／測定 |
| Stall／peak電流 | **TBD**（実測必須。[HW-TBD-010／011](tbd-register.md)、`power-budget.md`測定計画） | データシート／測定 |
| 制御logic要件 | **TBD**（ESP32のGPIOは3.3 V。SG90のlogic閾値を現物確認するまで確定しない） | データシート |
| PWM周期／rate | **TBD**（50 Hzが一般値だが、この表の確定値として採らない） | データシート |
| 許容最小pulse | **TBD** | データシートと無負荷試験 |
| Neutral pulse | **TBD** | Calibration |
| 許容最大pulse | **TBD** | データシートと無負荷試験 |

**modelは確定したが、駆動条件はまだ確定していない。**上表で`TBD`が残る項目は、
一般的なhobby servoの値やdatasheetの代表値を確定値として使用しない。
とくにpulse幅とstall電流は、実機のcalibrationと測定で決める。

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
| Neutral位置 | TBD | TBD | Calibration。[HW-TBD-010](tbd-register.md) |
| 最小位置 | TBD | TBD | Calibration。[HW-TBD-010](tbd-register.md) |
| 最大位置 | TBD | TBD | Calibration。[HW-TBD-010](tbd-register.md) |
| 最大command範囲 | TBD | TBD | 受理してよいcommand値の範囲。最小位置と最大位置で決まる。[HW-TBD-010](tbd-register.md) |
| 最大速度 | TBD | TBD | 動作／電流試験。[HW-TBD-011](tbd-register.md) |
| 最大加速度 | TBD | TBD | 動作／電流試験。[HW-TBD-011](tbd-register.md) |
| 単一commandの最大変化量 | TBD | TBD | 動作・安全試験。[HW-TBD-020](tbd-register.md) |
| **最大連続電流** | **250 mA（予算）** | **firmwareへ直接は設定しない。**電流監視を採用しない構成ではfirmwareが電流を測れないため、強制点はこの表の**可動域・最大速度・最大加速度・最大連続動作時間・最大duty cycle**である。250 mAは、それらを強制した状態での**実測結果として確認する**量である（電流監視を採用する場合の遮断しきい値は`TBD`。[HW-TBD-020](tbd-register.md)） | [power-budget.md](power-budget.md)が5 V ingressの定格を見積もるためにservoへ割り当てた予算。**実測値ではない。**可動域・速度・duty cycleは、実機の連続電流がこの予算を超えないよう決める。**超えた場合の既定の対処は制限を締めることであり、電源側でingressの定格を上げても、この表がfirmwareへ渡す値は変わらない。**予算そのものを変えるには同文書の正式改訂の手順（両文書を同時に改訂し、受け入れchecklistを通し直す）を踏む。確認は`受け入れchecklist`で行う |
| 最大連続動作時間 | TBD | TBD | 温度／電流試験。[HW-TBD-020](tbd-register.md) |
| 最大duty cycle（一定時間窓あたりの動作時間比） | TBD | TBD | 温度／電流試験。[HW-TBD-020](tbd-register.md) |
| 秒あたり受理motion command数 | TBD | TBD | 動作・温度試験。[HW-TBD-020](tbd-register.md) |
| Command timeout | TBD | TBD | Protocol／fail-safe試験。[HW-TBD-020](tbd-register.md) |

この表は、[Hardware Safety Policy](../governance/hardware-safety-policy.md)が
Firmwareへ強制を要求する制限のうち、**数値と受理上限**に対応する。
policy側のその種の項目がこの表に無い状態を作らない。無いと、強制すべき値が
定義もされず追跡もされないまま残る。

policyが要求する**挙動**（拘束・過負荷時の停止、通信断時の動作、
resetとdriver故障時の動作）はこの表の対象外であり、それぞれ
「[拘束（stall）と過負荷](#拘束stallと過負荷)」「[通信断時動作](#通信断時動作)」
「[起動時とdriver故障時の動作](#起動時とdriver故障時の動作)」と有効化ゲートで扱う。
数値の行として重複させない。

Runtime設定では、動作をより保守的にしてよい。Firmwareへcompileするか安全にprovisionしたhard configurable boundを超えてはならない。

## 拘束（stall）と過負荷

開放loopのhobby servoは、機構が拘束されても最大torqueを出し続け、静かに過熱・焼損する。DeskCatは卓上で人が触る前提のため、「首を手で押さえたままmotion commandが走る」状況を通常起こりうる事象として扱う。

Firmwareは、正確なservoが確定した後に次を**少なくとも一つ**実装する。
時間base制限は常に必須であり、電流監視を採用する場合も併用する。

| 手段 | 前提 | 備考 |
|---|---|---|
| Servo電源ラインの電流監視 | 低側shunt抵抗とESP32 ADC、またはcurrent sense IC | 拘束を直接検知できる。部品追加が必要 |
| 時間baseの強制duty制限 | 追加部品なし | 検知ではなく予防。連続動作時間とduty cycleの上限で無条件に停止する |

正確な部品が確定するまでは、時間base制限を**設計上のfallback**として扱う。
これは実装計画のための最低線であり、**サーボ出力を有効にしてよいという意味ではない**。
有効化の条件は「[サーボ出力を有効化してよい条件](#サーボ出力を有効化してよい条件)」に集約する。

次を確定する。`HW-TBD-020`のfieldであり、**正はfield単位で一つだけ**である
（割り当ては[TBD台帳](tbd-register.md)の表を参照する）。
この文書は安全要件の正本であり、実測値の正本ではない。値をここで確定しない。

| 項目 | 正 | 状態 |
|---|---|---|
| 採用する検知／予防手段 | この文書 | TBD |
| 拘束／過負荷を検知したときの物理動作 | この文書 | TBD（trajectory中止、またはPWM disable） |
| 復帰条件 | この文書 | TBD |
| 電流しきい値と判定時間 | [TBD台帳](tbd-register.md)（実測値） | TBD |
| 連続動作時間の上限 | [TBD台帳](tbd-register.md)（実測値） | TBD |
| Duty cycle窓と上限 | [TBD台帳](tbd-register.md)（実測値） | TBD |

**拘束または過負荷を検知した場合は、必ず物理的に停止する。**
`hardware-safety-policy.md` §6は検知時の停止を要求している。
error codeの返却は報告であって停止ではない。両方を行う。

事象を**2種類に分ける**。混同すると、過熱を防ぐための制限が実効しなくなる。

**(a) 実行時の安全制限の超過** — 実行中の動作そのものが危険源である。停止する。

| 事象 | 物理動作 | 応答（[Protocol](../protocol/esp32-pi-protocol.md)のcode） |
|---|---|---|
| 拘束／過負荷の検知 | trajectory中止またはPWM disable | 専用のfault eventで報告する。`sensor_fault`／`protocol_fault`は使わない。event名とpayload schemaは`PROTO-TBD-014`で確定する |
| 最大連続動作時間の超過 | **実行中のtrajectoryを中止する** | 同上のfault event。原因を`PROTO-TBD-014`のpayloadで拘束／過負荷と区別する |
| Duty cycle上限の超過 | **実行中のtrajectoryを中止する** | 同上のfault event。原因を`PROTO-TBD-014`のpayloadで拘束／過負荷と区別する |

連続動作時間とduty cycleは、電流監視を採用しない場合の唯一の過熱予防手段である
（上記「時間baseの強制duty制限」）。超過しても動作を継続させると、この予防が成立しない。

3事象は同じfault eventで報告するが、**原因は区別できなければならない**。
Piの取るべき動作が異なるためである。拘束は物理的な干渉の除去を要し、
連続動作時間とduty cycleの超過は冷却時間の経過を待てば復帰する。
区別できないと、拘束したまま再試行を繰り返す実装が書ける。

**(b) commandの受理拒否** — 新しい動作を始めないだけで、実行中の動作は危険源ではない。

| 事象 | 物理動作 | 応答 |
|---|---|---|
| 単位時間あたりの受理数を超過 | 新しいtrajectoryを開始しない（実行中は継続） | `rate_limited` |
| 実行中trajectoryによるresourceの一時的な占有 | 同上 | `busy` |
| 値そのものが許容範囲外 | 同上 | `out_of_range` |

いずれの場合も無言で捨てず、machine-readableな理由を返す。
受理上限は、設定値がNのとき1からN番目までを受理し、N+1番目以降を拒否する。

## サーボ出力を有効化してよい条件

**この節が、サーボ出力の有効化条件の正本である。**
他の文書はここを参照し、条件の一部だけを再掲しない。

次の`TBD`が**すべて**解決するまで、サーボ出力を有効にしない。

| TBD | 内容 | 解決しないと起きること |
|---|---|---|
| ~~`HW-TBD-006`~~ | ~~正確なservo model~~ | **解決済み（2026-08-05、TowerPro SG90）。**このgateは満たした。ただしdatasheet値を得ただけであり、**peak／stall電流の実測は`HW-TBD-010`／`HW-TBD-011`で引き続き必要**である |
| `HW-TBD-007` | 電源modelとpower budget | 供給能力が不明なまま駆動し、brownoutとESP32のresetを招く |
| `HW-TBD-009` | backfeed review | サーボ側からESP32へ電流が回り込む経路を検出できない |
| `HW-TBD-010` | 機械的可動域とneutral | calibration済みの最小・neutral・最大位置が無いまま駆動し、機械端へ衝突する |
| `HW-TBD-011` | 速度／加速度制限 | 最大角速度・角加速度が未定のまま駆動する |
| `HW-TBD-026` | SG90の電気的駆動条件（制御logic要件、PWM周期／rate、許容pulse幅範囲） | ESP32の3.3 V出力でSG90のlogic入力を確実に駆動できるかが未確認のまま配線する。PWM周期とpulse幅範囲も一般値で置くことになり、`HW-TBD-010`のcalibrationが基準を持たない |
| `HW-TBD-017`／`PROTO-TBD-010` | 通信断の検知方式（heartbeat source、loss timeout） | 断を検知できず、fail-safeが起動しない |
| `HW-TBD-018`／`PROTO-TBD-013` | 通信断時のfail-safe sequenceと**recovery／reconnect動作** | 断を検知しても取るべき動作が未定。復帰時の再有効化条件も未定なら、断から戻った直後に条件を満たさないまま駆動しうる。stale commandの拒否条件が未定なら、断の前後で受理すべきcommandを判別できない |
| `HW-TBD-019` | 起動時とdriver故障時のサーボ出力状態 | 電源投入直後の挙動が未定。PWM driverの初期化失敗・実行中故障を検知したときの動作も未定 |
| `HW-TBD-027` | `SERVO-PWM`の外部pull-down（`RES-PULL-01`）の抵抗値と本数 | GPIO27はreset時にhigh-Zであり、pull-downが実装されるまでPWM driver初期化前のservoの動きを止められない。`HW-TBD-019`の起動時状態は、この部品が決まらないと確定できない。**抵抗値を決めただけではこのgateは開かない。**購入・実装・reset中のGPIO27がLowであることの実測、の3点を記録するまで閉じたままとする（[TBD台帳](tbd-register.md)の`HW-TBD-027`行）。**文書上の選定完了をもって通電しない** |
| `HW-TBD-020`／`PROTO-TBD-005`／`PROTO-TBD-011`／`PROTO-TBD-012`／`PROTO-TBD-013`／`PROTO-TBD-014` | 実行時のサーボ安全制御（検知／予防手段、電流しきい値と判定時間、連続動作時間、duty cycle窓と上限、検知時の物理動作、復帰条件、秒あたり受理command数、**単一commandの最大変化量**、**command timeout**、**duplicate履歴の保持期間とretry window**、**retired sessionの保持件数と期間**） | 拘束を検知できず、また動作時間、受理数、1 commandあたりの変化量、timeoutの上限が未定のまま駆動する。duplicate履歴（`PROTO-TBD-005`、現在のsession用）とretired session保持（`PROTO-TBD-011`、retired `sid`を`stale_session`で遮蔽する用）は**別モデル**であり、下限はfieldごとに分けて満たす。`PROTO-TBD-005`は**保持期間**が遅延messageの最大生存時間＋再送window以上であること（retry windowはこの期間に収まること）。`PROTO-TBD-011`は**保持期間**が同じく遅延messageの最大生存時間＋再送window以上、かつ**保持件数**がその期間中に起こりうる最大session遷移数以上であること。件数側を決めないと、期間内でも古いsessionが押し出され、遅延した相対移動commandが新規commandとして再実行され二重動作になる。さらに同IDの`sid`生成・衝突回復・`hello`最大retry回数を含むsession回復契約も未解決なら、ACK喪失時の動作を有限に保てないためgateを開かない |

`command timeout`は3つに分かれる。**同じfieldに正を2つ置かない。**

| 対象 | 正 |
|---|---|
| timeoutを設ける要件と、超過時に取る動作 | この文書 |
| timeoutの**実測値** | [TBD台帳](tbd-register.md) |
| どのcommandをstaleとみなすかの拒否条件 | Protocolの`PROTO-TBD-013` |

値だけ決めても、どのcommandをstaleとみなすかが未定なら強制できない。

**`HW-TBD-*`の解決だけではこのゲートは開かない。**上表で`／`区切りで併記したIDは、
field単位でProtocol側が正となるものである（`PROTO-TBD-005`のduplicate履歴保持とretry window、
`PROTO-TBD-010`のheartbeat方式、`PROTO-TBD-011`のretired session保持件数・保持期間とsession回復契約、
`PROTO-TBD-012`のlink負荷管理、
`PROTO-TBD-013`のstale command拒否条件、`PROTO-TBD-014`のfault event schema）。hardware側だけをcloseしても、
heartbeat方式やstale commandの拒否条件、runtime faultの報告形式が未定のまま
サーボ出力を許すことになる。

対応関係の正本は[TBD台帳](tbd-register.md)のHW↔PROTO対応表である。
併記が台帳と食い違った場合は台帳に従い、この表を直す。

対応するProtocol側`TBD`が解決するまで、そのfieldを含む`HW-TBD-*`はcloseしない。
対応表は[TBD台帳](tbd-register.md)のfield単位の正に従う。

ここに挙げた`TBD`は、[TBD台帳](tbd-register.md)側で別の`TBD`にBlockedされているものがある
（例: `HW-TBD-010`は`HW-TBD-007`〜`HW-TBD-009`によりBlocked）。
**列挙したIDを解決するには、その前提となるIDも解決している必要がある。**
ここに現れないIDでも、台帳の依存を辿って未解決であれば、ゲートは通らない。

`HW-TBD-010`と`HW-TBD-011`は、[Hardware Safety Policy](../governance/hardware-safety-policy.md)が
Firmwareの必須制限として要求する値（calibration済みの最小・neutral・最大位置、
最大角速度、最大角加速度）そのものである。これらが未確定なら、
強制すべき制限値が存在しないまま出力を有効にすることになる。

`HW-TBD-007`と`HW-TBD-009`は電源側の条件である。サーボは基板より大きな電流を引くため、
供給能力とbackfeed経路が未確定のまま出力を有効にすると、brownoutでESP32がresetし、
その瞬間のGPIO状態でサーボが動く。servo modelだけを確定しても防げない。

`HW-TBD-020`は実行時の安全制御そのものである。他が解決しても、これらが未定であれば、
機構が拘束されたまま最大torqueを出し続ける状態を止められない。
field単位の正は[TBD台帳](tbd-register.md)に定義しており、一部だけの解決でcloseしない。

加えて、初回動作時は次を満たす。

- 負荷を外すか、意図的に狭い安全範囲を設定する
- 人間が立ち会い、直ちにactuator電源を遮断できる（[Hardware Safety Policy §7](../governance/hardware-safety-policy.md#7-人間の監視が必要な操作)）

この文書の状態`Blocked`は、**有効化ゲートが閉じていること**を表す。
個別の`TBD`は先に解決してよく、その進捗は[TBD台帳](tbd-register.md)を正とする。
ゲートは、列挙したすべての依存が解決するまで閉じたままとする。
ゲートの状態と個別項目の状態を同じ記述で扱わない。

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

## 起動時とdriver故障時の動作

無負荷試験後に次を決定する。いずれも`HW-TBD-019`で追跡する。

- PWM driver初期化前のGPIO state
- PWMをdisabledで開始するか、calibration済みneutralで開始するか
- Actuator enableまでのdelay
- Pi未接続時の動作
- Watchdog、panic、brownout reset後の動作
- **PWM driverの初期化失敗または実行中の故障を検知したときの動作**

最後の項目は起動時に限らない。[Hardware Safety Policy](../governance/hardware-safety-policy.md)は
「resetまたはdriver故障時の定義済み動作」をFirmwareの必須強制項目としており、
resetだけを決めても要求を満たさない。故障を検知できるか、検知できない場合に
何を安全側の既定とするかも、この項目に含める。

承認されるまで、安全状態は「未検証の動作出力を行わない」とする。

## 通信断時動作

機械試験後に、次のいずれかを選択する。

- 短時間保持し、低速でneutralへ移動してからdisableにする
- 現在位置を保持する
- 直ちにPWMをdisableにする

PWMをdisableにしたときに首が落下したり予期せず動いたりするかを考慮して選択する。

次を記録する。各行は、その行に対応するhardware側またはprotocol側のTBD IDで追跡する。
追跡IDは行ごとに異なり、`PROTO-TBD-013`のようにprotocol側が正となる行もある。
表の`TBD`だけで管理しない。

| 項目 | 値 | 追跡ID |
|---|---|---|
| Heartbeat source | TBD | HW-TBD-017／PROTO-TBD-010 |
| Loss timeout | TBD | HW-TBD-017 |
| 選択したfail-safe sequence | TBD | HW-TBD-018 |
| Recovery／reconnect動作 | TBD | HW-TBD-018（復帰時に受理するcommandは PROTO-TBD-013） |
| Stale command rejection | TBD | PROTO-TBD-013 |

heartbeat source、loss timeout、fail-safe sequence、recovery／reconnect動作は、
サーボ出力を有効化するための**必要条件の一部**である。十分条件ではない。

recovery／reconnect動作は`HW-TBD-018`の範囲に含める。fail-safe sequenceだけを
確定して`HW-TBD-018`をcloseすると、**復帰時の再有効化条件が持ち主のないまま残り**、
サーボ出力のゲートを素通りする。断の検知、断時の動作、復帰時の動作の3つが揃うまでcloseしない。

`hardware-safety-policy.md` §8は、断の検知だけでなく、検知後に取る動作の確定も要求している。
断を検知できても取るべき動作が未定であれば、fail-safeは成立しない。
検知手段だけが決まった状態を、出力有効化の条件として扱わない。

有効化条件の全体は「[サーボ出力を有効化してよい条件](#サーボ出力を有効化してよい条件)」を参照する。
この節の3項目だけを満たしても、有効化してよいことにはならない。

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
- [ ] **定常電流**でingressとconnectorの定格を確認した（[power-budget.md](power-budget.md)の`ingressの電流制限`。定格は熱の制限のため、判定量は定常電流である）
- [ ] **peak時**の5 V／3.3 Vの電圧droopを測定し、brownoutとresetが起きないことを確認した（peakはこの確認にのみ使う）
- [ ] **承認範囲内の最悪動作**でservo railの**定常電流**を実測し、`動作制限`表の`最大連続電流`の予算（250 mA）以下であることを確認した。超える場合は可動域・速度・duty cycleを締めて再測定した。**[power-budget.md](power-budget.md)でingressの定格を上げただけでは、この項目を合格にしない**（予算はこの表がfirmwareへ渡す値であり、経路側の定格とは別の量である）
- [ ] 予算そのものを変えた場合は、[power-budget.md](power-budget.md)の`変換基板に必要な定格の見積もり`にある**正式改訂の手順**（両文書の値を同時に改訂し、経路部品とgate値を決め直し、受け入れchecklistを通し直す）を踏んだ。**測定値に合わせて判定を緩めていない**
- [ ] そのとき**強制していた**可動域、最大速度、最大加速度、最大連続動作時間、最大duty cycleを`動作制限`表へ記録した（予算を守らせているのはこれらの値であり、電流の設定値ではない）
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
| 2026-08-05 | 1 | `HW-TBD-006`（正確なservo model）が[TBD台帳](tbd-register.md)で解決済み（TowerPro SG90）となったため、有効化gate表の該当行を解決済みとして打ち消し、依存例（`HW-TBD-010`のBlocked元）を`HW-TBD-007`〜`009`へ更新した。**gate自体は開いていない。**残る`TBD`が未解決であり、peak／stall電流の実測も`HW-TBD-010`／`011`で必要である |
| 2026-08-08 | 2 | `サーボ識別情報`表がメーカー・model・定格電圧をすべて`TBD`のまま残しており、[HW-TBD-006](tbd-register.md)を解決済みとした本文（Revision 1）および[hardware-bom.md](hardware-bom.md) SERVO-01と矛盾していた。安全確認の作業者がservo modelを未確定と誤認する状態だったため、確定済みの値（TowerPro SG90、4.8–6 V、datasheet link）を反映した。**駆動条件は未確定のまま残す。**pulse幅、stall電流、logic閾値は実機のcalibrationと測定で決めるものであり、datasheetの代表値を確定値として採らない旨も明記した。識別情報の正本が`hardware-bom.md`であることも冒頭に記した |
| 2026-08-09 | 3 | 自己レビューで検出: [power-budget.md](power-budget.md)がservoへ連続電流250 mAを予算として割り当て、その担保をこの文書のtrajectory制限に委ねていたが、**この文書側に該当する制限が無かった**。電源側だけが予算を持ち、firmwareへ渡っていない状態だった。動作制限表へ`最大連続電流`の行を追加し、可動域・速度・duty cycleをこの予算の内側で決めることを明記した |
| 2026-08-09 | 4 | 受け入れchecklistに`Peak電流から電源容量を決定した`が残っており、[power-budget.md](power-budget.md)の現行規則（ingressとconnectorの定格判定は定常電流で行い、peakは電圧droop・brownout・resetの確認にのみ使う）と矛盾していた。2項目に分けて揃えた |
| 2026-08-09 | 5 | [PR #64](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)のレビュー指摘を反映（[#65](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/65)の6件目）。Revision 3で`最大連続電流`に250 mAの予算を置いたが、`設定可能なhard bound`が`TBD`のままで、受け入れchecklistもtrajectoryに250 mA以下を要求していなかった。**予算があるだけで強制されていなかった。**hard bound列に「firmwareへ直接は設定せず、強制点は可動域・速度・加速度・連続動作時間・duty cycleである」ことを明記し、受け入れchecklistへ「承認範囲内の最悪動作での定常電流の実測」と「そのとき強制していた値の記録」の2項目を追加した |
| 2026-08-09 | 6 | Revision 5に対する[PR #64](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)のレビュー指摘を反映。受け入れchecklistが「可動域・速度・duty cycleを締めて再測定するか、ingressの定格を上げる判断へ戻す」としており、**ingressの定格を上げるだけで250 mAの予算違反を通せる書き方**になっていた。予算はこの表がfirmwareへ渡す値であり経路側の定格とは別の量であるため、既定の対処を「制限を締めて再測定する」に限定し、予算そのものを変える場合は[power-budget.md](power-budget.md)の正式改訂の手順を踏むことを別項目として追加した |
| 2026-08-10 | 7 | [#72](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/72)の全数照合で、`サーボ識別情報`表の`制御logic要件`・`PWM周期／rate`・`許容最小／最大pulse`が本文で`TBD`のまま[TBD台帳](tbd-register.md)に行を持たないことが判明したため、`HW-TBD-026`として登録し有効化gate表へ追加した。あわせて、`SERVO-PWM`の外部pull-down（`RES-PULL-01`）の抵抗値未選定を`HW-TBD-027`として登録し、`HW-TBD-019`の前提としてgate表へ追加した |
| 2026-08-11 | 8 | [PR #99](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/99)のレビュー指摘を反映。`HW-TBD-027`の有効化gate行が抵抗値と本数の未確定だけを条件としており、**値を決めた文書だけでgateが開きうる**書き方だった。pull-downが実装されていなければGPIO27はreset時にhigh-Zのままであり、文書上の選定完了は物理的な保護を何ら与えない。購入・実装・reset中のGPIO27がLowであることの実測、の3点を記録するまでgateを閉じたままにすることを明記した |
