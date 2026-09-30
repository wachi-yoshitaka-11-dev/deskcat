//! `DISP-01`のbring-up（単色fill、四隅pattern）を、main loopの中で1段ずつ進める試験モード。
//!
//! `bringup-display-13` feature付きbuildだけがこのmoduleを持つ（`main.rs`の
//! module doc「`DISP-01`のbring-upを有効にする手順」節。**既定buildがLCD関連pinへ
//! 触れないことは#451の決定であり、変えない**）。
//!
//! # なぜ1段ずつ進めるか
//!
//! [#487]以前は、`run_display_bringup`が5色のfillと四隅patternを1回の呼び出しで
//! 描き切り、色ごとに[`config::DISPLAY_HOLD_MS`]の`FreeRtos::delay_ms`を挟んでいた。
//! その間main loopへ戻らないため、heartbeatは描画段階の境界の`bringup_hb`だけになり、
//! Pi linkの受信も止まっていた（[#13]の受け入れ条件6「更新中も通信とwatchdogがactiveで
//! ある」を示せない理由）。
//!
//! このmoduleは描画を小さな段に分け、[`DisplayBringup::poll`]が1回に1段だけ描いて戻る。
//! 1段は、画面の横幅いっぱいで[`BAND_ROWS`]行の帯1本、または四隅patternの要素1つである。
//! 色を保つ間（[`config::DISPLAY_HOLD_MS`]）は描かず、締切だけを返す。main loopは
//! 段と段の間に、heartbeat、health snapshot、Pi linkの受信と`boot`の再送を回す。
//!
//! **1段の所要時間は測っていない。**帯1本は`WIDTH`×[`BAND_ROWS`]×2 byte（320×16×2＝10,240 byte）
//! であり、SPI clock 6 MHz（`docs/hardware/gpio-assignment.md`の`LCD-SCLK`行）なら
//! 転送だけで約14 msになる（計算であり、実測ではない。command・CS・chunkの切れ目の分は含まない）。
//! 各段の実測は`display_fill`等の行の`elapsed_us`に出る。
//!
//! # logの行
//!
//! 行の名前（`display_fill`、`display_pattern_element`、`display_corner_pattern`、
//! `display_pattern_hold`）は、1回で描き切っていた頃と同じにする。
//! `docs/hardware/power-budget.md`の`DISP-01`追加接続の手順が、完了の証拠としてこれらの
//! 行を挙げているためである。**`elapsed_us`の意味は1点だけ変わる。**帯の描画時間の
//! 合計であり、帯と帯の間にmain loopが使った時間を含まない（1回で描き切っていた頃は、
//! 間にmain loopが入らなかったため、両者は同じだった）。
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
//! 最後の保持（`display_pattern_hold`）が済むと、[`DisplayBringup::poll`]はdriverを
//! 手放して[`None`]を返す。driverが持つ`LCD-BL`のpinが解放され、backlightが消える
//! （1回で描き切っていた頃に`run_display_bringup`から戻ったときと同じである）。
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
    /// **描画のerrorで止めない。**errorは`log::error!`へ分類して次の段へ進む（どの段で失敗しても
    /// panicしない）。1回で描き切っていた頃との違いが1つある。以前は四隅patternの背景（黒）の
    /// fillに失敗するとpatternを描かずに戻っていたが、今は背景の帯の失敗を記録して四隅へ進む。
    ///
    /// 最後の保持が済んだら`None`を返す。driverはここで手放され、backlightが消える
    /// （module doc「終わり方」）。
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
                let drawn_us = drawn_us + self.band(row, color, "display_fill_failed", name);
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
            Step::FillHold { index, .. } => {
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
            Step::PatternBackground { row, drawn_us } => {
                let drawn_us = drawn_us
                    + self.band(
                        row,
                        display::color::BLACK,
                        "display_corner_background_failed",
                        "background",
                    );
                let next_row = row + BAND_ROWS;
                if next_row < display::HEIGHT {
                    Step::PatternBackground {
                        row: next_row,
                        drawn_us,
                    }
                } else {
                    Step::Corner { index: 0, drawn_us }
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
                let y_len = display::HEIGHT - Y_END - ORIGIN;
                log::info!(
                    "display_pattern_element name=y_axis x={AXIS_OFFSET} y={ORIGIN} width={AXIS} height={y_len} style=dashed dash={DASH}"
                );
                Step::YAxis {
                    offset: 0,
                    drawn_us: drawn_us + start.elapsed().as_micros(),
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
                return None;
            }
        };
        Some(self)
    }

    /// `row`行目から[`BAND_ROWS`]行の帯を`color`で塗り、描画時間（µs）を返す。
    /// errorは`error_tag`の行へ分類し、時間はそのまま返す（止めない）。
    fn band(&mut self, row: u16, color: u16, error_tag: &str, name: &str) -> u128 {
        let last_row = (row + BAND_ROWS - 1).min(display::HEIGHT - 1);
        let start = Instant::now();
        if let Err(err) = self
            .lcd
            .fill_rect(0, row, display::WIDTH - 1, last_row, color)
        {
            log::error!("{error_tag} name={name} row={row} error={err}");
        }
        start.elapsed().as_micros()
    }
}
