//! `set_expression`（Protocol §5.2）の受理状態と、LCDへの表情の描画（[#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)）。
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
//! - `FaceDrawer`: LCD driverを持ち、受けた表情を帯1本ずつ塗る。1回の`FaceDrawer::poll`で帯1本だけ
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

use deskcat_face::{DisplayState, Painter};
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
}

impl<'d> FaceDrawer<'d> {
    /// 初期化とbacklightの点灯を済ませたdriverを受け取る。
    pub fn new(lcd: Ili9341<'d>) -> Self {
        Self { lcd, painter: None }
    }

    /// `name`の表情を、最初から塗り直す。塗っている途中なら、そちらは捨てる。
    pub fn show(&mut self, name: ExpressionName) {
        self.painter = Painter::new(name, FACE_BAND_ROWS);
        log::info!("face_show name={name:?}");
    }

    /// 塗る帯が残っているか。
    #[must_use]
    pub const fn is_drawing(&self) -> bool {
        self.painter.is_some()
    }

    /// 帯を1本だけ塗る。**失敗してもpanicしない。**失敗したら、その表情の残りを塗らずに諦める。
    pub fn poll(&mut self) {
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
}
