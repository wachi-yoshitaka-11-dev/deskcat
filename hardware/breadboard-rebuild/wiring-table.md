<!-- generate.py が生成する。手で直さない。 -->
# 組み直しの結線表（#512）

穴の書き方: `B1 L12` は主板の L 行・12列、`B1 −25` は主板の − の列の25列の下の穴、`B2 e3` は LCD 板の e 行・3列。
配置の考え方、値と並びの出所、`TBD` は [README.md](README.md) にある。

## 板と module

| 何 | どこへ | 向き |
|---|---|---|
| ESP32 DevKitC V4（`MCU-01`） | B1。J3（GND 23 22 … CLK）を B 行の12〜30列、J2（3V3 EN VP … 5V）を J 行の12〜30列 | USB を30列の側（板の端）、アンテナを12列の側にする。B 行の上に1穴（A）、J 行の下に2穴（K・L）が残る |
| `ENV-01`（AE-BME280） | B3 の a 行。VDD 10、GND 11、CSB 12、SDI 13、SDO 14、SCK 15 | 本体を板の外（上の辺の外）へ出す向き。ピンの名前の順が逆になる向きしか取れないときは止める（TBD） |
| `ACCEL-01`（M-06724） | 下段5本を B3 の f 行。INT1 2、INT2 3、SDO 4、SDA 5、SCL 6。上段4本（CS Vs GND VDD）は上半分の2〜5列（CS 2、Vs 3、GND 4、VDD 5。写真の読み）の、間隔に合う行 | 2列が溝をまたぐ向き。上段の行は TBD。間隔が7ピッチ以上で上段が a 行に来る（上に空き穴が残らない）ときは組めないので止める |

## 抵抗（足は切る）

| ID | 値 | 品 | 端1 | 端2 | 用途 |
|---|---|---|---|---|---|
| R1 | 4.7 kΩ | `RES-PULL-01` | B1 L20 — GPIO25（I2C SDA） | B1 ＋20 — 3V3（J2-1 から） | I2C SDA の pull-up |
| R2 | 4.7 kΩ | `RES-PULL-01` | B1 L21 — GPIO26（I2C SCL） | B1 ＋21 — 3V3（J2-1 から） | I2C SCL の pull-up |
| R3 | 4.7 kΩ | `RES-PULL-01` | B1 L22 — GPIO27（SERVO-PWM） | B1 −22 — GND | SERVO-PWM の pull-down |
| R4 | 4.7 kΩ | `RES-PULL-01` | B1 A1 — GPIO14（PI-UART-RX） | B1 A5 — PI_TXD（Pi pin 8 TXD 側） | UART 直列（Pi→ESP32） |
| R5 | 4.7 kΩ | `RES-PULL-01` | B1 C2 — GPIO13（PI-UART-TX） | B1 C6 — PI_RXD（Pi pin 10 RXD 側） | UART 直列（ESP32→Pi） |
| R6 | 330 Ω | `CF25J330RB` | B1 E3 — GPIO27（SERVO-PWM） | B1 E7 — SERVO_SIG（サーボの黄の線の側） | サーボ信号の直列（EXP-028） |
| R7 | 330 Ω | `CF25J330RB` | B1 H5 — GPIO2（LED-COMM） | B1 H1 — LED_W_A（白LEDのアノード） | 白LEDの電流制限 |
| R8 | 330 Ω | `CF25J330RB` | B1 I2 — GPIO5（LED-REACT） | B1 I6 — LED_R_A（赤LEDのアノード） | 赤LEDの電流制限 |
| R9 | 10 kΩ | `RES-PULL-01` | B1 L5 — GPIO2（LED-COMM） | B1 −5 — GND | LED-COMM の pull-down |
| R10 | 10 kΩ | `RES-PULL-01` | B1 L2 — GPIO5（LED-REACT） | B1 −2 — GND | LED-REACT の pull-down |
| R11 | 10 kΩ | `MEAS-01 分圧用` | B1 K8 — 5V（J2-19 の 5V pin） | B1 K4 — GPIO33（ADC-5V） | ADC-5V 分圧の上 |
| R12 | 10 kΩ | `MEAS-01 分圧用` | B1 L4 — GPIO33（ADC-5V） | B1 −4 — GND | ADC-5V 分圧の下 |
| R13 | 10 kΩ | `RES-PULL-01` | B2 e3 — GPIO22（LCD-CS） | B2 f3 — 3V3（J2-1 から） | LCD-CS の pull-up |
| R14 | 10 kΩ | `RES-PULL-01` | B2 e4 — GPIO16（LCD-RST） | B2 f4 — 3V3（J2-1 から） | LCD-RST の pull-up |
| R15 | 10 kΩ | `RES-PULL-01` | B2 e10 — GPIO21（TOUCH-CS） | B2 f10 — 3V3（J2-1 から） | TOUCH-CS の pull-up |
| R16 | 4.7 kΩ | `RES-PULL-01` | B2 e8 — GPIO4（LCD-BL） | B2 f8 — GND | LCD-BL の pull-down |
| R17 | 10 kΩ | `MEAS-01 分圧用` | B3 g10 — 3V3（J2-1 から） | B3 g14 — GPIO36（ADC-3V3） | ADC-3V3 分圧の上 |
| R18 | 10 kΩ | `MEAS-01 分圧用` | B3 h14 — GPIO36（ADC-3V3） | B3 h11 — GND | ADC-3V3 分圧の下 |

## LED（足は切る。向きをこの表で残す）

| ID | 品 | アノード（長い足） | カソード（短い足） |
|---|---|---|---|
| D1 | 白 LED-COMM-01（OSPW5111B-QR） | B1 L1 | B1 −1 — GND |
| D2 | 赤 LED-REACT-01（OSHR5161A-QR） | B1 L6 | B1 −6 — GND |

## 硬いジャンパー（被覆あり）

長さは両端の穴の間隔（ピッチ＝2.54 mm の数）。溝をまたぐものと電源列へ下りるものは、写真からの目安で TBD（現物で合う長さを選ぶ）。

| ID | 端1 | 端2 | 長さ（ピッチ） | 信号 |
|---|---|---|---|---|
| H1 | B1 L12 | B1 ＋12 | 4（目安、TBD） | 3V3（J2-1 から） |
| H2 | B1 L25 | B1 −25 | 3（目安、TBD） | GND |
| H3 | B2 e1 | B2 f1 | 3（目安、TBD） | 3V3（J2-1 から） |
| H4 | B2 e2 | B2 f2 | 3（目安、TBD） | GND |
| H5 | B2 j1 | B2 j3 | 2 | 3V3（J2-1 から） |
| H6 | B2 i3 | B2 i4 | 1 | 3V3（J2-1 から） |
| H7 | B2 h4 | B2 h10 | 6 | 3V3（J2-1 から） |
| H8 | B2 g2 | B2 g8 | 6 | GND |
| H9 | B3 e10 | B3 f10 | 3（目安、TBD） | 3V3（J2-1 から） |
| H10 | B3 e11 | B3 f11 | 3（目安、TBD） | GND |
| H11 | B3 c11 | B3 c14 | 3 | GND |

## 柔らかい線（両端オス）

| ID | 端1 | 端2 | 信号 | 組み直した直後 |
|---|---|---|---|---|
| S1 | B1 ＋26 | B2 b1 | 3V3（J2-1 から） | 挿す |
| S2 | B1 −26 | B2 b2 | GND | 挿す |
| S3 | B1 ＋27 | B3 b10 | 3V3（J2-1 から） | 挿す |
| S4 | B1 −27 | B3 b11 | GND | 挿す |
| S5 | B1 A14 | B2 b3 | GPIO22（LCD-CS） | 挿す |
| S6 | B1 A23 | B2 b4 | GPIO16（LCD-RST） | 挿す |
| S7 | B1 A22 | B2 b5 | GPIO17（LCD-DC） | 挿す |
| S8 | B1 A13 | B2 c6 | GPIO23（LCD-MOSI） | 挿す |
| S9 | B1 A20 | B2 c7 | GPIO18（LCD-SCLK） | 挿す |
| S10 | B1 A24 | B2 b8 | GPIO4（LCD-BL） | 挿す |
| S11 | B1 A19 | B2 c9 | GPIO19（LCD-MISO） | 挿す |
| S12 | B1 A17 | B2 b10 | GPIO21（TOUCH-CS） | 挿す |
| S13 | B1 K16 | B2 b11 | GPIO34（TOUCH-IRQ） | 挿す |
| S14 | B1 K20 | B3 g5 | GPIO25（I2C SDA） | 挿す |
| S15 | B1 K21 | B3 g6 | GPIO26（I2C SCL） | 挿す |
| S16 | B1 K14 | B3 f14 | GPIO36（ADC-3V3） | 挿す |
| S17 | B1 K19 | B1 G4 | GPIO33（ADC-5V） | 挿す |
| S18 | B1 K30 | B1 G8 | 5V（J2-19 の 5V pin） | G8 側を先に挿し、K30 側を後に挿す（5 V の端を最後にする） |
| S19 | B1 A26 | B1 G5 | GPIO2（LED-COMM） | A26 側を抜いておく（EXP-026 で ESP32 側を抜いた状態） |
| S20 | B1 A21 | B1 G2 | GPIO5（LED-REACT） | A21 側を抜いておく（EXP-026 で ESP32 側を抜いた状態） |
| S21 | B1 K23 | B1 F1 | GPIO14（PI-UART-RX） | 挿す |
| S22 | B1 K26 | B1 F2 | GPIO13（PI-UART-TX） | 挿す |
| S23 | B1 K22 | B1 F3 | GPIO27（SERVO-PWM） | 挿す |
| S24 | B3 h5 | B3 b13 | GPIO25（I2C SDA） | 挿す |
| S25 | B3 h6 | B3 b15 | GPIO26（I2C SCL） | 挿す |
| S26 | B3 c10 | B3 の3列（ACCEL-01 上段の Vs）の、上段より上の空き穴（行は TBD） | 3V3（J2-1 から） | 挿す |
| S27 | B3 d11 | B3 の4列（ACCEL-01 上段の GND）の、上段より上の空き穴（行は TBD） | GND | 挿す |
| S28 | B3 の2列（ACCEL-01 上段の CS）の、上段より上の空き穴（行は TBD） | B3 の5列（ACCEL-01 上段の VDD）の、上段より上の空き穴（行は TBD） | 3V3（J2-1 から） | 挿す |
| S29 | B3 g4 | B3 i11 | GND | 挿す |

## 板の外へ出る線（柔らかい線）

| ID | 板の端 | 板の外の端 | 信号 | 組み直した直後 |
|---|---|---|---|---|
| X1 | B1 D5 | Pi の header pin 8（GPIO14 `TXD`） | PI_TXD（Pi pin 8 TXD 側） | D5 側を抜いておく。挿すのは gpio-assignment.md の `信号線をつないでよい条件` を満たすときだけ |
| X2 | B1 D6 | Pi の header pin 10（GPIO15 `RXD`） | PI_RXD（Pi pin 10 RXD 側） | D6 側を抜いておく。条件は X1 と同じ |
| X3 | B1 A18 | Pi の header pin 39（GND） | GND | 条件は X1 と同じ（GND の線の確かめ方も同じ節） |
| X4 | B1 D7 | サーボの plug の黄の穴 | SERVO_SIG（サーボの黄の線の側） | サーボの試験の記録（#17）の状態に合わせる |
| X5 | B1 −30 | 端子台S の −（ESP32 の GND をサーボの GND と共通にする線。EXP-034 の ②） | GND | サーボの試験の記録（#17）の状態に合わせる |

## 首を通る LCD の14本（胴側のオスの端）

| LCD pin | 表記 | 挿す穴 | 信号 |
|---:|---|---|---|
| 1 | VCC | B2 a1 | 3V3（J2-1 から） |
| 2 | GND | B2 a2 | GND |
| 3 | CS | B2 a3 | GPIO22（LCD-CS） |
| 4 | RESET | B2 a4 | GPIO16（LCD-RST） |
| 5 | DC/RS | B2 a5 | GPIO17（LCD-DC） |
| 6 | SDI | B2 a6 | GPIO23（LCD-MOSI） |
| 7 | SCK | B2 a7 | GPIO18（LCD-SCLK） |
| 8 | LED | B2 a8 | GPIO4（LCD-BL） |
| 9 | SDO | B2 a9 | GPIO19（LCD-MISO） |
| 10 | T_CLK | B2 b7 | GPIO18（LCD-SCLK） |
| 11 | T_CS | B2 a10 | GPIO21（TOUCH-CS） |
| 12 | T_DIN | B2 b6 | GPIO23（LCD-MOSI） |
| 13 | T_DO | B2 b9 | GPIO19（LCD-MISO） |
| 14 | T_IRQ | B2 a11 | GPIO34（TOUCH-IRQ） |

## 足を切る部品・切らない部品

- 切る: 抵抗 18 本（R1〜R18）、LED 2 本（D1・D2）。穴の奥まで入る長さを残し、板の上に出る金属を短くする。
- LED は切る前に長い足（アノード）を確かめ、上の表の穴に挿してから、もう一度向きを表と照らす。
- 切らない: `PROTO-02` へ移す電源部の部品（`PROT-OC-01` の PTC、`WIRE-PWR-01` の端子台と線、`MEAS-01` の shunt）。どれもブレッドボードに載せない。
- module（ESP32、`ENV-01`、`ACCEL-01`）のピンは切らない。

## 載せないもの

- `MEAS-02`（Arduino）の線（A0・GND）と、EXP-030 の Arduino 用の 10 kΩ 分圧（a10–a12、b12–b14）。測定器であり、必要なら実機側が試験のときに足す。
- `MEAS-01` の shunt と GPIO32（`ADC-SHUNT`）の線。shunt はサーボの電流を通すのでブレッドボードに載せない。GPIO32 の線は今の配線に無く、足すかどうかは #17・#405 で決める（J2-7 の列 B1 K18・L18 を空けてある）。

