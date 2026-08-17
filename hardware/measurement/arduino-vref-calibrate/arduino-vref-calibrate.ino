// DeskCat 基準電圧の校正: Arduino Uno R3 側の測定 sketch（`HW-TBD-034` 作業2）
//
// ADC は絶対値を返さない。**基準電圧を1としたときの割合**を返す。
// `HW-TBD-028`(b) の判定値は絶対電圧なので、校正しないと照合できない。
// 正本は docs/hardware/power-budget.md の `基準電圧が未解決である（設計上の論点）` である。
//
// この sketch が出すのは **AV_CC を基準にしたときの各入力の生ADC値**だけである。
// 絶対電圧へ変換するには、AV_CC 自身をデジタルテスター（MAS830L）で測った値が要る。
//
//     V_INT_actual = (VBG の生ADC値 / 1023) * AV_CC の実測値
//
// **テスターの確度が校正の精度の上限になる。**それを超える主張をしない。
//
// 一次資料: ATmega48A/PA/88A/PA/168A/PA/328/P Data Sheet `DS40002061B`
//   sha256 b9b9d83cda56a95d999ea8d54fe5a540748ae9020e5e7ae19b913d384ba9320e
//   Table 24-3 `Voltage Reference Selections for ADC`: REFS1:0 = 01 は
//     `AVCC with external capacitor at AREF pin`
//   Table 24-4 `Input Channel Selections`: MUX3:0 = 1110 は `1.1V (VBG)`、1111 は `0V (GND)`
//   §24.5.2: `The first ADC conversion result after switching reference voltage source may be
//     inaccurate, and the user is advised to discard this result.`
//   §24.4: `When the bandgap reference voltage is used as input to the ADC, it will take a
//     certain time for the voltage to stabilize. If not stabilized, the first value read after
//     the first conversion may be wrong.`
//
// **AREF pin をテスターで直接測る方法には依らない。**§24.5.2 は VREF を
// `high impedance source` と述べ、`high impedance voltmeter` を要求している。
// MAS830L の入力インピーダンスは未取得であり、この要求を満たすか判定できない。
// 比率法は AREF へ何も接続しないため、この条件を回避できる。
// （満たさないと断定しているのではなく、判定材料が無いため依存しないという選択である）

#include <Arduino.h>

#ifndef CAL_ADPS
// ADC clock = F_CPU / 2^CAL_ADPS。16 MHz で 7 なら 125 kHz。
// `DS40002061B` §24.4 が最大分解能に要求する 50〜200 kHz の規定内である。
// 校正では速度より確度を採る。
#define CAL_ADPS 7
#endif

#ifndef CAL_SAMPLES
#define CAL_SAMPLES 64
#endif

// 入力を切り替えた直後に捨てる変換の数。**この値の妥当性は下の可視化で確認する。**
#ifndef CAL_DISCARD
#define CAL_DISCARD 8
#endif

// 切り替え直後の生値を何個表示するか（捨てる根拠を目で見えるようにする）。
#define CAL_SHOW_SETTLE 10

// REFS1:0 = 01 で AV_CC を基準にする（Table 24-3）。
static const uint8_t REF_AVCC = (1 << REFS0);

static const uint8_t MUX_A0 = 0x00;   // ADC0
static const uint8_t MUX_A1 = 0x01;   // ADC1
static const uint8_t MUX_VBG = 0x0E;  // 1.1V (VBG)
static const uint8_t MUX_GND = 0x0F;  // 0V (GND)

static void adc_begin(void) {
  ADCSRB = 0;
  ADCSRA = (uint8_t)((1 << ADEN) | (CAL_ADPS & 0x07));  // 単発変換。ADATE は立てない。
}

// 単発変換を1回行う。`DS40002061B` §24.5.1 は単発変換では
// 「常に変換開始前に channel を選ぶ」ことを求めており、そのとおりにする。
static uint16_t adc_read_once(uint8_t mux) {
  ADMUX = (uint8_t)(REF_AVCC | (mux & 0x0F));
  ADCSRA |= (uint8_t)(1 << ADSC);
  while (ADCSRA & (1 << ADSC)) {
    // 変換完了を待つ。
  }
  const uint8_t lo = ADCL;  // ADCL を先に読む。
  const uint8_t hi = ADCH;
  return (uint16_t)((uint16_t)hi << 8 | lo);
}

static void show_settle(const char *label, uint8_t mux) {
  // 入力を切り替えた直後の生値を並べる。**捨てる必要があることを値で示す。**
  Serial.print(F("# settle "));
  Serial.print(label);
  Serial.print(F(" :"));
  for (uint8_t i = 0; i < CAL_SHOW_SETTLE; i++) {
    Serial.print(' ');
    Serial.print(adc_read_once(mux));
  }
  Serial.println();
}

static void measure(const char *label, uint8_t mux) {
  for (uint8_t i = 0; i < CAL_DISCARD; i++) {
    (void)adc_read_once(mux);
  }

  uint32_t sum = 0;
  uint16_t lo = 0xFFFF;
  uint16_t hi = 0;
  for (uint16_t i = 0; i < CAL_SAMPLES; i++) {
    const uint16_t v = adc_read_once(mux);
    sum += v;
    if (v < lo) lo = v;
    if (v > hi) hi = v;
  }

  const double mean = (double)sum / (double)CAL_SAMPLES;

  Serial.print(label);
  Serial.print(F("  mux=0x"));
  Serial.print(mux, HEX);
  Serial.print(F(" n="));
  Serial.print((uint16_t)CAL_SAMPLES);
  Serial.print(F(" min="));
  Serial.print(lo);
  Serial.print(F(" max="));
  Serial.print(hi);
  Serial.print(F(" mean="));
  Serial.print(mean, 3);
  Serial.print(F(" ratio="));
  Serial.println(mean / 1023.0, 6);
}

void setup(void) {
  Serial.begin(115200);
  while (!Serial) {
  }
  adc_begin();
}

void loop(void) {
  Serial.println();
  Serial.print(F("# deskcat-vref-calibrate f_cpu="));
  Serial.print((uint32_t)F_CPU);
  Serial.print(F(" adps="));
  Serial.print((uint8_t)CAL_ADPS);
  Serial.print(F(" ref=AVCC discard="));
  Serial.print((uint8_t)CAL_DISCARD);
  Serial.print(F(" n="));
  Serial.println((uint16_t)CAL_SAMPLES);

  // 基準は AV_CC 固定のまま、入力だけを切り替える。
  // §24.5.2 が「基準を切り替えた直後の1変換は捨てる」と述べているのは基準の切替であり、
  // ここでは基準を動かさない。ただし VBG は §24.4 の安定待ちが要るため、
  // 入力切替後の値も捨てて数を記録する。
  show_settle("GND", MUX_GND);
  show_settle("VBG", MUX_VBG);

  measure("GND", MUX_GND);
  measure("VBG", MUX_VBG);
  measure("A0 ", MUX_A0);
  measure("A1 ", MUX_A1);

  // serial 出力は ASCII だけにする。flash を節約し、端末側の encoding に依存しない。
  Serial.println(F("# V_INT_actual = ratio(VBG) * AVCC_measured"));
  Serial.println(F("# AVCC_measured: measure the 5V pin with the DMM (MAS830L)."));
  Serial.println(F("# Do not claim precision beyond the DMM accuracy."));
  delay(2000);
}
