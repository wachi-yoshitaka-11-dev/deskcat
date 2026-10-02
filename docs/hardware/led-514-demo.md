# #514 発光ダイオードのデモ回路案

> 状態: 初回デモの配線案と、点灯試験用のfirmware（`bringup-led-514`。build確認まで）。実機での配線・通電・明るさ確認は未実施。
> **`3V3` pinへLEDを足して通電することは、まだ人間の承認を得ていない**（下の`3V3 pinへ負荷を足す条件`）。
> 対象: 筐体から見える白1本（通信）と赤1本（猫の反応）。フォトICダイオードは使わない。

## 判断の出所

| 判断 | 誰が | 記録 |
|---|---|---|
| 用途（通信状態1本、猫の演出1本）と本数（計2本） | ユーザーが承認（2026-10-01） | [#514](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514)本文 |
| 色の割当て（白＝通信、赤＝猫）と表示の意味 | ユーザーの「早く決めよう。どんどん進めよう」を受けてAIが選定（2026-10-01） | 同上 |
| GPIO2／15、10 kΩ、配線、点滅周期・点灯時間 | **AI（作業セッション）が選定した。人間の承認は得ていない** | この文書 |
| 電流制限抵抗は手持ちの270 Ωを第1案、購入する330 Ωを第2案にする。抵抗のW数が不明なことは通電の承認で残余riskとして示す | PMの判断。受け入れはユーザーが通電前に行う | [#514のコメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514#issuecomment-5942531761) |

## 役割と表示

ESP32が両LEDを駆動する。通信LEDはESP32が自分のprotocol受信状態から判定し、Piが停止しても通信未成立を示せるようにする。Piは撫でられたときの猫の反応を決め、ESP32は受け取った反応を赤LEDへ反映する。これは[プロジェクトの役割分担](../DeskCat_Microcontroller_Development_Guide.md)に沿う。

| LED | デモで表示する状態 | 実装上の前提 |
|---|---|---|
| 白 `LED-COMM-01` | ESP32が起動中でPiとの通信が未成立・喪失なら1秒周期で点滅（0.5秒ずつ）。通信が継続して成立すれば点灯 | 成立・喪失を決めるheartbeat／timeoutは[PROTO-TBD-010](../protocol/esp32-pi-protocol.md)と[HW-TBD-017](tbd-register.md)に依存する。現行firmwareは`hello`／`ping`／`get_status`を受信する（`firmware/esp32/src/pi_link.rs`）が、継続通信の判定（heartbeatとtimeout）はまだ無い。受信しただけでは、その後も通信が続いていることを示せないため、点灯へ切り替える判定はまだ実装できない。`bringup-led-514`は点滅だけを出す（下の`点灯試験用のfirmware`） |
| 赤 `LED-REACT-01` | Piが撫でられた反応として`happy`を指示したら1秒点灯し、通常時は消灯 | 既存protocolの`set_expression`を候補に使う。現行firmwareとprotocol crateは`set_expression`の受信・実行をまだ実装していない。`bringup-led-514`は点灯試験の区間だけ赤を点ける |

ESP32自体の電源断はLEDの消灯にしか見えない。白の消灯を「通信正常」と解釈しない。

## 初回デモの配線案

ESP32 boardの`3V3`系だけを使い、GPIOとLEDの間にそれぞれ独立した電流制限抵抗`R`を入れる。LEDの向きは現物の極性を確認する。

```text
白: ESP32 GPIO2 (J3-15) ─ R ─ 白LEDのアノード → カソード ─ GND
    GPIO2 ─ 10 kΩ ─ GND                     (起動時のLowを維持)

赤: ESP32 3V3 ─ R ─ 赤LEDのアノード → カソード ─ GPIO15 (J3-16)
    GPIO15 ─ 10 kΩ ─ ESP32 3V3              (起動時のHighを維持)
```

白はGPIO2をHighにすると点灯する。赤はGPIO15をLowにすると点灯する。

**起動時の電位。**[ESP32-WROOM-32D／32U datasheet v2.8](https://documentation.espressif.com/esp32-wroom-32d_esp32-wroom-32u_datasheet_en.pdf)の§3 Table 4は、strapping pinの既定をGPIO2がpull-down、MTDO（GPIO15）がpull-upとする。外部10 kΩも同じ向きに置く。同§3.1 Table 6はdownload bootを「GPIO0＝0かつGPIO2＝0」とし、GPIO2のLowはこれと整合する。同§3.3 Table 7はMTDO＝1でboot logを出す（既定）とする。GPIO0／5／12には触れない。**reset直後の内部pullより後、bootloaderが動く区間の実際の電位は確かめていない。**

**board上の接続。**[ESP32-DevKitC V4回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)（`ESP32_DevKitc_V4`、2017-12-06。この基板に当てる理由は[GPIO Assignment](gpio-assignment.md#board識別情報)）では、IO2とIO15がつながる先はmodule、未実装の`ESP32_WROVER(NC)` footprint、header `J3`だけである。**回路図の上では、board上にLED、pull抵抗、コンデンサは無い。**現物の導通は測っていない。header位置は`J3`の15番がIO2、16番がIO15であり、[Espressifのuser guide](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html)の`J3`表も同じである。

**抵抗`R`。**第1案は手持ちの270 Ω（[hardware-bom.md](hardware-bom.md)の`PROTO-03`＝K-03249の部品表にある値。部品表の記載で、現物とは未照合）、第2案は購入する330 Ω（[秋月 `CF25J330RB`](https://akizukidenshi.com/catalog/g/g125331/)、FAITHFUL LINK INDUSTRIAL CORP.、1/4 W・±5%）である。K-03249の部品表に330 Ωは無い。部品表も販売店の商品ページも、`R`の値の根拠にしない。**使う2本は接続前に必ず実測し、下の式は実測値`R_meas`で計算する**（`段階的な確認`の1）。10 kΩは手持ちの`RES-PULL-01`から2本を使う。電流制限抵抗を省略しない。

**270 ΩのW数はTBDである**（部品表に記載が無く、メーカーも特定していない）。抵抗が開放に壊れた場合は、電流が止まる側に倒れる。この枝では短絡の経路の最弱部品が`R`になり、その定格が未確定である。**通電の承認の文に残余riskとして入れる**（下の`3V3 pinへ負荷を足す条件`）。

### 電流の見積もりと残る観察

真の抵抗が`R_meas`である場合の計算で、LEDを短絡した最悪側を、ESP32 moduleの推奨給電上限3.6 V（同datasheet §5.2 Table 14）で見積もる。

```text
1枝の電流の見積もり    I_max = 3.6 V / R_meas
抵抗の損失の見積もり    P_max = (3.6 V)² / R_meas
pull 1本の電流          3.6 V / 9.5 kΩ ≈ 0.38 mA（10 kΩの−5%側）
3V3系の増分の見積もり   2 × I_max + 2 × 0.38 mA
```

テスターの確度は未取得で、真の抵抗が`R_meas`より小さい場合はこの見積もりより大きくなる（下の`3V3 pinへ負荷を足す条件`の残余risk）。

**計画値**（`R_meas`が公称の−5%側だった場合。実測値ではない）は次のとおり。

| `R` | `R_meas`（−5%側） | `I_max` | `P_max` | 3V3系の増分 |
|---|---|---|---|---|
| 270 Ω（第1案） | 256.5 Ω | 14.0 mA | 50.5 mW | 約28.8 mA |
| 330 Ω（第2案） | 313.5 Ω | 11.5 mA | 41.3 mW | 約23.7 mA |

どちらの`I_max`も、白LEDのDC絶対最大30 mA、赤LEDの50 mAより低い。GPIOの駆動能力の参考値は、同datasheet §5.3 Table 15のsource 40 mA／sink 28 mA（3.3 V・25 ℃、最大drive設定時）である。ただし**どちらもtyp値だけでmin／maxの記載が無い。全条件の保証値とは扱わない。**firmwareはdrive strengthをESP-IDFの既定のままにする。電流を決めるのは`R`である。330 Ωの計画値の`P_max`（41.3 mW）は1/4 W定格より小さい（計算）。270 Ωは定格がTBDである（上記）。**これは回路計算であり、実機測定ではない。**

白LED [`OSPW5111B-QR`](https://akizukidenshi.com/goodsaffix/OSPW5111B-QR.pdf)のVfは20 mA・25 ℃で2.8–4.0 Vなので、3.3 Vで期待する明るさは保証できない。同資料にVf–If曲線は無い。初回デモで視認できるか観察し、不足する場合は5 V系とスイッチ素子を使う回路を別途検討する。赤 [`OSHR5161A-QR`](https://akizukidenshi.com/goodsaffix/OSHR5161A-QR.pdf)も明るさを観察する。筐体の取付位置は[#34](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/34)と合わせて決める。

## `3V3` pinへ負荷を足す条件

**この条件はまだ満たしていない。満たすまでLEDをつないで通電しない。**

ESP32 boardの`3V3` pinから外部負荷を取ること（段階B-2a）は、[HW-TBD-023](tbd-register.md#hw-tbd-023)の(a)が未確定の間は許可されていない。例外は`ACCEL-01`／`ENV-01`（[#445](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/445)）と`DISP-01`（[#461](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/461)）だけであり、**どちらもLEDを含まない。**[Power Budget](power-budget.md)の`DISP-01`追加接続のbring-upの手順の条件(3)(a)も、`3V3` railに載る負荷をESP32本体・`ACCEL-01`・`ENV-01`・`DISP-01`だけに限っている。LEDの枝と10 kΩ pullを足すと、この条件から外れる。白の枝はGPIO2（`VDD3P3_RTC` domain）から流れる。その電流が`3V3` pinを経由するかは正本のどこにも記録が無い（[Power Budget](power-budget.md)のGPIO4について同じ扱い）。**ここでは安全側の仮定として`3V3`系に数える。**

**必要なのは、人間が「LEDの2枝を`3V3` pinへ足して通電してよい」と明示し、その記録が通電前に#514のコメント等で確認できることである。**AIの判断として、次の理由で承認を求める。

- **増える電流の見積もり。**LEDが短絡した場合の計算値は、270 Ωの計画値で各枝約14.0 mA、pullを含めた増分は約28.8 mAである（上の`電流の見積もりと残る観察`）。テスターの確度が未取得であることは、下の残余riskに挙げた。
- **#461と同じ枠組みで余裕を見積もれる。**#461の判断は保守側の場合(a)を使い、`3V3`系の通常動作の合計を約245.4 mA、`U2`（UMW `LD1117-3.3`）の連続定格1 Aに対して約4.1倍と見積もった（[tbd-register-history.md](tbd-register-history.md)の`HW-TBD-024`、2026-09-23追記）。270 Ωの計画値で増分の約28.8 mAを足すと**約274 mAで、約3.6倍**になる。**#461と同じくtyp値を含む見積もりであり、worst-caseは未確定のままである。**`U2`の損失の増分は、USB給電（`Vin`＝5 V）で `0.0288 A × (5.0 − 3.3) V ≈ 49 mW`、同じ記録の熱計算（`Rθja` 88 ℃/W）で温度上昇は約4.3 ℃増える（計算）。
- **故障時の熱の余裕が小さくなる。**同じ記録は、`DISP-01`の`R5`の先が短絡した場合の電流を約646.5 mAとし、そのとき働きうる保護は`U2`の熱shutdown（`TSD` typ 150 ℃）だけで、`TJ`は約141 ℃、`TSD`まで約9 ℃としている。LEDの増分を足すと約675 mA、`TJ`は約145 ℃で、**`TSD`までの余裕は約5 ℃に縮む**（計算）。
- **余裕が足りなかったときの帰結が#461と同じである。**`U2`の電流制限（`Ilimit` min 1.25 A）か熱shutdownによるrail低下・reset・表示不良であり、#461はこれを機能の失敗として受け入れた。

- **残余risk（承認の文に入れる）：**
  - 270 ΩのW数は確認できていない。最悪の損失は約50 mW（計算）。抵抗が開放に壊れた場合は電流が止まる側に倒れる。
  - `U2`の余裕はtyp値を含む見積もりで、worst-caseは未確定である。`DISP-01`の`R5`の先が短絡する故障では、`TSD`までの余裕が約9 ℃から約5 ℃に縮む（計算）。
  - 赤LEDの型番`OSHR5161A-QR`は、ユーザーの申告とセット写真による識別であり、ロットを照合していない（2026-10-01のユーザー判断）。
  - firmwareの初期化で、GPIO15が一瞬Low（赤が点く側）になりうる。Hardware Safety Policy §4（output modeへ切り替える前に安全な初期出力を定義する）から外れる。長さは測っていない（下の`点灯試験用のfirmware`の1）。
  - テスター（`DT830B`）の確度は未取得であり（[hardware-bom.md](hardware-bom.md)の`MEAS-03`）、抵抗の実測値から真の抵抗の下限は保証できない。

**承認されるまで、この節は判断の材料であって許可ではない。**

## 点灯試験用のfirmware

`firmware/esp32`の`bringup-led-514` featureを付けたbuildだけがGPIO2／GPIO15を駆動する。既定buildは両pinに触れない。既定buildとも`bringup-display-13`とも組み合わせられる（`bench-servo-test-17`は#474で`compile_error!`のため対象外）。build commandの正本は[検証済みコマンド](../toolchains/verified-commands.md)である。

1. `Peripherals::take()`の直後に両pinを出力にし、消灯側（GPIO2はLow、GPIO15はHigh）へ設定する。**向きの設定と消灯levelの設定の間に、GPIO15が一瞬Low（点灯側）になりうる。長さは測っていない。**[Hardware Safety Policy](../governance/hardware-safety-policy.md)の§4は「output modeへ切り替える前に、安全な初期出力を定義する」とする。esp-idf-halの`PinDriver`は向きを先に設定する。`unsafe`を使わずにこの区間を無くす方法は見つけていない（試していない）。電流は`R`で制限される。この点は、上の`3V3 pinへ負荷を足す条件`の残余riskに挙げた。
2. 起動時のbring-upが終わった時刻（`main.rs`の`bringup_done_ms`）から、赤だけ→白だけ→両方を10秒ずつ点ける（点灯試験）。その後のPi linkのUART初期化と`boot`送信の分だけ、最初の区間が短くなる。`bringup-display-13`付きbuildでは、LCDの描画とI2Cのbring-upがmain loopの中で進み、点灯試験はそれと並行して進む。
3. その後は白を1秒周期（0.5秒点灯・0.5秒消灯）で点滅させ、赤を消す。白を点灯へ切り替える経路と、赤を`happy`で点ける経路はまだ無い。

UART0のlogには、区間が変わるたびに`led phase=<lamp_red|lamp_white|lamp_both|link_unconfirmed> white=<on|off> red=<on|off> uptime_ms=<…>`が出る。点滅の1回ごとには出さない。pinの初期化に失敗すると`led_init_error`を出し、LEDを使わずに起動を続ける。片方のpinだけ初期化できた後に失敗した場合、そのpinはDropで無効（内部pullなし）へ戻り、消灯は外部pullだけが保つ。書き込みの失敗は`led_write_error`として数える。値と根拠は`firmware/esp32/src/config.rs`の`LED_*`定数、動作は`firmware/esp32/src/led.rs`が持つ。

**build端末と書き込み。**buildはESP32 Build profile（build-only）で、Hyper-V上のLinux VM（`systemd-detect-virt`＝`microsoft`）で行う。**この作業端末にはUSB deviceが無く、flashできない。**flashと実機試験は、USBを使える実機LinuxのESP32 Flash / HIL端末で人間が行う（[Machine Profiles](../toolchains/machine-profiles.md)）。

**検証の記録。**develop `095f3fa`（#487のB1・B2の後）へ載せ直した版で、`cargo fmt --all -- --check`と、`cargo clippy --all-targets --locked -- -D warnings`／`cargo build --locked`を既定、`bringup-led-514`、`bringup-display-13,bringup-led-514`の3構成で実行し、すべて成功した。**main checkoutの`.embuild`（ESP-IDF一式）を共有したbuildであり、clean buildではない。Version Recordは作っていない。**`bench-servo-test-17`は#474の`compile_error!`のため対象外である。点灯試験と点滅の時刻計算（`outputs_at`／`next_change_ms`）は、repository外の使い捨てのtestで確かめた（firmware crateはhostでtestを走らせる仕組みを持たない）。**flashと実機起動はしていない。**

## 段階的な確認

0. 上の`3V3 pinへ負荷を足す条件`を満たしたことを確認する（人間の承認の記録がある）。
1. 電源を切った状態でGPIO番号、3V3・GND、LEDの極性を確認する。各枝で`R`がLEDと直列に入っていることを目で確かめる。**`R` 2本と10 kΩ 2本を、接続前にテスター（`DT830B`）の`20k`rangeで測って記録する**（このrangeは`10 kΩ`で正常を確認済み。[hardware-bom.md](hardware-bom.md)の`MEAS-03`。最小単位は10 Ω）。読みには最小単位1つ分（10 Ω）の幅がある。**テスターの確度は未取得で（同`MEAS-03`）、実測値から真の抵抗の下限は保証できない。**読みが270 Ωは`0.27`〜`0.28`、330 Ωは`0.33`〜`0.35`、10 kΩは`10.0`前後の品だけを使い、外れた品は使わない。使った品の読みを`R_meas`として、`I_max`と`P_max`を記録する。
2. まず赤の枝（`R`、赤LED、GPIO15の10 kΩ pull-up）だけを配線し、GPIO2には何もつながない。`bringup-led-514`付きのbuildで起動する。起動・reset中に消灯していること、logが`lamp_red`と`lamp_both`の区間だけ点灯すること（`lamp_white`と`link_unconfirmed`では消灯）を、人間の監視下で確認する。**pinの初期化の瞬間に赤が一瞬光ることはありうる**（上の`点灯試験用のfirmware`の1）。`lamp_white`の区間や`link_unconfirmed`に入った後も点いたままなら異常として止める。点灯中に`R`の両端電圧をテスターで読み、電圧÷`R_meas`を電流の推定値（確度未取得）として記録する。次に赤の枝を外し、白の枝（`R`、白LED、GPIO2の10 kΩ pull-down）だけで同じ確認を行う（`lamp_white`と`lamp_both`で点灯し、その後は点滅）。白が3.3 Vで見えなければ回路を見直す。
3. 両方を接続し、起動・reset・書き込み時に想定外の点灯や起動失敗がないか確認する。ESP32の`boot:` logでstrapping値と起動modeを確認する。電源railの電圧、LEDの電流、発熱の有無を記録する。
4. protocolの継続通信と`set_expression`の受信経路が実装された後、Pi停止・再接続・撫で操作を含む表示動作を確認する。表示試験だけでservoの通信断安全試験を代替しない。

回路変更後の初回通電は[Hardware Safety Policy](../governance/hardware-safety-policy.md)に従い人間が監視する。電源予算への影響は2か所に分けて書いた。B-2a（`U2`）側は上の`3V3 pinへ負荷を足す条件`にある。段階C（合成給電）の境界は[Power Budget](power-budget.md)の`#514のLEDを追加する場合の差分`にある（23 ℃前提、実測前）。**どちらもLEDを含めた通電の承認ではない。**
