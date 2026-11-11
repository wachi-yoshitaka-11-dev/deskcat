# Servo Safety Limits

> 状態: Blocked — 正確なservoとmechanical assemblyが必要
> 正本とする情報: サーボの電気的制限、機械的制限、動作制限、fail-safe動作

## 確定しているproject規則

- サーボにはESP32の電源pinではなく、外部電源経路を使用する。
- ESP32とサーボ電源のGNDを意図的に共通化する。
- ESP32はRaspberry Piから独立して、hard motion limitを強制する。
- AIが生成したcommandやdebug commandでもhard limitを迂回できない。**`bench-servo-test-17`
  featureによる単発bench試験は、この規則が禁じる迂回に該当する（[承認の状態](#承認の状態)参照）。**
  **2026-09-28追記: 初回駆動の方法はユーザーが決めた（[承認の状態](#承認の状態)の項目6）。**
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

### 測定のための駆動（承認の状態の項目6）

**このゲートは開かない。**上の`TBD`を解決するための測定（#17の電気特性、#18の可動域・速度・加速度の校正、#19の実機limiterの監視下試験）に限り、
[承認の状態](#承認の状態)の項目6の条件(a)〜(c)がすべてそろったときだけ、servoを駆動してよい。これはユーザーの決定（2026-09-28、#17）による限定した例外であり、
一般の運用やPiからのcommandによる駆動を認めるものではない。**(c)（ユーザーの実行承認）は駆動の回ごとに取る。**

駆動してよいのは、次をすべて満たすときだけである。

- **電源:** 項目6の(a)の構成（`M-12001`を試験の間だけservo専用にし、予備の`MF-R135`を直列に入れる。Piは外す。ESP32はPCのUSBから給電する。servo電源の経路は端子台と線材で組み、breadboardとjumper wireを使わない（#18の段0・段1は、servoのplugへ行く2本のjumper wireとシャントだけを除く。段2は`PSU-BENCH-01`の CC の間に限り同じ。breadboardは通さない。[#18の校正（案）](#18の校正案)の段0の2と[#18の段2（案）](#段234-の頭を付け可動域を片側ずつ広げる)））。
- **build:** 測定用のbuildだけを使い、**Piとの通信linkを持たない。**初回（#17）は、1回の起動で1 commandだけを出し、動作角を狭い仮の制限でclampし、露出時間の後にdutyを0へ戻し、NVSのlatchで再実行を止めるbuildを使う（下の`初回動作の実行手順`の`build`と`単発動作`）。**#18の校正と#19の実機limiter試験で使うbuild（複数のcommandやlimiterを通す駆動）は、その動作と仮の制限をこの文書に書いたうえで、その回の実行承認で範囲を決める。**（#18の分の動作と仮の制限の案は[#18の校正（案）](#18の校正案)。承認していない。）firmwareの変更と`compile_error!`の扱いは、項目5の、外す注記を挙げた段落（「再開の条件は、この節では定めない。」で始まる段落）とその後の2026-09-30追記の手順に従う別のPull Requestで行う（2026-10-13に行った。[承認の状態](#承認の状態)の項目6の(b)の記録）。
- **負荷:** 初回は無負荷（ホーンを外す）。頭を載せる駆動（#18の校正）は、その回の実行承認で範囲を決める。
- **監視:** 人間が立ち会い、`M-12001`を直ちに抜ける状態を保つ。無人で続けない。停止基準は`初回動作の実行手順`の`停止基準`に従う。

未解決の`TBD`ごとの扱いは次のとおりである。**いずれも解決したことにはしない。**

| TBD | この駆動での扱い |
|---|---|
| `HW-TBD-007`（電源とpower budget） | servoは専用の`M-12001`から給電し、ESP32はPCのUSBから給電する。servoの電流でESP32の電源が落ちる経路を作らない |
| `HW-TBD-009`（backfeed） | 解決しない。servoとESP32が共有するのは信号線とGNDだけである。残余riskとして`初回動作の実行手順`に記録する |
| `HW-TBD-010`／`HW-TBD-011`（可動域、速度・加速度） | この駆動が測る対象である。値が決まるまでは、狭い仮の制限で駆動する（初回は単発command。#18・#19はそのbuildに書く仮の制限） |
| `HW-TBD-035`（servoへ入れてよい電圧） | 解決しない。`M-12001`の5 Vを監視下で入れて観察する（同行の解決手段(3)にあたる）。同行は(3)が`HW-TBD-007`／`009`のgateに従うとするが、この例外の中では、その2行を上の行の扱いで代える（どちらも解決はしていない）。**段1（`PSU-BENCH-01`）では、入れる電圧は小節の4が持つ**（回ごとの承認が置く値で、上限は5 V）。扱いはこの行のとおりで、監視下で観察し、解決はしない（4.8 V など5 V 未満を置いても、同じ扱いである） |
| `HW-TBD-026`（logic閾値、PWM周期、pulse幅） | 解決しない。一般的な値で駆動し、観察を記録する |
| `HW-TBD-017`／`018`（通信断） | 該当しない。測定用のbuildはPiとの通信linkを持たない |
| `HW-TBD-019`／`HW-TBD-027`（起動時の状態、外部pull-down） | 解決しない。`RES-PULL-01`の実装状況を`通電前の現物確認`で記録する |
| `HW-TBD-020`（実行時の安全制御） | 解決しない。初回は単発command、露出時間、NVSのlatchで、#18・#19はそのbuildに書く仮の制限で代え、どちらも人間の監視と電源遮断を併せる |

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

5. **2026-09-25、[#474](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/474)により、この節の状態を「進めてよい」から「実行しない」へ改めた。**PMと作業セッションの判断であり、ユーザーが承認した。承認はIssueやpull requestのcommentではなく、この変更（PR）を作成したAI sessionとユーザーとの対話の中で得た。承認を求めたときに示したのは、下の2つの理由（短絡、循環）と、`AGENTS.md`とHardware Safety Policyを変えずにfeatureを`compile_error!`で塞ぐことである。flash済みのESP32についての注意と再開の手順は、承認の後にこの変更のreviewで足した。記録は、この節、#474の本文、この変更を含むcommitとpull requestが持つ。**上の1〜4の記録は書き換えていない。**

   この例外は規則と整合させられないと判断した。理由は2つある。
   - **5項目の短絡と両立しない。**[Hardware Safety Policy](../governance/hardware-safety-policy.md)の`安全要件の5項目`の`短絡`の行は、`PROT-OC-01`が入るまで故障電流を制限するものが経路に無く、connectorと線材の発熱は秒から分のorderで進むため「人が読み取って電源を落とすのでは止まらない」と定める。この試験は、`PROT-OC-01`が経路に入っているかを`通電前の現物確認`で記録するだけで、結果にかかわらず進む（下の`決定事項`）。入っていなければ、故障電流への備えは人間の手動遮断だけになる（下の残余riskの「この試験には、故障電流を制限する手段が無い。」）。**1〜4の承認時に示した点に、この衝突は含まれていない**（項目2は§6の電流制限電源が無いことを、項目3は`PROT-OC-01`の実装状況が未確認であることを示したが、そのとき人の遮断では止まらないという`短絡`の行の定めは示していない）。`PROT-OC-01`が入っていることを条件に足すと、上の1〜4で承認された手順（結果にかかわらず進む）とは別の手順になり、その承認の範囲を超える。
   - **例外を規則に書く道も取れない（前提が循環する）。**PR #471のreviewは、承認の記録だけでは迂回を禁じる規則が変わらないと指摘した。規則に例外として書くことを先に検討した（#474）。[サーボ出力を有効化してよい条件](#サーボ出力を有効化してよい条件)は、`HW-TBD-010`を含む全行が解決するまでservo出力を有効にしないと定める。この試験は`HW-TBD-010`の実測に着手する最初のstepである（下の`初回動作の実行手順`）。`010`は`026`を含む項目にBlockedされ、`026`に残るのは無負荷試験を要する範囲である（[TBD台帳](tbd-register.md)の各行）。そのため、gateを満たすまで待つ形に書くと例外は一度も使えず、使える形に書くとgateと5項目（上の短絡）を外す規則になる。**`PROT-OC-01`を条件に足しても、この循環は解けない。**

   `bench-servo-test-17` featureは、有効にするとcompileが止まる形で塞いだ（`firmware/esp32/src/main.rs`の`compile_error!`）。**`compile_error!`が止めるのは、これからのbuildだけである。**`bench-servo-test-17`付きbuildを一度flashしたESP32は、NVSに実行済みflagが無ければ次の起動で試験を実行する（`run_servo_bench_test`のdoc）。[experiment-log.md](experiment-log.md)に、同featureを付けたbuildをflashした記録は無い。flashしたESP32があれば、servoの外部5 V電源を入れる前に既定buildを書き込む。同featureを付けて以前buildした成果物（ELF）が残っていても、書き込まない。

   codeとfeatureの定義は、再開するときの出発点として残す。**ただし`#[cfg(feature = "bench-servo-test-17")]`の付いたcode（`run_servo_bench_test`等）は、この間どのbuildも成功せず、既定buildとCIの型検査・lint・buildはこのcodeに届かない（届くのは`cargo fmt`の構文検査だけである）。壊れても気づきにくい。**再開するときに確かめる。

   **再開の条件は、この節では定めない。**[サーボ出力を有効化してよい条件](#サーボ出力を有効化してよい条件)は`HW-TBD-010`（監視下calibration）を含む全行の解決を求めるが、`010`の実測にはservo出力が要る。**gateと5項目の範囲で最初の駆動をどう行うかは未決であり、[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)が扱う。**`PROT-OC-01`の実装（[#405](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/405)）は、その前提の1つである。再開するには、この節の状態を改め、`main.rs`の#474の`compile_error!`を外し、#474の`compile_error!`で止まることを述べた注記を外す変更が要る。**`pi-protocol-mode`との排他の`compile_error!`とその注記は残し、#474に触れた括弧書き・文だけを外す。**外す注記は、`firmware/esp32`の`Cargo.toml`・`README.md`・`src/main.rs`・`src/servo.rs`のものと、この文書の`確定しているproject規則`・`build`stepへの2026-09-25追記である。**この項目5、残余riskへの2026-09-25追記、Revision 28は記録として残す。**これらの注記は承認の状態ではなく、`compile_error!`があるというfirmwareの挙動を述べたものであり、`compile_error!`を外すと偽になるため外す。

   **2026-09-30追記（[#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)のPR B1）: 上の「`pi-protocol-mode`との排他の`compile_error!`とその注記は残し」の`compile_error!`は、もう無い。**#487で`pi-protocol-mode` featureを廃止し、Piとの通信link（UART1）を製品build（既定build）へ入れたためである。**測定用のbuild（`bench-servo-test-17`）がPiとの通信linkを持たないこと（[測定のための駆動](#測定のための駆動承認の状態の項目6)の`build`）は、`firmware/esp32/src/main.rs`の`#[cfg(not(feature = "bench-servo-test-17"))]`で保っている**（このfeature付きbuildはPi link（`crate::boot_session`とUART1）をcompileしない）。**再開のときに外すのは、#474の`compile_error!`とその注記だけである。**ただし、このfeature付きbuildは#474の`compile_error!`があるためbuildしておらず、#487でmain loopを組み替えた後に、外してcompileが通るかは確かめていない。この項目5の本文は、当時の記録として書き換えていない。

6. **2026-09-28、ユーザーが初回駆動の方法を決めた（[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)）。**判断者はユーザー（wachi-yoshitaka-11-dev）であり、決定の記録は#17のコメントにある。**あわせてユーザーは、項目5の「実行しない」について「そのルールを出していない」と述べた**（#17、[#488](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/488)）。**上の1〜5の記録は書き換えていない。**

   決定の内容は次のとおりである。
   - #17（電気特性）、#18（可動域・速度・加速度の校正）、#19（実機limiterの監視下試験）は、**servo専用の別電源に故障電流を制限する手段を置いて、はんだ付け（[#405](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/405)）より前に行う。**`PROT-OC-01`の実装を待たない。
   - 初回は無負荷・単発commandから始め、人間が立ち会って監視する。
   - 故障電流を制限する手段は手持ち部品から選び、**根拠（一次資料または実測）を確認してから決める。**

   **項目5の2つの理由に対する扱い。**
   - **短絡:** 故障電流を制限する手段を経路に置くことを条件にする。[Hardware Safety Policy](../governance/hardware-safety-policy.md)の`安全要件の5項目`は変えない。
   - **循環:** 最初の駆動は、上の条件を満たしたうえで`HW-TBD-010`等を測るための監視下の測定として行う。ユーザーが判断した。

   **現在の状態: その回の実行承認が無い間は、駆動しない。**（2026-10-13に改めた。それまでは「まだ実行しない。次の(a)〜(c)がそろうまで、この項目による駆動を始めない。」だった。）(a)(b)は下の記録のとおりそろった。(c)は、reviewと、駆動の回ごとの実行承認に分ける（下の(c)の記録）。**駆動してよいかは、その回の実行承認の記録（[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)のcomment。判断者・出所・内容の形）が持つ。**この節が承認の状態を持つこと（この節の冒頭）は変えない。この節が定めるのは、回ごとの実行承認のcommentをその回の状態の記録とする規則であり、駆動のたびにこの節を書き換えない。**上の項目4の「進めてよい」は、2026-09-22の承認（項目1〜4）の範囲の語であり、この項目による駆動には使わない。**
   **2026-10-19追記（ユーザーの作業指示）:** ユーザーは、#17を完成させるための実機作業を1回だけ行い、測定用buildの指令角を規約上の90度に変えてAC投入時と動作時の電流を測り、reset回数を数えるというPMの判断を承認した。この追記とbuildの変更は、実機での動作・測定を記録するものではない。これまでの105度指令から軸が105度付近にあるという読みは未確認であり、90度がこの個体の機械的中央かどうかも未確認（`HW-TBD-010`）である。回ごとの実行承認を[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)のcommentで確認する条件と停止基準は維持する。
   - (a) servo専用の別電源と、故障電流を制限する手段を選び、手段・値・根拠をこの節へ記録する。
     **(a)の記録（2026-09-28、ユーザー決定。購入なし）。**決定の記録は[#488](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/488)のコメントにある。
     - **別電源:** `M-12001`（5 V／3 A）を、servoの試験の間だけservo専用にする。Piは外す（試験にPiは要らない）。ESP32はPCのUSBから給電する。
     - **故障電流を制限する手段:** 手持ちの予備のPTC（`PROT-OC-01`と同じBourns `MF-R135`）を、servo電源の経路に直列に入れる。
       `Ihold` 1.35 A／`Itrip` 2.70 A（23 ℃）、`5×Ihold`＝6.75 Aでの最大トリップ時間7.3秒（MF-R Series datasheet REV. AG。
       sha256 `7d980de97abdadc53465454d48a39c6926f1287a2fcfca77d0520a7512245fe4`、[hardware-bom.md](hardware-bom.md)の`PROT-OC-01`の記録と一致）。
     - **弱点（ユーザーが記録したうえで採った）:** `M-12001`の出力定格は3 Aであり、[power-budget.md](power-budget.md)はこれを短絡時に流れる電流として扱っている。`MF-R135`の3 A付近のtypのトリップ時間は1000秒前後以上である
       （同文書の`3 Aでのトリップ時間を読み、線材と嵌合接点の判定で止まった（2026-08-27）`節）。
       `M-12001`の短絡時の特性は公開されていない（[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)の2026-09-28の調査コメント）。
       **したがって短絡時に実際に電流を止める手段は、人間が電源を抜くことである。**この試験は人間の監視を前提とし、無人で続けない。
       **ただし人間が電源を抜くのでは、配線と接点の発熱に間に合わないことがある。**[Hardware Safety Policy](../governance/hardware-safety-policy.md)の`安全要件の5項目`の「短絡」は、connectorと線材の発熱は秒から分のorderで進み、人が読み取って電源を落とすのでは止まらない、としている。
     - **経路の部品（2026-09-28、CodeRabbitの指摘を受けて追加）:** 上の弱点を小さくするため、servo電源の経路は`M-12001`の出力定格3 Aに耐える手持ちの部品だけで組む。端子台（`114217`、定格6 A。[hardware-bom.md](hardware-bom.md)の`WIRE-PWR-01`）と線材（`111641`、AWG22相当。servo railの5 V線として周囲30 ℃・2条密着で約6.6 A。[power-budget.md](power-budget.md)の`経路部品と定格`）を使い、**breadboardとjumper wireをservo電源の経路に使わない。**`MF-R135`の`Imax`は40 Aである（同datasheet）。**これで経路が守れるのは、短絡時の電流が`M-12001`の出力定格3 Aを超えない場合だけである。**`M-12001`の短絡時の特性は公開されていない（上記）。**servo本体のリード線とconnectorの定格は資料が無く、弱点として残る。** **2026-11-03追記：**#18の段0・段1は、この条件のうち、servoのplugへ行く2本のjumper wireとシャントだけを例外とする。breadboardは通さない（[#18の校正（案）](#18の校正案)の段0の2。ユーザーの決定）。**2026-11-11追記：**段2も、`PSU-BENCH-01`の CC の間に限り、同じ例外とする（[#18の段2（案）](#段234-の頭を付け可動域を片側ずつ広げる)。ユーザーの決定）。
     - 配線の方法（PTCの置き場所、GNDの共通化の方法）は、(b)では決めていない。初回の実行承認（(c)）を求めるときに、配線図とあわせて示す。
     - **ジャンパー線（2026-10-17追記。[EXP-028](experiment-log.md#exp-028-sg90の初回のサーボ動作の試み測定用buildジャンパー線の接続2回試みた)）:** 上の「経路の部品」の条件（breadboardとjumper wireをservo電源の経路に使わない）は、AIが2026-09-28のPR #500で、CodeRabbitの指摘を受けて足したものである。ユーザーの継続承認の範囲に書いてあるのは、breadboardを通さない条件だけであり、jumper wireは無い（[#17のcomment 5870421147](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17#issuecomment-5870421147)の`範囲`の(2)）。jumper wireを使わない条件は、AIがPR #500のdiffで、その範囲の外へ足したものである。**この条件は、この追記では書き換えない。**EXP-028では、ユーザーの決定（[#17のcomment 6222108076](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17#issuecomment-6222108076)。#17の初回のサーボ動作とその前後の確認）で、サーボ電源の経路（端子台Sとサーボのplugの穴のあいだ）に、両端オスのジャンパー線を使った。**この条件と食い違ったまま使った。**ジャンパー線の導体の太さと定格は、記録に無い。この条件の目的は、短絡や拘束のときに、経路が発熱しないことである。EXP-028の時点で、その目的に対して置いたものは、直列のPTC（`MF-R135`。3 A付近でtripが遅い）、人間の立ち会いとACプラグを直ちに抜けること（上の弱点のとおり、配線と接点の発熱に間に合わないことがある）、単発300 ms、停止基準である。ジャンパー線の導体が耐える電流は、記録に無く、PTCの特性や`M-12001`の出力定格3 Aと比べられない（未確認）。短絡や拘束での発熱は、検証していない。EXP-028では、端子台Sの−の穴に3導体（黒い線、サーボの茶のジャンパー線の針、ESP32のGNDのジャンパー線の針）を入れた。ジャンパー線の針の断面積は記録に無く、0.64 mm角と仮定した計算では、1つの穴の導体の断面積の合計の目安（AWG18＝0.8230 mm²）を超える。引いて抜けないことは、ユーザーの申告である
     - **サーボの線の並び（2026-10-17追記。[EXP-028](experiment-log.md#exp-028-sg90の初回のサーボ動作の試み測定用buildジャンパー線の接続2回試みた)）:** 赤＝＋5 V、茶＝GND、黄＝信号は、**慣行であり、仮説である。**メーカー（TowerPro）の資料に線の説明は無く（作業セッションが2026-10-14に取得して読んだ、公式のSG90 Digitalと Analogの製品ページの記載は、`Servo Plug: JR (Fits JR and Futaba)`と`servo wire length: 25 cm`まで。取得したHTMLのsha256の先頭16桁は、Digitalが`ba35a5dd53879b20`、Analogが`b243fb8a28371761`）、袋・同梱物・本体にも説明は無い（ユーザーの申告。[#17のcomment 6166559786](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17#issuecomment-6166559786)）。無電源の抵抗の測定は、黄−茶＝約13 kΩ（向きを替えても同じ）、赤は黄・茶の両方に対して両方向とも開放（`OL`）だった。電流制限つきの通電（電源の経路に330 Ωを入れた）で、端子台Sの＋−間が+2.10 V（赤の穴の側が高い向き）、330 Ωの両端が+3.22 V（`3.22 V÷330 Ω`の資料値の計算で約9.8 mA）だった。**これらは、慣行と矛盾しない、までである。並びを確定させる実測は無い。**ユーザーの決定（[#17のcomment 6222108076](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17#issuecomment-6222108076)。測定せず、慣行を決定とする）は、出所として残す。**この決定は、[Hardware Safety Policy](../governance/hardware-safety-policy.md)が一次資料または実測を要する水準の根拠ではなく、この記録は、その水準の例外として置いたものではない。**並びが違えば、サーボが壊れうるほか、信号の線からESP32のGPIO27へ5 Vが加わりうる（信号用の330 Ωに5.0 Vが加わり、GPIO27がLowを出力している場合、電流は約15.2 mA、記録の5.35 Vなら約16.2 mAとなりうる。これは計算上の可能電流であり、安全な許容電流ではない。ESP32のclamp電流の許容値は未確認）
   - (b) 別電源の構成に合わせて、下の`初回動作の実行手順`を改める。現行の手順は`M-12001`からの分岐と`PROT-OC-01`を前提に書かれている。測定用のbuildも、項目5が挙げた「hard limitの迂回」に当たらない形（試験の範囲を狭い仮の制限で強制する等）にする。あわせて、[サーボ出力を有効化してよい条件](#サーボ出力を有効化してよい条件)（有効化条件の正本）に、この測定のための駆動をどう扱うかを書く。**それまで同節の「すべて解決するまで出力を有効にしない」がそのまま適用される。**
     **(b)の記録（2026-09-28）。**下の`初回動作の実行手順`を(a)の構成に合わせ、[サーボ出力を有効化してよい条件](#サーボ出力を有効化してよい条件)へ`測定のための駆動（承認の状態の項目6）`を足した。
     **測定用のbuildのfirmware側の変更（`compile_error!`の扱いを含む）はまだ行っていない。**別のPull Requestで行い、それまでこの項目による駆動は始めない。
     **2026-10-13追記: 行った。**#474の`compile_error!`と、項目5が挙げた注記を外した。測定用のbuildの動作（1回の起動で1 command、動作角のclamp、露出時間の後にdutyを0、NVSのlatch）は変えていない。このbuildはlimiterを通らない。limiterを通らないことは、上の「試験の範囲を狭い仮の制限で強制する」で扱った（limiterを通す駆動は#18・#19のbuildで扱う。[測定のための駆動](#測定のための駆動承認の状態の項目6)の`build`）。
   - (c) (a)(b)の変更をPull Requestでreviewし、ユーザーが実行を承認する。
     **(c)の記録（2026-10-13）。**(c)を2つに分ける。**review:** (b)のfirmware側の変更のPull Requestで行い、ユーザーがその内容を読んでmergeを承認する。**実行承認:** 駆動の回ごとに取り、[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)のcommentに判断者・出所・内容の形で保存してから通電する。承認を求めるときは、配線図（PTCの置き場所、GNDの共通化）、手順、停止基準、firmwareの版、起動の回数を示す。

   **`firmware/esp32/src/main.rs`の#474の`compile_error!`は、(a)〜(c)がそろうまで外さない。**（2026-10-13追記: この文のとおりに読むと、外すPull Requestのreview自体が(c)の一部であるため、外せなかった。**`compile_error!`は、(c)のreviewのPull Requestで外した。外しても、flashとservoの通電は、その回の実行承認まで行わない。**）外すときの手順は項目5の、外す注記を挙げた段落（「再開の条件は、この節では定めない。」で始まる段落）と、その後の2026-09-30追記に従う。

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
項目6による駆動では、「進めてよい」の代わりに、その回の実行承認の記録で読む（同節の項目6）。

**2026-09-28、項目6の条件(b)としてこの手順を別電源の構成に合わせた。**電源は項目6の(a)の構成（`M-12001`を試験の間だけservo専用にし、予備の`MF-R135`を直列に入れる。Piは外す）である。下の残余riskのうち、`PSU-SERVO-01`の共用、`PROT-OC-01`が経路に入っているか、故障電流を制限する手段が無いこと、を述べた3項目は、この構成で扱いが変わる（各項目の末尾の2026-09-28の追記）。駆動してよい条件の全体は[サーボ出力を有効化してよい条件](#サーボ出力を有効化してよい条件)の`測定のための駆動（承認の状態の項目6）`が持つ。

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
  **2026-09-28追記: 項目6の構成では、試験の間`M-12001`をservo専用にし、Piは外す。**`通電前の現物確認`の(a)は記録ではなく、満たさなければ試験を始めない条件になる。
- `hardware-bom.md`の`PROT-OC-01`（Bourns MF-R135、`Ihold` 1.35 A／`Itrip` 2.70 A）は
  5 V ingressの過電流保護として`Selected`だが、**この部品がこの試験の物理経路に
  実際に入っているかはBOMからもこの文書からも確認できない。**入っていれば、servoの
  動作電流（`TBD`。目安は0.5–2 A、上記参照）は保持帯とtrip帯の間に架かりうる。
  `通電前の現物確認`で人間が現物を確認して記録する。
  **2026-09-28追記: 項目6の構成では、予備の`MF-R135`をservo電源の経路に直列に入れる。**`通電前の現物確認`の(b)は、満たさなければ試験を始めない条件になる。
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
  対し、この試験は止まらずに進む。**扱いが異なることを明示する。**（2026-09-25追記: 「この試験は止まらずに進む」は#474より前の扱いである。この試験を行うかは[承認の状態](#承認の状態)が持つ。flash済みのESP32の扱いも同節にある。）**実際に電流を
  制限しうるのは、
  [hardware-bom.md](hardware-bom.md)の`PROT-OC-01`（PTC、Bourns `MF-R135`。
  `Ihold` 1.35 A／`Itrip` 2.70 A。値の正は同文書、ここへ再掲しない）が経路に
  物理的に入っている場合だけである。**入っているかは`通電前の現物確認`で人間が確認するまで
  分からない。**入っていなければ、電流を制限するものは無く、人間の手動遮断だけが
  残る。**入っていても「2.70 Aでtripするから安全」とは言えない。**`MF-R135`の
  `Max. Time to Trip`は5×`Ihold`（6.75 A）で7.3秒であり、**動作電流の目安である
  0.5–2 A付近の電流ではtripまでの時間ははるかに長い**（一般にPTCのtrip時間は電流が低いほど
  長くなる。具体的な秒数は`hardware-bom.md`にも記載が無く、この文書では算出しない）。
  **2026-09-28追記: 項目6の構成では、故障電流を制限する手段は直列の`MF-R135`である。**ただし`M-12001`の出力定格3 A付近ではtypのトリップ時間が1000秒前後以上であり（項目6の(a)の記録）、**短絡時に実際に電流を止める手段は、引き続き人間が`M-12001`を抜くことである。**
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
- **`DISP-01`を接続しないことを引き続き求める。**2026-09-23（[#461](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/461)）で、
  `HW-TBD-024`は「`DISP-01`を3.3 V系へ通常接続すること自体」を一律に妨げる扱いではなくなった。
  **それでもこの試験でDISP-01を接続しない理由は別にある。**この試験はservoの拘束・過負荷を
  対象とする試験であり、`DISP-01`を足すと3.3V railの状態にこの試験が説明しない変数が
  加わる。試験を単純に保つため、`DISP-01`は引き続き接続しない。
  [#451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)より前、
  `run_display_bringup`は既定buildでも常時実行され`lcd.backlight_on()`を呼んでいた。
  **`#451`で同関数は`bringup-display-13`か`face-21`のfeature（既定off）付きbuildだけの経路になり、
  [#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)で製品build（featureなし）も加わった。
  `bench-servo-test-17`単独のbuildは、製品buildではなく（featureを指定すると製品経路のcfgが外れる）、
  それらのfeatureも付けないため、firmwareがbacklightを点ける経路は無い。**確認は下の`通電前の現物確認`(d)で行う
  （backlight以外に残る理由もそちらが持つ。**ここへ再掲しない**）。

**対象部品の識別（Digital／Analog）は行わない**（2026-09-22、ユーザー決定。
「識別できた」のではない。経緯は[tbd-register.md](tbd-register.md)の`HW-TBD-026`が正本）。

**この手順のstepは名前で参照する。番号は目次の役割だけで、本文中の相互参照には
使わない**（stepを1つ足すたびに番号がずれ、参照が壊れることを防ぐため）。

**決定事項（reviewer・実行者は以下を前提として読む。蒸し返さない）**

- 拘束を止めるのは人間による外部5 V電源の遮断であり、`arm delay`や`露出時間`の
  待機時間そのものではない。
- `arm delay`（既定10秒）は安全な秒数の主張ではなく、人間が確認の語を入力してから
  手を離して見守るための運用上の猶予である。安全要件5項目のどれにも効かない。
  **2026-10-17から、firmwareは起動だけではservoを動かさない。**人間がUART0のconsoleへ`GO17`を
  入力して改行した後にだけ、latchを立て、`arm delay`の後に1回動かす（`main::wait_for_bench_confirmation`）。
- SG90のstall電流はメーカー資料に無く`TBD`である。0.5–2 Aはメーカー回答由来の
  **動作電流**の目安であり、stall電流ではない。stall電流は一般に動作電流より高い。
- `通電前の現物確認`（(a)(b)(c)）はgateではなく記録である。結果がどうであれ
  試験を中止しない（中止基準は`停止基準`が別に定める）。**2026-09-28追記: 項目6の構成では、(a)（Piを外している）と(b)（`MF-R135`が経路に入っている）は満たさなければ試験を始めない条件である。(c)(d)の扱いは変わらない。** **(c)（`RES-PULL-01`未実装）
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
   **2026-09-28追記: 項目6による駆動では、同項目の条件(a)〜(c)がそろい、その回のユーザーの実行承認があることを確認する。**
   **2026-10-13追記: 項目6による駆動では、上の「進めてよい」の確認を、その回の実行承認の[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)のcomment（判断者・出所・内容）があることの確認で読む。**
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
   **2026-09-28追記: 項目6の構成では、(a)はPiが`M-12001`から外れていること、(b)は予備の`MF-R135`がservo電源の経路に直列に入っていること、あわせてservo電源の経路が端子台と線材だけで組まれ、breadboardとjumper wireを通っていないことを確認する（#18の段0・段1は、servoのplugへ行く2本のjumper wireとシャントだけを除く。段2は`PSU-BENCH-01`の CC の間に限り同じ。breadboardは通さない。[#18の校正（案）](#18の校正案)の段0の2と[#18の段2（案）](#段234-の頭を付け可動域を片側ずつ広げる)）。**どれかを満たさなければ、servoの電源を入れずに報告する。
   (c) `RES-PULL-01`（GPIO27の外部pull-down、4.7 kΩ）が現物に実装されているか。
   (d) `DISP-01`（MSP2807）がESP32の`3V3` pinへ接続されていないことを確認する。
   `HW-TBD-024`は2026-09-23（[#461](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/461)）で
   「`DISP-01`を3.3 V系へ通常接続すること自体」を一律に妨げる扱いではなくなったが、
   この試験（servoの拘束・過負荷を対象とする）は`DISP-01`を試験変数に含めないため、
   接続されていないことを引き続き確認する。接続されていると
   ESP32へのUSB接続と同時に`DISP-01`のlogic側へ給電される。
   **backlightについては、この試験のbuildにfirmware側の点灯経路が無い**
   （[#451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)。
   `run_display_bringup`は`bringup-display-13`か`face-21`のfeature付きbuildだけが呼び、
   手順`build`はそれらのfeatureを付けない）。**ただし(d)の確認はそれでも省かない。**
   GPIO4がreset中Lowへ確定することも
   ESP32側の信号レベルまでしか確認できていない
   （[gpio-assignment.md](gpio-assignment.md)の`信号inventory`の`LCD-BL`行、
   [experiment-log.md](experiment-log.md)の`EXP-011`。**ここへ再掲しない**）。
   **(a)(b)(c)(d)の結果は残余riskの該当項目を変えるが、実行可否は変えない**
   （上記「決定事項」参照）。
4. **`電源準備`（人間）** サーボの電源をまだ入れない。ESP32側だけ電源を入れられる
   状態にする（ESP32はPCのUSBから給電する。servoの外部5 V系とは電源を分離する）。
   **2026-09-28追記: 項目6の構成では、servoの電源は`M-12001`（servo専用）から`MF-R135`を経て供給する。**
   手やcableを、サーボが動きうる範囲の外へ置く（[Hardware Safety Policy §6](../governance/hardware-safety-policy.md#6-サーボ)のPWM出力前7項目の1つ）。
5. **`build`（AI。flashは人間の立ち会いのもとでAIまたは人間）** 既存の105度指令には`cargo build --locked --features bench-servo-test-17`、2026-10-19に承認された90度指令には`cargo build --locked --features bench-servo-90-17`を使って
   AIがfirmwareをbuildし、ESP32 Flash／HIL profileの端末でflashする（`espflash`）。**flashの
   実行者は、[power-budget.md](power-budget.md)の`ACCEL-01`／`ENV-01`単体bring-upの手順8と同じである**
   （[#489](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/489)。書き込みのcommandと、USBの接続・
   給電を止める判断を、同手順が分けて定めている）。AIがこの端末を持たない場合、この工程は人間が行う。
   `再武装`と`終了処理`の再flashも同じである。このfeatureを付けない通常buildでは
   `crate::servo`は`main()`から呼ばれず、GPIO27は駆動されない。build検証の状況は
   PR本文参照。
   `run_servo_bench_test`はESP32起動のたびに呼ばれるが、**起動だけではservoを動かさない。**
   latchが立っていなければ、人間がUART0のconsoleへ`GO17`を入力して改行するまで待つ
   （違う語や改行だけでは動かない。consoleの読み取りに失敗したら動かさずに抜ける）。
   書き込みの後のreset、記録用の読み取りがportを開いたときのDTR／RTSのreset、brownout resetでは、
   入力が無いので動かない（[EXP-028](experiment-log.md#exp-028-sg90の初回のサーボ動作の試み測定用buildジャンパー線の接続2回試みた)で、
   起動のたびに自動で動く作りが、書き込みの手順とDTR／RTSの扱い次第で意図しない起動を動作にしうると分かったため）。
   入力を受けた後、**NVSへ実行済みflagを実際に動かす前にcommitし**、resetのたびの再実行を防ぐ。
6. **`起動とarm delay`（人間）** ESP32を起動する（flash直後）。consoleで
   `servo_bench_test_waiting_for_confirmation`のlogを待つ。出たら、サーボの外部5 V電源を入れ、
   `監視維持`の状態を確かめてから、同じconsoleへ`GO17`を入力して改行する。`servo_bench_test_confirmed`と
   `servo_bench_test_nvs_latch_set`が出た後、`config::SERVO_BENCH_TEST_ARM_DELAY_MS`（既定10秒）の
   `arm delay`の後に1回動く。**入力するまでservoへ信号は出ない**（`Sg90::new`は入力と`arm delay`の後）。
   **このlogが出るまでの時間に、
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
   **`GO17`を入力した後は、動かす意図でservoの電源を入れ直さない。**`停止基準`に当たれば、直ちに外部5 V電源を遮断する（この遮断は、この文に妨げられない）。`arm delay`終了後も
   `Sg90::stop`までの短い間（既定300 ms）はduty出力が残っており、「`arm delay`を
   過ぎたら絶対に動かない」とは言えない（`停止基準`の「動かなかった場合」はこの
   windowも逃した場合を含む）。
   **2026-10-17追記（[EXP-028](experiment-log.md#exp-028-sg90の初回のサーボ動作の試み測定用buildジャンパー線の接続2回試みた)。実施の記録で、この手順を変えるものではない）:** 書き込みを`espflash flash --after no-reset`で行った直後は、ENを押したあとの出力が無く、`espflash reset`のあとにダウンロードモードの出力（`boot:0x3 (DOWNLOAD_BOOT...)`、`waiting for download.`）が出た。1回目の試みは、この状態だった可能性が高い（起動ログが無く、確かめていない）。pyserialでDTRを解除し、RTSでリセットを保持・解除すると、appが起動した。記録用の読み取りは、pyserialでポートを開くと、開いた直後にESP32がリセットされる。`stty -hupcl`を指定して`cat`で読むと、起動中のappの出力は読めて、リセットの出力は出なかった。2回目の試みは、ESP32をリセットしたままサーボの電源を先に入れ、そのあとでリセットを解除して起動した。**この順序は、当時のこのstepの本文（logを待ち、arm delayの間に外部5 V電源を入れる。同日の`GO17`の変更より前の版）の外である**（EXP-028は`手順書からの逸脱2`として書いている）。作業セッションの案を、PMが了承し（VMも了解）、ユーザーの指示（原文「覚えてないから再計測白」）の範囲として、作業セッションが書き添えた（[#17のcomment 6224969168](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17#issuecomment-6224969168)。「順序を変える：ユーザーが先にACを挿し…そのあとでENを押す」は、作業セッションの文で、ユーザーが見て違えば止める、と書いた）。この間、GPIO27は未設定で、実際のレベルとpull-downの有無は確かめていない。**この追記は、順序の変更を手順として認めるものではない**
7. **`監視維持`（人間、`起動とarm delay`と並行して行う）** `起動とarm delay`の
   電源投入と同時に、直ちに遮断できる状態を維持する（別のstepとして順番に行う
   ものではない）。動作の合図は人間の`GO17`の入力だけであり、AIへの実行指示は無い
   （firmwareは入力と`arm delay`の後に1回だけ動かす）。
8. **`単発動作`（firmware、`GO17`の入力と`arm delay`の後に実行）** `Sg90::move_to_angle_once`を
   1回だけ呼ぶ短いtest経路を使う。既存の`bench-servo-test-17`単独では、規約上の中央から
   `config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG`（既定15度、一次資料にも実測にも
   基づかない暫定値）だけ+側の105度を指令する。`bench-servo-90-17`では、
   規約上の中央である90度を指令する。**90度が機械的中央であるとは確認していない**
   （`HW-TBD-010`）。指令後に実際に軸が動くか、動く向きと角度は観察する。
   `config::SERVO_BENCH_TEST_EXPOSURE_MS`
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
    `_waiting_for_confirmation`が出たらlatchが効いていない（入力しなければ動かない。servoは無給電の
    ため物理的にも動かない。**このときconsoleへ`GO17`を入力しない**）。`_partition_failed`／`_open_failed`／`_get_failed`／
    `_set_failed`が出た場合はNVSアクセス自体の失敗として「効いているとは確認できない」
    扱いにする。
    **latchが効いている場合を除き、servoの外部5 V電源を入れずに報告する**
    （`終了処理`の通常buildへの復帰まで再試行しない）。結果はAIへ伝え、`記録`で
    まとめて[experiment-log.md](experiment-log.md)へ記録する。
    **2026-10-17追記（[EXP-028](experiment-log.md#exp-028-sg90の初回のサーボ動作の試み測定用buildジャンパー線の接続2回試みた)）:** 1回目の試みのあと、作業セッションが、ACが抜けているかを確認せずに、ポートを開いてESP32をリセットした。ポートを開く、閉じる、リセットする操作の前に、「servoの外部5 V電源は抜けているか」を確認する。latchは、2回目の試みの動作のあとの再起動で効いた（`skipped_already_ran`）。1回目の試みのあとの最初の再起動で、動作の指令の行まで進んだことは、1回目の試みでbenchが走っておらず、その再起動が最初の実行だった、と読めば整合するが、確かめていない。「latchが効かなかった」とは言えない
12. **`再武装`（人間）** 繰り返す場合は改めて承認を取ってから再武装する。**NVSの
    flagが立ったままでは再実行されないため**（`servo_bench_test_skipped_already_ran`）、
    NVSを消去してから再flashする（`espflash`の`erase-flash`または`erase-parts nvs`
    相当。対象board・partition名を実行前に確認する。検証状況はPR本文参照）。
    承認を得てから`起動とarm delay`へ戻る。承認無く連続動作・再試行はしない。
    **2026-10-17追記（[EXP-028](experiment-log.md#exp-028-sg90の初回のサーボ動作の試み測定用buildジャンパー線の接続2回試みた)）:** `espflash erase-parts nvs`は、partition tableの指定が要り、失敗した。NVSの領域（起動ログのpartition tableでは、`0x9000`から`0x6000`バイト）を`espflash erase-region 0x9000 0x6000`で消し、先頭の64バイトが`0xff`であることを読み戻して確認した

    **2026-09-30追記（[#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)のPR B1）: latchは、人間の
    承認なしに消えうる。**#487のPR B1以降、`bench-servo-test-17`以外のすべてのbuild（既定buildを含む）は、
    起動のたびに`generate_sid`（`firmware/esp32/src/main.rs`）から`EspDefaultNvsPartition::take()`を呼ぶ。
    起動のときだけではなく、`stale_session`を受けて`sid`を選び直すとき（`firmware/esp32/src/boot_session.rs`の
    `BootSession::reselect_sid`）にも、実行中に`generate_sid`を呼ぶ。
    これは`reinit=true`で初期化し（esp-idf-svc 0.52.1 `src/nvs.rs`）、`nvs_flash_init()`が
    `ESP_ERR_NVS_NO_FREE_PAGES`または`ESP_ERR_NVS_NEW_VERSION_FOUND`を返すと、default partition全体を
    `nvs_flash_erase()`で消してから再初期化する。**latch（namespace `bench17`、key `ran`）も同じpartitionに
    あるため、このときlatchも消える。**`終了処理`で通常buildへ戻した後も同じである。同じ消去の経路は
    bench build自身の`run_servo_bench_test`の起動にもある。firmwareの中で`bench17`／`ran`を読むのは
    `run_servo_bench_test`だけであり、**firmwareの中には、bench buildを書き込む前にlatchの状態を確かめる
    手段は無い。** **したがって、bench buildの書き込みは、latchが立っているはずの場合も含めて毎回
    「servoが1回動きうる操作」として扱う。**この扱いは、既存の手順の順序（`電源準備`でservoの電源を
    入れる前にESP32側を準備する）と、繰り返すときの改めての承認（この手順12）の中に入る。この追記は、
    値と手順の条件を足さず、上の手順12と各stepの本文を書き換えていない。
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

## #18の校正（案）

> **案である。まだ誰も承認していない。**[測定のための駆動](#測定のための駆動承認の状態の項目6)の`build`の項が求める、#18の校正で使うbuildの**動作と仮の制限**の下書きである。**手順は段0・段1・段2を書く**（下の「手順・停止基準・受け入れ条件」）。荷重をかける段などは、別のPull Requestで足す。
> 承認の状態は[承認の状態](#承認の状態)が持つ（ここへ再掲しない）。**この節が書いてあることは、駆動してよいことを意味しない。**駆動の回ごとの実行承認（項目6の(c)）が要る。
> firmwareの実装は別のPull Requestで行い、**この節がmergeされた後に着手する。この節の「校正buildの仕様」だけを仕様にする。**buildの名前（feature名）は`bench-servo-calib-18`を案とする。
> **値は「出所あり」（記録を指す）、「運用上の値」（出所は無い。監視下の測定のための仮の制限で、回ごとの実行承認が決める。5項目の根拠にはしない。小さい側・短い側へ倒した）、「TBD」（決め方を書く）に分ける。AIの記憶では決めない。**

### 前提（出所つきの事実。#17）

| 項目 | 値 | 限界・出所 |
|---|---|---|
| 指令して動いた範囲 | 角度規約（一般値。確定値ではない）の90°と105°。無負荷（ホーンなし）、露出300 ms | 規約上の90°が機械的中央かは未確認。[EXP-028](experiment-log.md#exp-028-sg90の初回のサーボ動作の試み測定用buildジャンパー線の接続2回試みた)、[EXP-045](experiment-log.md)、[EXP-049](experiment-log.md)、[#17の判定comment 6474881537](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17#issuecomment-6474881537) |
| 動いた向きと量 | 時計回り（軸を上から見て）、90°指令で約90°、105°指令で約105°（ユーザーの申告） | 目視。動く前の位置は不明。**機械的中央・可動域の根拠にしない** |
| PWM（実測） | 周期19.97〜20.07 ms、パルス幅1.56〜1.66 ms（16パルス、301.5 ms） | 刻み約0.104 ms。[EXP-030](experiment-log.md)の段1・段1b |
| 電流（無負荷） | AC投入 約0.51〜0.73 A、動作 約0.76〜0.86 A（山の幅 約144 ms） | 絶対値の確度は主張しない。[#17の判定comment 6474451236](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17#issuecomment-6474451236)。連続電流との比較は、範囲を広げる段のPull Requestで書く |
| 熱・異音・唸り | 無し（ユーザーの申告） | 荷重をかけた駆動は未実施 |
| 速度・加速度・stall電流 | **取っていない** | 無負荷の単発のみ。stall電流は取らない方針（[#17の判定comment 6474881537](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17#issuecomment-6474881537)の(d)）。**stall電流の実測は、`HW-TBD-010`／`011`で引き続き必要であり、未解決**（[tbd-register.md](tbd-register.md)） |
| 首の固定 | 発泡スチロールの天板に帯で固定。ホーンは軸に嵌めていない。頭の荷重は未確認 | [EXP-054](experiment-log.md#exp-054-首の固定方法を発泡スチロールの天板と底板に非通電で当てた)、[enclosure-and-neck.md](enclosure-and-neck.md) |

### 校正buildの仕様（案）

**firmwareの実装はこの節だけを仕様にする**（実装の担当は、この節の変更が`develop`へmergeされた後に着手する）。実装に要ることはここへ書く。**値は「出所あり」「運用上の値」「TBD」に分ける。「運用上の値」は出所が無く、監視下の測定のための仮の制限であり、回ごとの実行承認が決める。5項目の根拠にはしない（実行承認は小さくしてよい。大きくするときは、直前の回の記録を根拠に示す）。いずれも、build定数として回ごとの実行承認が決める**（下の「build定数」）。この分け方は実機側のPMの決定（2026-11-03）による。

**名前と構成**

| 項目 | 仕様 |
|---|---|
| feature名 | **`bench-servo-calib-18`**（案。`firmware/esp32/Cargo.toml`の`[features]`へ`bench-servo-calib-18 = []`。他のfeatureを含まない） |
| Pi link | **持たない。**`crate::boot_session`とUART1を、このfeatureのbuildでcompileしない。`bench-servo-test-17`と同じく、`#[cfg(not(feature = "bench-servo-calib-18"))]`で外す（製品経路の`cfg`も外れる） |
| 併用 | **単独の試験buildとして使う。**他の試験feature（`bench-servo-test-17`、`bench-servo-90-17`、`tap-21`、`debug-motion-19`、`mem-probe-23`、`bringup-*`、`*-sample-*`、`touch-*`、`face-21`ほか）と併用できない（`compile_error!`で理由を示して止める。`main.rs`の既存の同種の止め方に合わせる）。LCD（`DISP-01`）は通電しない（`通電前の現物確認`(d)） |
| 製品build | 変えない。featureを付けない通常buildは、`crate::servo`を呼ばず、GPIO27を駆動しない |
| servoを駆動する型 | **`LimitedServo`（`Limiter`を通る）だけを使う。**`Sg90::move_to_angle_once`（limiterを通らない）をこのbuildから呼ばない。`bench-servo-test-17`の`GO17`・NVSのlatch・単発の経路をこのbuildへ持ち込まない |
| `Sg90::new`を呼ぶ時点 | `GO18`の入力を受けた後。それまでGPIO27は未設定のまま（#17と同じ） |

**build定数（`firmware/esp32/src/config.rs`の`SERVO_CALIB_18_*`）。**値は**回ごとの実行承認**が決める。実行承認で決まった値を書いたものが、その回のfirmwareの版になる（書き換えはその回の承認の範囲で行い、`承認の状態`の項目6の(c)に従う）。**値が未設定の間、このfeature付きの実機buildができてはならない**（`compile_error!`等。型検査・host testのための値を、実機buildに入る形で置かない。方法は実装が決めてPR本文に示す）。

| 定数 | 意味 | 値 | 決め方・出所 |
|---|---|---|---|
| `APPROVED_MIN_DEG`／`APPROVED_MAX_DEG` | limiterの承認値の位置範囲（角度規約の度）。hard boundも同じ値にする（範囲を変える＝再build・再flash） | **TBD**（回ごと） | **範囲は、全ての回で、実測した機構の制限（非通電で手で測る）からmarginを引いた位置の内側でなければならない**（広げる回に限らない）。**ホーンや頭など荷重をかける回は、その状態で測った機構の制限による。**ホーンなしの回（軸の周りに機構が無い回）は、**#17で指令して動いた90°〜105°の内側**で足りる（`APPROVED_MAX_DEG`が105°を含む）。測り方とmarginの決め方は範囲を広げる段のPull Requestで書く。その外へ広げる量は、直前の回の記録を見て、その回の実行承認が決める。位置範囲の不変条件は、既存のlimiterが検査する |
| `START_DEG` | `LimitedServo::new`へ渡す起動時の位置（承認値の範囲の内側） | **TBD**（回ごと） | 前の回の最後のlogの`position_deg`を第一にする。人が目盛で読んだ位置と食い違ったら、その回の実行承認で扱う。**最初の回は105°**（[EXP-049](experiment-log.md)の指令値。到達位置は未確認）。`APPROVED_MAX_DEG`が105°未満の回は、`START_DEG`を範囲の内側へ合わせる（外だと`LimitedServo::new`が拒否する）。**実機の位置と違えば、最初の出力はlimiterの速度制限を受けない**（`servo.rs`のmodule doc。動く量の上限は、実機の位置が不明なため決まらない。実機の位置が承認値の範囲の外にあれば、範囲の幅を超えうる）**同じ版のまま再起動して`GO18`を入れると、新しい回の`LimitedServo::new`には、flashした時点の`START_DEG`が渡る（前の回の終わりの位置ではない）。**そのため、回が終わった後、次の`GO18`の前に、(i) 前の回の最後のlogの`position_deg`で`START_DEG`を書き換えてflashし直すか、(ii) 人が目盛で軸が`START_DEG`にあることを確かめるか（**(ii)で軸が`START_DEG`にあると確かめられなければ、(i)を行う**）、のどちらかを行う（どちらを行うかは実行承認が決める。buildに、再起動後に`GO18`を受けない仕組みは足さない。NVSを使わない決定と両立しないため） |
| `NEUTRAL_DEG` | `ServoLimits::new`へ渡すneutral（承認値の範囲の内側でなければならない） | **範囲が90°を含むときは90°**（規約上の仮定。運用上の値）。含まないときは、範囲の内側の値（`START_DEG`と同じでよい） | **検証していない。**機械的neutralとして記録しない（`HW-TBD-010`） |
| `MAX_STEP_DEG` | 単一commandの最大変化量（`Cap`の承認値） | **2°**（運用上の値。約11 µs。規約の線形変換で） | 実行承認がこの値を大きくする場合も、最初の回は#17で指令した2点の差（15°）を超えない。目盛の読みの分解能より大きいことの確かめ方は、範囲を広げる段のPull Requestで書く |
| `MAX_VELOCITY_DEG_S` | limiterの速度上限 | **10°/s**（運用上の値） | servoの実際の速度は取っていない。2°の移動は、加速と最初の`tick`の基準取り（出力しない）の分、算術の200 msより長くかかる。収まらなければ6の`exposure_elapsed`。**TBD：上げた後の値**（上げ方と見るものは、範囲を広げる段のPull Requestで書く） |
| `REACHED_TOLERANCE_DEG` | 「目標へ着いた」と判定する、位置と目標の差 | **TBD**（最初の回の値は実行承認が置く） | LEDCのdutyの1刻みに相当する角度より大きく、`MAX_STEP_DEG`より小さい値 |
| `MAX_ACCELERATION_DEG_S2` | limiterの加速度上限 | **TBD**（最初の回の値は実行承認が置く） | 手持ちに角加速度を測る手段が無い。**決め方：**最初の回は低い側から置き、行き過ぎ量・追従・音を見て上げる（範囲を広げる段のPull Requestで書く）。`acceleration_bound_conceded`が立たない組み合わせ（速度上限、制御周期との関係）は、実装が`Limiter`の仕様から確かめる。**測っているのは、上限を課したときの様子であり、servoの角加速度そのものではない** |
| `CONTROL_PERIOD_NOMINAL_MS` | `ControlPeriod`の公称 | **20 ms**（運用上の値。PWM周期に合わせる選択。[EXP-030](experiment-log.md)は、PWM周期の実測19.97〜20.07 msであり、制御周期の出所ではない） | |
| `CONTROL_PERIOD_TOLERANCE_MS` | `ControlPeriod`の許容幅 | **TBD**（最初の回の値は実行承認が置く） | **最初の回は、この値を決めるための回でもある**（`tick`の間隔をlogに出す）。外れると（各tickの間隔は`LimitedServo`が、窓の間隔は`Limiter`が検査する）rejectしてlatchし、dutyは0になる。間隔はlogに残るので、次の回で値を決める |
| `EXPOSURE_MS` | 1 commandの露出時間の期限（移動時間を含む。期限は制御の周の単位で判定するため、厳密な上限ではない） | **300 ms以下**（#17で駆動した値〔無負荷・ホーンなし、熱と異音の申告なし。安全の根拠ではない〕。ホーンなしより荷重をかける駆動は**TBD**） | 荷重（ホーン・底板・頭）をかけた値は#17に無い。その回の実行承認が、直前の記録（電流の山、熱、唸り）を見て決める |
| `MAX_COMMANDS` | 1回の起動で受理するmotion commandの数（N） | **10**（運用上の値） | その回の目的に要るstep数が10を超えるときは、回を分ける（実行承認が決める） |
| `MIN_COMMAND_INTERVAL_MS` | 前のcommandが終わってから、次を受けるまでの最短間隔 | **5000 ms**（運用上の値） | 人が結果を読む時間。`秒あたり受理command数`（`HW-TBD-020`）の仮の代わり |
| `MAX_SESSION_S` | `GO18`から回を打ち切るまでの時間 | **TBD**（実行承認が、AC挿しの時間の上限から、AC挿しから`GO18`までの時間と、抜くまでの余裕を引いた値で置く） | 回がこの値まで続いても、AC挿しの時間の上限に先に当たらない値にする |

**起動から終了までの動作**

1. 起動する。**PWMを出さない（GPIO27を設定しない）。**`servo_calib_18_waiting_for_confirmation token=GO18`をlogに出し、UART0のconsoleで`GO18`（完全一致）を待つ。違う語・改行だけ・長すぎる行（16 byte超）は捨てて、`servo_calib_18_confirmation_rejected reason=mismatch|too_long`を出す。consoleの読み取りに失敗したら、**動かさずに終える（再起動まで何も受けない。`Sg90::new`を呼ばない）。`servo_calib_18_confirmation_failed error=...`をlogに出す。**待つ間の期限は置かない。**resetと書き込みとportを開いたときのresetは`GO18`を入力しないので動かない。NVSのlatchは使わない**（NVSは他のbuildの起動で消えうる。初回動作の実行手順の`再武装`stepの2026-09-30追記）。
2. `GO18`を受けたら`servo_calib_18_confirmed`と、build定数の値（全部）を1行のlogに出す。`Sg90::new`、`LimitedServo::new`（`START_DEG`、`NEUTRAL_DEG`、上の制限、制御周期、catalog＝`calib`の1つ）を作る。作れなければ`servo_calib_18_limits_rejected error=...`を出して終える（PWMを出さない）。
3. 以降、1つのloopで、(a) consoleの読み取り（別threadで読みmain loopへ渡す。`crate::debug_motion`と同じ形。`STOP`をtickの間隔の内で受けられるように、blockingで読まない。**読み取りthreadが失敗した場合（`ConsoleEvent::ReadFailed`、threadを起こせない）は、`servo_calib_18_console_failed error=...`を出し、dutyを0にして回を終える**）、(b) 動いている間は制御周期ごとに`LimitedServo::tick`を回す。
4. 受けるcommandは**3種だけ**。大文字小文字を区別する。1行の長さの上限は既存の`MAX_DEBUG_LINE_BYTES`。
   - `MOTION calib <deg>`：`<deg>`は角度規約の度（小数可）。形は`deskcat_servo::parse_debug_line`で読む。**`calib`以外のmotion名は、下の5の最初の判定で`UnknownMotion`として拒否する。**
   - `STOP`：**進行中の動きを中止して、直ちにdutyを0にし、その回を終える**（下の「回の終わり」）。
   - `END`：動いていないときに、dutyを0にして、その回を終える。動いている間に受けたら`Busy`で拒否する。
   - それ以外の行は`UnknownCommand`で拒否し、logに残す。**拒否の名前は、`deskcat_servo::Rejection`の変数名をそのまま出す（`UnknownMotion`、`TargetOutOfHardRange`、`Busy`など）。校正buildが足す拒否は`UnknownCommand`、`RateLimited`、`ExceedsMaxStep`である。****無言で捨てない。**
5. `MOTION calib <deg>`を受けたときの判定（この順）：(0) motion名が`calib`でなければ`UnknownMotion`、(i) 動いている間は`Busy`（N個目の動作中に届いたものもここ）、(ii) 前のcommandの終了から`MIN_COMMAND_INTERVAL_MS`未満なら`RateLimited`、(iii) 目標と現在位置（直前に出した位置。回の最初は`START_DEG`）の差の絶対値が`MAX_STEP_DEG`を超えるなら`ExceedsMaxStep`（**校正buildが自分で行う判定。**limiterの`max_step`は`Limiter::step`1回（N tickの窓）分のclampで、これではない。`ServoLimits`の`max_step`には`MAX_STEP_DEG`を渡す）、(iv) `LimitedServo::admit`が拒否したらその理由。拒否はどれも、dutyに触れず、動作を始めず、回も終えない。**次の目標は、前のcommandのlogの`position_deg`から`MAX_STEP_DEG`以内に置く**（着かずに終わった、または行き過ぎた後は、`ExceedsMaxStep`になりうる。人がlogを見て選び直す）。受理したら`servo_calib_18_command_accepted n=<何個目> target_deg=<値>`をlogに出す。
6. **1 commandの動作：**受理した目標へ、`tick`ごとに間隔を検査し、N tickに1回（`SERVO_OUTPUT_DECIMATION_TICKS`）だけ、`Limiter`を窓（最後の出力から今回の出力までの時間）の間隔で進めて、その位置をLEDCへ出す。**目標へ着く（出力した位置と目標の差が`REACHED_TOLERANCE_DEG`以内の状態が、出力2回続く。N＝3なら最短6 tick）か、`EXPOSURE_MS`の期限に達した時点で`LimitedServo::stop`を呼び、dutyを0にする**（期限は周の単位で判定するため、厳密な上限ではない）（次のcommandは`LimitedServo::resume`で、`Stopped`が持つ最後の位置から作り直す）。dutyを保持しない。着いたら`servo_calib_18_motion_done reason=reached`、着かずに露出が尽きたら`reason=exposure_elapsed`。`Limiter`は目標の手前で減速しないため、加速度の上限が低いと行き過ぎて往復しうる（位置は承認値の範囲の内側）。`MAX_STEP_DEG`は目標の指定の上限で、行き過ぎの分を含まない。`position_deg=`（最後に出した位置）、`position_min_deg=`・`position_max_deg=`（そのcommandの間の最小・最大）、`pulse_us=`（位置から線形変換で計算した値。LEDCの量子化とclampの後の値ではない）、`elapsed_ms=`を出す。**limiterのcounter（`LimitedServo::counters`）の値を`stop`の前に取り、`stop`の後に`servo_calib_18_limiter_counters`としてcommandごとにlogへ出す**（dutyを先に0にするため。`stop`で捨てるため、回全体の値はlogを足して読む）。動作中に拒否した行のlog（`servo_calib_18_command_refused`）も`stop`の後に出す。**診断用に、次の時刻の列を`stop`の後にlogへ出す。値はすべて、そのcommandの`motion_start`からの経過時間（µs）である。`servo_calib_18_tick_trace`（`at_us=`）は各tickの時刻（基準のtickと、間隔の検査でrejectしたtickを含む。`tick`を呼ぶ直前に取った値で、`tick_min_ms`・`tick_max_ms`の元になる値と同じ）、`servo_calib_18_wake_trace`（`woke_us=`）は待ち（`delay_ms`）から戻った直後の時刻である。間隔が伸びたとき、待ちから戻るのが遅れたのか、戻った後の処理で時間を使ったのかを分ける。どちらも`n=`・`count=`・`dropped=`・`first=`を持ち（時刻が0件のときは`n=`・`count=0`・`dropped=`だけで、`first=`と時刻の項目は出ない）、1行に16個まで、配列の長さ（`EXPOSURE_MS`と最短の間隔から決まる件数。64件を超えない）を超えた分は`dropped=`に数える。さらに、どこで時間を使ったかを分けるため、`servo_calib_18_phase_trace`（`after_tick_us=`：`tick`が返った直後、`pre_delay_us=`：`delay_ms`を呼ぶ直前。それぞれ`tick_trace`・`wake_trace`と同じ番号で対になり、`tick`が失敗した周の`after_tick_us`も残る。形は上の2つと同じ）と、`servo_calib_18_stop_trace n= stop_pre_us= stop_post_us=`（`stop`の直前と直後。`motion_start`からのµs）を、同じく`stop`の後に出す。起動時には`servo_calib_18_thread_info`（main taskとconsole読み取りthreadのcore・名前、`CONFIG_FREERTOS_HZ`など）を1回出す。tick数とtaskの優先度の実値は出さない（取るには`unsafe`が要る）。判定・許容・出力の順序は変えない。****`stop`の失敗は握りつぶさずerrorで出し、その回を終える**（人が外部電源を切る）。
7. **回の終わり**（次のどれか。終えたら**dutyを0にして、再起動まで何も受けない**。`servo_calib_18_session_end reason=... position_deg=<`Stopped::resume_position`>`を出し、`ServoFaultCounters`（`stop`と`resume`を越えて残る）を出す。limiterのcounterは上の6でcommandごとに出す）：`STOP`、`END`、`MAX_COMMANDS`個目の動作の終了（それまで次のcommandは上の5の(i)で`Busy`）、`MAX_SESSION_S`の経過、faultのlatch（`TickError`のどれか）、`stop`の失敗、consoleの読み取りの失敗（上の3）。**終えた後に届いた行は、`servo_calib_18_ignored_after_end len=<byte数>`をlogに出して捨てる**（無言で捨てない。動かさない。Nの後の拒否として、段0の5で観察する）。**`MAX_SESSION_S`の経過で動いている最中なら、動きを中止してdutyを0にする。**
8. **fault：**`LimitedServo::tick`のエラー（測った間隔のreject、加速度の譲り、PWM出力の失敗）は、既存の動きどおりlatchし、`LimitedServo::stop`を呼び、回を終える。実機で確かめていないことは`servo.rs`のmodule docが持つ。
9. logは全部UART0（`crate::console`）。**logの名前は上のとおり固定する**（人がgrepして記録する）。commandごとに`tick`の間隔の最小・最大（ms）も出す（`CONTROL_PERIOD_TOLERANCE_MS`を決めるため）。

**この節の値は、`動作制限`表の承認値ではない。**表の承認値と設定可能なhard boundは、#18の測定が終わった後に、実測を根拠にPull Requestで埋める。

### 段0で分かったこと：`tick`の間隔が必ず外れる原因（[#705](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/705)）

**校正 build の`tick`の間隔が、制御周期20 msの許容を必ず外れ、limiter が reject して latch する。原因は jitter ではなく、LEDC の更新の遅れである。**判断は[ADR-0027](../decisions/0027-servo-output-decimation-for-ledc-update-latency.md)。

- ESP32 の LEDC では、`ledc_set_duty_and_update`が、前の更新が効くまで **critical section の中で busy-wait** する（ESP-IDF v5.5.3 `components/esp_driver_ledc/src/ledc.c:1596-1611`、待ちの本体は`components/hal/esp32/include/hal/ledc_ll.h:483-489`の`while (conf1.duty_start);`）。**その core の割り込みも止まる。**
- 実測では、`duty_start`が立ってから**2回目の PWM の周期の頭**（立てた位相に依って約20〜40.2 ms後。周期の頭の直後に立てた場合は約40.1〜40.2 ms。[EXP-057](experiment-log.md#exp-057-段0の3回目の窓704入りの校正buildでthreadphasestopの時刻の列を取り動作が約141-msと約101-msで止まった)の1回目の回で、立てた`duty_start`が落ちるまでは、最初が約32.3 ms、続く2つが約40.2 msと約40.1 ms。最初のtickは出力せず、`duty_start`を立てない）に落ちる。**この「2回目」の理由は ESP32 の TRM で確認していない**（log とパルス数への当てはめ）。PWM の周期は 約 20.06〜20.12 ms（`set_duty`の戻りの時刻から。[EXP-030](experiment-log.md)の実測 19.97〜20.07 ms と近い）。
- **duty の register への書き込みは、busy-wait の前に行われる**（`ledc.c:1606`）。一方、low speed では`ledc_ls_channel_update`（para_up）が待ちの後に来る（`ledc.c:982`）ので、**source の順序だけでは、出力に出る時刻は決まらない。**[EXP-058](experiment-log.md#exp-058-段0の4回目の窓書き直さない校正buildで動作1回ごとのパルスの本数をarduinoで数え同じ回のlogと並べた)のパルス数（4本・4本・2本）は、**待ちの前に書いた値が、`duty_start`が立っている間、周期の頭ごとに出力に出る**と読むと合った（値が引き金の後に見えると読むと、6本・6本・4本になって合わない）。**観察からの当てはめで、TRM は未確認。**
- **上の読みなら、`stop`（`set_duty(0)`）が効くのは、呼んだ次の周期の頭（最大1周期）。**`stop()`が戻るのは、前の更新が効いた後（出力した tick の直後なら最大約40 ms）。`stop_post_us`は`stop()`が戻った時刻であり、信号が止まった時刻ではない。**STOP の入力から信号が止まるまでの時間の同期した実測は、まだ無い**（実機側が持つ）。
- 制御周期20 ms・PWM 50 Hz では、2回目の出力から`set_duty`が最大約23 ms止まり、間隔が約30〜40 msになる。**必ず起きる。**
- **対処（[ADR-0027](../decisions/0027-servo-output-decimation-for-ledc-update-latency.md)、実装は[#708](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/708)）：**LEDC への出力は N tick に1回に間引き、**`Limiter`も出力する tick でだけ進める。`Limiter::step`の`dt`には、最後の出力から今回の出力までに測った時間（窓）を渡す**（ADR-0027 の追記。はじめは「`Limiter`は毎 tick 進める」だったが、揺れる間隔で決定3を満たせなかったため改めた）。**N は`firmware/esp32/src/config.rs`の`SERVO_OUTPUT_DECIMATION_TICKS`（設計の定数。3から始める。回ごとの実行承認の値ではない）で、`servo_calib_18_confirmed`の`output_decimation_ticks=`に出す。**2へ変えるのは code の変更（Pull Request）で行い、下の出力した tick の時間で判断する。**この表の`CONTROL_PERIOD_*`の値は、この節で変えていない。**
  - **各 tick の間隔は毎回検査する**（間引いた tick でも。許容の外なら reject して latch する）。`Limiter`が持つ`ControlPeriod`は、`SERVO_CALIB_18_CONTROL_PERIOD_*`をN倍した窓の許容（公称 N×、許容 N×）で、新しい承認値ではない。各 tick の間隔が許容の内側なら、N個の和は（f32の丸めの境界ぎりぎりを除き）窓の許容の内側に収まる。境界ぎりぎりで外れたときは、`Limiter::step`が reject して latch する（安全側）。
  - **N の下限の検査（compile 時）は必要条件までである。**`N × 制御周期 ≥ PWM の周期 × 2`を`config::decimation_meets_pwm_floor`が検査する（校正 build は`SERVO_CALIB_18_CONTROL_PERIOD_NOMINAL_MS`と`SERVO_PWM_FREQUENCY_HZ`で。N＝3・20 ms・50 Hz なら60 ms ≥ 40 ms）。**十分条件（実際の更新の遅れ 約40.1〜40.2 ms を超えること）は検査できない。**実際の PWM の周期は LEDC の timer 分周の丸めで決まり（約20.06〜20.12 ms）、「2回目の周期の頭」で落ちる理由も TRM で未確認なためである（N＝2 は必要条件を満たすが、40.2 ms に届かない周で busy-wait が入りうる）。その確認は、次の出力した tick の時間の log で行う。
  - **出力した tick は`servo_calib_18_output_trace`（`tick=`：`tick_trace`の何番目か、0始まり）で分かる。**その tick の`after_tick_us − at_us`（`servo_calib_18_phase_trace`と`servo_calib_18_tick_trace`）が、`set_duty`を含む`tick`の時間である。**段0で、出力した tick の時間が全部小さいことを、実機側が確かめる**（ユーザーの承認を得て、窓で）。
  - **閾値の案（#708の実装が置く。実機側が段0で確かめて確定する）：出力した tick の時間が全部 2 ms 以下。**間引かなかった実測では、`set_duty`の待ちが最大約23 ms 出た。**N＝3（60 ms）なら、`CONFIG_FREERTOS_HZ`＝100 の1 tick（10 ms）の量子化で間隔が50 msまで縮んでも約40.2 msを下回らない（20 msの余裕。ADR-0027の選択肢Aのコスト）ので、待ちは起きない見込みである。**待ちが残ったなら、その見込み（約40.2 msという読みなど）が外れている。N＝2（40 ms）にした場合は、間隔が約40.2 msをわずかに下回る周で、0 msから約10 msまでのどの長さの待ちも入りうる。どちらの場合も、小さい待ちを見落とさないよう、閾値は低く置く。待ちが無いときの`Limiter::step`と`set_duty`の処理時間は 1 ms に届かない見込みだが、**未測定である**。**2 ms を超える出力した tick が1つでもあれば、busy-wait が残っている疑いとして段1へ進まず、同じ周の他の core・task の処理（`wake_trace`などの時刻）と分けて見たうえで、N（または出力の形）を見直す。**根拠は上の観察からの見積もりで、実測した処理時間ではない（段0の log で確かめ、この値を直す）。
  - **`Limiter`は出力する tick でしか進まないので、latch しない限り、`Limiter`の位置と最後に出力した位置は同じである**（加速度の譲りと出力の失敗で latch したときだけ`Limiter`が 1 step 先へ進むが、再開は最後に出力した位置から作り直す）。`position_deg`・`position_min_deg`・`position_max_deg`・`reached`の判定・`stop`の後の位置は、出力した tick の位置だけで更新する。`reached`は、出力した位置が許容の内側に`REACHED_TICKS`回続いたときである。
  - **速度と加速度の上限は、出力の列を出力の間隔（窓）`T_k`で数えて、各 tick の間隔が許容の内側で揺れても超えない**（ADR-0027 の決定3）。出力の列が`Limiter`の測った間隔で制限した列そのものだからである。`crates/deskcat-servo/tests/decimation.rs`が、N（1〜6）、各 tick の間隔の揺れ（許容の90%まで）、再 target の位相を総当たりして確かめている（`acceleration_bound_conceded`も立たない）。数えるのは`Limiter`の内部状態（f32の位置の差分から求めると、100付近の分解能で20 msの全 step でも50.011が出るため）。参考：窓の中心どうしの間隔で数えると、揺れた間隔では上限を超えうる（間引かない`Limiter`も、step 自身の間隔で制限するため同じ。総当たりで、許容 ±2 ms で43.6、±10 ms で58.0）。
  - **代償：**出力（servo へ届く位置）の更新が N tick に1回になるのは、間引きそのものの代償で、旧設計と同じ。この変更で増える代償は、`Limiter`の速度の変化が窓の単位になること（1回の出力で変えられる速度は`最大加速度 × 窓`まで）と、窓の許容が N 倍に広がるので境界への接近の見積もりが保守的になること（違反ではなく遅さ）。

### 手順・停止基準・受け入れ条件

**この節は段0・段1・段2を書く。**段1は、校正 build と limiter が仕様のとおりに動くかを確かめる段であり、受け入れ条件はどれも取らない。段2は、#34 の頭（最終の荷重）を付けて可動域を片側ずつ広げる段である。それより先の段は、別の Pull Request でこの節へ足す。**それまで、段2の範囲（機構の制限から margin を引いた内側で、片側の上限 45°〜135°）の外への駆動はしない。**

**各回の最初に、その回の実行承認（[承認の状態](#承認の状態)の項目6の(c)）を取る。**承認を求めるときは、配線、手順、build 定数の値、firmware の版、起動の回数（port を開いたときの reset と、段0の5の再起動も数える）、AC挿しの回数と1回の時間の上限、動かす回の数、停止基準を示す。

#### 段0：組み直しと照合（駆動ではない）

1. 項目6の(a)の構成で組む（`M-12001`を servo 専用、`MF-R135`を直列、Pi はつながない、ESP32 は PC の USB、`DISP-01`は3.3 V へつながない）。**段1の servo の電源だけは、`M-12001`ではなく`PSU-BENCH-01`にする**（下の[段1の前の発熱の実測](#段1の前のservo-電源の経路の測れる区間の発熱の実測案)。段0は servo へ通電しないので、AC も出力も入れない）。
2. **段0・段1では、servo 電源の経路を、下の配線で組む**（ユーザーの決定、2026-11-03。原文「普通にオスオスのジャンパ線でいいんじゃないの？今までと同様。」。#17 の [EXP-045](experiment-log.md)・EXP-047・EXP-048・[EXP-049](experiment-log.md) で使った端子台まわりの配線である）。
   - **組む配線（端子台の6穴）：**A の赤＝`M-12001`の赤と PTC（`MF-R135`）の足、A の黒＝`M-12001`の黒と黒い線（W−）、B の穴1＝PTC の足と赤い線（W+）、B の穴2＝シャント（`MEAS-01`）の足2と、servo の plug の茶へ行く両端オスの jumper wire の針、S の＋＝W+ と、servo の plug の赤へ行く両端オスの jumper wire の針、S の−＝W− とシャントの足1（[EXP-045](experiment-log.md)の`組んだ配線`）。
   - **この配線に入れないもの：**EXP-049 の測定のための緑の jumper wire と分圧、組み直す前の板から出ていた基準の線のクリップ、Arduino（`MEAS-02`）の線（**段0の4と、窓の (A)・(D) の読みの間だけつなぐ（下の小節1の窓の順）。段1の配線には入れない**）、組み直す前の板から servo の黄へ行っていた jumper wire（`X4`に替える）。
   - **例外：**この配線は、「servo電源の経路は端子台と線材で組み、breadboardとjumper wireを使わない」という条件（`測定のための駆動`の電源の項、[承認の状態](#承認の状態)の項目6の(a)の`経路の部品`、`通電前の現物確認`の2026-09-28追記）のうち、servo の plug へ行く2本の jumper wire とシャントだけの例外である。**breadboard は、servo 電源の経路に入れない。**
   - **残る risk：**jumper wire の導体の太さと定格は記録に無く、短絡や拘束のときの発熱は確かめていない（項目6の(a)の弱点と`ジャンパー線`の追記のとおり）。端子台の6穴にはそれぞれ2本が入り、B の穴2は導体の断面積の合計が目安の上限を超える（EXP-045 の記録。針の断面積は仮定）。**範囲を広げる段（ホーンを付ける段以降）では、この例外を当てない。**ただし段2は、`PSU-BENCH-01`の CC の間に限り、この例外を当てる（[段2](#段234-の頭を付け可動域を片側ずつ広げる)の`電源の経路`）。
   - **段1へ進む条件：**jumper wire の定格は、[Hardware Safety Policy](../governance/hardware-safety-policy.md)の`5項目の中での根拠の水準`で資格として要る数（B）に当たり、公開値が無い（B-2）。最悪値になる`M-12001`の短絡時の電流も公開されていない。そのため、**段1の servo の電源を`PSU-BENCH-01`（電流上限 `I_lim` を設定できるベンチ電源）に替え、`I_lim`で定常の電流を押さえる**（出力コンデンサの放電による瞬時の電流は除く）（ユーザーの決定、2026-11-07。問い「段1の servo の電源を、今の AC アダプタ（M-12001）から、ベンチ電源（PSU-BENCH-01、Jesverty SPS-3010V）に替えてよいですか。」への回答の原文「はい」）。**段1でこの配線を通して servo へ通電する前に、下の[段1の前の、servo 電源の経路の測れる区間の発熱の実測（案）](#段1の前のservo-電源の経路の測れる区間の発熱の実測案)の記録を残す（発熱の実測 (B) は行わず、置き換えた一覧の (iv) による。[EXP-060](experiment-log.md)）。jumper wire の針と servo の plug の接点、servo 本体のリード線（測れない区間）は、この実測では扱わない。比べる相手は[#711 の決定](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6707654178)による（段1は OCP を無効にして CC で運用する。`I_lim`は[決定の補足](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6741451276)の2〜3のとおり、段1の前の窓の (D) で置いて固定し、段1の窓の中で上げない。ここでは議論し直さない）。**段1へ進む前に要るのは、下の`段1へ進む前に、次を確かめて記録する`の一覧のすべてである。もう1つ、段1の前に別に扱うと #711 が決めた件がある：firmware が止まったとき（hang。#712（d2b99ea）の前は、task watchdog は警告だけで reset しなかった（#711 の記録 6707317592）。#712 で PANIC=y と、calib build の main task の登録を入れた。[EXP-060](experiment-log.md)の (A) は、bench-wdt-711 の panic reset を実機で確かめた。calib が hang したときに止まることは確かめていない）の、servo の持続的拘束としての扱い**（[#711 の記録](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6707317592)。firmware の作業は #712 で行った（[#711 の 6741948327](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6741948327)）。この文書は決めない）。**その扱いが決まり、記録されるまで、段1へ進まない。**段0は servo へ通電しないため、この配線で組んでよい。
   - **次の順に行う。**(i) 端子台の6穴を現物と照合し、上の`組む配線`と違っていれば、その配線に直す（[EXP-052](experiment-log.md)の組み直しでは端子台とシャントに触れていないが、EXP-049 の窓の終わりにユーザーが機材を片付けており、何をどう片付けたかは記録に無い）。上の`この配線に入れないもの`が残っていれば、すべて外す（端子台の穴に入った線、シャントの足を挟んだクリップ、servo の plug に挿さった線を含む。緑の jumper wire は、EXP-049 の後に S の＋から抜いた記録が無い）。**servo の plug が jumper wire の針に挿さっていれば、外す。**外したことを記録し、外れたと確かめられなければ、ここで止める。(ii) ESP32 の GND と信号を、組み直しの結線表の`X5`・`X4`でつなぐ（GND を共通にする線と信号線であり、servo 電源の経路ではない）。**`X5`の端は、端子台Sの−の穴へは挿さない。**`X5`の針をクリップ付コード（`PROTO-01`）で挟み、もう一方のクリップで、S の−に入ったシャントの足を、穴と本体のあいだの金属で挟む（EXP-045 の基準の線と同じつなぎ方。穴の導体を増やさない）。(iii) servo の plug は、段1まで jumper wire の針に挿さない（段0の4。(i)で外れていることを確かめた状態のまま）。その後に段0の3を行う。
3. `通電前の現物確認`の(a)〜(d)を、配線と現物で照合して記録する。
4. **servo を接続しない状態で、PWM の波形を確かめる**（[Calibration手順](#calibration手順)の4）。servo の plug は挿さず、AC は挿さない。ESP32 に校正 build を書き（build 定数は全部に値が要る。値は段0の実行承認が置く）、`GO18` を入れ、信号線を、`X4`の servo 側の端で Arduino（`MEAS-02`）で記録する。(i) `MOTION calib <deg>` を1つ出し、その command が終わるまで待って、**周期とパルス幅**を読む。(ii) 別の `MOTION calib <deg>` を出し、その動作の間に `STOP` を送る（console へ送るのは AI。`MOTION` を送ってから、動作が終わる前の間隔で送る）。`servo_calib_18_motion_interrupted reason=stop` と `servo_calib_18_session_end reason=stop` が出ること、Arduino の記録で信号が止まったことを確かめる。**`STOP` の入力の時刻は log に出ないため、入力から信号が止まるまでの時間は、この段では測らない。**
5. 同じ状態で、**拒否と回の終わりを観察する**（servo を接続しないので動かない）。**回が終わると、校正 build は再起動まで何も受けないため、回が終わるたびに ESP32 を再起動し、`GO18` を入れ直してから次を観察する**（AC は挿さないので、再起動してよい）：`UnknownCommand`、`UnknownMotion`、`RateLimited`、`ExceedsMaxStep`、`TargetOutOfHardRange`、動作中の `Busy`（`MOTION` を送ってから、動作が終わる前の間隔でもう1つ `MOTION` を送る）、`MAX_COMMANDS`個目の後の `servo_calib_18_ignored_after_end`。拒否（`UnknownCommand`から`Busy`まで）は `servo_calib_18_command_refused` で確かめる（拒否しても回は続き、`servo_calib_18_session_end` は出ない）。`MAX_COMMANDS`個目の動作の終わりは `servo_calib_18_session_end reason=max_commands` で、その後に届いた行は `servo_calib_18_ignored_after_end`（`servo_calib_18_command_refused` は出ない）で確かめる。**校正 build の判定には host test が無いため、駆動の前にここで確かめる。**
6. **tick の間隔**（log の `tick_min_ms`・`tick_max_ms`）を記録する。`CONTROL_PERIOD_TOLERANCE_MS`は、段0の build に段0の実行承認が置いた値が入っている。間隔がその値を外れると reject して latch する（各 tick の間隔は`LimitedServo`が検査して`interval_rejections`に数え、`Limiter`の counter には入らない。窓の間隔は`Limiter`が検査する）（その回は終わる。間隔は log に残る）。段1の値は、この記録を見て段1の実行承認が置く。

#### 段1の前の、servo 電源の経路の測れる区間の発熱の実測（案）

> **案である。まだ誰も承認していない。**CV の電圧の値は、**その回の実行承認が置く**。この小節は、段1で`PSU-BENCH-01`を使うための条件を書く。

窓は1つで、順は (A) WDT、(C) GND の判断、(D) `I_lim`の読み、(B) 発熱の実測である（小節1）。**(B) は、一覧の (iv) を置き換えたため行わない（[EXP-060](experiment-log.md)）。servo は窓の最後までつながない。この小節が merge されるまで、窓を開かない。**

**1. 窓の順（1つの窓。servo は最後までつながない）。**(A)→(C)→(D)→(B) の順に行う（**(B) は行わない**。この窓を「段1の前の窓」、段1で servo を動かす窓を「段1の窓」と書く）。

   - **(A) WDT。**`bench-wdt-711`を書き、hang させて約 5 s で panic reset することと、reset の後に`X4`の信号が low のままであることを`MEAS-02`で見る。続けて calib build（[EXP-059](experiment-log.md)の config）を通しで動かし、誤 reset が無いことを見る。状態：PSU は端子台につながない（`M-12001`も無い）、servo の plug なし、ESP32 は USB、`X4`・`X5`は段0のまま、`MEAS-02`の A0 は`X4`の servo 側の端につなぐ。判定は log の時刻と記録で行う。
   - **(A) の後の手順（外したことの確かめ）。**(C) の前に、`X5`（シャントの足を挟んだクリップ）と`MEAS-02`の線（Arduino の GND と A0 のピンから出た線。[EXP-045](experiment-log.md)では、A0 の線の先をクリップでシャントの足に挟んだ）を外す。外れたことは現物で見て記録し、外れたと確かめられなければここで止める（段0の2の(i)と同じ扱い）。
   - **(C) GND の判断（9）。**PSU を端子台Aへつなぎ（7・8・12。出力 OFF）、PSU の−と ESP32 の GND の間を読んで、共通にしてよいかを判断する。**共通にしてよいと判断できなければ、(D)・(B) へ進まず、止めて報告する（段1へ進まない）。**
   - **(D) `I_lim`の読み。**(C) で共通にしてよいときだけ行う。出力 OFF で、負荷（`MEAS-01`の予備。つなぎ方は6）を plug の代わりの点に置き、`MEAS-02`の GND の線を S の−のシャントの足、A0 の線を B の穴2のシャントの足へ、それぞれクリップで挟む（[EXP-060](experiment-log.md)の (D)）。logger の hex の sha256 と`MEAS-02`の個体の照合（[決定の補足](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6741451276)の2。同じでなければ止める）は、この前に行う。電流のつまみを下げてから出力を ON にし、平坦部（9点の移動中央値の中央値）が 9 count 以下になる値を探して記録する（count のまま比べ、A に直さない）。**電流のつまみは出力 OFF の間だけ動かし、電圧のつまみには触れない。**読みの後は出力を OFF にして`MEAS-02`を外す。
   - **(B) 発熱の実測。**(D) で置いた`I_lim`のまま出力を ON にし、測れる区間の電圧降下を`DT830B`で読み、`P＝V×I`を出す（10。`I`は (D) で置いた`I_lim`で、PSU の電流表示の値も各周の開始時に記録する）。発熱は直接測れない（表面温度を測る道具は手持ちに無い）ので、区間ごとの電圧降下を時間を追って読む。比べる相手は、`I_lim`を流した実測である（jumper wire の非公開の定格とは比べない。[Hardware Safety Policy](../governance/hardware-safety-policy.md)の`5項目の中での根拠の水準`のB-2）。**測れない区間（jumper wire の針と servo の plug の接点、servo 本体のリード線）は扱わない**（[#711 の決定](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6707654178)による）。
   - **一覧の (iv) を置き換えたため、(B) は行わない（[EXP-060](experiment-log.md)）。**

**2. `I_lim`は段1の前の窓の (D) で置いて固定し、段1の窓の中で上げない**（[決定の補足](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6741451276)の2〜4。具体値は書かず、決め方だけを書く）。次の2つのうち、いちばん低い値にする。
   - **補足の2の値。**servo（`SERVO-01`）が普段の動作で続けて流す電流（EXP-045 の Run 1 の読み）を上限にし、補足の2の読み（`MEAS-02`の平坦部が 9 count 以下）になる値。
   - **`MF-R135`の`Ihold`の内側。**`Ihold`は温度で下がる（23 ℃で1.35 A、40 ℃で1.12 A。datasheet の値で、[power-budget.md](power-budget.md)が持つ）。power-budget の測定の規則に従い、`MEAS-04`で読んだ温度が40 ℃以下なら、40 ℃の`Ihold`の0.8倍（0.896 A）以下に置く。40 ℃を超えたら、続けずに止める。

**3. 段1で電流が`I_lim`に届いたとき（CC に入ったとき）。**段1は OCP を無効にして CC で運用する（5。[#711 の決定](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6707654178)の4）ので、出力は切れず、電圧が下がって電流が`I_lim`に留まる。**CC に入った回の校正値の扱い**（同決定の6）：
   - 使う：動きが止まり、CC の LED が消えた後の位置（軸の角度、指令の度と回転の対応、向き、neutral の再現）。
   - 使わない：速度、到達までの時間、追従、動作中の電流。
   - **速度と加速度の制限を測る段では、比べる相手を別に決める**（同決定の7。段1の`I_lim`ではその受け入れ条件は取れない）。

**4. CV の電圧（電圧違いのpin、5項目）。**値は**その回の実行承認が置く。「`M-12001`と同じ 5 V だから」とは書かない。**servo へ入れてよい電圧は [HW-TBD-035](tbd-register.md) が持ち、**Open である**（servo へ入れてよい電圧を確認した文書が無い。製品ページの仕様表は `Operating voltage: 4.8v` だけで、`4.8–6 V`は Q&A 欄の1文であり、[ADR-0017](../decisions/0017-what-counts-as-a-primary-source.md)の条件を満たさず採れない）。**この小節は、5 V を入れてよいとは決めない。案：回ごとの承認が置く値の上限は 5 V とし、それを超える値は置かない**（`PSU-BENCH-01`は 0–30 V を出せ、OVP は無い前提〔5〕のため）。毎回、servo をつなぐ前に、出力端で`DT830B`で測って記録する。段1で servo に通電している間（PSU の出力が ON の間）、電圧と電流のつまみに触れない。**例外は窓の (D) と、段2の窓の最初の (2) だけで、電流のつまみを出力 OFF の間だけ動かす（電圧のつまみには触れない）。**OVP は説明書に記述が無く、無い前提にする（5）。

**5. 保護機能。**段1は OCP を無効にする（[#711 の決定](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6707654178)の4）。OVP は無い前提にする（説明書に記述が無い）。

**6. 接続部材と負荷のつなぎ方。**
   - **PSU から端子台Aまで。**EXP-051 で使ったワニ口のクリップは、大電流の経路に使わない条件（`PROTO-01`の行。`WIRE-PWR-01`の端子台と線材で組む）と合わない。`WIRE-PWR-01`の線材（`111641`、AWG22相当）の端を剥き、`PSU-BENCH-01`の出力端子（ねじで締める。説明書 p.9）と、端子台Aの2つの穴（`M-12001`を外した穴）につなぐ。穴位置と線の対応は [`hardware/bench-18/wiring-table.md`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/hardware/bench-18/wiring-table.md) に載せる（`generate.py` が生成する）。クリップとブレッドボードは使わない。
   - **(D)・(B) の負荷（plug の代わりの点）。**赤い jumper wire の針 → 両端メスの jumper wire（赤側） → `MEAS-01`の予備（セメント抵抗） → 両端メスの jumper wire（茶側） → 茶の jumper wire の針の順につなぐ（ユーザーの答え「ある・嵌まる」、2026-11-09。問い「両端メスのジャンパー線は今も手元にあり、セメント抵抗（予備）の足に奥まで挿さって、軽く引いても抜けませんか？」）。メス側の接点はハウジングの中にあり、プローブが届かない。ほかの負荷抵抗は入れない。
   - **段1では**、両端メスの jumper wire を外し、servo の plug を jumper wire の針に挿す。両端メスの jumper wire は窓の (D)・(B) の間だけ使う。

**7. 給電経路の重複（5項目）。**`PSU-BENCH-01`をつなぐ前に、`M-12001`の AC プラグが抜けていることと、その線が端子台Aの穴に入っていないことを現物で確かめて記録する。端子台Aの穴に給電する電源が、同時に2つ入らないようにする。Pi はつながない。

**8. 逆極性（5項目）。**説明書は、出力端子の極性を逆にすると本機が破損する可能性があると書く（p.9）。`PSU-BENCH-01`の出力を ON にする前に、次の節点を**1つずつ**、色と穴と極性を現物で照らして記録する（PSU の端子から端子台Aまでの新しい線も、＋−を取り違えていないことを、ここで照らす）：PSU の＋ → 新しい線 → A の赤の穴（`M-12001`を外した穴） → `MF-R135` → B の穴1 → 赤い線（W+） → S の＋ → 赤い jumper wire → servo の plug（窓の (D)・(B) では、plug の代わりの負荷。6） → 茶の jumper wire → B の穴2 → シャント（足2から足1） → S の− → 黒い線（W−） → A の黒の穴 → 新しい線 → PSU の−。各穴に入る2本の導体も、どの線かを照らす。

**9. PSU の−と ESP32 の GND の共通。**段0の2の(ii)で`X5`（GND を共通にする線）はシャントの足につながっていて、S の−は PSU の−へつながるので、`X5`をつないだままでは PSU の−と ESP32 の GND が導通する。`X5`を戻して GND を共通にしてよいかは、窓の (C) で判断する。`X5`と`MEAS-02`を外した状態（小節1）で、PSU を端子台Aへつなぎ（7・8・12。出力 OFF）、`DT830B`で PSU の−と ESP32 の GND の間を、DC 2000 mV レンジと AC 200 V レンジで、PSU の電源 OFF と ON（出力 OFF）の両方で読む（計4つ）。**4つとも表示が 0（DC は ±1 digit 以内）なら共通にしてよい。1つでも読めたら、共通にせず止めて報告する（段1へ進まない）。**PC の USB の GND との回り込みの道があるかは、この読みで確かめ、判断できなければ`X5`をつながない。控え（条件ではない）：止まった場合の候補は、前面端子（− / GND / ＋）で−と GND の間が離れているかを見ること。

**10. 区間と読み方（窓の (B)）。**穴の中の導体（線の導体、jumper wire の針、メス側の接点）にはプローブが届かないため、**届く点の連なりで区切り、13区間とする**（足の無い穴〔S＋、A黒〕は隣の区間に含める）。点の名前（P1〜P14）は [`hardware/bench-18/wiring-table.md`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/hardware/bench-18/wiring-table.md) の`プローブを当てる点と区間`に載せる（`generate.py` が生成する）。
   - 区間：(1) P1↔P2（新しい線〔＋〕と、その両端の接点）、(2) P2↔P3（A赤の穴）、(3) P3↔P4（`MF-R135`）、(4) P4↔P5（B穴1）、(5) P5↔P6（W+ と両端の接点）、(6) P6↔P7（S＋の穴の jumper 側の接点、赤 jumper、針とメスの接点、両端メスの jumper〔赤側〕を1区間で）、(7) P7↔P8（負荷。抵抗の2本の足の間）、(8) P8↔P9（両端メスの jumper〔茶側〕、針とメスの接点、茶 jumper、B穴2の jumper 側の接点を1区間で）、(9) P9↔P10（B穴2の穴の、シャントの足2の接点）、(10) P10↔P11（シャント）、(11) P11↔P12（S−の穴の、シャントの足1の接点）、(12) P12↔P13（W− と両端の接点）、(13) P13↔P14（新しい線〔−〕と、その両端の接点）。
   - 出力は ON のまま、13区間を順に1回ずつ読むのを1周とする。レンジは`DT830B`の DC 200 mV に固定し、周の途中で変えない。各周の開始時刻と`MEAS-04`の温度を記録する。区間ごとの降下と`P＝V×I`、流した時間、CC の LED の状態も記録する。
   - **落ち着いた：**連続する2周で、全区間の差が 1 digit 以内。

**11. 止める条件とプローブの当て方。**次のどれかに当たれば、出力を OFF にして止める：1つの区間でも3周続けて 1 digit を超えて増えた。`MEAS-04`が 40 ℃を超えた。におい・変色。6周で落ち着かない。**プローブは片方を先に固定してから、もう片方を当て、1区間ずつ読む。**プローブが滑って S の＋と−を短絡しても、定常の電流は`I_lim`に収まる。ただし PSU の出力コンデンサの放電による瞬時の電流は、CC では制限されず、大きさはこの実測では分からない。短絡を意図して作らない。

**12. 段1の前の窓と段1の窓での PSU の設定の順。**`PSU-BENCH-01`は、電源を切る前の電圧と電流の設定値を記憶し（p.6 特長6）、電源投入時の出力状態の設定（OPN、p.7「6」）が有効だと POWER を入れた直後に出力が出る。そのため、(i) POWER を入れる前は、出力端子に何もつながない。(ii) POWER を入れ、"OPN" の LED が消灯していること、"OCP" の LED の状態、表示された設定値を記録する。(iii) 出力は OFF のまま、電圧をその回の承認が置いた値にする。電流は、段1の窓では`I_lim`（補足の2〜3の値に固定されたもの）にする。段1の前の窓では、(D) の前までは電流を小さい値にしておき、(D) の手順で置く（小節1）。(iv) POWER を切り、出力端子へ線をつなぐ（7・8）。(v) POWER を入れ直し、表示を確かめる。(vi) servo（段1の前の窓では plug の代わりの負荷）をつなぐ前に、出力を ON にして出力端で電圧を測って記録し（4）、出力を OFF にする。測った値が、その回の承認が置いた値を超えていれば（上限 5 V を超える場合を含む）、servo（段1の前の窓では負荷）をつながずに止める。(vii) 出力が OFF のまま servo の plug（段1の前の窓では負荷）をつなぎ、つないだ後で出力を ON にする（電圧が出ている出力へ servo をつながない）。

**段1へ進む前に、次を確かめて記録する（TBD。手順は小節1〜12。確かめた記録は、その回に残す）。**(i) 本体の銘板の型番が`SPS-3010V`であること。(ii) は外した（守りたいのは GND を共通にしたときに回り込みの電流が流れないことで、9の4つの読みが、つなぐ2点の間の電位差を直接見ている）。(iii) 段1で OCP が無効であること。(iv) `I_lim`と、記録のある抵抗から出した`P＝I²R`の見積もり。新しい線2本（`111641`、断面積 0.3054 mm²、約 0.0565 Ω/m で、より線は更に高い。[power-budget.md](power-budget.md)の値。各 30 cm）は往復で約 0.034 Ω、`I_lim`の設定 0.080 A で約 0.2 mW。[EXP-045](experiment-log.md)の抵抗の読み（プローブ短絡 1.0 Ω を引く。A の赤↔S の＋ 約 1.0 Ω、A の黒↔S の− 約 0.6 Ω、B の穴 2↔S の− 約 0.6 Ω。`DT830B`の抵抗レンジの確度は未取得）を足した経路の抵抗は約 2.2 Ω で、0.080 A で約 14 mW。茶の jumper wire：[EXP-035](experiment-log.md)の段2bの①（約 0.272 A で 93.2 mV。茶の jumper wire と g11 の接点を含む。電流は同じ節の計算）から、0.080 A で約 27 mV、約 2 mW（比例による計算）。EXP-035 の茶の jumper wire が今の1本と同じ個体かは、記録から言わない。赤の jumper wire：抵抗の記録は無い。設定値（4.80 V、0.080 A）から計算した経路全体の電力は約 384 mW である（段1の電圧・電流は記録していない。瞬時の電流は除く）。同じ窓の`MEAS-02`の平坦部は 4 count だった。EXP-045 の換算では約 0.04 A で、理由は確かめていない。0.080 A（PSU の CC の設定。PSU が出す電流の全体で、シャントを迂回する分も含む）を使うのは、高い側を取るためである。比べる相手：定格が公開されている部品は`MF-R135`の 40 ℃基準の上限値（[power-budget.md](power-budget.md)）で、余裕は 11 倍以上。jumper wire は、定格が非公開（B-2）である。**赤・茶の jumper wire の導体の比べる相手は TBD である。**[#711 の決定](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6707654178)の3（保守側に倒した選定）が覆うのは、servo の plug の接点とリード線であり、jumper wire の導体ではない。**比べる相手を決めて記録するまで、この例外で再通電しない。**段2では、[段2](#段234-の頭を付け可動域を片側ずつ広げる)の窓の最初の (2) の実測が比べる相手になる。その (2) の通電は、比べる相手を得るための通電であり、この禁止の例外とする。**発熱を実測した記録ではない。**この置き換えで段1を行う回は、`I_lim`を合わせる前に`MEAS-04`の温度を1回読んで記録し、40 ℃を超えていたら止める（2 の`Ihold`の条件）。[EXP-060](experiment-log.md) の回では読んでいない（未記録）。(v) `I_lim`が補足の2〜3の値に固定されていること（2）。(vi) logger の hex の sha256 と`MEAS-02`の個体の照合（(D) の前に行う。同じでなければ止める）。(vii) firmware が止まったとき（hang）の扱いが決まり、記録されていること（段0の2。#711）。(viii) 9の判断。`X5`を戻して GND を共通にできると判断できなければ、段1へ進まない。確かめて記録するまで、段1へ進まない。

**この小節が決めないこと：**段1で servo へ通電してよいこと。置き換えた (iv) の記録（[EXP-060](experiment-log.md)）と、(iv) の jumper wire の導体の比べる相手の記録が出て、その回の実行承認が出るまで、段1へ進まない。

#### 段1：無負荷（ホーンなし）、90°〜105°の内側

- **段1の servo の電源は`PSU-BENCH-01`である**（ユーザーの決定、2026-11-07）。この節より前の、`M-12001`を前提にした記述（項目6の(a)、測定のための駆動の表を含む）は、段1では`PSU-BENCH-01`に読み替える（「AC を挿す・抜く」は出力の ON／OFF。`通電前の現物確認`の(a)は、Pi と`M-12001`が外れていること）。ただし servo へ入れる電圧（HW-TBD-035）は、[小節の4](#段1の前のservo-電源の経路の測れる区間の発熱の実測案)が持つ。
- **段1は OCP を無効にし、CC で運用する**（[#711 の決定](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6707654178)）。`I_lim`は (D) で置いた固定値で、段1の窓の中で上げない（[小節の2](#段1の前のservo-電源の経路の測れる区間の発熱の実測案)）。**その`I_lim`で servo が目標に着かなければ、段1を止めて報告する**（`I_lim`を上げない。CC で遅くなるのは構わない）。CC に入った回は、止まった後の位置だけを校正に使い、速度・時間・追従・動作中の電流は使わない（小節の3）。速度を測る段では、比べる相手を別に決める。
- **最初の回は、軸が`START_DEG`にあることを確かめられない。**最後の指令は105°（[EXP-049](experiment-log.md)）だが、軸がどこへ着いたかは記録に無く（同 EXP の「主張しないこと」）、角度の規約と軸の向きの対応も駆動する前には分からない。そのため、最初の出力で軸は`START_DEG`へ、limiter の速度制限を受けずに動きうる（`START_DEG`の行）。**最初の回はホーンを付けず、軸の周りに当たるものが無いため、これを受け入れて始める。**最初の回の範囲は90°〜105°の内側とし、最初の出力の後の動きは limiter を通る。
- 2回目以降は、`START_DEG`の行の(i)か(ii)を行う。
- 軸の先に目印を付け、周りに置いた紙の目盛で位置を読む。**指令角の変化に対して軸が回る向き**（最初の回は105°からのため、指令角を減らす向きで読む）と、**指令の度の変化と軸の回転の変化が同じ大きさか**を読む。
- 1つの回で動かす向きは1つ（90°→105°、または105°→90°）。
- この段は可動域を広げないため、[Hardware Safety Policy](../governance/hardware-safety-policy.md#6-サーボ) §6の電流・電圧の記録は、範囲を広げる段の Pull Request で扱う。この段で見るのは、唸り・異音・熱と ESP32 の log（reset・brownout）に加えて、CC の LED と PSU の表示である（小節の2・3、停止基準）。

#### 段2（#34 の頭を付け、可動域を片側ずつ広げる）

**取る受け入れ条件：**#18 の1（衝突させず低速で物理的制限を確認した）、2（Software limit に明示的な margin がある）、3（Neutral を再現できる）、5（PWM-off 時の重力による動作が明確である）。4・6 は取らない（4 は速度を測る段で、比べる相手を別に決める。6 は fail-safe の段）。

**頭と首の構成：**
- 首は [EXP-054](experiment-log.md) で当てた発泡スチロールの天板と底板、頭は [enclosure-and-neck.md](enclosure-and-neck.md) の[頭の保持枠・天板の線の通り道・14線の取り回し](enclosure-and-neck.md#頭の保持枠天板の線の通り道14線の取り回し2026-11-11実機側の-pm現物に当てる前)のとおりに組む（LCD を背板に留め、VCC の線を除く13本を挿したまま天板の切り込みに通す）。これが最終の頭の荷重である。
- LCD には通電しない。VCC の線（LCD pin 1）は板から抜いたままにする（`通電前の現物確認`の (d)）。
- 頭の組み立て（同じ節の手作業）は、窓の前に非通電で行う。

**電源の経路：**段1と同じ（`PSU-BENCH-01` の CC、端子台、jumper wire の針に servo の plug）。
- 段0の2 は、段0・段1の jumper wire の例外（ユーザーの決定、2026-11-03）に、「範囲を広げる段（ホーンを付ける段以降）では、この例外を当てない」と書いている。**段2に限り、`PSU-BENCH-01` の CC で `I_lim` を固定する間は、段1と同じ jumper wire の例外を当てる**（ユーザーの決定、2026-11-11。問い「段2で、servo の電源の線に今の jumper wire をそのまま使ってよいですか？」。実機側の PM の勧めは「はい」で、理由は「いいえなら plug と端子台をつなぐ部品が手持ちに無く段2に進めない。例外を外した理由は当時の AC アダプタの短絡電流で、CC には当たらない。窓の最初に実測する」。ユーザーの「おすすめは？」の後の答えの原文「ではおすすめで」。比べる相手の電流は、続く問い「段2の jumper wire の確かめで、流す電流を 0.090 A から 0.8 A に上げてよいですか？」への答えの原文「はい」で、下の 0.8 A に改めた）。CC では、定常の電流が `I_lim` を超えない（出力コンデンサの放電による瞬時の電流は除く）。
- **jumper wire の比べる相手（B-2。[Hardware Safety Policy](../governance/hardware-safety-policy.md)の`5項目の中での根拠の水準`の「実測」）：**窓の最初に、今の赤・茶の jumper wire に、段2の最悪の定常値（`I_lim` の 0.080）の10倍の 0.8 A を、段2の1回の出力 ON の時間の上限以上流し、異常が無いことを記録する。これを、同じ個体での実測の比べる相手とし、余裕は10倍（桁が1つ）とする（[Hardware Safety Policy](../governance/hardware-safety-policy.md)の`5項目の中での根拠の水準`の B。下の`窓の最初に行うこと`の (2)）。経路の他の部品は 0.8 A の定格の内である：`MF-R135`は 40 ℃基準の上限値（[power-budget.md](power-budget.md)）の内、端子台（`114217`）と線材（`111641`）は[power-budget.md](power-budget.md#経路部品と定格)の`経路部品と定格`の行の内、`MEAS-01`は 0.1 Ω／5 W（[hardware-bom.md](hardware-bom.md)）で 0.8 A では約 0.064 W。比べるのは同じ個体であり、別の品の値は持ち込まない。異常があれば、段2を開かない。
- 手の背の確認は、上界ではなく停止基準として当てる。

**機構の制限（非通電で測る。`APPROVED_MIN_DEG`／`APPROVED_MAX_DEG` の行、決めたことの3）：**
- ホーンを servo の軸に押し込む前に、頭（底板にホーンと背板を留めたもの）を軸の上に載せ、軸に嵌めずに、軸を中心に手で回す。天板・servo 本体・線に当たる角度を、両側で、底板の下に置いた紙の分度器（1° 目盛）で読む。**servo の軸は回さない。**
- 押し込むときの軸の位置は、下の`窓の最初に行うこと`の (5) で `reached` を確かめた 90° とする。上で読んだ当たる角度を、90° からの角度の差として記録する。指令の度の増減と頭の回る向きの対応は、最初に広げる回で頭の動きを見て決める。**それまでは、両側の当たる角度のうち 90° に近い方から margin を引いた値を、両側に当てる。**
- 各回の承認範囲は、当たる角度から **5°** 引いた位置（margin）の内側にする。この位置が、その側の software limit の案である。

**窓の最初に行うこと（servo へ出力を出す前）：**
1. servo の plug を jumper wire の針から抜く。PSU の出力の先に負荷が何もつながっていない状態にする（小節12の (i)）。
2. PSU の POWER を入れる。本体の銘板の型番を読む（段1の前の一覧の (i)。[EXP-060](experiment-log.md) では取れていない）。"OPN" の LED が消灯していることを見る（小節12 の (ii)）。出力 OFF のまま、電圧の表示が、その窓の実行承認が置いた CV（段1は 4.80。上限 5 V。小節の4）であることを見て記録する（電源プラグを抜いた後に設定が残るかは、記録に無い）。違っていれば、出力を ON にせずに止める。`MEAS-04` を読み、40 ℃を超えていたら窓を開かない（小節の2 の `Ihold` の条件）。
   jumper wire の比べる相手の実測：出力 OFF のまま、負荷（`MEAS-01` の予備のセメント抵抗）を、小節6 のとおり両端メスの jumper wire で赤と茶の針の間につなぐ（小節12の (vii)）。CC の電流を、出力 OFF の間に 0.3 A → 0.6 A → 0.8 A と段を踏んで上げ、各段で出力を ON にして、手の背で jumper wire（赤・茶と両端メス）・針・負荷の接続部を確かめる。熱い、焦げ臭い、変色がある、のどれかがあれば、出力を OFF にして止め、段2を開かない。0.8 A で流す時間は、段2の1回の出力 ON の時間の上限以上とする。終わりに `MEAS-04` を読む。出力を OFF にし、電流の表示を (D) で置いた 0.080 に戻し、負荷を外す。表示が同じなら同じ設定とみなし、`MEAS-02` の読みは取り直さない（表示の1刻み 0.001 A は、(D) の読み 4 count と上限 9 count の差に比べて小さい）。
3. **段2では、小節の4 の「毎回、servo をつなぐ前に、出力端で `DT830B` で測って記録する」と、小節12 の (vi) を、(2) の電圧の表示の確認で代える**（止める条件は (2) に引き継ぐ）（ユーザーが出力端の読みを却下した。[EXP-060](experiment-log.md)）。
4. 頭を付けずに（ホーンを軸に押し込まずに）、servo の plug を針に挿し直す。このとき、plug のどの穴にどの色の針が入ったかを照らして記録する（逆極性。[EXP-060](experiment-log.md) では照らしていない）。
5. 軸の位置を決める：ホーンを付けないまま、calib build で 90° へ1回動かし、`servo_calib_18_motion_done` が `reached` であることを log で確かめる（段1と同じく、軸の周りに当たるものが無い状態であり、最初の出力が速度制限を受けないことは受け入れる）。`reached` でなければ止める。
6. 下の`機構の制限`を読む。
7. 頭を軸に押し込む（軸を回さない）。押し込んだときの頭の向きを、(5) の 90° に対応させて記録する。

**`I_lim`：**段1の前の窓の (D) で読んで置いた値（表示 0.080）に固定する。窓の最初の (2) の試験の後に 0.080 に戻した後は、段2の窓の中で動かさない。servo が目標に着かなければ、止めて報告する（[#711 の決定の補足](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6741451276)の2〜4）。

**動かし方：**
- 承認値の範囲を、90°〜105° から、片側ずつ 5° 広げる。広げるのは、上の margin の内側までとし、片側の上限は 45°〜135° とする。servo 内部の端は探さない。
- 速度・step・加速度は段1と同じ値のままにし、この段では上げない（速度と加速度は、受け入れ条件4の段で決める）。目盛の読みの分解能（分度器の 1°）は `MAX_STEP_DEG`（2°）より小さい。
- 1回は、前の回の端から新しい端へ 5° 進み、前の回の端へ戻る（`MAX_COMMANDS` 10 の内側）。90° への戻りと、下の Neutral の3回は、別の回にする。
- build は回ごとに作る。`START_DEG` には前の回の最後の log の `position_deg` を入れ、`APPROVED_MIN_DEG`／`APPROVED_MAX_DEG` にはその回の範囲を入れる（`START_DEG` の行の (i)）。書き込みは AI が行う。**加えて、各回の `GO18` の前に、人が頭の向きを分度器で読み、`START_DEG` と分度器の1刻み（1°）以内で合うかを返す**（同じ行の (ii)。押し込みの直後と、出力が止まった後に頭が重力で動いた回の後は、とくに読む）。合わなければ、読んだ角度で `START_DEG` を書き換えて作り直す。頭を付けた段2で、最初の出力が速度制限を受けずに動く量は、この読みの差（1° 以内）に収まり、margin（5°）の内側である。

**記録すること（[Hardware Safety Policy §6](../governance/hardware-safety-policy.md#6-サーボ)の「電流、電圧、noise、干渉、機械的clearance」）：**
- 電流と電圧：各回の、出力 ON の間の PSU の電流と電圧の表示（ユーザーが1行で読む）と、CC の LED の状態（確度は主張しない）。出力端の電圧は `DT830B` では読まない（上の (3)）。
- noise・干渉：唸り・異音と、ESP32 の log（reset・brownout）。
- 機械的 clearance：回の終わりに、頭と天板・servo 本体・線の隙間が「まだある」か「ほぼ無い」か（値は測らない）。

**人が見ること：**
- 当たりそうになったら、すぐ出力を OFF にする。
- 回の終わりに、上の隙間を返す。
- 回の終わりに、出力が止まった後（`servo_calib_18_motion_done` の後、duty が 0）、頭が重力で動くか、動くならどちら向きにどこで止まるかを返す（受け入れ条件5）。
- Neutral：両側が決まった後、90° へ戻す回を3回行い、目印が「同じ位置に戻る」か「ずれる」かを返す（受け入れ条件3）。

**決め方：**
- 「ほぼ無い」の答えが出たら、その側の広げを止める。そのときは、その回の端から 5° 引いた値と、上の margin の位置のうち、内側の方をその側の software limit の案にする。
- 案の値は、測定の後に `動作制限` 表を埋める Pull Request で承認を受ける（受け入れ条件7）。

**記録で代えるもの（手作業にしない）：**段1から手を入れていない配線の照合は、[EXP-060](experiment-log.md) で代える（上の (4) の plug と針の対応を除く）。

**拘束の上界（頭が天板・servo 本体・線に当たった場合）：**人が見て OFF にすることは上界ではない。上界は次の2つである（値は書き写さず、正本の行を指す）。
- 電流：`PSU-BENCH-01` の CC で `I_lim` に押さえられる（上の `I_lim`）。
- 時間：calib build は、1つの指令の後、露出の上限で出力を止める（[校正buildの仕様（案）](#校正buildの仕様案)の6、`EXPOSURE_MS` の行）。

**時間と起動の上限：**その窓の実行承認が置く USB・出力 ON の時間と起動の回数の上限は、窓の始めに、上限の時刻と数を記録する。起動は、USB の挿入、書き込み、console の open・release・reset をそれぞれ1回と数える（[EXP-060](experiment-log.md) と同じ）。上限に届いたら、途中でも止めてユーザーに訊く。

**停止基準：**下の停止基準のとおり（段1と同じ）。加えて、頭が天板・servo 本体・線に触れたら、直ちに出力を OFF にする。

#### 停止基準

[Hardware Safety Policy §6](../governance/hardware-safety-policy.md#6-サーボ)の7項目と、[初回動作の実行手順](#初回動作の実行手順人間とaiの作業順序)の`停止基準`による。次のどれかに当たれば、直ちに電源を切る（段0は`M-12001`の AC プラグを抜く。段1は`PSU-BENCH-01`の出力を OFF にする）。**段0は PSU も使わず電流と電圧を計らないため、過電流と電源の電圧降下はその場で数値では読めない。段1は区間の電圧降下と servo の電流を計らず、PSU の電圧と電流の表示を読める。**その代わりに、servo の動きが鈍る・止まる、唸り、熱、ESP32 の reset・brownout を見て止める（#17 の初回動作と同じく、人の目・耳・手の背で見る）。

- 指令したのに動かない。唸りが出る。指令していないのに動く。**最初の回で、指令角の変化に対して軸が回る向きを読んだ後は、その向きと逆に動く。**（最初の回の最初の動作では、期待する向きがまだ決まっていない。）目標を行き過ぎて戻る動き（「校正buildの仕様」の6）も、この項目に含めて止める（安全側。止まった後、`MAX_ACCELERATION_DEG_S2`の置き方を次の回の実行承認で見直す）。
- 手の背で、servo 本体・端子台・PTC・plug の接続部・servo の plug へ行く jumper wire・シャント（`MEAS-01`）が熱い（触らない。数値の基準は置かない）。
- ESP32 の意図しない reset、USB の切断、`panic`。limiter の fault。`STOP`への応答が無い。
- その回の実行承認が置いた AC挿しの時間（段1は出力 ON の時間）の上限を超えそうになる。
- ユーザーが止めると言った。

**CC に入ったこと（CC の LED の点灯）と、それによる動きの鈍り・止まりは、動作の間は停止基準に当たらない**（段1の運用。校正値の扱いは小節の3）。**ただし、その command の`servo_calib_18_motion_done`が出た後（duty が 0 になった後。小節の「校正buildの仕様」の6）も CC の LED が点いたままのときは、拘束として直ちに出力を OFF にする**（log が出ないまま止まる場合は、`STOP`への無応答、時間の上限、hang の扱いで当てる）（#711 の決定の6が使うのは、止まって LED が消えた後の位置である）。上の1つ目の「動かない」は、CC の LED が消えている（CC に入っていない）のに動かない場合に当てる。唸り、指令と逆の動き、熱、reset・brownout、`STOP`への無応答、時間の上限は、CC の間も当てる。

CC に入っていないのに動かなかった場合は、そこで止めて報告する。pulse 幅を変えて繰り返し試さない。

#### 受け入れ条件との対応

段0・段1では、#18 の受け入れ条件はどれも取らない。段2は、受け入れ条件1・2・3・5を取る（段2の節）。4は速度を測る段、6は fail-safe の段の Pull Request で取る。7は、測定の後に`動作制限`表を実測で埋める Pull Request で取る。

### 決めたこと（実機側のPM、2026-11-03。build に関わるもの）

**これは実機側のPMの決定であり、ユーザーの承認ではない。**承認は、回ごとの実行承認（項目6の(c)）と、この変更のPull Requestのmergeである。

1. **NVSのlatchをやめ、起動のたびに`GO18`を要する形にする。**NVSは他のbuildの起動で消えうるため。
2. **「運用上の値」**（build定数の表で、そう書いたもの）は、出所の無い、監視下の測定のための仮の制限であり、回ごとの実行承認が決める。5項目の根拠にはしない。
3. **範囲は、全ての回で、機構の制限からmarginを引いた位置の内側**（広げる回に限らない。ホーンなしの回（軸の周りに機構が無い回）は#17の範囲で足りる。build定数`APPROVED_MIN_DEG`／`APPROVED_MAX_DEG`）。
4. **jumper wireは、#18ではservo電源の経路に使わない**（段0・段1・段2は除く。段0・段1の例外は、この節の決定ではなく、段0の2に書いたユーザーの決定による。段2の例外は段2の節による）。改めた理由：この4を置いたとき、servo の plug と端子台を jumper wire なしでつなぐ部品を決めていなかった（[hardware-bom.md](hardware-bom.md)に記録が無い）。段1へ進む条件は段0の2に書いた。
5. 手順は、上の「手順・停止基準・受け入れ条件」の段0・段1・段2だけを書く。範囲を広げる段に関わる決定（servo内部の端を探さないこと、電流・電圧の記録、加速度の扱いなど）は、その段のPull Requestで書く（段2の分は段2の節に書いた）。

## 受け入れchecklist

**2026-10-04追記。**ベンチ電源の購入申告は[hardware-bom.md](hardware-bom.md#ベンチ電源)に記録した。
Pi最低入力電圧の復帰条件に関する現在の扱いは[power-budget.md](power-budget.md#受け入れ条件)の
`(a)を要件から外す（2026-09-06、PM決定）`節の同日追記を参照する。
下記のPi電圧droopの項目を合格にした記録ではなく、サーボ試験の電源構成や承認範囲も変更していない。

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
| 2026-09-23 | 27 | [#461](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/461)。**`HW-TBD-024`が「`DISP-01`を3.3 V系へ通常接続すること自体」を一律に妨げる扱いではなくなったこと（ユーザー承認）に追随した。**2箇所。(1) 残余riskの`DISP-01`項を、「`HW-TBD-024`が未解決のため」という理由から「この試験がservoの拘束・過負荷を対象とし、`DISP-01`を試験変数に含めないため」という理由へ書き換えた。**`DISP-01`を接続しないという結論、確認先（`通電前の現物確認`(d)）は変えていない。**(2) `通電前の現物確認`(d)も同じ理由で書き換えた。**(d)の確認自体は省かない。**承認の状態、安全値、手順の順序、停止基準は1つも変えていない。 |
| 2026-09-25 | 28 | [#474](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/474)（PR #471 のCodeRabbit review指摘）。`承認の状態`節に項目5を追記した（内容と現在の状態は同節が持つ。ここへ再掲しない）。`bench-servo-test-17` featureを`compile_error!`で塞いだことに合わせ、`確定しているproject規則`、残余riskの「この試験は止まらずに進む」の箇所（#474より前の扱いであることも書いた）、手順の`build`stepに、`承認の状態`への参照を足した。**`承認の状態`の1〜4の記録、安全値、手順の順序、停止基準は変えていない。** |
| 2026-09-28 | 29 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。**`承認の状態`へ項目6を追加した。**ユーザーが初回駆動の方法（servo専用の別電源と故障電流の制限手段を置き、はんだ付け前に、無負荷・単発・人間の監視下で行う）を決めたことを記録し、実行の前にそろえる条件(a)〜(c)を定めた。**状態は「まだ実行しない」のままであり、`compile_error!`も外していない。**`確定しているproject規則`へ項目6への参照を1行足した。項目1〜5とRevision 28は書き換えていない |
| 2026-09-28 | 30 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。**`承認の状態`の項目6の条件(a)へ、2026-09-28のユーザー決定を記録した**（購入なし）。試験の間だけ`M-12001`をservo専用にし、予備の`MF-R135`を直列に入れる。3 A付近でPTCのトリップが遅く、短絡時に実際に止める手段は人間が電源を抜くことである、という弱点を記録した。**状態は「まだ実行しない」のままであり、条件(b)(c)と`compile_error!`は変えていない** |
| 2026-09-28 | 31 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。**`承認の状態`の項目6の条件(b)を行った。**`サーボ出力を有効化してよい条件`へ`測定のための駆動（承認の状態の項目6）`を足し、ゲートを開かないまま、項目6の(a)〜(c)がそろったときだけ#17〜#19の測定に限って駆動してよい条件と、未解決の`TBD`ごとの扱いを書いた。`初回動作の実行手順`を(a)の構成（`M-12001`をservo専用、`MF-R135`を直列、Piを外す）に合わせ、`通電前の現物確認`の(a)(b)を試験を始める条件にした。**状態は「まだ実行しない」のままであり、測定用のbuildのfirmware側の変更と`compile_error!`は変えていない** |
| 2026-09-28 | 32 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。PR #500へのCodeRabbitの指摘（人が電源を抜くことを短絡保護として扱わない）を受け、項目6の(a)の弱点へ、安全要件の5項目「短絡」が述べる「人が電源を落とすのでは止まらない」を書き足した。servo電源の経路を`M-12001`の出力定格3 Aに耐える手持ちの部品（端子台、AWG22の線材）だけで組み、breadboardとjumper wireを使わないことを、測定のための駆動の条件と`通電前の現物確認`へ加えた（購入なし）。短絡時の電流が3 Aを超えない場合にしか経路を守れないこと、servo本体のリード線とconnectorの定格が不明であることは弱点として残した |
| 2026-09-29 | 33 | [#489](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/489)。`初回動作の実行手順`の`build`stepが「flashは人間」としており、[power-budget.md](power-budget.md)のbring-up手順（書き込みのcommandは、人間が立ち会って監視している状態であれば実機LinuxのAIセッションが実行してよい）と食い違っていた。flashの実行者を同文書の`ACCEL-01`／`ENV-01`単体bring-upの手順8への参照に改め（規則をここへ書き写さない）、`再武装`と`終了処理`の再flashも同じであることを書いた。**USBの接続と給電を止める判断は人間のままであり、`承認の状態`、`サーボ出力を有効化してよい条件`、停止基準、各stepの条件は変えていない** |
| 2026-09-30 | 34 | [#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)のPR B1。`承認の状態`の項目5へ2026-09-30追記を足した。`pi-protocol-mode`を廃止したため、項目5が再開の手順として残すよう求めた`pi-protocol-mode`との排他の`compile_error!`は無くなった。測定用のbuildがPiとの通信linkを持たないことは`#[cfg(not(feature = "bench-servo-test-17"))]`の構造で保っている。**servo出力のgate、承認の状態、項目5の本文は変えていない** |
| 2026-09-30 | 35 | [#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)のPR B1。`初回動作の実行手順`の`再武装`stepへ2026-09-30追記を足した。`bench-servo-test-17`以外のすべてのbuild（既定buildを含む）が起動のたびに（`stale_session`による`sid`の選び直しでは実行中にも）`EspDefaultNvsPartition::take()`（`reinit=true`）を呼ぶようになり、NVSの初期化が`ESP_ERR_NVS_NO_FREE_PAGES`／`ESP_ERR_NVS_NEW_VERSION_FOUND`を返すとdefault partitionが消え、単発latchも人間の承認なしに消えうるためである。firmwareには書き込む前にlatchを確かめる手段が無いため、bench buildの書き込みを毎回「servoが1回動きうる操作」として扱う、と書いた。あわせて項目5への2026-09-30追記に、`bench-servo-test-17`付きbuildは#474の`compile_error!`を外した後にcompileが通るかを確かめていないことを書き足し、項目6の(c)の後の「外すときの手順」と`測定のための駆動`の`build`の参照先（どちらも「項目5の最後の段落」）を、項目5へ追記を足したことに合わせて改めた。**servo出力のgate、承認の状態、値、各stepの本文と条件は変えていない** |
| 2026-10-13 | 36 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。`承認の状態`の項目6の(b)の残り（測定用のbuildのfirmware側の変更）を行った。#474の`compile_error!`と、項目5が挙げた注記（`確定しているproject規則`と`build`stepへの2026-09-25追記を含む）を外した。項目6の文のとおりだと、`compile_error!`を外すPull Requestのreview自体が(c)に含まれ、外せなかったため、(c)をこのPull Requestのreview（ユーザーがmergeを承認）と駆動の回ごとの実行承認（#17のcomment）に分け、**現在の状態を「まだ実行しない」から「その回の実行承認が無い間は、駆動しない」へ改めた。**`gate確認`と手順の冒頭の「進めてよい」の読みを合わせた。測定用のbuildの動作、動作制限表、有効化gate表、受け入れchecklistは変えていない。項目1〜5、残余riskへの2026-09-25追記、Revision 28は書き換えていない |
| 2026-10-17 | 37 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。**`承認の状態`の項目6の(a)へ、`ジャンパー線`と`サーボの線の並び`の追記を足した。**EXP-028（PR #606）の記録を出所にする。`初回動作の実行手順`の`起動とarm delay`、`latch検証`、`再武装`に、EXP-028の実施の記録（書き込み直後のダウンロードモード、ポートを開いた時のリセット、サーボの電源を先に入れた順序、latchの観察、`erase-parts nvs`が失敗し`erase-region`で消したこと、AC電源の確認なしにリセットした誤り）を追記した。`latch検証`の追記には、ポートを開く、閉じる、リセットする操作の前に、servoの外部5 V電源が抜けているかを確認する、という運用の記述も含む（記録でなく、運用の追加）。**項目6の条件の文面、手順の順序、動作制限表、有効化gate表、受け入れchecklist、承認の状態は書き換えていない。**サーボの線の並びは慣行であり仮説で、ユーザーの決定は根拠水準の例外として置いていない。ジャンパー線は、ユーザーの決定で使い、条件の文面と食い違ったままである。項目1〜5、Revision 28は書き換えていない |
| 2026-10-17 | 38 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。**測定用のbuild（`bench-servo-test-17`）を、起動だけでは動かさず、人間がUART0のconsoleへ`GO17`を入力したときだけ1回動かす形に変えた**（`firmware/esp32/src/main.rs`の`wait_for_bench_confirmation`）。EXP-028で、起動のたびに自動で動く作りが、書き込みの手順とDTR／RTSの扱い次第で意図しない起動を動作にしうると分かったため。`決定事項`の`arm delay`の行、`build`・`起動とarm delay`・`監視維持`・`単発動作`・`latch検証`の各stepを合わせて直した。latch（NVSの実行済みflag）は残し、入力を受けた後、動かす前にcommitする。動作制限表、有効化gate表、`承認の状態`、項目6の条件は変えていない |
| 2026-10-19 | 39 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。測定用の90度指令を`bench-servo-90-17`で選べるようにした。既存の`bench-servo-test-17`単独の105度指令は維持した。`承認の状態`の項目6へユーザーの2026-10-19の判断と未確認の前提を追記し、`初回動作の実行手順`の`build`・`単発動作`stepを両buildに合わせた。動作制限表、有効化gate表、停止基準、NVS latch、`GO17`、arm delay、露出時間、dutyを0へ戻す処理は変更していない。実機での動作・測定は行っていない |
| 2026-10-20 | 40 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。merge済みのPRを読み直して見つかった2点を直した。(1) **`起動とarm delay`の「`GO17`を入力した後は、servoの電源を抜き差ししない。」が、`停止基準`の「直ちに外部5 V電源を遮断する」を禁じるように読めた**（PR #608が、直前の「`arm delay`を過ぎたら、動かす意図で電源を入れない。」を置き換えた文）。意図は「動かす意図で入れ直さない」ことなので、その意図の文へ改め、`停止基準`に当たれば遮断することを同じ文に書いた。(2) **`承認の状態`の項目6の(a)の`ジャンパー線`の追記（PR #607）が、jumper wireを使わない条件もユーザーの継続承認の範囲に書いてあると読めた。**#17のcomment 5870421147の範囲にあるのはbreadboardを通さない条件だけで、jumper wireはPR #500のdiffでAIが範囲の外へ足したものである。事実のとおりに直した。手順の順序、動作制限表、有効化gate表、`承認の状態`が示す現在の状態、項目6の条件の文面は変えていない（項目6(a)の`ジャンパー線`の追記の出所の文だけを直した） |
| 2026-11-03 | 41 | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18)。**`#18の校正（案）`の節を足した。**測定のための駆動の`build`の項が求める、#18の校正buildの動作と仮の制限を、firmware実装の仕様として**案として**書いた（feature名`bench-servo-calib-18`、Pi linkを持たないこと、console command、log名、動作の順序、build定数）。値は、出所（#17の記録）のあるもの、出所の無い運用上の仮の制限（step 2°、速度10°/s、N＝10ほか。5項目の根拠にしない）。露出300 ms以下は#17で駆動した値、`TBD`に分けた。手順・停止基準・受け入れ条件との対応は、別のPull Requestで足す（それまでこのbuildで駆動しない）。実機側のPMの決定を、build に関わる範囲で書いた。ユーザーの承認ではない。**承認していない。**動作制限表、有効化gate表、`承認の状態`、受け入れchecklist、項目6の条件の文面は変えていない |
| 2026-11-03 | 42 | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18)。**`#18の校正（案）`の「手順・停止基準・受け入れ条件」に、段0（組み直し、照合、servoを接続しないPWMの波形・`STOP`で信号が止まること・拒否と回の終わりの観察）と段1（ホーンなし、90°〜105°の内側）を案として書いた。**あわせて、校正buildの仕様の2か所（counterのlogを`stop`の後に出すこと、露出の期限は周の始めで判定し厳密な上限ではないこと）を、[PR #700](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/700)の実装に合わせた。範囲を広げる段は別のPull Request。動作制限表、有効化gate表、`承認の状態`、受け入れchecklist、項目6の条件は変えていない。出所はPR #700の実装（squash `8ff836e`） |
| 2026-11-03 | 43 | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18)。**#18の段0・段1では、servo電源の経路を、#17のEXP-045・047・048・049で使った端子台まわりの配線で組むとした**（ユーザーの決定。servoのplugへ行く2本のjumper wireとシャントを含み、測定のための線は入れない。breadboardは経路に入れない）。例外は段0の2の1か所に書き、`測定のための駆動`の電源の項、`承認の状態`の項目6の(a)の`経路の部品`、`通電前の現物確認`の2026-09-28追記、決めたことの4は、段0の2を指す形にした。範囲を広げる段には当てない |
| 2026-11-09 | 44 | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18)。**段1の servo の電源を`M-12001`から`PSU-BENCH-01`に替えるとした**（ユーザーの決定、2026-11-07）。段1は OCP を無効にして CC で運用し、`I_lim`は[#711 の決定の補足](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711#issuecomment-6741451276)のとおり段1の前の窓の (D) で置いて固定する。段1で`PSU-BENCH-01`を使う条件（給電の重複、逆極性、GND の共通、段1の窓での設定の順、段1へ進む前の一覧）を案として書いた。**あわせて、段1の前の窓（WDT、GND の判断、`I_lim`の読み、発熱の実測。順は (A)→(C)→(D)→(B)）の手順と、(B) の読み方を小節1・6・9・10・11に書いた**（設計は [#18 の設計コメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18#issuecomment-6742547051)。ユーザーの承認「よい」による）。電流の名前は`I_lim`の1つにした。端子台から下流の図と結線表を[`hardware/bench-18/`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/hardware/bench-18/wiring-table.md)に置いた。承認していない |
| 2026-11-10 | 45 | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18)。**段1の前の窓で取れなかった2項目を、別の根拠に置き換えた。**(1) 段1へ進む前の一覧の (ii)（コードを抜いた状態の導通）を外した（9の4つの読みが2点の間の電位差を直接見るため）。(2) 一覧の (iv)（窓の (B) の記録）を、`I_lim`と記録のある抵抗から出した`P＝I²R`の見積もりに置き換えた（発熱を実測した記録ではない）。(3) 小節2の3つ目（発熱の実測で通した電流以下）を外した。(4) 小節1の「`MEAS-02`の線（GND と A0 のクリップ）」を直した。(5) 「段1へ進む条件」の発熱の実測の記述と、小節1の (B) に、(B) は行わず一覧の (iv) による旨を足した。(6) 一覧の (iv) に、`I_lim`を合わせる前の`MEAS-04`の確認と、赤・茶の jumper wire の導体の比べる相手を記録するまで再通電しないことを足した。記録は [EXP-060](experiment-log.md)。承認していない |
| 2026-11-11 | 46 | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18)。**`#18の校正（案）`の「手順・停止基準・受け入れ条件」に、段2（[#34](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/34) の頭を付け、可動域を片側ずつ 5° 広げる）を足した。**受け入れ条件1・2・3・5を取る。機構の制限は、ホーンを押し込む前に非通電で読み、margin 5° の内側だけを駆動する。`PSU-BENCH-01` の CC の間に限り、段0の2の jumper wire の例外を段2にも当てる。jumper wire の比べる相手は、窓の最初に同じ個体へ最悪の定常値（`I_lim` 0.080）の10倍の 0.8 A を段を踏んで流す実測とする。`I_lim` は 0.080 に固定する。押し込みの前に、ホーンなしで 90° の `reached` を確かめる。段0・段1の例外を段2に当てることと試験の電流は、ユーザーの決定による。段2では、小節の4 の出力端の読みと 12(vi) を、電圧の表示の確認で代える。決めたことの4・5と受け入れ条件の対応を合わせて直した。承認していない |
