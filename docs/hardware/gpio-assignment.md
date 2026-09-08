# GPIO Assignment

> 状態: Blocked — ただし理由は`#2`のchecklistではない。**`#2`（GPIO割り当ての承認）の受け入れ条件に対応するchecklist5件（1・2・4・6・7）は2026-09-07にすべて解消した**（1・2・6・7は達成、4は`#13`へ送った。下記）。**この文書がなお`Blocked`である理由は、サーボ出力を有効化してよいかの判断（[servo-safety-limits.md](servo-safety-limits.md#サーボ出力を有効化してよい条件)のgate）が未解決であることであり、`#2`のGPIO割り当て承認とは別のgateである。**
>
> **`#2`のclose条件（2026-09-07にPM `deskcat-f2`が再スコープ）: 1行ずつ書く。**
> (1) `ADC-5V`／`ADC-3V3`分圧器（`MEAS-01`の10 kΩ×4） → **2026-09-07にブレッドボードへ実装完了**。
> (2) `ACCEL-SDA`／`ACCEL-SCL`の外部4.7 kΩ pull-upと`LCD-BL`の外部4.7 kΩ pull-down → **2026-09-07に実装完了**（`LCD-CS`／`LCD-RST`／`TOUCH-CS`の3本は実装済み・3.3V接続確認済み）。
> (6) `LCD-RST`／`LCD-CS`／`LCD-BL`が`EN`でresetに保持した状態で安全な状態にあること → **2026-09-07に再測定し達成**（`experiment-log.md` `EXP-011`）。
> (7) `TOUCH-CS`が同じく`EN`保持で安全な状態にあること → **2026-09-07に再測定し達成**（同上）。
> **`#2`のclose条件はこの4件で全て達成した。**
>
> **`#2`のclose条件ではない（moduleをESP32へ配線した時点で満たす。`#13`／`#15`／`#16`側。2026-09-07にPM `deskcat-f2`が判定）:**
> (3) I2C実効pull-upが有効範囲内であることの確認待ち（`Cb`未測定。詳細は下記）。
> (4) MSP2807のlogic IO levelが非通電の現物確認待ち → 追跡を試みず`#13`（LCD bring-up）の通電実測へ送った。
> (5) ESP32電源投入前の外部moduleによるpin駆動有無が非通電の導通check待ち → moduleがESP32へ配線されるまで検証対象が存在しない。
>
> **servo起動時状態（`SERVO-PWM`のGPIO27起動時state）の安全reviewは2026-09-06に机上で完了し、上記のいずれにも含まれない。**サーボ出力を有効化してよいかの判断も別であり、[servo-safety-limits.md](servo-safety-limits.md#サーボ出力を有効化してよい条件)のgateが未解決のまま`Blocked`である。
> **I2C実効pull-upの有効範囲（(3)）は、2026-09-06に初回bring-upのmodeをStandard-modeと決定したことで一旦「達成」へ改めたが、同日PR #358のCodeRabbit手動reviewの指摘を受けて未達へ戻した。**mode決定（Standard-mode）自体は取り消していない。rise timeの制約（`Rp(max)`）には実配線の`Cb`が400 pFを超えても488 pFまで余裕があるが、**Standard-modeの規定`Cb`上限400 pFがrise time以外の制約（fall time等）にも由来する可能性を、この文書が引用した一次資料の範囲では排除できないため、`Cb`の実測または設計上の根拠を得るまで達成にしない。**根拠は[初回bring-upのmode決定](#初回bring-upのmode決定)節と競合checklistの当該項目。
> **(6)(7)は`#247`が止めている案A（Pi→ESP32のUSB OTG給電）を待たない。**`EN`保持測定はESP32単体・PC USB給電（段階B-1）で足り、`power-budget.md`は段階Aと段階B-1をgate不要・追加購入不要と定めている
> （**touch controller型番は2026-08-13に`XPT2046`と確定し、`HW-TBD-003`は2026-08-15にcloseした**）
> 正本とする情報: ESP32 boardのpin割り当て

## 割り当て規則

- 正確な現物board（下記「Board識別情報」参照）と搭載moduleの文書を使用する。
- flash、bootstrapping、USB-UART、board LED、使用制限のあるpinを考慮する。
- すべてのmoduleについて、電圧と起動時drive stateを確認する。
- 物理信号ごとに一行を使用する。
- Tutorialまたは類似boardのGPIO番号をコピーしない。
- Firmwareのpin定数は、この文書から生成するか、この文書と手動で同期させる。

## Board識別情報

| 項目 | 値 | 根拠 |
|---|---|---|
| Board family | ESP-WROOM-32D開発ボード（秋月電子 M-13628）。Espressif ESP32-DevKitC V4 wide版（38pin、flash pin露出タイプ）のpin配置に相当 | [hardware-bom.md](hardware-bom.md) MCU-01、現物写真（`D0`–`D3`／`CMD`／`CLK`相当のpin露出）、基板裏面silkscreen「**`ESP32_DevKitc_V4`**」（2026-08-15に大文字小文字を訂正。旧記載 `ESP32_DevkitC_V4`） |
| 正確なboard revision | 基板自体にrevision表示なし | 現物確認済み（`hardware-bom.md` Revision履歴3）。**旧記載の理由「秋月オリジナル基板のため」は根拠が無いため削除した**（`hardware-bom.md` Revision 29） |
| 搭載ESP32 module suffix | ESP-WROOM-32D | 購入履歴（秋月電子 M-13628商品名）、`hardware-bom.md` |
| 公式回路図revision | **正は[ESP32-DevKitC V4公式回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)**（title block `ESP32_DevKitc_V4`、2017-12-06。2026-08-10に図面を直接読み、`J2`／`J3`の19pin×2列の対応を取得済み）。[Espressif ESP32-DevKitC V4 pinout](https://docs.espressif.com/projects/esp-idf/en/v5.1/esp32/hw-reference/esp32/get-started-devkitc.html)のpin description表も同じ並びを示すが、**番号の正は回路図とする**。**照合は2026-08-13に完了し、一致した。**38pinヘッダ両側のsilkが公式`J2`／`J3`と19pin×2列すべてで一致した（GNDの位置を含む）。左列 `3V3 EN VP VN 34 35 32 33 25 26 27 14 12 GND 13 D2 D3 CMD 5V`、右列 `GND 23 22 TX RX 21 GND 19 18 5 17 16 4 0 2 15 D1 D0 CLK`（[tbd-register HW-TBD-001](tbd-register.md)） | Espressif公式資料。**秋月商品ページの添付はモジュールとチップのdatasheetのみで、boardのpin配列表・回路図を含まない**（旧記載はこれを照合先としていたが、実在しなかった） |
| Firmware board configuration ID | TBD | Toolchain bring-up時（#5）に定義する |

## 電圧domain（すべての外部pull-upに適用）

**この設計に5V logicは存在しない。**ESP32のGPIOは3.3V系であり、周辺moduleも
すべて3.3Vで給電する（`power-budget.md`の電源rail構成案を参照）。したがって次を守る。

- この文書で「pull-up」と書いた抵抗は、**すべて3.3Vへ接続する**。5Vへ接続しない。
- 5Vへpull-upすると、ESP32のGPIOと周辺module双方が定格超過となり破損しうる。
- 5V railはservoとlogic基板への給電に使用する。**GPIOへ5Vを直接入力してはならない。**
- **例外は`ADC-5V`だけである。**5V railの電圧を測るため、**指定した分圧器（10 kΩ／10 kΩ、比1/2）を介して**GPIO33へ入れる。分圧後は約2.5 Vであり、5VがGPIOへ直接掛かることはない。**分圧器を省いて直結すると、ADC定格3.3 Vを超えて破損する。**

## 起動時状態を確定させる外部pull

**この節は値と本数の導出の正本である。**選定した値は`信号inventory`の`Pull`列にも書く。
**[I2C busの実効pull-up](#i2c-busの実効pull-up)とは別の計算である。**
あちらはbus容量とrise timeから決まる。**こちらは漏れ電流、受け側の入力閾値、driverと競合しないことから決まる。**
**同じ部品（[hardware-bom.md](hardware-bom.md) `RES-PULL-01`）を使うが、片方の値を他方へ流用しない。**

**この節が決めるのは値と本数である。**GPIO割り当ての承認ではない。
**この文書の状態は`Blocked`のままである。**

### 一次資料と、そこから取った値

| 出所 | 取った値 |
|---|---|
| **ESP32 Series Datasheet v5.3、Table 5-3 `DC Characteristics (3.3 V, 25 °C)`（p.52）** | `VIH` ≥ 0.75×VDD、`VIL` ≤ 0.25×VDD、`IIH`／`IIL` ≤ **50 nA**、`VOH` ≥ 0.8×VDD、`VOL` ≤ 0.1×VDD、`IOH` typ 40 mA（`VDD3P3_CPU`／`VDD3P3_RTC` domain）／20 mA（`VDD_SDIO` domain）、`IOL` typ 28 mA、内部pull-up／pull-down抵抗 `RPU`／`RPD` typ **45 kΩ**（内部weak pullの駆動能力は約75 µA） |
| **同 Appendix `IO_MUX` の`At Reset`／`After Reset`列** | 下表の`reset時`。**resetの間は全pinがoutput-disableである**（同appendixの注記9） |
| **同 §3.2 `Internal LDO (VDD_SDIO) Voltage Control`（p.24）** | `MTDI`（GPIO12）= 0（既定）で`VDD_SDIO`は`VDD3P3_RTC`から直接給電され、典型値は3.3 Vである |
| **ILI9341 Datasheet V1.11 §18.2.1 `General DC Characteristics`（p.236）** | `VIH` ≥ 0.7×VDDI、`VIL` ≤ 0.3×VDDI、`IIH` ≤ **1 µA**、`IIL` ≥ −1 µA、`ILEA` ±0.1 µA、`VDDI` 1.65–3.3 V |
| **同 §12.1／§12.2（p.214–215）** | `RESX`が電源投入時にHighまたは不定なら、**VCIとVDDI投入後にhardware resetを当てる必要がある**（timing制約なし）。Lowで安定なら**投入後10 µs以上Lowに保つ**必要がある |
| **XPT2046 Datasheet（2007.5）`DIGITAL INPUT/OUTPUT`** | Logic FamilyはCMOS。`VIH` ≥ 0.7×IOVDD（\|`IIH`\| ≤ **5 µA**）、`VIL` ≤ 0.3×IOVDD（\|`IIL`\| ≤ 5 µA）、入力容量5–15 pF |
| **同 `PENIRQ Output`** | `PENIRQ`は**内部pull-up付きの出力**である。公称50 kΩで、process・温度で36 k–67 kΩに振れる。**logic low 0.35×(+VCC)を保証するには、X+とY−間の合成抵抗が21 kΩ未満である必要がある** |

**polarityは一次資料で確定した。**ILI9341の`CSX`と`RESX`はどちらも**active low**である
（同datasheet。`RESX`は`Signal is active low.`と明記）。XPT2046の`CS`も**active low**である
（同datasheetのpin表で`CS`にoverlineが付く）。**したがって`LCD-CS`と`TOUCH-CS`のpolarityは現物確認を要しない。**

### reset時のpin状態（`IO_MUX`から）

| 信号 | GPIO | Pin No. | Power domain | `At Reset` | 意味 |
|---|---|---|---|---|---|
| `SERVO-PWM` | 27 | 16 | `VDD3P3_RTC` | `oe=0, ie=0` | **内部pullが無い。真のhigh-Zである** |
| `LCD-BL` | 4 | 24 | `VDD3P3_RTC` | `oe=0, ie=1, wpd` | **内部weak pull-downが有効である。floatingではない** |
| `LCD-RST` | 16 | 25 | **`VDD_SDIO`** | `oe=0, ie=0` | 内部pullが無い |
| `LCD-CS` | 22 | 39 | `VDD3P3_CPU` | `oe=0, ie=0` | 内部pullが無い |
| `TOUCH-CS` | 21 | 42 | `VDD3P3_CPU` | `oe=0, ie=0` | 内部pullが無い |

**`SERVO-PWM`のreset時状態が`oe=0, ie=0`であることは、外部pull-downが必須である理由そのものである。**
内部pullが無いため、**ESP32側には線をLowへ引く要素が何も無い。**

### pull-upの上限（`Rmax = (VDD - VIH) / I漏れ`）

VDD = 3.3 Vである（`電圧domain`節）。受け側の`VIH`は0.7×3.3 = **2.310 V**である。

| 信号 | 受け側 | 数える漏れ電流 | `Rmax` | 選定値10 kΩの余裕 |
|---|---|---|---|---|
| `TOUCH-CS` | XPT2046 `CS` | 5 µA（XPT2046）＋50 nA（ESP32）＝5.05 µA | **196 kΩ** | **約20倍** |
| `LCD-CS` | ILI9341 `CSX` | 1 µA（ILI9341）＋50 nA（ESP32）＝1.05 µA | **943 kΩ** | **約94倍** |
| `LCD-RST` | ILI9341 `RESX` | 同上 | **943 kΩ** | **約94倍** |

**下限は駆動側で決まる。**10 kΩのとき、ESP32がLowを出す間に流れるのは
(3.3 − 0.33) / 10 kΩ = **297 µA**であり、`IOL` typ 28 mAの**1.1 %**である。
受け側の`VIL`（0.3×3.3 = 0.990 V）に対して、ESP32の`VOL`は0.1×3.3 = 0.330 V以下であり**余裕がある。**

### 選定した値と本数

| 信号 | 向き | 値 | 本数 | 決め手 |
|---|---|---|---|---|
| **`SERVO-PWM`** | **pull-down（必須）** | **4.7 kΩ** | **1** | 下記「`SERVO-PWM`を4.7 kΩにした理由」。**2026-08-26に一般値側と決まったため確定した** |
| `LCD-CS` | pull-up | **10 kΩ** | 1 | `Rmax` 943 kΩに対して94倍の余裕。駆動負荷は`IOL`の1.1 % |
| `LCD-RST` | pull-up | **10 kΩ** | 1 | 同上。**あわせて下記「`LCD-RST`の2つの注意」** |
| `TOUCH-CS` | pull-up | **10 kΩ** | 1 | `Rmax` 196 kΩに対して20倍の余裕 |
| `LCD-BL` | pull-down | **4.7 kΩ** | 1 | **2026-09-06に確定した。**極性は2026-09-05に一次資料（MSP2807公式User Manual）で`active-high`（`LED` pin「high level lighting」）と判明済みであり、`SERVO-PWM`と同じ理由（未知の競合電流に対しては、駆動側に余裕がある範囲で値を下げるほうが安全側）で`4.7 kΩ`を採った。下記「`LCD-BL`を決められない理由」 |
| `TOUCH-IRQ` | **外部pullを付けない** | — | **0** | 下記「`TOUCH-IRQ`へ外部pull-upを付けてはならない」 |

10 kΩ（秋月 125103）と4.7 kΩ（秋月 125472）がどちらも1袋100本入で2026-08-08に着荷している
（[hardware-bom.md](hardware-bom.md) `RES-PULL-01`）。**確定した5本（4.7 kΩ×2 と 10 kΩ×3）は手元の2種でまかなえる。追加の発注は要らない。**
**ただし現物の表示・値の確認はしていない**（同BOMの`着荷済み`と`受け入れ済み`の区別）。
消費電力はどちらも問題にならない（3.3 Vで4.7 kΩが2.32 mW、10 kΩが1.09 mW。**1/4 W = 250 mWの1 %未満**）。

#### `SERVO-PWM`を4.7 kΩにした理由

**この信号だけ、上限を一次資料から計算できない。**SG90の`logic閾値`と入力インピーダンスは
**どの一次資料にも記載が無い**（2026-08-24に確定。[`HW-TBD-026`](tbd-register.md)(a)）。
**したがって「Lowと解釈される上限電圧」が無く、`Rmax`を出せない。**

**それでも決められることがある。**

- **既知の漏れだけを数えた持ち上がりは、どちらの値でも無視できる。**
  ESP32側の漏れは50 nA以下であり、4.7 kΩで**0.235 mV**、10 kΩで**0.500 mV**である。
- **駆動側の下限は両方とも余裕がある。**ESP32がHighを出す間に流れるのは
  4.7 kΩで**702 µA**（`IOH` typ 40 mAの1.8 %）、10 kΩで330 µA（0.8 %）である。
- **未知はservo側が信号線へ流し込む電流である。**同じ電流に対して、
  **4.7 kΩは10 kΩの半分の電圧しか持ち上がらない。**

**未知に対して強い側が安全側である。**上限が計算できない状況では、**下限側に余裕がある範囲で値を下げるほうが安全である。**
4.7 kΩは駆動負荷が`IOH`の1.8 %にとどまり、手元にもある。**したがって4.7 kΩ×1本を推奨とする。確定はしない。**

**振り分けは2026-08-26に決着した。**経緯を残す。[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の対応表は
「pull-upとdecouplingの値」を**一般値で開始してよい側**、「サーボPWM、可動域、速度、加速度」を**一次資料を要する側**に置く。
**`SERVO-PWM`のpull-down抵抗値は両方に読めた。**pull抵抗の値であるから前者に読め、
外すとreset時のhigh-Z区間でservoが不定pulseを受け、機構へ押し付けられれば安全5項目の
「servoの持続的拘束」に至りうるから後者にも読める。

**2026-08-26に人間が一般値側と決めた。**判断の材料は3つである。

- **持続的拘束に至る経路が成立しにくい。**servoが動くには有効なPWM pulse列が要る。
  high-Zで浮いた線が出すのは静的なレベルかノイズであり、pulse列ではない。
- **一次資料側に振ると無期限に止まる。**SG90の`logic閾値`とPWM受理条件は公式datasheetが存在せず、
  仕様表にも記載が無い（2026-08-24に全数確認）。一次資料を要求すると存在しない文書を待つことになる。
- **値の妥当性は文書ではなく実測で決まる。**`HW-TBD-027`の証拠契約は項目4でreset中の実測を求めており、
  格を上げてもこの実測は要る。

**この決定が変えたのは値の根拠の水準だけである。pull-down自体が必須であることは変わらない。**

**揃えた材料は3つである。**(1) reset時のpin状態（`oe=0, ie=0`＝内部pull無しの真のhigh-Z）、
(2) 内部pullの強さ（`RPU`／`RPD` typ 45 kΩ、駆動能力約75 µA。**ただしGPIO27にはそれが無い**）、
(3) **不定pulseが実際にservoを動かしうるかは判定できない**（SG90の`logic閾値`とPWM受理条件が
一次資料に無い。[`HW-TBD-026`](tbd-register.md)(a)(b)(c)）。**(3)が埋まらないため、
「動かない」とも「動く」とも言えない。**

**この選定は`HW-TBD-027`をcloseしない。**同行の証拠契約は選定のほかに購入・実装・reset中の実測を要求し、
**その判定閾値は`HW-TBD-026`が決まるまで確定しない。**値を決めたことと、Lowであることを確かめたことは別である。

#### `LCD-RST`の2つの注意

1. **`LCD-RST`（GPIO16）は`VDD_SDIO` domainにある。**この domain の電圧は`MTDI`（GPIO12）のreset時の値で決まる
   （同datasheet §3.2）。**`MTDI`はreset時に内部weak pull-downが有効（`oe=0, ie=1, wpd`）であり、既定は0である。**
   0なら`VDD_SDIO`は`VDD3P3_RTC`から直接給電され3.3 Vになる。
   **module内のflashは`VDD_SDIO`で給電されており、2026-08-20にflashと起動が成立していることから、
   この基板では`VDD_SDIO` = 3.3 Vである**（[Version Record](../toolchains/version-records/2026-08-20-esp32-flash-boot-native.md)）。
   **3.3 Vへのpull-upが定格内である前提はここにある。**`MTDI`をHighへ引く改造を行うと`VDD_SDIO`は1.8 Vになり、**この前提が崩れる。**
2. **pull-upを付けても、firmwareのhardware resetは省けない。**ILI9341 §12.1は、`RESX`が電源投入時に
   **Highまたは不定**なら「VCIとVDDI投入後にhardware resetを当てる必要がある。当てなければ正しい動作を保証しない」としている。
   **`High`と`不定`を同じ扱いにしている。**したがってpull-upの効果は「初期化を正しくすること」ではなく、
   **入力を閾値付近に浮かせないことと、意図しないreset assertを防ぐことである。**
   **§12.2（Lowで安定＝pull-down）も一次資料が認めた選択肢であり、その場合は投入後10 µs以上Lowを保つ必要がある。**
   `HW-TBD-032`は極性も選定対象に含めている。**この節はpull-upの値を`正本`の推奨に沿って決めたが、
   向きそのものの最終判断は人間に残る。**

#### `LCD-BL`を4.7 kΩにした理由

**旧見出し（`LCD-BLを決められない理由`）は2026-09-06時点で実体と合わなくなっていた。**
極性はすでに2026-09-05に確定しており（下記）、この節が「決められない」理由として
挙げていた2点はどちらも解消済みか、そもそも値の決定条件ではなかった。**見出しを含めて訂正する。**

- **極性は2026-09-05に一次資料で確定した。**MSP2807公式User Manual（LCDWIKI、
  `2.8inch_SPI_Module_MSP2807_User_Manual_EN.pdf`）のInterface Description表が
  `LED` pinを「high level lighting」と定めている（正は
  [sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`Backlight回路／電流／polarity`行、
  Revision 12）。**`active-high`（HighでOn）であり、`信号inventory`が前提としてきた
  「外部pull-down推奨」の向きと一致する。**現物確認は要しない。
- **backlight回路の入力条件のうち、点灯時に流れる電流の上限は未確定のままである**
  （`R5`＝6.8 Ω／`R6`＝1 kΩ／`Q1`＝`J3Y`は2026-08-13に読み取ったが、**パターンを追っていないため
  回路modelが確定していない**。[`HW-TBD-024`](tbd-register.md)）。**ただし2026-09-06に判定した
  とおり、この未確定はpull-down値の選定を止めない。**`HW-TBD-024`が要るのは「backlightを
  点灯させるときにどれだけ電流を流してよいか」（段階B-2の電流制限設定）であり、pull-downの
  仕事は「firmware初期化前にGPIO4をLowへ保ち、backlightを消したままにする」ことである。
  **消えている状態では、backlightのLED電流経路そのものに電流が流れない。**したがって
  点灯時の耐性電流上限を知らなくても、消灯を維持するpull-downの値は選べる。
  **旧記載（`HW-TBD-024`の後に決める）はこの2つの問いを混同していた。訂正する。**
- **ESP32側は分かっている。**GPIO4はreset時も reset後も`oe=0, ie=1, wpd`であり、**内部weak pull-downが有効である。**
  **この pin は floating ではない。**外部pull-downは内部`RPD`（typ 45 kΩ）と並列になる
  （外部10 kΩで8.18 kΩ、外部4.7 kΩで4.26 kΩ）。
- **値は`SERVO-PWM`と同じ理由で`4.7 kΩ`を採った。**backlight回路の正確な入力impedanceが
  未確定（上記）であるため`Rmax`を一次資料から計算できない。**未知に対して強い側が安全側である**
  という`SERVO-PWM`で確立した原則（上限が計算できない状況では、下限側（駆動能力）に余裕がある
  範囲で値を下げるほうが安全）をそのまま適用し、手元の2種のうち低い方（`4.7 kΩ`）を選定した。
  **これは`hardware-safety-policy.md`の対応表が「pull-upとdecouplingの値」を一般値で開始してよい
  側に置いていることに基づく一般値であり、一次資料からの導出ではない。**

**したがって`LCD-BL`の値・向き・本数は2026-09-06に確定した（`4.7 kΩ`・pull-down・1本）。**
`HW-TBD-032`が対象とする残作業は実装と起動時の状態確認だけである（本節と`信号inventory`の
`LCD-BL`行を参照。ここへ再掲しない）。

#### `TOUCH-IRQ`へ外部pull-upを付けてはならない

**`信号inventory`の同行は「外部pull-up推奨（一般的なtouch controllerはactive-low IRQ。要現物確認）」としていたが、
これはcontrollerが未確定だった時期の記述である。**2026-08-13に`XPT2046`と確定した。

**XPT2046の`PENIRQ`は内部pull-up付きの出力である**（公称50 kΩ、36 k–67 kΩ）。**外部pull-upは不要であり、有害である。**

- **不要**: 内部pull-upだけで、GPIO34の漏れ（50 nA以下）に対する High は3.297 V である。
  ESP32の`VIH`（0.75×3.3 = 2.475 V）に対して1.33倍の余裕がある。
- **有害**: datasheetは`logic low 0.35×(+VCC)`を保証する条件として**X+とY−間の合成抵抗が21 kΩ未満**であることを挙げている。
  外部10 kΩを並列に足すと実効pull-upは8.33 kΩになり、同じ21 kΩに対する low は
  **0.716×VCC**（2.362 V）まで上がる。**0.35×VCCを大きく超え、touchがLowとして読めなくなる。**
  4.7 kΩならさらに悪く0.830×VCCである。

**したがって`TOUCH-IRQ`の外部pull本数は0本である。**

## I2C busの実効pull-up

**この節は計算の式と前提の正本である。**`I2C sensor bus`行はここを参照する。

**この節は必須要件ではない。**I2Cのpull-up値は
[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の対応表で
**一般値で開始してよい側**に置かれている（2026-08-26。ADR-0014／0016）。
**したがって一次資料から式を導くことは要求されていない。一般値で始めてよい。**
外れても busが応答しないだけで、安全要件5項目のどれにも当たらない。
**この節を置く理由は、始めた値で動かなかったときに診断できるようにするためである。**
値を詰める必要が出た時点で、下の式と境界表をそのまま使える。**甲乙丙の区分を通す必要も無い**
（同policyが「5項目に効かない値は出所を問わない」と定める）。

**値そのものはまだ確定していない。**下記「まだ確定できない2つの入力」が埋まるまで決まらない。
**ただし確定を待つ必要は無い。**上のとおり一般値で開始してよい。

**起動時の状態を確定させるための外部pull（[`HW-TBD-027`](tbd-register.md)／[`HW-TBD-032`](tbd-register.md)、[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)）とは別の計算である。**
あちらは漏れ電流とノイズ耐性、driverと競合しないことから決まる。**こちらはbus容量とrise timeから決まる。**
同じ部品（[hardware-bom.md](hardware-bom.md) `RES-PULL-01`）を使うが決定は別であり、**片方の値を他方へ流用しない。**

### 式（一次資料）

正本は **I2C-bus specification and user manual UM10204 Rev. 7.0（NXP B.V.、2021-10-01）**の
[§7.1 `Pull-up resistor sizing`](https://web.archive.org/web/2023/https://www.nxp.com/docs/en/user-guide/UM10204.pdf)（p.50/62）である。
**NXPの直リンク `https://www.nxp.com/docs/en/user-guide/UM10204.pdf` は404を返す**（2026-08-25確認）ため、
同一pathのarchive snapshotを参照先にする。

- **Rp(max) = tr / (0.8473 × Cb)**
  係数`0.8473`は、`VIL(max)`＝0.3VDDから`VIH(min)`＝0.7VDDまでのRC充電時間である。
  同節が導出を示している（`t1 = 0.3566749 × RC`、`t2 = 1.2039729 × RC`、`T = t2 - t1 = 0.8473 × RC`）。
- **Rp(min) = (VDD(max) - VOL(max)) / IOL**
  同文書§7.2.4の計算例が同じ式である（`VDD = 5 V ± 10 %`、`VOL(max) = 0.4 V` at 3 mA で
  `Rp(min) = (5.5 - 0.4) / 0.003 = 1.7 kΩ`）。

規定値は同文書のTable 10（p.43/62）とTable 11（p.44/62）による。

| Mode | `fSCL` max | `tr` max | `Cb` max | `IOL`（`VOL` = 0.4 V） |
|---|---|---|---|---|
| Standard-mode | 100 kHz | 1000 ns | 400 pF | 3 mA |
| Fast-mode | 400 kHz | 300 ns | 400 pF | 3 mA |
| Fast-mode Plus | 1000 kHz | 120 ns | 550 pF | 20 mA |

`VOL1`の最大値は0.4 V（open-drain、Table 10）である。

### 前提

- **VDD = 3.3 Vである**（`電圧domain`節。この設計に5V logicは存在しない）。
  したがって`IOL` 3 mAの側では **`Rp(min)` = (3.3 - 0.4) / 0.003 = 約967 Ω** である。
  **これは公称3.3 Vでの値であり、受け入れの下限そのものではない。**`Rp(min)`は`VDD(max)`で決まる。
  この基板の3.3 V railは`U2 = UMW LD1117-3.3`の出力で **3.234–3.366 V** である
  （正は[power-budget.md](power-budget.md)。**ここへ再掲しない**）。**上限3.366 Vを入れると`Rp(min)`は約989 Ωになる。**
  **ESP-WROOM-32Dの`VDD33`が受けられる3.6 Vを使わない。**あれはmoduleの受入範囲であって、この基板のrail電圧ではない。
  **選定した10 kΩはどちらで測っても約10倍の余裕があり、この差は選定を変えない。**
- **BME280側の4.7 kΩは数えない。**`J1`／`J2`が開放でbusへ繋がっていないことを2026-08-22に実測で確定した
  （正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)。**ここへ再掲しない**）。
- **module搭載のpull-upとして数える候補はADXL345側の`01C`（EIA-96で10 kΩ、1%）だけである**（同文書`Module搭載pull-up`）。**ただし`ACCEL-SDA`／`ACCEL-SCL`行（[信号inventory](#信号inventory)）で外部`4.7 kΩ` pull-upを選定済みであり、実際のbus上ではmodule搭載分と並列になる。**下の表はmodule搭載分だけの参考値であり、受け入れ計算にはその下の実構成値を使う。

### 確定した入力・まだ確定できない1つの入力

1. **ADXL345 module側の`01C`（10 kΩ）4個がどのpinへ付くかは、2026-08-27に現物写真のパターンを
   追って確定した。**`SDA`へ2本、`SCL`へ2本であり、それぞれ並列合成で**各line 5.00 kΩ**になる
   （[`HW-TBD-004`](tbd-register.md)）。
2. **採るmodeは、2026-09-06に初回bring-upの範囲としてStandard-modeへ決定した。**
   理由と根拠は下記「初回bring-upのmode決定」節。[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の
   `検証済み最大bus速度`は、実測して確認した値ではないため`TBD`のまま変更しない
   （**この決定は「採用するmode」であり「実測して確認した最大速度」ではない**）。
3. **bus容量`Cb`を得ていない。**`Cb`は配線・接続・pinの合計容量であり、**実配線が存在しない。**
   `J1`／`J2`をはんだ付けするかの判断にはこの入力が引き続き要る。

### 判断に使える境界

上の式へ値を入れたものである。**確定値ではなく、確定した時点で判断に使う境界である。**

`Rp`に許される`Cb`の上限（`Cb(max) = tr / (0.8473 × Rp)`）。

**下表はmodule搭載のpull-upだけを数えた参考値であり、外部`4.7 kΩ`（`ACCEL-SDA`／`ACCEL-SCL`）を含まない。受け入れ計算には使わない。**

| `Rp`（module搭載分のみ） | 成立する場合 | Standard-mode | Fast-mode |
|---|---|---|---|
| 10 kΩ | ADXL345側の1本だけがbusに付く（**現物確認により不成立。各lineに2本ずつ付く**） | 118 pF | 35 pF |
| 5.00 kΩ | **ADXL345側の`01C`が2本並列で同じlineに付く（2026-08-27に現物確認で確定。SDA/SCLとも該当）** | 236 pF | 71 pF |
| 3.20 kΩ | 10 kΩに`J1`／`J2`の4.7 kΩを並列に足す（**`J1`／`J2`は開放で実測済みのため、この構成は現状不成立**） | 369 pF | 111 pF |

**実構成（受け入れ計算に使う値）**: ADXL345側の`01C`並列5.00 kΩと、外部選定済みの`4.7 kΩ`（`ACCEL-SDA`／`ACCEL-SCL`）が同じlineで並列になる。

| `Rp`（実構成） | 内訳 | Standard-mode | Fast-mode |
|---|---|---|---|
| 約2.42 kΩ | ADXL345側5.00 kΩ ∥ 外部4.7 kΩ | 約488 pF | 約145 pF |

**あわせて次が言える。**Fast-modeで`Cb`が規定上限の400 pFに達した場合、
`Rp(max)` = 300 ns / (0.8473 × 400 pF) = **約885 Ω**となり、**`Rp(min)`を下回る**（公称3.3 Vでの約967 Ω、rail上限3.366 Vでの約989 Ωのいずれに対しても下回る）。
**この組み合わせは3.3 Vの受動pull-upでは成立しない。**
UM10204 Table 10の注記も、400 kHzでfull bus loadを駆動するには`VOL` = 0.6 Vで`IOL` 6 mAが要るとしている。
**したがってmodeと`Cb`の決定は、pull-up値の選定と切り離せない。**

### 初回bring-upのmode決定

**2026-09-06に、初回bring-upで採るmodeをStandard-mode（100 kHz）と決定した。**
**この決定自体は取り消していない。**ただし[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)の受け入れchecklist
「Moduleのpull-upを並列合成した実効抵抗が有効範囲内である」は、この決定だけでは閉じない
（2026-09-06、PR #358のCodeRabbit手動reviewでの指摘を受けて訂正した。詳細は競合checklistの
当該項目を参照。**ここへ再掲しない**）。

**mode決定の根拠。**上の`判断に使える境界`節のとおり、実構成（`Rp`約2.42 kΩ）はStandard-modeの
規定`Cb`上限（400 pF）でも`Rp(max)`約2.945 kΩを下回り、**rise timeの制約だけを見れば実配線の
`Cb`を測らなくても余裕がある。**Fast-modeは規定`Cb`上限で`Rp(max)`約885 Ωとなり実構成を大きく
下回るため成立しない（現在のpull-up構成のままではFast-modeを採れない）。**ただし「rise timeの
制約に余裕がある」ことと「実効抵抗がStandard-modeの規定範囲内にあることの確認」は同じではない**
（理由は競合checklistの当該項目に記載）。

**用途側の要求を確認した。**現時点でこのbusを使う受け入れ条件（[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)
のaccelerometer bring-up、[#22](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/22)の軽打→驚く反応統合）は、
tapしきい値・retrigger動作・end-to-end latencyを「測定する」と書くのみで、**具体的な数値要求（下限bus速度）を挙げていない。**
[#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)（boot/ping/status/ACK/reconnect）はUSB serial protocol側であり、
I2C busの速度と無関係である。加速度の軽打検出は`ACCEL-IRQ`によるhardware割り込みで受ける設計（`信号inventory`）であり、
割り込み後にレジスタを数byte読むだけであれば100 kHzでも遅延はサブミリ秒order、BME280のpollingは秒orderであるため、
**bus速度がこれらの検出遅延を決めない。**したがってStandard-modeを妨げる下流要件は現時点で見つからない。

**この決定の性質を明記する。**これは**初回bring-upの選択であり、恒久的な確定ではない。**
見直す条件は次のとおりである。

- 後の統合（[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)／[#22](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/22)）で
  Fast-mode相当の速度が実際に必要だと判明した場合、**まずpull-up値の再設計から着手する**
  （現在の実構成`Rp`約2.42 kΩはFast-modeのRp(max)約885 Ωを満たさないため、値を下げるかADXL345側の
  搭載pull-upとの並列関係を見直す必要がある）。
- `sensor-datasheet-notes.md`の`検証済み最大bus速度`は、この決定によっても`TBD`のまま変更しない
  （この決定は採用するmodeであって、実測して確認した最大速度ではないため）。
- `J1`／`J2`をはんだ付けするかの判断は、この決定の対象外である。`bus容量Cb`が実配線が無く
  未確定のままであり、`J1`／`J2`（BME280側の4.7 kΩ追加）を含めた実構成でも同じ余裕があるかは
  別途計算が要る。

### `J1`／`J2`の判断

**まだ決めていない。**残る未確定入力は`bus容量Cb`だけであり、これが埋まった時点でこの節の式で決める。
**計算前にはんだ付けしない。**

## I2C addressの選択

**この節は判断材料までである。**

**addressは一般値で開始してよい側である**（[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の対応表。2026-08-26）。
**外れても deviceが応答しないだけで壊れない。**したがって「決まらないから配線できない」ではない。
**下の材料は、どちらを採るかを選ぶためのものであって、着手の前提条件ではない。**
**候補値そのものの正本は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)であり、ここへ再掲しない。**
**実装は配線であり、`J3`のはんだ付けと同じ機会に行う作業である。**

| 判断材料 | 内容 |
|---|---|
| **衝突では決まらない** | ADXL345の候補（`0x1D`／`0x53`）とBME280の候補（`0x76`／`0x77`）は、**2×2の4通りすべてで重複しない**（2026-08-25に確認）。**どの組み合わせを採っても衝突回避の観点では差が付かない** |
| **未接続は選択肢ではない** | **両moduleとも`SDO`が基板上でどこにも固定されていないことを2026-08-22に実測で確定した。**したがって**配線しなければaddressが定まらない。**未接続のまま通電しない |
| **`0x76`はmodule資料が「既定」と記す側である** | `SDO`→GNDが`0x76`である。**driverやlibraryの既定値と一致しやすく、実装時の不一致を減らせる。**これは実装コストの差であって電気的な優劣ではない |
| **2個目のBME280は現時点の判断材料にならない** | 同一busへ同種deviceを2個載せるなら両addressが要るが、**初期MVPにその計画は無い** |
| **ADXL345側も同じ状態である** | ADXL345の`実装済みI2C address`も`TBD`であり、`SDO/ALT ADDRESS`の配線で決まる（[`HW-TBD-004`](tbd-register.md)）。**BME280側だけを決めても、bus上のaddressは確定しない** |

**この節が決めていないこと。**

- **どちらのaddressを採るか。**上の材料には電気的な優劣が無く、**実装コストの差だけである。**
- **ADXL345側のaddress。**追跡は[`HW-TBD-004`](tbd-register.md)である。

## ESP32の使用制限pin（Espressif公式資料より、この基板に適用）

| 区分 | GPIO | 制約 |
|---|---|---|
| Flash通信専用（**使用禁止**） | 6, 7, 8, 9, 10, 11（`CLK`／`D0`／`D1`／`D2`／`D3`／`CMD`） | 内蔵SPI Flashとの通信に使用。外部回路から絶対に使用しない |
| Strapping pin（起動modeを決定。用途を厳選） | 0, 2, 5, 12, 15 | GPIO0: boot button。GPIO2: download mode判定。GPIO12(MTDI): flash電圧選択（Highだと起動しない可能性）。GPIO15(MTDO): boot logのsilence制御。今回の割り当てでは**いずれも使用しない**（安全側） |
| UART0（Flashingとboard上USB-UARTブリッジ専用） | 1（TX）, 3（RX） | **firmware flashingとdebug log専用。**board上のUSB-UARTブリッジが占有するため、外部配線用のGPIOとして使わない。**Pi linkはUSB serialであり、この2本は使わない**（下記`Pi–ESP32間のtransport`） |
| Input-only（出力不可） | 34, 35, 36（VP）, 39（VN） | 純粋なinput信号（interrupt、ADC）にのみ割り当て可 |
| WROOM/SOLO-1専用（WROVERでは予約） | 16, 17 | 今回のmoduleはESP-WROOM-32Dのため使用可 |

## 信号inventory

| Signal ID | Device | 信号 | ESP32側の方向 | GPIO | Boot state | Pull | Bus設定 | 共有先 | 制約／根拠 |
|---|---|---|---|---|---|---|---|---|---|
| LCD-SCLK | DISP-01 | SCLK | Output | GPIO18 | 起動時floating（input）。CSがinactiveの間はbus上で無害 | 外部pull不要 | VSPI、SPI mode要確認（ILI9341は一般にMode0）。速度は実測で確認 | TOUCH-01と共有 | ESP32 VSPIの既定CLK pin。Flash／strapping pinではない |
| LCD-MOSI | DISP-01 | MOSI | Output | GPIO23 | 同上 | 外部pull不要 | 同上 | TOUCH-01と共有 | ESP32 VSPIの既定MOSI pin |
| LCD-MISO | DISP-01 | MISO | Input | GPIO19 | 同上 | 外部pull不要 | 同上 | TOUCH-01と共有 | ILI9341自体はMISO未使用の可能性が高い（要現物確認）。Touch controller（**`XPT2046`。2026-08-13に現物刻印で確定**）の読み取りに使用 |
| LCD-CS | DISP-01 | Chip select | Output | GPIO22 | 起動時floating→firmware初期化前は不定 | **外部`10 kΩ`×`1本`を選定した**（2026-08-25。active-low CSをfirmware初期化前もinactive＝Highに保つため）（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**2026-08-27にブレッドボード上へ実装した。通電・検証は未了である** | **Active-low（一次資料で確定）。**ILI9341 datasheet V1.11が`CSX`をactive lowと明記している。**現物のpolarity確認は要しない** | なし | Output設定前にinactiveにする。Pull-up未実装の場合、起動直後の数十ms間bus contentionのriskがある |
| LCD-DC | DISP-01 | Data／command | Output | GPIO17 | floating | 外部pull不要（WROOM-32Dのため使用可） | Device固有（要現物確認） | なし | WROOM/SOLO-1専用pin。今回のmoduleはWROOM-32Dのため使用可 |
| LCD-RST | DISP-01 | Reset | Output | GPIO16 | floating | **外部`10 kΩ`×`1本`を選定した**（2026-08-25）（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**pull-upを付けてもfirmwareのhardware resetは省けない**（ILI9341 §12.1は`RESX`がHighまたは不定なら電源投入後にhardware resetを要するとしている）。**GPIO16は`VDD_SDIO` domainであり、3.3 Vへのpull-upが定格内である前提は同domainが3.3 Vであることに依る**（同節）。**2026-08-27にブレッドボード上へ実装した。通電・検証は未了であり、向きの最終判断は`HW-TBD-032`に残る** | **Active-low（一次資料で確定。**ILI9341 datasheet V1.11が`RESX`を`Signal is active low.`と明記**）。**Pulse timingは現物確認後に決定 | なし | 起動時glitchを防ぐ。Firmwareが最初にHighを出力するまでの間もHighに保つ設計が望ましい |
| LCD-BL | DISP-01 | Backlight | Output（PWM調光は将来検討） | GPIO4 | **不定**。ESP32のGPIO4はreset時にinput（内部weak pull-downあり）で、driveされた状態にはならない。ただし内部weak pullは外部回路に対して弱く、backlight回路の入力仕様によっては点灯しうる。firmwareまたは外部pullが確定させるまでOffを保証しない | **外部pull-down（確定）**（backlightをfirmware初期化前もOffに確定させるため）。**`4.7 kΩ`×`1本`を2026-09-06に確定した。**極性は2026-09-05に一次資料（MSP2807公式User Manual、`LED` pin「high level lighting」）で`active-high`と確定済みであり、現物確認は要しない（正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`Backlight回路／電流／polarity`行）。値は`SERVO-PWM`と同じ理由（未知の競合電流に対しては値を下げるほうが安全側）で選定した。**backlight点灯時の耐性電流上限（`HW-TBD-024`）はこの値の決定条件ではない**（消灯を維持するpull-downの仕事に点灯時の電流上限は関係しない。導出は[`LCD-BLを4.7 kΩにした理由`](#lcd-blを47-kωにした理由)節。**ここへ再掲しない**）。**本数は1本。2026-09-07にブレッドボードへ実装し（GPIO4→抵抗→GND帯）、同日の`EN`保持測定でGPIO4＝0.00 Vを得た。****確認できたのはESP32側の信号レベルまでである。**`EXP-011`は実機のLCD／touch panelを接続せずに行っており、**backlightそのものが物理的にOffであることは未確認である。**確認できたのは「reset中、GPIO4という制御信号がLowへ確定すること」であり、backlightをdriveする実際の回路（module側）を含めた消灯確認はLCD接続後に残る。**あわせてGPIO4は内部weak pull-down（`wpd`）が有効なため、この電圧測定だけでは外部pull-downの存在を証明しない**（`LCD-CS`／`LCD-RST`は内部pullが無いため区別できるが、`LCD-BL`は異なる）。**外部pull-downの存在そのものの確認と、LCD接続後の物理的なbacklight消灯確認の両方が残る**（正は下記`受け入れchecklist`の該当項目と[experiment-log.md](experiment-log.md)の`EXP-011`。**ここへ再掲しない**） | 現状はdigital on/off。将来PWM調光も可能なpinを選定 | なし | 回路上のLED電流経路はmodule内蔵に依存。直接大電流をdriveしない（module側で電流制限されている前提、現物確認要） |
| TOUCH-CS | TOUCH-01 | Chip select（touch controller用、LCD-SPIバスを共有） | Output | GPIO21 | 起動時floating→不定 | **外部`10 kΩ`×`1本`を選定した**（2026-08-25。LCD-CSと同じ理由）（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**根拠は別である**（受け側がXPT2046であり、漏れ電流が\|`IIH`\| ≤ 5 µAでILI9341の1 µAより大きい。`Rmax`は196 kΩ）。**2026-08-27にブレッドボード上へ実装した。****2026-08-29に通電しての実測を行い、3.30 Vを得た。****ただしこの値は、pull-upが効いている場合と、firmwareがGPIO21をHighへ駆動している場合を区別しない。**測定時にboard上で走っていたfirmwareを同定していないため、**pull-upが正常であることの根拠にはならない**（受け入れchecklist参照） | **Active-low（一次資料で確定）。**XPT2046 datasheetのpin表が`CS`にoverlineを付けている。**現物のpolarity確認は要しない** | DISP-01とSCLK／MOSI／MISOを共有 | **Touch controllerは2026-08-13に`XPT2046`と確定した**（現物chip刻印。`hardware-bom.md` TOUCH-01）。polarityは`XPT2046`のdatasheetで確認する |
| TOUCH-IRQ | TOUCH-01 | Interrupt（touch検出） | Input | GPIO34 | 入力専用、floating | **外部pullを付けない（本数0本）。**2026-08-25に一次資料から判定した。**XPT2046の`PENIRQ`は内部pull-up付きの出力**（公称50 kΩ）であり、外部pull-upは不要かつ**有害である**（並列に足すとlow levelが`0.35×VCC`の保証を超える）（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**旧記載「外部pull-up推奨（一般的なtouch controllerはactive-low IRQ。要現物確認）」はcontroller未確定時の記述であり、2026-08-13の`XPT2046`確定で前提が変わっていた** | Edge／level要確認 | なし | Input-only pin。Output不可のため他用途に転用できない |
| ACCEL-SDA | ACCEL-01 | I2C SDA | Bidirectional | GPIO25 | floating（open-drain想定） | 外部4.7kΩ pull-up（**一般値での開始値である。**I2Cのpull-up値は[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の対応表で一般値で開始してよい側に置かれている（2026-08-26。ADR-0014／0016）。**導出された確定値ではない。**実効pull-upの式は[I2C busの実効pull-up](#i2c-busの実効pull-up)節にある。**ADXL345モジュールは`01C`＝10 kΩのpull-upを4個搭載しており、2026-08-27に現物写真でパターンを確認したところSDA・SCLへ各2本ずつ付いている**（各line並列合成で5.00 kΩ）） | **100 kHz(Standard-mode)を初回bring-upとして採用した**（2026-09-06。導出は[初回bring-upのmode決定](#初回bring-upのmode決定)節。**ここへ再掲しない**）。**Fast-modeへの変更にはpull-up値の再設計が要る**（現構成はFast-modeの`Rp(max)`を満たさない）。旧記載「400kHz(Fast-mode)を想定、要実測」はmode決定前の記述であり訂正した | ENV-01と共有 | ADXL345はI2C／SPI選択式。Interface選択jumperの現物確認が必要（`hardware-bom.md` ACCEL-01） |
| ACCEL-SCL | ACCEL-01 | I2C SCL | Bidirectional | GPIO26 | 同上 | 同上 | 同上 | ENV-01と共有 | 同上 |
| ACCEL-IRQ | ACCEL-01 | Interrupt（tap／free-fall検出） | Input | GPIO35 | 入力専用 | **外部pull要確認**（`HW-TBD-004`）。**ICの事実:**ADXL345のINT1/INT2は**push-pull固定**であり、設定で切り替えられない（`Both interrupt pins are push-pull, low impedance pins`。Rev. G page 19）。polarityは`DATA_FORMAT` register（`0x31`）の`INT_INVERT` bitで選び、**同registerのreset値が`00000000`であるためICの既定はactive-highである**（Rev. G Table 19 page 23、page 27）。**旧記載の「push-pull／open-drainを設定可能」はICの事実として誤りであり、2026-08-12に訂正した**（Revision 9）。**module levelは別である。**M-06724のboard上でINT pinがheaderへ直結しているか（直列抵抗、level shift、引き出しの有無）を示す資料が無いため、**外部pullの要否とheaderで観測されるpolarityは現物確認まで確定しない。ICがpush-pullであることからmoduleの配線条件を導かない**（[tbd-register HW-TBD-004](tbd-register.md)） | Edge想定 | なし | ADXL345のtap／free-fall検出hardwareを軽打／持ち上げ判定に使う場合に使用（`hardware-bom.md` ACCEL-01の採用理由） |
| ENV-SDA | ENV-01 | I2C SDA | Bidirectional | GPIO25（ACCEL-01と共有） | 同上 | 同上 | 同上 | ACCEL-01と共有 | BME280はI2C／SPI選択式。**2026-08-22に選択jumperを実測し、`J1`／`J2`／`J3`は3つとも開放であると確定した**（正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`現物の実装状態を実測で確定させた（2026-08-22）`）。**したがってI2Cで使うには`J3`のはんだ付けが要る**（実装作業）。**module搭載の4.7 kΩプルアップも繋がっていない** |
| ENV-SCL | ENV-01 | I2C SCL | Bidirectional | GPIO26（ACCEL-01と共有） | 同上 | 同上 | 同上 | ACCEL-01と共有 | 同上 |
| SERVO-PWM | SERVO-01 | PWM control | Output | GPIO27 | **不定**。ESP32のGPIO27はreset時にhigh-Z（output disable、input disable）であり、Lowにdriveされる保証はない。**Lowと仮定しない。**外部pull-downが確定させるまで、servoは不定pulseを受けうる | **外部pull-down必須**（推奨ではない）。high-Z期間中もLowを保証する唯一の手段であり、これがないとPWM driver初期化前にservoが動きうる。詳細は`servo-safety-limits.md`。**`4.7 kΩ`×`1本`で確定した。**導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない。****この抵抗値を一般値で開始してよい側に置くことを2026-08-26に人間が決めた**（[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の対応表は「pull-upとdecouplingの値」を一般値側、「サーボPWM、可動域、速度、加速度」を一次資料側に置き、**この項目は両方に読めた**）。**pull-down自体が必須であることは変わらない。**一般値側になったのは値の根拠の水準だけである。**上限はSG90の`logic閾値`が一次資料に無いため計算できない**（[`HW-TBD-026`](tbd-register.md)(a)）。**そのうえで、駆動側の下限に余裕がある範囲で未知に強い側（低い値）を採った。**部品は`hardware-bom.md`の`RES-PULL-01`（10 kΩと4.7 kΩが各1袋100本入、2026-08-08着荷。**追加の発注は要らない**）。**2026-08-27にブレッドボード上へ実装した。通電・検証は未了である** | 50Hz、pulse幅は`servo-safety-limits.md`で規定する制限に従う | なし | Strapping pinでもflash pinでもない。起動時とdriver故障時の状態は`tbd-register.md` HW-TBD-019で引き続き検討する |
| ADC-SHUNT | MEAS-01 | Servo rail低側shuntの電圧 | Input（ADC1_CH4） | GPIO32 | 入力専用扱い、high-Z | 外部pull不要（shunt両端が電位を決める） | ADC1、減衰0 dB（0–1.1 V）。0.1Ω×最大2 A＝0.2 Vがfull scale内 | なし | ADC1のためWi-Fi動作中も使用可。ADC2は**Wi-Fi有効時に使用不可**のため測定へ割り当てない。低電流側の精度限界（実用域は約1 A以上）は`power-budget.md`の測定計画を参照 |
| ADC-5V | MEAS-01 | 5 V railの電圧 | Input（ADC1_CH5） | GPIO33 | 入力専用扱い、high-Z | 分圧器10 kΩ／10 kΩ（比1/2）。分圧後の最大は約2.5 V | ADC1、減衰11 dB（約0–3.1 V）。分圧なしでは5 VがADC定格3.3 Vを超え破損する | なし | 分圧比は10 kΩ抵抗で構成する（`hardware-bom.md` MEAS-01）。**`ADC-5V`と`ADC-3V3`で計4本を使う。抵抗は入手済みであり**（2026-08-08着荷、1袋100本入。2026-08-12に購入履歴と照合して訂正した）、**残るのは実装と検証である** |
| ADC-3V3 | MEAS-01 | ESP32 3.3 V railの電圧 | Input（ADC1_CH0） | GPIO36（VP） | 入力専用、high-Z | 分圧器10 kΩ／10 kΩ（比1/2）。分圧後の最大は約1.65 V | ADC1、減衰11 dB | なし | 3.3 Vは減衰11 dBのfull scale（約3.1 V）を超えるため直結しない。Input-only pinのためoutputへ転用不可 |
| UART-TX | Firmware flashingとdebug log（**Pi linkではない**） | TX | Output | GPIO1（固定、board上USB-UARTブリッジへ内部接続） | SDK既定（起動logを出力） | 変更不可（chip内蔵UART0） | 115200 8N1（候補、`esp32-pi-protocol.md`で最終確定） | Boot log | board上のUSB-UARTブリッジが占有するため、**外部配線用のGPIOとして使用しない**。Pi linkは下記のとおりUSB connector経由であり、この2本をPiへ直接配線しない |
| UART-RX | Firmware flashingとdebug log（**Pi linkではない**） | RX | Input | GPIO3（固定） | 同上 | 変更不可 | 同上 | Flashing | 同上 |

正確なmoduleが使用しない信号は削除し、不足しているreset、enable、address-select、interrupt、power-control信号はすべて追加する。

## Pi–ESP32間のtransport（USB serialに確定）

[Protocol](../protocol/esp32-pi-protocol.md)が`物理／論理link`を**USB serial**とProject decisionで
確定しているため、この文書もUSB serialだけを採る。**GPIO UARTによる直接配線は採用しない。**
両者はconnector、配線、flashing手順が異なるため、片方に統一しないと配線が決まらない。

| 項目 | 採用する方式 | 採用しない方式 |
|---|---|---|
| 物理接続 | Pi（USB host）のUSB OTG port ⇔ ESP32 boardのMicro USB port を**USB cable 1本**で接続する | ESP32のGPIO1／GPIO3とPiのGPIO14／GPIO15をjumperで直接配線する |
| ESP32側の経路 | board上のUSB-UARTブリッジICが内部でUART0（GPIO1／GPIO3）へ接続する。GPIO headerには何も配線しない | GPIO1／GPIO3をheaderから引き出す |
| Pi側のdevice | USB CDC serial（`/dev/ttyUSB*`。実際の名称は#8で確認） | `/dev/serial0`（Pi内蔵UART） |
| 追加部品 | Pi側がMicro-B（OTG）のため、**USB OTG変換（Micro-B → Type-A）またはMicro-B ⇔ Micro-B OTG cable**が必要。**2026-08-22に手持ちで充当と確定した**（`hardware-bom.md`の`CABLE-PI-LINK-01`。購入待ちリストから外した） | jumper wireのみ |

この結果、GPIO1／GPIO3は**board上のブリッジが占有する予約pin**であり、外部配線用に空いていない。
PCからflashingするときは同じUSB portを使うため、Piとの同時接続は想定しない。

## Bus計画

| Bus | 候補device | 状態 | 不足している根拠 |
|---|---|---|---|
| USB serial（Pi link） | Raspberry Pi | **USB connector経由に確定**（GPIO配線なし）。GPIO1／GPIO3はboard上ブリッジの予約pin | Pi上のdevice名（`/dev/ttyUSB*`等）は#8で確認。USB OTG変換cableが**手持ちで充当**（2026-08-22） |
| ADC測定（`power-budget.md`） | Shunt、5 V rail、3.3 V rail | GPIO32／33／36に確定（すべてADC1） | 分圧器の実装と実測値。ADC2はWi-Fi有効時に使用不可のため割り当てない |
| SPI display bus | LCD（MSP2807／ILI9341）、touch（同module） | GPIO18／23／19（SCLK／MOSI／MISO）＋CS個別（LCD: GPIO22、Touch: GPIO21）に確定 | Touch controller型番の現物確認、実際のSPI mode／速度の実測 |
| I2C sensor bus | Accelerometer（ADXL345）、environment sensor（BME280） | GPIO25（SDA）／GPIO26（SCL）に確定 | **BME280側のjumperは2026-08-22に実測で確定した**（`J1`／`J2`／`J3`はすべて開放）。**残るのはADXL345側のpin接続の確認と、実効pull-up抵抗の計算である。****計算の式と前提は[I2C busの実効pull-up](#i2c-busの実効pull-up)節が正本であり、ここへ再掲しない。**同節は2026-08-25に一次資料（UM10204 Rev. 7.0 §7.1）から式と規定値を確定させた。**値が決まらない理由は3つある**（ADXL345側のpin接続、bus容量`Cb`、採るmode）。**いずれも同節に書いた。** **2026-08-22にBME280側のjumperを実測した。`J1`／`J2`はどちらも開放であり、module搭載の4.7 kΩプルアップはbusへ繋がっていない**（正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`現物の実装状態を実測で確定させた（2026-08-22）`。**ここへ再掲しない**）。**したがって実効pull-upの計算にBME280側の4.7 kΩを入れない。**`J1`／`J2`をはんだ付けするかは、この計算の結果で決める。**まだ決めていない。****計算前にはんだ付けしない。****あわせて`J3`が開放であるため、I2Cで使うには`J3`のはんだ付けが要る。** |
| PWM／timer | Servo（SG90） | GPIO27に確定 | `servo-safety-limits.md`のpulse幅制限確定、起動時安全状態のreview |

## 競合check

- [x] 割り当てたpinがmodule flash用に予約されていない（GPIO6-11を使用していないことを確認済み）
- [x] Outputがbootstrap要件と競合しない（GPIO0/2/5/12/15を一切使用していない）
- [x] UART flashingとboot logを引き続き利用できる（GPIO1/3を変更していない）
- [x] Input-only制約を守っている（GPIO34/35/36は入力専用として使用。GPIO36はADC-3V3、outputへ転用しない）
- [x] ADC測定pinを予約済みで、ADC2をWi-Fi併用下で使っていない（GPIO32/33/36はすべてADC1）
- [x] 5 Vと3.3 V railのADC入力に分圧器が実装され、ADC定格3.3 Vを超えない（分圧比1/2を規定済み。**2026-09-07にブレッドボード上へ実装した。**`ADC-5V`(GPIO33)側は`5V`ピン→10 kΩ→分圧点(青)→GPIO33、分圧点→10 kΩ→GND。`ADC-3V3`(GPIO36)側は3V3帯→10 kΩ→分圧点(白)→GPIO36、分圧点→10 kΩ→GND。**抵抗の個別実測（DT830B）は行っていない。**5 %許容差の着荷済み品であり`Rp`等の余裕は桁で足りるため、2026-09-07に人間と合意のうえ個別実測をしないと決めた。**通電しての起動時電圧の検証はこの項目の対象外**（ADCは受動素子でありreset時のGPIO駆動状態に関わらない）
- [x] 共有SPI上の各deviceに個別CSがある（LCD: GPIO22、Touch: GPIO21）
- [x] I2C deviceのaddressが一意、または明示的な対策がある（**候補の全組み合わせで衝突しない。**ADXL345は`0x1D`（`SDO/ALT ADDRESS`をhigh）／`0x53`（同pinをGNDへ）、BME280は`0x76`（`SDO`→GND）／`0x77`（`SDO`→VDD）であり、**2×2の4通りすべてで重複が無い**（2026-08-25に確認）。**したがってaddressの選択は衝突回避を理由に決まらない。**候補の出所は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)であり、**ここへ再掲しない。****両moduleとも`SDO`が基板上で固定されていないことを実測で確定しており**（2026-08-22）、**どちらのaddressも配線しなければ定まらない。**未接続のまま通電しない。選択の判断材料は[I2C addressの選択](#i2c-addressの選択)節にある）
- [x] すべての外部pull-upが3.3Vへ接続され、5Vへ接続されていない（`電圧domain`節で規定済み。`LCD-CS`／`LCD-RST`／`TOUCH-CS`の3本（10 kΩ pull-up）はRevision 20でブレッドボードへ実装済みであり、Revision 21の配線色確認（黄／紫／茶がいずれも`3V3`帯）で3.3Vへの接続を確認済み。**残る2本（`ACCEL-SDA`／`ACCEL-SCL`の外部4.7 kΩ pull-up、`LCD-BL`のpull-down`4.7 kΩ`）も2026-09-07にブレッドボードへ実装した。**`ACCEL-SDA`(GPIO25)→抵抗(青)→3V3帯、`ACCEL-SCL`(GPIO26)→抵抗(緑)→3V3帯。`LCD-BL`はpull-downのためGND帯へ接続（GPIO4→抵抗(紫)→GND帯）であり、この項目（3.3Vへの接続）の対象は`ACCEL-SDA`／`ACCEL-SCL`の2本である。5Vへの誤接続は無い）
- [ ] Moduleのpull-upを並列合成した実効抵抗が有効範囲内である（**式と前提の正本は[I2C busの実効pull-up](#i2c-busの実効pull-up)節である。****ADXL345は`01C`＝10 kΩ×4を搭載しており、2026-08-27に現物写真でパターンを確認した結果、SDA・SCLへ各2本ずつ付き、各lineの合成値は5.00 kΩである。**BME280は`J1`／`J2`のはんだジャンパで4.7 kΩの接続を選ぶ設計で、**2026-08-22の導通確認で両方とも開放と確定した。したがって実効pull-upの計算にBME280側の4.7 kΩを入れない。****`J1`／`J2`をはんだ付けするかはこの計算の結果で決める。まだ決めていない（残る未確定入力は`bus容量Cb`のみ。採るmodeは下記のとおり2026-09-06に決定済み）。**

**この項目は`#2`のclose条件ではない（2026-09-07、PM `deskcat-f2`判定）。**`Cb`の実測にはI2C busへ実際にmoduleを接続した配線が要り、`初回bring-upの範囲`節が定める`#2`の範囲（ESP32単体・pull資源の割り当て）に含まれない。満たすのはmoduleをESP32のI2C busへ配線した時点であり、`#13`／`#15`／`#16`側で扱う。
  **2026-09-06に、`SERVO-PWM`項目と同じ「値によらず判定できるか」を検討したが、この項目は同じ形では閉じられない。**`Rp(min)`側（約967 Ω、`VDD(max)`と`IOL`だけで決まる）は実構成の`Rp`約2.42 kΩが常に上回り、`Cb`／modeの値によらず成立する。**しかし`Rp(max)`側は`Cb`とmodeに依存し、両方の値によっては不成立になる組み合わせが実在する。**Fast-modeで`Cb`が規定上限の400 pFに達した場合、`Rp(max)`は約885 Ωとなり実構成の約2.42 kΩを下回る（`判断に使える境界`節に既出）。**したがって「`Cb`とmodeが何であっても有効範囲内」とは言えない。**servoの場合と違うのは、servoは「どんな閾値でもPWMとして復号されない」という**否定形**を示せたのに対し、こちらは「ある組み合わせ（Fast-mode かつ 高`Cb`）では実際に成立しない」ことがすでに分かっている点である。**閉じるには、少なくとも採るmodeを決める必要がある。**Standard-modeを採る場合に限定すれば、同modeの規定`Cb`上限（400 pF）における`Rp(max)`は約2.945 kΩであり、実構成の約2.42 kΩはこの上限を下回るため、**実配線の`Cb`を測らなくても「Standard-modeなら有効範囲内」と言える。**この条件付きの結論はmode選定の判断材料になった。**2026-09-06に、初回bring-upで採るmodeを
  Standard-modeへ決定した（人間の判断。根拠は[初回bring-upのmode決定](#初回bring-upのmode決定)節）。
  **Standard-modeが決まったことで、規定`Cb`上限（400 pF）における`Rp(max)`約2.945 kΩを実構成の
  約2.42 kΩが下回ることが確定した。**
  **2026-09-06に、CodeRabbitの手動review（PR #358）で、この結論だけでは項目を達成にできないという指摘を受け、検討のうえ採用した。**
  `Rp(max)`約2.945 kΩという数字は**rise time（`tr`）の制約からRpの側で導いた上限であり、
  「実配線のCbがStandard-modeの規定上限400 pFを超えていないこと」を示すものではない。**
  実構成`Rp`（約2.42 kΩ）を使えば、rise timeの制約だけを見るなら実際には`Cb`が約488 pFまで
  許容される（`Rp(max) = tr/(0.8473×Cb)`を`Cb`について解いた値）。**しかしStandard-modeの
  `Cb`上限400 pFは、この文書が引用した一次資料の範囲では、rise time以外の制約（fall time等、
  driveする素子の出力impedanceに依存し`Rp`の選定では動かせない可能性がある制約）から来ている
  可能性があり、この文書は`tr`／`Cb`／`IOL`の列しか引用していないため、その制約の有無を
  確認できていない。**したがって、実配線の`Cb`が400 pFを超えて488 pF以下に収まる場合、
  rise timeの計算だけは通っても、Standard-modeとして規定範囲内と言い切れない可能性が残る。
  **実配線の`Cb`が存在しない現状では、これを判定できない。****mode決定（Standard-mode採用）は
  取り消さない。**初回bring-upの選択としては変わらず有効である。**ただしこの項目（実効抵抗が
  有効範囲内である）は、実配線の`Cb`を測定するか、`Cb`≤400 pFを裏付ける設計上の根拠を得るまで、
  未達のまま残す。**`J1`／`J2`をはんだ付けするかの判断もこのmode決定の対象外であり、
  `bus容量Cb`が未確定のまま別途残る
- [ ] MSP2807のlogic IOが3.3Vで動作することを現物で確認した（VCC 3.3–5V対応だがlogic IOは3.3V TTL。`power-budget.md`参照。**確認方法は[実機check（電源off）の確認方法](#実機check電源offの確認方法)節を参照。記載だけでは足りないと判定した**。**2026-09-07、PM `deskcat-f2`判定によりこの項目は`#2`のclose条件ではない。**`#360`自身が「`U1`出力側の脚とcontroller IC電源pinの導通追跡は、`R5`／`Q1`と同じ理由（部品が小さくプローブを確実に当てられない）で決まらない可能性が高い」としており、追跡を試みず`#13`（LCD bring-up）の通電実測へ送る。判定材料の`VOH`／`VIH`計算は同節にある（`VCC`直結なら約160–170 mVの余裕、`U1`出力経由なら約34–133 mV不足）。`#13`が見るべきはmodule→ESP32の向き（touchの`DOUT`／`PENIRQ`等）のみで、ESP32→moduleの向きはどちらの想定でも問題ない）
- [ ] ESP32の電源投入前に外部moduleがESP32 pinをdriveしない（未検証、実機電源offでの導通checkが必要。**確認方法は[実機check（電源off）の確認方法](#実機check電源offの確認方法)節を参照**。**2026-09-07、PM `deskcat-f2`判定によりこの項目は`#2`のclose条件ではない。**moduleをESP32へ配線しないと検証対象（module電源pin⇔ESP32`3V3`pinの導通）が存在せず、`初回bring-upの範囲`節はmoduleの配線を`#2`に含めていない。満たすのはmoduleを配線した時点であり、`#13`／`#15`／`#16`側で扱う）
- [x] Resetとbacklight lineが安全な状態で起動する（LCD-RST/LCD-CSへの外部pull-up実装が前提。**2026-08-25に値と本数を選定した**（`LCD-RST`／`LCD-CS`とも10 kΩ×1本。導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節）。**2026-08-27にブレッドボード上へ実装した。**2026-08-29の通電実測（`LCD-CS`＝3.30 V、`LCD-RST`＝3.30 V）は`EN`保持なしであり、pull-upの効果とfirmwareのHigh駆動を区別できなかった。**2026-09-07に`EN`を押し続けてESP32をresetに保持した状態で再測定し、`LCD-CS`(GPIO22)＝3.31 V、`LCD-RST`(GPIO16)＝3.31 Vを得た（`EN`押下の有無で値は変化しない。`fc42332`の`SERVO-PWM`測定と同じ方法。記録は`experiment-log.md`の`EXP-011`）。**firmwareが動作しない状態でHighを維持しており、pull-upが正常であることを確認した。**`LCD-BL`も2026-09-07にpull-down（`4.7 kΩ`×1本、GPIO4）を実装し、同じ`EN`保持測定でGPIO4＝0.00 Vを得た（期待どおりLow）。**この項目が要求するのはbacklight lineが安全な状態（Low）にあることであり、それは実測（GPIO4＝0.00 V）と一次資料（MSP2807公式User Manual、`LED` pinは「high level lighting」＝active-high。正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`Backlight回路／電流／polarity`行）の組み合わせで満たしている。すべての構成要素（`LCD-CS`／`LCD-RST`／`LCD-BL`）が達成した。**あわせて次の2点はcaveatとして残す（達成の取り消しではない）。**(1) `EN`保持のGPIO4＝0.00 Vは、外部pull-downの存在を内部weak pull-down（`wpd`）から切り分けない**（`LCD-CS`／`LCD-RST`は内部pullが無いため区別できるが、`LCD-BL`は異なる。PM指摘、2026-09-07）。**(2) 実機LCDを接続した状態での物理的な消灯そのものは未確認である**（`EXP-011`はLCD／touch panelを接続せずに行った。確認は`#13`（LCD bring-up）で行う）。[`HW-TBD-032`](tbd-register.md)を参照）
- [x] Servo PWMがdisabledまたは承認済みの安全状態で起動する（GPIO27はreset時high-Zであり、外部pull-downを**必須**とした。**reset時状態が`oe=0, ie=0`＝内部pull無しであることをESP32 Datasheet v5.3の`IO_MUX`で2026-08-25に確認した。**同日に**4.7 kΩ×1本を選定し、2026-08-26に一般値側と決まって確定した**（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節）。**2026-08-27にブレッドボード上へ実装した。****2026-08-29に通電しての実測を行い、GPIO27＝0 Vを確認した（期待どおり、pull-down正常。USB抜き差しによる再現性も確認した）。****この実測はservo・LCD本体を接続していない状態（Revision 23参照）で行った。**
  **2026-09-06に机上reviewを行い、下記の論はservoの接続有無に依存しないことを確認したうえで、この項目を達成と判定した。**`HW-TBD-026`（SG90の`logic閾値`が一次資料に無い）は、この項目を止めない。**理由**: RC servoのPWM制御信号は、周期的な立ち上がり・立ち下がりedgeとpulse幅で位置commandを符号化する方式であり、**受信側は一定のDC levelからpulse幅を測れない。**GPIO27の起動時state（内部pullの無い真のhigh-Zを、外部4.7 kΩ pull-downで確定させた定常0 V）は、SG90側の`VIH`／`VIL`が具体的にいくつであっても——0 Vはどのような正の閾値よりも低く、high側に誤読される余地が無い——edgeを持たないDC levelとしてしか受信されず、**有効なPWM pulseとして復号されない。**したがって「起動時に安全な（動きえない）状態で止まっているか」は`HW-TBD-026`の数値によらず判定できる。**この論はservoの接続有無にも依存しない。**2026-08-29の実測はservo・LCD本体を接続していない状態（Revision 23）で行ったが、edgeが無いという性質は信号線の電圧レベルそのものに依るのではなく波形の形（DC定常か周期パルスか）に依るため、servoを接続してGPIO27側から見た電位が仮に変化したとしても——たとえばservo内部のpull-up等でlevelが持ち上がったとしても——**edgeを持たないDC levelである限り復号されないことに変わりはない。**したがって接続後の再測定を待たずにこの結論を採用してよい。**`HW-TBD-026`が実際に要るのは別の判定である**（firmware初期化後、3.3 V driveのactiveなPWM pulseがSG90側に正しく`HIGH`として認識されるかという実運用側の判定であり、この項目＝起動時のdisabled stateとは別の問いである）。
  **この判定が閉じるのはこの項目（gpio-assignment.md側のGPIO27起動時state review）だけである。**servoの出力を実際に有効化してよいかの判断（[servo-safety-limits.md](servo-safety-limits.md#サーボ出力を有効化してよい条件)のgate）は、`HW-TBD-007`／`009`／`010`／`011`／`019`／`020`／`026`／`035`ほか多数のTBDとProtocol側TBDが未解決のままであり、依然`Blocked`である。**`HW-TBD-019`（起動時とdriver故障時の動作の全6サブ項目）もこの1点の解決だけでは閉じない**（「PWM driver初期化前のGPIO state」というサブ項目1つが埋まっただけであり、開始mode・enableまでのdelay・Pi未接続時の動作・reset／panic後の動作・driver故障検知時の動作の5項目は未解決のまま`tbd-register.md`で追跡する）。`tbd-register.md` HW-TBD-019と連動、当該サブ項目のみ解決）
- [x] Touch CS lineが安全な状態（inactive＝High）で起動する（`TOUCH-CS`(GPIO21)への外部pull-up実装が前提。**2026-08-25に値と本数を選定した**（10 kΩ×1本、`LCD-CS`と同じ理由。導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節）。**2026-08-27にブレッドボード上へ実装した。**2026-08-29の通電実測（GPIO21＝3.30 V）は`EN`保持なしであり、pull-upの効果とfirmwareのHigh駆動を区別できなかった（**2026-08-29に一度「満たした」と判断したが2026-08-31に取り消した経緯がある**）。**2026-09-07に`EN`を押し続けてESP32をresetに保持した状態で再測定し、GPIO21＝3.31 Vを得た（`EN`押下の有無で値は変化しない。`fc42332`の`SERVO-PWM`測定と同じ方法。記録は`experiment-log.md`の`EXP-011`）。**firmwareが動作しない状態でHighを維持しており、pull-upが正常であることを確認した。この項目は達成した）

## 実機check（電源off）の確認方法

**[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)本文は実機checkの範囲を
「電源offでの導通とpin header対応だけを確認する。Actuatorは動作させない」と定めている。**
競合checklistの2項目（MSP2807のlogic IO、外部moduleがESP32 pinをdriveしないこと）は、
この範囲でどう確認するかを定義していなかった。2026-09-06に定義する。

### ESP32の電源投入前に外部moduleがESP32 pinをdriveしない

**結論: この項目は非通電の導通checkだけで判定できる。module側の出力段の性質は効かない。**

**前提。**この設計では、周辺module3点（`DISP-01`／`ACCEL-01`／`ENV-01`）はいずれも
ESP32 boardの`3V3` pinから給電される（[power-budget.md](power-budget.md#測定計画)の
測定点定義「ESP32 boardの`3V3` pinと周辺module3点のrailの間」）。**独立した電源を持つ
moduleは無い。**したがってESP32が電源offのとき（USB非接続。`3V3` pinはESP32board上の
regulatorが作るrailであり、ESP32自体が無給電ならこのrailも無給電である）、周辺module側も
無給電である。

**判定に効かない理由。**無給電のIC出力段は、内部topology（`Rp`直列抵抗の値、
`MSP2807`の`U1`経由かVCC直結か等）によらず、能動的にlogic levelを駆動できない。
駆動には電源が要るためである。無給電状態で存在しうるのは、内部ESD／clamp diode経由の
微小な漏れ電流だけであり、これは「drive」ではない。**したがって`HW-TBD-004`の直列抵抗
未追跡やMSP2807の`U1`経路未追跡は、この項目の判定に影響しない。**（比較として、通電後の
微小漏れ電流はすでに`起動時状態を確定させる外部pull`節のpull-up `Rmax`計算で余裕を
確認済みであり、別の話である。）

**したがって、確認すべきは「module側が本当に独立電源を持たないこと」と「配線が
意図した1対1対応どおりであること」の2点である。**

**この2点で、導通checkから言えることの強さが違う。**「独立電源を持たないこと」は
導通の有無で判定できる。**「1対1対応どおりであること」は、導通の有無だけでは判定できない。**
導通が示すのは導電経路の存在であり、**直結（1:1）そのものではない。**
このboard上には直結でない導電経路が実在する。`ACCEL-SDA`／`ACCEL-SCL`の2本については、
**module側にADXL345の`01C`（10 kΩ）が各lineへ2本ずつ付いて並列合成5.00 kΩ**であり
（[I2C busの実効pull-up](#i2c-busの実効pull-up)節。**ここへ再掲しない**）、
**ESP32側にも外部`4.7 kΩ`のpull-upが2026-09-07に実装済み**である（`信号inventory`の該当行）。
**したがってjumperが1本欠けていても、module pin→5.00 kΩ→`3V3`→4.7 kΩ→GPIOの経路で
約9.7 kΩの導通が読める。**同じ`3V3`へpull-upされたpin同士（`SDA`と`SCL`など）も同様に
9 kΩ台で導通する。**いずれも配線の存在を意味しない。**
したがって**判定は抵抗値で行い、閾値を下に置く**（下表）。
**それでも「意図したGPIOへ直結している」ことの証明にはならない**（同じ抵抗値を示す別経路を
排除できない）。**直結の同一性そのものを要求する場合はパターン追跡が要る。**
この節が導通checkで判定するのは「独立電源が無いこと」と「意図しない導通が無いこと」までである。

| 測定項目 | 測定点 | 計器・レンジ | 判定基準 |
|---|---|---|---|
| module電源pinの独立性 | 各moduleの電源pin（`DISP-01`の`VCC`、`ACCEL-01`の`Vs`／`VDD`、`ENV-01`の`VDD`）⇔ ESP32の`3V3` pin | DT830B、**抵抗レンジ（2000 Ωレンジ、`HW-TBD-002`の先例に合わせる）** | **2000 Ωレンジでフルスケール未満（＝2 kΩ未満）の値として読めること。****5V rail・他の独立電源へは2 kΩ未満の導通が無いこと** |
| pin header対応 | `信号inventory`の各signalについて、moduleの header pin ⇔ 対応するESP32 GPIO pin | 同上 | **2000 Ωレンジでフルスケール未満（＝2 kΩ未満）の値として読めること。****他のGPIO・5V rail・GND・`3V3`へ2 kΩ未満の導通が無いこと**（`3V3`直結の誤配線は、pull-up経由の経路と同様2 kΩ未満で読めてしまうため、除外先へ加える） |

**閾値を2 kΩ未満に置く理由。**このboard上のpassive経路の最小値は、`ACCEL-SDA`／`ACCEL-SCL`が
結線済みのときの並列合成**約2.42 kΩ**である（module側5.00 kΩとESP32側4.7 kΩの並列。
[I2C busの実効pull-up](#i2c-busの実効pull-up)節）。pull-upを2つ直列に経由する経路は約9.7 kΩである。
**2000 Ωレンジではいずれもフルスケールを超えて読めない。**
一方、直結のjumperはcontact抵抗を含めても数十Ωの桁である。**桁が3つ違うため、
このレンジで読めるか否かがpassive経路と直結を分ける。**
**buzzer（導通音）単独では判定しない。**鳴る閾値が計器の仕様として定まっておらず、
上のpassive経路を除外できるとは言えないためである。

**判定基準を両方満たせば、この項目は達成とする。**電源pinの独立性が確認できれば
「moduleが独立電源でESP32 pinを駆動する経路自体が存在しない」ことが示され、
pin header対応が確認できれば「配線ミスによる意図しない接続」も排除される。
**この項目が要求するのはこの2点であり、直結（1:1）の同一性の証明ではない。**
**この2点は、`ACCEL-01`についてはすでに`HW-TBD-004`で個別に着手されている**
（`Vs`／`VDD`間の導通は2026-09-05に確認済み）。`DISP-01`／`ENV-01`は未実施である。

**この項目が見ていないもの。**module側のpull-upがESP32の起動時levelへ与える影響は、
この項目の対象外である。`ACCEL-01`は`01C`（10 kΩ）を4個搭載しており（`信号inventory`の
`ACCEL-SDA`行）、ESP32が電源offの間もmodule側にこの抵抗経由の経路が物理的に存在する。
**ただし、この経路がESP32の起動時levelに与える影響（内部weak pullとの兼ね合い等）は、
`起動時状態を確定させる外部pull`節とbootstrap pinのreview（`ESP32の使用制限pin`節）が
扱う範囲であり、この項目（moduleが能動的にdriveしないこと）が扱う範囲ではない。**この項目が
確認するのは「無給電のmoduleが能動的にlogic levelを出力しないこと」だけであり、
「受動的な抵抗経路が起動時levelへ与える影響が無いこと」までは確認しない。混同しないこと。

### MSP2807のlogic IOが3.3Vで動作することを現物で確認した

**結論: メーカー資料の記載（Logic IO port voltage: 3.3 V TTL）だけでは足りないと判定する。
非通電で追加確認できる項目を1つ定義するが、それでも解決しない可能性が残る。**

**なぜ記載だけでは足りないか。**この設計は`DISP-01`のVCCをESP32の`3V3` rail
（3.234–3.366 V）から給電し、5 Vを給電しない。**これは「5V給電時の出力levelが不明」
という既知のriskを避けるためであり、記載どおりの動作を保証する行為ではない。**
`DISP-01`のboard上には`U1`（`UMW XC6206P332MR`、3.3 V固定LDO）があり、
[power-budget.md](power-budget.md#33-v-rail下限の設計判断2026-09-06)が2026-09-06に確認したとおり、
**このrail電圧（3.234–3.366 V）では`U1`は常時dropout領域にあり、出力は入力からVdrop
（typ 160 mV／max 240 mV）だけ下がった値になる（regulatorとして完全に3.3 Vへ収束しない）。**
**controller IC（ILI9341・XPT2046）が`U1`の出力（dropoutにより約3.0–3.2 V）から
給電されているのか、`VCC`（rail電圧そのもの、3.234–3.366 V）から直接給電されているのかは、
現時点でどちらの一次資料にも記録が無い。**したがって、メーカー資料の「3.3 V TTL」という
記載は、**このboardの実際のlogic供給nodeの電圧を保証しない。**

**非通電で追加確認できること。**`U1`の出力側の脚は`HW-TBD-024`の測定（2026-09-05）で
すでに識別済みである。**この脚とcontroller ICの電源pinの間の導通を追跡すれば、
logic供給nodeが`U1`出力側か`VCC`側かを非通電で判別できる可能性がある。**

| 測定項目 | 測定点 | 計器・レンジ | 判定基準 |
|---|---|---|---|
| logic供給nodeの特定 | `U1`の出力側の脚 ⇔ controller IC（ILI9341／XPT2046）の電源pin | DT830B、導通（buzzer）または抵抗レンジ | 導通していれば`U1`出力側から給電（dropoutにより約3.0–3.2 V）。非導通なら`VCC`直結の可能性を検討する |

**ただしこの追跡が成功する保証は無い。**`backlight`回路のLED給電経路特定
（`R5`・`Q1`を用いた同種の追跡）は、**部品が小さくプローブを確実に当てられないため
非通電の導通測定では決められないとすでに結論している**（[sensor-datasheet-notes.md](sensor-datasheet-notes.md)
の`Backlight回路／電流／polarity`行）。controller ICのpinも同程度に微小である可能性が高く、
**同じ理由で追跡できない場合、この項目は`TBD`のまま残る。**

**計算で閉じられないかを試した。**ILI9341とXPT2046のdatasheetがVOHを与えているかを
一次資料で確認した。**両方とも与えている。**

- **ILI9341 Datasheet V1.11 §18.2.1（p.236）**: `VOH` Min `0.8×VDDI`、条件`IOL=-1.0mA`
  （2026-09-06にPDFを取得し直接確認した。この文書がこれまで引用してきた§18.2.1の抜粋
  （`VIH`／`VIL`／`IIH`／`IIL`／`ILEA`）は`VOH`を含んでいなかったため、新たに読み取った）
- **XPT2046 Datasheet（2007.5）`DIGITAL INPUT/OUTPUT`**: `VOH` Min `IOVDD×0.8`、条件`IOH=-250µA`
  （同様に新たに読み取った）

**ESP32側の`VIH`（受信側のしきい値）と突き合わせる。**ESP32の`VIH` ≥ 0.75×VDDであり
（`起動時状態を確定させる外部pull`節の一次資料）、rail上限3.366 Vのとき`VIH`min = **2.5245 V**、
rail下限3.234 Vのとき`VIH`min = **2.4255 V**である（ESP32とDISP-01は同一rail上にあるため、
実際にはこの2値の間でrail電圧に応じて連動して動く）。

**2通りのlogic供給nodeで場合分けする。**

| logic供給node | `VDDI`（module側供給電圧） | `VOH`min（`0.8×VDDI`） | ESP32`VIH`minとの余裕 |
|---|---|---|---|
| `VCC`直結（rail電圧そのまま） | 3.234–3.366 V | 2.587–2.693 V | **約160–170 mVの余裕がある**（railの両端で確認。ESP32の`VIH`もrailに連動するため） |
| `U1`出力経由（dropout、約2.99 V） | 約2.99 V（`power-budget.md`導出値、実測値ではない） | **約2.392 V** | **不足する**（ESP32`VIH`min 2.4255–2.5245 Vに対し約34–133 mV不足。ESP32が`VOH`をLowと誤読しうる） |

**したがって、計算だけではこの項目を閉じられない。**`VCC`直結なら余裕があるが、`U1`出力経由なら
datasheetの保証値どうしを突き合わせただけで不足が出る。**答えはlogic供給nodeがどちらであるかに
懸かっており、それこそが上表の非通電追跡（`U1`出力側の脚とcontroller IC電源pinの導通）が
判定しようとしているものである。**この追跡が成功すれば、上の表のどちらの行を採るかが決まり、
`VCC`直結ならこの項目は計算で達成にできる。**追跡が失敗する場合（部品サイズの理由で）、
この項目は`TBD`のまま残り、通電を伴う実測（`U1`出力またはcontroller IC`VDDI`pinの電圧測定）が
必要になる。**なお、ESP32からmoduleへの向き（`SCLK`／`MOSI`／`CS`／`DC`／`RST`）は、
`VDDI`がどちらの値でもESP32の`VOH`（0.8×VDD以上）がmodule側`VIH`（0.7×`VDDI`以下）を
十分に上回るため問題にならない（`起動時状態を確定させる外部pull`節の`Rmax`計算とは別に、
この方向の余裕はどちらのnode想定でも成立する）。**余裕が不足しうるのはmoduleからESP32への
向き（touch controllerの`DOUT`／`PENIRQ`等）だけである。**

**この場合に要ること。**非通電の追跡で解決しない場合、残る手段は通電を伴う実測
（controller ICのVDD pinの電圧を実際に測る）であり、これは`#2`本文の範囲（非通電）を超える。
**通電を伴う実際のlogic動作確認（SPI通信が実際に成立するか）は、そもそもこの項目の範囲外であり、
LCD bring-up（[#13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)）で行う**
（[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`Data／command動作`行にも同じ切り分けが
すでに書かれている）。

## 初回bring-upの範囲

**[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)の受け入れ条件7件目
「最初のbring-up範囲について未解決GPIOがない」は、範囲の定義を要する。**
2026-09-06に、初回bring-upの範囲を次のとおり定義した（設計判断）。

**含める。**ESP32単体を、PCのUSB経由（`power-budget.md`の段階B-1）で給電し、
`信号inventory`に列挙したすべての信号へGPIOを割り当て、起動時stateとpull方向を確定させること。

**含めない。**

- **`#247`が止めているPi→ESP32のUSB OTG給電（案A、段階C）。**GPIO割り当てとは無関係な
  電源topologyの問題であり、servoのPWM制御用信号にもLCD／sensor用信号にも影響しない。
- **servoの実actuator動作（torque出力）。**これは`servo-safety-limits.md`の
  `サーボ出力を有効化してよい条件`gateが扱う範囲であり、GPIO割り当ての完了条件ではない。

**この定義の下で判定する。**`信号inventory`のすべての行（LCD、touch、accelerometer、
environment sensor、servo、ADC測定、UART）にGPIO番号が入っており、GPIO番号が空欄または
`TBD`の信号は無い。**したがってこの範囲では未解決GPIOが無い。**未確定が残るのは個々の属性
（`LCD-BL`の実装、I2C bus容量`Cb`、`ACCEL`／`ENV`のaddress配線）であり、これらはGPIO割り当て
そのものではなく、別のTBD（`HW-TBD-004`／`005`／`024`／`026`／`027`／`032`）が個別に追跡する。

## Firmwareとの同期

割り当てを承認した後、次を行う。

1. 配線revisionへ識別子を付ける。
2. 一致するfirmware board configuration IDを追加する。
3. `firmware/esp32`内でpinを一元管理する。
4. 起動時にboard configuration IDを出力する。
5. 可能であれば、重複割り当てに対する起動checkを追加する。
6. GPIOを変更するPull Requestでは、この文書の更新を必須にする。

## Revision履歴

| 日付 | Revision | 変更 | 根拠 |
|---|---|---|---|
| 2026-09-08 | 33 | [#367](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/367)。[PR #366](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/366)（`develop`→`main`昇格）の`full review`でCodeRabbitが出した指摘のうち、この文書に関わる2件を直した。**過去のRevision行は書き換えない。**(1) **`信号inventory`の`LCD-BL`行が古かった。**「実装と起動時の状態確認は未実施」と書いていたが、`受け入れchecklist`の2項目（Revision 32で更新）が2026-09-07の実装と`EN`保持測定（GPIO4＝0.00 V）を記録している。実装済み・reset中の安全状態を確認済み・**ただしGPIO4は内部weak pull-downが有効なため電圧測定だけでは外部pull-downの存在を証明しない**の3点へ揃えた。**外部pull-downの存在そのものの確認は残る。**(2) **導通checkの判定基準に抵抗値の上限を入れた。**それまで判定基準は「導通していること」だけで、計器は「buzzer**または**2000 Ωレンジ」であり、どちらを使うかで判定が変わった。このboard上には直結でない導電経路が実在する（`ACCEL-SDA`／`ACCEL-SCL`は、module側のADXL345`01C`並列5.00 kΩとESP32側の外部4.7 kΩの両方でpull-upされているため、jumperが欠けていても約9.7 kΩの導通が読める）。判定を**2000 Ωレンジでフルスケール未満（2 kΩ未満）**と定め、**buzzer単独では判定しない**ことにした（鳴る閾値が計器仕様として定まらない）。あわせて、**導通checkで言えるのは「独立電源が無いこと」と「意図しない導通が無いこと」までであり、直結（1:1）の同一性の証明ではない**ことを明記した（同一性を要求する場合はパターン追跡が要る） | [PR #366](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/366)のCodeRabbit指摘、および引用先を開いたPMの検証 |
| 2026-07-27 | 0 | 信号inventoryを作成。実GPIO割り当てはすべて引き続きTBD | — |
| 2026-08-05 | 1 | Board識別情報を確定（ESP-WROOM-32D開発ボード、秋月電子 M-13628）。Espressif公式ESP32-DevKitC V4のpin制約（flash pin6-11、strapping pin0/2/5/12/15、input-only pin34/35/36/39、WROOM専用pin16/17）を反映し、全信号にGPIOを割り当てた。LCD/Touch CSとLCD RSTには起動時safe state確保のため外部pull-up追加を推奨。競合checkのうち実機確認が必要な項目（電源off導通、pull-up実効抵抗、servo PWM起動時状態）は未完了のまま残した | [Espressif ESP32-DevKitC V4 pinout](https://docs.espressif.com/projects/esp-idf/en/v5.1/esp32/hw-reference/esp32/get-started-devkitc.html)、`hardware-bom.md` |
| 2026-08-05 | 2 | 自己レビューで検出: 外部pull-upの接続先電圧を明記していなかったため、`電圧domain`節を追加し、すべてのpull-upを3.3Vへ接続する（5Vへ接続しない）ことを規定。MSP2807はVCC 3.3–5V対応だがlogic IOが3.3V TTLであり、5V給電時の出力levelがメーカー資料でも不明なため、現物確認項目を競合checkへ追加。存在しないRef ID「LCD-01」を「DISP-01」に訂正 | [LCD Wiki MSP2807](http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807)の「Logic IO port voltage: 3.3V(TTL)」記載、自己レビュー |
| 2026-08-05 | 3 | 自己レビューで検出: pull-up電圧の競合check項目を`[x]`（完了）としていたが、実配線が存在しないため検証不能であり`[ ]`へ訂正。文書冒頭の状態にMSP2807のlogic IO level確認を追加 | 自己レビュー |
| 2026-08-05 | 4 | レビュー指摘3件を反映。(a) `power-budget.md`のADC測定計画に対応するADC pinが未予約だったため、`ADC-SHUNT`(GPIO32)／`ADC-5V`(GPIO33)／`ADC-3V3`(GPIO36)をADC1で予約し、分圧比1/2と減衰設定を明記。ADC2をWi-Fi併用下で使わない旨も記載。(b) GPIO4／GPIO27のboot stateを「floating（Low相当）」と記載していたが、Lowにdriveされる保証はないため「不定」へ訂正し、`SERVO-PWM`の外部pull-downを推奨から**必須**へ格上げ。(c) Pi linkがUSB serialとGPIO UARTのどちらか曖昧だったため、Protocolの`物理／論理link` を `USB serial` とする決定に合わせUSB serialへ統一し、GPIO1／GPIO3をboard上ブリッジの予約pinと明記。USB OTG変換cableが未購入である旨も記載 | [PR #55レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55)、[Protocol](../protocol/esp32-pi-protocol.md)の物理link決定、ESP32のreset時GPIO state |
| 2026-08-05 | 5 | 自己レビューで検出: ADC行の分圧抵抗の参照先が`hardware-bom.md` PROTO-01のままだったが、測定用部品は同fileに新設した`MEAS-01`へ移したため参照を訂正。ADC行の`Device`列を他行と同じRef ID表記（`MEAS-01`）へ揃え、shunt測定の低電流側精度限界への参照を追加 | 自己レビュー、[hardware-bom.md](hardware-bom.md) MEAS-01 |
| 2026-08-09 | 6 | [PR #64](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)のレビューで、ingress低側shuntをESP32 ADCで測る案が**電気的に成立しない**と判明した。star pointを基準にするとadapter return側は`I × R`だけ負の電位になり、ESP32のADCでは測れない（pin破損のriskもある）。一度`ADC-INGRESS`をGPIO39へ予約したが、`power-budget.md`側でingressの判定量を定常値（connector定格は熱の制限のため）へ改めた結果、この測定点自体が不要になったため予約を取り消した。peakによる電圧降下は既存の`ADC-5V`／`ADC-3V3`で捉える | [PR #64レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/64)、[power-budget.md](power-budget.md)の`ingressの電流制限` |
| 2026-08-09 | 7 | [#65](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/65)の発注前走査で、**この文書が要求する部品が`hardware-bom.md`の購入待ちリストに載っていなかった**ことが判明した。`ADC-5V`／`ADC-3V3`の分圧用10 kΩ（計4本）と、`SERVO-PWM`の外部pull-down（`RES-PULL-01`）が該当する。前者は本文が「購入する」と書いているだけ、後者はBOMに行すら無い状態だった。両行から購入待ちリストへ辿れるようにした | [#65](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/65)、[hardware-bom.md](hardware-bom.md) |
| 2026-08-10 | 8 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)。**Board識別情報の照合先を、実在するEspressif公式資料へ訂正した。**従来は「秋月商品ページ添付データシート」を照合先としていたが、**秋月の添付はESP-WROOM-32Dモジュールとチップのdatasheetだけで、boardのpin配列表も回路図も含まない**ことが判明した（照合先が存在しなかった）。[公式回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)と公式guideのpin description表を照合先とし、両者が一致することを確認した。あわせて根拠の無い断定「秋月オリジナル基板のため」「秋月独自基板のため」を削除した（詳細は[hardware-bom.md](hardware-bom.md) Revision 29）。**現物pin表記との対応確認は引き続き必要である**（理由が「独自基板だから」から「文書だけでは実装を保証できないから」へ変わった）。表記ゆれ`秋月 M-13628`を`秋月電子 M-13628`へ揃えた（2箇所） | [ESP32-DevKitC V4公式回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)、[Espressif公式guide](https://docs.espressif.com/projects/esp-idf/en/v5.1/esp32/hw-reference/esp32/get-started-devkitc.html)、[秋月商品ページ](https://akizukidenshi.com/catalog/g/g113628/) |
| 2026-08-12 | 9 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)。ADXL345のメーカー公式datasheet（Rev. G）を入手して照合したところ、**`ACCEL-IRQ`行のpull欄が事実に反していた。**「ADXL345のINT1/INT2はpush-pull／open-drainを設定可能」と書いていたが、Rev. G page 19は`Both interrupt pins are push-pull, low impedance pins`と定めており、**設定で切り替えられない。**この誤りは外部pull-upの要否の判断を誤らせるため、**ICはpush-pull固定である**と訂正した。あわせて「既定active-highの想定」は正しかったが**ICについては想定ではなく確定である**ため書き改めた。`DATA_FORMAT` register（`0x31`）の`INT_INVERT` bitで選び、同registerのreset値が`00000000`であることをRev. G Table 19 page 23とpage 27で確認した。**ただし`外部pull要確認`は残した。**[PR #112](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/112)のreviewで指摘を受けて自己点検したところ、**当初の改訂案は「外部pullは不要である」とmodule levelの結論まで書いており、ICの定格からmodule boardの配線条件を導いていた。**M-06724のboard上でINT pinがheaderへ直結しているかを示す資料は無く、`HW-TBD-004`のままである。**ICの事実とmodule levelの未確認を書き分けた。****GPIO割り当て（`ACCEL-IRQ`＝GPIO35）もEdge想定も変更していない** | [ADXL345 Data Sheet](https://www.analog.com/media/en/technical-documentation/data-sheets/adxl345.pdf) Rev. G（2026-08-12取得）、[sensor-datasheet-notes.md](sensor-datasheet-notes.md) Revision 5 |
| 2026-08-12 | 10 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)。`ADC-5V`行が分圧用10 kΩ抵抗を「計4本が必要であり、**未購入である**」とし、`hardware-bom.md`の購入待ちリストを参照していたが、**抵抗は2026-08-08に着荷済みであった**（同文書 Revision 37。発注漏れではなく記録漏れ）。**参照先の購入待ちリストの行も同時に削除されたため、この記述は宛先を失っていた。**「入手済み。残るのは実装と検証」へ改めた。**分圧比1/2もpin割当ても変えていない** | 購入履歴（2026-08-08着荷分）、[hardware-bom.md](hardware-bom.md) Revision 37 |
| 2026-08-12 | 11 | [PR #116](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/116)のreview指摘。`SERVO-PWM`行が`RES-PULL-01`を「**未購入**・抵抗値未選定」としていたが、**10 kΩと4.7 kΩが各1袋入手済みである**（[hardware-bom.md](hardware-bom.md) Revision 37）。二重発注を招くため「一部の抵抗値は入手済み。ただし必要な本数と抵抗値が未選定であり、手元の2種で足りるとは限らない」へ改めた。**外部pull-downを必須とする規則も`HW-TBD-027`のgateも変えていない** | [PR #116のreview](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/116)、[hardware-bom.md](hardware-bom.md) Revision 37 |
| 2026-08-15 | 12 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)。**現物写真の読み取り結果を反映した。**(a) **`HW-TBD-001`のpin照合が完了し、一致した。**38pinヘッダ両側のsilkが公式`J2`／`J3`と19pin×2列すべてで一致した（GNDの位置を含む）。`Board識別情報`の`公式回路図revision`欄へ読み取った並びを記録した。(b) 基板裏面silkscreenの大文字小文字を**`ESP32_DevKitc_V4`**へ訂正した（旧記載`ESP32_DevkitC_V4`。現物と公式回路図のtitle blockが一致する）。(c) Touch controllerを**`XPT2046`と確定**し、`LCD-MISO`と`TOUCH-CS`の「想定」「現物確認待ち」を確定表現へ改めた。(d) `ACCEL-SDA`と競合checklistへ、**ADXL345が`01C`＝10 kΩのpull-upを4個搭載している**ことを記録した。**ただしどのpinへ付くかはパターンを追っておらず、BME280側は半田の有無が光学判別できないため、実効抵抗は両方が確定するまで計算しない** | 現物写真（斜光＋接写）。詳細は[tbd-register.md](tbd-register.md)の`HW-TBD-001`／`003`／`004`／`005` |
| 2026-08-15 | 13 | [PR #122](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/122)のレビュー指摘を反映。文書冒頭の状態行が`touch controller型番`を現物確認待ちに挙げたままだったため、**`XPT2046`確定と`HW-TBD-003`のcloseを反映した** | [PR #122レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/122) |
| 2026-08-22 | 14 | **`I2C sensor bus`行へ、BME280側のjumperの実測結果を反映した。**`J1`／`J2`はどちらも開放であり、**module搭載の4.7 kΩプルアップはbusへ繋がっていない**（正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`現物の実装状態を実測で確定させた（2026-08-22）`。**ここへ再掲しない**）。**したがって実効pull-upの計算にBME280側の4.7 kΩを入れない。**同じbusのADXL345モジュールは`01C`（10 kΩ）を搭載しており、計算はそちら側だけを数える形になる。**`J1`／`J2`をはんだ付けするかは計算の結果で決める。まだ決めていない。****あわせて`J3`が開放であるため、I2Cで使うには`J3`のはんだ付けが要る** | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1) |
| 2026-08-22 | 15 | [PR #173](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/173)の自動reviewの指摘を反映した。**BME280の実測結果を記録したのに、同じ文書に古い記述が残っていた。**`ENV-SDA`行の`選択jumperの現物確認が必要`、`I2C sensor bus`行の`両moduleのinterface選択jumperの現物確認`、受け入れchecklistの`半田の有無が光学的に判別できず未確定`である。**いずれも実測結果へ更新した。**未解決として残す対象を**ADXL345側のpin接続の確認と実効pull-up計算に限定**し、**BME280の`J3`は実装作業、`J1`／`J2`は計算後の判断**として記録した | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1) |
| 2026-08-22 | 16 | [PR #174](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/174)の自動reviewの指摘を反映した。**USB OTG cableを`未購入`としていた記述が2箇所残っていた**（`追加部品`行と`USB serial（Pi link）`行）。**`CABLE-PI-LINK-01`は2026-08-22に手持ちで充当と確定し購入待ちリストから外している**（正は[hardware-bom.md](hardware-bom.md)の`CABLE-PI-LINK-01`）。両方を更新した | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) |
| 2026-08-25 | 17 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)。**`I2C busの実効pull-up`節と`I2C addressの選択`節を新設した。**`I2C sensor bus`行と受け入れchecklistが「実効pull-upの計算はまだできない」と書きながら、**式も規定値も前提もどこにも無かった。**そのため何が足りないのかを行の記述から読み取れなかった。一次資料（**I2C-bus specification and user manual UM10204 Rev. 7.0、NXP、2021-10-01**）の§7.1から`Rp(max) = tr / (0.8473 × Cb)`と`Rp(min) = (VDD(max) - VOL(max)) / IOL`、およびTable 10／Table 11の規定値（`tr` max、`Cb` max、`IOL`）を取り、**VDD = 3.3 Vでの`Rp(min)`（約967 Ω）と、`Rp`候補ごとに許される`Cb`の上限を算出した。****値そのものは確定させていない。**確定できない入力を3つ明示した（ADXL345側の`01C`がどのpinへ付くか、bus容量`Cb`、採るmode）。**あわせて、Fast-modeで`Cb`が規定上限の400 pFに達すると`Rp(max)`が`Rp(min)`を下回り、3.3 Vの受動pull-upでは成立しないことを示した。**addressについては、ADXL345の候補（`0x1D`／`0x53`）とBME280の候補（`0x76`／`0x77`）が**全4通りで衝突しない**ことを確認し、**選択が衝突回避では決まらない**ことを判断材料として記録した。**決定は行っていない。**`J1`／`J2`のはんだ付けもaddressの配線も未実施である | [UM10204 Rev. 7.0 §7.1、Table 10、Table 11](https://web.archive.org/web/2023/https://www.nxp.com/docs/en/user-guide/UM10204.pdf)（**NXPの直リンクは404を返すため同一pathのarchive snapshotを参照先にした。**2026-08-25確認）、[sensor-datasheet-notes.md](sensor-datasheet-notes.md)、[tbd-register.md](tbd-register.md) `HW-TBD-004`／`HW-TBD-005` |
| 2026-08-25 | 18 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**`起動時状態を確定させる外部pull`節を新設し、`信号inventory`の`Pull`列へ選定した値と本数を入れた。****確定した内訳（4本）**: `SERVO-PWM`のpull-downが**4.7 kΩ×1本**、`LCD-CS`／`LCD-RST`／`TOUCH-CS`のpull-upが**各10 kΩ×1本**。**`LCD-BL`の1本は未選定のまま残る。**したがって**手元の2種で全体が足りるとは、まだ確定していない。****`SERVO-PWM`の振り分け（一般値側か一次資料側か）は2026-08-26に人間が一般値側と決めた。****`LCD-BL`は未選定のまま**（極性とbacklight回路の入力条件が未確定で上限を出せない）。**`TOUCH-IRQ`は外部pullを付けない（0本）へ改めた。**根拠は一次資料である（**ESP32 Series Datasheet v5.3** の Table 5-3 と Appendix `IO_MUX` と §3.2、**ILI9341 Datasheet V1.11** の §18.2.1 と §12.1／§12.2、**XPT2046 Datasheet**（2007.5）の`DIGITAL INPUT/OUTPUT`と`PENIRQ Output`）。**一次資料で分かった重要な点が4つある。**(a) **`SERVO-PWM`（GPIO27）のreset時状態は`oe=0, ie=0`で内部pullが無く、真のhigh-Zである。**外部pull-downが必須である理由が`IO_MUX`の値として裏付いた。(b) **`LCD-CS`／`LCD-RST`／`TOUCH-CS`のpolarityは現物確認を要しない。**ILI9341が`CSX`と`RESX`をactive low、XPT2046が`CS`をactive lowと明記している。**旧記載の`要現物のpolarity確認`を削除した。**(c) **`TOUCH-IRQ`へ外部pull-upを付けると有害である。**XPT2046の`PENIRQ`は内部pull-up付きの出力（公称50 kΩ）であり、外部10 kΩを並列に足すと実効8.33 kΩになり、datasheetが`logic low 0.35×(+VCC)`を保証する条件（X+とY−間21 kΩ未満）に対してlowが**0.716×VCC**まで上がる。**touchがLowとして読めなくなる。**旧記載の`外部pull-up推奨`はcontroller未確定時のものであった。(d) **`LCD-RST`（GPIO16）は`VDD_SDIO` domainにある。**3.3 Vへのpull-upが定格内である前提は同domainが3.3 Vであることに依り、それは`MTDI`（GPIO12）のreset時の内部weak pull-downと、module内flashが動作している事実から成り立つ。**`SERVO-PWM`だけは上限を計算できない**（SG90の`logic閾値`が一次資料に無い。`HW-TBD-026`(a)）。**駆動側の下限に余裕がある範囲で未知に強い側を採り、4.7 kΩ×1本を選定した。****この変更は値と本数の選定であって実装ではない。5本とも未実装である。****この文書の状態`Blocked`は解除していない。checkboxも1つも開いていない** | [ESP32 Series Datasheet v5.3](https://www.espressif.com/sites/default/files/documentation/esp32_datasheet_en.pdf)、[ILI9341 Datasheet V1.11](https://cdn-shop.adafruit.com/datasheets/ILI9341.pdf)、[XPT2046 Datasheet](https://grobotronics.com/images/datasheets/xpt2046-datasheet.pdf)、[tbd-register.md](tbd-register.md) `HW-TBD-026`／`HW-TBD-027`／`HW-TBD-032`、[hardware-bom.md](hardware-bom.md) `RES-PULL-01` |
| 2026-08-27 | 19 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)。**`I2C busの実効pull-up`節の未確定だった3入力のうち1つを、現物写真でパターンを追って確定した。**ADXL345モジュールの`01C`（10 kΩ）4個は、**SDAへ2本、SCLへ2本**が付いている。したがって各lineの並列合成値は**5.00 kΩ**であり、`Rp`候補表の該当行が候補から確定値へ変わった。`ACCEL-SDA`行と受け入れchecklistの該当項目を合わせて更新した。**残る未確定入力は`bus容量Cb`と`採るmode`の2つのみである。**`J1`／`J2`をはんだ付けするかの判断はこの2入力が埋まるまで引き続き未着手 | 現物写真（ADXL345基板の`01C`4個とパターンの接写） |
| 2026-08-27 | 20 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**Revision 18で選定した4本のpull抵抗を、ブレッドボード上へ実装した。**`SERVO-PWM`(GPIO27)の4.7 kΩ pull-down、`LCD-CS`／`LCD-RST`／`TOUCH-CS`の各10 kΩ pull-upの計4本である。ESP32基板はブレッドボードへ直接挿さず、対象ピンからジャンパー線で引き出し、抵抗を経由して電源用の帯(`GND`または`3.3V`)へ接続する形で実装した。**通電しての起動時状態の検証は未了。**`LCD-BL`は値が未選定のままで対象外 | 現物作業（ブレッドボード配線） |
| 2026-08-27 | 21 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**Revision 20の配線について、線の色と対象の対応を記録した。**`緑`＝GPIO27(`SERVO-PWM`、抵抗はGND側)、`黄色`＝GPIO22(`LCD-CS`、抵抗は3V3側)、`紫`＝GPIO16(`LCD-RST`、抵抗は3V3側)、`茶色`＝GPIO21(`TOUCH-CS`、抵抗は3V3側)、`オレンジ`＝`GND`、`赤`＝`3V3`。**`SERVO-PWM`の抵抗がGND側(pull-down)、残り3本の抵抗が3V3側(pull-up)であることを本人に確認した** | 現物作業（配線色の確認） |
| 2026-08-28 | 22 | PR #254のCodeRabbitレビュー指摘（PM経由で受領）。**`Rp`候補表（L268-271）が、外部`4.7 kΩ` pull-up（`ACCEL-SDA`／`ACCEL-SCL`、Revision 18で選定済み）を含んでいなかった。**3行ともmodule搭載のpull-upだけを数えており、実際に選定済みの外部抵抗が候補にも実構成にも現れていなかった。**加えて3行目（`3.20 kΩ`＝`J1`／`J2`の4.7 kΩを足す構成）は、`J1`／`J2`が開放と実測済み（L249）であるため現状は成立しない。**表をmodule搭載分だけの参考値と明示し、**外部4.7 kΩを含む実構成（ADXL345側5.00 kΩ ∥ 外部4.7 kΩ ≈ 2.42 kΩ）を受け入れ計算用の値として追加した。**安全要件5項目には該当しない（`Rp(min)`約967 Ωを上回り、sink電流は`IOL`の範囲内） | PR #254 CodeRabbitレビュー |
| 2026-08-29 | 23 | work-instructions文書に基づく初回通電（人間の監視下、ESP32単体、サーボ・LCD本体は未接続）。**`SERVO-PWM`(GPIO27)＝0 V、`LCD-CS`(GPIO22)＝3.30 V、`LCD-RST`(GPIO16)＝3.30 Vを実測し、いずれも期待どおりであることを確認した（USB抜き差しによる再現性も確認）。**受け入れchecklistの該当2項目へ実測値を反映した（値は`HW-TBD-026`の閾値未確定のため項目自体はまだ満たしていない）。**あわせて、`TOUCH-CS`(GPIO21)の起動時状態がこれまでどの受け入れ項目にも`HW-TBD`にも追跡されていなかったことが判明した**（`HW-TBD-032`は`LCD-RST`／`LCD-CS`／`LCD-BL`の3信号だけを対象と明記しており、`TOUCH-CS`を含まない）。**同信号は`LCD-CS`と同じ理由で10 kΩ pull-upを実装済みであるため、その場でGPIO21＝3.30 Vを実測し（期待どおり）、新設した受け入れ項目を満たした** | 現物実測（テスターDT830B、DC電圧20Vレンジ） |
| 2026-09-06 | 24 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**自己レビューで検出: `I2C busの実効pull-up`節が、未確定入力の数を「3つ」（L209、`J1`／`J2`の判断節）と「2つ」（旧L253見出し）の両方で書いていた。**Revision 19（2026-08-27）でADXL345側のpin接続を確定させ見出しと`ACCEL-SDA`行・受け入れchecklistは更新したが、**同節内の他2箇所（値そのものが確定していないと述べる文と`J1`／`J2`の判断節）は「3つ」のまま取り残されていた。**実際に未確定なのは`bus容量Cb`と`採るmode`の2つであるため、両箇所を「2つ」へ訂正した。**あわせて、`LCD-BLを決められない理由`節が`HW-TBD-002`を未解決の引用先としていたが、`HW-TBD-002`は2026-09-06にcloseしている**（[#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)）。**同行の close記録自身が「backlight回路のLED接続経路の判定は`HW-TBD-024`の範囲であり、`HW-TBD-002`には含まれない」と明記しているため、closeはこの回路modelの未確定を解消していない。**参照を`HW-TBD-024`単独へ訂正し、`HW-TBD-002`がcloseした事実と、それでも解消していない理由を明記した。**GPIO割り当てそのものは変えていない** | 自己レビュー（file全体の走査）、[tbd-register.md](tbd-register.md) `HW-TBD-002`（2026-09-06 close記録） |
| 2026-09-06 | 25 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**文書冒頭`状態`行と競合checklistの「Servo PWMがdisabledまたは承認済みの安全状態で起動する」項目について、机上reviewを行い達成へ改めた。**従来は`HW-TBD-026`（SG90の`logic閾値`が一次資料に無い）が未確定であることを理由に、GPIO27＝0 V実測（2026-08-29）を得てもこの項目は満たしていないとしていた。**RC servoのPWM制御信号は周期的なedgeとpulse幅で位置commandを符号化する方式であり、受信側は一定のDC levelからpulse幅を測れない。**GPIO27の起動時state（外部4.7 kΩ pull-downで確定させた定常0 V）は、SG90側の`VIH`／`VIL`の具体的な値によらず——0 Vはどのような正の閾値よりも低い——edgeを持たないDC levelとしてしか受信されず、有効なPWM pulseとして復号され得ない。**したがって「起動時に動きえない状態で止まっているか」は`HW-TBD-026`の数値によらず判定できる。**`HW-TBD-026`が実際に要るのは、firmware初期化後の能動的な3.3 V drive PWMがSG90側に正しく認識されるかという別の判定である。**この判定が閉じるのはGPIO27起動時state reviewだけであり、`servo-safety-limits.md`の`サーボ出力を有効化してよい条件`gate（`HW-TBD-007`／`009`／`010`／`011`／`019`／`020`／`026`／`035`ほかProtocol側TBDを含む）は依然`Blocked`のまま変えていない。**`HW-TBD-019`（起動時とdriver故障時の動作、全6サブ項目）も「PWM driver初期化前のGPIO state」1点が埋まっただけで、他5項目は未解決のまま`tbd-register.md`で追跡する。**あわせて文書冒頭の`状態`行から、この理由（servo起動時状態の安全review待ち）を外し、residualな2件（電源off導通check、MSP2807 logic IO level現物確認）だけを残した | 自己レビュー（`servo-safety-limits.md`の`サーボ出力を有効化してよい条件`節、`tbd-register.md` `HW-TBD-019`／`026`／`027`）、RC servo PWM信号の一般的な符号化方式（周期的edgeとpulse幅） |
| 2026-09-06 | 26 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。PM検査の指摘を反映した。**(a) Revision 25のservo項目に、2026-08-29のGPIO27＝0 V実測がservo・LCD本体を未接続の状態（Revision 23）で行われたことを明記し、edgeの有無は電圧levelそのものではなく波形の形に依るため接続後も結論が変わらない旨を追記した。**(b) 文書冒頭`状態`行が「電源off導通check」「MSP2807 logic IO level現物確認」の2件しか挙げていなかったが、**競合checklistには`[ ]`が7件残っており過少申告だった。**7件を1行ずつ列挙する形へ書き直した。**(c) checklist L388（ADC分圧器）とL391（外部pull-up全体の3.3V接続）の「実配線が存在しない」が何を指すか曖昧だったため、具体化した。**L388は`MEAS-01`の分圧用10 kΩ×4本自体が未実装であることを明記。L391は`LCD-CS`／`LCD-RST`／`TOUCH-CS`の3本がRevision 20-21で実装済み・3.3V接続確認済みであり、**未検証で残るのは`ACCEL-SDA`／`ACCEL-SCL`の外部4.7 kΩ pull-upと`LCD-BL`のpull-down（値未選定）の2本だけであると内訳を分けた。**(d) L392（I2C実効pull-upの有効範囲）を`SERVO-PWM`項目と同じ形（値によらず判定できるか）で検討したが、**同じ形では閉じられないと判定した。**`Rp(min)`側は`Cb`／modeによらず常に成立するが、`Rp(max)`側は`Fast-modeかつ高Cb`の組み合わせで不成立になることが既に判明しており、servoの「どんな閾値でも復号されない」という否定形の議論とは性質が異なる。**Standard-modeに限定すれば実配線の`Cb`を測らなくても有効範囲内と言えることを条件付きの結論として記録したが、modeが未決定のため項目自体は未達のまま残した。****GPIO割り当てそのものは変えていない** | PM（deskcat-f2）検査コメント、`power-budget.md`の段階A／B-1定義 |
| 2026-09-06 | 27 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。PMの決定と質問を反映した。**(a) 初回bring-upで採るI2C modeをStandard-mode（100 kHz）と決定した（人間の判断）。**`I2C busの実効pull-up`節に`初回bring-upのmode決定`小節を新設し、根拠（実構成`Rp`約2.42 kΩはStandard-modeの規定`Cb`上限400 pFでも`Rp(max)`約2.945 kΩを下回り成立、Fast-modeは`Rp(max)`約885 Ωで不成立）、下流要件の確認結果（`#12`／`#15`／`#22`の受け入れ条件に具体的な下限bus速度の要求は無く、`ACCEL-IRQ`によるhardware割り込み設計とBME280の秒order pollingであればStandard-modeで足りる）、見直し条件（Fast-modeが要るならpull-up値の再設計から。`sensor-datasheet-notes.md`の`検証済み最大bus速度`は変更しない）を記録した。checklist「Moduleのpull-upを並列合成した実効抵抗が有効範囲内である」を`[x]`へ改めた。**`J1`／`J2`をはんだ付けするかの判断はこの決定の対象外のまま残した（`bus容量Cb`が未確定）。**(b) `LCD-BL`のpull-down値が`HW-TBD-024`待ちだとしていた記載を訂正した。**`HW-TBD-024`（MSP2807が耐えられる電流の上限）は backlightを点灯させるときの電流制限設定に要る値であり、firmware初期化前にGPIO4をLowへ保って消灯を維持するpull-downの仕事とは無関係である。**`LCD-BL`が決まらない実際の理由は極性の現物未確認（`HW-TBD-032`）1点であると判定した。`LCD-BLを決められない理由`節、`信号inventory`の`LCD-BL`行、checklist L391の該当箇所を訂正した。**値の選定自体（`4.7 kΩ`か`10 kΩ`か）は極性確定後に`SERVO-PWM`と同じ一般値の方法で決められる見込みであることも記録したが、極性の現物確認はこの作業指示書の範囲（机上）では行っていないため、値そのものは選定していない。**(c) 文書冒頭`状態`行を、(a)(b)の変更を反映して未達6件へ更新した | PM（deskcat-f2）の決定・質問、[#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)／[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)／[#22](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/22)の受け入れ条件、[tbd-register.md](tbd-register.md) `HW-TBD-024`／`HW-TBD-032` |
| 2026-09-06 | 28 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**Revision 27の判定は、それ自体が古い前提に基づいていた。**PMから`HW-TBD-032`（owner `#2`）のstale記述（後述）を指摘された過程で判明した。**`LCD-BL`の極性は2026-09-05にすでに一次資料で確定していた**（MSP2807公式User Manual、LCDWIKI、Interface Description表が`LED` pinを「high level lighting」と定める。正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`Backlight回路／電流／polarity`行、Revision 12）。**Revision 27はこの確定を見落とし、「極性の現物未確認が残る」と誤って報告した。**`gpio-assignment.md`側（この文書）が2026-09-05の確定を反映していなかったため、`tbd-register.md`・`sensor-datasheet-notes.md`を照合せずに判定した結果である。**訂正する。**極性が確定済みである以上、Revision 27で示した「極性確定後に`SERVO-PWM`と同じ一般値の方法で決められる」という条件は満たされたため、**`LCD-BL`のpull-downを`4.7 kΩ`×1本に確定した。**`選定した値と本数`表、`LCD-BLを4.7 kΩにした理由`節（旧`LCD-BLを決められない理由`。見出しも実体に合わせて改めた）、`信号inventory`の`LCD-BL`行、checklist L391／L395、文書冒頭`状態`行を更新した。**あわせて`tbd-register.md`の`HW-TBD-032`（owner `#2`）を、実装状況（`LCD-CS`／`LCD-RST`は実装済み、未実装は`LCD-BL`のみ）と極性確定の反映が遅れていた点を含めて訂正した（別途同file側のRevisionで記録）。**このRevisionは、今日指摘・修正してきた「実体が進んだのに参照側が取り残される」型の不整合を、自分自身の直前の判定でも起こしていたことの記録である | PM（deskcat-f2）指摘、[sensor-datasheet-notes.md](sensor-datasheet-notes.md) Revision 12、[tbd-register.md](tbd-register.md) `HW-TBD-032` |
| 2026-09-06 | 29 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**commit前の自己レビュー（要件照合Passとfresh-context Pass）で検出した3件を反映した。**(a) `LCD-BLを4.7 kΩにした理由`節末尾の結論文が、直前の箇条書き（極性確定・値確定）と矛盾していた。「`LCD-BL`の値と向きは`HW-TBD-032`（極性の現物確認）の後に決める」「極性の現物確認は…この作業指示書の範囲では行っていない」という、Revision 28以前の未確定を前提にした文をRevision 28後も直しておらず、同じ節の中で「確定した」と「未確定」が両方書かれていた。**値・向き・本数が確定済みであることを述べる形へ書き直した。**(b) checklist L392（I2C実効pull-upの有効範囲）の本文中に、mode決定（Revision 27で`[x]`化した同じ文の中）より後ろの文で「残る未確定入力は`bus容量Cb`と`採るmode`の2つである」という、mode決定前の文言が残っていた。**採るmodeは決定済みである旨へ訂正した。**(c) 受け入れ条件7件目「最初のbring-up範囲について未解決GPIOがない」の範囲定義を、これまでPMへの報告（chat）でのみ述べており、**正本である本文書には一度も書いていなかった。**新設した`初回bring-upの範囲`節（`競合check`の直後）に、含める範囲（ESP32単体・PC USB給電・`信号inventory`の全信号へのGPIO割り当てと起動時state確定）と含めない範囲（`#247`のPi給電、servoの実actuator動作）を明記し、この定義の下で未解決GPIOが無いことを示した。**要件照合Passはこの3件で1 round目に0件へ収束していない。****fresh-context Passで(a)(b)を、要件照合Passで(c)を検出した。**この訂正後に改めて両Passを実施し、新規指摘0件を確認した（2 round目） | 自己レビュー（`CONTRIBUTING.md`「自己レビュー」の2つのPass） |
| 2026-09-06 | 30 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**PR #358へ手動で依頼したCodeRabbit reviewの指摘2件を反映した。**(a) `信号inventory`の`ACCEL-SDA`行が、初回bring-upのmode決定（Standard-mode、Revision 27）の後も「400kHz(Fast-mode)を想定、要実測」というmode決定前の記述のままだった。**Standard-mode採用と、Fast-modeへ変更する場合はpull-up再設計が要る旨へ訂正した。**(b) checklist「Moduleのpull-upを並列合成した実効抵抗が有効範囲内である」を、Revision 27で「Standard-mode採用により実配線の`Cb`を測らなくても有効範囲内」として`[x]`にしていたが、**この判定は不十分だった。**`Rp(max)`約2.945 kΩ（Standard-modeの規定`Cb`上限400 pFから導いた値）は**rise timeの制約から`Rp`側で導いた上限であり、実配線の`Cb`が規定上限400 pFの範囲内であることを示すものではない。**実構成`Rp`（約2.42 kΩ）を使えばrise timeの制約だけなら`Cb`は約488 pFまで許容されるが、**Standard-modeの`Cb`上限400 pFがrise time以外の制約（fall time等、`Rp`の選定では動かせない可能性がある制約）にも由来するかどうかを、この文書が引用した一次資料（`tr`／`Cb`／`IOL`の列のみ）の範囲では確認できていない。**したがって、実配線の`Cb`が400〜488 pFの間にある場合、rise timeの計算は通ってもStandard-modeとして規定範囲内と言い切れない可能性が残る。**mode決定（Standard-mode採用）自体は取り消さない**（初回bring-upの選択としては変わらず有効）が、**この checklist項目は`Cb`の実測または設計上の根拠を得るまで未達へ戻した。**文書冒頭`状態`行も未達7件へ戻した。**受け入れ条件5件目（I2C addressとpull-upに互換性がある）の判定は変えていない。**同判定は`Rp(min)`（`Cb`／modeによらず常に成立し、素子の破損に至らないことの根拠）に基づくものであり、今回reopenしたのは`Rp(max)`側（通信の正しさに関わるが、外れても安全要件5項目には該当しない一般値tierの論点）である | PR #358のCodeRabbit手動review（2026-09-06、`@coderabbitai full review`） |
| 2026-09-06 | 31 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**競合checklistの2項目（MSP2807のlogic IO確認、ESP32電源投入前に外部moduleがpinをdriveしないこと）が確認方法を持っていなかったため、`実機check（電源off）の確認方法`節を新設して定義した。**`#2`本文の範囲（電源offでの導通とpin header対応だけ）を超えないこと、推測禁止（一般値や記憶で判定基準を作らない）を守った。**(1) driveしないことの確認は、非通電の導通checkだけで判定できると判定した。**周辺module3点はいずれもESP32の`3V3` pinから給電され独立電源を持たない（[power-budget.md](power-budget.md)の測定点定義）ため、ESP32電源offの状態では周辺moduleも無給電であり、無給電のICは内部topology（`HW-TBD-004`の直列抵抗未追跡、MSP2807の`U1`経路未追跡）によらず能動的にdriveできない。**したがってmodule側の出力段の性質はこの判定に効かない。**確認すべきは(a)module電源pinがESP32の`3V3`と同一netであること（独立電源が無いこと）、(b)pin header対応（配線ミスが無いこと）の2点であり、測定点・計器・判定基準を表にまとめた。**(2) MSP2807のlogic IO確認は、メーカー記載（3.3 V TTL）だけでは足りないと判定した。**`DISP-01`のVCCはESP32の3.3 V rail（3.234–3.366 V）から給電されるが、[power-budget.md](power-budget.md)が2026-09-06に確認したとおり同rail電圧では`DISP-01`board上の`U1`（3.3 V LDO）は常時dropout領域にあり、出力は3.3 Vへ完全収束しない。**controller IC（ILI9341／XPT2046）が`U1`出力側から給電されているかVCC直結かは、どちらの一次資料にも記録が無い。**したがってメーカー記載はこのboardの実際のlogic供給node電圧を保証しない。**非通電で追加確認できる方法（`U1`出力側の脚とcontroller IC電源pinの導通追跡）を1つ定義したが、backlight LED給電経路の追跡（`R5`／`Q1`）がすでに部品サイズを理由に非通電では決められないと結論しており、同程度のIC pinで同じ制約に当たる可能性が高いことも明記した。**解決しない場合、残る手段は通電を伴う実測であり`#2`の範囲を超え、実際の動作確認はLCD bring-up（`#13`）で行うと整理した。**PMの検査を受け、2点を追記した。**(a) (1)の項目へ「見ていないもの」を明記した。**`ACCEL-01`の`01C`（10 kΩ×4）のような module側pull-upがESP32起動時levelへ与える影響は、この項目（moduleが能動的にdriveしないこと）の対象外であり、`起動時状態を確定させる外部pull`節とbootstrap pinのreviewが扱う範囲であると書き分けた。**(b) (2)へ、ILI9341とXPT2046のdatasheetを2026-09-06に新たに取得し`VOH`（それぞれ`0.8×VDDI`、`IOVDD×0.8`）を確認したうえでの計算を追記した。**ESP32の`VIH`（rail 3.234–3.366 Vで2.4255–2.5245 V）と突き合わせると、logic供給nodeが`VCC`直結なら約160–170 mVの余裕があるが、`U1`出力経由（dropout、約2.99 V）なら`VOH`min約2.392 Vとなり約34–133 mV不足する。**計算だけではこの項目を閉じられず、答えは非通電追跡（logic供給nodeの特定）に懸かっていることを示した。**あわせて、ESP32からmoduleへの向き（`SCLK`等）はどちらのnode想定でも余裕があり、不足しうるのはmoduleからESP32への向き（`DOUT`／`PENIRQ`等）だけであることも明記した。**GPIO割り当てそのものは変えていない | PM（deskcat-f2）の追加依頼・検査、`WORK-INSTRUCTIONS-BENCH-2026-09-06.md`（PM作成、gitignore対象）、[power-budget.md](power-budget.md)、[sensor-datasheet-notes.md](sensor-datasheet-notes.md)、[ILI9341 Datasheet V1.11](https://cdn-shop.adafruit.com/datasheets/ILI9341.pdf) §18.2.1（2026-09-06取得）、[XPT2046 Datasheet](https://grobotronics.com/images/datasheets/xpt2046-datasheet.pdf)（2026-09-06取得） |
| 2026-09-07 | 32 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**`#2`のclose作業。**(a) 残っていた7本の抵抗（ADC分圧器4本、外部pull3本。`ADC-5V`／`ADC-3V3`分圧器`10 kΩ`×4、`ACCEL-SDA`／`ACCEL-SCL`pull-up`4.7 kΩ`×2、`LCD-BL`pull-down`4.7 kΩ`×1）をブレッドボードへ実装した。配線色は`ADC-5V`＝青、`ADC-3V3`＝白、`ACCEL-SDA`＝青、`ACCEL-SCL`＝緑、`LCD-BL`＝紫（人間の申告）。**抵抗の個別実測（DT830B）は行わない。**5%許容差の着荷済み品であり`Rp`等の余裕は桁で足りるため、人間と合意のうえ個別実測をしないと決めた。**やっていないことをやったと書かないため、この決定自体を記録する。**(b) `EN`を押し続けてESP32をresetに保持した状態で、`LCD-CS`(GPIO22)＝3.31 V、`LCD-RST`(GPIO16)＝3.31 V、`TOUCH-CS`(GPIO21)＝3.31 V、`LCD-BL`(GPIO4)＝0.00 Vを実測した（いずれも`EN`押下の有無で値が変化しないことを確認。`fc42332`の`SERVO-PWM`測定と同じ方法）。**2026-08-29の実測はpull-upの効果とfirmwareのHigh駆動を区別できていなかったが、今回`EN`保持により切り分けた。**記録は[experiment-log.md](experiment-log.md)の`EXP-011`。(c) **PM（`deskcat-f2`）が2026-09-07に`#2`のclose条件を再スコープした。**checklist項目3（I2C実効pull-up有効範囲）と項目5（外部moduleがpinをdriveしないこと）は、いずれもmoduleをESP32へ配線することが検証の前提であり、`初回bring-upの範囲`節が定める`#2`の範囲に含まれないため、close条件から外した（満たすのはmoduleを配線した時点。`#13`／`#15`／`#16`側）。**項目4（MSP2807のlogic IO）も、`#360`自身が「`R5`／`Q1`と同じ理由で決まらない可能性が高い」としていることと、`#2`本文の受け入れ条件7件のどれにも対応せず安全要件5項目にも該当しないことから、追跡を試みず`#13`（LCD bring-up）の通電実測へ送った。**checklistからは削除せず、理由と送り先を項目の記述へ書き足した。**文書冒頭`状態`行を、`#2`のclose条件（達成した4件）と、close条件ではない3件（module配線時に満たす）とに分けた。**`#2`のclose条件（項目1・2・6・7）はすべて達成した。**(d) [sensor-datasheet-notes.md](sensor-datasheet-notes.md) Revision 12にあった誤りをRevision 14で訂正した（Revision 12の本文は書き換えていない。詳細は同文書のRevision履歴）。**`U1`の脚の識別結果（`VCC`／`GND`導通による入力／GND／出力の判定）は電気的関係に基づく再現可能な識別であり、有効なまま残す** | 人間の現物作業（配線、`EN`保持測定）、PM（deskcat-f2）の判定（2026-09-07）、[experiment-log.md](experiment-log.md) `EXP-011` |
