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
| LCD-CS | DISP-01 | Chip select | Output | GPIO22 | 起動時floating→firmware初期化前は不定 | **外部10kΩ pull-up推奨**（active-low CSをfirmware初期化前もinactive＝Highに保つため） | Active-low（要現物のpolarity確認） | なし | Output設定前にinactiveにする。Pull-up未実装の場合、起動直後の数十ms間bus contentionのriskがある |
| LCD-DC | DISP-01 | Data／command | Output | GPIO17 | floating | 外部pull不要（WROOM-32Dのため使用可） | Device固有（要現物確認） | なし | WROOM/SOLO-1専用pin。今回のmoduleはWROOM-32Dのため使用可 |
| LCD-RST | DISP-01 | Reset | Output | GPIO16 | floating | **外部pull-up推奨**（reset非activeをfirmware初期化前も既定にするため） | Pulse timingは現物確認後に決定 | なし | 起動時glitchを防ぐ。Firmwareが最初にHighを出力するまでの間もHighに保つ設計が望ましい |
| LCD-BL | DISP-01 | Backlight | Output（PWM調光は将来検討） | GPIO4 | **不定**。ESP32のGPIO4はreset時にinput（内部weak pull-downあり）で、driveされた状態にはならない。ただし内部weak pullは外部回路に対して弱く、backlight回路の入力仕様によっては点灯しうる。firmwareまたは外部pullが確定させるまでOffを保証しない | **外部pull-down推奨**（backlightをfirmware初期化前もOffに確定させるため）。極性は現物確認後に決定 | 現状はdigital on/off。将来PWM調光も可能なpinを選定 | なし | 回路上のLED電流経路はmodule内蔵に依存。直接大電流をdriveしない（module側で電流制限されている前提、現物確認要） |
| TOUCH-CS | TOUCH-01 | Chip select（touch controller用、LCD-SPIバスを共有） | Output | GPIO21 | 起動時floating→不定 | **外部10kΩ pull-up推奨**（LCD-CSと同じ理由） | Active-low（要現物のpolarity確認） | DISP-01とSCLK／MOSI／MISOを共有 | **Touch controllerは2026-08-13に`XPT2046`と確定した**（現物chip刻印。`hardware-bom.md` TOUCH-01）。polarityは`XPT2046`のdatasheetで確認する |
| TOUCH-IRQ | TOUCH-01 | Interrupt（touch検出） | Input | GPIO34 | 入力専用、floating | 外部pull-up推奨（一般的なtouch controllerはactive-low IRQ。要現物確認） | Edge／level要確認 | なし | Input-only pin。Output不可のため他用途に転用できない |
| ACCEL-SDA | ACCEL-01 | I2C SDA | Bidirectional | GPIO25 | floating（open-drain想定） | 外部4.7kΩ pull-up（**ADXL345モジュールは`01C`＝10 kΩのpull-upを4個搭載していることを2026-08-13に現物確認した。**ただし**どのpinへ付くかはパターンを追っていないため未確定**であり、実効抵抗の計算前に配線を確認する） | 400kHz(Fast-mode)を想定、要実測 | ENV-01と共有 | ADXL345はI2C／SPI選択式。Interface選択jumperの現物確認が必要（`hardware-bom.md` ACCEL-01） |
| ACCEL-SCL | ACCEL-01 | I2C SCL | Bidirectional | GPIO26 | 同上 | 同上 | 同上 | ENV-01と共有 | 同上 |
| ACCEL-IRQ | ACCEL-01 | Interrupt（tap／free-fall検出） | Input | GPIO35 | 入力専用 | **外部pull要確認**（`HW-TBD-004`）。**ICの事実:**ADXL345のINT1/INT2は**push-pull固定**であり、設定で切り替えられない（`Both interrupt pins are push-pull, low impedance pins`。Rev. G page 19）。polarityは`DATA_FORMAT` register（`0x31`）の`INT_INVERT` bitで選び、**同registerのreset値が`00000000`であるためICの既定はactive-highである**（Rev. G Table 19 page 23、page 27）。**旧記載の「push-pull／open-drainを設定可能」はICの事実として誤りであり、2026-08-12に訂正した**（Revision 9）。**module levelは別である。**M-06724のboard上でINT pinがheaderへ直結しているか（直列抵抗、level shift、引き出しの有無）を示す資料が無いため、**外部pullの要否とheaderで観測されるpolarityは現物確認まで確定しない。ICがpush-pullであることからmoduleの配線条件を導かない**（[tbd-register HW-TBD-004](tbd-register.md)） | Edge想定 | なし | ADXL345のtap／free-fall検出hardwareを軽打／持ち上げ判定に使う場合に使用（`hardware-bom.md` ACCEL-01の採用理由） |
| ENV-SDA | ENV-01 | I2C SDA | Bidirectional | GPIO25（ACCEL-01と共有） | 同上 | 同上 | 同上 | ACCEL-01と共有 | BME280はI2C／SPI選択式。選択jumperの現物確認が必要（`hardware-bom.md` ENV-01） |
| ENV-SCL | ENV-01 | I2C SCL | Bidirectional | GPIO26（ACCEL-01と共有） | 同上 | 同上 | 同上 | ACCEL-01と共有 | 同上 |
| SERVO-PWM | SERVO-01 | PWM control | Output | GPIO27 | **不定**。ESP32のGPIO27はreset時にhigh-Z（output disable、input disable）であり、Lowにdriveされる保証はない。**Lowと仮定しない。**外部pull-downが確定させるまで、servoは不定pulseを受けうる | **外部pull-down必須**（推奨ではない）。high-Z期間中もLowを保証する唯一の手段であり、これがないとPWM driver初期化前にservoが動きうる。詳細は`servo-safety-limits.md`。**部品は`hardware-bom.md`の`RES-PULL-01`。一部の抵抗値は入手済みだが（10 kΩと4.7 kΩが各1袋。2026-08-08着荷）、必要な本数と抵抗値が未選定であり、手元の2種で足りるとは限らない** | 50Hz、pulse幅は`servo-safety-limits.md`で規定する制限に従う | なし | Strapping pinでもflash pinでもない。起動時とdriver故障時の状態は`tbd-register.md` HW-TBD-019で引き続き検討する |
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
| I2C sensor bus | Accelerometer（ADXL345）、environment sensor（BME280） | GPIO25（SDA）／GPIO26（SCL）に確定 | 両moduleのinterface選択jumper（I2C側になっているか）の現物確認、実効pull-up抵抗の計算 |
| PWM／timer | Servo（SG90） | GPIO27に確定 | `servo-safety-limits.md`のpulse幅制限確定、起動時安全状態のreview |

## 競合check

- [x] 割り当てたpinがmodule flash用に予約されていない（GPIO6-11を使用していないことを確認済み）
- [x] Outputがbootstrap要件と競合しない（GPIO0/2/5/12/15を一切使用していない）
- [x] UART flashingとboot logを引き続き利用できる（GPIO1/3を変更していない）
- [x] Input-only制約を守っている（GPIO34/35/36は入力専用として使用。GPIO36はADC-3V3、outputへ転用しない）
- [x] ADC測定pinを予約済みで、ADC2をWi-Fi併用下で使っていない（GPIO32/33/36はすべてADC1）
- [ ] 5 Vと3.3 V railのADC入力に分圧器が実装され、ADC定格3.3 Vを超えない（分圧比1/2を規定済み。実配線が存在しないため未検証）
- [x] 共有SPI上の各deviceに個別CSがある（LCD: GPIO22、Touch: GPIO21）
- [x] I2C deviceのaddressが一意、または明示的な対策がある（ADXL345既定0x53、BME280既定0x76／0x77で衝突なし。要現物のaddress pin設定確認）
- [ ] すべての外部pull-upが3.3Vへ接続され、5Vへ接続されていない（`電圧domain`節で規定済み。ただし実配線が存在しないため未検証）
- [ ] Moduleのpull-upを並列合成した実効抵抗が有効範囲内である（**ADXL345は`01C`＝10 kΩ×4を搭載していることを2026-08-13に確認したが、どのpinへ付くかは未確定。**BME280は`J1`／`J2`のはんだジャンパで4.7 kΩの接続を選ぶ設計で、**半田の有無が光学的に判別できず未確定**。導通確認は[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)の範囲。**両方が確定するまで計算しない**）
- [ ] MSP2807のlogic IOが3.3Vで動作することを現物で確認した（VCC 3.3–5V対応だがlogic IOは3.3V TTL。`power-budget.md`参照）
- [ ] ESP32の電源投入前に外部moduleがESP32 pinをdriveしない（未検証、実機電源offでの導通checkが必要）
- [ ] Resetとbacklight lineが安全な状態で起動する（LCD-RST/LCD-CSへの外部pull-up実装が前提。未実装のため要対応。[`HW-TBD-032`](tbd-register.md)で追跡する）
- [ ] Servo PWMがdisabledまたは承認済みの安全状態で起動する（GPIO27はreset時high-Zであり、外部pull-downを**必須**とした。実装・検証とも未了。`tbd-register.md` HW-TBD-019と連動、未解決）

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
| 2026-08-22 | 16 | [PR #174](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/174)の自動reviewの指摘を反映した。**USB OTG cableを`未購入`としていた記述が2箇所残っていた**（`追加部品`行と`USB serial（Pi link）`行）。**`CABLE-PI-LINK-01`は2026-08-22に手持ちで充当と確定し購入待ちリストから外している**（正は[hardware-bom.md](hardware-bom.md)の`CABLE-PI-LINK-01`）。両方を更新した | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) |
