# GPIO Assignment

> 状態: Blocked — 実機での電源off導通check、MSP2807のlogic IO levelの現物確認、servo起動時状態の安全review待ち
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
| **`SERVO-PWM`** | **pull-down（必須）** | **未確定**（4.7 kΩを推奨） | 1 | 下記「`SERVO-PWM`の値を確定させない理由」。**振り分けがPMで決まるまで確定しない** |
| `LCD-CS` | pull-up | **10 kΩ** | 1 | `Rmax` 943 kΩに対して94倍の余裕。駆動負荷は`IOL`の1.1 % |
| `LCD-RST` | pull-up | **10 kΩ** | 1 | 同上。**あわせて下記「`LCD-RST`の2つの注意」** |
| `TOUCH-CS` | pull-up | **10 kΩ** | 1 | `Rmax` 196 kΩに対して20倍の余裕 |
| `LCD-BL` | pull-down | **未選定** | 1（予定） | 極性と backlight 回路の入力条件が未確定である。下記「`LCD-BL`を決められない理由」 |
| `TOUCH-IRQ` | **外部pullを付けない** | — | **0** | 下記「`TOUCH-IRQ`へ外部pull-upを付けてはならない」 |

**手元の2種で足りる。**10 kΩ（秋月 125103）と4.7 kΩ（秋月 125472）がどちらも1袋100本入で2026-08-08に着荷している
（[hardware-bom.md](hardware-bom.md) `RES-PULL-01`）。**確定したのは 10 kΩ×3 の3本である。**`SERVO-PWM` の 4.7 kΩ×1 は推奨にとどまる。**いずれにしても手元の2種で足り、追加の発注は要らない。**
**ただし現物の表示・値の確認はしていない**（同BOMの`着荷済み`と`受け入れ済み`の区別）。
消費電力はどちらも問題にならない（3.3 Vで4.7 kΩが2.32 mW、10 kΩが1.09 mW。**1/4 W = 250 mWの1 %未満**）。

#### `SERVO-PWM`の値を確定させない理由と、揃えた材料

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

**確定しない理由は規則側にある。**[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の対応表は
「pull-upとdecouplingの値」を**一般値で開始してよい側**、「サーボPWM、可動域、速度、加速度」を**一次資料を要する側**に置く。
**`SERVO-PWM`のpull-down抵抗値は両方に読める。**pull抵抗の値であるから前者に読め、
外すとreset時のhigh-Z区間でservoが不定pulseを受け、機構へ押し付けられれば安全5項目の
「servoの持続的拘束」に至りうるから後者にも読める。**この振り分けはこの節では決めない。**

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

#### `LCD-BL`を決められない理由

- **極性が未確定である。**`信号inventory`の同行が「極性は現物確認後に決定」としている。
  **向きが逆なら値も無意味になる。**
- **backlight回路の入力条件が未確定である。**`R5`＝6.8 Ω／`R6`＝1 kΩ／`Q1`＝`J3Y`は2026-08-13に読み取ったが、
  **パターンを追っていないため回路modelが確定していない**（[`HW-TBD-024`](tbd-register.md)／[`HW-TBD-002`](tbd-register.md)）。
  **どれだけの電流が流れ込むかが分からないため、上限を出せない。**
- **ESP32側は分かっている。**GPIO4はreset時も reset後も`oe=0, ie=1, wpd`であり、**内部weak pull-downが有効である。**
  **この pin は floating ではない。**外部pull-downは内部`RPD`（typ 45 kΩ）と並列になる
  （外部10 kΩで8.18 kΩ、外部4.7 kΩで4.26 kΩ）。

**したがって`LCD-BL`の値は`HW-TBD-002`／`HW-TBD-024`の後に決める。本数は1本を見込む。**

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

**値そのものはまだ確定していない。**下記「まだ確定できない3つの入力」が埋まるまで決まらない。
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
- **BME280側の4.7 kΩは数えない。**`J1`／`J2`が開放でbusへ繋がっていないことを2026-08-22に実測で確定した
  （正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)。**ここへ再掲しない**）。
- **数える候補はADXL345 module側の`01C`（EIA-96で10 kΩ、1%）だけである**（同文書`Module搭載pull-up`）。

### まだ確定できない3つの入力

1. **ADXL345 module側の`01C`（10 kΩ）4個が、どのpinへ付くかが未確定である。**
   パターンを追っていない（[`HW-TBD-004`](tbd-register.md)）。SDA／SCLへ各1本なのか、
   別pinを含むのかで並列合成後の実効値が変わる。**現物でパターンを追う必要がある。**
2. **bus容量`Cb`を得ていない。**`Cb`は配線・接続・pinの合計容量であり、**実配線が存在しない。**
3. **採るmodeを決めていない。**[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の
   `検証済み最大bus速度`は`TBD`である。**`tr` maxがStandardとFastで3.3倍違うため、`Rp(max)`も同じ比で動く。**

### 判断に使える境界

上の式へ値を入れたものである。**確定値ではなく、確定した時点で判断に使う境界である。**

`Rp`に許される`Cb`の上限（`Cb(max) = tr / (0.8473 × Rp)`）。

| `Rp` | 成立する場合 | Standard-mode | Fast-mode |
|---|---|---|---|
| 10 kΩ | ADXL345側の1本だけがbusに付く | 118 pF | 35 pF |
| 5.00 kΩ | ADXL345側の`01C`が2本並列で同じlineに付く | 236 pF | 71 pF |
| 3.20 kΩ | 10 kΩに`J1`／`J2`の4.7 kΩを並列に足す | 369 pF | 111 pF |

**あわせて次が言える。**Fast-modeで`Cb`が規定上限の400 pFに達した場合、
`Rp(max)` = 300 ns / (0.8473 × 400 pF) = **約885 Ω**となり、**`Rp(min)`の約967 Ωを下回る。**
**この組み合わせは3.3 Vの受動pull-upでは成立しない。**
UM10204 Table 10の注記も、400 kHzでfull bus loadを駆動するには`VOL` = 0.6 Vで`IOL` 6 mAが要るとしている。
**したがってmodeと`Cb`の決定は、pull-up値の選定と切り離せない。**

### `J1`／`J2`の判断

**まだ決めていない。**上の3つの入力が埋まった時点で、この節の式で決める。
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
| LCD-CS | DISP-01 | Chip select | Output | GPIO22 | 起動時floating→firmware初期化前は不定 | **外部`10 kΩ`×`1本`を選定した**（2026-08-25。active-low CSをfirmware初期化前もinactive＝Highに保つため）（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**未実装である** | **Active-low（一次資料で確定）。**ILI9341 datasheet V1.11が`CSX`をactive lowと明記している。**現物のpolarity確認は要しない** | なし | Output設定前にinactiveにする。Pull-up未実装の場合、起動直後の数十ms間bus contentionのriskがある |
| LCD-DC | DISP-01 | Data／command | Output | GPIO17 | floating | 外部pull不要（WROOM-32Dのため使用可） | Device固有（要現物確認） | なし | WROOM/SOLO-1専用pin。今回のmoduleはWROOM-32Dのため使用可 |
| LCD-RST | DISP-01 | Reset | Output | GPIO16 | floating | **外部`10 kΩ`×`1本`を選定した**（2026-08-25）（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**pull-upを付けてもfirmwareのhardware resetは省けない**（ILI9341 §12.1は`RESX`がHighまたは不定なら電源投入後にhardware resetを要するとしている）。**GPIO16は`VDD_SDIO` domainであり、3.3 Vへのpull-upが定格内である前提は同domainが3.3 Vであることに依る**（同節）。**未実装であり、向きの最終判断は`HW-TBD-032`に残る** | **Active-low（一次資料で確定。**ILI9341 datasheet V1.11が`RESX`を`Signal is active low.`と明記**）。**Pulse timingは現物確認後に決定 | なし | 起動時glitchを防ぐ。Firmwareが最初にHighを出力するまでの間もHighに保つ設計が望ましい |
| LCD-BL | DISP-01 | Backlight | Output（PWM調光は将来検討） | GPIO4 | **不定**。ESP32のGPIO4はreset時にinput（内部weak pull-downあり）で、driveされた状態にはならない。ただし内部weak pullは外部回路に対して弱く、backlight回路の入力仕様によっては点灯しうる。firmwareまたは外部pullが確定させるまでOffを保証しない | **外部pull-down推奨**（backlightをfirmware初期化前もOffに確定させるため）。**値は2026-08-25時点で未選定である。**極性とbacklight回路の入力条件が未確定であり上限を出せない（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**本数は1本を見込む。**極性は現物確認後に決定 | 現状はdigital on/off。将来PWM調光も可能なpinを選定 | なし | 回路上のLED電流経路はmodule内蔵に依存。直接大電流をdriveしない（module側で電流制限されている前提、現物確認要） |
| TOUCH-CS | TOUCH-01 | Chip select（touch controller用、LCD-SPIバスを共有） | Output | GPIO21 | 起動時floating→不定 | **外部`10 kΩ`×`1本`を選定した**（2026-08-25。LCD-CSと同じ理由）（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**根拠は別である**（受け側がXPT2046であり、漏れ電流が\|`IIH`\| ≤ 5 µAでILI9341の1 µAより大きい。`Rmax`は196 kΩ）。**未実装である** | **Active-low（一次資料で確定）。**XPT2046 datasheetのpin表が`CS`にoverlineを付けている。**現物のpolarity確認は要しない** | DISP-01とSCLK／MOSI／MISOを共有 | **Touch controllerは2026-08-13に`XPT2046`と確定した**（現物chip刻印。`hardware-bom.md` TOUCH-01）。polarityは`XPT2046`のdatasheetで確認する |
| TOUCH-IRQ | TOUCH-01 | Interrupt（touch検出） | Input | GPIO34 | 入力専用、floating | **外部pullを付けない（本数0本）。**2026-08-25に一次資料から判定した。**XPT2046の`PENIRQ`は内部pull-up付きの出力**（公称50 kΩ）であり、外部pull-upは不要かつ**有害である**（並列に足すとlow levelが`0.35×VCC`の保証を超える）（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**旧記載「外部pull-up推奨（一般的なtouch controllerはactive-low IRQ。要現物確認）」はcontroller未確定時の記述であり、2026-08-13の`XPT2046`確定で前提が変わっていた** | Edge／level要確認 | なし | Input-only pin。Output不可のため他用途に転用できない |
| ACCEL-SDA | ACCEL-01 | I2C SDA | Bidirectional | GPIO25 | floating（open-drain想定） | 外部4.7kΩ pull-up（**ADXL345モジュールは`01C`＝10 kΩのpull-upを4個搭載していることを2026-08-13に現物確認した。**ただし**どのpinへ付くかはパターンを追っていないため未確定**であり、実効抵抗の計算前に配線を確認する） | 400kHz(Fast-mode)を想定、要実測 | ENV-01と共有 | ADXL345はI2C／SPI選択式。Interface選択jumperの現物確認が必要（`hardware-bom.md` ACCEL-01） |
| ACCEL-SCL | ACCEL-01 | I2C SCL | Bidirectional | GPIO26 | 同上 | 同上 | 同上 | ENV-01と共有 | 同上 |
| ACCEL-IRQ | ACCEL-01 | Interrupt（tap／free-fall検出） | Input | GPIO35 | 入力専用 | **外部pull要確認**（`HW-TBD-004`）。**ICの事実:**ADXL345のINT1/INT2は**push-pull固定**であり、設定で切り替えられない（`Both interrupt pins are push-pull, low impedance pins`。Rev. G page 19）。polarityは`DATA_FORMAT` register（`0x31`）の`INT_INVERT` bitで選び、**同registerのreset値が`00000000`であるためICの既定はactive-highである**（Rev. G Table 19 page 23、page 27）。**旧記載の「push-pull／open-drainを設定可能」はICの事実として誤りであり、2026-08-12に訂正した**（Revision 9）。**module levelは別である。**M-06724のboard上でINT pinがheaderへ直結しているか（直列抵抗、level shift、引き出しの有無）を示す資料が無いため、**外部pullの要否とheaderで観測されるpolarityは現物確認まで確定しない。ICがpush-pullであることからmoduleの配線条件を導かない**（[tbd-register HW-TBD-004](tbd-register.md)） | Edge想定 | なし | ADXL345のtap／free-fall検出hardwareを軽打／持ち上げ判定に使う場合に使用（`hardware-bom.md` ACCEL-01の採用理由） |
| ENV-SDA | ENV-01 | I2C SDA | Bidirectional | GPIO25（ACCEL-01と共有） | 同上 | 同上 | 同上 | ACCEL-01と共有 | BME280はI2C／SPI選択式。**2026-08-22に選択jumperを実測し、`J1`／`J2`／`J3`は3つとも開放であると確定した**（正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`現物の実装状態を実測で確定させた（2026-08-22）`）。**したがってI2Cで使うには`J3`のはんだ付けが要る**（実装作業）。**module搭載の4.7 kΩプルアップも繋がっていない** |
| ENV-SCL | ENV-01 | I2C SCL | Bidirectional | GPIO26（ACCEL-01と共有） | 同上 | 同上 | 同上 | ACCEL-01と共有 | 同上 |
| SERVO-PWM | SERVO-01 | PWM control | Output | GPIO27 | **不定**。ESP32のGPIO27はreset時にhigh-Z（output disable、input disable）であり、Lowにdriveされる保証はない。**Lowと仮定しない。**外部pull-downが確定させるまで、servoは不定pulseを受けうる | **外部pull-down必須**（推奨ではない）。high-Z期間中もLowを保証する唯一の手段であり、これがないとPWM driver初期化前にservoが動きうる。詳細は`servo-safety-limits.md`。**値は未確定である。**`4.7 kΩ`×`1本`を**推奨として**置くところまでで、確定はしていない（材料と導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節。**ここへ再掲しない**）。**2026-08-26時点で、この抵抗値を一般値で開始してよい側と一次資料を要する側のどちらに置くかが決まっていない**（[hardware-safety-policy.md](../governance/hardware-safety-policy.md)の対応表は「pull-upとdecouplingの値」を一般値側、「サーボPWM、可動域、速度、加速度」を一次資料側に置いており、**この項目は両方に読める**）。**振り分けが決まるまで値を確定させない。****上限はSG90の`logic閾値`が一次資料に無いため計算できない**（[`HW-TBD-026`](tbd-register.md)(a)）。**そのうえで、駆動側の下限に余裕がある範囲で未知に強い側（低い値）を採った。**部品は`hardware-bom.md`の`RES-PULL-01`（10 kΩと4.7 kΩが各1袋100本入、2026-08-08着荷。**追加の発注は要らない**）。**選定は実装ではない。未実装である** | 50Hz、pulse幅は`servo-safety-limits.md`で規定する制限に従う | なし | Strapping pinでもflash pinでもない。起動時とdriver故障時の状態は`tbd-register.md` HW-TBD-019で引き続き検討する |
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
- [ ] 5 Vと3.3 V railのADC入力に分圧器が実装され、ADC定格3.3 Vを超えない（分圧比1/2を規定済み。実配線が存在しないため未検証）
- [x] 共有SPI上の各deviceに個別CSがある（LCD: GPIO22、Touch: GPIO21）
- [x] I2C deviceのaddressが一意、または明示的な対策がある（**候補の全組み合わせで衝突しない。**ADXL345は`0x1D`（`SDO/ALT ADDRESS`をhigh）／`0x53`（同pinをGNDへ）、BME280は`0x76`（`SDO`→GND）／`0x77`（`SDO`→VDD）であり、**2×2の4通りすべてで重複が無い**（2026-08-25に確認）。**したがってaddressの選択は衝突回避を理由に決まらない。**候補の出所は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)であり、**ここへ再掲しない。****両moduleとも`SDO`が基板上で固定されていないことを実測で確定しており**（2026-08-22）、**どちらのaddressも配線しなければ定まらない。**未接続のまま通電しない。選択の判断材料は[I2C addressの選択](#i2c-addressの選択)節にある）
- [ ] すべての外部pull-upが3.3Vへ接続され、5Vへ接続されていない（`電圧domain`節で規定済み。ただし実配線が存在しないため未検証）
- [ ] Moduleのpull-upを並列合成した実効抵抗が有効範囲内である（**式と前提の正本は[I2C busの実効pull-up](#i2c-busの実効pull-up)節である。****ADXL345は`01C`＝10 kΩ×4を搭載していることを2026-08-13に確認したが、どのpinへ付くかは未確定。**BME280は`J1`／`J2`のはんだジャンパで4.7 kΩの接続を選ぶ設計で、**2026-08-22の導通確認で両方とも開放と確定した。したがって実効pull-upの計算にBME280側の4.7 kΩを入れない。****`J1`／`J2`をはんだ付けするかはこの計算の結果で決める。まだ決めていない。****ADXL345側のpin接続の確認が残るため、実効pull-upの計算はまだできない**（旧記載: 導通確認は[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)の範囲としていたが、**BME280側は2026-08-22に確定した**）
- [ ] MSP2807のlogic IOが3.3Vで動作することを現物で確認した（VCC 3.3–5V対応だがlogic IOは3.3V TTL。`power-budget.md`参照）
- [ ] ESP32の電源投入前に外部moduleがESP32 pinをdriveしない（未検証、実機電源offでの導通checkが必要）
- [ ] Resetとbacklight lineが安全な状態で起動する（LCD-RST/LCD-CSへの外部pull-up実装が前提。**2026-08-25に値と本数を選定した**（`LCD-RST`／`LCD-CS`とも10 kΩ×1本。導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節）**が、実装は未了であり、`LCD-BL`の値は未選定のままである。**したがってこの項目は満たしていない。[`HW-TBD-032`](tbd-register.md)で追跡する）
- [ ] Servo PWMがdisabledまたは承認済みの安全状態で起動する（GPIO27はreset時high-Zであり、外部pull-downを**必須**とした。**reset時状態が`oe=0, ie=0`＝内部pull無しであることをESP32 Datasheet v5.3の`IO_MUX`で2026-08-25に確認した。**同日に**4.7 kΩ×1本を推奨として置いた**（導出は[起動時状態を確定させる外部pull](#起動時状態を確定させる外部pull)節）。**値は確定していない**（一般値側か一次資料側かの振り分けが未了）。**実装・検証とも未了である。****判定閾値はSG90の`logic閾値`が未確定のため確定していない**（[`HW-TBD-026`](tbd-register.md)(a)）。`tbd-register.md` HW-TBD-019と連動、未解決）

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
| 2026-08-25 | 18 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)。**`起動時状態を確定させる外部pull`節を新設し、`信号inventory`の`Pull`列へ選定した値と本数を入れた。****確定した内訳**: `LCD-CS`／`LCD-RST`／`TOUCH-CS`のpull-upが**各10 kΩ×1本**（一般値で開始してよい側）。**`SERVO-PWM`のpull-downは4.7 kΩ×1本を推奨として置いたが確定していない**（一般値側か一次資料側かの振り分けが未了）。**`LCD-BL`は未選定のまま**（極性とbacklight回路の入力条件が未確定で上限を出せない）。**`TOUCH-IRQ`は外部pullを付けない（0本）へ改めた。**根拠は一次資料である（**ESP32 Series Datasheet v5.3** の Table 5-3 と Appendix `IO_MUX` と §3.2、**ILI9341 Datasheet V1.11** の §18.2.1 と §12.1／§12.2、**XPT2046 Datasheet**（2007.5）の`DIGITAL INPUT/OUTPUT`と`PENIRQ Output`）。**一次資料で分かった重要な点が4つある。**(a) **`SERVO-PWM`（GPIO27）のreset時状態は`oe=0, ie=0`で内部pullが無く、真のhigh-Zである。**外部pull-downが必須である理由が`IO_MUX`の値として裏付いた。(b) **`LCD-CS`／`LCD-RST`／`TOUCH-CS`のpolarityは現物確認を要しない。**ILI9341が`CSX`と`RESX`をactive low、XPT2046が`CS`をactive lowと明記している。**旧記載の`要現物のpolarity確認`を削除した。**(c) **`TOUCH-IRQ`へ外部pull-upを付けると有害である。**XPT2046の`PENIRQ`は内部pull-up付きの出力（公称50 kΩ）であり、外部10 kΩを並列に足すと実効8.33 kΩになり、datasheetが`logic low 0.35×(+VCC)`を保証する条件（X+とY−間21 kΩ未満）に対してlowが**0.716×VCC**まで上がる。**touchがLowとして読めなくなる。**旧記載の`外部pull-up推奨`はcontroller未確定時のものであった。(d) **`LCD-RST`（GPIO16）は`VDD_SDIO` domainにある。**3.3 Vへのpull-upが定格内である前提は同domainが3.3 Vであることに依り、それは`MTDI`（GPIO12）のreset時の内部weak pull-downと、module内flashが動作している事実から成り立つ。**`SERVO-PWM`だけは上限を計算できない**（SG90の`logic閾値`が一次資料に無い。`HW-TBD-026`(a)）。**駆動側の下限に余裕がある範囲で未知に強い側を採り、4.7 kΩを推奨とした。確定はしていない。****この変更は値と本数の選定であって実装ではない。5本とも未実装である。****この文書の状態`Blocked`は解除していない。checkboxも1つも開いていない** | [ESP32 Series Datasheet v5.3](https://www.espressif.com/sites/default/files/documentation/esp32_datasheet_en.pdf)、[ILI9341 Datasheet V1.11](https://cdn-shop.adafruit.com/datasheets/ILI9341.pdf)、[XPT2046 Datasheet](https://grobotronics.com/images/datasheets/xpt2046-datasheet.pdf)、[tbd-register.md](tbd-register.md) `HW-TBD-026`／`HW-TBD-027`／`HW-TBD-032`、[hardware-bom.md](hardware-bom.md) `RES-PULL-01` |
