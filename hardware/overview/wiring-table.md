<!-- generate.py が生成する。手で直さない。 -->
# 全体の配線（最終形のつながり）

ブレッドボード（B1・B2・B3）、電源の基板 `PROTO-02`、Pi、サーボ、LCD、AC アダプター M-12001 の、**最終形のつながり**である。つながりの正本は回路図 [`../deskcat.kicad_sch`](../deskcat.kicad_sch) で、この表と [`layout.svg`](layout.svg) は、回路図から書き出した netlist（[`deskcat-netlist.xml`](deskcat-netlist.xml)）から生成する。位置は胴の中の位置関係（[enclosure-and-neck.md](../../docs/hardware/enclosure-and-neck.md) の`物理的な完成形`）だけで、寸法は表さない。電源の基板は未実装の計画である（[universal-board/plan.md](../universal-board/plan.md)、#405）。この表は配線・通電の許可ではない。

**描かない物：**#18 の試験の間だけの物（端子台S、`PSU-BENCH-01`、試験用の負荷、試験用の shunt。PROTO-02 の R20 とは別）。それらは [`../bench-18/`](../bench-18/wiring-table.md) にある。

## まとまりと置き場所

| まとまり | 何 | 置き場所 | 回路図の参照番号 |
|---|---|---|---|
| LCD | LCD（DISP-01） | 頭。背板の前の面（ふたの上、サーボの前寄り） | J1 |
| SERVO | サーボ（SERVO-01） | ふたの上の天板。軸は胴の真ん中 | M1 |
| PROTO-02 | 電源の基板 PROTO-02 | 左の壁の内側に立てる | C1、C2、F1、J5、J6、J7、J8、Q1、R18、R20、W1 |
| M-12001 | AC アダプター M-12001 | 左の壁の後ろ寄りの穴から線が入る | PS1 |
| Pi | Raspberry Pi Zero W | 右の壁の内側の後ろ寄りに立てる | J4、J9 |
| B2 | B2（BB-601。LCD の14本を受ける） | 床の B1 の後ろ | R1、R2、R3、R6 |
| B1 | B1（EIC-3901。ESP32） | 床の前寄り | D1、D2、R4、R5、R7、R8、R9、R12、R13、R14、R15、R16、R17、R19、U1 |
| B3 | B3（BB-601。ENV-01・ACCEL-01） | 前の壁の内側に立てる | J2、J3、R10、R11 |

## 線（まとまりの間）

| ID | 節点 | 端1 | 端2 | 線 |
|---|---|---|---|---|
| S1 | `3V3` | B1 | B2 | 柔らかい線 S1（B1:+26 – B2:b1） |
| S2 | `GND` | B1 | B2 | 柔らかい線 S2（B1:-26 – B2:b2） |
| S3 | `3V3` | B1 | B3 | 柔らかい線 S3（B1:+27 – B3:b10） |
| S4 | `GND` | B1 | B3 | 柔らかい線 S4（B1:-27 – B3:b11） |
| S5 | `LCD-CS` | B1 | B2 | 柔らかい線 S5（B1:A14 – B2:b3） |
| S6 | `LCD-RST` | B1 | B2 | 柔らかい線 S6（B1:A23 – B2:b4） |
| S7 | `LCD-DC` | B1 | B2 | 柔らかい線 S7（B1:A22 – B2:b5） |
| S8 | `LCD-MOSI` | B1 | B2 | 柔らかい線 S8（B1:A13 – B2:c6） |
| S9 | `LCD-SCLK` | B1 | B2 | 柔らかい線 S9（B1:A20 – B2:c7） |
| S10 | `LCD-BL` | B1 | B2 | 柔らかい線 S10（B1:A24 – B2:b8） |
| S11 | `LCD-MISO` | B1 | B2 | 柔らかい線 S11（B1:A19 – B2:c9） |
| S12 | `TOUCH-CS` | B1 | B2 | 柔らかい線 S12（B1:A17 – B2:b10） |
| S13 | `TOUCH-IRQ` | B1 | B2 | 柔らかい線 S13（B1:K16 – B2:b11） |
| S14 | `I2C-SDA` | B1 | B3 | 柔らかい線 S14（B1:K20 – B3:g3） |
| S15 | `I2C-SCL` | B1 | B3 | 柔らかい線 S15（B1:K21 – B3:g2） |
| S16 | `ADC-3V3` | B1 | B3 | 柔らかい線 S16（B1:K14 – B3:f14） |
| N1 | `3V3` | B2 | LCD（J1 pin 1） | 首の線 LCD pin 1 VCC（B2:a1） |
| N2 | `GND` | B2 | LCD（J1 pin 2） | 首の線 LCD pin 2 GND（B2:a2） |
| N3 | `LCD-CS` | B2 | LCD（J1 pin 3） | 首の線 LCD pin 3 CS（B2:a3） |
| N4 | `LCD-RST` | B2 | LCD（J1 pin 4） | 首の線 LCD pin 4 RESET（B2:a4） |
| N5 | `LCD-DC` | B2 | LCD（J1 pin 5） | 首の線 LCD pin 5 DC/RS（B2:a5） |
| N6 | `LCD-MOSI` | B2 | LCD（J1 pin 6） | 首の線 LCD pin 6 SDI（B2:a6） |
| N7 | `LCD-SCLK` | B2 | LCD（J1 pin 7） | 首の線 LCD pin 7 SCK（B2:a7） |
| N8 | `LCD-BL` | B2 | LCD（J1 pin 8） | 首の線 LCD pin 8 LED（B2:a8） |
| N9 | `LCD-MISO` | B2 | LCD（J1 pin 9） | 首の線 LCD pin 9 SDO（B2:a9） |
| N10 | `LCD-SCLK` | B2 | LCD（J1 pin 10） | 首の線 LCD pin 10 T_CLK（B2:b7） |
| N11 | `TOUCH-CS` | B2 | LCD（J1 pin 11） | 首の線 LCD pin 11 T_CS（B2:a10） |
| N12 | `LCD-MOSI` | B2 | LCD（J1 pin 12） | 首の線 LCD pin 12 T_DIN（B2:b6） |
| N13 | `LCD-MISO` | B2 | LCD（J1 pin 13） | 首の線 LCD pin 13 T_DO（B2:b9） |
| N14 | `TOUCH-IRQ` | B2 | LCD（J1 pin 14） | 首の線 LCD pin 14 T_IRQ（B2:a11） |
| X1 | `PI-UART-TX-Pi` | B1 | Pi（J4 pin 8） | X1（B1:D5 → Pi の header pin 8（GPIO14 `TXD`）） |
| X2 | `PI-UART-RX-Pi` | B1 | Pi（J4 pin 10） | X2（B1:D6 → Pi の header pin 10（GPIO15 `RXD`）） |
| X3 | `GND` | B1 | Pi（J4 pin 39） | X3（B1:A18 → Pi の header pin 39（GND）） |
| X4 | `SERVO-SIG` | B1 | SERVO（M1 pin yellow） | X4（B1:D7 → サーボの plug の黄の穴） |
| X5 | `GND` | B1 | PROTO-02（J8 pin -） | X5（B1:-30 → 共通 GND（PROTO-02 の J8−。ESP32 の GND の戻りで、J8− が star point へ直結する。案B、ユーザーの決定 2026-11-23）（ESP32 の GND をサーボの GND と共通にする線）） |
| P1 | `5V-IN` | M-12001（PS1 pin +） | PROTO-02（J5 pin +） | M-12001 の線（Micro-B の手前で切った導体） |
| P2 | `GND` | M-12001（PS1 pin -） | PROTO-02（J5 pin -） | M-12001 の線（同上） |
| P3 | `5V-NODE` | PROTO-02（J6 pin +） | SERVO（M1 pin red） | サーボの赤（`111641` で延ばす。plug とのつなぎ方は #405） |
| P4 | `ADC-SHUNT` | PROTO-02（J6 pin -） | SERVO（M1 pin brown） | サーボの茶（同上）。節点は shunt のサーボ側（`/ADC-SHUNT`） |
| P5 | `5V-NODE` | PROTO-02（J7 pin +） | Pi（J9 pin VBUS） | `CABLE-PI-PWR-01`（`109314` を切った導体）→ Pi の `PWR IN` |
| P6 | `GND` | PROTO-02（J7 pin -） | Pi（J9 pin GND） | `CABLE-PI-PWR-01`（同上） |
| P7 | `5V-NODE` | PROTO-02（J8 pin +） | B1（U1 pin J2-19） | ESP32 の 5V pin へ（案B。`111641` 1本） |
| P8 | `ADC-SHUNT` | PROTO-02（W1 pin 1） | B1（U1 pin J2-7） | `ADC-SHUNT`（GPIO32）へ。`111641` 1本を床側へ通す（`W1`） |

サーボの3本：黄（信号）は B1 から X4、赤（5 V）と茶（GND）は PROTO-02 の J6 から（P3・P4）。

## 決まっていない所（TBD）

| ID | 間 | 何 |
|---|---|---|
| T1 | PROTO-02 – SERVO | サーボの plug とのつなぎ方（servo の枝の線の先）：#405 |
| T2 | B2 – LCD | LCD の VCC（pin 1）の電圧：今は 3V3（J1 開放）。最終は、決める事実（`HW-TBD-024`・`HW-TBD-025`(a)）が揃うまで決められず、揃えば #405 |
