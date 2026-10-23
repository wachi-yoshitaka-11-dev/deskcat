//! #14 の試験 build `touch-lcd-14` 専用。LCD を描き続けながら、同じ SPI host で `TOUCH-01`（XPT2046）の raw を読む。
//!
//! 受け入れ条件「LCD 通信による許容できない data loss がない」を実機で観察するための build である。
//!
//! # bus の共有
//!
//! `SpiDriver` は 1 つで、`Rc` で共有する。LCD（[`crate::display::Ili9341::with_shared_bus`]、6 MHz）とタッチ
//! （[`crate::touch::TouchSampler::with_shared_bus`]、1 MHz）はそれぞれ hardware CS なしの `SpiDeviceDriver` を持ち、
//! CS は自前の GPIO（`LCD-CS`＝GPIO22、`TOUCH-CS`＝GPIO21）で扱う。main loop は 1 task なので、
//! [`TouchLcd::poll`] は「LCD の帯を 1 本塗る → タッチを 1 sample 読む」の順に呼ぶだけで、2 つの転送は重ならない。
//! 帯の途中ではタッチを読まない。
//!
//! # 描画と出力
//!
//! `deskcat_face::Painter` で neutral → happy → surprised を、帯 [`BAND_ROWS`] 行ずつ塗り続ける（製品の `crate::face` と同じ帯）。
//! `touch_lcd_window` は [`WINDOW_STRIPES`] 本ごとに、窓内の帯 1 本の所要時間の平均と最大（`fill_rect` の呼び出しの前後）と、起動からの帯の本数・
//! LCD の失敗の件数・タッチの SPI の失敗の件数を UART0 へ出す。タッチの raw は `touch_raw`／`touch_window`（`crate::touch`）。
//!
//! # 見えないもの
//!
//! LCD は書き込み専用で、転送が壊れても件数に出ない。件数に出るのは SPI driver が返す失敗だけで、
//! 表示の乱れは目視でしか分からない。

use std::time::Instant;

use deskcat_face::Painter;
use deskcat_protocol::ExpressionName;

use crate::display::Ili9341;
use crate::touch::TouchSampler;

/// 帯 1 本の行数。`crate::face::FACE_BAND_ROWS` と同じ値（320×16×2 byte）。
pub const BAND_ROWS: u16 = 16;
/// 1 窓の帯の本数。
pub const WINDOW_STRIPES: u32 = 200;
/// 続けて失敗したら描画を止める回数。
pub const MAX_CONSECUTIVE_LCD_ERRORS: u32 = 5;

const EXPRESSIONS: [ExpressionName; 3] = [
    ExpressionName::Neutral,
    ExpressionName::Happy,
    ExpressionName::Surprised,
];

pub struct TouchLcd {
    lcd: Ili9341<'static>,
    sampler: TouchSampler,
    painter: Option<Painter>,
    next_expression: usize,
    consecutive_lcd_errors: u32,
    lcd_stopped: bool,
    window: u32,
    window_stripes: u32,
    window_sum_us: u64,
    window_max_us: u32,
    stripes_total: u32,
    lcd_errors_total: u32,
    expressions_total: u32,
}

impl TouchLcd {
    /// 初期化と backlight の点灯を済ませた LCD と、結線済みのタッチを受け取る。
    pub fn new(lcd: Ili9341<'static>, sampler: TouchSampler) -> Self {
        Self {
            lcd,
            sampler,
            painter: None,
            next_expression: 0,
            consecutive_lcd_errors: 0,
            lcd_stopped: false,
            window: 0,
            window_stripes: 0,
            window_sum_us: 0,
            window_max_us: 0,
            stripes_total: 0,
            lcd_errors_total: 0,
            expressions_total: 0,
        }
    }

    /// 次に動く時刻（uptime の ms）。描画が続く間は `now_ms`（待たない）。
    pub fn next_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        let touch = self.sampler.next_deadline_ms();
        if self.lcd_stopped {
            touch
        } else {
            Some(touch.map_or(now_ms, |t| t.min(now_ms)))
        }
    }

    /// LCD の帯を 1 本塗り、そのあとタッチを（時刻が来ていれば）1 sample 読む。
    pub fn poll(&mut self, now_ms: u64) {
        let started = Instant::now();
        if !self.lcd_stopped {
            self.draw_stripe();
        }
        // 描画の時間を足した時刻でタッチを読む（`T_IRQ` の変化の `t_ms` と周期の起点が、描画の前の時刻にならないように）。
        let drawn_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.sampler.poll(now_ms.saturating_add(drawn_ms));
    }

    fn draw_stripe(&mut self) {
        if self.painter.is_none() {
            let name = EXPRESSIONS[self.next_expression % EXPRESSIONS.len()];
            self.next_expression = self.next_expression.wrapping_add(1);
            self.painter = Painter::new(name, BAND_ROWS);
            self.expressions_total = self.expressions_total.saturating_add(1);
        }
        let Some(painter) = self.painter.as_mut() else {
            return;
        };
        let Some(s) = painter.next_stripe() else {
            self.painter = None;
            return;
        };
        let started = Instant::now();
        let result = self.lcd.fill_rect(s.x0, s.y0, s.x1, s.y1, s.color);
        let us = u32::try_from(started.elapsed().as_micros()).unwrap_or(u32::MAX);
        match result {
            Ok(()) => {
                self.consecutive_lcd_errors = 0;
                self.stripes_total = self.stripes_total.saturating_add(1);
                self.window_stripes += 1;
                self.window_sum_us += u64::from(us);
                self.window_max_us = self.window_max_us.max(us);
                if self.window_stripes >= WINDOW_STRIPES {
                    self.finish_window();
                }
            }
            Err(err) => {
                self.lcd_errors_total = self.lcd_errors_total.saturating_add(1);
                self.consecutive_lcd_errors += 1;
                self.painter = None;
                log::error!(
                    "touch_lcd_fault kind=lcd y={} error={err} count={}",
                    s.y0,
                    self.lcd_errors_total
                );
                if self.consecutive_lcd_errors >= MAX_CONSECUTIVE_LCD_ERRORS {
                    log::error!("touch_lcd_draw_stopped");
                    self.lcd_stopped = true;
                }
            }
        }
    }

    fn finish_window(&mut self) {
        self.window += 1;
        log::info!(
            "touch_lcd_window w={} stripes={} stripe_us_avg={} stripe_us_max={} stripes_total={} expressions_total={} lcd_errors_total={} touch_spi_errors_total={}",
            self.window,
            self.window_stripes,
            self.window_sum_us / u64::from(self.window_stripes),
            self.window_max_us,
            self.stripes_total,
            self.expressions_total,
            self.lcd_errors_total,
            self.sampler.spi_errors()
        );
        self.window_stripes = 0;
        self.window_sum_us = 0;
        self.window_max_us = 0;
    }
}
