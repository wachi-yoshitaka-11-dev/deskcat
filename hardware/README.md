# ハードウェア設計データ

このディレクトリは編集可能な図面を置く。型番、GPIO、抵抗値、電源条件、安全制限、実験結果の正本は [`docs/hardware/`](../docs/hardware/README.md) にある。**配線を変える PR は、同じ PR で KiCad 回路図とブレッドボード配線図も更新する。** 実験記録の `Wiring revision` の行を変えて配線図を更新しない PR は、`scripts/check_wiring_figure_sync.py` が CI で落とす（図に関係しない場合の宣言は [CONTRIBUTING](../CONTRIBUTING.md) の「ハードウェア実験」）。この検査は配線図の更新だけを要求する。穴位置や線色の変更は論理回路を変えないためで、論理接続が変わる場合の KiCad 回路図の更新は人が行う。 図と正本が食い違う場合は変更を止めて照合する。

| データ | 対象と状態 |
| --- | --- |
| [`deskcat.kicad_pro`](deskcat.kicad_pro)、[`deskcat.kicad_sch`](deskcat.kicad_sch) | #512 の論理回路。EXP-019 測定前の基準配線に、EXP-022 の UART と EXP-025 の LED 2枝を加えた設計上の合成図。抵抗の `Value` は数値の代わりに値の正本文書名を示す。`ACCEL-01`・`ENV-01` の KiCad pin 番号の `TBD-*` は物理端子番号ではない。 |
| [`breadboard/layout.svg`](breadboard/layout.svg) | 最終の配線の図。測定のためだけの一時配線（EXP-030〜035 の Arduino・分圧・シャント・測定用の戻り線）は描かない。基準は [#512 の図①](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/512#issuecomment-5951254069)。EXP-022（UART）・EXP-025（LED）・EXP-028（サーボの信号と電源の経路。端子台・PTC・M-12001・サーボは板の外）・EXP-038（touch の 5 本）を足した。LED の脚は [#514 の 2026-10-20 の申告](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514#issuecomment-6291626373)、抵抗・GPIO2・GPIO5 の線は同 comment の 2026-10-12 の申告、黄の線2本・基準線の色・UART の穴・e28–f28 は 2026-10-22 のユーザーの申告である。出所は右パネルに一覧。穴位置を推定して描いていない。 |
| [`wokwi/esp32-boot-pulls/diagram.json`](wokwi/esp32-boot-pulls/diagram.json) | 起動時 GPIO と外部 pull の初期評価用。ESP32、pull、LED の枝、ILI9341 互換表示部だけを配置した。全機器をシミュレーションした構成ではない。 |

基準図①は、ユーザーの配線地図と回答から AI が作り、ユーザーが照合した版である。SVG 本文の SHA-256 は末尾改行を除いて `a91e6d1e669258f477fcde5e0d77fda64260d6a0d322a3c12491654c39a985fa` と一致した。基準時点は EXP-019 の測定前で、測定のためだけに抵抗やジャンパを挿した状態は含めない。EXP-021・EXP-022 は `ACCEL-01`、`ENV-01`、`DISP-01` の配線が戻した状態から変わっていないと記録する。Pi の給電差し替え（EXP-020、EXP-023）は USB 端子側であり、ブレッドボードの変更ではない。

#512 コメントの `RES-PULL-01` と `HW-TBD-032` の食い違いは、現行の [Hardware BOM](../docs/hardware/hardware-bom.md) と [TBD Register](../docs/hardware/tbd-register.md) を照合した。BOM は後の改訂で、当初の4本に `LCD-BL` の1本を加えた確定5本と実装済みの状態を記録している。`HW-TBD-032` も close 済みである。BOM の「計4本」は当初の選定経緯を指し、現行の総数ではない。GPIO Assignment に残っていた `LCD-CS`／`LCD-RST` の古い状態説明を、2026-09-07 の `EXP-011` と照合して訂正した。

EXP-022 の UART は [GPIO Assignment](../docs/hardware/gpio-assignment.md) の GPIO と接続条件、および [実験記録](../docs/hardware/experiment-log.md) に従う。抵抗の位置は EXP-022 が「紫の線（Pi の pin 8、GPIO14 `TXD`）に `h1`–`h4`、青の線（pin 10、GPIO15 `RXD`）に `i2`–`i5`、各線に 4.7 kΩ が1本」と記録している。抵抗の両端につながる線の穴位置は記録に無く TBD である。GND は Pi pin 39 と ESP32 の J3 の GND を結ぶ。UART 信号線を接続してよい条件は正本を毎回参照する。

EXP-025 の [LED デモ回路](../docs/hardware/led-514-demo.md)は GPIO → 電流制限抵抗 → LED アノード → カソード → GND、GPIO → pull-down → GND の2枝である。GND rail の導通はユーザー申告で、実測されていない。EXP-025 の点灯試験中は UART 信号線の ESP32 側を外した。[EXP-026](../docs/hardware/experiment-log.md)では LED の GPIO 線を外している。この合成図を、UART と LED を同時に接続・通電して検証した証拠として使わない。

## Wokwi と物理図の評価

[Wokwi の diagram.json 形式](https://docs.wokwi.com/diagram-format)は JSON で部品と pin 間の接続を編集できる。抵抗の `value` 属性には数値が必須であり、[GPIO Assignment](../docs/hardware/gpio-assignment.md) と [LED デモ回路](../docs/hardware/led-514-demo.md)を写した値を入れている。値の変更時は正本と同じ PR で照合する。[Wokwi の抵抗モデル](https://docs.wokwi.com/parts/wokwi-resistor)は簡易的なアナログモデルで、起動時の pull を配置できても実機の起動挙動は立証できない。ブラウザでの表示と boot mode の再現試験は未実施である。

Wokwi の [ILI9341 部品](https://docs.wokwi.com/parts/wokwi-ili9341)では RST と backlight はシミュレーションされない。`DISP-01` の touch は XPT2046 だが同部品の touch とは一致しない。Wokwi の汎用 LED も `LED-COMM-01`／`LED-REACT-01` の電気特性を保証するモデルではない。`ACCEL-01` の ADXL345、`ENV-01` の BME280、Pi Zero W の UART、ブレッドボードの実際の穴位置は、この評価構成にモデル化していない。外部 pull の論理接続には Wokwi を使い、実際の穴位置には SVG を使う。Fritzing は専用部品図の端子対応とライセンス、KiCad との差分維持を確かめる必要があるため採用を保留する。同じ回路を Wokwi と Fritzing で二重に手編集しない。

## 現物照合待ち（TBD）

- UART の線の穴位置（Pi 側 g1 紫・g2 青、ESP32 側 j4 橙・j5 黄）は 2026-10-22 のユーザーの申告で、抵抗と信号の対応は EXP-022 の記録である。現物照合は未実施。
- LED 2枝は、抵抗（赤枝 330 Ω `h8–h10`・10 kΩ `g10–g7`、白枝 330 Ω `h13–h15`・10 kΩ `g15–g12`）と GPIO5（茶→`j10`）・GPIO2（灰→`j15`）の線を 2026-10-12 の申告の位置で、黄の線2本（`f7`↔上の－14、`f12`↔上の－15）を 2026-10-22 の申告で、実線で描いた。LED の脚は 2026-10-20 の申告（白 長 `i13`・短 `i12`、赤 長 `i8`・短 `i7`）で、2026-10-12 の位置と同じである。当日 13:33 の変更と EXP-026 の後の今の位置は申告されていない。GPIO5・GPIO2 の線は ESP32 側を外している（EXP-026）。同 comment は 13:33 に茶を外したと記録している。最終の配線は同じ、というのは 2026-10-22 のユーザーの指摘（EXP-037 の記録）で、図は 2026-10-12 の位置と色で描いた。LED 極性の申告と値の正本は [EXP-025](../docs/hardware/experiment-log.md)を参照する。
- EXP-026〜EXP-034 の線のうち、測定のためだけの線（EXP-030〜035）は描かない。最終配線として描いたのは、基準線（上の－20→端子台 S の－。黒）、`i25` の黄（EXP-028。2026-10-16 20:01:27 の原文が裏付ける）、330 Ω `h25–h28`、GPIO27 の `d28` と `h28` をつなぐ `e28–f28`（2026-10-22 の申告）である。端子台 S・A・B・PTC・M-12001・サーボ・線 W+・W− は板の外に描いた（端子台3つは板に載せていない。2026-10-22 の申告）。線色の記録が無いもの：サーボの線の色の意味（未確認、#17）。EXP-034 の `g11` の茶の針と MEAS-01 は測定用で描かない。ADC 分圧点の線色は、図①（2026-10-02）が 5V 黒・3V3 灰、2026-09-07 の実装記録（gpio-assignment.md）が 5V 青・3V3 白で、変更時点の記録は無い。
- 上の電源・GND rail の途中の分断の有無、基準図から復帰後の非通電確認、各抵抗の現物表示と位置の対応、`ENV-01` の `CSB`–`VDD` はんだジャンパ `J3` の導通。
- `PROTO-02` の実装する部品、表裏、穴、はんだ接続、線材、基板外の接続点。物理実装図は現物との照合後に作る。基板の未実装状態を完成図として描かない。
- Wokwi のブラウザ表示、起動時 GPIO と boot mode の挙動、KiCad でのファイル読み込みと ERC。実機の電源・安全確認は図やシミュレーションで代替できない。

写真と地図の画像は Git に入れない。大きな製造用生成物、vendor binary、第三者文書も追跡方法とライセンスを決めずに追加しない。
