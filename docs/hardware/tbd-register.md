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

| ID | 優先度 | 不足している情報／判断 | 必要な根拠 | 妨げる対象 | Owner | 状態 |
|---|---|---|---|---|---|---|
| HW-TBD-001 | P0 | **残: board回路図と現物pin表記の照合。**module suffix（ESP-WROOM-32D）と「基板にrevision表示なし」は現物確認済み。秋月独自基板のためpin配列がEspressif ESP32-DevKitC V4と完全一致する保証がなく、GPIO割り当ての前提が未検証 | 秋月商品ページ添付データシートと現物pin表記の1対1照合。確定済み部分の根拠: [hardware-bom.md](hardware-bom.md) MCU-01、[gpio-assignment.md](gpio-assignment.md#board識別情報) | 最終GPIO割り当て | Human | Open（範囲を縮小） |
| HW-TBD-002 | P0 | **残: 現物確認。**MSP2807（ILI9341）は**2026-08-08に着荷済み**だが、pin配列・電源pinをまだ現物で確認していない | 現物確認＋[LCD Wiki](http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807)。選定の根拠: [hardware-bom.md](hardware-bom.md) DISP-01 | LCD driver、SPI pin、電源 | Human | Open |
| HW-TBD-003 | P0 | **残: touch controllerの型番特定。**MSP2807一体のtouch panelを使うと確定したが、controller型番はメーカー未公開（`XPT2046`系と推定、未確認）。**現物は2026-08-08に着荷済み** | 現物chip刻印の確認。選定の根拠: [hardware-bom.md](hardware-bom.md) TOUCH-01 | Touch driver、pin、gestureしきい値 | Human | Open |
| HW-TBD-004 | P0 | **残: interface選択jumperと実装済みI2C addressの現物確認。**module／ICはADXL345（秋月 M-06724）と特定済み | 現物のjumper／address pin設定の確認。特定の根拠: [hardware-bom.md](hardware-bom.md) ACCEL-01 | Accelerometer driverとしきい値 | Human | Open（範囲を縮小） |
| HW-TBD-005 | P0 | **残: I2C／SPI選択jumperの実装状態と実装済みI2C addressの現物確認。**module／ICはBosch BME280（秋月 K-09421）と特定済み | 現物のjumper設定の確認。特定の根拠: [hardware-bom.md](hardware-bom.md) ENV-01 | Environment driverとbus計画 | Human | Open（範囲を縮小） |
| HW-TBD-007 | P0 | **残: 5 V ingressの変換部品の購入と、connector定格の検証。**電源modelはM-12001（5 V／3 A）と確定し、現物も2026-08-08に着荷済み。ただしMicro-Bオスplugをbreadboardへ引き込む変換部品が未購入である（候補: 秋月 g110972、定格1ピン1.5 A）。上限は経路上の全部品の定格の最小値の80%（候補構成の最弱部品が1.5 Aなら1.2 A以下。未確定の経路要素が残るため確定値ではない。HW-TBD-021／022）であり、**判定は定常電流で行う**（connector定格は熱の制限のため）。**段階A（Pi単体起動）、段階B-1（ESP32をPCのUSBから給電）、段階B-2（周辺module3点を`3V3` pinから給電して3.3 V側の定常電流を測る）はこの行に依存せず進められる**が、**合成給電（段階C）はこの部品が要る** | 変換部品の選定＋実測。確定済み部分の根拠: [hardware-bom.md](hardware-bom.md) PSU-PI-01／PSU-SERVO-01、[power-budget.md](power-budget.md#5-v-ingress物理的な引き込み経路) | 初回統合通電 | Human | Open（範囲を縮小） |
| HW-TBD-008 | P0 | GPIO割り当て | Board／module回路図＋競合review | すべてのhardware driver | Joint | 001–005によりBlocked（006は解決済み。下書きは[gpio-assignment.md](gpio-assignment.md)にあるが、競合checkに未完了項目が残る） |
| HW-TBD-009 | P0 | 電源予算とbackfeed review | 部品電流＋回路図＋測定計画 | Servoと全体統合 | Joint | 001–005、007、**021、022**によりBlocked（006は解決済み。下書きは[power-budget.md](power-budget.md)にあるが、実測値が皆無。021と022が解決するまで合成給電（段階C）へ進めず、実測が始まらない） |
| HW-TBD-010 | P1 | サーボの機械的可動域とneutral | 監視下calibration | 首振り動作の受け入れ | Human | 007–009、および首機構（[#34](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/34)）によりBlocked |
| HW-TBD-011 | P1 | サーボの速度／加速度制限 | 電流・動作試験 | Motion profile | Joint | 010によりBlocked |
| HW-TBD-012 | P1 | Touch gestureのしきい値 | 取得したraw sample | 撫で動作の受け入れ | Joint | 003、008によりBlocked |
| HW-TBD-013 | P1 | 軽打／持ち上げしきい値 | サーボ動作を含む取得済みraw sample | 軽打／持ち上げ動作の受け入れ | Joint | 004、008によりBlocked |
| HW-TBD-014 | P1 | 最終serial baudと最大line length | Pi／ESP32 transport test | Protocol v1の受け入れ | Joint | 候補値あり |
| HW-TBD-015 | P1 | **残: health checkと耐久性の確認。**識別情報はSamsung EVO Plus microSDHC 32GB（UHS Speed Class 1）と現物print確認済み。使用前の健全性checkは未実施 | health checkの実施。識別の根拠: [hardware-bom.md](hardware-bom.md) SD-01 | Deployと耐久性 | Human | Open（範囲を縮小） |
| HW-TBD-016 | P2 | **残: MVPへ含めるかの判断と役割定義。**識別情報はHamamatsu S11059-02DT（秋月 K-08316、I2C）と特定済み | MVP review。特定の根拠: [hardware-bom.md](hardware-bom.md) COLOR-01 | 将来の環境色feature | Human | Deferred（識別は完了、採否判断が残る） |
| HW-TBD-017 | P0 | 通信断の検知方式（heartbeat source、loss timeout） | Protocol合意＋latency測定。正本: [servo-safety-limits](servo-safety-limits.md#通信断時動作)、[protocol](../protocol/esp32-pi-protocol.md#13-未決定事項) | サーボの実機動作全般 | Joint | Open |
| HW-TBD-018 | P0 | 通信断時のfail-safe sequenceの選択と検証、および**recovery／reconnect動作**（断からの復帰時にサーボ出力を再有効化してよい条件と手順） | 監視下の機械試験（PWM断時の首の挙動、および復帰時の挙動）。正本: [servo-safety-limits](servo-safety-limits.md#通信断時動作) | #20、MVP受け入れ | Joint | 010、017、PROTO-TBD-013によりBlocked |
| HW-TBD-019 | P0 | 起動時とdriver故障時のサーボ出力状態（PWM driver初期化前のGPIO state、開始mode、enableまでのdelay、Pi未接続時、reset後、driver故障検知時） | 無負荷でのPWM測定＋起動時glitch確認。正本: [servo-safety-limits](servo-safety-limits.md#起動時とdriver故障時の動作) | 初回統合通電 | Joint | 010によりBlocked |
| HW-TBD-020 | P1 | 実行時のサーボ安全制御（採用する検知／予防手段、電流しきい値と判定時間、連続動作時間の上限、duty cycle窓と上限、検知時の物理動作、復帰条件、秒あたり受理command数、単一commandの最大変化量、command timeout、duplicate履歴の保持期間とretry window、retired sessionの保持件数と期間） | 電流測定手段の選定＋温度／電流試験。**正はfield単位**で[下表](#hw-tbd-020のfield単位の正)に定める。要件は[servo-safety-limits](servo-safety-limits.md#拘束stallと過負荷)、link側は[protocol](../protocol/esp32-pi-protocol.md#13-未決定事項) | 長時間動作とM6耐久試験 | Joint | 009、全fieldのresolution evidence未記録、およびPROTO-TBD-005／011／012／013／014未解決によりBlocked |
| HW-TBD-021 | P0 | **5 V ingressの過電流保護部品（`PROT-OC-01`）の選定とtrip値。**M-12001は3 Aを供給でき、候補構成の最弱部品は1.5 Aである。現状は「上限を超えたらテスターの読みで人が電源を落とす」だけであり、connectorと線材が発熱する前に電流を止める手段が無い | メーカーの時間-電流特性（一次資料）に基づく選定。選定基準・挿入位置・上限との関係は[power-budget.md](power-budget.md#過電流保護段階cのgate) | **合成給電（段階C）**。保護部品の選定・実装まで実施しない | Human | Open |
| HW-TBD-022 | P0 | **大電流経路の線材・接続部材（`WIRE-PWR-01`）の選定と許容電流。**手持ちのbreadboardとジャンパー線は個別の許容電流がメーカー資料で確認できず、経路の最小定格を出せない。最小定格が出ないとingressの上限（経路部品の定格の最小値の80%）が確定しない | 公称許容電流が公開されている線材・接続部材の選定。決定の根拠は[power-budget.md](power-budget.md#経路部品と定格) | ingressの上限の確定、**合成給電（段階C）** | Human | Open |

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

関連する安全要件は[Servo Safety Limits](servo-safety-limits.md)を参照する。

## 解決手順

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
6. 影響するGPIO、電源、protocol、安全文書を更新する。
7. 関連Issueをblockedからreadyへ変更する。
8. 解決referenceを付けてTBD行をcloseする。履歴は削除しない。

## 解決済み項目

| ID | 解決内容 | 根拠 | Close日 |
|---|---|---|---|
| HW-TBD-006 | 正確なservo modelを`TowerPro Micro servo 9g SG90`と確定した。定格電圧4.8–6 V、stall電流はデータシート値0.5–2 A（負荷依存）。**Peak／stall電流の実測は本行の範囲外**であり、`power-budget.md`の測定計画と`HW-TBD-010`／`HW-TBD-011`で追跡する | 現物ラベルの写真（`TOWER PRO Micro servo 9g SG90`）、[TowerPro公式](https://towerpro.com.tw/product/sg90-7/)、[datasheet](https://www.mouser.com/catalog/specsheets/Soldered_101246.pdf)。反映先: [hardware-bom.md](hardware-bom.md) SERVO-01、[power-budget.md](power-budget.md)負荷表、[gpio-assignment.md](gpio-assignment.md) SERVO-PWM | 2026-08-05 |
