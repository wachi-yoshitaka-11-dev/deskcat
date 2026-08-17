# 実験記録

[README](README.md)が正式文書として挙げる`実験記録`の実体である。書式は
[技術ガイド](../DeskCat_Microcontroller_Development_Guide.md)の`28.2 実験ログ`に従う。

**この文書は未加工の観測を残す場所である。**受け入れ要件と照合する値の正は各正本文書側に置き、
ここからは参照するだけにする。数値をここと正本の両方に正として置かない。

| 実験 | 対象 | 値の正 |
|---|---|---|
| [EXP-001](#exp-001-連続streamingの実効sample rate) | `HW-TBD-034` 作業1・作業2 | [power-budget.md](power-budget.md)の`手持ち候補に固有の制約: SRAMと取得方式`、`基準電圧が未解決である（設計上の論点）` |

**大容量の生dataはこのrepositoryへ入れていない。**保存場所は
[development-foundation-plan.md](../planning/development-foundation-plan.md)の
「実験ログと大容量計測データの正式な保存場所を最初の実験Issueで決める」が未確定であり、
**この文書の新設をもってその決定としてよいかは人間の確認を待つ。**
確定するまで、CSV等は`hardware/measurement/.gitignore`で追跡対象から外している。

---

## EXP-001: 連続streamingの実効sample rate

- **日時**: 2026-08-17（初回測定）、**2026-08-18（治具を直して取り直し）**。いずれもJST
- **目的**: `HW-TBD-034`の未解決3点のうち、(2)「SRAM 2 KBに対する取得方式」と
  (1)「基準電圧の校正」の測定側を解く。**連続streamingで何Sample/s出るかを実測し**、
  `Sample rateとlog形式`の必須要件と突き合わせる。あわせてAV<sub>CC</sub>基準での
  内蔵1.1 V基準の生ADC値を採る
- **仮説**:
  1. 連続streamingは`Sample rateとlog形式`の必須要件を2 channel同時で満たす（**要件値はここへ再掲しない**）
  2. `DS40002061B` Table 24-1 の`Auto Triggered conversions`は13.5 cyclesであり、
     ADC clock 125 kHzでは合計約9.26 kSample/sになる
  3. free running modeでISRから`ADMUX`を書くと、§24.5.1により結果のchannel帰属が
     1変換ぶん遅れる。したがって素朴に現在の`ADMUX`でtagを付けるとlabelが入れ替わる
- **ハードウェア構成**:
  - 測定器: Arduino Uno R3（`MEAS-02`。`BOARD MODEL UNO R3`／`ATMEGA328P-PU`／`MEGA16U2`）
  - USB idは`2341:0043`（Arduino SA, Uno R3 CDC ACM）として認識。**serial番号は記録しない**
  - **DeskCat側へは一切接続していない。**Arduino単体とPCだけで完結する
  - 開発端末: 実機のLinux（Ubuntu 24.04.4、kernel 7.0.0-28-generic、x86_64、
    `systemd-detect-virt: none`）。ADR-0005の要求を満たす
- **回路 revision**: 配線は治具のみ。`A0` → Arduinoの`3V3` pin、`A1` → Arduinoの`GND`。
  **抵抗を入れていない。**分圧も外部電源も使っていない。
  2 channelへ意図的に異なる電圧を入れて、channel帰属の入れ替わりを検出できる形にした
- **firmware commit**: 治具は`hardware/measurement/`配下にある。初回測定時はrepository commit `3a34aef`、
  取り直し時は同じbranchのreview指摘修正後（`測定治具のreview指摘4件を直す`）である
- **電源条件**: ArduinoはPCのUSBからのみ給電。`VUSBMax` 5.5 V。監視対象への通電は行っていない。
  **DeskCat側のrailを通らないことの裏付けは、公式Power Treeのblock図の粒度までである**
  （`手持ち候補の現物識別`）。**schematic粒度での一致は未確認であり、独立給電は候補条件のままである。**
  この実験はそれを前進させていない
- **設定**:
  - 治具: `hardware/measurement/arduino-transient-logger`（連続streaming）と
    `hardware/measurement/arduino-vref-calibrate`（基準電圧）
  - 集計: `hardware/measurement/adc_stream_rate.py`（Python 3標準ライブラリのみ）
  - toolchain: `arduino-cli` 1.5.1（公式releaseのtarballをsha256照合して導入）、
    `arduino:avr` 1.8.8、`avr-gcc` 7.3.0-atmel3.6.1-arduino7、`avrdude` 8.0.0-arduino1
  - ADC基準はAV<sub>CC</sub>（`REFS1:0 = 01`。`DS40002061B` Table 24-3）
  - 取得方式は**連続streaming**。burst captureではない
  - `dialout` group追加後、再loginしていないため全serial操作を`sg dialout -c '...'`で実行
- **手順**:
  1. board無しで parser の自己testを通す（`test_adc_stream_rate.py`、12 case）
  2. sketchをcompileし、`arduino-cli compile --upload`でuploadする
  3. 各条件で8〜10秒streamingを受け、届いたsample数・取りこぼし・block欠番を数える
  4. 起動直後の4 blockを捨てる（過渡を除くため。捨てた数も報告させる）
  5. channel別の生ADC値の大小関係が、手順で入れた既知電圧と一致するか確認する
  6. 基準電圧側は単発変換で`GND`／`VBG`／`A0`／`A1`を読み、切替直後の生値も並べて記録する

### raw data

sample rateの測定（各条件8〜10秒。`ADC clock = F_CPU / 2^ADPS`、F_CPU = 16 MHz）。

| channel数 | ADPS | ADC clock | baud | 実効rate 合計 | 1 ch当たり | 取得rate（Arduino時計） | ISRが捨てたsample | block欠番 | 予約bit破れ |
|---|---|---|---|---|---|---|---|---|---|
| 2 | 7 | 125 kHz | 1000000 | 9603.5 | 4801.8 | 9615.4 | 0 | 1 | 1 |
| 2 | 7 | 125 kHz | 500000 | 9616.4 | 4808.2 | 9615.4 | 0 | 0 | 0 |
| 2 | 7 | 125 kHz | 115200 | 5416.4 | 2708.2 | 9615.4 | **41371** | 0 | 0 |
| 2 | 6 | 250 kHz | 1000000 | 19229.0 | 9614.5 | 19230.8 | 0 | 0 | 0 |
| 2 | 6 | 250 kHz | 500000 | 19235.7 | 9617.8 | 19230.8 | 0 | 0 | 0 |
| 1 | 7 | 125 kHz | 1000000 | 9618.2 | 9618.2 | 9615.4 | 0 | 0 | 0 |
| 1 | 6 | 250 kHz | 1000000 | 19216.0 | 19216.0 | 19230.8 | 0 | 1 | 1 |

単位はSample/s。「実効rate」はPCの壁時計（`time.monotonic()`）基準で届いたsample数から、
「取得rate」はArduino自身の`micros()`基準でISRが取得したsample数から出した。
**どの条件でも区間収支が閉じた**（`取得 = 届いた + 捨てた + ring滞留の増減 + 回線上の欠落`）。
header XOR不一致は全条件で0である。

**この表は2026-08-18に取り直した値である。**2026-08-17の初回測定は、
壁時計rateの分子と分母の区間が揃っておらず**約0.5 %低く出ていた**（下記`訂正`）。

channel別の生ADC値（10 bit、AV<sub>CC</sub>基準）。

| 条件 | ch0（`A0` = `3V3` pin） | ch1（`A1` = `GND`） |
|---|---|---|
| ADPS 7（125 kHz） | min 672 / max 676 / mean 673.2 | min 0 / max 0 / mean 0.0 |
| ADPS 6（250 kHz） | min 672 / max 676 / mean 673.2〜673.4 | min 0 / max 0 / mean 0.0 |

基準電圧側（単発変換、ADPS 7、n = 64、切替後8変換を捨てた。4回反復）。

| 入力 | MUX | min | max | mean | mean / 1024 |
|---|---|---|---|---|---|
| `0V (GND)` | `1111` | 0 | 0 | 0.000 | 0.000000 |
| `1.1V (VBG)` | `1110` | 225 | 226 | 225.828〜225.984 | **0.220535〜0.220688** |
| `A0` | `0000` | 673 | 674 | 673.297〜673.437 | 0.657516〜0.657654 |
| `A1` | `0001` | 0 | 0 | 0.000 | 0.000000 |

**分母は1024である**（§24.7 `ADC = (V_IN × 1024) / V_REF`）。**慣習の1023ではない。**

切替直後の生値（`VBG`。捨てる必要があることを値で示すために並べた）。

```text
# settle VBG : 126 226 226 226 226 226 226 226 226 226
# settle VBG : 127 226 226 226 226 226 226 226 226 226
# settle VBG : 158 226 226 226 226 226 226 226 226 226
# settle VBG : 161 226 226 226 226 226 226 226 226 226
```

### 観測結果

1. **必須要件は2 channel同時で満たした。**最も遅い条件でも1 channelあたり2708.2 Sample/sである。
   ただしその条件（baud 115200）は取りこぼしを41371件出しており、**rateを満たすことと
   取りこぼしが無いことは別である。**
2. **baud 115200はtransportが律速している。**1 blockはheader 22 B + payload 256 Bで
   128 sampleを運ぶため1 sampleあたり2.172 Bとなり、実効約11.52 kB/sでは約5304 Sample/sになる。
   実測5416.4はこれと同じ桁で一致する。500000以上ではADC側が律速し、取りこぼしは0になった。
3. **取得rateはchannel数に依存しなかった。**1 chと2 chで合計取得rateが同じ値になり、
   1 channelあたりのrateは合計をchannel数で割った値と一致した。
4. **仮説2は外れた。**取得rateは全条件で`ADC clock / 13`と一致した
   （125000 / 13 = 9615.38に対し実測9615.4、250000 / 13 = 19230.77に対し実測19230.8）。
   Table 24-1が`Auto Triggered conversions`に与える13.5 cyclesとは一致しない。
   **この差の機序はこの測定では確定していない。**実測値を採り、差がある事実を記録する。
5. **仮説3は当たった。**§24.5.1の1変換遅れを織り込んだchannel帰属で、
   全条件で`ch0 > ch1`（673 対 0）となり、入れた既知電圧の大小関係と一致した。
   **段数を誤ると値は正常に見えたままlabelだけが入れ替わるため、
   異なる既知電圧を入れることがこの確認の前提である。**
6. **`DS40002061B` §24.4の「bandgapは安定するまで最初の値が誤りうる」が実測で見えた。**
   `VBG`へ切り替えた直後の1変換だけが126〜161とばらつき、以降は226で安定した。
   捨てるべき変換は1個で足りており、設定した8個は十分である。
7. **ADC offsetは現分解能では検出されなかった。**内部の`0V (GND)`（MUX `1111`）と、
   `GND`へ配線した`A1`がいずれも0を返した。
8. `A0`のmin/max幅は125 kHzでも250 kHzでも4 LSB（672〜676）に収まり、
   **この測定では両者の差を分離できなかった。**§24.4 は250 kHzが最大分解能の規定外である
   と述べているが、**その影響をこの観測で定量してはいない。**
9. **transportは完全に無損失ではない。**7条件のうち2条件で、予約bitの破れを1件検出し、
   その block を読み捨てた結果 block欠番が1件出た（約750〜1500 blockに対し1件）。
   **parserが検出して読み捨て、収支もその欠落を正しく説明した。**
   取りこぼし（ISRがringに入れられなかった件数）とは別の事象である。
10. **壁時計基準と Arduino 時計基準の rate が 0.1 % 以内で一致した。**
   初回測定でこの2つに約0.5 %の差があったのは測定系の集計の誤りであり、下記`訂正`のとおりである。

### 訂正（2026-08-18）

**2026-08-17の初回測定で記録した値には、測定系の集計に起因する誤りがあった。**
[PR #145](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/145)の自動reviewの指摘を受けて
確認し、治具を直して**同じ条件を取り直した**。上の`raw data`は取り直した値である。

| 何が誤っていたか | 影響 | 直し方 |
|---|---|---|
| 壁時計rateの分子と分母が同じ区間でなかった（捨てたblockの時間が分母に残っていた） | **実効rateが約0.5 %低く出ていた。**壁時計と`micros()`の差を「window端の副作用」と誤って説明していた | 分子・分母をともにblock受信時刻の区間へ揃えた |
| ADC変換式の分母に1023を使っていた | 校正比が約0.098 %ずれていた | §24.7 の `ADC = (V_IN × 1024) / V_REF` に従い1024へ直した |
| `taken`がring滞留分を含むことを収支から差し引いていなかった | 正常な測定に対して「不整合」を誤報しうる状態だった（実測では誤報は出ていない） | headerへ`pending`を追加して差し引いた |
| CSVの時刻indexを単調加算だけで進めていた | 回線上でblockを失うと以降の時刻ずれが累積した。**上記9のとおりこの欠落は実際に起きるため、実害のある誤りだった** | blockごとに`taken - pending`から引き直し、`lost_blocks`もguardへ入れた |

**結論は変わっていない。**1 channelあたりの実測値は125 kHzで4808.2 Sample/sとなり、
**依然として目標帯には届かない。**取得rateが`ADC clock / 13`と一致することも変わらない。

### 結論

- **仮説1: supported。**連続streamingは必須要件を2 channel同時で満たす。
  **burst captureを採らずに済み、SRAM 2 KBの制約は取得方式の障害にならない。**
- **仮説2: rejected。**上記4のとおり実測は`ADC clock / 13`だった。
- **仮説3: supported。**上記5のとおり。
- **基準電圧は未解決のままである。**得られたのはAV<sub>CC</sub>に対する比
  （`VBG`が約0.2206）だけで、**絶対電圧にするにはAV<sub>CC</sub>自身の実測が要る。**
  `V_INT_actual = (VBGの生ADC値 / 1024) × AVCC実測`である。
  **デジタルテスター（MAS830L）による5 V pinの実測と、テスター自身の確度が未取得であり、
  校正の不確かさは`TBD`である。**確度を超える主張をしない。
- **`HW-TBD-034`はcloseしない。**解いたのは未解決3点のうち(2)と、(1)の測定側だけである。
  (3)「GND topologyへの影響」は未着手であり、**DeskCat側への接続と通電を伴うため
  人間の立ち会いが要る。**
- **`MEAS-02`の採否は未確定である。**方式1の採用は決定済みだが、この品で足りるかは
  (1)と(3)が解けるまで判定しない。
- **未観測区間について新たな主張をしない。**連続streamingにより`capture window外`は
  消えるが、**`sample間`は残る。**「電源過渡をすべて実測済み」とは扱わない。

### 次の実験

1. **作業2の残り**: MAS830Lで5 V pinの実電圧を測り、`V_INT_actual`を確定する。
   あわせて**テスター自身のrange別確度**（`±(x % + n digit)`）と**確度を保証する基準温度**、
   および測定時の周囲温度を記録する。**これが揃うまで校正の不確かさは`TBD`である。**
2. **作業3**: GND topologyの確定とDeskCat側への接続。**通電を伴い、人間の立ち会いが要る。**
   配線図を先に書き、電源off時の導通チェックで図どおりであることを確認してから通電する。
   `5 V railをADCへ直結する条件`に従い、**dividerは既定で残す。**
3. **必要帯域の根拠**（`電源過渡の測定方式のdesign review`で`TBD`）。
   これが確定するまで、ADC clockの既定を規定外の250 kHzへ動かさない。

---

## Revision履歴

| 日付 | Revision | 変更 | 根拠 |
|---|---|---|---|
| 2026-08-18 | 0 | 文書を新設し、`EXP-001`（実測は2026-08-17）を記録した。[README](README.md)が正式文書として挙げていた`実験記録`に実体が無かったため、[技術ガイド](../DeskCat_Microcontroller_Development_Guide.md)の`28.2 実験ログ`の書式で作成した。**`HW-TBD-034`の未解決3点のうち(2)取得方式を解き、(1)基準電圧の測定側を採った。(3) GND topologyは未着手である。**`HW-TBD-034`はcloseしていない。**保存場所の決定そのものは人間の確認を待つ**（[development-foundation-plan.md](../planning/development-foundation-plan.md)の未チェック項目） | [#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、`DS40002061B`（sha256は[hardware-bom.md](hardware-bom.md)の`MEAS-02`行） |
| 2026-08-18 | 1 | **`EXP-001`の値を取り直した。**[PR #145](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/145)の自動reviewが治具の集計の誤りを4件指摘し、**うち壁時計rateの区間の不揃いとADC変換式の分母は記録済みの値に効いていた。**治具を直して同じ条件を取り直し、`raw data`を差し替えたうえで`訂正`節を新設した。**過去のRevision行は書き換えていない。****結論は変わっていない**（1 channelあたりは依然として目標帯へ届かない）。あわせて**transportが完全に無損失ではないこと**（予約bitの破れ1件とblock欠番1件を2条件で観測）と、**独立給電の裏付けが公式Power Treeのblock図の粒度までであること**を記録した | [PR #145のreview指摘](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/145)、`DS40002061B` §24.7 |
