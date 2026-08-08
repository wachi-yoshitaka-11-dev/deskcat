# GPIO Assignment

> 状態: Blocked — 実機での電源off導通check、touch controller型番の現物確認、servo起動時状態の安全review待ち
> 正本とする情報: ESP32 boardのpin割り当て

## 割り当て規則

- 正確なESP-WROOM-32D開発ボード（秋月M-13628）と搭載moduleの文書を使用する。
- flash、bootstrapping、USB-UART、board LED、使用制限のあるpinを考慮する。
- すべてのmoduleについて、電圧と起動時drive stateを確認する。
- 物理信号ごとに一行を使用する。
- Tutorialまたは類似boardのGPIO番号をコピーしない。
- Firmwareのpin定数は、この文書から生成するか、この文書と手動で同期させる。

## Board識別情報

| 項目 | 値 | 根拠 |
|---|---|---|
| Board family | ESP-WROOM-32D開発ボード（秋月電子 M-13628）。Espressif ESP32-DevKitC V4 wide版（38pin、flash pin露出タイプ）のpin配置に相当 | [hardware-bom.md](hardware-bom.md) MCU-01、現物写真（`D0`–`D3`／`CMD`／`CLK`相当のpin露出）、基板裏面silkscreen「`ESP32_DevkitC_V4`」 |
| 正確なboard revision | 基板自体にrevision表示なし（秋月オリジナル基板のため） | 現物確認済み（`hardware-bom.md` Revision履歴3） |
| 搭載ESP32 module suffix | ESP-WROOM-32D | 購入履歴（秋月M-13628商品名）、`hardware-bom.md` |
| 公式回路図revision | 秋月商品ページ添付データシート、および参考として[Espressif ESP32-DevKitC V4 pinout](https://docs.espressif.com/projects/esp-idf/en/v5.1/esp32/hw-reference/esp32/get-started-devkitc.html)。秋月独自基板のため、pin配列がEspressif公式と完全一致するとは限らない点に注意（要現物pin表記との対応確認） | 秋月商品ページ |
| Firmware board configuration ID | TBD | Toolchain bring-up時（#5）に定義する |

## ESP32の使用制限pin（Espressif公式資料より、この基板に適用）

| 区分 | GPIO | 制約 |
|---|---|---|
| Flash通信専用（**使用禁止**） | 6, 7, 8, 9, 10, 11（`CLK`／`D0`／`D1`／`D2`／`D3`／`CMD`） | 内蔵SPI Flashとの通信に使用。外部回路から絶対に使用しない |
| Strapping pin（起動modeを決定。用途を厳選） | 0, 2, 5, 12, 15 | GPIO0: boot button。GPIO2: download mode判定。GPIO12(MTDI): flash電圧選択（Highだと起動しない可能性）。GPIO15(MTDO): boot logのsilence制御。今回の割り当てでは**いずれも使用しない**（安全側） |
| UART0（Flashingとboard上USB-UARTブリッジ専用） | 1（TX）, 3（RX） | Pi linkとfirmware flashingで共用。Pi接続時はPC切断が前提 |
| Input-only（出力不可） | 34, 35, 36（VP）, 39（VN） | 純粋なinput信号（interrupt、ADC）にのみ割り当て可 |
| WROOM/SOLO-1専用（WROVERでは予約） | 16, 17 | 今回のmoduleはESP-WROOM-32Dのため使用可 |

## 信号inventory

| Signal ID | Device | 信号 | ESP32側の方向 | GPIO | Boot state | Pull | Bus設定 | 共有先 | 制約／根拠 |
|---|---|---|---|---|---|---|---|---|---|
| LCD-SCLK | DISP-01 | SCLK | Output | GPIO18 | 起動時floating（input）。CSがinactiveの間はbus上で無害 | 外部pull不要 | VSPI、SPI mode要確認（ILI9341は一般にMode0）。速度は実測で確認 | TOUCH-01と共有 | ESP32 VSPIの既定CLK pin。Flash／strapping pinではない |
| LCD-MOSI | DISP-01 | MOSI | Output | GPIO23 | 同上 | 外部pull不要 | 同上 | TOUCH-01と共有 | ESP32 VSPIの既定MOSI pin |
| LCD-MISO | DISP-01 | MISO | Input | GPIO19 | 同上 | 外部pull不要 | 同上 | TOUCH-01と共有 | ILI9341自体はMISO未使用の可能性が高い（要現物確認）。Touch controller（想定`XPT2046`系、`hardware-bom.md`参照）の読み取りに使用 |
| LCD-CS | DISP-01 | Chip select | Output | GPIO22 | 起動時floating→firmware初期化前は不定 | **外部10kΩ pull-up推奨**（active-low CSをfirmware初期化前もinactive＝Highに保つため） | Active-low（要現物のpolarity確認） | なし | Output設定前にinactiveにする。Pull-up未実装の場合、起動直後の数十ms間bus contentionのriskがある |
| LCD-DC | DISP-01 | Data／command | Output | GPIO17 | floating | 外部pull不要（WROOM-32Dのため使用可） | Device固有（要現物確認） | なし | WROOM/SOLO-1専用pin。今回のmoduleはWROOM-32Dのため使用可 |
| LCD-RST | DISP-01 | Reset | Output | GPIO16 | floating | **外部pull-up推奨**（reset非activeをfirmware初期化前も既定にするため） | Pulse timingは現物確認後に決定 | なし | 起動時glitchを防ぐ。Firmwareが最初にHighを出力するまでの間もHighに保つ設計が望ましい |
| LCD-BL | DISP-01 | Backlight | Output（PWM調光は将来検討） | GPIO4 | floating（Low相当、非点灯） | 外部pull不要 | 現状はdigital on/off。将来PWM調光も可能なpinを選定 | なし | 回路上のLED電流経路はmodule内蔵に依存。直接大電流をdriveしない（module側で電流制限されている前提、現物確認要） |
| TOUCH-CS | TOUCH-01 | Chip select（touch controller用、LCD-SPIバスを共有） | Output | GPIO21 | 起動時floating→不定 | **外部10kΩ pull-up推奨**（LCD-CSと同じ理由） | Active-low（要現物のpolarity確認） | LCD-01とSCLK／MOSI／MISOを共有 | Touch controller型番は現物確認待ち（`hardware-bom.md` TOUCH-01） |
| TOUCH-IRQ | TOUCH-01 | Interrupt（touch検出） | Input | GPIO34 | 入力専用、floating | 外部pull-up推奨（一般的なtouch controllerはactive-low IRQ。要現物確認） | Edge／level要確認 | なし | Input-only pin。Output不可のため他用途に転用できない |
| ACCEL-SDA | ACCEL-01 | I2C SDA | Bidirectional | GPIO25 | floating（open-drain想定） | 外部4.7kΩ pull-up（ADXL345モジュールのon-board pull-upと合成する場合は実効抵抗を確認） | 400kHz(Fast-mode)を想定、要実測 | ENV-01と共有 | ADXL345はI2C／SPI選択式。Interface選択jumperの現物確認が必要（`hardware-bom.md` ACCEL-01） |
| ACCEL-SCL | ACCEL-01 | I2C SCL | Bidirectional | GPIO26 | 同上 | 同上 | 同上 | ENV-01と共有 | 同上 |
| ACCEL-IRQ | ACCEL-01 | Interrupt（tap／free-fall検出） | Input | GPIO35 | 入力専用 | 外部pull要確認（ADXL345のINT1/INT2はpush-pull／open-drainを設定可能。既定active-highの想定） | Edge想定 | なし | ADXL345のtap／free-fall検出hardwareを軽打／持ち上げ判定に使う場合に使用（`hardware-bom.md` ACCEL-01の採用理由） |
| ENV-SDA | ENV-01 | I2C SDA | Bidirectional | GPIO25（ACCEL-01と共有） | 同上 | 同上 | 同上 | ACCEL-01と共有 | BME280はI2C／SPI選択式。選択jumperの現物確認が必要（`hardware-bom.md` ENV-01） |
| ENV-SCL | ENV-01 | I2C SCL | Bidirectional | GPIO26（ACCEL-01と共有） | 同上 | 同上 | 同上 | ACCEL-01と共有 | 同上 |
| SERVO-PWM | SERVO-01 | PWM control | Output | GPIO27 | floating（Low相当、駆動なし） | **外部pull-down推奨**（起動時にPWM出力が不定にならないようにするための安全対策。詳細は`servo-safety-limits.md`） | 50Hz、pulse幅は`servo-safety-limits.md`で規定する制限に従う | なし | Strapping pinでもflash pinでもない。起動時とdriver故障時の状態は`tbd-register.md` HW-TBD-019で引き続き検討する |
| UART-TX | Pi link／debug | TX | Output | GPIO1（固定、board上USB-UARTブリッジ） | SDK既定（起動logを出力） | 変更不可（chip内蔵UART0） | 115200 8N1（候補、`esp32-pi-protocol.md`で最終確定） | Boot log | Board上のUSB-UARTブリッジ経由。Flashingと共用のため、Pi接続時はPCとの同時接続を想定しない |
| UART-RX | Pi link／debug | RX | Input | GPIO3（固定） | 同上 | 変更不可 | 同上 | Flashing | 同上 |

正確なmoduleが使用しない信号は削除し、不足しているreset、enable、address-select、interrupt、power-control信号はすべて追加する。

## Bus計画

| Bus | 候補device | 状態 | 不足している根拠 |
|---|---|---|---|
| USB serial／UART | Raspberry Pi | GPIO1／GPIO3に確定（board上USB-UARTブリッジ経由） | Pi上のdevice名（`/dev/ttyUSB*`等）は#8で確認 |
| SPI display bus | LCD（MSP2807／ILI9341）、touch（同module） | GPIO18／23／19（SCLK／MOSI／MISO）＋CS個別（LCD: GPIO22、Touch: GPIO21）に確定 | Touch controller型番の現物確認、実際のSPI mode／速度の実測 |
| I2C sensor bus | Accelerometer（ADXL345）、environment sensor（BME280） | GPIO25（SDA）／GPIO26（SCL）に確定 | 両moduleのinterface選択jumper（I2C側になっているか）の現物確認、実効pull-up抵抗の計算 |
| PWM／timer | Servo（SG90） | GPIO27に確定 | `servo-safety-limits.md`のpulse幅制限確定、起動時安全状態のreview |

## 競合check

- [x] 割り当てたpinがmodule flash用に予約されていない（GPIO6-11を使用していないことを確認済み）
- [x] Outputがbootstrap要件と競合しない（GPIO0/2/5/12/15を一切使用していない）
- [x] UART flashingとboot logを引き続き利用できる（GPIO1/3を変更していない）
- [x] Input-only制約を守っている（GPIO34/35は入力専用として使用）
- [x] 共有SPI上の各deviceに個別CSがある（LCD: GPIO22、Touch: GPIO21）
- [x] I2C deviceのaddressが一意、または明示的な対策がある（ADXL345既定0x53、BME280既定0x76／0x77で衝突なし。要現物のaddress pin設定確認）
- [ ] Moduleのpull-upを並列合成した実効抵抗が有効範囲内である（ADXL345／BME280双方のon-board pull-up有無を現物確認後に計算）
- [ ] ESP32の電源投入前に外部moduleがESP32 pinをdriveしない（未検証、実機電源offでの導通checkが必要）
- [ ] Resetとbacklight lineが安全な状態で起動する（LCD-RST/LCD-CSへの外部pull-up実装が前提。未実装のため要対応）
- [ ] Servo PWMがdisabledまたは承認済みの安全状態で起動する（`tbd-register.md` HW-TBD-019と連動、未解決）

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
| 2026-08-05 | 1 | Board識別情報を確定（ESP-WROOM-32D開発ボード、秋月M-13628）。Espressif公式ESP32-DevKitC V4のpin制約（flash pin6-11、strapping pin0/2/5/12/15、input-only pin34/35/36/39、WROOM専用pin16/17）を反映し、全信号にGPIOを割り当てた。LCD/Touch CSとLCD RSTには起動時safe state確保のため外部pull-up追加を推奨。競合checkのうち実機確認が必要な項目（電源off導通、pull-up実効抵抗、servo PWM起動時状態）は未完了のまま残した |
