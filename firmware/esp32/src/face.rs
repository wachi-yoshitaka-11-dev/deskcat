//! `set_expression`（Protocol §5.2）の受理状態と、LCDへの表情の描画（[#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)）。
//! #23 の日本語 `show_text` 描画 adapter も持つ。Pi link からの受信・ACKは #19 PR-3 後の別段階で接続する。
//!
//! 表情の図形と、受理の判定の順序、帯への分け方は`deskcat_face`が持つ（hostでtestできる）。
//! このmoduleは、それをLCD driver（`crate::display`）とPi link（`crate::pi_link`）へつなぐだけである。
//!
//! # 構成
//!
//! - `FaceState`: 全buildが持つ。LCDの状態（`DisplayState`）と、受理した最新の表情を持つ。
//!   製品buildと`face-21`試験buildが`Busy`／`Ready`にする。**表情を持たない試験build
//!   （`bringup-display-13`単独など）は、`set_expression`を`hardware_unavailable`で拒否する**（§7）。
//!   `bringup-display-13`と組み合わせたbuildは、bring-upの間`busy`で拒否する。
//!   **製品buildと`face-21`付き試験buildは、Pi commandを受けなくても、描ける状態になった時点（製品と`face-21`単独は起動直後、
//!   `bringup-display-13`との組み合わせはbring-upが済んだ時点）で`neutral`を1回描く。**
//! - `FaceDrawer`: LCD driverを持ち、受けた表情を帯1本ずつ塗る。文字表示中は文字を優先し、
//!   表情の最新要求を保持して期限後に描き直す。1回の`FaceDrawer::poll`で帯かglyphを1本だけ
//!   塗ってmain loopへ戻るため、描画の間もheartbeatとPi linkの受信は止まらない見込みである
//!   （`crate::display_test`の同じ方式。**実機では確かめていない**）。
//!
//! # 確かめていないこと
//!
//! **このmoduleの振る舞いは、実機でも確かめていない。**`firmware/esp32`はhostのworkspaceから除外され、
//! `esp_idf_svc`へ依存するため、host testでは動かせない。判定と帯の分け方は`deskcat_face`のhost
//! testが固定している。LCDに表情が出ること、1帯の所要時間、描画中の受信は実機の確認が要る
//! （#21の実機統合）。切り替えの補間はしない。`transition_ms`は上限の検査だけに使い、受けた表情へ
//! 即座に描き替える。

// 表情を持たない試験buildは`FaceDrawer`を使わないためdead_codeになる。
// moduleごと`#[cfg]`で落とさず、各試験buildでもcross-compileを確認する。
#![allow(dead_code)]

use deskcat_face::{
    color,
    text::{TextError, TextLayout, TextPresentation},
    DisplayState, Painter,
};
use deskcat_protocol::ExpressionName;

use crate::display::Ili9341;

/// 帯1本の行数。**安全要件の5項目に効かない値である**（描画の分け方であり、表示の内容を変えない）。
/// `crate::display_test::BAND_ROWS`と同じ値であり、1帯は320×16×2 byte（約10 KB）である。
pub const FACE_BAND_ROWS: u16 = 16;

// `deskcat_face`の画面の寸法は、`display`の寸法の複製である。向きを変えたときに食い違えば止める。
const _: () = assert!(deskcat_face::SCREEN_WIDTH == crate::display::WIDTH);
const _: () = assert!(deskcat_face::SCREEN_HEIGHT == crate::display::HEIGHT);

/// `set_expression`の受理状態。
#[derive(Debug)]
pub struct FaceState {
    display: DisplayState,
    requested: Option<ExpressionName>,
}

impl Default for FaceState {
    fn default() -> Self {
        Self::new()
    }
}

impl FaceState {
    /// 表情を描かない状態（`face-21`を持たないbuild）で作る。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            display: DisplayState::Unavailable,
            requested: None,
        }
    }

    /// LCDは初期化済みだが、bring-upの試験が使っていることを記録する（終われば描ける）。
    pub fn set_busy(&mut self) {
        self.display = DisplayState::Busy;
    }

    /// bring-upが失敗して終わり、LCDを使えなくなったことを記録する（待っても描けない）。
    pub fn set_unavailable(&mut self) {
        self.display = DisplayState::Unavailable;
    }

    /// 表情を描けるようになったことを記録する。
    pub fn set_ready(&mut self) {
        self.display = DisplayState::Ready;
    }

    /// LCDの状態。`set_expression`の判定へ渡す。
    #[must_use]
    pub const fn display_state(&self) -> DisplayState {
        self.display
    }

    /// 受理した表情を記録する。描画前に次が来たら、最新だけが残る。
    pub fn request(&mut self, name: ExpressionName) {
        self.requested = Some(name);
    }

    /// 未描画の表情を取り出す。
    pub fn take_request(&mut self) -> Option<ExpressionName> {
        self.requested.take()
    }
}

/// LCDへ表情を塗る。
pub struct FaceDrawer<'d> {
    lcd: Ili9341<'d>,
    painter: Option<Painter>,
    text: Option<TextLayout>,
    text_clear_y: u16,
    text_next_index: usize,
    presentation: TextPresentation,
    text_counters: TextCounters,
}

/// 文字表示の失敗を分類する。protocol status への追加は後続の統合段階で判断する。
#[derive(Debug, Default, Clone, Copy)]
pub struct TextCounters {
    pub empty: u64,
    pub too_long: u64,
    pub missing_glyph: u64,
    pub duration: u64,
    pub draw_failed: u64,
}

impl<'d> FaceDrawer<'d> {
    /// 初期化とbacklightの点灯を済ませたdriverを受け取る。
    pub fn new(lcd: Ili9341<'d>) -> Self {
        Self {
            lcd,
            painter: None,
            text: None,
            text_clear_y: 0,
            text_next_index: 0,
            presentation: TextPresentation::new(),
            text_counters: TextCounters::default(),
        }
    }

    /// `name`の表情を、最初から塗り直す。塗っている途中なら、そちらは捨てる。
    pub fn show(&mut self, name: ExpressionName) {
        if let Some(name) = self.presentation.show_expression(name) {
            self.painter = Painter::new(name, FACE_BAND_ROWS);
            log::info!("face_show name={name:?}");
        } else {
            log::info!("face_deferred_during_text name={name:?}");
        }
    }

    /// 文字を全画面に一時表示する。受理したら現在の表情の描画を中断する。
    /// 期限後は文字を消して、表示中に受けた最新の表情を描き直す。
    ///
    /// # Errors
    ///
    /// glyph 不足、仮1行 layout 超過、空文字、仮 duration 上限超過。
    pub fn show_text(
        &mut self,
        text: &str,
        duration_ms: u32,
        now_ms: u64,
    ) -> Result<(), TextError> {
        let layout = TextLayout::new(text).inspect_err(|&error| {
            self.record_text_rejection(error);
        })?;
        self.presentation
            .show_text(duration_ms, now_ms)
            .inspect_err(|&error| {
                self.record_text_rejection(error);
            })?;
        self.painter = None;
        self.text = Some(layout);
        self.text_clear_y = 0;
        self.text_next_index = 0;
        log::info!(
            "text_show chars={} duration_ms={duration_ms}",
            text.chars().count()
        );
        Ok(())
    }

    fn record_text_rejection(&mut self, error: TextError) {
        let counter = match error {
            TextError::Empty => &mut self.text_counters.empty,
            TextError::TooLong => &mut self.text_counters.too_long,
            TextError::MissingGlyph(_) => &mut self.text_counters.missing_glyph,
            TextError::Duration => &mut self.text_counters.duration,
        };
        *counter = counter.saturating_add(1);
        log::warn!("text_rejected category={error:?} code={:?}", error.code());
    }

    #[must_use]
    pub const fn text_counters(&self) -> TextCounters {
        self.text_counters
    }

    /// main loop の待機締切に文字消去時刻を含める。
    #[must_use]
    pub const fn text_deadline_ms(&self) -> Option<u64> {
        self.presentation.deadline_ms()
    }

    /// 塗る帯が残っているか。
    #[must_use]
    pub const fn is_drawing(&self) -> bool {
        self.painter.is_some()
            || match &self.text {
                Some(layout) => {
                    self.text_clear_y < crate::display::HEIGHT
                        || self.text_next_index < layout.len()
                }
                None => false,
            }
    }

    /// 帯を1本だけ塗る。**失敗してもpanicしない。**失敗したら、その表情の残りを塗らずに諦める。
    pub fn poll(&mut self, now_ms: u64) {
        if let Some(name) = self.presentation.expire(now_ms) {
            self.text = None;
            self.painter = Painter::new(name, FACE_BAND_ROWS);
            log::info!("text_expired_restore_face name={name:?}");
        }
        if let Some(layout) = self.text.as_ref() {
            if self.text_clear_y < crate::display::HEIGHT {
                let y0 = self.text_clear_y;
                let y1 = y0
                    .saturating_add(FACE_BAND_ROWS - 1)
                    .min(crate::display::HEIGHT - 1);
                if let Err(err) =
                    self.lcd
                        .fill_rect(0, y0, crate::display::WIDTH - 1, y1, color::BLACK)
                {
                    self.text_draw_failed("clear", err);
                } else {
                    self.text_clear_y = y1 + 1;
                }
                return;
            }
            if let Some(cell) = layout.cell(self.text_next_index) {
                if let Err(err) = self.lcd.draw_glyph_cell(cell) {
                    self.text_draw_failed("glyph", err);
                } else {
                    self.text_next_index += 1;
                }
                return;
            }
        }
        let Some(painter) = self.painter.as_mut() else {
            return;
        };
        match painter.next_stripe() {
            Some(s) => {
                if let Err(err) = self.lcd.fill_rect(s.x0, s.y0, s.x1, s.y1, s.color) {
                    log::error!("face_draw_failed y={} error={err}", s.y0);
                    self.painter = None;
                }
            }
            None => {
                self.painter = None;
                log::info!("face_drawn");
            }
        }
    }

    fn text_draw_failed(&mut self, step: &str, err: esp_idf_svc::sys::EspError) {
        self.text_counters.draw_failed = self.text_counters.draw_failed.saturating_add(1);
        log::error!("text_draw_failed step={step} error={err}");
        self.text = None;
        if let Some(name) = self.presentation.cancel() {
            self.painter = Painter::new(name, FACE_BAND_ROWS);
        }
    }
}
