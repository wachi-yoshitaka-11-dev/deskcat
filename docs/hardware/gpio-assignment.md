# GPIO Assignment

> 状態: Blocked — 周辺部品の識別とboard現物確認が必要
> 正本とする情報: ESP32 boardのpin割り当て

## 割り当て規則

- 正確なESP32-DevKitC-32E boardと搭載moduleの文書を使用する。
- flash、bootstrapping、USB-UART、board LED、使用制限のあるpinを考慮する。
- すべてのmoduleについて、電圧と起動時drive stateを確認する。
- 物理信号ごとに一行を使用する。
- Tutorialまたは類似boardのGPIO番号をコピーしない。
- Firmwareのpin定数は、この文書から生成するか、この文書と手動で同期させる。

## Board識別情報

| 項目 | 値 | 根拠 |
|---|---|---|
| Board family | ESP32-DevKitC-32E | ADR-0001のproject target。board revisionは未確認 |
| 正確なboard revision | TBD | Board現物と公式回路図を確認する |
| 搭載ESP32 module suffix | TBD | Module shieldの表示を確認する |
| 公式回路図revision | TBD | Board識別後に選定する |
| Firmware board configuration ID | TBD | Toolchain bring-up時に定義する |

## 信号inventory

| Signal ID | Device | 信号 | ESP32側の方向 | GPIO | Boot state | Pull | Bus設定 | 共有先 | 制約／根拠 |
|---|---|---|---|---|---|---|---|---|---|
| LCD-SCLK | DISP-01 | SCLK | Output | TBD | Safe／inactiveはTBD | TBD | SPI speed／modeはTBD | TOUCH-01候補 | LCDの識別が必要 |
| LCD-MOSI | DISP-01 | MOSI | Output | TBD | Safe／inactiveはTBD | TBD | SPI speed／modeはTBD | TOUCH-01候補 | LCDの識別が必要 |
| LCD-MISO | DISP-01 | MISO | Input | TBD | TBD | TBD | SPI speed／modeはTBD | TOUCH-01候補 | 未使用の可能性あり。要確認 |
| LCD-CS | DISP-01 | Chip select | Output | TBD | Inactive | TBD | Active polarityはTBD | なし | Output設定前にinactiveにする |
| LCD-DC | DISP-01 | Data／command | Output | TBD | SafeはTBD | TBD | Device固有 | なし | LCDの識別が必要 |
| LCD-RST | DISP-01 | Reset | Output | TBD | Reset／safeはTBD | TBD | Pulse timingはTBD | なし | 起動時glitchを防ぐ |
| LCD-BL | DISP-01 | Backlight | Output／PWMはTBD | TBD | Off推奨 | TBD | Polarity／current pathはTBD | なし | 回路上の根拠なしにLED電流を直接driveしない |
| TOUCH-SIGNAL | TOUCH-01 | GPIO／I2C／SPI信号 | TBD | TBD | TBD | TBD | TBD | LCD bus候補 | 正確な信号へ置き換える |
| TOUCH-IRQ | TOUCH-01 | Interrupt | Input | TBD | TBD | TBD | Edge／levelはTBD | なし | Controllerの識別が必要 |
| ACCEL-SDA | ACCEL-01 | I2Cの場合のSDA | Bidirectional | TBD | High／open-drain | External pull-upはTBD | Address／speedはTBD | ENV-01候補 | Interface未選定 |
| ACCEL-SCL | ACCEL-01 | I2Cの場合のSCL | Bidirectional | TBD | High／open-drain | External pull-upはTBD | SpeedはTBD | ENV-01候補 | Interface未選定 |
| ACCEL-IRQ | ACCEL-01 | Interrupt | Input | TBD | TBD | TBD | Edge／levelはTBD | なし | 任意。要確認 |
| ENV-SDA | ENV-01 | I2Cの場合のSDA | Bidirectional | TBD | High／open-drain | External pull-upはTBD | Address／speedはTBD | ACCEL-01候補 | Interface未選定 |
| ENV-SCL | ENV-01 | I2Cの場合のSCL | Bidirectional | TBD | High／open-drain | External pull-upはTBD | SpeedはTBD | ACCEL-01候補 | Interface未選定 |
| SERVO-PWM | SERVO-01 | PWM control | Output | TBD | Disabled／safe | TBD | Period／pulseはTBD | なし | 安全文書が必要 |
| UART-TX | Pi link／debug | TX | Output | TBD | SDK defaultはTBD | TBD | 115200 8N1候補 | Boot log候補 | USB-UART経路を確認する |
| UART-RX | Pi link／debug | RX | Input | TBD | SDK defaultはTBD | TBD | 115200 8N1候補 | Flashing候補 | USB-UART経路を確認する |

正確なmoduleが使用しない信号は削除し、不足しているreset、enable、address-select、interrupt、power-control信号はすべて追加する。

## Bus計画

| Bus | 候補device | 状態 | 不足している根拠 |
|---|---|---|---|
| USB serial／UART | Raspberry Pi | Architecture上の候補として確定 | 正確なboard経路とPi上のdevice名はTBD |
| SPI display bus | LCD、場合によりtouch | TBD | 正確なLCD／touch controller |
| I2C sensor bus | Accelerometer、environment sensor | TBD | 正確なdevice、address、pull-up |
| PWM／timer | Servo | Required | 正確なservoと安全なpulse範囲 |

## 競合check

- [ ] 割り当てたpinがmodule flash用に予約されていない
- [ ] Outputがbootstrap要件と競合しない
- [ ] UART flashingとboot logを引き続き利用できる
- [ ] Input-only制約を守っている
- [ ] 共有SPI上の各deviceに個別CSがある
- [ ] I2C deviceのaddressが一意、または明示的な対策がある
- [ ] Moduleのpull-upを並列合成した実効抵抗が有効範囲内である
- [ ] ESP32の電源投入前に外部moduleがESP32 pinをdriveしない
- [ ] Resetとbacklight lineが安全な状態で起動する
- [ ] Servo PWMがdisabledまたは承認済みの安全状態で起動する

## Firmwareとの同期

割り当てを承認した後、次を行う。

1. 配線revisionへ識別子を付ける。
2. 一致するfirmware board configuration IDを追加する。
3. `firmware/esp32`内でpinを一元管理する。
4. 起動時にboard configuration IDを出力する。
5. 可能であれば、重複割り当てに対する起動checkを追加する。
6. GPIOを変更するPull Requestでは、この文書の更新を必須にする。

## Revision履歴

| 日付 | Revision | 変更 |
|---|---|---|
| 2026-07-27 | 0 | 信号inventoryを作成。実GPIO割り当てはすべて引き続きTBD |
