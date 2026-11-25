# Hardware TBD Register

> 状態: Active
> 目的: 安全な実装を妨げる未確定情報を追跡する

## 優先度

| 優先度 | 意味 |
|---|---|
| P0 | 電源、配線、または最初の関連driverを妨げる |
| P1 | Featureの受け入れまたは安全な統合を妨げる |
| P2 | 後のmilestoneまで保留できる |

## 未解決項目

次の14項目は、同じ台帳内の短い項目別表で読む:
[HW-TBD-004](#hw-tbd-004)、[HW-TBD-007](#hw-tbd-007)、[HW-TBD-021](#hw-tbd-021)、[HW-TBD-022](#hw-tbd-022)、[HW-TBD-023](#hw-tbd-023)、[HW-TBD-024](#hw-tbd-024)、[HW-TBD-025](#hw-tbd-025)、[HW-TBD-026](#hw-tbd-026)、[HW-TBD-027](#hw-tbd-027)、[HW-TBD-028](#hw-tbd-028)、[HW-TBD-030](#hw-tbd-030)、[HW-TBD-034](#hw-tbd-034)、[HW-TBD-035](#hw-tbd-035)、[HW-TBD-037](#hw-tbd-037)。
その他の項目は次の表に置く。

| ID | 優先度 | 不足している情報／判断 | 必要な根拠 | 妨げる対象 | 対応Issue | Owner | 状態 |
|---|---|---|---|---|---|---|---|
| HW-TBD-008 | P0 | GPIO割り当て | Board／module回路図＋競合review | すべてのhardware driver | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)、[#803](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/803) | Joint | **004によりBlocked**（**001と003は2026-08-15、002は2026-09-06にclose。006と029も解決済み**）。下書きは[gpio-assignment.md](gpio-assignment.md)にあるが、[競合check](gpio-assignment.md#競合check)に未完了項目が残る。**001のcloseでpin配列の前提は成立した** |
| HW-TBD-009 | P0 | 電源予算とbackfeed review | 部品電流＋回路図＋測定計画 | Servoと全体統合 | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#525](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/525) | Joint | **004**、007、**021、022、023、024、025**、および**028、030、034**によりBlocked（**001と003は2026-08-15、002は2026-09-06にclose。006と029も解決済み。029は2026-08-12にclose**。下書きは[power-budget.md](power-budget.md)にあるが、実測値が皆無。021と022が解決するまで合成給電（段階C）へ進めず、**合成給電の実測と受け入れが始まらない**。段階Aと段階B-1は021／022に依存せず実施できる。**段階B-2は021／022には依存しないが、共通条件の025と、経路別の023（B-2a）または024（B-2b）にBlockedされている。****025(a)は`HW-TBD-004`に加えて、3.3 V rail下限の両立方式の決定にもBlockedされている**（2026-08-12に判明）。B-2の3.3 V側の実測はingress部品の選定にも要るため、023と024はその選定も止めている。**なお2026-08-12に、`PSU-PI-01`の5 VがRaspberry Pi公式の要求する5.1 Vと一致しないことが判明した。**予算の判定に使う最低電圧は`028`(a)として未確定であり、**この差の可否は評価できない**（`007`）。**034は2026-08-16に追加した。**同日に方式1（独立した外部観測）を採用済みだが、close条件が未達であり、**閾値が確定しても、測定値から受け入れの合否を導けない。**未解決なのは方式の選択ではなく、測定器の採否とclose条件である**（全数は`HW-TBD-034`の`必要な根拠`欄）**） |
| HW-TBD-010 | P1 | サーボの機械的可動域とneutral | 監視下calibration | 首振り動作の受け入れ | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18) | Human | 007–009、**026**、および首機構（[#34](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/34)）によりBlocked |
| HW-TBD-011 | P1 | サーボの速度／加速度制限 | 電流・動作試験 | Motion profile | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、[#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18) | Joint | 010によりBlocked |
| HW-TBD-013 | P1 | 軽打／持ち上げしきい値 | サーボ動作を含む取得済みraw sample | 軽打／持ち上げ動作の受け入れ | [#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15) | Joint | 004、008によりBlocked |
| HW-TBD-014 | P1 | 最終serial baudと最大line length | Pi／ESP32 transport test | Protocol v1の受け入れ | [#802](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/802) | Joint | 候補値あり |
| HW-TBD-017 | P0 | 通信断の検知方式（heartbeat source、loss timeout） | Protocol合意＋latency測定。正本: [servo-safety-limits](servo-safety-limits.md#通信断時動作)、[protocol](../protocol/esp32-pi-protocol.md#13-未決定事項) | サーボの実機動作全般 | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18) | Joint | Open（**loss timeoutの値は7000 ms**（2026-11-21、[#20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20)）。[EXP-066](experiment-log.md)の条件で、定常の到着の間隔の最大は3380 msで、その約2倍である。`ping`の間隔は[protocol §5.7](../protocol/esp32-pi-protocol.md#57-ping)が持つ。見張りは`boot`の確立から始める。`HW-TBD-020`の連続動作時間の上限との比較はしていない（servo出力の有効化の前に行う）。値を入れたbuildの実機での確認（断と判定しないこと、Piを止めたらloss timeoutで断と判定すること）は未実施である。heartbeat方式（`PROTO-TBD-010`）が案のままなので、closeしない） |
| HW-TBD-018 | P0 | 通信断時のfail-safe sequenceの選択と検証、および**recovery／reconnect動作**（断からの復帰時にサーボ出力を再有効化してよい条件と手順） | 監視下の機械試験（PWM断時の首の挙動、および復帰時の挙動）。正本: [servo-safety-limits](servo-safety-limits.md#通信断時動作) | #20、MVP受け入れ | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18) | Joint | 010、017、PROTO-TBD-013によりBlocked |
| HW-TBD-019 | P0 | 起動時とdriver故障時のサーボ出力状態（PWM driver初期化前のGPIO state、開始mode、enableまでのdelay、Pi未接続時、reset後、driver故障検知時） | 無負荷でのPWM測定＋起動時glitch確認。正本: [servo-safety-limits](servo-safety-limits.md#起動時とdriver故障時の動作) | 初回統合通電 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、[#801](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/801) | Joint | 010、**027**によりBlocked |
| HW-TBD-020 | P1 | 実行時のサーボ安全制御（採用する検知／予防手段、電流しきい値と判定時間、連続動作時間の上限、duty cycle窓と上限、検知時の物理動作、復帰条件、秒あたり受理command数、単一commandの最大変化量、command timeout、duplicate履歴の保持期間とretry window、retired sessionの保持件数と期間） | 電流測定手段の選定＋温度／電流試験。**正はfield単位**で[下表](#hw-tbd-020のfield単位の正)に定める。要件は[servo-safety-limits](servo-safety-limits.md#拘束stallと過負荷)、link側は[protocol](../protocol/esp32-pi-protocol.md#13-未決定事項) | 長時間動作とM6耐久試験 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、[#24](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/24) | Joint | 009、全fieldのresolution evidence未記録、およびPROTO-TBD-005／011／012／013／014未解決によりBlocked |
| HW-TBD-036 | P2 | **PiとESP32の間のUART（`PI-UART-TX`／`PI-UART-RX`）で、片方だけ電源が入っているときの回り込みについて、採った対策が安全だという根拠と、GNDを共通にしたときのPC–Pi間の電流経路。**検証の構成では、PiとESP32を別電源で給電する（2026-09-28決定。最終構成の給電経路は[#405](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/405)の実装設計の見直しで決める）。ESP32の入力の上限はVDD＋0.3 Vであり、ESP32の電源が切れているときにPiの送信線がHighだと上限を超える。**2026-09-28、ユーザーの決定で、対策を手持ち品で決めた。**2本の信号線にそれぞれ直列4.7 kΩ（流れ込む電流は計算で最大約0.7 mA）を入れ、UARTの信号線は両方の電源が入っている間だけつなぐ（電源を切る前に外す）。予期しない電源断（cableの抜け、PCのsleepなど）では手順が働かず、直列抵抗だけが電流を抑える。**残るのは、流れ込んでよい電流がESP32にもPiにも公開されておらず、0.7 mAが安全だという保証が無いこと**と、PC–Pi間の電流経路を確認していないことである。ユーザーはこの弱点を記録したうえで対策を採った。正本: [gpio-assignment.md](gpio-assignment.md)の`Pi–ESP32間のtransport`節 | 流れ込み電流の許容値を示す一次資料、または実測 | なし（2026-09-28以降、信号線の接続は`gpio-assignment.md`の`信号線をつないでよい条件`に従って行い、この行では止めない） | [#446](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446)、[#803](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/803) | Joint | 対策を決定済み（2026-09-28、ユーザー決定。購入なし）。保証値が無いことを弱点として記録している。接続を止めないためP2とし、最終構成の給電経路（#405）を決めるときに再評価する。**2026-11-23に、経路は案Bに決まった（ユーザーの決定。記録は[power-budget.md](power-budget.md)の`ESP32の給電経路`）。再評価（PiとESP32が同じ節点から給電されるときの、J8の線が抜けた場合の片側給電と、UARTの4.7 kΩ・X3の扱い）は[#803](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/803)が受け持つ。案Bの最終構成は`gpio-assignment.md`の条件5（別々に給電）と両立せず、条件5との関係は#405の見直しで改めて決める。それまで案Bの構成ではPiとESP32のUARTの信号線とGNDの線をつながない。**決定の記録: [#488](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/488) |
| HW-TBD-038 | P0 | **`PROTO-02`の構成で、servoの戻りがshunt（R20）を通らなくなったときに、3.3 Vの入力（GPIO32・GPIO27）へ回る電位の保護。**(a) R20またはその足のはんだ付けが開放になると、servoのGND（`ADC-SHUNT`の節点）はBへの道を失って電位が上がり、servoの戻りはESP32を通って`X5`からBへ回りうる。回り先は、W1からGPIO32の保護diode（W1をつないだとき）と、servoの信号入力から`SERVO-SIG`・R19（回路図の番号。B1の配線表のR6）を通ってGPIO27（high-Zの間は3.3 Vを超えうる）の2つである。(b) 案Bの構成で`J8`−（`X5`）だけが外れると、ESP32の戻りの電流がW1と`SERVO-SIG`の線を通りうる。どちらも5 Vになりうる節点を3.3 Vの入力へつなぐ状態である（[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の`電圧違いのpinへ挿す`）。正本: [hardware/universal-board/plan.md](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/hardware/universal-board/plan.md)の`未決の一覧`、[power-budget.md](power-budget.md)の`ESP32の給電経路`の`J8`の枝の項 | 保護の要否と、要るなら部品と構成の決定（計算または一次資料による） | (a) `PROTO-02`のW1をGPIO32（`J2-7`）へつなぐこと、`PROTO-02`の`J6`からservoへ給電すること。(b) 案Bの構成で`J8`からESP32へ給電すること（[power-budget.md](power-budget.md)の`案Bを採る条件`の(d)に入れてある）。いずれも[power-budget.md](power-budget.md)の`サーボ接続前`checklistが拾う | [#405](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/405) | Joint | Open（2026-11-24、#512で登録。保護の要否と部品は未定） |

## 現在状態と根拠の読み方

以下の14項目は項目別表で読む。004・023・024・025は現在の判定を短く記載し、以前の行全文を[判断記録](tbd-register-history.md)へ移した。007・021・022・026・027・028・030・034・035・037は、現在の判定を含む元の行全文を判断記録の該当節へ移し、書き直すまでその節を現在の判定の読み先とする（各項目の状態欄の案内に従う）。
判断記録の古いOpen／Blockedや「まだ無い」は当時の記述であり、現在状態を更新する場所ではない。ただし、上の10項目の元の行全文は、書き直すまで現在の判定を含む。
値・安全条件の正本は各行のリンク先、状態と妨げる対象はこの台帳が持つ（上の10項目の状態は、書き直すまで判断記録の該当節が持つ）。

### HW-TBD-004

| 項目 | 現在の判定 |
|---|---|
| 優先度 | P0 |
| 不足している情報／判断 | 実装済みI2C address（SDO配線で選ぶ）。header〜ICの経路は未確認 |
| 必要な根拠 | [現物確認・経緯・承認（回答の原文は省略）](tbd-register-history.md#hw-tbd-004)。moduleへ入れてよい電圧の判定は済み。正しく製造された個体を前提とし、経路確認済みとは主張しない |
| 妨げる対象 | addressは一般値扱いで着手を止めない。給電の条件は[電源文書](power-budget.md#段階b-2の測定)で別に確認する |
| 対応Issue | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#803](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/803) |
| Owner | Human |
| 状態 | Open（範囲を縮小） |

電圧の判断根拠、2026-09-21の人間承認、
旧(1)〜(6)の現物情報は[元の行全文](tbd-register-history.md#hw-tbd-004)。未確認経路を確認済みにしない。

### HW-TBD-007

| 項目 | 内容 |
|---|---|
| 優先度 | P0 |
| 妨げる対象 | 初回統合通電 |
| 対応Issue | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809) |
| Owner | Human |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-007](tbd-register-history.md#hw-tbd-007)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。この項目の現在の判定は、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809)で設計を決めたときに書き直す。|

### HW-TBD-021

| 項目 | 内容 |
|---|---|
| 優先度 | P0 |
| 妨げる対象 | **合成給電（段階C）**。保護部品の選定・実装まで実施しない |
| 対応Issue | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809) |
| Owner | Human |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-021](tbd-register-history.md#hw-tbd-021)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。この項目の現在の判定は、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809)で設計を決めたときに書き直す。|

### HW-TBD-022

| 項目 | 内容 |
|---|---|
| 優先度 | P0 |
| 妨げる対象 | ingressの上限の確定、**合成給電（段階C）** |
| 対応Issue | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809) |
| Owner | Human |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-022](tbd-register-history.md#hw-tbd-022)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。この項目の現在の判定は、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809)で設計を決めたときに書き直す。|

### HW-TBD-023

| 項目 | 現在の判定 |
|---|---|
| 優先度 | P0 |
| 不足している情報／判断 | (a) 3V3 pinの外部供給可能電流、(c) 短絡・折り返し特性、(d) decouplingの内訳・容量。(b) LDO種別は確定 |
| 必要な根拠 | [部品同定・資料・限定判断](tbd-register-history.md#hw-tbd-023)。(a)はboard levelで非公開、実測が必要。(c)は電流制限・熱shutdownのみ確定、(d)は個数まで確認 |
| 妨げる対象 | B-2a。(a)未確定では許可しない。B-2bは別条件。ACCEL-01／ENV-01の限定判断は#445、DISP-01の通常接続は[HW-TBD-024](#hw-tbd-024)を参照。**LEDの2枝（[#514](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514)。白＝GPIO2、赤＝GPIO5、各330 Ωと各10 kΩ pull-down）は、2026-10-12のユーザーの承認（[#514のコメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514#issuecomment-6124265343)）による例外である。その電流は安全側の仮定として`3V3`系に数える。書き込みと初回通電は、別の承認（版・起動の回数・停止条件）を取る。**いずれも本項の値の解決ではない |
| 対応Issue | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#804](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/804) |
| Owner | Human |
| 状態 | Open |

部品同定・(a)〜(d)の証拠と
限定判断は[元の行全文](tbd-register-history.md#hw-tbd-023)。

### HW-TBD-024

| 項目 | 現在の判定 |
|---|---|
| 優先度 | P0 |
| 不足している情報／判断 | MSP2807が耐えられる電流の上限。U1の供給能力だけでは決まらない |
| 必要な根拠 | [調査・測定・#461の承認と計算](tbd-register-history.md#hw-tbd-024)。公開資料と非通電導通測定ではbacklight経路・LED個数を確定できず、上限は未解決。非破壊の原則を維持する |
| 妨げる対象 | B-2（B-2a／B-2b）とB-2bの制限値決定。**DISP-01のESP32 3V3 pinへの通常接続（backlight点灯、ACCEL-01／ENV-01への追加を含む）は#461の承認範囲**。B-2・外部電源・段階Cへ広げない。**2026-10-27：段階Cの測定4は、MSP2807を接続せず行わない（[power-budget.md](power-budget.md)の`MSP2807の電流上限の扱いと、段階Cの測定4（2026-10-27、#556）`節）** |
| 対応Issue | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#405](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/405)、[#461](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/461)、[#556](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/556) |
| Owner | Human |
| 状態 | Open（接続判断と上限値を区別。2026-10-27：上限は公開資料からも非破壊の方法からも得られていない。`U1`の`Ilimt`はtypのみ。最終の構成は#405の見直しで決める） |

#461は残余riskの受け入れ判断であり、
上限値や下流module保護の証明ではない。通常動作と故障時を分けた計算、R5の公差不明、
未知の枝、U1の保証範囲と熱の限定は[元の行全文](tbd-register-history.md#hw-tbd-024)で読む。
接続許可とfirmware実行の承認も別である。実行には[電源文書](power-budget.md)の
`DISP-01`追加接続手順の条件(1)〜(7)・停止条件を用い、人間の当日の判断を得る。

2026-10-01の段階B-2（[experiment-log.md](experiment-log.md)の`EXP-019`）は、この行を解いていない。

2026-10-27の判断（資料のsha256、`U1`のdatasheetの値、回路図の`J1`の注記、今の構成の事実）は[power-budget.md](power-budget.md)の`MSP2807の電流上限の扱いと、段階Cの測定4（2026-10-27、#556）`節にある。

2026-11-10（[#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)）：featureなしの製品buildの通電は、`touch`の5本の挿し有無にかかわらず、`XPT2046`が`touch panel`へ流す駆動電流に上界が無く、[power-budget.md](power-budget.md#製品buildfeatureなしの通電の条件)が`TBD`として持つ。この行が問う`MSP2807`の電流の上限とは別の未確認で、この行の状態は変えない。

### HW-TBD-025

| 項目 | 現在の判定 |
|---|---|
| 優先度 | P0 |
| 不足している情報／判断 | B-2共通条件。(a) moduleとしての許容電圧範囲、(b) 全moduleの安全な電流上限／代替の現物回路確認 |
| 必要な根拠 | (a) [電源文書の許容電圧範囲](power-budget.md#段階b-2の測定)の限定付き設計判断を使う。(b) ACCEL-01／ENV-01は確認済み、DISP-01は未達。[判定の出所と訂正](tbd-register-history.md#hw-tbd-025) |
| 妨げる対象 | B-2a／B-2bとも。(b)は**電流の大きさでなく現物回路確認の手続き**を判定する。HW-TBD-024の通常接続判断・電流上界計算を代替にしない |
| 対応Issue | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#405](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/405)、[#556](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/556) |
| Owner | Human |
| 状態 | Open（(b)のDISP-01が残る。2026-10-27：段階CではDISP-01を使わず、最終の構成での確認は#405の見直しへ渡す） |

(a)のrail値は条件付きの設計判断であり、
無条件の保証値ではない。ILI9341ロジック単体負荷を前提とする下限の再導出条件は
[電源文書](power-budget.md#段階b-2の測定)、公開値不在の確認とmodule別の(b)判定は
[元の行全文](tbd-register-history.md#hw-tbd-025)に残す。

2026-10-01の段階B-2（[experiment-log.md](experiment-log.md)の`EXP-019`）は、この行を解いていない。

### HW-TBD-026

| 項目 | 内容 |
|---|---|
| 優先度 | P0 |
| 妨げる対象 | `SERVO-PWM`の駆動条件の確定、servoの実機動作全般 |
| 対応Issue | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、[#801](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/801) |
| Owner | Joint |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-026](tbd-register-history.md#hw-tbd-026)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。|

### HW-TBD-027

| 項目 | 内容 |
|---|---|
| 優先度 | P0 |
| 妨げる対象 | 初回統合通電、`HW-TBD-019`の起動時状態の確定 |
| 対応Issue | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)、[#801](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/801) |
| Owner | Human |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-027](tbd-register-history.md#hw-tbd-027)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。証拠契約が記録先とする「この台帳の行」は、この項目別表の状態欄を指す。|

### HW-TBD-028

| 項目 | 内容 |
|---|---|
| 優先度 | P1 |
| 妨げる対象 | 電源系の受け入れ承認、合成給電（段階C）の合否判定 |
| 対応Issue | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809) |
| Owner | Joint |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-028](tbd-register-history.md#hw-tbd-028)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。この項目の現在の判定は、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809)で設計を決めたときに書き直す。|

### HW-TBD-030

| 項目 | 内容 |
|---|---|
| 優先度 | P1 |
| 妨げる対象 | 合成給電（段階C）の配線承認 |
| 対応Issue | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809) |
| Owner | Human |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-030](tbd-register-history.md#hw-tbd-030)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。この項目の現在の判定は、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809)で設計を決めたときに書き直す。|

### HW-TBD-034

| 項目 | 内容 |
|---|---|
| 優先度 | P1 |
| 妨げる対象 | **[技術ガイド](../DeskCat_Microcontroller_Development_Guide.md)のゲートA**（起動時の電源過渡の実測）、および`HW-TBD-009`（電源予算とbackfeed review） |
| 対応Issue | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809) |
| Owner | Human |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-034](tbd-register-history.md#hw-tbd-034)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。この項目の現在の判定は、[#809](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/809)で設計を決めたときに書き直す。|

### HW-TBD-035

| 項目 | 内容 |
|---|---|
| 優先度 | P0 |
| 妨げる対象 | servoへの給電、`SERVO-PWM`の駆動条件の確定、servoの実機動作全般 |
| 対応Issue | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、[#801](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/801) |
| Owner | Human |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-035](tbd-register-history.md#hw-tbd-035)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。証拠契約が記録先とする「この台帳の行」は、この項目別表の状態欄を指す。|

### HW-TBD-037

| 項目 | 内容 |
|---|---|
| 優先度 | P0 |
| 妨げる対象 | LEDの配線・通電・hardware依存driver・統合表示。用途と本数はユーザーが承認済み。色の割当てはAIが選定し、人間の承認はまだ無い。抵抗の型番、初回デモ回路はAIが選定し、人間が承認した（2026-11-15、VM側のPMのセッション） |
| 対応Issue | [#514](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514)、[#538](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/538)、[#20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20)（Pi停止後の再接続の実機確認と`PROTO-TBD-010`）、[#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)（撫で操作の実機確認） |
| Owner | Joint |
| 状態 | Open。現在の判定を含む元の行全文は、[tbd-register-history.md の HW-TBD-037](tbd-register-history.md#hw-tbd-037)にある。不足している情報／判断・必要な根拠・状態の各欄は、そこで読む。|

## 対応Issue列

`対応Issue`列は、その`TBD`を**解消する作業Issue**を指す。台帳を見た人が
「この`TBD`を解くのはどのIssueか」を判断できるようにするための列である。

逆方向（Issue → TBD）は、[#65](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/65)で
[#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)と[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)の本文へ対応表を追加した。
**この2件以外のIssue本文には対応表が無い。**2026-11-23に起票した[#801](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/801)・[#802](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/802)・[#803](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/803)・[#804](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/804)の本文は、この列でそのIssueを指す行を、表でなく本文の記述で示す。それ以外のIssueでは、この列が両方向を辿れる唯一の起点である行が多い。

### 記載規則

| 場面 | 書き方 |
|---|---|
| 対応Issueが1件 | `[#N](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/N)` |
| 複数Issueに跨る | 該当Issueを`、`で区切って**すべて**列挙する。**主従を付けない。**どれか一つが解決しても行はcloseしない |
| 対応Issueが無い | `なし（未起票）`と書く。**空欄にしない。**空欄では「未記入」と「対応Issueが無い」を区別できない |

書くのは**解消する側**のIssueだけである。その`TBD`が解けるのを待っている側のIssueは書かない。
両方を書くと列が依存関係の写しになり、「どれを進めれば解けるか」が読めなくなる。
待っている側は`妨げる対象`列と各Issueの依存関係が表す。

**同じIssue番号が`妨げる対象`と`対応Issue`の両方に現れたら、どちらかが誤りである。**
待っている側が解消する側を兼ねることはない。

どのIssueが解消するかは、**その行の`必要な根拠`列が求める作業を範囲に含むか**で判定する。
Issueの題名や扱う領域の近さで決めない。たとえば`HW-TBD-019`の根拠は
「無負荷でのPWM測定＋起動時glitch確認」であり、これを範囲に持つのは
[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)である。
[#20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20)は同じ状態を**検証する**側であって、値を決める側ではない。

行を追加するときは、この列も同時に埋める。埋められないなら`なし（未起票）`とし、
下の一覧へ加える。

### closeしたIssue番号の扱い

**Issueがcloseされていても、その`TBD`が解決済みであることを意味しない。**
台帳の行のcloseは[解決手順](#解決手順)の1〜8による別の判定であり、Issueのcloseとは一致しない。
Issueは受け入れ条件を満たせばcloseされるが、`TBD`は根拠の記録、正本文書への反映、
関連Issueのblocked解除まで終えて初めてcloseする。

この列のIssueがcloseされたら次を行う。**列の記載は消さない。**

1. [解決手順](#解決手順)の1〜8を満たしたかを確認する
2. 満たしていれば[解決済み項目](#解決済み項目)へ移す
3. 満たしていなければ行を残す。残作業を担うIssueが無くなったなら、起票してこの列へ追加する

### 対応Issueが無い行

| 行 | 対応Issueが無い理由 |
|---|---|
| なし | 2026-11-23に、閉じたIssueだけを指していた行と`なし（未起票）`の行のすべてへ、残りを受け持つIssueを起票して足した |

2026-11-23まで`HW-TBD-014`がここにあった（transport testを受け入れ条件に持つIssueが無かった）。同日に[#802](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/802)を起票して列へ足した。

## Owner列

`Owner`列は、**その行に残っている行為を誰が行うかを示す。****「誰が中身を決めるか」を示す列ではない。**

| 値 | 意味 |
|---|---|
| `Human` | 人間しかできない行為だけが残っている（現物を見る、はんだ付けする、買う、監視下で通電する） |
| `Joint` | 人間しかできない行為と、机上で閉じる行為の両方が残っている |
| `AI` | 机上で閉じる行為だけが残っている（一次資料の読み取り、導出、値・方式・要否の提案） |

**この列は、提案を誰が出すかを示さない。****提案はAIが出し、承認は人間が行う。**
これはどの行でも同じであるため、列では区別しない。

`Owner: Human`は「人間が値を決める」ではなく、「**人間が手を動かさないと先へ進まない**」を意味する。
**`Owner`を、AIが提案を出さない理由として引用しない。**AIが提案を出さない理由になるのは、
一次資料が無いこと（[AGENTS.md](../../AGENTS.md) 推測禁止）と、実測が要ることだけである。

**この定義は2026-09-07に置いた。****それ以前に各行へ付けた値は、この定義に照らして付けたものではない。**
各行の値は、その行の`対応Issue`を進めるときに、残っている行為を読み直して合わせる。

## 登録範囲

`status:blocked`判定の根拠は、**この台帳と[Protocol TBD register](../protocol/esp32-pi-protocol.md#13-未決定事項)の和集合**である。
どちらか一方だけを見ると、もう一方の未解決項目がgateを素通りする。

台帳に無い未確定事項は着手可否のgateを素通りするため、次を守る。

- 正本文書の本文に`TBD`と書いた安全・電気・機械項目は、必ずこの台帳へ行を追加する。表の`TBD`セルだけで管理しない。
- 追加した行から、値を確定させる正本文書へリンクする。
- Protocol側の未決事項は`PROTO-TBD-*`で管理し、ここでは行を重複させない。
  **ただしIssueのblocked判定では、`PROTO-TBD-*`も同じ強さで扱う。**
  行を重複させないのは記述の重複を避けるためであり、gateの対象から外す意味ではない。

Issueの着手可否を判断するときは、次の両方を確認する。

1. この台帳の`HW-TBD-*`で、そのIssueを妨げる行が解決済みか
2. Protocol側の`PROTO-TBD-*`で、そのIssueを妨げる行が解決済みか

protocol実装Issue（M2系）は、hardware側TBDが未解決でも`PROTO-TBD-*`だけで
blockedになりうる。逆にhardware Issueが`PROTO-TBD-*`でblockedになることもある。

両方に関係する項目は、片方を正、もう片方を参照とする。

正・参照はIDの単位ではなくfieldの単位で決める。片方が解決しても、もう片方を自動でcloseしない。

| このIDの正 | 対応するprotocol側ID | field単位の正 |
|---|---|---|
| HW-TBD-014 | PROTO-TBD-001、PROTO-TBD-002 | baudと最大line長の値はProtocol側。実機transport testの実施責任はこの台帳 |
| HW-TBD-017 | PROTO-TBD-010 | heartbeat方式（何をheartbeatとするか）はProtocol側。loss timeoutの実測値はこの台帳 |
| HW-TBD-018 | PROTO-TBD-013 | fail-safe sequenceの選択と機械試験はこの台帳。**recovery／reconnect動作（復帰時の物理的な再有効化条件と手順）もこの台帳**。Stale commandの拒否条件はProtocol側であり、復帰時にどのcommandを受理するかは`PROTO-TBD-013`が決める |
| HW-TBD-019 | — | この台帳 |
| HW-TBD-020 | PROTO-TBD-005、PROTO-TBD-011、PROTO-TBD-012、PROTO-TBD-013、PROTO-TBD-014 | `PROTO-TBD-005`はduplicate履歴の保持期間とretry window。`PROTO-TBD-011`はretired sessionの保持件数と期間（`PROTO-TBD-005`とは別モデル。下限は独立に満たす）に加え、`sid`生成・衝突回復・`hello`の有限retry上限を持つ。`PROTO-TBD-013`はCommand timeoutのstale command拒否条件。`HW-TBD-020`をcloseするには、対応IDの一部fieldだけでなく下記の全fieldと各Protocol TBD全体の解決が必要 |

### HW-TBD-027の証拠契約

**closeに必要な記録をここで一意に定める。**「実装した」「測ってLowだった」という主張だけでは
closeしない。次の4点がすべて揃うまでOpenとする。

| # | 条件 | 記録先 | 記録する内容 |
|---|---|---|---|
| 1 | 抵抗値と本数の選定 | [gpio-assignment.md](gpio-assignment.md#信号inventory)の`SERVO-PWM`行 | 決めた抵抗値・本数と、ESP32のGPIO駆動能力およびservo側入力インピーダンスから導いた根拠 |
| 2 | 購入 | [hardware-bom.md](hardware-bom.md)の`RES-PULL-01` | 型番と入手日。`残作業`の`未購入`を消す |
| 3 | 実装 | この台帳の`HW-TBD-027`行 | 実装日と接続先（`SERVO-PWM`とGNDの間であること） |
| 4 | reset中の実測 | この台帳の`HW-TBD-027`行 | 下の測定条件と、測定した電圧値 |

測定条件は次を満たす。**満たさない測定はこの行の根拠にしない。**

- **servoを接続せず**、servo電源も投入しない状態で測る。servo側の入力が並列に入ると、
  pull-downだけの効果を分離できない
- **ESP32をresetに保持した状態**で測る。GPIO27がhigh-Zになるのはこの区間であり、
  通常起動後の測定では何も確認したことにならない
- 測定点は`SERVO-PWM`の信号線とGNDの間である

**判定閾値は`HW-TBD-026`が決まるまで確定しない。**「Lowである」と言うには、SG90が
Lowと解釈する電圧の上限が要る。それは`HW-TBD-026`の`制御logic要件`である。
**したがって`HW-TBD-027`は`HW-TBD-026`より先にcloseできない。**測定値だけを記録して
「Lowだった」と結論しない。

### HW-TBD-035の証拠契約

**closeに必要な記録をここで一意に定める。**「メーカーが4.8–6 Vと言っている」という主張だけでは
closeしない。**Q&A欄の回答は、それ単独ではこの行を閉じる根拠にならない。**
次の4点がすべて揃うまでOpenとする。

| # | 条件 | 記録先 | 記録する内容 |
|---|---|---|---|
| 1 | **現物の品の特定**、または**現物に適用できるメーカー資料** | この台帳の`HW-TBD-035`行 | `SG90 Digital`／`SG90 Analog`／`SG90 360 degree`のどれであるかと、その判定根拠。**寸法の実測は3品目を除外する手段になる**（`23×12.2x29mm` 対 `23×12.5x22mm`）。**Digitalと Analogは寸法が同一のため、寸法では分離できない。**`Operating speed`も**Analog側ページが自己矛盾しているため判別に使えない**（箇条書き`0.12`／同ページの`PRODUCT CONFIGURE TABLE`は`0.1`） |
| 2 | **servo入力電圧の実測** | この台帳の`HW-TBD-035`行 | 下の測定条件と、測定した電圧値 |
| 3 | **測定条件の記録** | 同上 | 下記の条件を満たしたことと、満たせなかった条件 |
| 4 | **合否判定の基準** | 同上 | 判定に使った上限・下限と、その出所 |

**条件ごとの追跡先。**条件1〜3は[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)（サーボ電気動作のcharacterization）の範囲である。同Issueの受け入れ条件は`正確なservo dataを記録した`・`外部電源と電流制限を記録した`・`5 Vと3.3 Vのtransientを取得した`を持つ。**ただし`servoの入力端での電圧測定`を名指ししてはいない。**#17を追跡先に置くのはservoの電気動作の検証がそこへ集約されているためであり、**条件2・3の文言そのものはこの台帳が持つ。****品の特定そのものは`HW-TBD-026`（同じく#17）が持つ。****条件4が待っていた規則は、2026-08-26に[ADR-0017](../decisions/0017-what-counts-as-a-primary-source.md)として決まった。**メーカー自身による仕様書以外の記述（乙）は3条件を満たすときだけ根拠にできる。**この行の`4.8–6 V`は、その乙の条件2（どの品に対する記述かが特定できる）を満たさない**（同名3品があり現物がどれか未確定）。**ADR-0017の乙の条件番号であり、この節の条件1〜4とは別の体系である。****したがって条件4に残るのは規則の判断ではなく、条件1が解けた後の適用であり、それは条件1〜3と同じ#17の範囲である。****`対応Issue`列を#17単独へ改めた**（2026-08-26。従来は#193を併記していた）。**条件そのものも判定基準もここで変えていない。****2026-11-23、#17のcloseの後の残りを受け持つ[#801](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/801)を起票して列へ足した。**#17の記載は消さない（`closeしたIssue番号の扱い`）。

測定条件は次を満たす。**満たさない測定はこの行の根拠にしない。**

- **servo railの、servoの入力端で測る。**電源側やbreadboard railの別の点で測ると、
  経路の電圧降下を含めた実際の入力電圧が分からない
- **servoが動作している状態と停止している状態の両方で測る。**起動時の突入電流で
  railが落ちるため、静止時の値だけでは入力電圧の範囲を示せない
- 給電経路は`PSU-SERVO-01`の実構成とする。**別の電源で測った値を代用しない**

**判定基準はこの行では確定していない。**「5 Vを入れてよい」と言うには、この品が受け入れて
よい電圧の上限が要る。**その上限の出所として何を採るかが、この行の未解決の本体である。**

- 現物が`SG90 360 degree`だと確定すれば、**仕様表の`4.8v-6V`が使える**
- `SG90 Digital`／`SG90 Analog`だと確定した場合、**仕様表は`4.8v`単独であり、上限の出所が無い。**
  Q&A欄の回答を採るか、メーカーへ問い合わせるか、4.8 V供給へ変えるかの**判断が要る**

**したがって条件1が解けるまで、条件4の判定基準は決まらない。**測定値だけを記録して
「5 Vで問題なかった」と結論しない。**`HW-TBD-035`は`HW-TBD-026`の品の特定に従属する。**

### HW-TBD-020のfield単位の正

`HW-TBD-020`は対象fieldが多いため、field単位で正を一つだけ定める。
二つの文書が同じfieldの正を名乗ると、片方だけを見た判断が起きる。

分担の原則は次のとおり。**[servo-safety-limits](servo-safety-limits.md)は安全要件と
有効化ゲートの正本**（何を満たすべきか、検知したら何をするか）であり、
**この台帳は実測値の正本**（しきい値、時間、上限の数値）である。
正でない側は evidence 及び参照として扱い、値や要件をそこで確定しない。

| Field | 正 | もう一方の役割 |
|---|---|---|
| 採用する検知／予防手段 | [servo-safety-limits](servo-safety-limits.md#拘束stallと過負荷)（安全要件としての選択） | この台帳は選定試験のevidence |
| 検知したときの物理動作 | [servo-safety-limits](servo-safety-limits.md#拘束stallと過負荷)（trajectory中止かPWM disableか） | この台帳は機械試験のevidence |
| 復帰条件 | [servo-safety-limits](servo-safety-limits.md#拘束stallと過負荷)（復帰を許す条件） | この台帳は冷却時間の実測値 |
| 電流しきい値と判定時間 | この台帳（電流測定） | 安全要件側は値を再掲しない |
| 連続動作時間の上限 | この台帳（温度試験） | 同上 |
| Duty cycle窓と上限 | この台帳（温度試験） | 同上 |
| servoの秒あたり受理command数 | この台帳（温度／電流試験） | 同上 |
| 単一commandの最大変化量 | この台帳（動作・安全試験） | 同上 |
| Command timeout | この台帳（Protocol／fail-safe試験） | 同上。Protocol側のstale command拒否条件はPROTO-TBD-013 |
| duplicate履歴の**保持期間** | PROTO-TBD-005（**現在のsession**用）。下限: 遅延messageの最大生存時間＋再送window | — |
| duplicate履歴の**retry window** | PROTO-TBD-005。制約: 保持期間以下であること。window > 保持期間だと、windowの内側でも履歴が消えている状態が生じる | — |
| duplicate履歴の**保持件数の上限とoverflow時の動作** | PROTO-TBD-005。受理budget（PROTO-TBD-012）と保持結果の最大sizeから件数上限を導出する。上限超過時は最も古いentryをevictし、evict済みentryへの再送は`duplicate_expired`で拒否する（新規commandとして実行しない） | —。件数側が無いと、保持期間内でも無制限にentryが増え、Memory予算を超える |
| retired sessionの**保持期間** | PROTO-TBD-011（**retired `sid`を`stale_session`で遮蔽する**ため）。下限: 遅延messageの最大生存時間＋再送window。確定値は時間値`T_retention`と単位を一組で記録する | —。PROTO-TBD-005とは目的が異なる別モデルであり、一方から他方を導出しない |
| retired sessionの**保持件数** | PROTO-TBD-011。`PROTO-TBD-012`の遷移上限を任意の連続windowあたり`N_transition`回、window長を`T_window`、retired保持期間を同じ時間単位の`T_retention`としたとき、下限を`N_transition × ceil(T_retention / T_window)`件とする。保持期間がwindowの端数を含む場合は切り上げ、必要なsessionを取りこぼさない | —。件数側を決めないと、保持期間内でも古い`sid`が押し出され`stale_session`で遮蔽できない |
| link全体の負荷管理parameter | PROTO-TBD-012（protocol負荷試験）。session遷移上限は任意の連続`T_window`あたり`N_transition`回として回数、window長、時間単位を一組で記録し、固定window境界で上限を迂回できない方式にしてPROTO-TBD-011の保持件数式へ渡す | — |
| fault eventの名前とpayload schema（3原因を区別） | PROTO-TBD-014 | — |

HW-TBD-017は、heartbeat方式とloss timeoutの両方が確定するまでcloseしない。
HW-TBD-018は、fail-safe sequenceとrecovery／reconnect動作の両方が確定するまでcloseしない。
片方だけでcloseすると、残る一方が持ち主のないままサーボ出力のgateを素通りする。

`HW-TBD-020`は、上表の**全field**について、正本へ確定した要件または値、根拠へのlink、
適用条件、review結果がfield単位で記録されるまでcloseしない。さらに、対応する
`PROTO-TBD-005`、`PROTO-TBD-011`、`PROTO-TBD-012`、`PROTO-TBD-013`、
`PROTO-TBD-014`がProtocolの未決定事項表からすべて削除され、Revision履歴に解決根拠が
残ることを必要とする。一つでもfieldのevidenceまたは対応Protocol TBDが欠ける場合は、
`HW-TBD-020`を未解決のまま保ち、サーボ出力の有効化gateを開かない。

**`HW-TBD-017`・`HW-TBD-018`・`HW-TBD-019`・`HW-TBD-020`の4つがすべてcloseしたら、
[ADR-0023](../decisions/0023-no-model-checking-for-now.md)を見直す。**
同ADRは「状態機械の遷移が未確定であること」を理由にモデル検査を見送っている。
**4つのcloseが意味するのは、同ADRが再検討の起点として挙げた条件が揃ったことである。
決定が自動的に反転するわけではない。**判断そのものは同ADRが持つ。

**この段落を`HW-TBD-020`の節へ置いたのは、close条件の散文が`017`・`018`・`020`とも
この節に集まっているためである。**見出しは`020`だけを名指ししているが、**見直しの対象は4件である。**
とくに`019`は、この節にclose条件の散文を持たない。**4件から外さない。**
同項目が扱うサーボ出力状態を状態機械の遷移として数えるのは**ADR-0023の起案者の読みである**
（この台帳は同項目を遷移とは書いていない）。その範囲は
[未解決項目](#未解決項目)の`HW-TBD-019`の行が持ち、**同行の`必要な根拠`欄が「正本:」として指す
[起動時とdriver故障時の動作](servo-safety-limits.md#起動時とdriver故障時の動作)が最終的な正本である**
（`reset後`の限定はそちらにある）。**ここへ写さない。**

**再検討の条件の正本はADR-0023側である。**この段落が持つのは、同ADRの決定理由の一文、
見直しの起点となるID 4件、それが何を意味しないかの注意、`019`を外さないための注意、
`019`を遷移と数えるのが同ADRの起案者の読みであることの断り、範囲の参照先、
この節へ置いた理由、および正本がどちらにあるかの明示である。
**各TBDのclose条件も、再検討の可否の判断も、ここへ複製しない。**

関連する安全要件は[Servo Safety Limits](servo-safety-limits.md)を参照する。

## 本文との照合手順

台帳に行が無い`TBD`はgateを素通りするため、正本文書の本文と台帳を全数照合する。
手順は再実行できる形で残し、**使用したpatternそのものをreviewの対象とする。**
過去に狭いpatternで「0件」と誤報告した例がある（[hardware-bom.md](hardware-bom.md)の[Revision 19](hardware-bom-history.md#revision履歴)。
3語から9語へ広げて5件の漏れが出た）。

### 対象範囲

| 区分 | 対象 |
|---|---|
| 対象 | `docs/hardware/`配下、`docs/decisions/`、`docs/toolchains/` |
| 対象外 | `docs/backlog/`、`docs/toolchains/version-records/`、Issue本文 |

version-recordは実施時点のsnapshotであり、遡って改変しない。
`docs/protocol/`の未決事項は`PROTO-TBD-*`として別に管理するため、この照合には含めない
（[登録範囲](#登録範囲)を参照）。**protocol側の全数照合はまだ実施していない。**

### 抽出command

```bash
# 1) 対象範囲の全出現
grep -rn "TBD" docs/hardware docs/decisions docs/toolchains \
  --include="*.md" --exclude-dir=version-records --exclude=tbd-register.md

# 2) 識別子参照を除いた「本文のTBD」
grep -rnE "TBD" docs/hardware docs/decisions docs/toolchains \
  --include="*.md" --exclude-dir=version-records --exclude=tbd-register.md \
  | grep -vE "(HW|PROTO)-TBD-[0-9]{3}"
```

**この台帳自身を`--exclude`で外している。**台帳の`TBD`は行の識別子か、この節のような
自己言及であり、いずれも登録対象ではない。外さないと、この節を書き足すだけで件数が動き、
前回と比較できなくなる。台帳の行そのものは、定義上すでに登録済みである。

**command 2)は行単位で除外するため、識別子と本文の`TBD`が同じ行にあると本文側も落ちる。**
1)と2)の差分（識別子を含む行）を必ず開き、**その行の識別子が、同じ行の`TBD`を
指しているか**を確認する。指していなければ登録漏れである。この確認を省くと、
表の1行に複数の`TBD`セルがある文書で漏れが出る。

差分は次の2つに分かれる。目視が要るのは後者だけである。

1. 行に現れる`TBD`が**識別子だけ**の行。本文の`TBD`を含まないため、落ちるものが無い
2. 識別子を除いても`TBD`が残る行。**この行を開いて、識別子が同じ行の`TBD`の登録先かを確かめる**

2 の絞り込みは次で行う。「TBD台帳」という台帳名も識別子と同じ扱いで外す。

```bash
grep -rnE "TBD" docs/hardware docs/decisions docs/toolchains \
  --include="*.md" --exclude-dir=version-records --exclude=tbd-register.md \
  | grep -E "(HW|PROTO)-TBD-[0-9]{3}" \
  | sed -E 's/(HW|PROTO)-TBD-[0-9]{3}//g; s/TBD ?台帳|TBD register//g' \
  | grep "TBD"
```

### 区別の基準

`TBD`という語は、識別子としても、説明文中の「未確定である」という語としても使われる。
**数える前に次の3段で区別する。**

1. **識別子**: `HW-TBD-\d{3}`／`PROTO-TBD-\d{3}`の一部。台帳行への参照であり、新規登録の対象ではない。上のcommand 2)で機械的に除く
2. **本文の`TBD`**: 値または判断が未確定であることを示す記述。表のセル、箇条書き、散文を問わない
3. 2 のうち**安全・電気・機械項目**、および**下記の識別項目**に当たるものだけが登録対象である

**識別項目とは、正本文書が確定条件として挙げている部品・chipの同定である。**安全・電気・機械の
どれにも当たらないが、**満たされないと正本文書が状態を上げられない**ため追跡する。
`HW-TBD-031`（ESP32 chipの識別。**登録時の要件は「chipの刻印」であり、2026-08-15に書き換えた**）が
これに当たり、[ESP32 Rust Toolchain](../toolchains/esp32-rust-toolchain.md)が
`検証済み`ではなく`build検証済み`にとどまっていた理由の一方であった。
**2026-08-20に満たされ、同文書は`検証済み`へ昇格した**（`esptool` 4.12.0が`ESP32-D0WD`を報告した）。
**この行は解決手順を通して2026-08-25にcloseした。**登録対象としての性質（識別項目）は変わらない。
**この段を明記するまで、031は基準3のどれにも当てはまらないまま登録されていた**
（2026-08-11の全数分類で登録し、[PR #105](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/105)のreviewで基準側の不足を指摘されて補った）。
`HW-TBD-001`〜`005`も同じ性質を持つが、いずれもGPIO割り当てや給電経路という
電気項目を直接blockするため、基準3の1つ目で拾えている。

3 に当たらないものは対象外とし、**1件ごとに理由を残す。**次の類型が繰り返し現れる。

| 類型 | 例 | 対象外とする理由 |
|---|---|---|
| **この台帳の名前** | 「TBD台帳」「TBD register」 | 識別子と同じく台帳への参照である。**command 2)の正規表現では除けないため、目視で分ける** |
| 規則文 | 「不明値は`TBD`とする」「識別情報が`TBD`の間はdriver開発を開始しない」 | 特定の未確定値を指していない |
| 凡例 | [toolchains/README.md](../toolchains/README.md)の状態ラベル表の`TBD`行 | 語の定義であり、未確定値ではない |
| Revision履歴 | 「実GPIO割り当てはすべて引き続きTBD」 | 過去時点の記録であり、現在の未確定を表さない |
| firmwareまたは文書運用の項目 | `Firmware board configuration ID`、公式文書欄の`TBD` | 安全・電気・機械項目ではない |
| 既存行が包含するtemplateの空欄 | [sensor-datasheet-notes.md](sensor-datasheet-notes.md)の未記入セル | module単位で`HW-TBD-002`〜`004`が既に追跡している。cell単位の行を追加しない |

**この基準は`TBD`という語だけを見る。**「未選定」「未決定」「未購入」のように、
`TBD`以外の語で書かれた未確定は上のcommandに掛からない。実際、`HW-TBD-027`は
`TBD`セル（公式文書欄）ではなく同じ行の「未購入・未選定」から登録した（**この記述は2026-08-15に「一部入手済み・未選定」へ訂正した。訂正後も`TBD`の語は無く、この例が成り立たなくなったわけではない**）。
**`TBD`だけを走査した照合は、この1件を取り落とす。**

### command 3) `TBD`以外の語で書かれた未確定

patternは2群からなる。**群ごとに正本を1つだけ置く。**同じ語の一覧を2箇所に書くと、
片方だけを広げた状態で「走査した」と言えてしまう。実際に過去、狭いpatternで
「0件」と誤報告している。

| 群 | 語 | 正本 |
|---|---|---|
| A: 調達状態 | `未購入`／`購入`／`発注`／`未選定`／`未確定`／`Required`／`Blocked`／`手配`／`調達` | [hardware-bom.md](hardware-bom.md#購入待ちリスト)の発注前の走査。**この台帳では再定義しない** |
| B: 検証状態 | `未実装`／`未測定`／`未確認`／`未検証` | **この節。**部品が手元にあっても、実装・測定・確認が済んでいない状態を指す |

**群Bを発注前の走査へ足さない。**あちらの目的は発注漏れの検出であり、`未測定`や`未検証`は
買う対象を示さない。混ぜると発注時に無関係なhitを読ませることになる。

```bash
# 群A（正本はhardware-bom.md）と群B（正本はこの節）を合わせて走査する。
grep -rnE "未購入|購入|発注|未選定|未確定|Required|Blocked|手配|調達|未実装|未測定|未確認|未検証" \
  docs/hardware docs/decisions docs/toolchains \
  --include="*.md" --exclude-dir=version-records --exclude=tbd-register.md
```

**このcommandはtracked Markdownしか見ない。**GitHubのIssue本文・Pull Request本文・
commentはどの引数にも入らず、台帳への登録漏れを生む経路になっていた
（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。群A（調達状態）だけを
GitHub側へ広げる報告専用の補助scriptが`scripts/scan_procurement_mentions.py`にある。
**このcommandを代替しない。**群Bを含まず、判定も止めず、tracked Markdown側は
引き続きこのcommandで走査する。

```bash
python3 scripts/scan_procurement_mentions.py --owner wachi-yoshitaka-11-dev --repo deskcat
```

**発注前の走査とこの command 3) は出力先が違う。**あちらは`購入待ちリスト`への計上漏れを探し、
こちらは台帳への**登録**漏れを探す。群Aが重なっていても判定は別であるため、
一方を実施したことをもって他方を済ませたとしない。

hitは[区別の基準](#区別の基準)の3段に掛けたうえで、次のいずれかへ分類する。**分類結果は
[実施記録](#実施記録)へ残す。**

| 分類 | 扱い |
|---|---|
| 既存の台帳行が包含する | 行番号を記録し、新規登録しない |
| 安全・電気・機械の未確定で、包含する行が無い | **新規登録する。**`HW-TBD-027`がこれに当たる |
| 発注状態のみを表す（部品は決まっており購入していないだけ） | 台帳の対象外。`購入待ちリスト`側で追う |
| 規則文・凡例・Revision履歴 | 上表の類型と同じ理由で対象外 |

### 逆方向（台帳にあるが本文から`TBD`が消えた行）

本文から`TBD`表記が消えても、台帳行を自動でcloseしない。closeは[解決手順](#解決手順)の1〜8による別の判定である。
現在該当するのは次の2種で、いずれも**行を残す**と判断した。

| 行 | 状態 | 扱い |
|---|---|---|
| `HW-TBD-008` | [gpio-assignment.md](gpio-assignment.md)は全信号へGPIOを割り当て済みで、**GPIO割り当てそのものについての`TBD`表記は残っていない**（同fileに残る`TBD`は`Firmware board configuration ID`とRevision履歴であり、いずれも別件） | 競合checkに未完了項目が残るため行を残す。この事情は同行の状態欄に記載済みである |
| `HW-TBD-013` | しきい値の正本はこの台帳であり、正本文書側に`TBD`行を持たない | 実測値の正本を台帳側に置く方針（[HW-TBD-020のfield単位の正](#hw-tbd-020のfield単位の正)と同じ構造）に従い、行を残す |

### 実施記録

件数は上のcommandをそのまま実行した値である。

| 照合日 | 1) | 2) | 差分（1−2） | 差分のうち目視した行 | 追加した行 | 実施 |
|---|---|---|---|---|---|---|
| 2026-08-10 | 219 | 136 | 83 | 30。**いずれも同じ行の`TBD`を指す識別子であり、漏れ0件**。残る53行は行中の`TBD`が識別子だけで、落ちる本文`TBD`が無い | `HW-TBD-026`〜`HW-TBD-030`（5件） | [#72](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/72) |

読み方を2点補う。`grep -n`が数えるのは**出現回数ではなく行数**であり、1行に複数の`TBD`が
あっても1と数える。また2)のhitは候補であって登録対象ではない。
[区別の基準](#区別の基準)で3段に分けたうえで判定する。

件数は照合を実施したcommitに対する値である。文書を変更すれば動くため、
**次の照合では件数を突き合わせず、commandを再実行して判定する。**

追加した5行の出所は次のとおりである。**どの記述から登録したかを残さないと、次の照合で
同じ記述を二重登録するか、逆に「既に登録済み」と誤って飛ばす。**

| 行 | 出所 |
|---|---|
| `HW-TBD-026` | [servo-safety-limits.md](servo-safety-limits.md#サーボ識別情報)の`サーボ識別情報`表。`制御logic要件`・`PWM周期／rate`・`許容最小／最大pulse`が`TBD` |
| `HW-TBD-027` | [gpio-assignment.md](gpio-assignment.md#信号inventory)の`SERVO-PWM`行。**`TBD`ではなく「未購入・未選定」**から登録した（**登録時の記述である。同記述は訂正済みで、両文書とも「未購入」とは書いていない**）。[command 3)](#command-3-tbd以外の語で書かれた未確定)を設けた契機である |
| `HW-TBD-028` | [power-budget.md](power-budget.md#受け入れ条件)の受け入れ条件。電源品質の数値制限が未定義 |
| `HW-TBD-029` | [power-budget.md](power-budget.md)の`配線・保護表`。local decouplingが`TBD`／`Blocked`（**登録時点の状態。同行は2026-08-12にcloseした**） |
| `HW-TBD-030` | [power-budget.md](power-budget.md)の`配線・保護表`。逆極性保護が`TBD`／`Blocked`（**登録時点の状態。要否と方式は2026-08-24に人間が承認したが、品番・定格が未確定で、同行は`Blocked`のまま。行もcloseしていない**） |

**2026-08-10の記録は、command 2)の136件をどう分類したかを残していない。**上表の「漏れ0件」は
差分83行についての判定であり、136件の候補そのものについての判定ではない。
**2026-08-11に、command 2)の136件とcommand 3)の156件を全件分類した。**内訳を下に残す。

### 2026-08-11の全数分類

commit `52661b9` に対して実行した。**件数は文書を変更すれば動く。**次回は件数を
突き合わせず、commandを再実行して判定する。

#### command 2)（本文の`TBD`）136件

| 分類 | 件数 | 内訳 |
|---|---|---|
| 既存行が包含する（templateの空欄） | 57 | [sensor-datasheet-notes.md](sensor-datasheet-notes.md)の4 module節。`HW-TBD-002`〜`005`がmodule単位で追跡する |
| 既存行が包含する（安全・電気・機械） | 32 | servo識別情報と駆動条件 7（`HW-TBD-010`／`011`／`026`）、機械組み立て 8（`HW-TBD-010`。`PWM停止時の重力による動作`は`HW-TBD-018`の監視下機械試験）、拘束・過負荷 6（`HW-TBD-020`）、経路部品と定格 6（`HW-TBD-021`／`022`）、負荷表 1（`HW-TBD-024`）、配線・保護表 3（`HW-TBD-009`／`029`／`030`）、受け入れ条件 1（`HW-TBD-028`） |
| 既存行が包含する（BOMの列） | 9 | `Address／mode`列 4（`HW-TBD-003`／`004`／`005`／`016`）、`Peak電流`列 4（[power-budget.md](power-budget.md#測定計画)の測定計画が包含。段階ごとに`HW-TBD-007`／`021`／`022`でgate済みであり、部品ごとの行を足すと二重管理になる）、`RES-PULL-01` 1（`HW-TBD-027`／`032`） |
| Revision履歴・履歴的記述 | 13 | 過去時点の記録。「従来は`TBD`／`Blocked`だった」を含む |
| 規則文 | 12 | 「不明値は`TBD`とする」「`TBD`が残る項目は一般値を使わない」等。特定の未確定値を指していない |
| 台帳への参照 | 10 | 「追跡は`HW-TBD-021`で行う」「field単位の正は台帳に定義」等 |
| firmware／文書運用の項目 | 1 | [gpio-assignment.md](gpio-assignment.md)の`Firmware board configuration ID` |
| 凡例 | 1 | [toolchains/README.md](../toolchains/README.md)の状態ラベル表 |
| 未分類 | 0 | — |
| **新規登録** | **1** | **`HW-TBD-031`**（ESP32 chipの刻印）。[esp32-rust-toolchain.md](../toolchains/esp32-rust-toolchain.md)が「刻印の読み取りはTBD台帳の追跡対象にはなっていない」と**自ら書いていた** |

#### command 3)（`TBD`以外の語）156件

| 分類 | 件数 | 内訳 |
|---|---|---|
| 既存行が包含する | 63 | `未購入`／`未選定`／`未確定`の大半は`HW-TBD-007`／`021`／`022`／`023`／`024`／`025`／`027`が追跡する部品と値。`未確認`は`HW-TBD-001`〜`005`の現物確認 |
| Revision履歴 | 34 | 過去時点の記録。`power-budget.md`と`hardware-bom.md`のRevision履歴が長いため件数が大きい |
| 規則文・状態ラベル・確定済み記述 | 23 | 表header、`Required`／`Blocked`の凡例、確定済みの購入履歴への言及など |
| 発注状態のみ | 21 | 部品は決まっており購入していないだけ。**調達はIssueで追う**（[購入待ちリスト](hardware-bom.md#購入待ちリスト)節が方針を持つ） |
| 文書運用・toolchain状態 | 13 | ADR-0002／0004／0007、[machine-profiles.md](../toolchains/machine-profiles.md)、toolchain索引。安全・電気・機械項目ではない |
| 未分類 | 0 | — |
| **新規登録** | **2行（1件）** | **`HW-TBD-032`**（`LCD-RST`／`LCD-CS`のpull-upと`LCD-BL`のpull-down）。[gpio-assignment.md](gpio-assignment.md#信号inventory)の受け入れchecklistが「**未実装のため要対応**」と書いていたが、`HW-TBD-027`は`SERVO-PWM`のpull-downだけを対象としており含んでいなかった |

群Aだけなら122件、群Bだけなら41件である（重なる行があるため合計は一致しない）。

### この分類で分かったこと

**2つの走査は、それぞれ相手が拾えないものを拾った。**

| 新規行 | 検出した走査 | もう一方では |
|---|---|---|
| `HW-TBD-031`（chip刻印） | command 2)（`TBD`） | 群Bの`未読`は走査語に含めていないため**検出できない** |
| `HW-TBD-032`（LCDの外部pull） | command 3) 群B（`未実装`） | 該当行に`TBD`の語が無いため**検出できない** |

**片方だけの走査では、どちらも取り落とす。**`HW-TBD-027`（登録時の`未購入・未選定`から登録。現在は`一部入手済み・未選定`）に続き、
`TBD`だけを見る照合では不十分であることが2件目・3件目の実例として確かめられた。

あわせて`HW-TBD-022`の記述へ`CABLE-PI-PWR-01`を明示した。同じ規則の適用対象でありながら
行の題名が`WIRE-PWR-01`だけを挙げており、包含関係が読み取れなかったためである。

**両commandの分類が完了したため、[Implementation Readiness Review](../runbooks/implementation-readiness-review.md)の
Hardware gateに対する本節由来の阻止条件は解消した。**ただしgate自体は
`HW-TBD-004`以下の未解決項目により`Fail／TBD`のままである（**`HW-TBD-001`は2026-08-15に、`HW-TBD-002`は2026-09-06にcloseしたため、
この代表例を差し替えた**）。**分類の完了は、
「取り落としが無いことを示した」だけであって、項目が解決したという意味ではない。**

## 解決手順

### 状態変更時の参照確認

状態を変える前に、対象IDと関連ID・部品IDで`docs/`全体を検索し、関連Issue／PRの本文・コメントも確認する。
例えば`rg -n 'HW-TBD-024|HW-TBD-025|DISP-01' docs`で候補を出す。
この検索は下の登録漏れ照合とは目的が異なり、履歴や計画文書も最初から除外しない。

| 分類 | 更新の扱い |
|---|---|
| 現在の値・条件・状態の正本 | 該当fieldと判断根拠を更新する。ID全体のcloseと部分解決を区別する |
| 現在状態を写した派生文（「Xが未解決だからY不可」等） | 適用対象を照合し、状態を重記せず正本のID・節へリンクする |
| 手順固有の開始・停止条件 | 維持する。状態が変わっても自動解除せず、その手順への適用を判断する |
| 日付付きの観測・承認・旧判断・Revision行・固定commit引用 | 当時の記録として保持する。現在状態への導線を付け、一括置換しない |

検索範囲・採否・変更前後・残件は同じIssue／PRに残す。行番号だけで引用せず、現在の参照には
節へのリンク、過去の証拠にはcommit SHAを固定したリンクを使う。


`PROTO-TBD-*`の解決判定は、[Protocol](../protocol/esp32-pi-protocol.md#13-未決定事項)の
未決定事項表に**行が残っているかどうか**で行う。解決した項目はその表から削除し、
同文書のRevision履歴へ解決日と根拠を残す。この台帳のように行を残してcloseする方式ではない。
判定方法が違うため、ゲートの確認時にどちらの規約かを取り違えない。

Protocol側の表に行が残っている限り未解決として扱い、その`TBD`をfieldに含む
`HW-TBD-*`はcloseしない。

**以下の手順1-8は`HW-TBD-*`にだけ適用する。**`PROTO-TBD-*`は行をcloseせず、
Protocolの未決定事項表から削除する。削除前に、同文書のRevision履歴へ
**そのIDそのもの**、解決日、根拠へのlink、解決したfieldを記録する。
IDを書かずに削除すると、どの`TBD`が解決したのかを後から辿れない。

1. 現物の正確な表示を記録する。
2. 公式文書を添付またはリンクする。
3. 関連する制限を正本文書へ記録する。
4. 文書だけでは不十分な場合は必要な測定を実行する。
5. 実験記録をリンクする。
6. [状態変更時の参照確認](#状態変更時の参照確認)で分類し、影響するGPIO、電源、protocol、安全文書の現在の記述を更新する。
7. 関連Issueをblockedからreadyへ変更する。
8. 解決referenceを付けてTBD行をcloseする。履歴は削除しない。

## 解決済み項目

| ID | 解決内容 | 根拠 | Close日 |
|---|---|---|---|
| HW-TBD-015 | **使用前のhealth checkを実施し、`SD-01`の健全性を確認した。**`f3`が書き込んだ全空き領域（29.80 GiB）へ位置依存dataを書いて読み戻し、`Data LOST`は0 sectorであった（**容量詐称は確認されず、検査範囲に読み書き不良も確認されなかった**）。判定は`Partial`である。**この行の`必要な根拠`は`health checkの実施`であり、それを満たした。****`妨げる対象`にあった「耐久性」は、この行が未解決だと妨げられる対象であって、測定すべき項目ではない。**2026-08-15までこの行を「耐久性を別途測定する」と読み違えてopenのまま残していたが、誤りであった（[PR #128](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/128)）。**判定基準③（速度）と`manfid`によるメーカー照合は、いずれもこの行の`必要な根拠`に含まれず、追跡を打ち切っている** | [sd-health-check.md](sd-health-check.md)、[hardware-bom.md](hardware-bom.md) SD-01、[#114](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/114) | 2026-08-15 |
| HW-TBD-006 | 正確なservo modelを`TowerPro Micro servo 9g SG90`と確定した。**`SG90 Digital`のQ&A欄でメーカーが許容と回答した電圧範囲は4.8–6 V**、動作電流は0.5–2 A（負荷依存）。**2026-08-26に、出所と欄名の記載を訂正した。****旧記載は`定格電圧4.8–6 V、stall電流はデータシート値0.5–2 A`だった。****この2値はデータシート値ではない。**公式siteの`SG90 Digital`ページのQ&A欄にある`admin`（メーカー側）の回答（2021-11-07）にのみ由来し、**仕様表の記載は`Operating voltage: 4.8v`だけである。**また`0.5–2 A`はメーカーが`operation current`として述べた値であり、**stall電流ではない**（`The operation current for our SG90 is about 0.5A - 2A.`）。**`定格`という語を外したのは、出所の格を決めたからではない**（その判断は#193に残る）。**値は1つも変えておらず、この行を再openしたのでもない**（[PR #194](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/194)が`TBD`へ戻さない判断を下している）。**同名の品が複数あり現物ラベルで判別できない点は`HW-TBD-026`が正である**（ここへ再掲しない）。**Peak／stall電流の実測は本行の範囲外**であり、`power-budget.md`の測定計画と`HW-TBD-010`／`HW-TBD-011`で追跡する | 現物ラベルの写真（`TOWER PRO Micro servo 9g SG90`）、[TowerPro公式 SG90 Digital](https://towerpro.com.tw/product/sg90-7/)（仕様表と、`admin`回答のあるQ&A欄）。**[Soldered_101246.pdf](https://www.mouser.com/catalog/specsheets/Soldered_101246.pdf)を`datasheet`として引いていた記載は、2026-08-26に外した。**TowerPro発行と確認できておらず、[hardware-bom.md](hardware-bom.md) `SERVO-01`は同じPDFを**「一次資料の扱いにしない」**としている。**2箇所が同じ資料へ別の格を与えていた。**反映先: [hardware-bom.md](hardware-bom.md) SERVO-01、[power-budget.md](power-budget.md)負荷表、[gpio-assignment.md](gpio-assignment.md) SERVO-PWM | 2026-08-05 |
| HW-TBD-029 | **各deviceのlocal decouplingについて、datasheetが指定する値と配置を確定した。**`ENV-01`（AE-BME280）は`C1`／`C2`とも0.1 µFをmodule上に実装済みで、Bosch datasheetの推奨100 nFを満たすため**外付けは不要である**。`ACCEL-01`（ADXL345 IC）は`CS` 1 µF tantalum＠`VS`／`CI/O` 0.1 µF ceramic＠`VDD I/O`をsupply pin近傍へ、追加が要れば100 Ω以下の抵抗かferrite beadを`VS`と直列に、`VS`のbypassは10 µF tantalum ∥ 0.1 µF ceramicへ増やす。**`CS`は推奨値1 µFを採ることを根拠付きで決めた**（Table 1の10 µFは測定条件であって設計要件ではない）。`DISP-01`（MSP2807）は**メーカーdatasheetに記載が無いことを確認した**。`MCU-01`は参照設計で`U2`入出力とも22 µFである。**残る「module boardに何が実装済みか、外付けが要るか」は現物確認であり、この行では追わない。**datasheetでは決まらないためであり、`HW-TBD-004`（`ACCEL-01`）、`HW-TBD-002`（`DISP-01`）、`HW-TBD-023`（`MCU-01`）の確認項目へ移した。**gateは維持している**（3行とも未解決であり、電源系の受け入れ承認を引き続き塞ぐ） | [sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`Local decoupling`節（値と出典の正）、[power-budget.md](power-budget.md)の`local decouplingの外付け要否`節（要否の判断）。一次資料は[ADXL345 Data Sheet](https://www.analog.com/media/en/technical-documentation/data-sheets/adxl345.pdf) Rev. G `POWER SUPPLY DECOUPLING` page 29、[BME280 Data Sheet](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf) Revision 1.24 Figure 17–19、[AE-BME280説明書](https://akizukidenshi.com/goodsaffix/AE-BME280_manu_v1.1.pdf) v1.1 部品表、[msp2807.pdf](https://akizukidenshi.com/goodsaffix/msp2807.pdf) Product Parameters | 2026-08-12 |
| HW-TBD-001 | **公式pin表と現物pin表記の照合が一致した。**38pinヘッダ両側のsilkがEspressif公式`J2`／`J3`のpin description表と**19pin×2列すべてで一致**した（GNDの位置を含む）。左列（`J2`）`3V3 EN VP VN 34 35 32 33 25 26 27 14 12 GND 13 D2 D3 CMD 5V`、右列（`J3`）`GND 23 22 TX RX 21 GND 19 18 5 17 16 4 0 2 15 D1 D0 CLK`。**これによりGPIO割り当ての前提が成立した。**module suffix（ESP-WROOM-32D）と「基板にrevision表示なし」も現物確認済みで、**module刻印**（`ESPRESSIF` / `ESP32-WROOM-32D`）も読了した。**旧記載の「秋月独自基板のため一致する保証がない」は根拠が無く、2026-08-10に削除済みである** | 現物写真（2026-08-13、斜光＋接写。写真はrepositoryへ置かない）と[ESP32-DevKitC V4公式回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)（`J2`／`J3`はいずれも`CON19X1_2P54`で計38pin）。[公式guide](https://docs.espressif.com/projects/esp-idf/en/v5.1/esp32/hw-reference/esp32/get-started-devkitc.html)のpin description表も同じ並びを示すが、**番号の正は回路図とする**（HTML経由の取得時に`J3`の一部で番号表示に乱れがあったため）。反映先: [hardware-bom.md](hardware-bom.md) `MCU-01`、[gpio-assignment.md](gpio-assignment.md#board識別情報)、[esp32-rust-toolchain.md](../toolchains/esp32-rust-toolchain.md)の確定条件 | 2026-08-15 |
| HW-TBD-003 | **touch controllerを`XPT2046`と確定した。**現物裏面`U2`（TSSOP-16）の刻印（`XPT`ロゴ、`XPT2046`、ロット`ABDEAB`）による。**メーカーdatasheetには型番の記載が無いため、刻印が唯一の根拠である。**「`XPT2046`系と推定、未確認」は推定でなくなった。**driverに要る動作仕様（SPI mode、最大bus速度、raw出力、calibration、IRQのedge／level）は`XPT2046`のdatasheetを当てる段階へ移った**（[sensor-datasheet-notes.md](sensor-datasheet-notes.md)のTouch controller節に`TBD`として残る） | 現物写真（2026-08-13、斜光＋接写。写真はrepositoryへ置かない）。型番がdatasheetに無いことの確認は[msp2807.pdf](https://akizukidenshi.com/goodsaffix/msp2807.pdf)（2026-08-10）。反映先: [hardware-bom.md](hardware-bom.md) `TOUCH-01`、[sensor-datasheet-notes.md](sensor-datasheet-notes.md)、[gpio-assignment.md](gpio-assignment.md)の`LCD-MISO`／`TOUCH-CS` | 2026-08-15 |
| HW-TBD-031 | **搭載moduleの識別と中核chipの識別の両方を満たした。**module刻印は2026-08-13に読了した（`ESPRESSIF` / `ESP32-WROOM-32D` / `FCC ID : 2AC7Z-ESPWROOM32D`。斜光＋接写）。**中核chipの識別は、要件を2026-08-15に書き換えた。**旧要件「chip刻印の読み取り」は**非破壊では成立しない**（chipは半田付けされた金属シールドの内側にあり、シールドの除去は分解であるため範囲外）。同日に満たし方を`esptool`が報告するchip名へ再定義し、**2026-08-20に[#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)のflash時に実施した。**`esptool` 4.12.0が`ESP32-D0WD`を報告し、期待値に一致した。**`espflash`の出力では満たせない**（family名しか返さない）。**主張するのはchip名が期待値に一致したことまでであり、周辺回路・servo・Protocol・電源品質は含まない。**満たし方の正本は[esp32-rust-toolchain.md](../toolchains/esp32-rust-toolchain.md)の`chip 識別の満たし方`であり、**ここへ再掲しない。**<br>**解決手順6（影響文書の更新）**: [esp32-rust-toolchain.md](../toolchains/esp32-rust-toolchain.md)の状態を`build検証済み`から`検証済み`へ昇格し、同文書内に残っていた「この項目は未達のままである」を訂正した。[implementation-readiness-review.md](../runbooks/implementation-readiness-review.md)の「未達なのは次の1項目である」も訂正した。本台帳の`登録範囲`節にあった「`build検証済み`にとどまる理由」も過去形へ直した。<br>**解決手順7（関連Issueのblockedからreadyへの変更）は対象が無い。**031に言及するIssueは[#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)（open）、[#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)（closed）、[#133](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/133)（closed）の3件だけであり、**openな#1は`status:blocked`を持たない**（2026-08-25にlabelを確認）。**031だけにblockされていた行も無い。**この行の`妨げる対象`はIssueではなく正本文書の状態であった | module刻印は現物の斜光＋接写（2026-08-13）。chip識別は[Version Record 2026-08-20-esp32-flash-boot-native](../toolchains/version-records/2026-08-20-esp32-flash-boot-native.md)の`chip 識別（HW-TBD-031）`節（`esptool`の未加工出力を収録）。満たし方の正本は[esp32-rust-toolchain.md](../toolchains/esp32-rust-toolchain.md)の`chip 識別の満たし方`。chipの品番の出所は[ESP-WROOM-32D datasheet v2.7](https://documentation.espressif.com/esp32-wroom-32d_esp32-wroom-32u_datasheet_en.pdf)。反映先: [hardware-bom.md](hardware-bom.md) `MCU-01` | 2026-08-25 |
| HW-TBD-002 | **残っていた3点（pin配列の現物照合、電源pinの現物照合、board上のdecoupling実装の確認）をすべて満たした。**pin配列は2026-08-13の現物確認で既に照合済みだった（裏面ラベルが左→右に`T_IRQ`〜`VCC`、datasheetの1=VCC…14=T_IRQとは逆順であることを確認済み）。**電源pinの現物照合は2026-09-05に、非通電導通測定で行った。**`VCC`ピンが`U1`（3.3V LDO、刻印`662K`／`UMW S4T`）の入力側の脚と導通し、`GND`ピンが`U1`のGND側の脚と導通することを確認し、両ピンがラベル通りの電源ネットであることを裏付けた。**decoupling実装は`C1`（積層セラミック、`U1`近傍）1個の存在を接写で確認した。容量表記は無く値は不明のままだが、これはメーカーdatasheetに記載が無いことが`HW-TBD-029`で既に確定しているため、現物確認で得られる情報の上限である。**backlight回路のLED接続経路（`VCC`側か`U1`出力側か）は非通電測定では決定できないと判明したが、これは`HW-TBD-024`（LED個数・電流上限）の範囲であり、この行（pin配列・電源pin・decoupling）には含まれない | pin配列: 現物写真（2026-08-13、斜光＋接写）と[メーカーdatasheet](https://akizukidenshi.com/goodsaffix/msp2807.pdf)＋[LCD Wiki](http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807)の14pin定義。電源pin: 2026-09-05の非通電導通測定（DT830B、`2000Ω`レンジ）。decoupling: 2026-09-05の接写。反映先: [hardware-bom.md](hardware-bom.md) `DISP-01`、[sensor-datasheet-notes.md](sensor-datasheet-notes.md)のBacklight行 | 2026-09-06 |
| HW-TBD-032 | **`LCD-RST`／`LCD-CS`／`LCD-BL`の外部pull（`RES-PULL-01`）の抵抗値・本数・極性の選定、購入、実装、起動時の状態確認3点をすべて満たした。**`LCD-CS`／`LCD-RST`は10 kΩ×1本（active-low、pull-up）、`LCD-BL`は4.7 kΩ×1本（active-high、pull-down）で確定済み。**2026-09-07に`LCD-BL`をブレッドボードへ実装し、3信号すべての実装が完了した。**同日、`EN`を押し続けてESP32をresetに保持した状態（firmware非動作）で3信号を再測定し、`LCD-CS`(GPIO22)＝3.31 V、`LCD-RST`(GPIO16)＝3.31 V、`LCD-BL`(GPIO4)＝0.00 Vを得た（いずれも期待どおりで`EN`押下の有無による変化なし）。**`LCD-CS`／`LCD-RST`は内部pullが無い（`oe=0, ie=0`）ため、この3.31 Vは外部pull-upがfirmwareのHigh駆動と切り分けて有効であることの証拠になる。**`LCD-BL`（GPIO4）は内部weak pull-down（`wpd`）が有効なため、0.00 Vだけでは外部pull-downの存在を切り分けられない（PM指摘、2026-09-07）。**ただし項目6が要求する「reset中に安全な状態（backlight Off）」自体はこの0.00 Vで満たしている** | [gpio-assignment.md](gpio-assignment.md)の`起動時状態を確定させる外部pull`節・`LCD-BLを4.7 kΩにした理由`節・受け入れchecklist、[experiment-log.md](experiment-log.md) `EXP-011`、[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2) | 2026-09-07 |
| HW-TBD-016 | **COLOR-01（Hamamatsu S11059-02DT、秋月 K-08316、I2C）を初期MVPへ含めない、と決めた。**識別情報は特定済みで、[hardware-bom.md](hardware-bom.md) COLOR-01は`Deferred`、「初期MVPでは選定しない」と記録している。残っていた採否の判断を、2026-11-10にPM（VM側）が「MVPへ入れない」と下した。役割の定義は、将来featureで再検討する際に行う。MVP後の拡張は[#686](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/686)で扱う。**`対応Issueが無い行`の本行の記載は、判断が済んだため外した** | PM（VM側）の判断（2026-11-10）。承認の有無・日時・出所の記録先はPull Requestの本文である（ユーザーの発言は書かない）。根拠: [hardware-bom.md](hardware-bom.md) COLOR-01、[#686](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/686) | 2026-11-10 |
| HW-TBD-012 | **MVPの撫でのしきい値として、firmwareが今持つ値を採ると決めた。**値の正は[touch.rs](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/firmware/esp32/src/touch.rs#L117-L125)の`STROKE_*`定数であり、**ここへ写さない。**[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の対応表`タッチ、加速度のしきい値`を参照。**観測（事実だけを書く。受け入れ条件の判定は書かない）**: `EXP-041`の`touch-sample-14`（このしきい値を使わないraw sampling）は、操作40件のうち見逃し6件（すべて「軽く触れる」）、窓の外の`level=low`は0件。`EXP-044`（製品build、develop `f3a3a97`）は、撫でる15合図に`stroke`22件（15合図すべてで1回以上）、長押し10合図に9件（7合図）、軽く触れる10合図に`pressed`0件、窓の外の`stroke`は0件。**採った値を後で変える場合は、製品buildを書き込む実機の窓で判定の回数を数える**（`EXP-044`と同じ形） | [experiment-log.md](experiment-log.md)の`EXP-041`・`EXP-044`、[#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)。PM（VM側）の判断（2026-11-10）。承認の有無・日時・出所の記録先はPull Requestの本文である（ユーザーの発言は書かない）。反映先: [touch.rs](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/firmware/esp32/src/touch.rs)のdoc comment（「暫定」と「数えてから決める」の文を外した。コードの値は変えていない） | 2026-11-10 |
| HW-TBD-005 | **`ENV-01`（BME280）の`J1`／`J2`は付けない、と決めた。bus の pull-up は今の構成のままである。**`J3`は付けた。はんだ付けは2026-09-07に行われた（ユーザーの申告であり、実測ではない。状態と根拠の水準の正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`jumper（AE-BME280）`節で、**ここへ再掲しない**）。`SDO`は`GND`へ配線し、Chip IDを読み出した（生byte。`EXP-015`自身は一致の判定をしていない。**値と条件は[EXP-015](experiment-log.md)が持ち、ここへ写さない**）。I2Cのpull-up値とaddressの扱いは[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の対応表`I2Cアドレス`と`pull-upとdecouplingの値`を参照。**`Cb`は測っていない。**この判断は`Cb`の測定を待たない。`J1`／`J2`を足さないので、BME280側の4.7 kΩは bus に入らず、実効pull-upの計算に数えない（正は[gpio-assignment.md](gpio-assignment.md)の`I2C busの実効pull-up`節）。**`Cb`の扱いは[gpio-assignment.md](gpio-assignment.md)の受け入れchecklistが持つ。** | PM（VM側）の判断（2026-11-11）。承認の有無・日時・出所の記録先はPull Requestの本文である（ユーザーの発言は書かない）。根拠: [experiment-log.md](experiment-log.md)の`EXP-014`（`J3`）・`EXP-015`（`SDO`→`GND`、Chip ID）、[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`closeまでに要ること（実装と確認を含む）`表。反映先: [sensor-datasheet-notes.md](sensor-datasheet-notes.md)（`推奨測定rate`行は、レジスタの値を[env.rs](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/firmware/esp32/src/env.rs)、周期を[env_report.rs](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/firmware/esp32/src/env_report.rs)が持つ）、[gpio-assignment.md](gpio-assignment.md)（`J1`／`J2`の判断の節を外した）、[hardware-bom.md](hardware-bom.md) `ENV-01` | 2026-11-11 |
