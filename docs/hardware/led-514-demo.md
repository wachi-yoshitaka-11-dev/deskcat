# #514 発光ダイオードのデモ回路案

> 状態: 初回デモの配線案と、点灯試験用のfirmware（`bringup-led-514`。build確認まで）。実機での配線・通電・明るさ確認は未実施。
> **LEDの2枝を足して通電することは、まだ人間の承認を得ていない**（その電流は安全側の仮定として`3V3`系に数える。下の`3V3 pinへ負荷を足す条件`）。
> 対象: 筐体から見える白1本（通信）と赤1本（猫の反応）。フォトICダイオードは使わない。

## 判断の出所

| 判断 | 誰が | 記録 |
|---|---|---|
| 用途（通信状態1本、猫の演出1本）と本数（計2本） | ユーザーが承認（2026-10-01） | [#514](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514)本文 |
| 色の割当て（白＝通信、赤＝猫）と表示の意味 | ユーザーの「早く決めよう。どんどん進めよう」を受けてAIが選定（2026-10-01） | 同上 |
| GPIO2／5、10 kΩ、配線、点滅周期・点灯時間 | **AI（作業セッション）が選定した。人間の承認は得ていない** | この文書 |
| 電流制限抵抗は330 Ω（FAITHFUL LINK `CF25J330RB`）にする。W数はメーカーのdatasheetで確かめた値を使う。W数を確かめられない手持ちの270 Ωは使わない | 抵抗値（330 Ω）と品（`CF25J330RB`）はAIが選定した。人間の承認は得ていない。W数をメーカーのdatasheetで確かめた値にすることと、W数を確かめられない手持ちの270 Ωを使わないことは、PMの判断（W数を残余riskとして受け入れるとした前の判断を訂正した） | [#514のコメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514#issuecomment-5964034773) |

## 役割と表示

ESP32が両LEDを駆動する。通信LEDはESP32が自分のprotocol受信状態から判定し、Piが停止しても通信未成立を示せるようにする。Piは撫でられたときの猫の反応を決め、ESP32は受け取った反応を赤LEDへ反映する。これは[プロジェクトの役割分担](../DeskCat_Microcontroller_Development_Guide.md)に沿う。

| LED | デモで表示する状態 | 実装上の前提 |
|---|---|---|
| 白 `LED-COMM-01` | ESP32が起動中でPiとの通信が未成立・喪失なら1秒周期で点滅（0.5秒ずつ）。通信が継続して成立すれば点灯 | 成立・喪失を決めるheartbeat／timeoutは[PROTO-TBD-010](../protocol/esp32-pi-protocol.md)と[HW-TBD-017](tbd-register.md)に依存する。現行firmwareは`hello`／`ping`／`get_status`を受信する（`firmware/esp32/src/pi_link.rs`）が、継続通信の判定（heartbeatとtimeout）はまだ無い。受信しただけでは、その後も通信が続いていることを示せないため、点灯へ切り替える判定はまだ実装できない。`bringup-led-514`は、点灯試験（赤→白→両方）の後は点滅だけを出す（下の`点灯試験用のfirmware`） |
| 赤 `LED-REACT-01` | Piが撫でられた反応として`happy`を指示したら1秒点灯し、通常時は消灯 | 既存protocolの`set_expression`を候補に使う。protocol crateには`set_expression`の型とdecodeがある（#534）。`face-21`付きbuildは受け取った`set_expression`をLCDへ描く（#21）が、赤LEDを点ける経路はまだ無い。`bringup-led-514`は点灯試験の区間だけ赤を点ける |

ESP32自体の電源断はLEDの消灯にしか見えない。白の消灯を「通信正常」と解釈しない。

## 初回デモの配線案

ESP32 boardのGPIOから電流を出し、GPIOとLEDの間にそれぞれ独立した電流制限抵抗`R`を入れる。LEDの向きは現物の極性を確認する。

```text
白: ESP32 GPIO2 (J3-15) ─ R ─ 白LEDのアノード → カソード ─ GND
    GPIO2 ─ 10 kΩ ─ GND                     (起動時のLowを維持)

赤: ESP32 GPIO5 (J3-10) ─ R ─ 赤LEDのアノード → カソード ─ GND
    GPIO5 ─ 10 kΩ ─ GND                     (起動時のLowを定める)
```

白はGPIO2を、赤はGPIO5をHighにすると点灯する。どちらもGPIOから電流を出す向き（source）で駆動する。GPIOが電流を引き込む向き（sink）には、絶対最大や保証値（max）が公開されていない。公開されているのは`IOL`のtyp 28 mAだけである（[GPIO Assignment](gpio-assignment.md)）。そのためsinkでは使わない（下の`電流の見積もりと残る観察`）。

**起動時の電位。**[ESP32-WROOM-32D／32U datasheet v2.8](https://documentation.espressif.com/esp32-wroom-32d_esp32-wroom-32u_datasheet_en.pdf)の§3 Table 4は、strapping pinの既定をGPIO2がpull-down、GPIO5がpull-upとする。外部10 kΩはどちらもpull-downとして置く。同§3.1 Table 6はdownload bootを「GPIO0＝0かつGPIO2＝0」とし、GPIO2のLowはこれと整合する。同§3.4 Table 8によれば、GPIO5が効くのはMTDOとの組でのSDIO slaveのtimingだけであり、外部pull-downでGPIO5をLowに定めると、その組は（MTDO＝1、GPIO5＝0）になる。DeskCatはSDIO slaveを使わない。GPIO0／12／15には触れないので、MTDO（GPIO15）は既定のpull-up（boot logを出す側。同§3.3 Table 7）のままである。**reset中とreset直後の内部pullは[ESP32 Series Datasheet](https://documentation.espressif.com/esp32_datasheet_en.pdf) v5.3のA.4 `IO_MUX`による（[GPIO Assignment](gpio-assignment.md)の`reset時のpin状態`）。その後、bootloaderが動く区間の実際の電位は確かめていない。**

**board上の接続。**[ESP32-DevKitC V4回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)（`ESP32_DevKitc_V4`、2017-12-06。この基板に当てる理由は[GPIO Assignment](gpio-assignment.md#board識別情報)）では、IO2とIO5がつながる先はmodule、未実装の`ESP32_WROVER(NC)` footprint、header `J3`だけである。WROOM-32D datasheet v2.8のFigure 5（module回路図）では、内蔵flashの`/CS`は`SCS/CMD`であり、GPIO5はmoduleのpin 29（IO5）へ出ているだけである。**回路図の上では、board上にLED、pull抵抗、コンデンサは無い。**現物の導通は測っていない。header位置は`J3`の15番がIO2、10番がIO5である（[GPIO Assignment](gpio-assignment.md#board識別情報)の`J3`の並び）。

**抵抗`R`。**330 Ωの`CF25J330RB`（FAITHFUL LINK INDUSTRIAL CORP.）を使う。メーカーのdatasheet『CF series / Carbon Film Fixed Resistors』（各頁にFAITHFUL LINKの社名がある。取得したのは[秋月のserverにある写し](https://akizukidenshi.com/goodsaffix/cf.pdf)で、sha256 `0035f42051d04cd054a880826d6d53d4e6c606c44309c9b925262c0b61258f55`、2026-10-03取得。版と日付の記載は無い。メーカーのsiteは403で取得できなかった）の`How To Order`は、`CF`が型、`1/4W = 25`、`J = ± 5%`、`B = Bulk`とする。330 Ωを`330R`と書く例は表に無く、`2.3K = 2K3`と同じ書き方からの読みである。抵抗値そのものは、袋の表示と実測で確かめる。`Electrical Characteristics`は`Power rating at 70℃`を`1/4W`、`Operating Temp. Range`を`- 55℃ ~ +155℃`、`Max. Working Voltage`を`250V`（最大使用電圧の上限。この抵抗値の定格連続使用電圧は、CF seriesのdatasheetの式 RCWV＝√(定格電力×抵抗値) で、330 Ωの1/4 Wは約9.1 V（計算）で、想定の最大3.6 Vはその内側）とし、`POWER GRAPH`は定格電力が70 ℃まで100%で、155 ℃で0%まで直線で下がる形を示す。周囲温度が70 ℃以下なら、定格は1/4 Wのままである。調達は発注Issue [#538](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/538)が持つ（[hardware-bom.md](hardware-bom.md)の`RES-LED-01`）。`CF25J330RB`以外の抵抗は使わない。**使う2本は接続前に必ず実測し、取り違え（別の値の抵抗）が無いことを確かめる**（`段階的な確認`の1）。10 kΩは手持ちの`RES-PULL-01`から2本を使う。電流制限抵抗を省略しない。

### 電流の見積もりと残る観察

抵抗はdatasheetの許容差（`J = ± 5%`）の下限、公称の−5%の313.5 Ωとして、LEDを短絡した最悪側を、ESP32 moduleの推奨給電上限3.6 V（ESP-WROOM-32D datasheet v2.8 §5.2 Table 14）で見積もる。

```text
1枝の電流の見積もり    I_max = 3.6 V / 313.5 Ω ≈ 11.5 mA
抵抗の損失の見積もり    P_max = (3.6 V)² / 313.5 Ω ≈ 41.3 mW
pull 1本の電流          3.6 V / 5 kΩ ≈ 0.72 mA
3V3系に数える増分の見積もり 2 × I_max + 2 × 0.72 mA ≈ 24.4 mA
```

pullの10 kΩ（`RES-PULL-01`）は許容差の記録が無いため、[Power Budget](power-budget.md)と同じく抵抗の半分（5 kΩ）として見積もる。

`I_max`は、白LEDのDC絶対最大30 mA、赤LEDの50 mAより低い。LEDが短絡すると、GPIO2とGPIO5はそれぞれ`I_max`を出す（source）。ESP-WROOM-32D datasheet v2.8 §5.1 Table 13（Absolute Maximum Ratings）は`Cumulative IO output current`をmax 1,100 mAとし、その表のNote 1は、3つのpower domainのIOをHighで出力してGNDへつないだ24時間の試験の後も正常に動いたとする（GPIO2は`VDD3P3_RTC`、GPIO5は`VDD3P3_CPU`）。GPIOが電流を引き込む向き（sink）は、絶対最大や保証値（max）が公開されておらず、公開されているのは`IOL`のtyp 28 mAだけであるため、sinkで駆動する回路は使わない。firmwareはdrive strengthをESP-IDFの既定のままにする。電流を決めるのは`R`である。`P_max`（41.3 mW）は、datasheetの70 ℃での定格1/4 W（250 mW）の約17%である（計算）。**これは回路計算であり、実機測定ではない。**

白LED [`OSPW5111B-QR`](https://akizukidenshi.com/goodsaffix/OSPW5111B-QR.pdf)のVfは20 mA・25 ℃で2.8–4.0 Vなので、3.3 Vで期待する明るさは保証できない。同資料にVf–If曲線は無い。初回デモで視認できるか観察し、不足する場合は5 V系とスイッチ素子を使う回路を別途検討する。赤 [`OSHR5161A-QR`](https://akizukidenshi.com/goodsaffix/OSHR5161A-QR.pdf)も明るさを観察する。白の型番は、包装の型番とロットの表示で識別した。赤の型番は、セット写真の袋の印字とユーザーの申告で識別した。赤のロットは照合していない（2026-10-01のユーザー判断。[hardware-bom.md](hardware-bom.md)の`LED-COMM-01`／`LED-REACT-01`）。筐体の取付位置は[#34](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/34)と合わせて決める。

## `3V3` pinへ負荷を足す条件

**LEDは`3V3` pinにつながない（GPIO → `R` → LED → GND）。LEDの電流はGPIOから出るが、その電流が`3V3` pinを経由するかは記録が無く、安全側の仮定として`3V3`系に数える。この仮定のもとで、LEDをこの条件の対象に入れる。**

**この条件はまだ満たしていない。満たすまでLEDをつないで通電しない。**

ESP32 boardの`3V3` pinから外部負荷を取ること（段階B-2a）は、[HW-TBD-023](tbd-register.md#hw-tbd-023)の(a)が未確定の間は許可されていない。例外は`ACCEL-01`／`ENV-01`（[#445](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/445)）と`DISP-01`（[#461](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/461)）だけであり、**どちらもLEDを含まない。**[Power Budget](power-budget.md)の`DISP-01`追加接続のbring-upの手順の条件(3)(a)も、`3V3` railに載る負荷をESP32本体・`ACCEL-01`・`ENV-01`・`DISP-01`だけに限っている。LEDの枝と10 kΩ pullを`3V3`系に数えると、この条件から外れる。白の枝はGPIO2（`VDD3P3_RTC` domain）から流れる。その電流が`3V3` pinを経由するかは正本のどこにも記録が無い（[Power Budget](power-budget.md)のGPIO4についても、経由するかを断定していない）。**ここでは安全側の仮定として`3V3`系に数える。**

**必要なのは、人間が「LEDの2枝を足して通電してよい（その電流は安全側の仮定として`3V3`系に数える）」と明示し、その記録が通電前に#514のコメント等で確認できることである。**AIの判断として、次の理由で承認を求める。

- **増える電流の見積もり。**LEDが短絡した場合の計算値は、330 Ωの許容差の下限で各枝約11.5 mA、pullを含めた増分は約24.4 mAである（上の`電流の見積もりと残る観察`）。
- **#461と同じ枠組みで余裕を見積もれる。**#461の判断は保守側の場合(a)を使い、`3V3`系の通常動作の合計を約245.4 mA、`U2`（UMW `LD1117-3.3`）の連続定格1 Aに対して約4.1倍と見積もった（[tbd-register-history.md](tbd-register-history.md)の`HW-TBD-024`、2026-09-23追記）。増分の約24.4 mAを足すと**約270 mAで、約3.7倍**になる（これは#461の比で、Power Budgetの`marginの定義`が5 V経路に当てる「定格の最小値の80%」の規則とは別の見積もりである。`U2`の1 Aの80%は800 mAで、270 mAはその内側にあるが、typ値を含む見積もりのため合否には使わない）。**#461と同じくtyp値を含む見積もりであり、worst-caseは未確定のままである。**`U2`の損失の増分は、USB給電（`Vin`＝5 V）で `0.0244 A × (5.0 − 3.3) V ≈ 41 mW`、同じ記録の熱計算（`Rθja` 88 ℃/W）で温度上昇は約3.7 ℃増える（計算）。
- **故障時の熱の余裕が小さくなる。**同じ記録は、`DISP-01`の`R5`の先が短絡した場合の電流を約646.5 mAとし、そのとき働きうる保護は`U2`の熱shutdown（`TSD` typ 150 ℃）だけで、`TJ`は約141 ℃、`TSD`まで約9 ℃としている。LEDの増分を足すと約671 mA、`TJ`は約144.8 ℃で、**`TSD`までの余裕は約5.2 ℃に縮む**（計算）。
- **#461の記録は、余裕が足りなかったときの帰結を受け入れたとは書いていない。**#461が受け入れた残余riskは、未知の経路の可能性と熱の余裕の薄さである（[tbd-register-history.md](tbd-register-history.md)の`HW-TBD-024`、2026-09-23追記）。同じ記録が「機能の失敗」と書くのは`U1`の出力不足（backlightが暗くなる・LCDが正常に動かない）の帰結であり、`U2`の電流制限（`Ilimit` min 1.25 A）か熱shutdownによるrail低下・reset・表示不良を受け入れたとは書いていない。`U2`の`TJ(max)`は同じ記録も`power-budget.md`も記録を持たず、約141 ℃が`U2`にとって許容される温度かどうかは、正本のどこにも書かれていない。

- **残余risk（承認の文に入れる）：**
  - `U2`の余裕はtyp値を含む見積もりで、worst-caseは未確定である。`DISP-01`の`R5`の先が短絡する故障では、`TSD`までの余裕が約9 ℃から約5.2 ℃に縮む（計算）。
  - firmwareの初期化で、両pinとも、向きの設定と消灯levelの設定の間に出力registerの値が一瞬出る。その値が点灯側か消灯側かも、長さも確かめていない。Hardware Safety Policy §4（output modeへ切り替える前に安全な初期出力を定義する）から外れる（下の`点灯試験用のfirmware`の1）。

**承認されるまで、この節は判断の材料であって許可ではない。**

## 点灯試験用のfirmware

`firmware/esp32`の`bringup-led-514` featureを付けたbuildだけがGPIO2／GPIO5を駆動する。既定buildは両pinに触れない。`bringup-display-13`とも組み合わせられる（`bench-servo-test-17`は#474で`compile_error!`のため対象外）。build commandの正本は[検証済みコマンド](../toolchains/verified-commands.md)である。

1. `Peripherals::take()`の直後に両pinを出力にし、消灯側（Low）へ設定する。**向きの設定と消灯levelの設定の間に、出力registerの値が一瞬出る。その値が点灯側（High）か消灯側かも、長さも確かめていない。**[Hardware Safety Policy](../governance/hardware-safety-policy.md)の§4は「output modeへ切り替える前に、安全な初期出力を定義する」とする。esp-idf-halの`PinDriver`は向きを先に設定する。`unsafe`を使わずにこの区間を無くす方法は見つけていない（試していない）。電流は`R`で制限される。この点は、上の`3V3 pinへ負荷を足す条件`の残余riskに挙げた。
2. 起動時のbring-upが終わった時刻（`main.rs`の`bringup_done_ms`）から、赤だけ→白だけ→両方を10秒ずつ点ける（点灯試験）。その後のPi linkのUART初期化と`boot`送信の分だけ、最初の区間が短くなる。`bringup-display-13`付きbuildでは、LCDの描画とI2Cのbring-upがmain loopの中で進み、点灯試験はそれと並行して進む。
3. その後は白を1秒周期（0.5秒点灯・0.5秒消灯）で点滅させ、赤を消す。白を点灯へ切り替える経路と、赤を`happy`で点ける経路はまだ無い。

UART0のlogには、区間が変わるたびに`led phase=<lamp_red|lamp_white|lamp_both|link_unconfirmed> white=<on|off> red=<on|off> uptime_ms=<…>`が出る。点滅の1回ごとには出さない。pinの初期化に失敗すると`led_init_error`を出し、LEDを使わずに起動を続ける。片方のpinだけ初期化できた後に失敗した場合、そのpinはDropで無効（内部pullなし）へ戻り、消灯は外部pullだけが保つ。書き込みの失敗は`led_write_error`として数える。値と根拠は`firmware/esp32/src/config.rs`の`LED_*`定数、動作は`firmware/esp32/src/led.rs`が持つ。

**build端末と書き込み。**buildはESP32 Build profile（build-only）で、Hyper-V上のLinux VM（`systemd-detect-virt`＝`microsoft`）で行う。**この作業端末にはUSB deviceが無く、flashできない。**flashと実機試験は、USBを使える実機LinuxのESP32 Flash / HIL端末で人間が行う（[Machine Profiles](../toolchains/machine-profiles.md)）。

**検証の記録。**develop `e7ae286`（#487のB1・B2、#532、#534、#518、#535、#536、#539、#540、#537、#543、#541の後）を取り込んだ、firmwareがこの文書と同じ版のtreeで、`cargo fmt --all -- --check`と、`cargo clippy --all-targets --locked -- -D warnings`／`cargo build --locked`を既定、`bringup-led-514`、`bringup-display-13,bringup-led-514`の3構成で実行し、すべて成功した。**main checkoutの`.embuild`（ESP-IDF一式）と、前の複製の`target`のcacheを使ったbuildであり、clean buildではない。Version Recordは作っていない。**`bench-servo-test-17`は#474の`compile_error!`のため対象外である。点灯試験と点滅の時刻計算（`outputs_at`／`next_change_ms`）は、repository外の使い捨てのtestで確かめた（firmware crateはhostでtestを走らせる仕組みを持たない）。**flashと実機起動はしていない。**

## 段階的な確認

0. 上の`3V3 pinへ負荷を足す条件`を満たしたことを確認する（人間の承認の記録がある）。
1. 電源を切った状態でGPIO番号、3V3・GND、LEDの極性を確認する。`R`の袋の表示で、メーカーがFAITHFUL LINKで型番が`CF25J330RB`であることを確かめて記録する（[#538](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/538)の袋の表示の記録と同じ品であること）。違う品は使わない。各枝で`R`がLEDと直列に入っていることを目で確かめる。**`R` 2本と10 kΩ 2本を、接続前にテスター（`DT830B`）の`20k`rangeで測って記録する**（このrangeは`10 kΩ`で正常を確認済み。[hardware-bom.md](hardware-bom.md)の`MEAS-03`。最小単位は10 Ω）。テスターの確度は未取得なので（同`MEAS-03`）、この測定は取り違えの確認に使う。読みが330 Ωは`0.31`〜`0.35`、10 kΩは`10.0`前後の品だけを使い、外れた品は使わない。読みを記録する。
2. まず赤の枝（`R`、赤LED、GPIO5の10 kΩ pull-down）だけを配線し、GPIO2には何もつながない。`bringup-led-514`付きのbuildで起動する。起動・reset中に消灯していること、logが`lamp_red`と`lamp_both`の区間だけ点灯すること（`lamp_white`と`link_unconfirmed`では消灯）を、人間の監視下で確認する。**pinの初期化の瞬間に一瞬光ることはありうる**（上の`点灯試験用のfirmware`の1）。`lamp_white`の区間や`link_unconfirmed`に入った後も点いたままなら異常として止める。点灯中に`R`の両端電圧をテスターで読み、電圧÷330 Ωを電流の推定値（テスターの確度は未取得）として記録する。**この値は記録であり、合否には使わない**（[Power Budget](power-budget.md)の`電流の測定点と読みの確かさの基準`。読みを合否に使える条件は[#525](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/525)が持つ）。次に赤の枝を外し、白の枝（`R`、白LED、GPIO2の10 kΩ pull-down）だけで同じ確認を行う（`lamp_white`と`lamp_both`で点灯し、その後は点滅）。白が3.3 Vで見えなければ回路を見直す。
3. 両方を接続し、起動・reset・書き込み時に想定外の点灯や起動失敗がないか確認する。ESP32の`boot:` logでstrapping値と起動modeを確認する。電源railの電圧、LEDの電流、発熱の有無を記録する。
4. protocolの継続通信の判定と、`set_expression`から赤LEDへの経路が実装された後、Pi停止・再接続・撫で操作を含む表示動作を確認する。表示試験だけでservoの通信断安全試験を代替しない。

回路変更後の初回通電は[Hardware Safety Policy](../governance/hardware-safety-policy.md)に従い人間が監視する。電源予算への影響は2か所に分けて書いた。B-2a（`U2`）側は上の`3V3 pinへ負荷を足す条件`にある。段階C（合成給電）の境界は[Power Budget](power-budget.md)の`#514のLEDを追加する場合の差分`にある（23 ℃前提、実測前）。**どちらもLEDを含めた通電の承認ではない。**
