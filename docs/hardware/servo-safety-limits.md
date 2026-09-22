# Servo Safety Limits

> 状態: Blocked — 正確なservoとmechanical assemblyが必要
> 正本とする情報: サーボの電気的制限、機械的制限、動作制限、fail-safe動作

## 確定しているproject規則

- サーボにはESP32の電源pinではなく、外部電源経路を使用する。
- ESP32とサーボ電源のGNDを意図的に共通化する。
- ESP32はRaspberry Piから独立して、hard motion limitを強制する。
- AIが生成したcommandやdebug commandでもhard limitを迂回できない。**`bench-servo-test-17`
  featureによる単発bench試験は、この規則が禁じる迂回に該当する（[承認の状態](#承認の状態)参照）。**
- 初回動作では負荷を外すか、意図的に狭い安全範囲を使用する。

## サーボ識別情報

部品の識別情報の**正本は[hardware-bom.md](hardware-bom.md)のSERVO-01**である。
この表はそこから引いた値を、安全確認の作業者がここだけで読めるように再掲したものであり、
値をこの文書で確定させない。BOM側を更新したらこの表も合わせる。

| 項目 | 値 | 根拠 |
|---|---|---|
| メーカー | TowerPro | 現物ラベル（`TOWER PRO Micro servo 9g SG90`）。[HW-TBD-006](tbd-register.md)で解決済み |
| 正確なmodel／suffix | SG90 | 同上 |
| データシートrevision | **公式のdatasheet PDFは無い。**一次資料は公式製品ページの仕様表だけである（[SG90 Digital](https://towerpro.com.tw/product/sg90-7/)、[SG90 Analog](https://towerpro.com.tw/product/sg90-analog/)。revision表記なし）。従来引いていた[Soldered_101246.pdf](https://www.mouser.com/catalog/specsheets/Soldered_101246.pdf)は**TowerPro発行と確認できず、取得もできない**（2026-08-24。正はBOM） | メーカーの製品ページ |
| 定格電圧範囲 | 4.8–6 V | **Digital側ページのQ&A欄にある`admin`（メーカー側）の回答（2021-11-07）にのみ由来する。**仕様表の記載は`Operating voltage: 4.8v`だけで、Analog側ページは`Reviews (0)`である。**どちらの品かが未確定であり、variantに依存しない定格として扱わない。****そして`PSU-SERVO-01`が供給する5 Vは、この2品の仕様表の`4.8v`より上である。****なお同名の3品目`SG90 360 degree`は仕様表が`Operating voltage: 4.8v-6V`を載せており、どの品かによって仕様表の裏付けの有無が変わる**（連続回転で寸法も異なるため別物の可能性が高い）。**2026-08-28に現物の高さを実測した（[Issue #257](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/257)。27 mm）。**`23×12.2x29mm`（Digital／Analog）側に近く、`23×12.5x22mm`（360 degree）とは5 mm差があるため、**`SG90 360 degree`ではないと判定した。**`Digital`と`Analog`は寸法が同一で、この実測では分離できない。未確定のまま残る。**5 Vを入れてよいかは[tbd-register HW-TBD-035](tbd-register.md)で追跡する。この表では決めない**（2026-08-24。正はBOM） |
| 無負荷電流 | **TBD** | 測定（**一次資料に記載が無い**。正はBOM） |
| 動作電流 | **TBD**（0.5–2 Aと負荷依存で幅が広い。**`データシート値`という表記を2026-08-24に外した。**この値もDigital側ページのQ&A欄にある`admin`（メーカー側）の回答（2021-11-07）にのみ由来する（Analog側は`Reviews (0)`）。**値は変えていない。**確定は`HW-TBD-010`／`011`の範囲である） | 測定（**一次資料に記載が無い**。正はBOM） |
| Stall／peak電流 | **TBD**（実測必須。[HW-TBD-010／011](tbd-register.md)、`power-budget.md`測定計画） | 測定（**一次資料に記載が無い**。正はBOM） |
| 制御logic要件 | **TBD**（ESP32のGPIOは3.3 V。SG90のlogic閾値を現物確認するまで確定しない）。**2026-08-24に、一次資料にlogic閾値の記載が無いことを確認した**（Digital／Analog両ページ）。**したがって3.3 V driveの可否をこの資料では判定できない。**level shifterの要否も決まらない | **一次資料に記載が無い**（[tbd-register HW-TBD-026](tbd-register.md)） |
| PWM周期／rate | **TBD**（50 Hzが一般値だが、この表の確定値として採らない）。**2026-08-24に、一次資料に記載が無いことを確認した。**50 Hzは出所を持たない | **一次資料に記載が無い**（[tbd-register HW-TBD-026](tbd-register.md)） |
| 許容最小pulse | **TBD**。**2026-08-24に、一次資料に記載が無いことを確認した** | **一次資料に記載が無い。**残るのは無負荷試験（[tbd-register HW-TBD-026](tbd-register.md)） |
| Neutral pulse | **TBD** | Calibration |
| 許容最大pulse | **TBD**。**2026-08-24に、一次資料に記載が無いことを確認した** | **一次資料に記載が無い。**残るのは無負荷試験（[tbd-register HW-TBD-026](tbd-register.md)） |
| Dead band width | **1 us**（`SG90 Digital`／`SG90 Analog`の両ページ共通）。**確定した運用値ではない。****同名の3品目`SG90 360 degree`については確認していない。**この`1 us`は2品の仕様表だけを出所とし、**3品目は2026-08-28の実測（27 mm、[Issue #257](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/257)）で除外できたが、`Digital`と`Analog`の分離はできていないため、variantに依存しない値として扱わない**（`定格電圧範囲`行と同じ扱いである。[tbd-register HW-TBD-035の証拠契約](tbd-register.md#hw-tbd-035の証拠契約)の条件1）。一次資料が示す制御信号の電気仕様であり、**許容pulse幅範囲の代わりにならない**（範囲の内側の分解能に当たる量である）。**この値だけでpulse幅範囲を導かない** | 公式製品ページの仕様表（[SG90 Digital](https://towerpro.com.tw/product/sg90-7/)、[SG90 Analog](https://towerpro.com.tw/product/sg90-analog/)。2026-08-24取得）。**正は[tbd-register HW-TBD-026](tbd-register.md)である**（`SERVO-01`にはこの項目を載せる列が無い） |

**modelは確定したが、駆動条件はまだ確定していない。**上表で`TBD`が残る項目は、
一般的なhobby servoの値やdatasheetの代表値を確定値として使用しない。
とくにpulse幅とstall電流は、実機のcalibrationと測定で決める。

**一次資料に記載が無いことが確認できた項目は、記載が無いという結論として記録する。**
無い記載を一般値で埋めない。一次資料から得られた値を記録する場合も、
**それが電気的な仕様であって確定した運用値ではないことを、値のそばに明記する。**

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
| `HW-TBD-035` | **servoへ入れてよい電圧。**`PSU-SERVO-01`は5 Vを供給するが、仕様表の`Operating voltage`は`4.8v`単独であり、`4.8–6 V`はメーカーのQ&A回答1文だけに依る | servoへ入れてよい電圧が未確認のまま5 Vを供給する。**過電圧側の余裕が不明のまま通電することになる** |
| `HW-TBD-026` | SG90の電気的駆動条件（制御logic要件、PWM周期／rate、許容pulse幅範囲） | ESP32の3.3 V出力でSG90のlogic入力を確実に駆動できるかが未確認のまま配線する。PWM周期とpulse幅範囲も一般値で置くことになり、`HW-TBD-010`のcalibrationが基準を持たない |
| `HW-TBD-017`／`PROTO-TBD-010` | 通信断の検知方式（heartbeat source、loss timeout） | 断を検知できず、fail-safeが起動しない |
| `HW-TBD-018`／`PROTO-TBD-013` | 通信断時のfail-safe sequenceと**recovery／reconnect動作** | 断を検知しても取るべき動作が未定。復帰時の再有効化条件も未定なら、断から戻った直後に条件を満たさないまま駆動しうる。stale commandの拒否条件が未定なら、断の前後で受理すべきcommandを判別できない |
| `HW-TBD-019` | 起動時とdriver故障時のサーボ出力状態 | 電源投入直後の挙動が未定。PWM driverの初期化失敗・実行中故障を検知したときの動作も未定 |
| `HW-TBD-027` | `SERVO-PWM`の外部pull-down（`RES-PULL-01`）の抵抗値と本数 | GPIO27はreset時にhigh-Zであり、pull-downが実装されるまでPWM driver初期化前のservoの動きを止められない。`HW-TBD-019`の起動時状態は、この部品が決まらないと確定できない。**抵抗値を決めただけではこのgateは開かない。**選定・購入・実装・reset中の実測の4点を、[HW-TBD-027の証拠契約](tbd-register.md#hw-tbd-027の証拠契約)が定める記録先へ残すまで閉じたままとする。**判定閾値は`HW-TBD-026`が決まるまで確定しないため、027は026より先に閉じられない。文書上の選定完了をもって通電しない** |
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

## 承認の状態

**この節だけが、初回動作手順の承認状態を持つ。**他の箇所（節見出し、Revision履歴、
各step、firmware側のdoc comment、commit message）は状態を再掲せず、この節を
参照する。状態が変わったら、この節だけを更新する。

1. **2026-09-22、ユーザーがこの手順1回分に限り承認した**（[Issue #17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)）。
2. **ただし、この承認は次の3点を明示せずに得たものだった（PMの不備）。**
   - [Hardware Safety Policy §6](../governance/hardware-safety-policy.md#6-サーボ)と
     [AGENTS.md](../../AGENTS.md)が禁じる「debug経路によるservo安全制限の迂回」に、
     この試験が該当すること。
   - §6が要求する「電流制限を設定でき十分な定格を持つ電源」が、この試験に無いこと。
   - SG90のstall電流はメーカー資料に無く`TBD`であり、承認依頼文にあった
     「0.5–2 A」は動作電流の目安であって、stall電流ではないこと。
3. **2026-09-22、上記3点に加えて次の点を明示した再確認を送った。**この再確認は
   Issueやpull requestのcommentではなく、この変更（PR）を作成したAI session と
   ユーザーとの対話の中で直接行った。全文の記録は、この節と、この変更を含む
   commitおよびpull requestが持つ。
   - 品種（TowerPro SG90のDigital／Analog）を識別していないこと。
   - サーボの機械的な真の中心位置（neutral）は未計測であり、firmwareが使う
     「90度を中央とする」は規約上の仮定にすぎないこと。
   - PWMの周波数・パルス幅の範囲は、メーカー資料に記載が無いため一般的な
     hobby servoの慣行値を使っていること。
   - サーボの電源をRaspberry Piと同じM-12001から分岐する**構成自体**は
     2026-08-05に人の判断として確定済みだが（[hardware-bom.md](hardware-bom.md)
     `PSU-SERVO-01`のRevision 5）、分岐点の接続部材と上流の過電流保護部品
     （`PROT-OC-01`）の物理的な実装・検証状況は、下記残余riskのとおり
     `通電前の現物確認`まで確認できないこと。
4. **ユーザーから「進めてよい」の回答を得た。**この回答は2026-09-22の当初承認
   （上記1）の範囲（本手順1回分に限る、単発command、繰り返しなし）を変えずに、
   その不備（上記2）を補うものである。**以後、この節の状態は「進めてよい」として
   扱う。**この状態は、`単発動作`が1回実行されるか、`停止基準`により中止される
   まで有効であり、実行後は再び`再武装`と改めての承認が要る（`再武装`step参照）。
   結果が「進めてよい」でなくなった場合、`gate確認`は実行の承認を出さない。

**gate自体（[サーボ出力を有効化してよい条件](#サーボ出力を有効化してよい条件)）は
開いていない。**上記の承認は、gateの「次の`TBD`が**すべて**解決するまでサーボ出力を
有効にしない」という原則に対する例外としてユーザーが個別に認めたものであり、
gate表のいずれの行も解決していない。動かなかった場合、再試行する場合は、
そこで止めて改めてこの節の状態を
更新する。

## 初回動作の実行手順（人間とAIの作業順序）

[Calibration手順](#calibration手順)の2〜7を、`SERVO-01`（TowerPro SG90）とこのリポジトリの
firmware（`firmware/esp32/src/servo.rs`、
`firmware/esp32/src/config.rs`）に沿って人間とAIの
作業順序へ具体化したものである。**別文書に分けない。**人間はAIの指示のもとで動く。

この手順は、Calibration手順のうち次を**満たさない**（下記`通電前の現物確認`で
信号線とサーボを接続するため、手順4は単独stepとして行わない）。理由の詳細は
残余riskの該当項目を参照する（ここへ再掲しない）。

- **手順4（waveform確認）:** 満たさない。したがって[受け入れchecklist](#受け入れchecklist)の
  「サーボ接続前にPWMを測定した」は、この試験だけでは満たされない。
- **手順6（検証済みneutral候補）:** 満たさない。`config::SERVO_ANGLE_CONVENTION_NEUTRAL_DEG`
  （90度）は規約上の仮定であり、検証済みの値ではない（`HW-TBD-010`）。
- **手順3のうち「安全な電流制限」:** 満たさない。詳細は下記残余riskの
  「この試験には、故障電流を制限する手段が無い」を参照する。

**実行条件はここでは再掲しない。**[承認の状態](#承認の状態)が「進めてよい」に
なるまで、以下のstepは`ホーン除去`以降へ進まない（`gate確認`参照）。

**この手順は`HW-TBD-010`（機械的可動域とneutral）の実測に着手する最初のstepである**
（`HW-TBD-010`自体は`HW-TBD-007`〜`009`によりBlockedのまま。closeするものではない。
経緯はPR本文参照）。

残余riskとして次を明示する。**gate表の他の行（`HW-TBD-007`／`009`／`011`／`019`／
`020`／`026`／`027`／`035`ほか）もこの承認だけでは解決しておらず、Blockedのまま
残る**（`HW-TBD-017`／`018`は本手順にPi通信linkが無いため該当しない）。

- `HW-TBD-035`（servoへ入れてよい電圧）は、[tbd-register.md](tbd-register.md)の
  `HW-TBD-035`行が残る解決手段として(1)メーカーへの問い合わせ、(2)4.8 V供給への変更、
  (3)監視下で5 Vを入れて観察する、の3つを挙げており、**この試験は(3)に近い。**ただし
  同行は「(3)は通電を伴うため`HW-TBD-007`／`009`のgateに従う」と明記しており、
  **その2行は未解決のままである。**したがってこの試験を実行しても、
  `HW-TBD-035`は単独では解決しない。Digital／Analogを識別しないというユーザー決定に
  より、乙（メーカー自身による仕様書以外の記述）の条件2（品種特定）は満たせないままで
  あり、5 Vは、メーカーQ&A回答（品種未特定）の範囲内というだけの判断である。
  **この「識別しない」という決定と、[tbd-register.md](tbd-register.md)の`HW-TBD-026`が
  追記する「Digital／Analogの分離は`HW-TBD-035`の判断には影響しない（どちらも仕様表は
  `4.8v`単独）」は、異なる主張であり矛盾しない。**後者が言うのは「識別できても、
  仕様表（甲）の記載が両品とも`4.8v`である事実は変わらない」ということであり、
  前者が言うのは「識別できれば、Digital側ページのQ&A回答（乙）をその個体に
  適用してよいかどうかの条件2を満たせた可能性がある」ということである。**識別しない
  という決定は、後者の可能性（乙の条件2を満たす経路）を閉じるものであり、前者
  （甲がそもそも4.8vしか無いという事実）には影響しない。**
- `HW-TBD-009`（backfeed）はservoの内部回路図が非公開のため机上reviewで排除できない。
  **ESP32とservoの電源を分離した構成（下記`電源準備`）は、片方だけ電源が落ちる状況をむしろ生みうる。**
  この経路を遮断する対策（追加部品等）は無く、人間の監視だけに委ねている。
- `HW-TBD-026`（logic閾値・PWM周期・pulse幅）は、[Hardware Safety Policy](../governance/hardware-safety-policy.md)の`5項目以外の扱い`対応表で
  一次資料または実測を要する側に分類される。**「5項目に効かないので一般値でよい」
  という扱いはしていない。**一般的な慣行値を使うこと自体が、この承認が受容した
  未解決の、一次資料または実測を要する項目である。
- **信号（PWM pulse）を止めても、servoが駆動を止める保証は無い。**Analog servoなら信号停止で
  駆動が終わると期待されるが、Digital servoは最後の位置を保持し続ける実装がある。品種を
  識別しないため、どちらとも言えない。メーカー資料にsignal loss時の挙動の記載は無い。
  **拘束を確実に止める手段は信号停止ではなく、人間による外部5 V電源の遮断である。**
- **`PSU-SERVO-01`はservo専用電源ではない。**[hardware-bom.md](hardware-bom.md)の同行に
  よれば、`PSU-PI-01`と**同一のM-12001アダプターから分岐したrail**であり（状態
  `Selected（入力源は確定。分岐後の部品はTBD）`）、分岐後の配線・端子・電流制限手段は
  BOM上`TBD`のままである。**この試験でPiが同じM-12001に接続されているかどうかを、
  `通電前の現物確認`で人間が確認し記録する。**接続されている場合、servoの過電流はPi側の電圧降下・
  brownoutを経路として引き起こしうる（残余riskに追加）。接続されていなければこの経路は
  生じない。**どちらであるかをこの文書だけでは決められない。**
- `hardware-bom.md`の`PROT-OC-01`（Bourns MF-R135、`Ihold` 1.35 A／`Itrip` 2.70 A）は
  5 V ingressの過電流保護として`Selected`だが、**この部品がこの試験の物理経路に
  実際に入っているかはBOMからもこの文書からも確認できない。**入っていれば、servoの
  動作電流（`TBD`。目安は0.5–2 A、上記参照）は保持帯とtrip帯の間に架かりうる。
  `通電前の現物確認`で人間が現物を確認して記録する。
- `動作制限`表が定める**最大連続電流の予算は250 mA**である。SG90の動作電流
  （`TBD`。目安は0.5–2 A）はこの予算を大きく超える見込みである。**この試験は可動域・速度・duty
  cycleでこの予算内に収める調整を行っていない**（そのための実測自体が`HW-TBD-010`／
  `011`で未完了である）。250 mA予算の遵守はこの試験の対象外であり、電流を制限する
  手段は人間による外部5 V電源の監視・遮断だけである。
- [Hardware Safety Policy §6](../governance/hardware-safety-policy.md#6-サーボ)は、
  最初のPWM出力前に行う7項目（正確なservo model確認、電圧・電流・PWM周期・pulse
  width要件の確認、hornまたは機械負荷の除去、**故障電流を制限する手段の明記**
  （手段が無い場合は無いことと代わりに何が守るのかを書く。2026-09-22改定。
  [#454](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/454)）、
  狭いpulse範囲と可動域の設定、直ちに操作できる電源遮断手段、手や
  cableを動作範囲の外へ置くこと）と、「debug commandでもこれらの制限を迂回しては
  ならない」という条件下でfirmwareが強制すべき11項目（calibration済み位置、
  速度・加速度制限、受理command数、連続動作時間とduty cycle、拘束検知時の停止、
  通信断時・reset時の定義済み動作）の両方を定めている。[AGENTS.md](../../AGENTS.md)
  「ハードウェア安全」も独立に「サーボ安全制限をデバッグ経路からも迂回させない」と
  定める。**`bench-servo-test-17` featureによる単発bench試験は、11項目のうち
  calibration済み位置・neutral位置・calibration済み最大位置・最大角速度・最大角加速度・
  受理command数の制限・連続動作時間とduty cycleの強制・拘束検知時の停止・通信断時の
  定義済み動作・reset時の定義済み動作の10項目を実装しない。**残る1項目「最大command
  範囲」は、`Sg90::move_to_angle_once`が`SERVO_FIRST_MOTION_MAX_DEVIATION_DEG`で
  角度をclampする形で部分的に対応するが、この値自体が一次資料にも実測にも基づかない
  暫定値である（`config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG`のdoc参照）ため、
  §6が要求する水準を満たす実装とは言えない。**加えて7項目のうち「電圧・電流・PWM
  周期・pulse width要件の確認」（`HW-TBD-026`が未解決）も満たしていない。これは
  両文書が明示的に禁じる「debug経路によるservo安全制限の迂回」に該当する。**
  承認の状態は[承認の状態](#承認の状態)を参照する（ここへ再掲しない）。
- **この試験には、故障電流を制限する手段が無い。**（2026-09-22、
  [Hardware Safety Policy §6](../governance/hardware-safety-policy.md#6-サーボ)は
  「電流制限を設定でき十分な定格を持つ電源を使用する」という要求から、「故障電流を
  制限する手段を明記する。手段が無い場合は、無いことと、代わりに何が守るのかを書く」
  という要求へ書き換わった（[#454](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/454)）。
  この項目は、その書き換え後の要求に対する回答である。**
  [power-budget.md](power-budget.md)の段階B-2b（`DISP-01`等の周辺module3点を
  外部3.3 V電源から給電する段階）は、「電流制限値の根拠と上限を定めてから」
  進める条件を明記しており、**その根拠と上限が定まっていないという同種の未達が
  LCD側にも存在する。**（正確には、LCD側の未達は「制限値の根拠と上限が無いこと」
  であり、この試験の未達は「制限付き電源そのものを使っていないこと」である。
  形は完全に同一ではないが、**どちらも故障電流を制限する手段を確立していないという点で
  共通する。**）**LCDはこの理由で段階B-2bを止めているのに
  対し、この試験は止まらずに進む。**扱いが異なることを明示する。実際に電流を
  制限しうるのは、
  [hardware-bom.md](hardware-bom.md)の`PROT-OC-01`（PTC、Bourns `MF-R135`。
  `Ihold` 1.35 A／`Itrip` 2.70 A。値の正は同文書、ここへ再掲しない）が経路に
  物理的に入っている場合だけである。**入っているかは`通電前の現物確認`で人間が確認するまで
  分からない。**入っていなければ、電流を制限するものは無く、人間の手動遮断だけが
  残る。**入っていても「2.70 Aでtripするから安全」とは言えない。**`MF-R135`の
  `Max. Time to Trip`は5×`Ihold`（6.75 A）で7.3秒であり、**動作電流の目安である
  0.5–2 A付近の電流ではtripまでの時間ははるかに長い**（一般にPTCのtrip時間は電流が低いほど
  長くなる。具体的な秒数は`hardware-bom.md`にも記載が無く、この文書では算出しない）。
- この試験がもたらしうる最悪の事態: SG90の**動作電流**は、上表`サーボ識別情報`の
  `動作電流`行によれば**`TBD`のまま**（0.5–2 Aと負荷依存で幅が広く、確定は
  `HW-TBD-010`／`011`の範囲）であり、値の出所もDigital側ページのQ&A欄の`admin`
  回答に限られ、variantに依存しない定格として扱わない。**この0.5–2 Aという幅は
  確定値ではなく、メーカー回答由来の目安にすぎない。**stall電流はメーカー資料に無く、
  `TBD`のまま実測が
  必要である**（同表`Stall／peak電流`行、`HW-TBD-010`／`011`）。一般にDCモーターの
  stall電流は動作電流より高いため、**0.5–2 Aを最悪値として扱わない。**拘束が続けば
  少なくともこの程度以上の電流が流れ続け、サーボ内部で発熱し、樹脂ギアの変形・溶融が
  起こり得る。**煙が出る可能性まで否定できない。**Piが同じM-12001に接続されている
  場合はPi側への影響も上記のとおり否定できない。これ以上先は資料から判断できない。
  このriskを限定しているのは、人間がいつでも外部5 V電源を手動遮断できる状態を
  維持することだけである。
- **`DISP-01`の許容電流上限は`HW-TBD-024`が未解決のままである。**
  [#451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)より前、
  `run_display_bringup`は既定buildでも常時実行され`lcd.backlight_on()`を呼んでいた。
  **`#451`で同関数は`bringup-display-13` feature（既定off）付きbuildだけの経路になり、
  この試験のbuild（`--features bench-servo-test-17`）はそのfeatureを付けないため、
  firmwareがbacklightを点ける経路は無い。**それでも`HW-TBD-024`は解けていない。
  **`DISP-01`を接続しないことを引き続き求める。**確認は下の`通電前の現物確認`(d)で行う
  （backlight以外に残る理由もそちらが持つ。**ここへ再掲しない**）。

**対象部品の識別（Digital／Analog）は行わない**（2026-09-22、ユーザー決定。
「識別できた」のではない。経緯は[tbd-register.md](tbd-register.md)の`HW-TBD-026`が正本）。

**この手順のstepは名前で参照する。番号は目次の役割だけで、本文中の相互参照には
使わない**（stepを1つ足すたびに番号がずれ、参照が壊れることを防ぐため）。

**決定事項（reviewer・実行者は以下を前提として読む。蒸し返さない）**

- 拘束を止めるのは人間による外部5 V電源の遮断であり、`arm delay`や`露出時間`の
  待機時間そのものではない。
- `arm delay`（既定10秒）は安全な秒数の主張ではなく、人間がservo電源を投入する
  ための運用上の猶予である。安全要件5項目のどれにも効かない。
- SG90のstall電流はメーカー資料に無く`TBD`である。0.5–2 Aはメーカー回答由来の
  **動作電流**の目安であり、stall電流ではない。stall電流は一般に動作電流より高い。
- `通電前の現物確認`（(a)(b)(c)）はgateではなく記録である。結果がどうであれ
  試験を中止しない（中止基準は`停止基準`が別に定める）。**(c)（`RES-PULL-01`未実装）
  でも続けてよいと判定済み。**`Sg90::new`が呼ばれる（`起動とarm delay`終了後）まで
  GPIO27は未configuredのままであり、`起動とarm delay`の間（servoへ5 V電源が入る
  window）を含めて未定義状態が続く。**この未定義windowの長さや servoの給電有無に
  関わらず安全側であることは、[gpio-assignment.md](gpio-assignment.md)の
  2026-09-06の机上判定（周期的edgeを持たないDC levelは有効なPWM pulseとして
  復号されない。servoの接続有無・給電有無に依存しない）が根拠であり、`(c)`の
  実装状況では変わらない。**一般運用まで安全とは主張せず、`gpio-assignment.md`の
  「必須」要求は変えない。

1. **`gate確認`（人間）** gateの解決状況を[TBD台帳](tbd-register.md)で確認する。
   **加えて、[承認の状態](#承認の状態)が「進めてよい」になっていることを確認する。**
   なっていない間は、`ホーン除去`以降へ進まない。
2. **`ホーン除去`（人間）** サーボのホーンを外す（未装着なら不要）。軸が自由に
   回ることを確認する。
3. **`通電前の現物確認`（人間）** `SERVO-PWM`（GPIO27）からサーボの信号線、外部
   5 V系（`PSU-SERVO-01`）からサーボの電源線、ESP32とサーボ電源側のGNDを接続する
   （電源線は接続するが、電源自体はまだ入れない）。接続後、これらが共通化されて
   いること、ESP32の電源pinからサーボへ給電していないことを目視で確認する。
   **加えて次の4点を確認し、結果をAIへ伝える（`記録`でAIが
   [experiment-log.md](experiment-log.md)へまとめて記録する）。**
   (a) この試験でRaspberry Pi（`PSU-PI-01`）が`PSU-SERVO-01`と同じM-12001に接続されて
   いるか。(b) `PROT-OC-01`（過電流保護PTC）がこの経路に物理的に入っているか。
   (c) `RES-PULL-01`（GPIO27の外部pull-down、4.7 kΩ）が現物に実装されているか。
   (d) `DISP-01`（MSP2807）がESP32の`3V3` pinへ接続されていないことを確認する。
   `HW-TBD-024`（module側の許容電流上限）が未解決であり、接続されていると
   ESP32へのUSB接続と同時に`DISP-01`のlogic側へ給電される。
   **backlightについては、この試験のbuildにfirmware側の点灯経路が無い**
   （[#451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)。
   `run_display_bringup`は`bringup-display-13` feature付きbuildだけが呼び、
   手順`build`はそのfeatureを付けない）。**ただし(d)の確認はそれでも省かない。**
   `HW-TBD-024`が未解決であること自体は変わらず、GPIO4がreset中Lowへ確定することも
   ESP32側の信号レベルまでしか確認できていない
   （[gpio-assignment.md](gpio-assignment.md)の`信号inventory`の`LCD-BL`行、
   [experiment-log.md](experiment-log.md)の`EXP-011`。**ここへ再掲しない**）。
   **(a)(b)(c)(d)の結果は残余riskの該当項目を変えるが、実行可否は変えない**
   （上記「決定事項」参照）。
4. **`電源準備`（人間）** サーボの電源をまだ入れない。ESP32側だけ電源を入れられる
   状態にする（ESP32はPCのUSBから給電する。servoの外部5 V系とは電源を分離する）。
   手やcableを、サーボが動きうる範囲の外へ置く（[Hardware Safety Policy §6](../governance/hardware-safety-policy.md#6-サーボ)のPWM出力前7項目の1つ）。
5. **`build`（AI、flashは人間）** `cargo build --locked --features bench-servo-test-17`で
   AIがfirmwareをbuildし、ESP32 Flash／HIL profileの端末で人間がflashする（`espflash`。
   AIがこの端末を持たない場合、この工程は人間が行う）。このfeatureを付けない通常buildでは
   `crate::servo`は`main()`から呼ばれず、GPIO27は駆動されない。build検証の状況は
   PR本文参照。
   `run_servo_bench_test`は手動triggerを持たずESP32起動のたびに呼ばれるため、
   **NVSへ実行済みflagを実際に動かす前にcommitし**、resetのたびの再実行を防ぐ
   （それでも書き込み前にresetする経路は理論上残るため、`停止基準`の追加基準を守る）。
6. **`起動とarm delay`（人間）** ESP32を起動する（flash直後）。`espflash`のmonitorで
   `servo_bench_test_arm_delay_start`のlogを待ち、出たら`config::SERVO_BENCH_TEST_ARM_DELAY_MS`
   （既定10秒）の間にサーボの外部5 V電源を入れる。**このlogが出るまでの時間に、
   実測に裏づけられた上限はまだ無い。**[#451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)
   で`run_i2c_bringup`のI2C読み出しは無期限timeoutをやめ、有限のtimeoutになった
   （1回あたりの上限は`firmware/esp32/src/config.rs`の`I2C_TRANSACTION_TIMEOUT_MS`、
   上限がどう効くかは`main::run_i2c_bringup`のdoc。**どちらもここへ再掲しない**）。
   **したがってI2C固着でこのlogが永久に出ないことは無い**が、`Err`が返るまでの時間を
   実機で測ってはいないため、待つべき秒数を数値では示さない。**明らかに出ない（他の
   bring-up logより大幅に遅い、または出ないまま止まって見える）場合は、異常として扱い、
   servoの電源を入れずに報告する。**I2C読み出しが失敗していれば
   `accel_device_id_read_failed`／`env_chip_id_read_failed`がlogに出るため、
   **bus固着とそれ以外は、まずこの行の有無で切り分ける。**
   **`arm delay`を過ぎたら、動かす意図で電源を入れない。**`arm delay`終了後も
   `Sg90::stop`までの短い間（既定300 ms）はduty出力が残っており、「`arm delay`を
   過ぎたら絶対に動かない」とは言えない（`停止基準`の「動かなかった場合」はこの
   windowも逃した場合を含む）。
7. **`監視維持`（人間、`起動とarm delay`と並行して行う）** `起動とarm delay`の
   電源投入と同時に、直ちに遮断できる状態を維持する（別のstepとして順番に行う
   ものではない）。`arm delay`の間に行うのはservo電源の投入だけであり、AIへの
   実行指示は無い（firmwareが待機終了後に自動で動かす）。
8. **`単発動作`（firmware、`arm delay`終了後に自動実行）** `Sg90::move_to_angle_once`を
   1回だけ呼ぶ短いtest経路（`bench-servo-test-17` feature）により、中央から
   `config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG`（既定15度、一次資料にも実測にも
   基づかない暫定値）**だけ+側（`main::run_servo_bench_test`が`neutral + max_deviation`で
   呼ぶ、片側のみ）**へ1回動かす。`config::SERVO_BENCH_TEST_EXPOSURE_MS`
   （既定300 ms。安全な保持時間の主張ではなく露出時間の技術的最小化）の後、`Sg90::stop`で
   dutyを0へ戻す（連続で保持しない）。**信号を止めてもservoが駆動を止める保証は無い**
   （上記「残余risk」参照）。
9. **`停止基準`（人間）** 人が試験を止める基準に該当する事象が無いかを確認する。
   該当すれば直ちに外部5 V電源を遮断する。基準は[Hardware Safety Policy §6](../governance/hardware-safety-policy.md#6-サーボ)が列挙する7項目
   （予期しない方向への動作、衝突または拘束、異音、過熱、**resetまたはbrownoutの反復**、
   過電流、commandまたは緊急停止応答の喪失）**を基本とし**、本手順として次を加える
   （根拠はPR本文参照）。
   - **唸り**（動かないまま音がする）。
   - 電源の電圧降下。
   - **指令していないのにservoが2回目に動いた場合。**直ちに外部5 V電源を遮断する。
   **動かなかった場合は、そこで止めて報告する。pulse幅を変えて繰り返し試さない。**
   `servo_bench_test_nvs_latch_set`のlogが出ていれば「1回分」は消費済み（再試行には
   `再武装`と改めての承認が要る）。`_partition_failed`／`_open_failed`／`_get_failed`／
   `_set_failed`ならNVSアクセス自体の異常として扱い、再flash前に原因を確認する
   （「1回分」は未消費）。
10. **`観察記録`（人間とAI）** 指令角への追従、動いた向きと角度、中央位置の手がかり、
    保持中に唸るかを観察し記録する。**ここでDigital／Analogのどちららしいかが
    分かることがあるが、分かっても「特定した」とは書かない。観察結果として記録する。**
11. **`latch検証`（人間）** **servoの外部5 V電源を先に遮断してから**、ESP32を1回
    resetする（reset押下またはUSB抜き差し）。servoを動かす必要は無く、判定はlogだけで
    行う。`servo_bench_test_skipped_already_ran`が出ればlatchが効いている。
    `_nvs_latch_set`と`_arm_delay_start`が出たらlatchが効いていない（servoは無給電の
    ため物理的には動かない）。`_partition_failed`／`_open_failed`／`_get_failed`／
    `_set_failed`が出た場合はNVSアクセス自体の失敗として「効いているとは確認できない」
    扱いにする。
    **latchが効いている場合を除き、servoの外部5 V電源を入れずに報告する**
    （`終了処理`の通常buildへの復帰まで再試行しない）。結果はAIへ伝え、`記録`で
    まとめて[experiment-log.md](experiment-log.md)へ記録する。
12. **`再武装`（人間）** 繰り返す場合は改めて承認を取ってから再武装する。**NVSの
    flagが立ったままでは再実行されないため**（`servo_bench_test_skipped_already_ran`）、
    NVSを消去してから再flashする（`espflash`の`erase-flash`または`erase-parts nvs`
    相当。対象board・partition名を実行前に確認する。検証状況はPR本文参照）。
    承認を得てから`起動とarm delay`へ戻る。承認無く連続動作・再試行はしない。
13. **`終了処理`（人間）** 試験終了後、外部5 V電源を遮断する。`単発動作`まで到達して
    いればGPIO27は`run_servo_bench_test`が返るとlowへ固定される（`Sg90`のdrop。
    [`crate::servo::Sg90::stop`]のdoc参照）。**`Sg90::new`より前にreturnした場合
    （NVS latch関連のerror、`skipped_already_ran`を含む）は`Sg90`が存在せず、
    GPIO27は未configuredのままである（上記「決定事項」参照）。**さらに、
    `bench-servo-test-17`を付けない通常buildへ直ちに戻す（再flash）。NVS latchの
    成否に関わらずservo出力の可能性自体を無くすため。前提の検証状況はPR本文参照。
14. **`記録`（AI）** 観察結果（`通電前の現物確認`の(a)(b)(c)、`観察記録`、`latch検証`を
    含む）を[experiment-log.md](experiment-log.md)へ新しいEXP番号（着手時点で未使用の
    次番号）で記録する。観察から確定値を導かない。calibrationとgate解決は別途進める。

## 受け入れchecklist

- [ ] 正確なサーボとデータシートを記録した
- [ ] **定常電流**でingressとconnectorの定格を確認した（[power-budget.md](power-budget.md)の`ingressの電流制限`。定格は熱の制限のため、判定量は定常電流である）
- [ ] **peak時**の5 V／3.3 Vの電圧droopを測定し、**ESP32の**brownoutとresetが起きないことを確認した（peakはこの確認にのみ使う）
- [ ] **（Blocked）**ESP32入力と3.3 V railの電圧が、peak時も許容範囲内に収まることを確認した。**上の項目のbrownout／resetが起きないことは、この項目の代わりにならない。**railはbrownout検出の閾値へ達しないまま動作範囲の下限を割りうるためである。**現状はこの項目を合格にできない。**`HW-TBD-028`の(b)（ESP32入力／3.3 Vで許容する最低電圧）、(c)（最大定常ripple）、(d)（最大transient droopと継続時間）はいずれも合否判定が`Blocked`であり、(b)の5 V入力側の4.6 Vは**2026-08-15に現物の`U2`（UMW `LD1117-3.3`）のdatasheetによる値になった**ため部品未確認を理由とするBlockedは解消したが、**閾値として確定させるかは`HW-TBD-028`で扱う。**3.3 V rail側は`HW-TBD-025`(a)に従属する。**測定値を記録してよいが、そこから受け入れの合否を導かない**（[power-budget.md](power-budget.md#受け入れ条件)の`電源品質の数値制限`、[tbd-register HW-TBD-028](tbd-register.md)）。**さらに`HW-TBD-034`により、閾値が確定してもこの項目を合格にできない。****現状の**測定系がESP32自身のADCであり、**brownout／reset中とsample間とcapture window外が未観測である**。**2026-08-16に方式1（独立した外部観測）を採用したため、これはESP32のADCを最終の受け入れ条件として固定するものではない。**方式1が実装されれば`brownout／reset中`と`capture window外`は観測対象にできるが、**`sample間`は方式1でも残る**（[power-budget.md](power-budget.md#測定計画)の`ESP32自身のADCは測定対象から独立していない`が定める未観測区間3種のうち、独立した記録係で塞げるのは`brownout／reset中`と`capture window外`だけである。**ここへ数値も規則も再掲しない**）。**したがって方式1の実装だけでこの項目を合格にできない。****合格にできるかは`HW-TBD-034`のclose条件が満たされたかで決まる**（条件の全数は[tbd-register.md](tbd-register.md)の`HW-TBD-034`。**ここへ再掲しない**）（[power-budget.md](power-budget.md#測定計画)の`ESP32自身のADCは測定対象から独立していない`。**ここへ再掲しない**）
- [ ] **（対象外）**Pi入力の電圧droopが許容範囲内であることを確認した。**現状はこの項目の合否を判定しない。**Raspberry Pi Zero Wには低電圧検出回路が無く（[power-budget.md](power-budget.md)の`Pi Zero Wには低電圧検出が無い`）、判定は`ADC-5V`の実測に依るが、**(i) 2026-09-06にPM（`deskcat-f2`）が`HW-TBD-028`(a)（判定に使う最低入力電圧）を受け入れ試験の要件から外す決定をしたため、この項目は対象外である。復帰条件（可変電源の調達、またはPi Zero W向け一次資料の公開）を満たすまで合否判定を行わない。(ii) `ADC-5V`の測定系も未実装である（**分圧抵抗は入手済み。実装と検証が残る**。2026-08-12に購入履歴と照合して訂正した）。****「確認した」と記入しない**（[power-budget.md](power-budget.md#受け入れ条件)の`(a)を要件から外す（2026-09-06、PM決定）`、[tbd-register HW-TBD-028](tbd-register.md)、[hardware-bom.md](hardware-bom.md)の`MEAS-01`）
- [ ] **承認範囲内の最悪動作**でservo railの**定常電流**を実測し、`動作制限`表の`最大連続電流`の予算（250 mA）以下であることを確認した。超える場合は可動域・速度・duty cycleを締めて再測定した。**[power-budget.md](power-budget.md)でingressの定格を上げただけでは、この項目を合格にしない**（予算はこの表がfirmwareへ渡す値であり、経路側の定格とは別の量である）
- [ ] 予算そのものを変えた場合は、[power-budget.md](power-budget.md)の`ingressに必要な定格の見積もり`にある**正式改訂の手順**（両文書の値を同時に改訂し、経路部品とgate値を決め直し、受け入れchecklistを通し直す）を踏んだ。**測定値に合わせて判定を緩めていない**
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
| 2026-08-12 | 9 | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)。受け入れchecklistの電圧droop項目が、**Piのbrownoutをsoftwareで検出できることを暗に前提としていた。**[Raspberry Pi公式documentation](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html)の`Power supply warnings`は、低電圧検出回路が`On all models of Raspberry Pi since the Raspberry Pi B+ (2014) except the Zero range`にあると明記しており、**`SBC-01`のRaspberry Pi Zero Wはこの`Zero range`に該当する。**同項目を「**ESP32の**brownoutとresetが起きないこと」に限定し、**Pi側は別項目として切り出したうえで`Blocked`とした。****`ADC-5V`の実測に置き換えるだけでは不十分である。**判定に使う最低入力電圧が`HW-TBD-028`(a)として未確定であり（一次資料に無いことを2026-08-12に確認した）、`ADC-5V`の分圧抵抗も未購入で測定系が存在しないためである。**未確定の閾値に対して合格を記入できる状態にしない。****ESP32は自身でbrownoutを検出するため、ESP32についての確認は変えていない。****サーボの動作制限も出力有効化のgateも変えていない** |
| 2026-08-12 | 10 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)。受け入れchecklistのPi入力droop項目が、Blockedの理由として「`ADC-5V`の分圧抵抗が未購入で測定系自体が無い」と書いていたが、**分圧抵抗は2026-08-08に着荷済みであった**（[hardware-bom.md](hardware-bom.md) Revision 37）。理由を「測定系が未実装（抵抗は入手済み、実装と検証が残る）」へ改めた。**この項目はBlockedのままである。**判定に使う最低入力電圧が`HW-TBD-028`(a)として未確定であることは変わらない |
| 2026-08-16 | 11 | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)の2026-08-16の追記を反映。**受け入れchecklistの`（Blocked）`項目（ESP32入力と3.3 V railの電圧）を3点直した。****Blockedは解除していない。**(a) **`HW-TBD-034`への参照を足した。**この項目は`HW-TBD-028`(b)(c)(d)を根拠にBlockedとしていたが、**閾値が確定しても合格にできない**理由がもう一つある。測定系がESP32自身のADCであり、**brownout／reset中・sample間・burst captureのwindow外が未観測である**（[power-budget.md](power-budget.md#測定計画)の`ESP32自身のADCは測定対象から独立していない`。**規則も数値も再掲していない**）。(b) **同項目が「(b)の5 V入力側は参照設計由来の4.6 Vで現物の`U2`が未確認」と述べていたが、これは古い。**`U2`は2026-08-15にUMW `LD1117-3.3`と特定され、[power-budget.md](power-budget.md#受け入れ条件) Revision 45が**部品未確認を理由とするBlockedの解消**を記録している。正本に合わせて訂正した。**この項目自体は3.3 V rail側と`HW-TBD-034`によりBlockedのままである。**(c) **記録として残す。**この項目は`a11d32d`（2026-08-16）で新設されたが、**対応するRevision行が無く、この文書には2026-08-16の記録が1つも無かった。**過去行は書き換えず、ここに記録する |
| 2026-08-18 | 12 | [PR #146](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/146)のreview指摘。**受け入れchecklistの`（Blocked）`項目が、ESP32自身のADCという現状の測定系を最終の受け入れ条件として固定して読めた。**2026-08-16に方式1（独立した外部観測）を採用しており、実装されれば未観測区間は観測対象にできる。**「現状の」測定系であることを明示し、合否は`HW-TBD-034`のclose条件で決まるとした**（条件の全数は台帳が正。再掲しない）。**Blockedは解除していない。サーボの動作制限も出力有効化のgateも変えていない** |
| 2026-08-18 | 13 | [PR #146](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/146)のreview指摘。**Revision 12で「方式1が実装されれば未観測区間は観測対象にできる」と書いたが、これは正本より広い主張だった。**[power-budget.md](power-budget.md#測定計画)の`ESP32自身のADCは測定対象から独立していない`は未観測区間を3種に分け、**独立した記録係で塞げるのは`brownout／reset中`と`capture window外`だけで、`sample間`は残ると定めている。**3区間すべてを覆う読みになっており、**この`（Blocked）`項目の根拠を実際より弱めていた。**塞げる区間と残る区間を書き分け、方式1の実装だけでは合格にできないことを明記した。**Blockedは解除していない。サーボの動作制限も出力有効化のgateも変えていない** |
| 2026-08-24 | 14 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。`HW-TBD-026`（SG90の電気的駆動条件）のうち一次資料で確定できる部分を確認し、`サーボ識別情報`表へ反映した。**駆動条件は未確定のまま残す。Revision 2の方針を変えていない。****動作制限表、サーボ出力を有効化してよい条件、受け入れchecklistは変えていない。**この文書の状態`Blocked`も変えていない。(a) **`制御logic要件`・`PWM周期／rate`・`許容最小／最大pulse`は、いずれも一次資料に記載が無いことを確認した。**`TBD`は維持したまま、**「記載が無い」ことを結論として記録した。**とくに`制御logic要件`は、3.3 V driveの可否がこの資料では判定できないことを明示した。**「3.3 Vで動く」とは書いていない。**(b) **`Dead band width`行を新設した（`1 us`）。**一次資料の仕様表から得られた制御信号の電気仕様はこの1点だけである。**この行の正は[tbd-register.md](tbd-register.md)の`HW-TBD-026`である。**[hardware-bom.md](hardware-bom.md)の`SERVO-01`にはこの項目を載せる列が無いため、この1行だけ`根拠`欄がBOMを指していない。**確定した運用値ではなく、許容pulse幅範囲の代わりにもならない旨を同じセルに書いた。**(c) `データシートrevision`行を訂正した。**公式サイトにSG90のdatasheet PDFは無く、従来引いていた`Soldered_101246.pdf`はTowerPro発行と確認できず取得もできない。**あわせて公式サイトに同名2品（`SG90 Digital`／`SG90 Analog`）があることを反映した。(d) `定格電圧範囲`の根拠を`データシート`から訂正した。**仕様表の記載は`Operating voltage: 4.8v`だけで、`4.8–6 V`はレビュー欄に由来する。値は変えていない。****この表はBOMからの再掲であり、値をこの文書で確定させていない。**(e) `動作電流`行の`データシート値は0.5–2 A`も同じ誤帰属であったため、同様に外した。**値は変えていない。**確定は`HW-TBD-010`／`011`の範囲であり、この改訂では判断していない。(f) **電流3行（`無負荷電流`・`動作電流`・`Stall／peak電流`）と`データシートrevision`行の`根拠`欄が、存在しない文書を指していた。**公式のdatasheet PDFが無いことが確定したため、前者3行を`データシート／測定`から`測定`へ、後者を`メーカー文書`から`メーカーの製品ページ`へ改めた。**`TBD`は3行とも維持しており、確定先（`HW-TBD-010`／`011`）も変えていない。**1行だけ直すと同じ誤りが同じ表の中に併存するため、`根拠`欄は全数を見た。正は[hardware-bom.md](hardware-bom.md) Revision 62と[tbd-register.md](tbd-register.md)の`HW-TBD-026`である |
| 2026-08-24 | 15 | [PR #190](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/190)のreview指摘。**`4.8–6 V`と`0.5–2 A`の出所が`SG90 Digital`側ページに限られることが判明した。**Analog側ページのreview欄は`There are no reviews yet.`であり、この2値はどちらもDigital側にしか無い。**現物がどちらの品かは未確定であるため、variantに依存しない定格として扱わない旨を`根拠`欄へ明記した。****値は変えていない。**`定格電圧範囲`を`TBD`へ戻すかは、closeした`HW-TBD-006`の扱いに掛かるため[tbd-register.md](tbd-register.md)の`HW-TBD-026`経由で人間へ渡す。**動作制限表、有効化gate、受け入れchecklistは変えていない** |
| 2026-08-24 | 16 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。**要約toolの出力に依らず、生HTMLで一次資料を再検証した。**`robots.txt`が指す`wp-sitemap.xml`からsite全体の203 URLを列挙し、`.pdf`が0件であることを確認した。SG90の3ページと`/download/`も`.pdf` link 0件、`datasheet`の語0件、**いずれもHTTP 200で実体を取得している**（403や空応答ではない）。**「公式のdatasheetが無い」という判断は維持される。****一方で、Q&A欄を読んでいなかったという抜けが判明した。****`定格電圧範囲`と`動作電流`の出所を、Revision 14／15の「レビュー欄」から「`admin`（メーカー側）の回答」へ格上げした**（2021-11-07。`Voltages from 4.8V - 6V are fine.`）。**匿名のreviewではない。**ただし**仕様表の値ではなくQ&A欄の記述である点は変わらない。値も変えていない。****Revision 14／15の記述は書き換えず、ここに訂正として残す。**全数と、pulse幅について客が直接聞いてメーカーが答えていない事実は[tbd-register.md](tbd-register.md)の`HW-TBD-026`が正である |
| 2026-08-24 | 17 | [#193](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/193)。**#193の判断のために`4.8–6 V`の依存関係を調べたところ、追跡されていない項目が1つ見つかった。**`PSU-SERVO-01`はM-12001の5 Vをservoへ供給するが、**製品ページの仕様表が載せる電圧は`Operating voltage: 4.8v`だけで、`4.8–6 V`はメーカーのQ&A回答1文にしか出てこない。****そして[power-budget.md](power-budget.md)の`許容電圧範囲`の積集合は3.3 V railのmoduleとESP32だけを対象とし、servoを含まない。****servoの入力電圧はどの文書でも適合を確認されていなかった。**`HW-TBD-035`として登録し、`サーボ識別情報`表の`定格電圧範囲`行と`サーボ出力を有効化してよい条件`の表へ追加した。**gateは締める方向にのみ変えている。**元から閉じており、**開けていない。****動作制限表と受け入れchecklistは変えていない。値も変えていない**（`4.8–6 V`は`TBD`へ戻していない。判断は#193） |
| 2026-08-24 | 18 | [PR #194](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/194)のreview指摘。**`定格電圧範囲`行が同名3品目の`SG90 360 degree`に触れていなかった。****同品の仕様表は`Operating voltage: 4.8v-6V`を載せており、どの品かによって`4.8–6 V`に仕様表の裏付けがあるかどうかが変わる。**この事実と、連続回転・寸法差による除外が**現物未計測のため確定していない**ことを明記した。**値は変えていない。動作制限表、有効化gate表、受け入れchecklistも変えていない。**closeに必要な記録は[tbd-register.md](tbd-register.md)の`HW-TBD-035の証拠契約`が正である |
| 2026-08-25 | 19 | [PR #199](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/199)のreview指摘。**`Dead band width`行が同名3品目`SG90 360 degree`に触れていなかった。**同じ表の`定格電圧範囲`行はRevision 18で同じ保留を得ており、**同一の表の中で扱いが割れていた。**3品目については仕様表を確認していない旨と、除外に現物の寸法の実測が要る旨を追記した。**値は変えていない**（`1 us`）。**動作制限表、有効化gate表、受け入れchecklist、gate値も変えていない。どのgateも開いていない** |
| 2026-08-29 | 20 | [Issue #257](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/257)で2026-08-28に実施した現物の高さ実測（27 mm）が、[PR #258](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/258)でこの文書へ反映されていなかったため反映する。**Revision 18／19が保留していた`SG90 360 degree`の除外を、この実測により判定した**（`23×12.2x29mm`側に近く、`23×12.5x22mm`側との差5 mmより有意に近い）。`定格電圧範囲`行・`Dead band width`行の該当箇所を「除外していない」から「実測により除外した」へ改めた。**`Digital`と`Analog`の分離はこの実測では不可能であり、未確定のまま残る。**値（`4.8–6 V`、`1 us`）はいずれも変えていない。動作制限表、有効化gate表、受け入れchecklistは変えていない。どのgateも開いていない |
| 2026-09-08 | 21 | [#367](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/367)。[PR #366](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/366)（`develop`→`main`昇格）の`full review`でCodeRabbitが出した指摘（outside diff）を反映した。**Pi入力の電圧droop受け入れchecklist項目の状態語が`power-budget.md`と食い違っていた。**`power-budget.md`は2026-09-06のPM決定により`HW-TBD-028`(a)を要件から外し状態語を`対象外`へ統一していたが、この文書は`（Blocked）`のままで、理由(i)も「最低入力電圧が未確定」という古い記述のままだった。**状態語を`（対象外）`へ改め、理由(i)を`power-budget.md`と同じ形（対象外である理由、復帰条件）へ揃えた。理由(ii)（`ADC-5V`の測定系が未実装）は生きているため変えていない。**値・測定計画・他の受け入れ項目は変えていない。どのgateも開いていない |
| 2026-09-09 | 22 | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)。**PM（`deskcat-f2`）が承認した節見出しの改名（[power-budget.md](power-budget.md)の`WORK-INSTRUCTIONS-INGRESS-PLAN-B.md`反映の一環）に合わせ、この文書からの参照を揃えた。**受け入れchecklistの「予算そのものを変えた場合は`power-budget.md`の`変換基板に必要な定格の見積もり`にある正式改訂の手順を踏んだ」という項目の参照先節名を`ingressに必要な定格の見積もり`へ改めた。**節見出し自体は`power-budget.md`側の改名であり、この文書は参照名を揃えただけである。**値・判定条件・他の受け入れ項目は変えていない。どのgateも開いていない |
| 2026-09-22 | 23 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。`SERVO-01`（TowerPro SG90）初回動作の準備として、`firmware/esp32/src/servo.rs`（`Sg90::move_to_angle_once`、1回だけ指定角度へ動かす）を追加し、`Calibration手順`の2〜7を人間とAIの作業順序へ具体化した`初回動作の実行手順（人間とAIの作業順序）`節と、承認状態を集約する`承認の状態`節を新設した。**別文書を作らず既存へ統合した**（PMの指摘。文書を増やすこと自体が食い違いの原因になるため）。新節は`サーボ出力を有効化してよい条件`の内容を再掲せず、同節へlinkするだけにしてある。この1つのcommit内で複数回の自己レビューを経ており、この行はその最終状態をまとめて記録する（途中経過を個別のRevisionへ分けない）。`HW-TBD-010`が要求する実測をこの手順が兼ねることを明記し、動かなかった場合やpulse幅を変えた再試行には改めて承認が要ることを記載した。信号停止（`Sg90::stop`）がservoの駆動停止を保証しない旨（Digital／Analogで挙動が異なりうるが識別しないため不明）と、拘束が続いた場合の最悪の事態（樹脂ギアの変形・溶融、煙が出る可能性を否定できない）を明記した。firmwareに`bench-servo-test-17` featureを追加し、このfeatureを付けたbuildだけが`main()`から`crate::servo`を呼ぶようにした（既定のbuildは変わらず呼ばない）。servoの突入電流やnoiseでESP32がbrownout resetした場合に承認の無いまま動作が繰り返される経路が見つかったため（PMの指摘）、NVSへ実行済みflagを実際に動かす前にcommitする一発limiterを追加し、resetのたびの再実行を防いだ。`停止基準`へ「指令していないのに2回目に動いたら直ちに遮断する」を追加した。Digital／Analog識別せず進めるユーザー決定（2026-09-22、[tbd-register.md](tbd-register.md)の`HW-TBD-026`）も新節へ反映した。**動作制限表・有効化gate表・受け入れchecklistは変えていない。**冒頭の`確定しているproject規則`のうち「AIが生成したcommandやdebug commandでもhard limitを迂回できない」の行へ、この試験がその迂回に該当する旨の一文を追記した（規則自体は変えていない）。承認の現在の状態は[承認の状態](#承認の状態)を参照する（このRevision行では再掲しない） |
| 2026-09-22 | 24 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。Revision 23が記録した3点の不備を補う再確認を行い、結果を`承認の状態`節へ追記した。同節の項目3・4がその内容と現在の状態を持つ（ここへ再掲しない）。動作制限表・有効化gate表・受け入れchecklistは変えていない。 |
| 2026-09-22 | 25 | [#454](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/454)。[Hardware Safety Policy §6](../governance/hardware-safety-policy.md#6-サーボ)の「電流制限を設定でき、十分な定格を持つ電源を使用する」という要求が、「故障電流を制限する手段を明記する」という要求へ書き換わったことに追随し、この書き換えで記述が偽になった箇所（`初回動作の実行手順`の残余risk一覧と、その手前の参照文）を最小限に直した。**`承認の状態`節（2026-09-22に何を明示して承認を得たかの記録）は変更していない。**故障電流を制限しうる手段（`PROT-OC-01`の有無）と、手段が無い場合に代わりに守るもの（人間による手動遮断）についての記述内容は変えていない。冒頭の`確定しているproject規則`は変えていない |
| 2026-09-23 | 26 | [#451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)。**firmware側の2つの変更に追随した。**3箇所。(1) 残余riskの`DISP-01` backlight項は「firmwareが無条件に点灯させる」と書いていたが、`run_display_bringup`が`bringup-display-13` feature（既定off）付きbuildだけの経路になり、この試験のbuild（`--features bench-servo-test-17`）には点灯経路が無い。**`HW-TBD-024`は解けていないため、`DISP-01`を接続しない要求は維持する。**(2) `通電前の現物確認`(d)も同じ理由で書き換え、**(d)の確認自体は省かないことを明記した**（`HW-TBD-024`未解決、`EXP-011`はESP32側の信号レベルまで）。(3) `起動とarm delay`は「このlogが出るまでの時間に定義された上限は無い（I2Cが無期限timeoutのため）」と書いていたが、`#451`でI2C読み出しは有限timeoutになった。**永久に出ないことは無い**旨と、**`Err`までの実機実測はまだ無いため秒数を示さない**旨、切り分けはlog行の有無で行う旨へ書き換えた。**承認の状態、安全値、手順の順序、停止基準は1つも変えていない。** |
