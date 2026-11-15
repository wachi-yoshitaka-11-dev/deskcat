# ハードウェア設計データ

このディレクトリは編集可能な図面を置く。型番、GPIO、抵抗値、電源条件、安全制限、実験結果の正本は [`docs/hardware/`](../docs/hardware/README.md) にある。**配線を変える PR は、同じ PR で KiCad 回路図とブレッドボード配線図も更新する。** 実験記録の `Wiring revision` の行を変えて配線図を更新しない PR は、`scripts/check_wiring_figure_sync.py` が CI で落とす（図に関係しない場合の宣言は [CONTRIBUTING](../CONTRIBUTING.md) の「ハードウェア実験」）。 **図は最終的な配線を描く。測定のためだけの一時配線は描かず、`Wiring-Figure: not-applicable <理由>` で通す。** 図は 3 つ（`breadboard/layout.svg`、`universal-board/` の電源の基板の図、`overview/layout.svg`）である。電源の基板の図は計画で、[#405](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/405) で付けた後に現物と照合して完成図にする。`overview/layout.svg` は最終形のつながりを回路図の netlist から生成する。この検査は配線図の更新だけを要求する。穴位置や線色の変更は論理回路を変えないためで、論理接続が変わる場合の KiCad 回路図の更新は人が行う。 図と正本が食い違う場合は変更を止めて照合する。

| データ | 対象と状態 |
| --- | --- |
| [`deskcat.kicad_pro`](deskcat.kicad_pro)、[`deskcat.kicad_sch`](deskcat.kicad_sch) | #512 の論理回路。EXP-019 測定前の基準配線に、EXP-022 の UART と EXP-025 の LED 2枝を加えた設計上の合成図。抵抗の `Value` は数値の代わりに値の正本文書名を示す。2026-11-15 に、`PROTO-02` の電源の経路（入力 → 端子台 → PTC → MOSFET → 分岐点 → servo・Pi・logic の枝。未実装の計画）を同じ図に足した。pin 番号の出所は下の`回路図の pin 番号`。KiCad 8.0.9 の `kicad-cli` で読み込み、ERC を実行した（下の`KiCad の ERC`）。 |
| [`universal-board/`](universal-board/plan.md)、[`deskcat.kicad_pcb`](deskcat.kicad_pcb) | `PROTO-02`（電源の基板）の**実装の計画（未実装）**。どの穴にどの部品をどの向きで挿し、どこをはんだでつなぎ、どの線をどこへ出すか。**正本は KiCad の基板ファイル `deskcat.kicad_pcb`**（回路図と同じ project）。図 `plan-front.svg`・`plan-back.svg` は KiCad から書き出した派生物で、`plan.md` は凡例・向き・測る所である。GND の star point の規則は `check_star_point.py` が基板ファイルを読んで確かめる。**完成図ではない。**#405 のはんだ付けはこの計画に従い、付けた後に現物と照合して完成図にする。 |
| [`overview/`](overview/wiring-table.md) | 最終形の全体の配線（ブレッドボード B1・B2・B3、`PROTO-02`、Pi、サーボ、LCD、M-12001）のつながり。位置は胴の中の位置関係だけで、寸法は表さない。`generate.py` が回路図から書き出した netlist（`deskcat-netlist.xml`）を入力にして `layout.svg` と `wiring-table.md` を生成し、図の線と netlist が合うことを検査する。板どうしの線はブレッドボードの `breadboard-rebuild/generate.py` のデータを読み込む。#18 の試験だけの物（端子台S、PSU、試験用の負荷、shunt）は描かない。 |
| [`breadboard/layout.svg`](breadboard/layout.svg) | 最終の配線の図。2026-10-31（EXP-052）に、組み直した後の配置（`breadboard-rebuild/layout.svg`）で置き換えた。測定のためだけの一時配線（EXP-030〜035 の Arduino・分圧・シャント・測定用の戻り線）は描かない。柔らかい線の色と穴は `breadboard-rebuild/wiring-table.md` が正である。組み直す前の図は Git の履歴にある。 |
| [`breadboard-rebuild/`](breadboard-rebuild/README.md) | #512 の組み直しの配置（2026-10-31 に組んだ。EXP-052）。配置図 `layout.svg` と結線表 `wiring-table.md` は `generate.py` のデータから生成する。組み直した後のこの配置で、上の図を置き換えた。組み直す前の図は Git の履歴にある。 |
| [`bench-18/`](bench-18/wiring-table.md) | #18 の窓と段1の servo 電源の経路（`PSU-BENCH-01` → 端子台 A・B・S → `MF-R135` → シャント → jumper wire → (D)・(B) の負荷／段1の servo の plug）の bench の図。正本は `generate.py` のデータで、結線表 `wiring-table.md`（節点の表）と `layout.svg` はそこから生成する。**#405 の図とは別物で、`breadboard/layout.svg` は置き換えない。**`MEAS-02` の線は測定のためだけの一時配線なので描かない。 |
| [`wokwi/esp32-boot-pulls/diagram.json`](wokwi/esp32-boot-pulls/diagram.json) | 起動時 GPIO と外部 pull の初期評価用。ESP32、pull、LED の枝、ILI9341 互換表示部だけを配置した。全機器をシミュレーションした構成ではない。 |

基準図①は、ユーザーの配線地図と回答から AI が作り、ユーザーが照合した版である。SVG 本文の SHA-256 は末尾改行を除いて `a91e6d1e669258f477fcde5e0d77fda64260d6a0d322a3c12491654c39a985fa` と一致した。基準時点は EXP-019 の測定前で、測定のためだけに抵抗やジャンパを挿した状態は含めない。EXP-021・EXP-022 は `ACCEL-01`、`ENV-01`、`DISP-01` の配線が戻した状態から変わっていないと記録する。Pi の給電差し替え（EXP-020、EXP-023）は USB 端子側であり、ブレッドボードの変更ではない。

#512 コメントの `RES-PULL-01` と `HW-TBD-032` の食い違いは、現行の [Hardware BOM](../docs/hardware/hardware-bom.md) と [TBD Register](../docs/hardware/tbd-register.md) を照合した。BOM は後の改訂で、当初の4本に `LCD-BL` の1本を加えた確定5本と実装済みの状態を記録している。`HW-TBD-032` も close 済みである。BOM の「計4本」は当初の選定経緯を指し、現行の総数ではない。GPIO Assignment に残っていた `LCD-CS`／`LCD-RST` の古い状態説明を、2026-09-07 の `EXP-011` と照合して訂正した。

EXP-022 の UART は [GPIO Assignment](../docs/hardware/gpio-assignment.md) の GPIO と接続条件、および [実験記録](../docs/hardware/experiment-log.md) に従う。抵抗の位置は EXP-022 が「紫の線（Pi の pin 8、GPIO14 `TXD`）に `h1`–`h4`、青の線（pin 10、GPIO15 `RXD`）に `i2`–`i5`、各線に 4.7 kΩ が1本」と記録している。抵抗の両端につながる線の穴位置は記録に無く TBD である。GND は Pi pin 39 と ESP32 の J3 の GND を結ぶ。UART 信号線を接続してよい条件は正本を毎回参照する。

EXP-025 の [LED デモ回路](../docs/hardware/led-514-demo.md)は GPIO → 電流制限抵抗 → LED アノード → カソード → GND、GPIO → pull-down → GND の2枝である。GND rail の導通はユーザー申告で、実測されていない。EXP-025 の点灯試験中は UART 信号線の ESP32 側を外した。[EXP-026](../docs/hardware/experiment-log.md)では LED の GPIO 線を外している。この合成図を、UART と LED を同時に接続・通電して検証した証拠として使わない。

## 設計データの層と、このディレクトリのファイル

電子回路の設計データは、論理（何と何がつながるか）、物理（どこに置き、どこをはんだでつなぐか）、部品の定義、製造の4つの層に分かれ、論理から物理へ写して作る。DeskCat の今の構成（ブレッドボードと、既製のユニバーサル基板1枚）で要るものと、そのファイルは次のとおりである。

| 層 | データ | このリポジトリ | 状態 |
|---|---|---|---|
| 論理 | 回路図 | [`deskcat.kicad_sch`](deskcat.kicad_sch)（信号の側と、`PROTO-02` の電源の経路） | あり。ESP32 の 5 V 入力と LCD の VCC は `TBD`（#405） |
| 論理 | netlist（どの部品のどの pin がどの節点か） | 回路図から `kicad-cli sch export netlist` で出す。Git には置かない（回路図から作れる派生物のため）。基板ファイルとの照合は KiCad の DRC（`--schematic-parity`）が行う | 回路図から出せる |
| 物理 | 試作のブレッドボードの配線 | [`breadboard/layout.svg`](breadboard/layout.svg)、[`breadboard-rebuild/wiring-table.md`](breadboard-rebuild/wiring-table.md) | あり（2026-10-31 の組み直し） |
| 物理 | 試作の端子台の配線（#18 の bench） | [`bench-18/`](bench-18/wiring-table.md) | あり。#18 の試験の間だけの配線 |
| 物理 | はんだ付けする基板の実装図 | KiCad の基板ファイル [`deskcat.kicad_pcb`](deskcat.kicad_pcb)（計画）と、書き出した図・凡例の [`universal-board/`](universal-board/plan.md) → 照合後に完成図（#405） | 計画あり。DRC（回路図との照合を含む）で違反 0 |
| 物理 | 全体の配線（基板どうしと胴の中） | [`overview/layout.svg`](overview/layout.svg)、[`overview/wiring-table.md`](overview/wiring-table.md) | あり。回路図から書き出した netlist（`overview/deskcat-netlist.xml`）から `overview/generate.py` で生成し、netlist と図の線が合うことを検査する |
| 部品の定義 | symbol（回路図の記号） | project の library [`DeskCat.kicad_sym`](DeskCat.kicad_sym)（`sym-lib-table`）。抵抗は KiCad 標準の `Device` | あり |
| 部品の定義 | footprint（はんだ付けする足の位置と形） | project の library [`DeskCat.pretty/`](DeskCat.pretty)（`fp-lib-table`）。`PROTO-02` に載る部品だけ | あり。寸法の出所は [`universal-board/plan.md`](universal-board/plan.md) |
| 部品の定義 | SPICE のモデル | 無い | 要らない。起動時の pull の解析に具体的な課題が無い（#512 の本文） |
| 製造 | BOM（部品表） | [`docs/hardware/hardware-bom.md`](../docs/hardware/hardware-bom.md) | あり（手編集の正本は1つ） |
| 製造 | Gerber・drill・Pick & Place・3D（STEP） | 無い | 要らない。工場で基板を作らず、筐体との寸法連携は #34・#405 の現物合わせで行う（#512 の本文）。必要になれば基板ファイルから KiCad で出せる |

**論理と物理のつながり：**回路図・基板ファイル・部品の library は1つの KiCad project（`deskcat.kicad_pro`）である。基板の部品の参照番号（`J5`〜`J8`、`F1`、`Q1`、`R18`）と節点は回路図から来ており、`kicad-cli pcb drc --schematic-parity hardware/deskcat.kicad_pcb` が食い違いを検出する。#512 の本文は「専用 PCB の `.kicad_pcb` は成果物に含めない」としている。ここでの `.kicad_pcb` は既製のユニバーサル基板の上の配置の計画であり、工場へ出す基板ではない（Gerber 等は作らない）。データの形式を業界標準の KiCad に揃えるために使う。KiCad の個人設定（`*.kicad_prl`）とバックアップは [`.gitignore`](.gitignore) で Git に入れない。

## 回路図の pin 番号

- `MCU-01` の header 番号（`J2-*`・`J3-*`）は、[GPIO Assignment](../docs/hardware/gpio-assignment.md) の`公式回路図revision`行（Espressif の ESP32-DevKitC V4 公式回路図と、現物の silk の照合）による。`GPIO34`（`TOUCH-IRQ`）の `J2-5` は、同行の左列の5番目が `34` であることと、公式回路図（`esp32_devkitc_v4-sch.pdf`、sha256 `92961687b5fac64b8870ddc2f5e8528351e515b6cafd49e7d1bba4fb974a7349`）の `J2` の pin 5 が `IO34` であることの両方で確かめた（2026-11-15）。
- `ENV-01` の pin 番号 1〜6 は、[AE-BME280 製品説明書](https://akizukidenshi.com/goodsaffix/AE-BME280_manu_v1.1.pdf)（v1.1、sha256 `903c2e49064f279ff2738391ca6beb24a70a3d91a4e2118621381faef66a1e76`）の`ピン番号と機能`（1 `VDD`、2 `GND`、3 `CSB`、4 `SDI`、5 `SDO`、6 `SCK`）による。pin 3（`CSB`）は module の上の `J3` で `VDD` へつなぐため、回路図では線を引いていない（netlist では未接続の pin になる）。
- `ACCEL-01`（秋月 M-06724）は、製造元の pin 番号が無い。商品ページは 404 で、module の資料が無い（[Sensor Datasheet Notes](../docs/hardware/sensor-datasheet-notes.md) の`module boardの値（秋月 M-06724）`）。**回路図の pin 番号には、header の silk の表記（`Vs`、`GND`、`SCL`、`SDA`、`CS`、`SDO`、`INT1`、`INT2`）をそのまま使った。**silk の表記は module の上で1つずつ違うので、どの端子かは一意に決まる。header の並び順に番号を振ることはしない（端からの順序の資料が無い）。

## KiCad の ERC

2026-11-15、KiCad 8.0.9 の `kicad-cli` で回路図を読み込み、ERC を実行した（環境は [Version Record](../docs/toolchains/version-records/2026-11-15-kicad-erc-vm.md)）。**Errors 0、Warnings 165** である（電源の経路、サーボ信号の直列抵抗 R19、project の library を足した後）。警告の内訳は、pin と wire の端が接続の grid の外にある（`endpoint_off_grid`）146件と、抵抗の記号の KiCad 標準 library `Device` がこの VM に入っていない（`lib_symbol_issues`、記号は回路図に埋め込み済み）19件で、どちらも接続を変えるものではない。netlist を出力し、`MCU-01` の各 pin の net が上の header 番号と [GPIO Assignment](../docs/hardware/gpio-assignment.md) の GPIO に合うこと、`ACCEL-01` の `SDO` と `ENV-01` の `SDO` が GND（address `0x53` と `0x76`。EXP-065 の起動 log の `accel=0x53 env=0x76`）にあることを確かめた。

基板ファイルの DRC（`kicad-cli pcb drc --schematic-parity --severity-all`）は、違反 0、未接続 0、回路図との食い違い 0 だった。

**この ERC が保証しないこと：**symbol の pin はすべて passive なので、出力どうしの衝突のような pin の種別の検査は効かない。GUI での表示は確かめていない。

## Wokwi と物理図の評価

[Wokwi の diagram.json 形式](https://docs.wokwi.com/diagram-format)は JSON で部品と pin 間の接続を編集できる。抵抗の `value` 属性には数値が必須であり、[GPIO Assignment](../docs/hardware/gpio-assignment.md) と [LED デモ回路](../docs/hardware/led-514-demo.md)を写した値を入れている。値の変更時は正本と同じ PR で照合する。[Wokwi の抵抗モデル](https://docs.wokwi.com/parts/wokwi-resistor)は簡易的なアナログモデルで、起動時の pull を配置できても実機の起動挙動は立証できない。

**ブラウザで試した結果（2026-11-15）。**ログインせずに `wokwi.com` の新しい ESP32 project を開き、`diagram.json` を貼り替え、sketch を「外部 pull のある pin を入力にして読み、serial へ出す」ものに替えて simulation を始めた。project は保存していない（保存にはログインが要る）。API トークンは使っていない。

- 表示：`diagram.json` を読み込み、ESP32・抵抗・LCD と線が描かれた（画面に入った範囲を見た）。
- 起動：ROM の log は `rst:0x1 (POWERON_RESET),boot:0x13 (SPI_FAST_FLASH_BOOT)` で、sketch まで進んだ。読んだ値は、pull-down の GPIO2・4・5・27 が 0、pull-up の GPIO16・21・22・25・26 が 1 で、図の pull の向きと合った。
- **boot mode は再現されない。**`GPIO0` を GND へ直結する線を足して起動し直すと、sketch は `GPIO0=0` を読んだが、ROM の log は同じ `boot:0x13 (SPI_FAST_FLASH_BOOT)` のままだった（download mode に入らない）。Wokwi は strapping pin の電圧から boot mode を決めていない。実機の起動 log は `boot:0x12`（EXP-052）で、値も一致しない。
- したがって Wokwi で言えるのは「外部 pull の論理接続を、起動後に入力として読むと図の向きになる」ことまでで、strapping pin の起動挙動は確かめられない。

Wokwi の [ILI9341 部品](https://docs.wokwi.com/parts/wokwi-ili9341)では RST と backlight はシミュレーションされない。`DISP-01` の touch は XPT2046 だが同部品の touch とは一致しない。Wokwi の汎用 LED も `LED-COMM-01`／`LED-REACT-01` の電気特性を保証するモデルではない。`ACCEL-01` の ADXL345、`ENV-01` の BME280、Pi Zero W の UART、ブレッドボードの実際の穴位置は、この評価構成にモデル化していない。外部 pull の論理接続には Wokwi を使い、実際の穴位置には SVG を使う。同じ回路を Wokwi と Fritzing で二重に手編集しない。

### 採否（2026-11-15）

| 観点 | Wokwi | Fritzing | KiCad と、生成した SVG |
|---|---|---|---|
| 必要な配線の表現 | ESP32・抵抗・LED・ILI9341 の論理接続は表せる。`ACCEL-01`・`ENV-01`・XPT2046 は標準部品に無い。strapping の起動挙動は再現されない（上） | ブレッドボードの穴と部品の形は表せるが、`ACCEL-01`（M-06724）・`ENV-01`（AE-BME280）・`DISP-01`（MSP2807）・端子台 `114217` の部品図は、実物に合うものを作るか探して端子対応とライセンスを確かめる必要がある | 回路図と `PROTO-02` は KiCad。ブレッドボードの穴と線の色（`breadboard-rebuild/`）と bench の節点（`bench-18/`）は、Python のデータから生成した SVG と結線表 |
| 実物との端子対応 | 部品の pin 名は Wokwi の model の名前で、実物の silk とは別。DevKitC V4 の `GND.1` のような名前を写し替える | 部品図ごとに作る必要がある（上） | 一次資料と現物の silk の名前を、記号・footprint・データにそのまま書いている |
| 編集・保存のしやすさ | JSON を Git で差分にできる。ブラウザでの保存はログインが要る（試していない） | `.fzz` は zip の binary で、差分を読めない。この VM には入っていない | KiCad の file は文字の S 式で、Git で差分を読める。ブレッドボードの SVG は生成と検査を `--check` で機械で回せる |

**Wokwi は、外部 pull の論理接続の評価構成としてだけ使い続ける**（`esp32-boot-pulls/` のまま広げない）。起動の安全の確認には使わない。**Fritzing は採らない。**実物（秋月の module・端子台）に合う部品図を探すか作り、端子対応とライセンスを確かめる手間が要り、それでも `.fzz` は差分を読めない。`PROTO-02` は KiCad で、ブレッドボードは生成した SVG で表せている。Fritzing の部品 library に実物に合う図があるかは、この判断では探していない。`hardware/deskcat.fzz` と `hardware/fritzing/` は作らない。ブレッドボードの図と KiCad の回路図との接続差分は、結線表（`wiring-table.md`）と netlist を照らして見る。

## 現物照合待ち（TBD）

- `PROTO-02` の現物：計画（[`universal-board/plan.md`](universal-board/plan.md)）は描いた。**計画図を先に描き、それに従ってはんだ付けし、付けた後に現物と照合して完成図にする（#405）。**基板の未実装状態を完成図として描かない。計画の`測る所`の寸法は、まだ測っていない。
- 回路図の GUI での表示（`kicad-cli` での読み込みと ERC は済み）。実機の電源・安全確認は図やシミュレーションで代替できない。

### 閉じた項目

- **`ENV-01` の `CSB`–`VDD` はんだジャンパ `J3` の導通：測らずに閉じた（2026-11-15）。**根拠は次の3つである。
  1. [EXP-014](../docs/hardware/experiment-log.md#exp-014-env-01bme280のj3はんだジャンパの実施記録)：`J3` ははんだ付け済み（2026-09-07）。確認の方法は写真とテスターである（申告の記録）。**測定値は無い。**
  2. EXP-052 の起動で `env_chip_id raw=0x60`（BME280）が読め、EXP-065 の起動 log は `env=0x76`、温湿度は Pi へ1分ごとに送られた。**これは `J3` の導通を測ったことではない。**言えるのは「それらの起動のたびに、`CSB` が `VDD` の側にあった」ことである。BME280 は `CSB` が `VDDIO` につながると I2C、一度 low に引かれると次の power-on-reset まで I2C を止める（Bosch BME280 Data Sheet、`BST-BME280-DS001-24`、Revision 1.24 の 6.1 `Interface selection`）。`J3` の抵抗値や、はんだの状態は分からない。
  3. `J3` は安全要件の5項目に効かない（[Hardware Safety Policy](../docs/governance/hardware-safety-policy.md)）。`J3` が切れて起きるのは I2C で `ENV-01` が応答しなくなることで、電源・servo の安全制限には関わらない。
- Wokwi のブラウザ表示と起動時の挙動、KiCad でのファイル読み込みと ERC（上の各節）。

写真と地図の画像は Git に入れない。大きな製造用生成物、vendor binary、第三者文書も追跡方法とライセンスを決めずに追加しない。
