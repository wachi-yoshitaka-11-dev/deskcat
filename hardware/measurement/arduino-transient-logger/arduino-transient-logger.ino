// DeskCat 電源過渡の独立観測: Arduino Uno R3 側の連続streaming sketch
//
// 用途は `HW-TBD-034` 方式1（独立した外部観測）の実効sample rate測定である。
// 正本は docs/hardware/power-budget.md の次の節であり、値も規則もここへ再掲しない。
//   - `Sample rateとlog形式`
//   - `手持ち候補に固有の制約: SRAMと取得方式`
//   - `手持ち候補の現物識別`
//
// これは測定治具であって DeskCat firmware ではない。DeskCat の Protocol とは無関係であり、
// このUSB serialは測定に専有する。
//
// 一次資料: ATmega48A/PA/88A/PA/168A/PA/328/P Data Sheet `DS40002061B`
//   sha256 b9b9d83cda56a95d999ea8d54fe5a540748ae9020e5e7ae19b913d384ba9320e
//   （sha256の記録は docs/hardware/hardware-bom.md の `MEAS-02` 行にある）

#include <Arduino.h>

// ---- compile-time parameter ------------------------------------------------
// arduino-cli からは次のように上書きする。
//   --build-property compiler.cpp.extra_flags="-DLOGGER_BAUD=500000 -DLOGGER_ADPS=6"

#ifndef LOGGER_BAUD
#define LOGGER_BAUD 1000000UL
#endif

// ADPS2:0。ADC clock = F_CPU / 2^LOGGER_ADPS。
// `DS40002061B` §24.4 は最大分解能には 50〜200 kHz を要求する。
// 16 MHz では 7 (/128) が 125 kHz で規定内、6 (/64) が 250 kHz で規定外である。
#ifndef LOGGER_ADPS
#define LOGGER_ADPS 7
#endif

// 1 または 2。2 のとき ADC0 と ADC1 を交互に読む。
#ifndef LOGGER_NCH
#define LOGGER_NCH 2
#endif

// 1 blockで送るsample数。header 24 B に対する payload の割合を決める。
#ifndef LOGGER_BLOCK
#define LOGGER_BLOCK 128
#endif

// micros() を latch する間隔。2の冪であること。ISR内のmicros()呼び出しを間引く。
#define LOGGER_MARK_EVERY 256

// ring buffer。2の冪であること。index が uint8_t で自然に wrap する 256 を使う。
#define RING_SIZE 256

#if (LOGGER_ADPS < 1) || (LOGGER_ADPS > 7)
#error "LOGGER_ADPS must be 1..7"
#endif
#if (LOGGER_NCH != 1) && (LOGGER_NCH != 2)
#error "LOGGER_NCH must be 1 or 2"
#endif
#if (LOGGER_BLOCK < 1) || (LOGGER_BLOCK > 255) || (LOGGER_BLOCK >= RING_SIZE)
#error "LOGGER_BLOCK must be 1..255 and smaller than RING_SIZE"
#endif

static const uint8_t MAGIC0 = 0xA5;
static const uint8_t MAGIC1 = 0x5A;

// ---- ISR と main loop で共有する状態 ---------------------------------------

static volatile uint16_t g_ring[RING_SIZE];
static volatile uint8_t g_head;  // ISRが書く
static volatile uint8_t g_tail;  // main loopが読む
// ringが満杯で捨てたsample数。**uint32 である。**baud 115200 の実測では10秒で
// 約4万件に達したため、uint16 では約16秒で wrap し、境界で差が0に見えてしまう。
static volatile uint32_t g_dropped;
static volatile uint32_t g_taken;     // ISRが取得したsample数（捨てたぶんを含む）
static volatile uint32_t g_mark_us;   // g_mark_taken の時点の micros()
static volatile uint32_t g_mark_taken;

// channel の帰属を追う。`DS40002061B` §24.5.1 が定める1変換ぶんの遅れに対応する。
//
//   In Free Running mode ... Since the next conversion has already started
//   automatically, the next result will reflect the previous channel selection.
//   Subsequent conversions will reflect the new channel selection.
//
// つまりISRで ADMUX を書き換えても、次に上がる結果は「前のchannel」のものである。
// したがって「いま読んだ結果のchannel」と「すでに飛行中の変換のchannel」を別に持ち、
// ISRで書き込む値はそのさらに次の変換に効く、として3段で扱う。
//
// **この対応が正しいかは実測で確認する。**`A0` と `A1` へ異なる既知電圧を入れ、
// 期待した大小関係が出るかを見る。逆だったらこの段数が1つずれている。
static volatile uint8_t g_ch_now;   // いまISRが読んだ結果のchannel
static volatile uint8_t g_ch_next;  // すでに飛行中の変換のchannel（次の結果）

static uint16_t g_blkseq;

// ---- ADC ------------------------------------------------------------------

static inline void adc_select_channel(uint8_t ch) {
  // REFS1:0 = 01 で AV_CC を基準にする（`DS40002061B` §24.9.1）。
  ADMUX = (uint8_t)((1 << REFS0) | (ch & 0x0F));
}

ISR(ADC_vect) {
  // ADCL を先に読む。ADCH を読むまでData Registerは更新されない。
  const uint8_t lo = ADCL;
  const uint8_t hi = ADCH;
  const uint16_t value = (uint16_t)((uint16_t)hi << 8 | lo);

  const uint8_t ch = g_ch_now;

  // channel pipeline を1段進め、さらに次の変換用の値を ADMUX へ書く。
  const uint8_t ch_new =
#if LOGGER_NCH == 2
      (uint8_t)((g_ch_next + 1) & 0x01);
#else
      0;
#endif
  g_ch_now = g_ch_next;
  g_ch_next = ch_new;
#if LOGGER_NCH == 2
  adc_select_channel(ch_new);
#endif

  const uint32_t taken = g_taken + 1;
  g_taken = taken;
  if ((taken & (LOGGER_MARK_EVERY - 1)) == 0) {
    g_mark_us = micros();
    g_mark_taken = taken;
  }

  const uint8_t head = g_head;
  const uint8_t next = (uint8_t)(head + 1);
  if (next == g_tail) {
    // main loopが追いついていない。捨てて数える。**buffer長を伸ばして隠さない。**
    g_dropped++;
    return;
  }
  // bit 0-9 が生ADC値、bit 10 が channel、bit 11-15 は 0 で予約する。
  // PC側はこの予約bitでframingの健全性を確認する。
  g_ring[head] = (uint16_t)(value | ((uint16_t)ch << 10));
  g_head = next;
}

static void adc_begin(void) {
  g_head = 0;
  g_tail = 0;
  g_dropped = 0;
  g_taken = 0;
  g_mark_us = 0;
  g_mark_taken = 0;
  g_ch_now = 0;
  g_ch_next = 0;

  // A0/A1 の digital input buffer を切る（`DS40002061B` §24.9.5）。
  DIDR0 = (uint8_t)((1 << ADC0D) | (1 << ADC1D));

  adc_select_channel(0);
  ADCSRB = 0;  // ADTS = 000。Free Running mode。
  ADCSRA = (uint8_t)((1 << ADEN) | (1 << ADATE) | (1 << ADIE) | (LOGGER_ADPS & 0x07));
  ADCSRA |= (uint8_t)(1 << ADSC);  // 最初の変換を開始する。以降は自動で連鎖する。
}

// ---- 送信 ------------------------------------------------------------------

static inline void put_u8(uint8_t *buf, uint8_t &i, uint8_t v) { buf[i++] = v; }

static inline void put_u16(uint8_t *buf, uint8_t &i, uint16_t v) {
  buf[i++] = (uint8_t)(v & 0xFF);
  buf[i++] = (uint8_t)(v >> 8);
}

static inline void put_u32(uint8_t *buf, uint8_t &i, uint32_t v) {
  buf[i++] = (uint8_t)(v & 0xFF);
  buf[i++] = (uint8_t)((v >> 8) & 0xFF);
  buf[i++] = (uint8_t)((v >> 16) & 0xFF);
  buf[i++] = (uint8_t)((v >> 24) & 0xFF);
}

// header は 24 B 固定である。PC側の parser と同じ並びを保つこと。
#define HEADER_LEN 24

static void send_block(void) {
  // ring から LOGGER_BLOCK 件そろうまで待つ。
  for (;;) {
    uint8_t head, tail;
    {
      const uint8_t sreg = SREG;
      cli();
      head = g_head;
      tail = g_tail;
      SREG = sreg;
    }
    if ((uint8_t)(head - tail) >= LOGGER_BLOCK) {
      break;
    }
  }

  // counter類は32 bitなのでISRに割り込まれないよう一括で取る。
  // pending（ring に滞留していて未送信のsample数）も同じ critical section で採る。
  // これが無いと PC 側の収支が閉じない。`taken` は ring 滞留分を含むためである。
  uint32_t taken, mark_us, mark_taken, dropped;
  uint8_t pending;
  {
    const uint8_t sreg = SREG;
    cli();
    taken = g_taken;
    dropped = g_dropped;
    mark_us = g_mark_us;
    mark_taken = g_mark_taken;
    pending = (uint8_t)(g_head - g_tail);
    SREG = sreg;
  }

  uint8_t header[HEADER_LEN];
  uint8_t i = 0;
  put_u8(header, i, MAGIC0);
  put_u8(header, i, MAGIC1);
  put_u16(header, i, g_blkseq);
  put_u32(header, i, taken);
  put_u32(header, i, dropped);
  put_u32(header, i, mark_us);
  put_u32(header, i, mark_taken);
  put_u8(header, i, (uint8_t)LOGGER_BLOCK);
  // pending は ring 滞留数。0〜255 に収まる（ring は 256 で、snapshot 時は LOGGER_BLOCK 以上）。
  put_u8(header, i, pending);
  // cfg: bit0-2 = ADPS、bit3 = (channel数 == 2)、bit4-7 は 0 で予約する。
  put_u8(header, i, (uint8_t)((LOGGER_ADPS & 0x07) | ((LOGGER_NCH == 2) ? 0x08 : 0x00)));

  uint8_t x = 0;
  for (uint8_t k = 0; k < i; k++) {
    x ^= header[k];
  }
  put_u8(header, i, x);

  Serial.write(header, HEADER_LEN);

  for (uint8_t k = 0; k < LOGGER_BLOCK; k++) {
    const uint8_t tail = g_tail;
    const uint16_t v = g_ring[tail];
    g_tail = (uint8_t)(tail + 1);
    uint8_t pair[2];
    pair[0] = (uint8_t)(v & 0xFF);
    pair[1] = (uint8_t)(v >> 8);
    Serial.write(pair, 2);
  }

  g_blkseq++;
}

// ---- Arduino entry points --------------------------------------------------

void setup(void) {
  Serial.begin(LOGGER_BAUD);
  while (!Serial) {
    // Uno では即座に true になる。移植時のために残す。
  }

  // 設定を1行だけtextで出す。PC側はmagicで同期するため、この行は読み飛ばせる。
  // **streaming開始前の1回だけである。**binary streamへtextを混ぜない。
  Serial.print(F("# deskcat-transient-logger baud="));
  Serial.print((uint32_t)LOGGER_BAUD);
  Serial.print(F(" adps="));
  Serial.print((uint8_t)LOGGER_ADPS);
  Serial.print(F(" nch="));
  Serial.print((uint8_t)LOGGER_NCH);
  Serial.print(F(" block="));
  Serial.print((uint8_t)LOGGER_BLOCK);
  Serial.print(F(" f_cpu="));
  Serial.print((uint32_t)F_CPU);
  Serial.print(F(" mark_every="));
  Serial.println((uint16_t)LOGGER_MARK_EVERY);
  Serial.flush();

  g_blkseq = 0;
  adc_begin();
}

void loop(void) { send_block(); }
