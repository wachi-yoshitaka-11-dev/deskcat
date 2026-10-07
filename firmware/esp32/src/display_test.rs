//! `DISP-01`のbring-up（単色fill、四隅pattern）を、main loopの中で1段ずつ進める試験モード。
//!
//! `bringup-display-13` feature付きbuildだけがこのmoduleを持つ（`main.rs`の
//! module doc「`DISP-01`のbring-upを有効にする手順」節。**既定buildがLCD関連pinへ
//! 触れないことは#451の決定であり、変えない**）。
//!
//! # なぜ1段ずつ進めるか
//!
//! [#487]のPR B1より前は、`run_display_bringup`が5色のfillと四隅patternを1回の呼び出しで
//! 描き切り、色ごとに[`config::DISPLAY_HOLD_MS`]の`FreeRtos::delay_ms`を挟んでいた。
//! その間main loopへ戻らないため、heartbeatは描画段階の境界の`bringup_hb`だけだった。
//! またLCDを描くbuildはPi linkを持たなかった（当時の`pi-protocol-mode`と排他）。どちらも
//! [#13]の受け入れ条件6「更新中も通信とwatchdogがactiveである」を示せない理由だった。
//!
//! このmoduleは描画を小さな段に分け、[`DisplayBringup::poll`]が1回に1段だけ描いて戻る。
//! 1段は、画面の横幅いっぱいで[`BAND_ROWS`]行の帯1本、または四隅patternの要素1つである（`y_axis`だけは
//! 破線1本ごとに1段とする）。
//! 色を保つ間（[`config::DISPLAY_HOLD_MS`]）は描かず、締切だけを返す。main loopは
//! 段と段の間に、heartbeat、health snapshot、Pi linkの受信と、budgetが残っている間の`boot`の
//! 再送を回す（`crate::boot_session`。ACKを受けた後は再送しない）。**したがって、描画の間も
//! heartbeatとPi linkの受信は止まらない見込みである。これはcodeの構造から導いたもので、実機では
//! 確かめていない。**ほかの文書と注記は、この段落を参照する。
//!
//! **1段の所要時間は測っていない。**帯1本は`WIDTH`×[`BAND_ROWS`]×2 byte（320×16×2＝10,240 byte）
//! であり、SPI clock 6 MHz（`docs/hardware/gpio-assignment.md`の`LCD-SCLK`行）なら
//! 転送だけで約14 msになる（計算であり、実測ではない。command・CS・chunkの切れ目の分は含まない）。
//! **段ごとの所要時間はlogへ出さない。**出すのは、色ごとの帯の合計（`display_fill`の`elapsed_us`）と、
//! 四隅patternの要素の合計（`display_corner_pattern`の`elapsed_us`）である。
//!
//! # logの行
//!
//! 行の名前（`display_fill`、`display_pattern_element`、`display_corner_pattern`、
//! `display_pattern_hold`）は、1回で描き切っていた頃と同じにする。`docs/hardware/power-budget.md`の
//! `DISP-01`追加接続の手順が、完了の証拠としてこれらの行を挙げているためである。bring-upを終えた印の
//! `display_bringup_done`だけは#487で足した行であり、同手順の手順9の完了の証拠には挙がっていない。
//!
//! **どちらの行も`elapsed_us`の意味は変わる。以前の値（`docs/hardware/experiment-log.md`の
//! `EXP-016`〜`EXP-018`）と直接比べない。**
//!
//! **失敗したときの振る舞いは、1回で描き切っていた頃と同じにする**（`docs/hardware/power-budget.md`の
//! `DISP-01`追加接続の手順9の「失敗時の扱い」が、この振る舞いを前提にしている）。
//! - 単色fill：ある色の帯が1本でも失敗したら、`display_fill_failed`を出し、その色の残りの帯を
//!   描かず、`display_fill`の行も保持も出さずに次の色へ進む。**`display_fill`の行は、その色の
//!   帯をすべて描けたときだけ出る。**
//! - 四隅patternの背景：帯が1本でも失敗したら、`display_corner_background_failed`を出し、四隅を
//!   描かずにbring-upを終える（driverを手放し、backlightが消える）。
//! - 四隅と軸：1つの要素の失敗は、他の要素の描画を止めない。
//!
//! # 単色fill
//!
//! 黒、赤、緑、青、白の5色を既知のRGB565値で順に塗る。受け入れ条件「単色fillが正しい」
//! 「Color orderが正しい」「更新timingを測定した」に対応する。**正しいかどうかの判定は
//! このmoduleでは行わない。**実機のLCDを目視して判定するのは人間であり（`AGENTS.md`ハードウェア
//! 安全、初回通電は人間監視下）、このmoduleは色と所要時間を機械可読な形でlogへ残すだけである。
//! 描けた色は[`config::DISPLAY_HOLD_MS`]だけ表示したまま保ち、人が色ごとに写真を撮れるようにする。
//! `display_fill`の行の`hold_ms`がその値であり、行が出てから次の色へ移るまでの目安になる。
//!
//! # 四隅pattern
//!
//! 四隅と2本の軸を描き、論理座標の四隅と向きを実機の写真で確かめられるようにする
//! （受け入れ条件「四隅とorientationが正しい」、[#13]）。座標は**論理座標**
//! （[`display::MADCTL_LANDSCAPE`]の向き、[`display::WIDTH`]×[`display::HEIGHT`]）である。
//! 色が入れ替わって見えても判別できるよう、**色だけでなく大きさと線の形でも区別する。**
//!
//! | 要素 | 論理座標 | 大きさ | 色 |
//! |---|---|---|---|
//! | `origin` | `x`＝0・`y`＝0の隅 | 48 px角（最大） | 赤 |
//! | `x_end` | `x`の最大・`y`＝0の隅 | 32 px角 | 緑 |
//! | `y_end` | `x`＝0・`y`の最大の隅 | 24 px角 | 青 |
//! | `far` | `x`・`y`とも最大の隅 | 16 px角（最小） | 白 |
//! | `x_axis` | `origin`から`x_end`へ、`x`の増える向き | 幅8 pxの**実線** | 白 |
//! | `y_axis` | `origin`から`y_end`へ、`y`の増える向き | 幅8 pxの**破線** | 白 |
//!
//! 各要素の座標は`display_pattern_element`の行としてlogにも出す。描き終えたら
//! [`config::DISPLAY_HOLD_MS`]だけ表示したまま保つ（`display_pattern_hold`の行）。
//!
//! **写真の見方。**判定の向きは「横向き（320×240）、14 pinの`J2`のheaderを右の辺にして見た向き」
//! である（[#13の判断(2)](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13#issuecomment-5860773100)）。
//! この向きでpanelを撮り、`J2`のheaderを写真に入れる。`origin`が左上、`x_end`が右上、`y_end`が
//! 左下、`far`が右下にあり、実線が右へ、破線が下へ伸びていれば、論理の四隅が物理の四隅と
//! 一致している（`docs/hardware/experiment-log.md`の`EXP-018`）。向きの値の決め方（MADCTLの`MV`を
//! 0にして生の座標を描き、線の伸びる向きから`MY`／`MX`を読む。§9.3 p.208の表による）は`EXP-018`に
//! ある。
//!
//! # 終わり方
//!
//! 最後の保持（`display_pattern_hold`）が済むと、[`DisplayBringup::poll`]は終わった状態
//! （[`DisplayBringup::is_done`]）で返り、呼び出し側（`main()`）が手放す。driverが持つ`LCD-BL`の
//! pinが解放され、backlightが消える（1回で描き切っていた頃に`run_display_bringup`から戻ったときと
//! 同じである）。**`face-21`付きbuildだけは手放さず、表情の描画へ渡すため、backlightは点いたままである**
//! （`crate::face`）。失敗で終えるときは、[`DisplayBringup::poll`]がその場で[`None`]を返して手放す。
//!
//! [#13]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13
//! [#487]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487

use std::time::Instant;

use crate::config;
use crate::display::{self, Ili9341};

/// 帯1本の行数。**安全要件の5項目に効かない値である**（描画の分け方であり、電流も
/// 表示の内容も変えない）。`HEIGHT`（240）を割り切る値にして、最後の帯を短くしない。
pub const BAND_ROWS: u16 = 16;

/// 単色fillの順と色。1回で描き切っていた頃の`run_fill_tests`と同じである。
const FILLS: [(&str, u16); 5] = [
    ("black", display::color::BLACK),
    ("red", display::color::RED),
    ("green", display::color::GREEN),
    ("blue", display::color::BLUE),
    ("white", display::color::WHITE),
];

/// 四隅patternの寸法。1回で描き切っていた頃の`run_corner_pattern`と同じである
/// （module doc「四隅pattern」）。
const ORIGIN: u16 = 48;
const X_END: u16 = 32;
const Y_END: u16 = 24;
const FAR: u16 = 16;
const AXIS: u16 = 8;
/// 軸の線を、`origin`の正方形の中ほどから出す。panelの縁の直下を避けるためである。
const AXIS_OFFSET: u16 = 20;
const DASH: u16 = 16;

/// 今どの段にいるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// `FILLS[index]`の色で、`row`行目から帯を描く。`drawn_us`はその色の描画時間の合計。
    Fill {
        index: usize,
        row: u16,
        drawn_us: u128,
    },
    /// `FILLS[index]`の色を保つ。`until_ms`まで描かない。
    FillHold { index: usize, until_ms: u64 },
    /// 四隅patternの背景（黒）を、`row`行目から帯で描く。
    PatternBackground { row: u16, drawn_us: u128 },
    /// 四隅の`index`番目を描く。
    Corner { index: usize, drawn_us: u128 },
    /// `x`の軸（実線）を描く。
    XAxis { drawn_us: u128 },
    /// `y`の軸（破線）の、`offset`から始まる1本を描く。
    YAxis { offset: u16, drawn_us: u128 },
    /// patternを保つ。`until_ms`まで描かない。
    PatternHold { until_ms: u64 },
    /// bring-upを終えた。呼び出し側が[`DisplayBringup::into_lcd`]でdriverを受け取るか、手放す。
    Done,
}

/// `DISP-01`のbring-upの状態機械。
pub struct DisplayBringup<'d> {
    lcd: Ili9341<'d>,
    step: Step,
}

impl<'d> DisplayBringup<'d> {
    /// 初期化とbacklightの点灯を済ませたdriverを受け取り、最初の段を用意する。
    pub fn new(lcd: Ili9341<'d>) -> Self {
        Self {
            lcd,
            step: Step::Fill {
                index: 0,
                row: 0,
                drawn_us: 0,
            },
        }
    }

    /// bring-upの段を全部終えたか。
    pub fn is_done(&self) -> bool {
        matches!(self.step, Step::Done)
    }

    /// driverを受け取る。手放せばbacklightが消える。
    pub fn into_lcd(self) -> Ili9341<'d> {
        self.lcd
    }

    /// 次の段の締切（uptimeのms）を返す。描く段なら`now_ms`（すぐ描いてよい）、
    /// 保つ段ならその終わりの時刻である。
    pub fn next_deadline_ms(&self, now_ms: u64) -> u64 {
        match self.step {
            Step::FillHold { until_ms, .. } | Step::PatternHold { until_ms } => until_ms,
            _ => now_ms,
        }
    }

    /// 締切に達していれば1段だけ進める。
    ///
    /// **どの段で失敗してもpanicしない。**errorは`log::error!`へ分類する。失敗の後にどこへ進むかは、
    /// 1回で描き切っていた頃と同じである（module doc「logの行」）。
    ///
    /// 失敗で終える場合は`None`を返す。driverはここで手放され、backlightが消える。最後の保持が済んだら
    /// 終わった状態（[`DisplayBringup::is_done`]）で返し、呼び出し側が[`DisplayBringup::into_lcd`]で
    /// driverを受け取るか、手放す（手放せば、module doc「終わり方」のとおりbacklightが消える）。
    pub fn poll(mut self, now_ms: u64) -> Option<Self> {
        if now_ms < self.next_deadline_ms(now_ms) {
            return Some(self);
        }
        self.step = match self.step {
            Step::Fill {
                index,
                row,
                drawn_us,
            } => {
                let (name, color) = FILLS[index];
                let Some(band_us) = self.band(row, color, "display_fill_failed", name) else {
                    // この色は描けなかった。残りの帯を描かず、保持もせずに次の色へ進む
                    // （module doc「logの行」の失敗の扱い）。
                    return Some(Self {
                        step: Self::after_fill(index),
                        ..self
                    });
                };
                let drawn_us = drawn_us + band_us;
                let next_row = row + BAND_ROWS;
                if next_row < display::HEIGHT {
                    Step::Fill {
                        index,
                        row: next_row,
                        drawn_us,
                    }
                } else {
                    let hold_ms = config::DISPLAY_HOLD_MS;
                    log::info!(
                        "display_fill name={name} color=0x{color:04x} elapsed_us={drawn_us} hold_ms={hold_ms}"
                    );
                    Step::FillHold {
                        index,
                        until_ms: now_ms + u64::from(hold_ms),
                    }
                }
            }
            Step::FillHold { index, .. } => Self::after_fill(index),
            Step::PatternBackground { row, drawn_us } => {
                let Some(band_us) = self.band(
                    row,
                    display::color::BLACK,
                    "display_corner_background_failed",
                    "background",
                ) else {
                    // 背景が描けなければ四隅を描かずに終える（module doc「logの行」の失敗の扱い）。
                    return None;
                };
                let drawn_us = drawn_us + band_us;
                let next_row = row + BAND_ROWS;
                if next_row < display::HEIGHT {
                    Step::PatternBackground {
                        row: next_row,
                        drawn_us,
                    }
                } else {
                    // `display_corner_pattern`の`elapsed_us`は四隅と軸の合計であり、背景を含めない。
                    // 以前の値と比べられるかはmodule doc「logの行」。背景の時間は捨てる。
                    Step::Corner {
                        index: 0,
                        drawn_us: 0,
                    }
                }
            }
            Step::Corner { index, drawn_us } => {
                let (w, h) = (display::WIDTH, display::HEIGHT);
                // `(name, x, y, 一辺, color)`。座標は論理座標（`x`＝CASET、`y`＝PASET。向きは`MADCTL_LANDSCAPE`）。
                let corners: [(&str, u16, u16, u16, u16); 4] = [
                    ("origin", 0, 0, ORIGIN, display::color::RED),
                    ("x_end", w - X_END, 0, X_END, display::color::GREEN),
                    ("y_end", 0, h - Y_END, Y_END, display::color::BLUE),
                    ("far", w - FAR, h - FAR, FAR, display::color::WHITE),
                ];
                let (name, x, y, size, color) = corners[index];
                log::info!(
                    "display_pattern_element name={name} x={x} y={y} width={size} height={size} color=0x{color:04x}"
                );
                let start = Instant::now();
                if let Err(err) = self.lcd.fill_rect(x, y, x + size - 1, y + size - 1, color) {
                    log::error!("display_corner_failed corner={name} error={err}");
                }
                let drawn_us = drawn_us + start.elapsed().as_micros();
                if index + 1 < corners.len() {
                    Step::Corner {
                        index: index + 1,
                        drawn_us,
                    }
                } else {
                    Step::XAxis { drawn_us }
                }
            }
            Step::XAxis { drawn_us } => {
                // 実線: `origin`の右端から`x_end`の左端まで、`x`の増える向き。
                let x_len = display::WIDTH - X_END - ORIGIN;
                log::info!(
                    "display_pattern_element name=x_axis x={ORIGIN} y={AXIS_OFFSET} width={x_len} height={AXIS} style=solid"
                );
                let start = Instant::now();
                if let Err(err) = self.lcd.fill_rect(
                    ORIGIN,
                    AXIS_OFFSET,
                    ORIGIN + x_len - 1,
                    AXIS_OFFSET + AXIS - 1,
                    display::color::WHITE,
                ) {
                    log::error!("display_axis_failed axis=x_axis error={err}");
                }
                // 計時は`y_axis`の要素のlogより前で止める（要素の描画だけを測る）。
                let drawn_us = drawn_us + start.elapsed().as_micros();
                let y_len = display::HEIGHT - Y_END - ORIGIN;
                log::info!(
                    "display_pattern_element name=y_axis x={AXIS_OFFSET} y={ORIGIN} width={AXIS} height={y_len} style=dashed dash={DASH}"
                );
                Step::YAxis {
                    offset: 0,
                    drawn_us,
                }
            }
            Step::YAxis { offset, drawn_us } => {
                // 破線: `origin`の下端から`y_end`の上端まで、`y`の増える向き。`DASH`ごとに描いて空ける。
                let y_len = display::HEIGHT - Y_END - ORIGIN;
                let y0 = ORIGIN + offset;
                let y1 = y0 + DASH.min(y_len - offset) - 1;
                let start = Instant::now();
                if let Err(err) = self.lcd.fill_rect(
                    AXIS_OFFSET,
                    y0,
                    AXIS_OFFSET + AXIS - 1,
                    y1,
                    display::color::WHITE,
                ) {
                    log::error!("display_axis_failed axis=y_axis y={y0} error={err}");
                }
                let drawn_us = drawn_us + start.elapsed().as_micros();
                let next_offset = offset + 2 * DASH;
                if next_offset < y_len {
                    Step::YAxis {
                        offset: next_offset,
                        drawn_us,
                    }
                } else {
                    log::info!("display_corner_pattern elapsed_us={drawn_us}");
                    let hold_ms = config::DISPLAY_HOLD_MS;
                    log::info!("display_pattern_hold hold_ms={hold_ms}");
                    Step::PatternHold {
                        until_ms: now_ms + u64::from(hold_ms),
                    }
                }
            }
            Step::PatternHold { .. } => {
                log::info!("display_bringup_done");
                Step::Done
            }
            Step::Done => Step::Done,
        };
        Some(self)
    }

    /// `index`番目の色を終えた（描けた後の保持が済んだ、または描けなかった）後の段。
    fn after_fill(index: usize) -> Step {
        if index + 1 < FILLS.len() {
            Step::Fill {
                index: index + 1,
                row: 0,
                drawn_us: 0,
            }
        } else {
            Step::PatternBackground {
                row: 0,
                drawn_us: 0,
            }
        }
    }

    /// `row`行目から[`BAND_ROWS`]行の帯を`color`で塗り、描画時間（µs）を返す。
    /// 失敗したら`error_tag`の行へ分類して`None`を返す（止めるかどうかは呼び出し側が決める）。
    fn band(&mut self, row: u16, color: u16, error_tag: &str, name: &str) -> Option<u128> {
        let last_row = (row + BAND_ROWS - 1).min(display::HEIGHT - 1);
        let start = Instant::now();
        match self
            .lcd
            .fill_rect(0, row, display::WIDTH - 1, last_row, color)
        {
            Ok(()) => Some(start.elapsed().as_micros()),
            Err(err) => {
                log::error!("{error_tag} name={name} row={row} error={err}");
                None
            }
        }
    }
}
