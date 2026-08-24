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

| ID | 優先度 | 不足している情報／判断 | 必要な根拠 | 妨げる対象 | 対応Issue | Owner | 状態 |
|---|---|---|---|---|---|---|---|
| HW-TBD-002 | P0 | **残: pin配列と電源pinの現物照合、およびboard上のdecoupling実装の確認。****2026-08-13の現物確認で判明した重要な事実**: **board上に3.3V LDO（`U1`、刻印`662K` / `UMW S4T`）が実装されている。**moduleが「VCC 3.3–5 V」を受けられる理由がこれである。ほかに`U2`＝`XPT2046`（`HW-TBD-003`）、backlight回路の`R5`＝6.8 Ω／`R6`＝1 kΩ／`Q1`＝`J3Y`／`J1`＝オープン（`HW-TBD-024`）。**pinヘッダのラベルは裏面で左→右に`T_IRQ T_DO T_DIN T_CS T_CLK SDO(MISO) LED SCK SDI(MOSI) DC RESET CS GND VCC`であり、datasheetの1=VCC…14=T_IRQ とは逆順である**（`VCC`が反対端にある）。**silkの解像度表記は`2.8 TFT SPI 240X320 V1.2`**で、パネルは`HSD028309 A2`。decouplingは部品が実装されているが**容量表記が無く値は不明**。MSP2807（ILI9341）は**2026-08-08に着荷済み**である。**decouplingは`HW-TBD-029`から2026-08-12に移した。**メーカーdatasheetに記載が無いことは同行のcloseで確定しており、**残るのは現物でしか決まらない部分である** | 現物確認＋[メーカーdatasheet](https://akizukidenshi.com/goodsaffix/msp2807.pdf)＋[LCD Wiki](http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807)。**datasheetから確定済み**（2026-08-10）: 14pinのinterface定義（`VCC` `GND` `CS` `RESET` `DC/RS` `SDI(MOSI)` `SCK` `LED` `SDO(MISO)` `T_CLK` `T_CS` `T_DIN` `T_DO` `T_IRQ`）、`ILI9341`、320×240、4-wire SPI、VCC 3.3–5V、Logic IO 3.3V(TTL)、動作温度-20〜60℃。**現物ではこの並びと一致するかを見る。**選定の根拠: [hardware-bom.md](hardware-bom.md) DISP-01。**同じ現物moduleで`HW-TBD-024`（backlight回路から得る、耐えられる電流の上限）も確認できる。現物を扱う機会は限られるためまとめて行う** | LCD driver、SPI pin、電源 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1) | Human | Open |
| HW-TBD-004 | P0 | **2026-08-13の現物確認で次が確定した。**(1) **IC刻印は`345B` / `#727` / `750B` / `PHIL`**であり、`345B`はAnalog DevicesのADXL345の刻印である。**これによりICの根拠が、silkと購入履歴からIC自身の刻印になった。**(2) **regulatorは非搭載**（IC＋`C1`×2＋抵抗のみで、3端子部品が無い）。(3) **裏面の半田ジャンパ2箇所はいずれもオープン。**(4) **搭載pull-upは`01C`（EIA-96で10 kΩ、1%）が4個**、ほかに`R2`＝`0`（0 Ωジャンパ）1個。(5) pin列は上`CS Vs GND VDD`、下`INT1 INT2 SDO SDA SCL`で、**`Vs`と`VDD`が別pinに出ている。****ただしこれはheaderのラベルであって、board上の配線ではない。**各pinがICの`VS`／`VDD I/O`へ直結しているか、直列抵抗やlevel shiftが入るかは**パターンを追っていないため未解決である。**「2系統電源を個別に受ける設計である」と断定しない。(6) local decouplingは`C1`が2個実装されているが、**容量表記が無く値は不明**。**残: 実装済みI2C addressと`moduleへ入れてよい電圧`。**addressは`SDO`の接続で決まるが**基板上で固定されておらず、配線時に決まる**。`moduleへ入れてよい電圧`はregulator非搭載が確定したため**ICの`VS`／`VDD I/O`がheaderへ直結している可能性が高いが、直列抵抗やlevel shiftの有無をパターンで追っていないため断定しない**。module／ICはADXL345（秋月 M-06724）と特定済み。**module boardの資料が現在入手できない**（秋月 M-06724の商品ページは`gM-06724`／`g106724`とも404。2026-08-10確認）ため、jumper構成をIC datasheetから推定せず現物で読む。**`moduleへ入れてよい電圧`は2026-08-12に明示追加した。**[hardware-bom.md](hardware-bom.md)と[power-budget.md](power-budget.md)が3.3V給電を確定形で書いていたが、**ICの定格3.6Vはboardの許容値ではない**（regulatorやlevel shiftの有無で変わる）ため、3.3Vを唯一の候補としたうえで確定はこの行に依存させた。**5V直結の禁止は別の主張として維持する。**これは「moduleがregulatorを持たなければ5VがICへ直接掛かる」ことを否定できないための**確認前の安全規則**であって、IC定格からmoduleの許容入力電圧を導いたものではない | 現物のjumper／address pin設定の確認、基板上のregulatorの有無の確認、moduleへ入れてよい電圧の確定、および**board上に実装済みのdecoupling capacitorの確認**（`HW-TBD-029`から2026-08-12に移した。ICへの指定値は同行のcloseで確定しているが、**外付けが要るかはboardの実装でしか決まらない**。値の正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`Local decoupling`節）。ICの根拠は[Analog Devices ADXL345 datasheet](https://www.analog.com/media/en/technical-documentation/data-sheets/adxl345.pdf)（**Rev. G。2026-08-12にブラウザで取得して確認した**。**「`analog.com`へは接続できない」という旧記載は誤りであったため同日訂正した**。到達できないのはCLI clientだけである。読み取った内容は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`ICの値`）。**ただしdatasheetを得てもこの行は解決しない。**jumper構成、実装済みaddress、regulatorの有無、moduleへ入れてよい電圧は、いずれもICの資料では決まらない。特定の根拠: [hardware-bom.md](hardware-bom.md) ACCEL-01 | Accelerometer driverとしきい値、**ADXL345への給電電圧の確定** | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1) | Human | Open（範囲を縮小） |
| HW-TBD-005 | P0 | **残件は解決した。**`J1`／`J2`／`J3`は**3つとも開放**である（2026-08-22に導通確認で実測。**電源を接続しない状態でpin間の抵抗を測った。**測定値と判定方法の正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`現物の実装状態を実測で確定させた（2026-08-22）`。**ここへ再掲しない**）。**説明書が示す工場出荷状態と一致する。**2026-08-13の接写では光学的に判別できず（パッドが錫めっきで鏡面）、**「全てオープンである可能性が高いが断定しない」としていた点が実測で決まった。****`SDO`が基板上でどこにも固定されていないことも実測で裏付けた。**したがってI2Cアドレスは配線時に決まり、基板側に既定値は無い。module／ICはBosch BME280（秋月 K-09421）と特定済み。**jumperの意味とaddressの決まり方は一次資料で確定している** | **[AE-BME280製品説明書 v1.1](https://akizukidenshi.com/goodsaffix/AE-BME280_manu_v1.1.pdf)（2015-06-02）で確定**（2026-08-10）: `J1`＝I2C時のSDA用プルアップ(4.7kΩ)選択、`J2`＝同SCL用、`J3`＝**I2C時にはんだジャンパする**（SPI 4W／3W時は`J1`〜`J3`すべてオープン）。I2Cアドレスは`0x76`（`SDO`→GND、既定）／`0x77`（`SDO`→VDD）。6pin SIPは1=VDD, 2=GND, 3=CSB, 4=SDI, 5=SDO, 6=SCK。VDDとVDDIOは基板上で接続済み。Chip ID register `0xD0`、reset値`0x60`。ICの根拠は[Bosch BME280 datasheet Rev 1.24](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf)。特定の根拠: [hardware-bom.md](hardware-bom.md) ENV-01 **判定は2026-08-22に完了した。**導通確認はDT830Bの`Ω`の`20k`レンジで行い、**ゼロ点を確認したうえで測った。** **2026-08-22に、closeまでに要ることを実装と確認まで含めて確定した。****この行はそれまで一次資料の内容しか持たず、実装作業を含んでいなかった。**`HW-TBD-032`が選定・購入・実装・確認を`必要な根拠`に含めているのと構造を揃える。**closeには5項目が要る**（実装状態の確定、`J3`のはんだ付け、実効pull-upの計算と`J1`／`J2`の判断、I2Cアドレスの配線、実装後の動作確認）。**内容と状態の正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`closeまでに要ること（実装と確認を含む）`であり、ここへ再掲しない。****済んでいるのは実装状態の確定だけである。** | Environment driverとbus計画 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1) | Human | Open（**残件の`J1`／`J2`／`J3`の半田の有無は2026-08-22に実測で確定した。3つとも開放である。****この状態から実装作業と未決の判断が出る。****(1) I2Cで使うには`J3`のはんだ付けが要る**（説明書が定める。開放では`CSB`が`VDD`へ繋がらない）。**(2) module搭載の4.7 kΩプルアップは繋がっていないため、busのpull-upをこのmoduleから供給しない。**`J1`／`J2`をはんだ付けするかは、[gpio-assignment.md](gpio-assignment.md)の`I2C sensor bus`で**bus全体の実効pull-upを計算してから決める。まだ決めていない。****(3) アドレスは配線で決める**（`SDO`→GNDで`0x76`、`SDO`→VDDで`0x77`）。**通電はしていない。****closeの手順（正本文書への反映と関連項目の解除）を通すまで行は残す**。**2026-08-22にcloseまでに要る5項目を確定した**（正は[sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`closeまでに要ること（実装と確認を含む）`）。**済んでいるのは実装状態の確定だけである。****`J3`のはんだ付けがI2Cで動かすための必須作業であり未実施。****`J1`／`J2`の判断はADXL345側のpin接続の確認が先に要るためまだできない。****アドレスの配線と動作確認も未実施**） |
| HW-TBD-007 | P0 | **残: 5 V ingressの変換部品の購入と、connector定格の検証。**電源modelはM-12001（5 V／3 A）と確定し、現物も2026-08-08に着荷済み。ただしMicro-Bオスplugをbreadboardへ引き込む変換部品が未購入である（候補: 秋月 g110972、定格1ピン1.5 A）。上限は経路上の全部品の定格の最小値の80%（候補構成の最弱部品が1.5 Aなら1.2 A以下。未確定の経路要素が残るため確定値ではない。HW-TBD-021／022）であり、**判定は定常電流で行う**（connector定格は熱の制限のため）。**段階A（Pi単体起動）と段階B-1（ESP32をPCのUSBから給電）はこの行に依存せず進められる。ただし段階Aは探索的な通電であって、Piの電源経路の受け入れではない。**M-12001の5 VがRaspberry Pi公式の要求する5.1 Vを満たすかはこの行で未判断であり、判定に使う最低電圧も`HW-TBD-028`(a)として未確定である。**段階Aが正常に起動したことをもって合格としない。****段階B-2（周辺module3点から3.3 V側の定常電流を測る）も同様だが、B-2自体が共通条件のHW-TBD-025と、経路別のHW-TBD-023（B-2a）またはHW-TBD-024（B-2b）にBlockedされている。**なお**合成給電（段階C）はこの部品が要る。****2026-08-12に電源modelそのものについて1件判明した。**Raspberry Pi公式documentationは全modelが**5.1 V**供給を要求すると述べているが、選定済みのM-12001は**5 V**である。**この差の可否はまだ判断していない。**判定に使う最低電圧が`HW-TBD-028`(a)として未確定であり（一次資料に無いことを確認済み）、**電圧の余裕を評価する根拠が無い**ためである。**部品の選定は変更していない** | 変換部品の選定＋実測。確定済み部分の根拠: [hardware-bom.md](hardware-bom.md) PSU-PI-01／PSU-SERVO-01、[power-budget.md](power-budget.md#5-v-ingress物理的な引き込み経路)。**5.1 Vと5 Vの差についての根拠**: [Raspberry Pi公式documentation](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html)の`Power supply`（`All models require a 5.1V supply`。2026-08-12取得）。**同documentationは許容範囲を示さないため、5 Vが範囲内かどうかもこの資料からは決まらない** | 初回統合通電 | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Human | Open（範囲を縮小。**2026-08-12に電源modelの供給電圧について未判断の論点が1件加わった**） |
| HW-TBD-008 | P0 | GPIO割り当て | Board／module回路図＋競合review | すべてのhardware driver | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2) | Joint | **002、004、005によりBlocked**（**001と003は2026-08-15にclose。006と029も解決済み**）。下書きは[gpio-assignment.md](gpio-assignment.md)にあるが、競合checkに未完了項目が残る。**001のcloseでpin配列の前提は成立した**が、残る3行はmodule側のjumper・address・decouplingの現物確認である |
| HW-TBD-009 | P0 | 電源予算とbackfeed review | 部品電流＋回路図＋測定計画 | Servoと全体統合 | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Joint | **002、004、005**、007、**021、022、023、024、025**、および**028、030、034**によりBlocked（**001と003は2026-08-15にclose。006と029も解決済み。029は2026-08-12にclose**。下書きは[power-budget.md](power-budget.md)にあるが、実測値が皆無。021と022が解決するまで合成給電（段階C）へ進めず、**合成給電の実測と受け入れが始まらない**。段階Aと段階B-1は021／022に依存せず実施できる。**段階B-2は021／022には依存しないが、共通条件の025と、経路別の023（B-2a）または024（B-2b）にBlockedされている。****025(a)は`HW-TBD-004`に加えて、3.3 V rail下限の両立方式の決定にもBlockedされている**（2026-08-12に判明）。B-2の3.3 V側の実測はingress部品の選定にも要るため、023と024はその選定も止めている。**なお2026-08-12に、`PSU-PI-01`の5 VがRaspberry Pi公式の要求する5.1 Vと一致しないことが判明した。**予算の判定に使う最低電圧は`028`(a)として未確定であり、**この差の可否は評価できない**（`007`）。**034は2026-08-16に追加した。**同日に方式1（独立した外部観測）を採用済みだが、close条件が未達であり、**閾値が確定しても、測定値から受け入れの合否を導けない。**未解決なのは方式の選択ではなく、測定器の採否とclose条件である**（全数は`HW-TBD-034`の`必要な根拠`欄）**） |
| HW-TBD-010 | P1 | サーボの機械的可動域とneutral | 監視下calibration | 首振り動作の受け入れ | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18) | Human | 007–009、**026**、および首機構（[#34](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/34)）によりBlocked |
| HW-TBD-011 | P1 | サーボの速度／加速度制限 | 電流・動作試験 | Motion profile | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、[#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18) | Joint | 010によりBlocked |
| HW-TBD-012 | P1 | Touch gestureのしきい値 | 取得したraw sample | 撫で動作の受け入れ | [#14](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/14) | Joint | **008によりBlocked**（**003は2026-08-15にclose**。controller型番は`XPT2046`と確定したため、残るのはGPIO割り当ての確定と、`XPT2046` datasheetからのraw出力仕様である） |
| HW-TBD-013 | P1 | 軽打／持ち上げしきい値 | サーボ動作を含む取得済みraw sample | 軽打／持ち上げ動作の受け入れ | [#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15) | Joint | 004、008によりBlocked |
| HW-TBD-014 | P1 | 最終serial baudと最大line length | Pi／ESP32 transport test | Protocol v1の受け入れ | なし（未起票） | Joint | 候補値あり |
| HW-TBD-016 | P2 | **残: MVPへ含めるかの判断と役割定義。**識別情報はHamamatsu S11059-02DT（秋月 K-08316、I2C）と特定済み | MVP review。特定の根拠: [hardware-bom.md](hardware-bom.md) COLOR-01 | 将来の環境色feature | なし（未起票） | Human | Deferred（識別は完了、採否判断が残る） |
| HW-TBD-017 | P0 | 通信断の検知方式（heartbeat source、loss timeout） | Protocol合意＋latency測定。正本: [servo-safety-limits](servo-safety-limits.md#通信断時動作)、[protocol](../protocol/esp32-pi-protocol.md#13-未決定事項) | サーボの実機動作全般 | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18) | Joint | Open |
| HW-TBD-018 | P0 | 通信断時のfail-safe sequenceの選択と検証、および**recovery／reconnect動作**（断からの復帰時にサーボ出力を再有効化してよい条件と手順） | 監視下の機械試験（PWM断時の首の挙動、および復帰時の挙動）。正本: [servo-safety-limits](servo-safety-limits.md#通信断時動作) | #20、MVP受け入れ | [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18) | Joint | 010、017、PROTO-TBD-013によりBlocked |
| HW-TBD-019 | P0 | 起動時とdriver故障時のサーボ出力状態（PWM driver初期化前のGPIO state、開始mode、enableまでのdelay、Pi未接続時、reset後、driver故障検知時） | 無負荷でのPWM測定＋起動時glitch確認。正本: [servo-safety-limits](servo-safety-limits.md#起動時とdriver故障時の動作) | 初回統合通電 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17) | Joint | 010、**027**によりBlocked |
| HW-TBD-020 | P1 | 実行時のサーボ安全制御（採用する検知／予防手段、電流しきい値と判定時間、連続動作時間の上限、duty cycle窓と上限、検知時の物理動作、復帰条件、秒あたり受理command数、単一commandの最大変化量、command timeout、duplicate履歴の保持期間とretry window、retired sessionの保持件数と期間） | 電流測定手段の選定＋温度／電流試験。**正はfield単位**で[下表](#hw-tbd-020のfield単位の正)に定める。要件は[servo-safety-limits](servo-safety-limits.md#拘束stallと過負荷)、link側は[protocol](../protocol/esp32-pi-protocol.md#13-未決定事項) | 長時間動作とM6耐久試験 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、[#24](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/24) | Joint | 009、全fieldのresolution evidence未記録、およびPROTO-TBD-005／011／012／013／014未解決によりBlocked |
| HW-TBD-021 | P0 | **5 V ingressの過電流保護部品（`PROT-OC-01`）の選定とtrip値。**M-12001は3 Aを供給でき、候補構成で定格が判明している部品のうち最小は1.5 Aである（HW-TBD-022が解決するまで、これが真の最小値である保証はない）。現状は「上限を超えたらテスターの読みで人が電源を落とす」だけであり、connectorと線材が発熱する前に電流を止める手段が無い | メーカーの時間-電流特性（一次資料）に基づく選定。選定基準・挿入位置・上限との関係は[power-budget.md](power-budget.md#過電流保護段階cのgate) **2026-08-22に、部品が満たすべき数値の窓と、窓が閉じる境界を計算した。選定はしていない。**手順1〜3の入力は既存の表から揃うため計算できる。**MSP2807を除いた定常電流の合計から下限が出て、判明している最弱部品の定格が上限になる。****MSP2807の定常電流が一定値を超えると窓が閉じ、両方を満たす保護部品が存在しなくなる。**そのときは保護部品の選定の問題ではなく**経路の定格が足りないという結論**である（値と限定の正は[power-budget.md](power-budget.md)の`窓が閉じる境界（2026-08-22の計算）`。**ここへ再掲しない**）。**方式（PTC／ヒューズ）は未決定であり、これは人間の判断である。****上限も確定値ではない**（`経路部品と定格`に未確定の行が残っており、より低い定格が現れれば窓は狭くなる） **2026-08-22に境界の不等号を厳密不等号へ揃えた。**`方式によらない基準`は`その電流の80%が想定する定常電流の合計を上回ること`と定めており**等号を含まない。****したがって下限は`I > 合計/0.8`であり、境界の値そのものは窓に入らない。****MSP2807の定常電流が境界値以上になると窓が閉じる**（境界そのものが閉じる側である。値の正は[power-budget.md](power-budget.md)の`窓が閉じる境界（2026-08-22の計算）`。**ここへ再掲しない**） | **合成給電（段階C）**。保護部品の選定・実装まで実施しない | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Human | Open（**2026-08-22に窓と境界を計算した。選定も方式の決定もしていない。****この境界は段階B-2でのMSP2807の測定に対する合否の境界になる。境界値以上で窓が閉じる。**その場合は保護部品の選定へ進む前に、経路の変更か`servo-safety-limits.md`の制限の締め直しを決める。**方式の決定は人間の判断である。**残るのは(1) 方式の決定、(2) `HW-TBD-022`（経路の最小定格の確定）、(3) 品番と数値の一次資料（時間-電流特性）である） |
| HW-TBD-022 | P0 | **大電流経路の線材・接続部材（`WIRE-PWR-01`）と`CABLE-PI-PWR-01`の選定と許容電流。**（`CABLE-PI-PWR-01`は[power-budget.md](power-budget.md#経路部品と定格)で同じ規則の適用対象であり、2026-08-11の照合でこの行に含めることを明示した。）手持ちのbreadboardとジャンパー線は個別の許容電流がメーカー資料で確認できず、経路の最小定格を出せない。最小定格が出ないとingressの上限（経路部品の定格の最小値の80%）が確定しない | 公称許容電流が公開されている線材・接続部材の選定。決定の根拠は[power-budget.md](power-budget.md#経路部品と定格) **2026-08-22に選定規則を評価可能な形にした。選定はしていない。**販売元が公開しているのは住友電工の`許容電流計算式`（式と材料別の係数表）であって数値ではない。**式の入力のうち`r0`（20 ℃における導体抵抗の規格値）だけが電線側の仕様から来るため、`r0`が公開されていない製品では式を評価できない。**候補として見た秋月の`UL1015 AWG18`は`導体抵抗`も`許容電流`も公開していない。**第三者の公開値は18 Aと19.2 Aで食い違い、条件の記載も揃っていない。****したがって定格として採らない。**詳細は[power-budget.md](power-budget.md)の`線材の許容電流をどう確定させるか（2026-08-22の調査）`。**ここへ再掲しない** **2026-08-22に、線材・cableと接続部材で受け入れ条件が違うことを分けた。****`導体抵抗`は導体の証拠であって接点の定格を証明しない。**線材・cableの導体は「許容電流そのものか製造者が公開する`導体抵抗`のいずれか」、**接続部材は「製造者が公開する電流定格」**である（正は[power-budget.md](power-budget.md)の`線材と接続部材で受け入れ条件が違う`。**ここへ再掲しない**）。**cableは導体と接点の両方を持つため、両方の条件を満たす必要がある。** | ingressの上限の確定、**合成給電（段階C）** | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Human | Open（**2026-08-22に選定規則を評価可能な形にした。選定はしていない。**残るのは(1) **線材・cableの条件**を満たす品の選定、(2) 分岐点の接続部材（**未調査**）、(3) `CABLE-PI-PWR-01`（**未調査**）、(4) GND戻り経路である。**AWG18級なら経路の既知の最小値1.5 Aより一桁大きいことは分かったが、これは非メーカー値との比較であり定格の確定ではない。**`経路部品と定格`の表の`TBD`は埋めていない。****2026-08-24に選定条件が2つ増えた。**(1) `CABLE-PI-PWR-01`をrail側で受ける端子台がこの行の接続部材に入る（rail側の受け方が②（直付け）で確定したため）。(2) **手配線区間の接続部材は極性が固定される形であること**（`HW-TBD-030`の承認内容の2から渡された条件である）。**どちらも正は[power-budget.md](power-budget.md)であり、ここへ再掲しない**） |
| HW-TBD-023 | P0 | **ESP32 boardの`3V3` pinから外部負荷を取ってよい条件。**(a) 外部供給可能電流の定格、(b) board上regulatorの種別（LDO／switching）、(c) **過電流保護／短絡保護の有無と動作**（制限電流、折り返し特性、熱shutdownの有無）、(d) **`U2`周辺のdecoupling実装**（参照設計では入出力とも22 µF。`HW-TBD-029`から2026-08-12に移した）。**根拠は2段階に分けて扱う。参照設計の回路図で分かったことは、現物の事実ではない。**現物が公式V4リファレンス設計どおりに実装されている保証は文書だけでは得られない。**`HW-TBD-001`（pin配列の照合）は2026-08-15にcloseしたが、それはpin配列が一致したことを示すだけであり、実装部品が回路図どおりであることまでは保証しない。実際に`U2`は回路図と違う部品であった**（下記）。**参照設計で判明**: `U2 = AMS1117-3.3`（LDO）であり、回路図上にヒューズ・ポリスイッチは無い（`LESD5D5.0CT1G`×3はESD保護であって過電流保護ではない）。参照設計どおりなら、短絡・配線ミス時に電流を制限しうるのは`AMS1117-3.3` IC自身の内蔵保護だけになる。**2026-08-13の現物確認で、参照設計と現物が違うことが判明した。****現物の`U2`の刻印は`LD1117AG` / `33AQVCUV`であり、`AMS1117-3.3`ではない。**どちらもSOT-223の3.3V LDOでpin互換の二次ソースだが、部品が違う。`33`は3.3V版を示すと解される。**これは「参照設計由来の事実を現物の事実として書かない」という扱いが実際に効いた例である。**回路図を信じて`AMS1117`のdatasheetを当てていれば、別部品の仕様で段階B-2aの可否を判定していた。**2026-08-15に`U2`のメーカーを特定した。**現物の刻印は**UMW（UTD Semiconductor Co., Limited）の`LD1117-3.3`（SOT-223）**の刻印規則と一致する（根拠欄）。**これにより(b)が確定し、(c)は一部だけ埋まった。****(b)はLDOである。****(c)のうち確定したのは制限電流と熱shutdownの有無だけである。**内蔵の電流制限（`Ilimit` min 1.25 A／max 1.6 A）と熱shutdown（`TSD` typ 150 ℃）を持つ。**短絡保護の有無と折り返し特性はdatasheetに記載が無い。これは「無い」ことの確認ではない**（[AGENTS.md](../../AGENTS.md) 推測禁止）。**したがって(c)が求める「短絡・配線ミス時の動作」は未確定であり、(c)はcloseしない。****それでも段階B-2aに不利な事実が1つ確定した。**電流制限のtrip点1.25–1.6 Aは**この品の1 Aという定格より上にある。**したがって**`IOUT` 1.0–1.25 Aの区間は電流制限が動作しない範囲である。**この区間で熱shutdownが働くかは放熱条件次第でありdatasheetからは決まらない。**「熱shutdownが代わりに効く」と読み替えない。****残**: (a)、(c)の短絡・折り返し、(d)。**(a)はメーカーdatasheetでは埋まらない。**ICの供給能力は`3V3` pinが外部へ出せる電流の**上限を与えるだけ**であり、**Espressifは`3V3` pinの外部供給可能電流をboard levelで公開していない。**熱からの導出（datasheetのNote 1が与える`PD(max) = (TJ(max) - Tamb) / Rθja`に`TJ(max)` 150 ℃と`Rθja` 88 ℃/Wを入れる）も**この基板の実力ではない。**同Note 1が**`Rθja`は実装方法で変わると明記している**ためであり、[ESP32-DevKitC V4公式回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)は基板の銅箔面積を与えない。**したがって(a)は現物の実測でしか埋まらない。**(d)は`U2`周辺に部品が実装されていることは確認したが、**容量値は読めていない**（積層セラミックで表記が無い）。回路図に無い保護部品は現物にも見当たらない | 参照設計の根拠: [ESP32-DevKitC V4公式回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)。経路は`VBUS → D3 BAT760-7（Schottky）→ EXT_5V → U2 AMS1117-3.3 → VDD33 → J2 pin1`、入出力とも22µF（2026-08-10確認）。**現物の根拠**: 2026-08-13に斜光＋接写で`U2`の刻印を読み取り、`LD1117AG` / `33AQVCUV`を得た。**メーカーは2026-08-15に特定した。**出典は[UMW LD1117 datasheet](https://www.umw-ic.com/static/pdf/a5e0c99cefdefaf03cfa7777b369e45b.pdf)（発行元`UTD Semiconductor Co., Limited`、Mar.2025、14page、sha256は`5dec5c09027316c375c27108cbac9d088ba94060bebacd614a04a3b8bb18a3d3`。2026-08-15取得）である。**同datasheetの`14.Ordering information`はSOT-223の刻印を1行目`LD1117AG`、2行目`xxAQwwT S`（`xx`はVoltage、`ww`はWeek Code）と図示し、発注表に`Order Code: UMW LD1117-xx`／`Marking: LD1117AG xx`／`Package: SOT-223`を載せている。****現物の`LD1117AG` / `33AQVCUV`はこのtemplateと一致する。**1行目はliteralが一致し、2行目は`33`が`xx`（3.3 V）、続く`AQ`がtemplateのliteralと一致し、残る`VCUV`がdate／lotに当たる。**8文字の構成が一致する。****UTC（Unisonic Technologies）は候補から外した。**UTCにも`LD1117AG`系の型番があり、[UTC LD1117/A-K datasheet](http://www.utc-ic.com/uploadfile/2011/0915/20110915031520257.pdf)（文書自身が`UNISONIC TECHNOLOGIES CO., LTD`、文書番号`QW-R125-113.F`、footer `www.unisonic.com.tw`、Copyright 2024を記載する。sha256は`2af2ec5e4cf07cabcf2b76eb66cf24257554bb019d228d43a520460a7b1430b7`。2026-08-15取得。**`www.unisonic.com.tw`は同じPDFを直接返さず、`utc-ic.com`はhttpsの証明書が期限切れのためhttpで取得した。この1点だけはUMW側の資料より出所が弱い**）の`MARKING INFORMATION`は`A`をCurrent Code（1 A）、`G`をHalogen Free、`33`をVoltage Codeと定義するため**`LD1117AG`と`33`を意味の上では説明できる。しかしUTCの刻印templateは`UTC`の社名行を含み、`AQ`というliteralを生まない。**現物の読み取りは2行で社名行が無く、`AQ`を含むため、**UMWの方が一致度が高い。****決め手は刻印単体ではなく、メーカーが公開したtemplateとの一致である。**要件は[power-budget.md](power-budget.md#段階b-2の測定)の`B-2a: 3V3 pinから給電する経路`、確認項目は[hardware-bom.md](hardware-bom.md) `MCU-01`と部品受け入れchecklist。**配線規則の正本は[power-budget.md](power-budget.md)であり、この行では定めない** | **段階B-2a**。**(a)が未確定である限りB-2aを許可しない。**B-2bへ移れば回避できるが、`HW-TBD-024`／`025`は経路によらず必要なため、それらが未解決ならどちらの経路も進まない。**なお(c)が一部埋まったことでB-2aの前提が一つ悪くなった。**B-2aは`故障電流を制限するもの`をregulator自身の保護に置く経路だが（[power-budget.md](power-budget.md#段階b-2の測定)の経路表）、**電流制限は1.25 Aまで動かず、短絡時の挙動はdatasheetから決まらない。**採否の判断材料としてこの事実を`HW-TBD-009`と`HW-TBD-030`へ渡す | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Human | Open（**(b)は2026-08-15に確定した。(c)は制限電流と熱shutdownだけが埋まり、短絡保護と折り返し特性はdatasheetに記載が無いため未確定のまま残る。**(a)と(d)も残るためcloseしない。(a)はEspressifがboard levelで公開しておらず、**メーカーdatasheetでは埋まらないことが確定した**ため、残る手段は現物の実測だけになった。**そしてその実測は段階B-2そのものであり、この行がB-2aをBlockedしているため循環する**（`HW-TBD-024`と同じ構造である）。**B-2bを採れば(a)は要らなくなるため、循環を解く必要は無い。経路の選択で回避できる。**(d)は容量値が読めていない。**`HW-TBD-001`は2026-08-15にcloseしたが、この行は独立して残る**） |
| HW-TBD-024 | P0 | **MSP2807が耐えられる電流の上限。**メーカー未公開である。外部の3.3 V電源から給電する経路（段階B-2b）は**設定した電流制限が唯一の保護**であり、その設定値の上限を決めるにはmodule側の安全な上限が要る。**上限が無いままではMSP2807へ給電できない。**これは`HW-TBD-025`の共通条件(b)の一部であり、**B-2bだけでなくB-2aにも掛かる**（B-2aでもregulatorの保護のtrip点がmoduleにとって安全かを判定できない）。ADXL345とBME280だけ測っても未知数は埋まらないため、B-2自体が成立しない | **一次資料には無いことが確定した。**メーカーdatasheet（[msp2807.pdf](https://akizukidenshi.com/goodsaffix/msp2807.pdf)）の`Power Consumption`欄は`TBD`と印字されている（2026-08-10確認）。[LCD Wiki](http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807)にもbacklight駆動回路の記載は無い。**したがって現物のbacklight回路を見る以外に埋める手段が無い。****2026-08-13に現物を確認した結果は次のとおりである。**`R5`＝`6R8`（6.8 Ω）、`R6`＝`102`（1 kΩ）、`Q1`＝SOT-23で刻印`J3Y`、**`U1`＝SOT-23で刻印`662K` / `UMW S4T`**、`C1`＝積層セラミック（容量表記なし）、`J1`＝半田ジャンパで**オープン**。**`662K`はSOT-23の3.3V LDOの標準刻印であり、moduleが「VCC 3.3–5 V」を受けられる理由がboard上のLDOであることを示す。したがってこのLDOの電流能力がmoduleの上限を決める要素になる。****`U1`のメーカーと品番は2026-08-15に特定した。****UMW（UTD Semiconductor Co., Limited）の`XC6206P332MR`（SOT-23、3.3 V）である。**出典は[UMW XC6206 datasheet](https://www.umw-ic.com/static/pdf/e442437afc9daa185da86486e2bfe8c5.pdf)（発行元`UTD Semiconductor Co., Limited`、Mar.2025、13page、sha256は`8d8e3e30dbf48a39e0b4e11122ac242c2849b68ee8225ba0d672d83decf30504`。2026-08-15取得）である。**同datasheetの`13.Ordering information`は発注表に`Order Code: UMW XC6206P332MR`／`Marking: 662K`／`Package: SOT-23`を載せ、SOT-23の刻印図を1行目がmarking code、2行目が`UMW yww`（`yy`はYear Code、`ww`はWeek Code）と示している。****現物の`662K` / `UMW S4T`はこれと一致する。**1行目が3.3 V版のmarking codeと一致し、2行目の`UMW`が社名の位置に来る。**残る`S4T`は復号できない。**datasheetがyear／week codeの対応表を出していないためである。**`662K`は複数社が二次ソースする刻印であり、刻印単体では特定できない。決め手は2行目の`UMW`と、UMWが`662K`を自社の発注表に載せていることの組み合わせである。****供給能力**は次のとおりである。`IOUT(max)`は`VIN=VOUT(T)+1V`で**min 100 mA**、過電流保護`Ilimt`（**datasheetの表記のまま。`HW-TBD-023`の`LD1117`側は`Ilimit`と綴っており、資料が違う**）は**typ 380 mA**、短絡電流`Ishort`は**typ 30 mA**、絶対最大定格は`IOUT` **500 mA**と`PD`（SOT-23）**300 mW**である。**なお同datasheetは`1.Description`と`3.Features`で「`Vin=4.3V`／`Vout=3.3V`で`Iout=250mA`」と述べるが、`8.Electrical characteristics`は同じ条件で`IOUT(max)` min 100 mAしか規定していない。****保証値は100 mAであり、250 mAは本文の記述であって表の保証ではない。**矛盾を平均も統合もせず、両方を記録する（[AI Agent Policy](../governance/ai-agent-policy.md)）。**ただしLED個数と接続は確認できない。**パネル端がカプトンテープで覆われており、剥がすのは分解にあたるため目視では不能である。**`J1`がLDOのbypass選択かどうかもパターンを追っていないため断定しない。****さらに、`U1`の供給能力が判明してもmoduleの上限は決まらない。**理由は**backlightのLED列が`U1`の出力側にあるのか、`VCC`側（`U1`の上流）にあるのかを確認していない**ことである。**上流なら`U1`の能力はbacklight電流について何も言わない。**`R5`／`R6`／`Q1`／`J1`の部品値までは読めているが、**パターンを追っていないため経路を断定しない。**したがって`U1`のdatasheetで押さえられるのは**`U1`を通る分の上限だけ**である。なおLCD Wikiに現れる「0.31 W」はtypicalの消費であって**耐えられる上限ではないため、この行の答えにしない**。要件は[power-budget.md](power-budget.md#段階b-2の測定)の`B-2b: 外部の3.3 V電源から給電する経路`の`実施前に満たす条件` | **段階B-2（B-2a／B-2bとも。`HW-TBD-025`の共通条件(b)の一部であるため）**。MSP2807の定常電流の実測と、そこから決まる`PSU-INGRESS-01`の選定が段階Cまで進まない | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Human | Open（**`U1`のメーカーと供給能力は2026-08-15に確定したが、moduleの上限は出ていない。**LDO経路の上限は資料で押さえた（絶対最大`IOUT` 500 mA、`PD` 300 mW、保証100 mA、過電流保護typ 380 mA）。**しかしbacklightがこの経路を通るかが未確認であり、LED個数も依然としてテープで隠れている。****循環の扱いは2026-08-15に決めた。**datasheetと実測の二択のうち**datasheet側は尽きたが、実測へは進まない。**残る2点はどちらも**通電を要さない現物確認**であり、実測より先に行うべきものだからである。**したがってこの行は段階B-2を引き続きBlockedする**） |
| HW-TBD-025 | P0 | **段階B-2の共通条件。給電経路によらず必要であり、B-2a／B-2bのどちらを選んでも省略できない。**(a) `3.3 V rail`の許容電圧範囲。周辺module3点が**moduleとして受け入れてよい電圧**の積集合とESP32のlogic levelから決める（**IC単体の動作範囲の積集合ではない**）。(b) 周辺module3点それぞれが耐えられる電流の上限。B-2bでは電源に設定する電流制限値の上限を決めるために、B-2aでは**board上regulatorの保護のtrip点がmoduleにとって安全かを判定する**ために要る。**datasheet調査は2026-08-12に完了した。3点とも絶対最大定格に電流の上限は記載が無く、公開値からは得られない。**残るのは現物回路の確認であり、それも得られないなら当該moduleへB-2で給電しない。**`HW-TBD-023`（B-2a）や`HW-TBD-024`（B-2b）が解決しても、この行が未解決ならB-2を実施できない** | (a)の材料は公開spec（[hardware-bom.md](hardware-bom.md)の`DISP-01`／`ACCEL-01`／`ENV-01`の電源欄）と[gpio-assignment.md](gpio-assignment.md#電圧domainすべての外部pull-upに適用)である。**ただし積集合を取る対象は各moduleが受け入れてよい電圧であって、IC単体の動作範囲ではない。****`ACCEL-01`分はこれが未確定である**（`moduleへ入れてよい電圧`は`HW-TBD-004`。ADXL345 ICの`VS` 2.0–3.6 Vからmodule boardの許容入力電圧は決まらない）。**したがって(a)は`HW-TBD-004`が解けるまで確定せず、「現物確認を待つ必要はない」とした旧記載は2026-08-12に撤回した。**`DISP-01`と`ENV-01`分はmodule boardの資料があるため先に置ける。**2026-08-12に、その2点とESP-WROOM-32D moduleから暫定値を置いた**（値の正は[power-budget.md](power-budget.md#段階b-2の測定)の共通条件表の`許容電圧範囲`行であり、導出は同節の`3.3 V railの許容電圧範囲`。**ここへ再掲しない**）。**同時に、その暫定値の下限がそのままでは成立しないことが判明した。**下限がrailの公称値と同一で下振れの余裕が原理的に無く、さらに参照設計の`U2 = AMS1117-3.3`の出力がdatasheetの与える2行のどちらを採っても下限を割りうる（`HW-TBD-023`。**参照設計由来であり現物の事実ではない**。**2行の条件の違いはdatasheetに明記されていないため、広い方を採る**）。**`msp2807.pdf`はVCCについて`3.3V~5V`の一行しか持たず、下限が公称値か絶対最小値かを決められない。解釈で埋めない**（[AGENTS.md](../../AGENTS.md) 推測禁止）。**これは(a)そのものの中身であるため、別行を立てず(a)で追跡する。****両立させる経路は3つあり、いずれも人間の判断か現物確認を要する。****(1) MSP2807のmodule level資料に下限の性質を求める → 不可であることを2026-08-12に確認した。****(2) MSP2807のVCCを5 V railへ移す**（module specは3.3–5 Vを許す）**→ `DISP-01`の現物確認待ち**（logic IOが3.3 V TTLであり、5 V給電時に出力が5 VになればESP32のGPIOを壊す。level shiftの有無が未確認。`HW-TBD-002`）。**(3) 周辺module用に別途3.3 V regulatorを置き、設定点とtoleranceで下限を成立させる → 設計判断**（採る場合は[hardware-bom.md](hardware-bom.md)の購入待ちリストへ部品が増える）。**経路(2)を採る場合、`module VCC 3.3–5 V`の範囲を確認しただけでこの行をcloseしない。**(2)は段階B-2の前提そのものを引き直す。[power-budget.md](power-budget.md)は現在B-2を**周辺module3点を3.3 Vで測る**測定と定義しており、MSP2807だけ5 V railへ移すと**測定するrail、電流予算の割り付け、許容電圧範囲の積集合、logic levelの保護、decouplingの要否、B-2a／B-2bの依存関係のすべてが変わる。****closeの条件にはこれらの更新を含める**（詳細は[power-budget.md](power-budget.md#段階b-2の測定)の`暫定値の下限は、そのままでは成立しない`節の経路表）。**(b)** は各moduleのdatasheetの絶対最大定格を当たる。**2026-08-12に当たった結果、ADXL345とBME280はいずれも絶対最大定格を公開しているが、電流の上限は記載が無い**（[ADXL345 Data Sheet](https://www.analog.com/media/en/technical-documentation/data-sheets/adxl345.pdf) Rev. G Table 2 page 5は電圧・加速度・温度・短絡持続時間のみ、[BME280 Data Sheet](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf) Revision 1.24 Table 5 page 13は電圧・温度・圧力・ESDのみである）。**「あるはず」と仮定せずに確認した結果であり、無いことの確認も成果である。**したがって**MSP2807と同じ扱いが3点すべてに及ぶ**（現物回路の確認か、上限を得られないなら当該moduleへB-2で給電しない）。**MSP2807分は`HW-TBD-024`**。要件は[power-budget.md](power-budget.md#段階b-2の測定)の共通条件表、確認項目は[hardware-bom.md](hardware-bom.md)の部品受け入れchecklist | **段階B-2（B-2a／B-2bとも）** | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Human | Open（**(a)はBlocked。理由は2つある。**`ACCEL-01`のmoduleとしての許容入力電圧が未確定であり積集合を取れないこと（`HW-TBD-004`）と、**`DISP-01`と`ENV-01`から出る暫定値の下限そのものが成立しないこと**（上記の3経路。`HW-TBD-002`または設計判断）。暫定値を置いたことは進捗だが、**確定ではない**。(b)のdatasheet調査は2026-08-12に完了し、**3点とも電流上限の公開値が無いことを確認した**。残るのは3点それぞれの現物回路の確認である。MSP2807分は`HW-TBD-024`） |
| HW-TBD-026 | P0 | **SG90の電気的駆動条件。**(a) 制御logic要件（ESP32のGPIOは3.3 Vだが、SG90のlogic閾値が未確定であり、3.3 V driveで確実に動作するかを判定できない。[servo-safety-limits.md](servo-safety-limits.md#サーボ識別情報)は現物確認まで確定しないとしている）、(b) PWM周期／rate（50 Hzは一般値であり、確定値として採らない）、(c) 許容最小／最大pulse幅（データシート由来の電気的な駆動範囲。**機械的可動域は`HW-TBD-010`**）。**Stall／peak電流と無負荷／動作電流の実測は`HW-TBD-010`／`HW-TBD-011`の範囲であり、この行では重複させない**。**2026-08-24に一次資料を確認した。結論は「(a)(b)(c)のいずれも一次資料に記載が無い」である。****まず公式のdatasheet PDFが存在しないことが確定した。**[TowerPro公式のDownload](https://towerpro.com.tw/download/)にファイルの掲載がなく、製品ページにも`.pdf` linkがない。**したがって一次資料は製品ページの仕様表だけである。**仕様表が載せているのは`Operating voltage: 4.8v`、`Stall torque: 1.8kg/cm(4.8v)`、`Operating speed`、`Dead band width: 1us`、`Temperature range: 0℃-55℃`、重量・寸法・gear種別・plug種別・wire長であり、**logic閾値（V_IH／V_IL）、PWM周期／rate、許容pulse幅の3つはどこにも無い。****項目別**では次のとおりである。**(a) 記載が無い。**したがって**3.3 V driveの可否をこの資料では判定できず、level shifterの要否も決まらない。****「一般に3.3 Vで動く」という情報は一次資料ではないため採らない**（[AGENTS.md](../../AGENTS.md) 推測禁止）。判断は人間へ渡す。**部品を先に決めない。****(b) 記載が無い。**`50 Hz`は出所を持たない一般値のままである。**(c) 記載が無い。****しかも2026-08-24の再検証で、メーカーが直接聞かれても答えていないことが分かった。**`sg90-7`のQ&A欄で客が2022-03-02に「What is the range for the pulse width to control the angle? Online it says 1ms-2ms but that doesn't seem to cover the full 180 degree range.」と質問しているが、**同日の`admin`の回答はpulse幅に触れず、角度の話に替わっている。**得られたのは`Dead band width: 1 us`の1点だけであり、**これは許容pulse幅範囲ではなく範囲の内側の分解能に当たる量である。この値からpulse幅範囲を導かない。****さらに、適用する一次資料のページ自体が特定できないことが判明した。**公式サイトには同名で**2品**ある。`SG90 Digital`（`Operating speed: 0.1sec/60degree(4.8v)`）と`SG90 Analog`（同`0.12sec/60degree(4.8v)`）で、torque・電圧・dead band・寸法・温度範囲は同一である。**現物ラベル`TOWER PRO Micro servo 9g SG90`はこの2品を区別しない。**`HW-TBD-006`（正確なservo model）の解決記録はDigital側のページだけを引いている。[hardware-bom.md](hardware-bom.md) `SERVO-01`は2026-08-24に両ページを併記したが、**どちらの品かは未確定のままである。****`HW-TBD-006`は解決済みであり、この行では再openしない。**どちらの品かの判断と、`HW-TBD-006`を再openするかの判断は人間へ渡す。**なお(a)(b)(c)は2品で差の出る項目ではなく、どちらのページにも記載が無い。****したがってこの判別が付いても(a)(b)(c)は埋まらない。****2026-08-24の生HTML再検証で、variantについて2件追加で判明した。**(1) **同名の製品は2つではなく3つある。**`SG90 360 degree`（連続回転、SKU`SG90 360 degree`）が別に存在する。寸法が`23×12.5x22mm`で他2品の`23×12.2x29mm`と異なり、`Operating Angle: 360 degree`・`Direction: CCW`を持つため**現物とは別物と見られるが、断定していない**（現物の寸法を測っていない）。(2) **`Operating speed`でDigital／Analogを判別する根拠は弱い。**Analog側ページは箇条書きが`0.12 sec/60degree(4.8v)`だが、**同一ページの`PRODUCT CONFIGURE TABLE`は`Speed(sec/60deg) 0.1`と書いており、ページ内で矛盾している。****一次資料が自己矛盾しているため、速度の実測で品を判別できるとは言えない。****さらに、`4.8–6 V`と`0.5–2 A`の出所はDigital側ページに限られる。****Analog側ページは`Reviews (0)`であり、この2つの値はどちらもDigital側にしか無い**（2026-08-24確認）。**出所の格は2026-08-24の生HTML再検証で1段上がった。****匿名のreviewではなく、`admin`（メーカー側）の回答である**（2021-11-07。`The operation current for our SG90 is about 0.5A - 2A. Voltages from 4.8V - 6V are fine.`）。**それでも仕様表の値ではなくQ&A欄の記述であり、`Operating voltage`の仕様表記載は`4.8v`だけである。****したがってこの2値は、どちらの品かが未確定な状態で、片方の品のページのreview欄だけを出所とする値である。****variantに依存しない定格として扱わない。**`定格電圧範囲`を`TBD`へ戻すか、`Peak／stall電流`を実測まで`TBD`とするかは、**closeした`HW-TBD-006`の扱いと`HW-TBD-010`／`011`の範囲に掛かるため、この行では判断しない。人間へ渡す。****あわせて、この行の範囲外だが`HW-TBD-010`（機械的可動域）に関わる記述をQ&A欄で見つけた。**`admin`が2022-03-02に**「all of our normal servos are operating 0-150 degrees unless specify 180 or 360 degrees」**と述べている。**メーカーが可動域を0〜150度と言っており、hobby servoで一般に前提される180度を否定している。****この行では値として記録しない**（機械的可動域は`HW-TBD-010`であり、`HW-TBD-026`と重複させない）。**`HW-TBD-010`のcalibrationは監視下の実測で決めるものであり、この記述を確定値に採らない。**事実の存在だけを人間へ渡す。**なお、同じ`データシート値`という誤帰属が`HW-TBD-006`の解決記録にも残っている**（`定格電圧4.8–6 V、stall電流はデータシート値0.5–2 A`）。**close済みの行であるため、この作業では書き換えていない。**訂正の要否は人間の判断へ渡す。**この行で記録したのは値ではなく、一次資料に何が有って何が無いかである**（[servo-safety-limits.md](servo-safety-limits.md#サーボ識別情報)が「datasheetの代表値を確定値として採らない」と定める） | **メーカー資料では(a)(b)(c)は埋まらないことが2026-08-24に確定した。**公式のdatasheet PDFが存在せず、一次資料である製品ページの仕様表にこの3項目の記載が無いためである（`HW-TBD-023`(a)と同じ構造。**メーカーが公開していないものを推定で埋めない**）。**残る手段は無負荷試験と実測だけであり、それは007／009によりBlockedである。****「datasheetが無い」の検証方法を明記する。**要約に依らず生HTMLで確認した。`robots.txt`が指す`wp-sitemap.xml`からsite全体の**203 URLを列挙し、`.pdf`は0件**。SG90の3ページと`/download/`の生HTMLに**`.pdf`へのlinkが0件**、**`datasheet`／`data sheet`／`spec sheet`の語も0件**。**いずれもHTTP 200で実体を取得しており、403や空応答ではない**（2026-08-24）。根拠とした一次資料は[TowerPro公式 SG90 Digital](https://towerpro.com.tw/product/sg90-7/)、[TowerPro公式 SG90 Analog](https://towerpro.com.tw/product/sg90-analog/)、[TowerPro Download](https://towerpro.com.tw/download/)（いずれも2026-08-24取得）である。**第三者掲載の`Soldered_101246.pdf`を一次資料に採らない。**TowerPro発行と確認できず、取得もできないためである（詳細と扱いは[hardware-bom.md](hardware-bom.md) `SERVO-01`が正）。**(a)については、閾値が確定しないままlevel shifterの要否を決めない。**要件は[servo-safety-limits.md](servo-safety-limits.md#サーボ識別情報)の識別情報表 | `SERVO-PWM`の駆動条件の確定、servoの実機動作全般 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17) | Joint | Open（**データシートから確定できる部分は2026-08-24に着手し、完了した。結果は「(a)(b)(c)のいずれも一次資料に記載が無い」であり、値は1つも確定していない。**この行に残るのは無負荷試験を要する範囲であり、**007／009によりBlocked**である。**あわせて人間へ渡す判断が残る**（level shifterの要否、どちらの品か、review欄由来の値の扱いなど。**全数はこの行の`不足している情報／判断`欄が正であり、ここで数え直さない**）） |
| HW-TBD-027 | P0 | **`RES-PULL-01`（`SERVO-PWM`の外部pull-down）の抵抗値と本数。**GPIO27はreset時にhigh-Zであり、外部pull-downはPWM driver初期化前にservoが不定pulseを受けないための**必須**部品である。それにもかかわらず抵抗値が未選定である。**部品は10 kΩと4.7 kΩが各1袋2026-08-08に着荷しており、未購入ではない**（[hardware-bom.md](hardware-bom.md) `RES-PULL-01`）。**残るのは必要な本数と抵抗値の選定と実装であり、手元の2種で足りるとは限らない。****この行は`TBD`セルではなく、同じ行の登録時の記述「未購入・未選定」から登録した**（**同記述は2026-08-15に「一部入手済み・未選定」へ訂正済みである。登録根拠として引いた文言は現在のBOMには無い**）（[区別の基準](#区別の基準)） | 抵抗値の選定（ESP32のGPIO駆動能力とservo側入力インピーダンスから決める）。要件は[gpio-assignment.md](gpio-assignment.md#信号inventory)の`SERVO-PWM`行、部品は[hardware-bom.md](hardware-bom.md) `RES-PULL-01`（**同部品は`LCD-RST`／`LCD-CS`のpull-upもまかなう。そちらは`HW-TBD-032`**）。**選定だけではcloseしない。**必要な根拠は[HW-TBD-027の証拠契約](#hw-tbd-027の証拠契約)に定める。**値を決めた文書だけでcloseすると、pull-downが実装されていないのに`HW-TBD-019`のBlockedが外れる** | 初回統合通電、`HW-TBD-019`の起動時状態の確定 | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2) | Human | Open |
| HW-TBD-028 | P1 | **電源品質の数値制限。**(a) Pi入力で許容する最低電圧、(b) ESP32入力／3.3 Vで許容する最低電圧、(c) 最大定常ripple、(d) 最大transient droopと継続時間、(e) connector／wireで許容する最大温度上昇。**許容するbrownout／reset回数は0回で確定済みであり、この行に含めない。****2026-08-12に5項目とも規則を定義した。数値が確定したのは(d)の継続時間0だけである**（値の正は[power-budget.md](power-budget.md#受け入れ条件)の`電源品質の数値制限`節。**ここへ再掲しない**）。残りは下記のとおり他の行に従属するか、一次資料が存在しない | **実測ではなく定義が先である。**各deviceのdatasheetの動作電圧範囲と絶対最大定格、および線材・connectorの温度定格から上限・下限を定める。実測はその後の受け入れ試験で照合する。要件は[power-budget.md](power-budget.md#受け入れ条件)が挙げている一覧。**2026-08-12時点の項目別の状況は次のとおり。****(a) 一次資料が存在しないことを確認した。**Raspberry Pi公式documentationは`All models require a 5.1V supply`と述べるのみで許容範囲も最低電圧も示さず、Zero W向けのproduct briefとreduced schematicsも公開されていない。**低電圧検出のしきい値4.63 Vは`except the Zero range`と明記されており、このboardに適用されない。二次情報でも代用しない**（[AGENTS.md](../../AGENTS.md) 推測禁止）。**帰結として段階AでPiの低電圧を判定する手段が二重に無い**（判定値が無く、Pi自身の検出回路も無い。[power-budget.md](power-budget.md)の`Pi Zero Wには低電圧検出が無い`）。**(b)** ESP32 boardの5 V入力は参照設計のdropout最大1.3 Vから4.6 Vと導いたが、**参照設計由来であり現物の`U2`は未確認である**（`HW-TBD-023`）。3.3 V rail側は`HW-TBD-025`(a)に従属する。**(c)(d)** 推奨動作条件が瞬時値の制約であることから導出規則を定めた。深さは(a)(b)の下限に従属する。**(e)** 規則だけを定めた。経路部品が未選定のため数値は`HW-TBD-021`／`HW-TBD-022`／`HW-TBD-007`に従属し、**一次資料に無い温度上昇値を代わりに置いていない**。**照合できる段階の対応表も同節に置いた。****測れる段階・測定のgate・合否判定を別の列に分けている**（(a)は段階A、(b)は5 V入力側が段階B-1で3.3 V rail側が段階B-2、(c)は段階A／B-1と段階Cを分ける、(d)(e)は段階C。**段階AとB-1はgateも追加購入も要らない**）。**ただし合否判定は5項目とも`Blocked`である。**測定はgateさえ満たせば進められるが、判定には確定した閾値と検証済みの測定系の両方が要り、現状はどの項目も揃っていない。**測定値を記録してよいが、そこから受け入れの合否を導かない。****2026-08-16に、測定方式の独立性と合格主張を張ってよい範囲を`HW-TBD-034`として分離した。**この行が扱うのは**数値**であり、034が扱うのは**測定方式と主張の範囲**である。**ここへ034の内容を再掲しない** | 電源系の受け入れ承認、合成給電（段階C）の合否判定 | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Joint | Open（**規則は5項目とも定義済み。数値は(d)の継続時間0のみ確定。**(a)は一次資料が無いため確定せず、(b)〜(e)は`HW-TBD-004`／`023`／`021`／`022`／`007`と、`HW-TBD-025`(a)の下限の両立に従属する。**照合（実測）は未着手である。段階A／B-1は他のgateを要さずに測れるが、測れることと合否を判定できることは別である。**(a)は判定値も測定系も無いため、段階Aで得た値から受け入れの合否を導かない。**(c)(d)は閾値が確定しても`HW-TBD-034`が解けるまで合否を判定できない**） |
| HW-TBD-030 | P1 | **逆極性保護の要否と方式。**[power-budget.md](power-budget.md)の`配線・保護表`で`TBD`／`Blocked`のまま残っている。配線ミス時にPi、ESP32、周辺module3点が同時に破損しうる。**2026-08-12にdesign reviewの材料と推奨案を用意した**（[power-budget.md](power-budget.md)の`逆極性保護のdesign review`節）。**登録時点では決定していない。**この行のOwnerはHumanであり、採否の判断を経るまで確定として扱わない。**要否と方式は2026-08-24に承認された**（状態欄。**行はcloseしていない**） | Design review。**2026-08-12に用意した材料**: (1) 逆接が起こりうる区間の洗い出し。**極性付きconnectorで固定される区間（M-12001→変換基板、rail→Piの`PWR IN`）では起こらず、起こりうるのは手配線区間だけである。**したがって**ingress 1箇所に保護部品を入れても下流の手配線区間は守れない。****この区間の分類は2026-08-24に変わった。**rail側の受け方が②（直付け）で確定してrail側が手配線になったため、**`rail→Piの PWR IN`は`可`である**（正は[power-budget.md](power-budget.md)の`逆極性保護のdesign review`の逆接可否表。**ここへ再掲しない**）。**2026-08-12の材料としての記述は残し、分類の訂正をここに置く。**(2) 方式ごとの電圧降下の代償。**直列Schottkyは5 V予算を食う。ただし採否は確定しない。**比較対象の4.6 Vは`IOUT ≤ 0.8 A`かつ参照設計の`U2`という条件付きの導出値であって確定した合格下限ではなく、**現物の`U2`と実負荷を確認するまで判断できない**（`HW-TBD-023`、`HW-TBD-028`(b)）。P-ch MOSFETによるideal diodeは降下が小さいが品番を選ばないと具体値が出ない。逆並列diode＋fuseは溶断までの間、負荷に逆電圧が掛かる。(3) 推奨案はingressへP-ch MOSFETを置き、**下流の手配線区間は保護部品ではなく極性が固定される接続方法と通電前の配線確認手順で守る**というもので、**2026-08-24に人間がこれを承認した**（状態欄）。**品番と定格は`PROT-OC-01`と同じく発注直前に一次資料で確定する** | 合成給電（段階C）の配線承認 | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Human | Open（**要否と方式は2026-08-24に人間が承認した。closeはしない。**承認内容の正は[power-budget.md](power-budget.md)の`逆極性保護のdesign review`の`決定（2026-08-24）`である（**ここへ再掲しない**）。**残るのは(1) 品番と定格の一次資料での確定、(2) 通電前の配線確認手順の作成、(3) 極性が固定される接続部材の選定**（`HW-TBD-022`の`WIRE-PWR-01`側で選ぶ）**である。****2026-08-24のreview指摘で、(2)(3)の対象にrail → Piの`PWR IN`区間が加わった。**②（直付け）を採ったことでrail側が手配線になり、端子台でVBUSとGNDを入れ替えるとPiへ逆電圧が掛かる。**ingressのMOSFETより下流であり、承認内容の1では守れない。****方式が決まっただけでは段階Cの配線承認は与えられない**） |
| HW-TBD-031 | P1 | **搭載moduleの識別は現物の刻印で、中核chipの識別は`esptool`が報告するchip名で確かめる。****2026-08-15に要件を書き換え、同日に中核chip側の満たし方を決定した。**旧要件は「ESP32 chipの刻印の読み取り（反射を避けた撮影、または実体顕微鏡）」だったが、**この要件は非破壊では成立しない。**中核chip `ESP32-D0WD` はESP-WROOM-32Dの**半田付けされた金属シールドの内側**にあり、シールドを外さない限りどのような撮影手段でも見えない。シールドの除去は分解であり、[Hardware Safety Policy](../governance/hardware-safety-policy.md)の範囲でも#1の範囲でもない。**module刻印の読み取りは2026-08-13に完了した**（下記）。**中核chipの識別の満たし方は2026-08-15に決定した。**`esptool`が報告するchip名で確認する（正本は[chip 識別の満たし方](../toolchains/esp32-rust-toolchain.md#chip-識別の満たし方)であり、期待値・使うcommand・担保しない範囲はそこに定める。**ここへ再掲しない**）。**`espflash`の出力では満たせない**（family名しか返さない）。**残: [#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)のflash時にその確認を実施し、出力を記録すること。**実機Linux限定であるため、この行はそれまで解決しない。**旧記載の「この判断は`Verified`にする条件の再定義を伴うため本行の範囲を超える」「#1では要件の成立しなさを記録するまでとし、再定義は別途扱う」は2026-08-15に撤回した。**受け皿のIssueが無いまま「範囲外」と書いており、記述と実態が食い違っていた。**再定義は[#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)の範囲内で行った** | **module刻印は読了した**（2026-08-13、斜光＋接写）。シールド上面の記載は次のとおりである。`ESPRESSIF` / `ESP32-WROOM-32D` / `FCC ID : 2AC7Z-ESPWROOM32D` / `IC : 21098-ESPWROOM32D` / `CMIIT ID : 2018DP2467` / 技適 `Ⓡ 211-171102` / KC `R-CRM-esp-ESPWROOM32D` / NCC `CCAH18LP020IT7`。**これによりmodule種別の根拠が、購入履歴と基板silkscreenに加えてmodule自身の刻印になった**（反映先は[hardware-bom.md](hardware-bom.md) `MCU-01`）。**ただしこれはmoduleの品番であってchipの品番ではない。**中核chipが`ESP32-D0WD`であるという記載の出典は[ESP-WROOM-32D datasheet v2.7](https://documentation.espressif.com/esp32-wroom-32d_esp32-wroom-32u_datasheet_en.pdf)のままである。**加えて、#6のflash時に`esptool`が報告するchip名の記録が要る。**その記録が中核chipの識別の根拠になる（正本は[chip 識別の満たし方](../toolchains/esp32-rust-toolchain.md#chip-識別の満たし方)） | [ESP32 Rust Toolchain](../toolchains/esp32-rust-toolchain.md)の状態を`Verified`にする条件。**条件そのものの再定義は2026-08-15に完了した。残るのは充足である**（上記） | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6) | Human | Open（**範囲を縮小し、要件を書き換え、2026-08-15に満たし方を決定した。**module刻印は完了。chip刻印は非破壊では読めないことが確定した。**2026-08-20に#6のflashを実施し、`esptool` 4.12.0 が `ESP32-D0WD` を報告した。期待値に一致する。**満たし方の正本は[esp32-rust-toolchain.md](../toolchains/esp32-rust-toolchain.md)の`chip 識別の満たし方`であり、**ここへ再掲しない。**未加工の出力と実行したcommandは[Version Record](../toolchains/version-records/2026-08-20-esp32-flash-boot-native.md)にある。**closeの手順（正本文書への反映と関連Issueのblocked解除）を通すまで行は残す**） |
| HW-TBD-032 | P0 | **`LCD-RST`／`LCD-CS`の外部pull-upと`LCD-BL`の外部pull-down（`RES-PULL-01`）の抵抗値・本数・極性。**[gpio-assignment.md](gpio-assignment.md#信号inventory)は`LCD-CS`へ**外部10kΩ pull-up推奨**（active-low CSをfirmware初期化前もinactiveに保つため）、`LCD-RST`へ**外部pull-up推奨**（reset非activeを既定にするため）、`LCD-BL`へ**外部pull-down推奨**（backlightをfirmware初期化前もOffに確定させるため）としているが、**3つとも未実装である。**同文書の受け入れchecklistが「Resetとbacklight lineが安全な状態で起動する（`LCD-RST`/`LCD-CS`への外部pull-up実装が前提。**未実装のため要対応**）」と明記している。未実装のまま通電すると、`LCD-CS`では起動直後の数十ms間bus contentionのriskがあり、`LCD-BL`はGPIO4の内部weak pull-downが外部回路に対して弱いためbacklightが点灯しうる。**`HW-TBD-027`は`SERVO-PWM`のpull-downだけを対象としており、この2信号を含まない。**`RES-PULL-01`は両方をまかなう部品である。**この行は2026-08-11の全数照合で登録した**（`未実装`の語による検出。command 3)の群B） | 抵抗値・本数・極性の選定、購入、実装、および起動時の状態確認3点（`LCD-CS`がHigh＝inactive、`LCD-RST`がreset非active、`LCD-BL`がbacklight Off）。**3信号すべてを確認するまでcloseしない。**`LCD-BL`と`LCD-RST`の極性は現物確認後に決まるため、`HW-TBD-002`が先に要る。要件は[gpio-assignment.md](gpio-assignment.md#信号inventory)の`LCD-RST`／`LCD-CS`／`LCD-BL`行と受け入れchecklist、部品は[hardware-bom.md](hardware-bom.md) `RES-PULL-01` | 初回統合通電、LCDのbring-up（#13） | [#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2) | Human | Open（`LCD-CS`のpolarityの現物確認は`HW-TBD-002`と同じ機会に行える） |
| HW-TBD-034 | P1 | **電源過渡の測定方式と、合格主張を張ってよい範囲。**[power-budget.md](power-budget.md#測定計画)の測定系は`ADC-5V`／`ADC-3V3`をESP32自身のADCで読むが、**そのESP32自身の給電が導かれるrailを測っているため、brownout／resetでsamplingが止まる区間を独立に観測できない。**sample間とburst captureのwindow外にも未観測区間がある。**この状態で「起動時の電源過渡をすべて実測した」とは主張できない。****論点の出所は[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)の2026-08-16の追記であり、この行は同日に登録した。****同日に人間が方式1（独立した外部観測）を採ると決定した。**oscilloscopeは購入せず、手持ちのArduino Uno R3を候補とする。**技術ガイドのゲートAの改訂は不要になった。****数値の項目ではない**（数値は`HW-TBD-028`。**ここへ再掲しない**） | **人間による方式の決定。**材料と全数の改訂対象は[power-budget.md](power-budget.md#測定計画)の`ESP32自身のADCは測定対象から独立していない`と`電源過渡の測定方式のdesign review`に置いた（**値も規則もここへ再掲しない**）。方式1は独立した外部観測を導入する案で、**必要帯域の根拠は`TBD`である**（一次資料が要る。[AGENTS.md](../../AGENTS.md) 推測禁止）。**手持ちのArduino Unoが候補にあり、成立すれば購入は不要になりうる**（[hardware-bom.md](hardware-bom.md) Revision 47）。**2026-08-16に現物の刻印で`UNO R3`／`ATMEGA328P-PU`／`MEGA16U2`まで確定した**（通電なし）。**同日に`DS40002061B`でADCの分解能・sample rate・入力範囲・基準電圧も確定した**（値の正は[power-budget.md](power-budget.md#測定計画)。**ここへ再掲しない**）。**それでも採否は決まっていない。**基準電圧が未解決で**校正か外部基準が要り**、USB給電での動作とGND topologyへの影響も未確認である。**方式2（ADC loggingへ限定し合格主張を狭める案）は採らないことになった。**ゲートAの合格主張を狭める＝安全側の要求を下げる方向であり、方式1を採ったことで適用先も無くなった。closeには次の全部が要る（**8項目**）。**(1) 測定器の採否、(2) 方式1の実装、(3) 基準電圧の校正（または外部基準の採用）、(4) SRAM 2 KBに対する取得方式、(5) GND topologyへの影響の確認、(6) 必要帯域の一次資料、**(6b) Power Treeのschematic粒度の確認**（`F1`／`T1`／`OPAMP`の役割、どちらが電源電流経路に直列でどちらが制御側か、GNDとbackfeed経路の一致。**block図の粒度では足りない**）、(7) 4文書（技術ガイド、[hardware-safety-policy.md](../governance/hardware-safety-policy.md)、power-budget.md、#3）の合格主張が一致したことの確認。****(1)〜(6)の値と規則の正はpower-budget.mdであり、ここへ再掲しない。****実装と文書整合だけでcloseしない。****技術ガイドは方式1では無変更でよいため、一致の確認は上の方式1の3点が解けた後になる** | **[技術ガイド](../DeskCat_Microcontroller_Development_Guide.md)のゲートA**（起動時の電源過渡の実測）、および`HW-TBD-009`（電源予算とbackfeed review） | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3) | Human | Open（**2026-08-16に方式1で決定済み。closeはしない。****方式1の技術的な未解決3点のうち、2026-08-22時点で解けたのは(1)と(2)である。(3)はGNDの両極性の確認は解消したが、star point構成の確認が残るため未解決である**（正は[power-budget.md](power-budget.md#測定計画)の`方式1で残っている未解決3点`。**ここへ値も状態の詳細も再掲しない**）。**(2) SRAM 2 KBに対する取得方式は2026-08-17に解けた**（未加工の観測は[experiment-log.md](experiment-log.md)の`EXP-001`）。**(1) 基準電圧の校正は2026-08-22に解けた。**比率法で比を実測し、**AV<sub>CC</sub>の代理値**（`5V` pin）と周囲温度を記録した。**以下の限定は2026-08-22に解消した。当時の経緯として残す。**`5V` pinは`AVCC` pinと同一nodeではなく、当初はその差を測っておらず上限も与えていなかったため、得られた`V_INT`は代理値ベースの推定であって絶対値ではないとしていた（[PR #158](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/158)の自動reviewの指摘。**限定を書いたうえで解決済みにするのは、限定を書いていないのと同じである**）。**その後、`5V` pinと`AVCC` pin（pin 20）の差を直接測り、上限がテスターのゼロ点以下で、記録済みの不確かさに埋もれることを確認した**（正は[power-budget.md](power-budget.md#測定計画)の`方式1で残っている未解決3点`、未加工の観測は[experiment-log.md](experiment-log.md)の`EXP-005`。**ここへ再掲しない**）。**したがってその不確かさの範囲で絶対値として扱える。****確度を超える主張はしていない。**デジタルテスターのrange別確度は取扱説明書にも本体表示にも無く**確度そのものは取得できないことが確定したため、`最小単位`から不確かさの下限を出している**（正は[power-budget.md](power-budget.md)の`DT830Bの一次資料で分かったこと（2026-08-21）`）。未加工の観測は[experiment-log.md](experiment-log.md)の`EXP-002`。**同日に測定器2品の記載も直した。**テスターの型番は**現物の本体表示`DT830B`**であり、#55以来の`MAS830L`は誤りだった。温度計は**手持ち品で要件を満たし購入不要になった**（[hardware-bom.md](hardware-bom.md)の`MEAS-03`／`MEAS-04`）。**(3) GND topologyへの影響は2026-08-22に段階B-1を実施したが、未解決である。**人間が基板のそばで観察している状態で通電し、**異常は無かった**（異音・発熱・reset・brownoutのいずれも無し）。段階B-1で実測できたのは3項目（ADC値の妥当性、GNDの結合の影響、入力headroom）である。**入力headroomの実測はdividerを外せないことの定量的な根拠になった。****それでも解けていない。理由が2つある。****(i) GNDの両極性の確認は2026-08-22に解消した。**極性を読める手段で測り、**ESP32の`GND`がArduinoの`GND`より低く、ADCの1 LSBより大きいoffsetがADCの死角にあったことが分かった。`GND`の直結はoffsetを縮めるが消さない**（正は[power-budget.md](power-budget.md#測定計画)の`方式1で残っている未解決3点`、未加工の観測は[experiment-log.md](experiment-log.md)の`EXP-004`。**ここへ再掲しない**）。**(ii) star point構成そのものの確認が残る**（値と限定の正は[power-budget.md](power-budget.md#測定計画)の`方式1で残っている未解決3点`、未加工の観測は[experiment-log.md](experiment-log.md)の`EXP-003`。**ここへ再掲しない**）。**2026-08-20に実施可能範囲を確定した。**star point構成は`5V ingress GND`を含み、それは変換基板`PSU-INGRESS-01`で作るものだが**同品は未購入であるため完全な構成は組めない。組める範囲は段階B-1に限られ、そこで確認できるのはPCのGNDが経路へ入る影響までである**（正は[power-budget.md](power-budget.md)の`GND topology（測定前に必ず確定させる）`。**ここへ再掲しない**）。**したがって(3)は変換基板が揃うまで完了しない。****測定器の採否も未確定である。**2026-08-20に現物の写真で`QUIMAT DSO Shell`の所持が判明し、**この文書と[power-budget.md](power-budget.md)が#55以来「oscilloscopeは所有していない」前提で候補をArduino Uno R3に限っていたことが誤りだと分かった。**2026-08-21にJYE Techの`DSO150`のmanualを取得したが、**現物は`QUIMAT`であり同一機種である根拠が無いため数値は参考値である。****参考値では`Record length`が固定であり、それが正しければArduinoの連続streamingを置き換えられず、入力もBNC 1系統である。****したがって2品は代替ではなく相補であり、どちらも単独では方式1の要求を満たさない**（正は[power-budget.md](power-budget.md)の`2つの手持ち候補は代替ではなく相補である（2026-08-21の材料）`。**ここへ再掲しない**）。**DSO Shellは2026-08-21に初回通電し、画面が表示されることを確認した**（異音・発熱・異臭なし。人間が基板のそばで観察）。**BNCには何も接続しておらず、波形の取得も垂直確度の確認もしていないため、測定器として使えることは確認できていない。****方式と測定器の判断は人間が下すものであり、この行では判断しない。****この3点はcloseの条件の一部であって全数ではない。**close条件は`必要な根拠`欄が持つ。**いずれもUSBを扱える端末が要り、Docs / Review端末では実施できない**） |
| HW-TBD-035 | P0 | **servoへ入れてよい電圧。**`PSU-SERVO-01`はM-12001（5 V）から分岐したservo railであり、**この5 Vをservoへ入れてよいかを確認した文書が無い。**[hardware-bom.md](hardware-bom.md) `SERVO-01`の`定格電圧範囲`は`4.8–6 V`だが、**2026-08-24の一次資料確認で、この範囲の出所が仕様表ではなく`SG90 Digital`ページのQ&A欄にある`admin`（メーカー側）の回答だと判明した**（2021-11-07。`Voltages from 4.8V - 6V are fine.`）。**製品ページの仕様表が載せる電圧は`Operating voltage: 4.8v`だけであり、供給予定の5 Vはこれより上である。**つまり**5 Vの可否は、仕様表ではなくQ&A欄の1文だけに依っている。**しかも**`SG90 Analog`ページは`Reviews (0)`でこの記述が無く、現物がどちらの品かも未確定である**（`HW-TBD-026`）。**そして同名の3品目`SG90 360 degree`は、仕様表そのものが`Operating voltage: 4.8v-6V`を載せている。****つまりどの品かによって、`4.8–6 V`に仕様表の裏付けがあるかどうかが変わる。**3品目は`Operating Angle: 360 degree`の連続回転で寸法も`23×12.5x22mm`（他2品は`23×12.2x29mm`）と異なるため、**現物とは別物である可能性が高い。ただし現物の寸法を測っていないため除外を確定していない。****除外を主張するなら寸法の実測が要る**（下の証拠契約の条件1）。**[power-budget.md](power-budget.md)の`許容電圧範囲`の積集合は3.3 V railの周辺module3点とESP32の`VDD33`だけを対象としており、servoを含まない。**servo railは5 Vで別系統のため積集合の対象外であるが、**代わりに確認する項目も存在しない。****したがってservoの入力電圧は、どの文書でも適合を確認されていない。**扱いは`HW-TBD-004`の`moduleへ入れてよい電圧`と同じ型である（IC・部品の定格から基板・製品の許容入力を導かない） | **メーカーの一次資料では埋まらない見込みである。**公式のdatasheetが存在せず（`HW-TBD-026`）、仕様表は`4.8v`単独である。**Q&A欄の回答を5 Vの根拠として採るかは人間の判断であり、機械的には決まらない。**採らない場合に残る手段は、(1) メーカーへの問い合わせ、(2) 4.8 Vを供給する構成へ変える、(3) 監視下で5 Vを入れて挙動を観察する、のいずれかである。**(3)は通電を伴うため`007`／`009`のgateに従う。****この行では5 Vを入れてよいと決めない。****closeに必要な記録は[HW-TBD-035の証拠契約](#hw-tbd-035の証拠契約)に定める。**関連は[servo-safety-limits.md](servo-safety-limits.md#サーボ識別情報)の識別情報表と[power-budget.md](power-budget.md)の`許容電圧範囲` | servoへの給電、`SERVO-PWM`の駆動条件の確定、servoの実機動作全般 | [#193](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/193) | Human | Open（**2026-08-24に登録した。**`HW-TBD-026`の調査で判明した、追跡されていなかった項目である。**値を確定させるのではなく、Q&A欄の回答を根拠として採るかの判断が先である**） |

## 対応Issue列

`対応Issue`列は、その`TBD`を**解消する作業Issue**を指す。台帳を見た人が
「この`TBD`を解くのはどのIssueか」を判断できるようにするための列である。

逆方向（Issue → TBD）は、[#65](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/65)で
[#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)と[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)の本文へ対応表を追加した。
**この2件以外のIssue本文には対応表が無い。**そのため、この列が両方向を辿れる唯一の起点である行が多い。

### 記載規則

| 場面 | 書き方 |
|---|---|
| 対応Issueが1件 | `[#N](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/N)` |
| 複数Issueに跨る | 該当Issueを`、`で区切って**すべて**列挙する。**主従を付けない。**どれか一つが解決しても行はcloseしない |
| 対応Issueが無い | `なし（未起票）`と書く。**空欄にしない。**空欄では「未記入」と「対応Issueが無い」を区別できない |

書くのは**解消する側**のIssueだけである。その`TBD`が解けるのを待っている側のIssueは書かない。
両方を書くと列が依存関係の写しになり、「どれを進めれば解けるか」が読めなくなる。
待っている側は`妨げる対象`列と各Issueの依存関係が表す。

**同じIssue番号が`妨げる対象`と`対応Issue`の両方に現れたら、どちらかが誤りである。**
待っている側が解消する側を兼ねることはない。

どのIssueが解消するかは、**その行の`必要な根拠`列が求める作業を範囲に含むか**で判定する。
Issueの題名や扱う領域の近さで決めない。たとえば`HW-TBD-019`の根拠は
「無負荷でのPWM測定＋起動時glitch確認」であり、これを範囲に持つのは
[#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)である。
[#20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20)は同じ状態を**検証する**側であって、値を決める側ではない。

行を追加するときは、この列も同時に埋める。埋められないなら`なし（未起票）`とし、
下の一覧へ加える。

### closeしたIssue番号の扱い

**Issueがcloseされていても、その`TBD`が解決済みであることを意味しない。**
台帳の行のcloseは[解決手順](#解決手順)の1〜8による別の判定であり、Issueのcloseとは一致しない。
Issueは受け入れ条件を満たせばcloseされるが、`TBD`は根拠の記録、正本文書への反映、
関連Issueのblocked解除まで終えて初めてcloseする。

この列のIssueがcloseされたら次を行う。**列の記載は消さない。**

1. [解決手順](#解決手順)の1〜8を満たしたかを確認する
2. 満たしていれば[解決済み項目](#解決済み項目)へ移す
3. 満たしていなければ行を残す。残作業を担うIssueが無くなったなら、起票してこの列へ追加する

### 対応Issueが無い行

| 行 | 対応Issueが無い理由 |
|---|---|
| `HW-TBD-014` | 値の正はProtocol側（`PROTO-TBD-001`／`PROTO-TBD-002`）であり、この台帳が負うのは実機transport testの実施責任である。**そのtestを受け入れ条件に持つIssueが無い。**[#10](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/10)は上限を超えた入力の扱い、[#11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11)はbaudを設定として扱うことを受け入れ条件としており、いずれも**値の決定を条件に含まない** |
| `HW-TBD-016` | COLOR-01をMVPへ含めるかを判断するIssueが無い。行は`Deferred`であり、判断を急ぐ状態ではない |

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

### HW-TBD-027の証拠契約

**closeに必要な記録をここで一意に定める。**「実装した」「測ってLowだった」という主張だけでは
closeしない。次の4点がすべて揃うまでOpenとする。

| # | 条件 | 記録先 | 記録する内容 |
|---|---|---|---|
| 1 | 抵抗値と本数の選定 | [gpio-assignment.md](gpio-assignment.md#信号inventory)の`SERVO-PWM`行 | 決めた抵抗値・本数と、ESP32のGPIO駆動能力およびservo側入力インピーダンスから導いた根拠 |
| 2 | 購入 | [hardware-bom.md](hardware-bom.md)の`RES-PULL-01` | 型番と入手日。`残作業`の`未購入`を消す |
| 3 | 実装 | この台帳の`HW-TBD-027`行 | 実装日と接続先（`SERVO-PWM`とGNDの間であること） |
| 4 | reset中の実測 | この台帳の`HW-TBD-027`行 | 下の測定条件と、測定した電圧値 |

測定条件は次を満たす。**満たさない測定はこの行の根拠にしない。**

- **servoを接続せず**、servo電源も投入しない状態で測る。servo側の入力が並列に入ると、
  pull-downだけの効果を分離できない
- **ESP32をresetに保持した状態**で測る。GPIO27がhigh-Zになるのはこの区間であり、
  通常起動後の測定では何も確認したことにならない
- 測定点は`SERVO-PWM`の信号線とGNDの間である

**判定閾値は`HW-TBD-026`が決まるまで確定しない。**「Lowである」と言うには、SG90が
Lowと解釈する電圧の上限が要る。それは`HW-TBD-026`の`制御logic要件`である。
**したがって`HW-TBD-027`は`HW-TBD-026`より先にcloseできない。**測定値だけを記録して
「Lowだった」と結論しない。

### HW-TBD-035の証拠契約

**closeに必要な記録をここで一意に定める。**「メーカーが4.8–6 Vと言っている」という主張だけでは
closeしない。**Q&A欄の回答は、それ単独ではこの行を閉じる根拠にならない。**
次の4点がすべて揃うまでOpenとする。

| # | 条件 | 記録先 | 記録する内容 |
|---|---|---|---|
| 1 | **現物の品の特定**、または**現物に適用できるメーカー資料** | この台帳の`HW-TBD-035`行 | `SG90 Digital`／`SG90 Analog`／`SG90 360 degree`のどれであるかと、その判定根拠。**寸法の実測は3品目を除外する手段になる**（`23×12.2x29mm` 対 `23×12.5x22mm`）。**Digitalと Analogは寸法が同一のため、寸法では分離できない。**`Operating speed`も**Analog側ページが自己矛盾しているため判別に使えない**（箇条書き`0.12`／同ページの`PRODUCT CONFIGURE TABLE`は`0.1`） |
| 2 | **servo入力電圧の実測** | この台帳の`HW-TBD-035`行 | 下の測定条件と、測定した電圧値 |
| 3 | **測定条件の記録** | 同上 | 下記の条件を満たしたことと、満たせなかった条件 |
| 4 | **合否判定の基準** | 同上 | 判定に使った上限・下限と、その出所 |

測定条件は次を満たす。**満たさない測定はこの行の根拠にしない。**

- **servo railの、servoの入力端で測る。**電源側やbreadboard railの別の点で測ると、
  経路の電圧降下を含めた実際の入力電圧が分からない
- **servoが動作している状態と停止している状態の両方で測る。**起動時の突入電流で
  railが落ちるため、静止時の値だけでは入力電圧の範囲を示せない
- 給電経路は`PSU-SERVO-01`の実構成とする。**別の電源で測った値を代用しない**

**判定基準はこの行では確定していない。**「5 Vを入れてよい」と言うには、この品が受け入れて
よい電圧の上限が要る。**その上限の出所として何を採るかが、この行の未解決の本体である。**

- 現物が`SG90 360 degree`だと確定すれば、**仕様表の`4.8v-6V`が使える**
- `SG90 Digital`／`SG90 Analog`だと確定した場合、**仕様表は`4.8v`単独であり、上限の出所が無い。**
  Q&A欄の回答を採るか、メーカーへ問い合わせるか、4.8 V供給へ変えるかの**判断が要る**

**したがって条件1が解けるまで、条件4の判定基準は決まらない。**測定値だけを記録して
「5 Vで問題なかった」と結論しない。**`HW-TBD-035`は`HW-TBD-026`の品の特定に従属する。**

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

## 本文との照合手順

台帳に行が無い`TBD`はgateを素通りするため、正本文書の本文と台帳を全数照合する。
手順は再実行できる形で残し、**使用したpatternそのものをreviewの対象とする。**
過去に狭いpatternで「0件」と誤報告した例がある（[hardware-bom.md](hardware-bom.md)のRevision 19。
3語から9語へ広げて5件の漏れが出た）。

### 対象範囲

| 区分 | 対象 |
|---|---|
| 対象 | `docs/hardware/`配下、`docs/decisions/`、`docs/toolchains/` |
| 対象外 | `docs/backlog/`、`docs/toolchains/version-records/`、Issue本文 |

version-recordは実施時点のsnapshotであり、遡って改変しない。
`docs/protocol/`の未決事項は`PROTO-TBD-*`として別に管理するため、この照合には含めない
（[登録範囲](#登録範囲)を参照）。**protocol側の全数照合はまだ実施していない。**

### 抽出command

```bash
# 1) 対象範囲の全出現
grep -rn "TBD" docs/hardware docs/decisions docs/toolchains \
  --include="*.md" --exclude-dir=version-records --exclude=tbd-register.md

# 2) 識別子参照を除いた「本文のTBD」
grep -rnE "TBD" docs/hardware docs/decisions docs/toolchains \
  --include="*.md" --exclude-dir=version-records --exclude=tbd-register.md \
  | grep -vE "(HW|PROTO)-TBD-[0-9]{3}"
```

**この台帳自身を`--exclude`で外している。**台帳の`TBD`は行の識別子か、この節のような
自己言及であり、いずれも登録対象ではない。外さないと、この節を書き足すだけで件数が動き、
前回と比較できなくなる。台帳の行そのものは、定義上すでに登録済みである。

**command 2)は行単位で除外するため、識別子と本文の`TBD`が同じ行にあると本文側も落ちる。**
1)と2)の差分（識別子を含む行）を必ず開き、**その行の識別子が、同じ行の`TBD`を
指しているか**を確認する。指していなければ登録漏れである。この確認を省くと、
表の1行に複数の`TBD`セルがある文書で漏れが出る。

差分は次の2つに分かれる。目視が要るのは後者だけである。

1. 行に現れる`TBD`が**識別子だけ**の行。本文の`TBD`を含まないため、落ちるものが無い
2. 識別子を除いても`TBD`が残る行。**この行を開いて、識別子が同じ行の`TBD`の登録先かを確かめる**

2 の絞り込みは次で行う。「TBD台帳」という台帳名も識別子と同じ扱いで外す。

```bash
grep -rnE "TBD" docs/hardware docs/decisions docs/toolchains \
  --include="*.md" --exclude-dir=version-records --exclude=tbd-register.md \
  | grep -E "(HW|PROTO)-TBD-[0-9]{3}" \
  | sed -E 's/(HW|PROTO)-TBD-[0-9]{3}//g; s/TBD ?台帳|TBD register//g' \
  | grep "TBD"
```

### 区別の基準

`TBD`という語は、識別子としても、説明文中の「未確定である」という語としても使われる。
**数える前に次の3段で区別する。**

1. **識別子**: `HW-TBD-\d{3}`／`PROTO-TBD-\d{3}`の一部。台帳行への参照であり、新規登録の対象ではない。上のcommand 2)で機械的に除く
2. **本文の`TBD`**: 値または判断が未確定であることを示す記述。表のセル、箇条書き、散文を問わない
3. 2 のうち**安全・電気・機械項目**、および**下記の識別項目**に当たるものだけが登録対象である

**識別項目とは、正本文書が確定条件として挙げている部品・chipの同定である。**安全・電気・機械の
どれにも当たらないが、**満たされないと正本文書が状態を上げられない**ため追跡する。
`HW-TBD-031`（ESP32 chipの識別。**登録時の要件は「chipの刻印」であり、2026-08-15に書き換えた**）が
これに当たり、[ESP32 Rust Toolchain](../toolchains/esp32-rust-toolchain.md)が
`Verified`ではなく`build検証済み`にとどまる理由の一方である。
**この段を明記するまで、031は基準3のどれにも当てはまらないまま登録されていた**
（2026-08-11の全数分類で登録し、[PR #105](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/105)のreviewで基準側の不足を指摘されて補った）。
`HW-TBD-001`〜`005`も同じ性質を持つが、いずれもGPIO割り当てや給電経路という
電気項目を直接blockするため、基準3の1つ目で拾えている。

3 に当たらないものは対象外とし、**1件ごとに理由を残す。**次の類型が繰り返し現れる。

| 類型 | 例 | 対象外とする理由 |
|---|---|---|
| **この台帳の名前** | 「TBD台帳」「TBD register」 | 識別子と同じく台帳への参照である。**command 2)の正規表現では除けないため、目視で分ける** |
| 規則文 | 「不明値は`TBD`とする」「識別情報が`TBD`の間はdriver開発を開始しない」 | 特定の未確定値を指していない |
| 凡例 | [toolchains/README.md](../toolchains/README.md)の状態ラベル表の`TBD`行 | 語の定義であり、未確定値ではない |
| Revision履歴 | 「実GPIO割り当てはすべて引き続きTBD」 | 過去時点の記録であり、現在の未確定を表さない |
| firmwareまたは文書運用の項目 | `Firmware board configuration ID`、公式文書欄の`TBD` | 安全・電気・機械項目ではない |
| 既存行が包含するtemplateの空欄 | [sensor-datasheet-notes.md](sensor-datasheet-notes.md)の未記入セル | module単位で`HW-TBD-002`〜`005`が既に追跡している。cell単位の行を追加しない |

**この基準は`TBD`という語だけを見る。**「未選定」「未決定」「未購入」のように、
`TBD`以外の語で書かれた未確定は上のcommandに掛からない。実際、`HW-TBD-027`は
`TBD`セル（公式文書欄）ではなく同じ行の「未購入・未選定」から登録した（**この記述は2026-08-15に「一部入手済み・未選定」へ訂正した。訂正後も`TBD`の語は無く、この例が成り立たなくなったわけではない**）。
**`TBD`だけを走査した照合は、この1件を取り落とす。**

### command 3) `TBD`以外の語で書かれた未確定

patternは2群からなる。**群ごとに正本を1つだけ置く。**同じ語の一覧を2箇所に書くと、
片方だけを広げた状態で「走査した」と言えてしまう。実際に過去、狭いpatternで
「0件」と誤報告している。

| 群 | 語 | 正本 |
|---|---|---|
| A: 調達状態 | `未購入`／`購入`／`発注`／`未選定`／`未確定`／`Required`／`Blocked`／`手配`／`調達` | [hardware-bom.md](hardware-bom.md#購入待ちリスト)の発注前の走査。**この台帳では再定義しない** |
| B: 検証状態 | `未実装`／`未測定`／`未確認`／`未検証` | **この節。**部品が手元にあっても、実装・測定・確認が済んでいない状態を指す |

**群Bを発注前の走査へ足さない。**あちらの目的は発注漏れの検出であり、`未測定`や`未検証`は
買う対象を示さない。混ぜると発注時に無関係なhitを読ませることになる。

```bash
# 群A（正本はhardware-bom.md）と群B（正本はこの節）を合わせて走査する。
grep -rnE "未購入|購入|発注|未選定|未確定|Required|Blocked|手配|調達|未実装|未測定|未確認|未検証" \
  docs/hardware docs/decisions docs/toolchains \
  --include="*.md" --exclude-dir=version-records --exclude=tbd-register.md
```

**発注前の走査とこの command 3) は出力先が違う。**あちらは`購入待ちリスト`への計上漏れを探し、
こちらは台帳への**登録**漏れを探す。群Aが重なっていても判定は別であるため、
一方を実施したことをもって他方を済ませたとしない。

hitは[区別の基準](#区別の基準)の3段に掛けたうえで、次のいずれかへ分類する。**分類結果は
[実施記録](#実施記録)へ残す。**

| 分類 | 扱い |
|---|---|
| 既存の台帳行が包含する | 行番号を記録し、新規登録しない |
| 安全・電気・機械の未確定で、包含する行が無い | **新規登録する。**`HW-TBD-027`がこれに当たる |
| 発注状態のみを表す（部品は決まっており購入していないだけ） | 台帳の対象外。`購入待ちリスト`側で追う |
| 規則文・凡例・Revision履歴 | 上表の類型と同じ理由で対象外 |

### 逆方向（台帳にあるが本文から`TBD`が消えた行）

本文から`TBD`表記が消えても、台帳行を自動でcloseしない。closeは[解決手順](#解決手順)の1〜8による別の判定である。
現在該当するのは次の2種で、いずれも**行を残す**と判断した。

| 行 | 状態 | 扱い |
|---|---|---|
| `HW-TBD-008` | [gpio-assignment.md](gpio-assignment.md)は全信号へGPIOを割り当て済みで、**GPIO割り当てそのものについての`TBD`表記は残っていない**（同fileに残る`TBD`は`Firmware board configuration ID`とRevision履歴であり、いずれも別件） | 競合checkに未完了項目が残るため行を残す。この事情は同行の状態欄に記載済みである |
| `HW-TBD-012`／`HW-TBD-013` | しきい値の正本はこの台帳であり、正本文書側に`TBD`行を持たない | 実測値の正本を台帳側に置く方針（[HW-TBD-020のfield単位の正](#hw-tbd-020のfield単位の正)と同じ構造）に従い、行を残す |

### 実施記録

件数は上のcommandをそのまま実行した値である。

| 照合日 | 1) | 2) | 差分（1−2） | 差分のうち目視した行 | 追加した行 | 実施 |
|---|---|---|---|---|---|---|
| 2026-08-10 | 219 | 136 | 83 | 30。**いずれも同じ行の`TBD`を指す識別子であり、漏れ0件**。残る53行は行中の`TBD`が識別子だけで、落ちる本文`TBD`が無い | `HW-TBD-026`〜`HW-TBD-030`（5件） | [#72](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/72) |

読み方を2点補う。`grep -n`が数えるのは**出現回数ではなく行数**であり、1行に複数の`TBD`が
あっても1と数える。また2)のhitは候補であって登録対象ではない。
[区別の基準](#区別の基準)で3段に分けたうえで判定する。

件数は照合を実施したcommitに対する値である。文書を変更すれば動くため、
**次の照合では件数を突き合わせず、commandを再実行して判定する。**

追加した5行の出所は次のとおりである。**どの記述から登録したかを残さないと、次の照合で
同じ記述を二重登録するか、逆に「既に登録済み」と誤って飛ばす。**

| 行 | 出所 |
|---|---|
| `HW-TBD-026` | [servo-safety-limits.md](servo-safety-limits.md#サーボ識別情報)の`サーボ識別情報`表。`制御logic要件`・`PWM周期／rate`・`許容最小／最大pulse`が`TBD` |
| `HW-TBD-027` | [gpio-assignment.md](gpio-assignment.md#信号inventory)の`SERVO-PWM`行。**`TBD`ではなく「未購入・未選定」**から登録した（**登録時の記述である。同記述は訂正済みで、両文書とも「未購入」とは書いていない**）。[command 3)](#command-3-tbd以外の語で書かれた未確定)を設けた契機である |
| `HW-TBD-028` | [power-budget.md](power-budget.md#受け入れ条件)の受け入れ条件。電源品質の数値制限が未定義 |
| `HW-TBD-029` | [power-budget.md](power-budget.md)の`配線・保護表`。local decouplingが`TBD`／`Blocked`（**登録時点の状態。同行は2026-08-12にcloseした**） |
| `HW-TBD-030` | [power-budget.md](power-budget.md)の`配線・保護表`。逆極性保護が`TBD`／`Blocked`（**登録時点の状態。要否と方式は2026-08-24に人間が承認したが、品番・定格が未確定で、同行は`Blocked`のまま。行もcloseしていない**） |

**2026-08-10の記録は、command 2)の136件をどう分類したかを残していない。**上表の「漏れ0件」は
差分83行についての判定であり、136件の候補そのものについての判定ではない。
**2026-08-11に、command 2)の136件とcommand 3)の156件を全件分類した。**内訳を下に残す。

### 2026-08-11の全数分類

commit `52661b9` に対して実行した。**件数は文書を変更すれば動く。**次回は件数を
突き合わせず、commandを再実行して判定する。

#### command 2)（本文の`TBD`）136件

| 分類 | 件数 | 内訳 |
|---|---|---|
| 既存行が包含する（templateの空欄） | 57 | [sensor-datasheet-notes.md](sensor-datasheet-notes.md)の4 module節。`HW-TBD-002`〜`005`がmodule単位で追跡する |
| 既存行が包含する（安全・電気・機械） | 32 | servo識別情報と駆動条件 7（`HW-TBD-010`／`011`／`026`）、機械組み立て 8（`HW-TBD-010`。`PWM停止時の重力による動作`は`HW-TBD-018`の監視下機械試験）、拘束・過負荷 6（`HW-TBD-020`）、経路部品と定格 6（`HW-TBD-021`／`022`）、負荷表 1（`HW-TBD-024`）、配線・保護表 3（`HW-TBD-009`／`029`／`030`）、受け入れ条件 1（`HW-TBD-028`） |
| 既存行が包含する（BOMの列） | 9 | `Address／mode`列 4（`HW-TBD-003`／`004`／`005`／`016`）、`Peak電流`列 4（[power-budget.md](power-budget.md#測定計画)の測定計画が包含。段階ごとに`HW-TBD-007`／`021`／`022`でgate済みであり、部品ごとの行を足すと二重管理になる）、`RES-PULL-01` 1（`HW-TBD-027`／`032`） |
| Revision履歴・履歴的記述 | 13 | 過去時点の記録。「従来は`TBD`／`Blocked`だった」を含む |
| 規則文 | 12 | 「不明値は`TBD`とする」「`TBD`が残る項目は一般値を使わない」等。特定の未確定値を指していない |
| 台帳への参照 | 10 | 「追跡は`HW-TBD-021`で行う」「field単位の正は台帳に定義」等 |
| firmware／文書運用の項目 | 1 | [gpio-assignment.md](gpio-assignment.md)の`Firmware board configuration ID` |
| 凡例 | 1 | [toolchains/README.md](../toolchains/README.md)の状態ラベル表 |
| 未分類 | 0 | — |
| **新規登録** | **1** | **`HW-TBD-031`**（ESP32 chipの刻印）。[esp32-rust-toolchain.md](../toolchains/esp32-rust-toolchain.md)が「刻印の読み取りはTBD台帳の追跡対象にはなっていない」と**自ら書いていた** |

#### command 3)（`TBD`以外の語）156件

| 分類 | 件数 | 内訳 |
|---|---|---|
| 既存行が包含する | 63 | `未購入`／`未選定`／`未確定`の大半は`HW-TBD-007`／`021`／`022`／`023`／`024`／`025`／`027`が追跡する部品と値。`未確認`は`HW-TBD-001`〜`005`の現物確認 |
| Revision履歴 | 34 | 過去時点の記録。`power-budget.md`と`hardware-bom.md`のRevision履歴が長いため件数が大きい |
| 規則文・状態ラベル・確定済み記述 | 23 | 表header、`Required`／`Blocked`の凡例、確定済みの購入履歴への言及など |
| 発注状態のみ | 21 | 部品は決まっており購入していないだけ。[購入待ちリスト](hardware-bom.md#購入待ちリスト)側で追う |
| 文書運用・toolchain状態 | 13 | ADR-0002／0004／0007、[machine-profiles.md](../toolchains/machine-profiles.md)、toolchain索引。安全・電気・機械項目ではない |
| 未分類 | 0 | — |
| **新規登録** | **2行（1件）** | **`HW-TBD-032`**（`LCD-RST`／`LCD-CS`のpull-upと`LCD-BL`のpull-down）。[gpio-assignment.md](gpio-assignment.md#信号inventory)の受け入れchecklistが「**未実装のため要対応**」と書いていたが、`HW-TBD-027`は`SERVO-PWM`のpull-downだけを対象としており含んでいなかった |

群Aだけなら122件、群Bだけなら41件である（重なる行があるため合計は一致しない）。

### この分類で分かったこと

**2つの走査は、それぞれ相手が拾えないものを拾った。**

| 新規行 | 検出した走査 | もう一方では |
|---|---|---|
| `HW-TBD-031`（chip刻印） | command 2)（`TBD`） | 群Bの`未読`は走査語に含めていないため**検出できない** |
| `HW-TBD-032`（LCDの外部pull） | command 3) 群B（`未実装`） | 該当行に`TBD`の語が無いため**検出できない** |

**片方だけの走査では、どちらも取り落とす。**`HW-TBD-027`（登録時の`未購入・未選定`から登録。現在は`一部入手済み・未選定`）に続き、
`TBD`だけを見る照合では不十分であることが2件目・3件目の実例として確かめられた。

あわせて`HW-TBD-022`の記述へ`CABLE-PI-PWR-01`を明示した。同じ規則の適用対象でありながら
行の題名が`WIRE-PWR-01`だけを挙げており、包含関係が読み取れなかったためである。

**両commandの分類が完了したため、[Implementation Readiness Review](../runbooks/implementation-readiness-review.md)の
Hardware gateに対する本節由来の阻止条件は解消した。**ただしgate自体は
`HW-TBD-002`以下の未解決項目により`Fail／TBD`のままである（**`HW-TBD-001`は2026-08-15にcloseしたため、
この代表例を差し替えた**）。**分類の完了は、
「取り落としが無いことを示した」だけであって、項目が解決したという意味ではない。**

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
| HW-TBD-015 | **使用前のhealth checkを実施し、`SD-01`の健全性を確認した。**`f3`が書き込んだ全空き領域（29.80 GiB）へ位置依存dataを書いて読み戻し、`Data LOST`は0 sectorであった（**容量詐称は確認されず、検査範囲に読み書き不良も確認されなかった**）。判定は`Partial`である。**この行の`必要な根拠`は`health checkの実施`であり、それを満たした。****`妨げる対象`にあった「耐久性」は、この行が未解決だと妨げられる対象であって、測定すべき項目ではない。**2026-08-15までこの行を「耐久性を別途測定する」と読み違えてopenのまま残していたが、誤りであった（[PR #128](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/128)）。**判定基準③（速度）と`manfid`によるメーカー照合は、いずれもこの行の`必要な根拠`に含まれず、追跡を打ち切っている** | [sd-health-check.md](sd-health-check.md)、[hardware-bom.md](hardware-bom.md) SD-01、[#114](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/114) | 2026-08-15 |
| HW-TBD-006 | 正確なservo modelを`TowerPro Micro servo 9g SG90`と確定した。定格電圧4.8–6 V、stall電流はデータシート値0.5–2 A（負荷依存）。**Peak／stall電流の実測は本行の範囲外**であり、`power-budget.md`の測定計画と`HW-TBD-010`／`HW-TBD-011`で追跡する | 現物ラベルの写真（`TOWER PRO Micro servo 9g SG90`）、[TowerPro公式](https://towerpro.com.tw/product/sg90-7/)、[datasheet](https://www.mouser.com/catalog/specsheets/Soldered_101246.pdf)。反映先: [hardware-bom.md](hardware-bom.md) SERVO-01、[power-budget.md](power-budget.md)負荷表、[gpio-assignment.md](gpio-assignment.md) SERVO-PWM | 2026-08-05 |
| HW-TBD-029 | **各deviceのlocal decouplingについて、datasheetが指定する値と配置を確定した。**`ENV-01`（AE-BME280）は`C1`／`C2`とも0.1 µFをmodule上に実装済みで、Bosch datasheetの推奨100 nFを満たすため**外付けは不要である**。`ACCEL-01`（ADXL345 IC）は`CS` 1 µF tantalum＠`VS`／`CI/O` 0.1 µF ceramic＠`VDD I/O`をsupply pin近傍へ、追加が要れば100 Ω以下の抵抗かferrite beadを`VS`と直列に、`VS`のbypassは10 µF tantalum ∥ 0.1 µF ceramicへ増やす。**`CS`は推奨値1 µFを採ることを根拠付きで決めた**（Table 1の10 µFは測定条件であって設計要件ではない）。`DISP-01`（MSP2807）は**メーカーdatasheetに記載が無いことを確認した**。`MCU-01`は参照設計で`U2`入出力とも22 µFである。**残る「module boardに何が実装済みか、外付けが要るか」は現物確認であり、この行では追わない。**datasheetでは決まらないためであり、`HW-TBD-004`（`ACCEL-01`）、`HW-TBD-002`（`DISP-01`）、`HW-TBD-023`（`MCU-01`）の確認項目へ移した。**gateは維持している**（3行とも未解決であり、電源系の受け入れ承認を引き続き塞ぐ） | [sensor-datasheet-notes.md](sensor-datasheet-notes.md)の`Local decoupling`節（値と出典の正）、[power-budget.md](power-budget.md)の`local decouplingの外付け要否`節（要否の判断）。一次資料は[ADXL345 Data Sheet](https://www.analog.com/media/en/technical-documentation/data-sheets/adxl345.pdf) Rev. G `POWER SUPPLY DECOUPLING` page 29、[BME280 Data Sheet](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf) Revision 1.24 Figure 17–19、[AE-BME280説明書](https://akizukidenshi.com/goodsaffix/AE-BME280_manu_v1.1.pdf) v1.1 部品表、[msp2807.pdf](https://akizukidenshi.com/goodsaffix/msp2807.pdf) Product Parameters | 2026-08-12 |
| HW-TBD-001 | **公式pin表と現物pin表記の照合が一致した。**38pinヘッダ両側のsilkがEspressif公式`J2`／`J3`のpin description表と**19pin×2列すべてで一致**した（GNDの位置を含む）。左列（`J2`）`3V3 EN VP VN 34 35 32 33 25 26 27 14 12 GND 13 D2 D3 CMD 5V`、右列（`J3`）`GND 23 22 TX RX 21 GND 19 18 5 17 16 4 0 2 15 D1 D0 CLK`。**これによりGPIO割り当ての前提が成立した。**module suffix（ESP-WROOM-32D）と「基板にrevision表示なし」も現物確認済みで、**module刻印**（`ESPRESSIF` / `ESP32-WROOM-32D`）も読了した。**旧記載の「秋月独自基板のため一致する保証がない」は根拠が無く、2026-08-10に削除済みである** | 現物写真（2026-08-13、斜光＋接写。写真はrepositoryへ置かない）と[ESP32-DevKitC V4公式回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)（`J2`／`J3`はいずれも`CON19X1_2P54`で計38pin）。[公式guide](https://docs.espressif.com/projects/esp-idf/en/v5.1/esp32/hw-reference/esp32/get-started-devkitc.html)のpin description表も同じ並びを示すが、**番号の正は回路図とする**（HTML経由の取得時に`J3`の一部で番号表示に乱れがあったため）。反映先: [hardware-bom.md](hardware-bom.md) `MCU-01`、[gpio-assignment.md](gpio-assignment.md#board識別情報)、[esp32-rust-toolchain.md](../toolchains/esp32-rust-toolchain.md)の確定条件 | 2026-08-15 |
| HW-TBD-003 | **touch controllerを`XPT2046`と確定した。**現物裏面`U2`（TSSOP-16）の刻印（`XPT`ロゴ、`XPT2046`、ロット`ABDEAB`）による。**メーカーdatasheetには型番の記載が無いため、刻印が唯一の根拠である。**「`XPT2046`系と推定、未確認」は推定でなくなった。**driverに要る動作仕様（SPI mode、最大bus速度、raw出力、calibration、IRQのedge／level）は`XPT2046`のdatasheetを当てる段階へ移った**（[sensor-datasheet-notes.md](sensor-datasheet-notes.md)のTouch controller節に`TBD`として残る） | 現物写真（2026-08-13、斜光＋接写。写真はrepositoryへ置かない）。型番がdatasheetに無いことの確認は[msp2807.pdf](https://akizukidenshi.com/goodsaffix/msp2807.pdf)（2026-08-10）。反映先: [hardware-bom.md](hardware-bom.md) `TOUCH-01`、[sensor-datasheet-notes.md](sensor-datasheet-notes.md)、[gpio-assignment.md](gpio-assignment.md)の`LCD-MISO`／`TOUCH-CS` | 2026-08-15 |
