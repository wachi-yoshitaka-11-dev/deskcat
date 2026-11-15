//! `set_expression`（Protocol §5.2）の表情を、LCD上の矩形の列として返す。
//!
//! このcrateはLCDもSPIも知らない。名前から「どの矩形をどの色で、どの順に塗るか」を返すだけであり、
//! 塗るのは`firmware/esp32`の`crate::display`である（hostでtestできるように分けた）。
//!
//! # 値の扱い
//!
//! 矩形の座標と色、[`MAX_TRANSITION_MS`]は**表示の見た目と受理の上限であり、安全要件の5項目に効かない。**
//! 最小の図形で始めた暫定の値である。デザインは後で決める（#21の決定。コンセプト画像に沿う）。
//! 画面の寸法は`firmware/esp32/src/display.rs`の`WIDTH`／`HEIGHT`の複製であり、firmware側が
//! 一致を検査する。

use deskcat_protocol::{ErrorCode, ExpressionName, SetExpression};

pub mod text;

/// 論理座標での画面の幅（`firmware/esp32/src/display.rs`の`WIDTH`と同じ。firmwareが一致を検査する）。
pub const SCREEN_WIDTH: u16 = 320;
/// 論理座標での画面の高さ（同`HEIGHT`）。
pub const SCREEN_HEIGHT: u16 = 240;

/// `transition_ms`の上限（ms）。§5.2は上限をdisplay実装で定義するとしている。
/// **暫定の一般値である。**今の描画は切り替えを補間せず、受けた表情へ即座に描き替える。
pub const MAX_TRANSITION_MS: u32 = 2_000;

/// RGB565の色。`firmware/esp32/src/display.rs`の`color`と同じく、各chをfull scaleにした値と、
/// 見た目だけのための色を持つ。
pub mod color {
    /// 黒。
    pub const BLACK: u16 = 0x0000;
    /// 白。
    pub const WHITE: u16 = 0xFFFF;
    /// 頬の薄い赤。
    pub const CHEEK: u16 = 0xFBAE;
    /// 寒い顔の頬の薄い青。
    pub const CHILL: u16 = 0x65BF;
}

/// 塗る矩形1つ。`x1`と`y1`は端を含む（`Ili9341::fill_rect`の引数と同じ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// 左端。
    pub x0: u16,
    /// 上端。
    pub y0: u16,
    /// 右端（含む）。
    pub x1: u16,
    /// 下端（含む）。
    pub y1: u16,
    /// RGB565。
    pub color: u16,
}

/// コメント（`show_text`）を出す下の帯の上端。帯は`TEXT_BAND_TOP`から画面の下端まで（高さ[`TEXT_BAND_HEIGHT`]）。
/// 表情の矩形は全部これより上にあり、表情を描いても帯は塗られない（#743）。
pub const TEXT_BAND_TOP: u16 = 212;
/// コメントの帯の高さ。
pub const TEXT_BAND_HEIGHT: u16 = SCREEN_HEIGHT - TEXT_BAND_TOP;

const fn rect(x0: u16, y0: u16, w: u16, h: u16, color: u16) -> Rect {
    Rect {
        x0,
        y0,
        x1: x0 + w - 1,
        y1: y0 + h - 1,
        color,
    }
}

/// 表情の背景。コメントの帯（[`TEXT_BAND_TOP`]から下）は塗らない。
const BACKGROUND: Rect = rect(0, 0, SCREEN_WIDTH, TEXT_BAND_TOP, color::BLACK);

const NEUTRAL: [Rect; 4] = [
    BACKGROUND,
    rect(80, 54, 48, 64, color::WHITE),
    rect(192, 54, 48, 64, color::WHITE),
    rect(130, 160, 60, 8, color::WHITE),
];

// 目は細い横線（笑った目）、口は広く、頬を足す。
const HAPPY: [Rect; 6] = [
    BACKGROUND,
    rect(76, 80, 56, 12, color::WHITE),
    rect(188, 80, 56, 12, color::WHITE),
    rect(110, 152, 100, 16, color::WHITE),
    rect(40, 134, 40, 20, color::CHEEK),
    rect(240, 134, 40, 20, color::CHEEK),
];

// 目は大きく、黒目を足す。口は小さな四角。
const SURPRISED: [Rect; 6] = [
    BACKGROUND,
    rect(72, 34, 64, 88, color::WHITE),
    rect(184, 34, 64, 88, color::WHITE),
    rect(96, 66, 16, 24, color::BLACK),
    rect(208, 66, 16, 24, color::BLACK),
    rect(144, 152, 32, 40, color::WHITE),
];

// 目を回す。目は入れ子の四角（白・黒・白・黒）で渦に見せ、内側を寄せて中心をずらす。口は段違いの波線。
const DIZZY: [Rect; 11] = [
    BACKGROUND,
    rect(72, 46, 64, 64, color::WHITE),
    rect(80, 54, 48, 48, color::BLACK),
    rect(88, 64, 32, 32, color::WHITE),
    rect(98, 76, 14, 14, color::BLACK),
    rect(184, 46, 64, 64, color::WHITE),
    rect(192, 54, 48, 48, color::BLACK),
    rect(200, 64, 32, 32, color::WHITE),
    rect(210, 76, 14, 14, color::BLACK),
    rect(120, 158, 40, 8, color::WHITE),
    rect(160, 166, 40, 8, color::WHITE),
];

// 目は閉じた細い横線、口は小さく、右上に「Z」を足す。
const SLEEPING: [Rect; 9] = [
    BACKGROUND,
    rect(80, 104, 48, 8, color::WHITE),
    rect(192, 104, 48, 8, color::WHITE),
    rect(150, 160, 20, 6, color::WHITE),
    rect(248, 20, 32, 6, color::WHITE),
    rect(268, 26, 8, 6, color::WHITE),
    rect(260, 32, 8, 6, color::WHITE),
    rect(252, 38, 8, 6, color::WHITE),
    rect(248, 44, 32, 6, color::WHITE),
];

// 暑い。目は上まぶたで半分閉じ、口は大きく開けて舌を出す。頬は赤く、右上に汗を足す。
const HOT: [Rect; 12] = [
    BACKGROUND,
    rect(80, 68, 48, 40, color::WHITE),
    rect(80, 68, 48, 20, color::BLACK),
    rect(192, 68, 48, 40, color::WHITE),
    rect(192, 68, 48, 20, color::BLACK),
    rect(128, 140, 64, 40, color::WHITE),
    rect(144, 160, 32, 20, color::CHEEK),
    rect(40, 124, 40, 20, color::CHEEK),
    rect(240, 124, 40, 20, color::CHEEK),
    rect(264, 38, 12, 12, color::WHITE),
    rect(260, 50, 20, 16, color::WHITE),
    rect(264, 66, 12, 8, color::WHITE),
];

// 寒い。目は小さく黒目を端へ寄せ、口は歯を鳴らす格子にする。頬は薄い青で、両脇に震えの縦線を足す。
const COLD: [Rect; 13] = [
    BACKGROUND,
    rect(84, 68, 40, 40, color::WHITE),
    rect(88, 82, 10, 10, color::BLACK),
    rect(196, 68, 40, 40, color::WHITE),
    rect(200, 82, 10, 10, color::BLACK),
    rect(124, 152, 72, 32, color::WHITE),
    rect(140, 152, 4, 32, color::BLACK),
    rect(158, 152, 4, 32, color::BLACK),
    rect(176, 152, 4, 32, color::BLACK),
    rect(40, 134, 40, 20, color::CHILL),
    rect(240, 134, 40, 20, color::CHILL),
    rect(20, 74, 8, 60, color::WHITE),
    rect(292, 74, 8, 60, color::WHITE),
];

// 困った。目は下を向く黒目、眉は内側が上がる階段、口は小さな波線。
const WORRIED: [Rect; 14] = [
    BACKGROUND,
    rect(80, 68, 48, 56, color::WHITE),
    rect(96, 96, 16, 20, color::BLACK),
    rect(192, 68, 48, 56, color::WHITE),
    rect(208, 96, 16, 20, color::BLACK),
    rect(68, 50, 16, 6, color::WHITE),
    rect(84, 42, 16, 6, color::WHITE),
    rect(100, 34, 16, 6, color::WHITE),
    rect(204, 34, 16, 6, color::WHITE),
    rect(220, 42, 16, 6, color::WHITE),
    rect(236, 50, 16, 6, color::WHITE),
    rect(128, 164, 24, 6, color::WHITE),
    rect(152, 158, 24, 6, color::WHITE),
    rect(176, 164, 16, 6, color::WHITE),
];

/// `name`の表情を塗る矩形を、塗る順に返す。先頭は、コメントの帯より上の背景である。表情の矩形は帯にかからない。
///
/// 未知の名前（`ExpressionName`は`non_exhaustive`）には`None`を返す。呼び出し側は拒否する。
#[must_use]
pub fn shapes(name: ExpressionName) -> Option<&'static [Rect]> {
    match name {
        ExpressionName::Neutral => Some(&NEUTRAL),
        ExpressionName::Happy => Some(&HAPPY),
        ExpressionName::Surprised => Some(&SURPRISED),
        ExpressionName::Dizzy => Some(&DIZZY),
        ExpressionName::Sleeping => Some(&SLEEPING),
        ExpressionName::Hot => Some(&HOT),
        ExpressionName::Cold => Some(&COLD),
        ExpressionName::Worried => Some(&WORRIED),
        _ => None,
    }
}

/// `transition_ms`が上限以内か検査する。超えたら[`ErrorCode::OutOfRange`]（§7「上限のある値が範囲外」）。
///
/// # Errors
///
/// `transition_ms`が[`MAX_TRANSITION_MS`]を超えたとき。
pub const fn check_transition(transition_ms: u32) -> Result<(), ErrorCode> {
    if transition_ms > MAX_TRANSITION_MS {
        Err(ErrorCode::OutOfRange)
    } else {
        Ok(())
    }
}

/// LCDの状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayState {
    /// 現時点で表情を描けない。表情を持たない試験構成や、LCDの初期化前・初期化失敗時に使う。
    Unavailable,
    /// LCDは初期化済みだが、bring-upの試験が使っている間である。終われば描ける。
    Busy,
    /// 表情を描ける。
    Ready,
}

/// `set_expression`を受理してよいか判定する。受理なら描く表情を返す。
///
/// 順序は§8の手順に従う。**名前の検証（手順7）→session権限（手順8）→`transition_ms`の上限とhardwareの準備
/// （手順10。§12が、手順7が持たない`transition_ms`の上限を手順10で適用すると定める）**であり、
/// 入れ替えると、別の失敗の符号が先に返る（範囲外のcommandが古いsessionから来ても`stale_session`を返し、
/// Piが`hello`から再開できるようにする）。
///
/// # Errors
///
/// - `ErrorCode::OutOfRange`: `transition_ms`が[`MAX_TRANSITION_MS`]を超える。
/// - `ErrorCode::InvalidPayload`: 名前に対応する表情が無い（`ExpressionName`は`non_exhaustive`）。
/// - `ErrorCode::StaleSession`: `sid`が現在のPi sessionでない。
/// - `ErrorCode::HardwareUnavailable`: 表情を描かない構成である（`face-21`を持たないbuild。待っても受け付けられない。§7）。
/// - `ErrorCode::Busy`: LCDがbring-upの試験で塞がっている（終われば受け付けられる。§7）。
pub fn judge(
    command: &SetExpression,
    session_is_current: bool,
    display: DisplayState,
) -> Result<ExpressionName, ErrorCode> {
    if shapes(command.name).is_none() {
        return Err(ErrorCode::InvalidPayload);
    }
    if !session_is_current {
        return Err(ErrorCode::StaleSession);
    }
    check_transition(command.transition_ms)?;
    match display {
        DisplayState::Unavailable => Err(ErrorCode::HardwareUnavailable),
        DisplayState::Busy => Err(ErrorCode::Busy),
        DisplayState::Ready => Ok(command.name),
    }
}

/// 1表情を、`max_rows`行までの帯へ分けて1本ずつ返す塗り手。
///
/// 全画面の背景を1回で塗ると、LCDのSPI転送（`6 MHz`で約200 ms。計算であり実測ではない）の間main loopへ戻れない。
/// そのため背景を帯に分け、1本塗るごとにmain loopへ戻れるようにする。
#[derive(Debug, Clone)]
pub struct Painter {
    shapes: &'static [Rect],
    index: usize,
    next_y: u16,
    max_rows: u16,
}

impl Painter {
    /// `name`の表情を塗る塗り手を作る。`max_rows`が0なら1行として扱う。
    #[must_use]
    pub fn new(name: ExpressionName, max_rows: u16) -> Option<Self> {
        Some(Self {
            shapes: shapes(name)?,
            index: 0,
            next_y: 0,
            max_rows: max_rows.max(1),
        })
    }

    /// 次に塗る帯を返す。全部塗り終えたら`None`を返す。
    pub fn next_stripe(&mut self) -> Option<Rect> {
        let r = *self.shapes.get(self.index)?;
        let y0 = r.y0.max(self.next_y);
        let y1 = r.y1.min(y0.saturating_add(self.max_rows - 1));
        if y1 == r.y1 {
            self.index += 1;
            self.next_y = 0;
        } else {
            self.next_y = y1 + 1;
        }
        Some(Rect { y0, y1, ..r })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [ExpressionName; 8] = [
        ExpressionName::Neutral,
        ExpressionName::Happy,
        ExpressionName::Surprised,
        ExpressionName::Dizzy,
        ExpressionName::Sleeping,
        ExpressionName::Hot,
        ExpressionName::Cold,
        ExpressionName::Worried,
    ];

    #[test]
    fn every_expression_starts_with_a_background_above_the_text_band() {
        for name in NAMES {
            let first = shapes(name).expect("定義済み")[0];
            assert_eq!(
                (first.x0, first.y0, first.x1, first.y1),
                (0, 0, SCREEN_WIDTH - 1, TEXT_BAND_TOP - 1),
                "{name:?}"
            );
        }
    }

    #[test]
    fn no_rect_touches_the_text_band() {
        assert_eq!((TEXT_BAND_TOP, TEXT_BAND_HEIGHT), (212, 28));
        for name in NAMES {
            for r in shapes(name).expect("定義済み") {
                assert!(r.y1 < TEXT_BAND_TOP, "{name:?} {r:?}");
            }
        }
    }

    #[test]
    fn every_rect_is_non_empty_and_inside_the_screen() {
        for name in NAMES {
            for r in shapes(name).expect("定義済み") {
                assert!(r.x0 <= r.x1 && r.y0 <= r.y1, "{name:?} {r:?}");
                assert!(
                    r.x1 < SCREEN_WIDTH && r.y1 < SCREEN_HEIGHT,
                    "{name:?} {r:?}"
                );
            }
        }
    }

    #[test]
    fn all_expressions_look_different() {
        assert_eq!(NAMES, ExpressionName::ALL);
        for (i, a) in NAMES.iter().enumerate() {
            for b in &NAMES[i + 1..] {
                assert_ne!(shapes(*a), shapes(*b), "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn transition_is_accepted_up_to_the_limit_and_rejected_above() {
        assert_eq!(check_transition(0), Ok(()));
        assert_eq!(check_transition(MAX_TRANSITION_MS), Ok(()));
        assert_eq!(
            check_transition(MAX_TRANSITION_MS + 1),
            Err(ErrorCode::OutOfRange)
        );
        assert_eq!(check_transition(u32::MAX), Err(ErrorCode::OutOfRange));
    }

    fn cmd(name: ExpressionName, transition_ms: u32) -> SetExpression {
        SetExpression {
            name,
            transition_ms,
        }
    }

    #[test]
    fn judge_accepts_a_current_session_with_a_ready_display() {
        assert_eq!(
            judge(&cmd(ExpressionName::Happy, 300), true, DisplayState::Ready),
            Ok(ExpressionName::Happy)
        );
    }

    #[test]
    fn judge_returns_the_error_of_the_earliest_failing_step() {
        let over = MAX_TRANSITION_MS + 1;
        // sessionは、値の上限とhardwareより先（§8手順8→10）。範囲外でも古いsessionは`stale_session`。
        assert_eq!(
            judge(
                &cmd(ExpressionName::Happy, over),
                false,
                DisplayState::Unavailable
            ),
            Err(ErrorCode::StaleSession)
        );
        // 値の上限は、hardwareより先（手順10の中）。
        assert_eq!(
            judge(
                &cmd(ExpressionName::Happy, over),
                true,
                DisplayState::Unavailable
            ),
            Err(ErrorCode::OutOfRange)
        );
        assert_eq!(
            judge(
                &cmd(ExpressionName::Happy, 0),
                true,
                DisplayState::Unavailable
            ),
            Err(ErrorCode::HardwareUnavailable)
        );
        assert_eq!(
            judge(&cmd(ExpressionName::Happy, 0), true, DisplayState::Busy),
            Err(ErrorCode::Busy)
        );
    }

    #[test]
    fn painter_stripes_cover_exactly_the_rects_in_order() {
        for name in NAMES {
            for max_rows in [1, 7, 16, SCREEN_HEIGHT, u16::MAX / 2, u16::MAX] {
                let mut painter = Painter::new(name, max_rows).expect("定義済み");
                let mut stripes = Vec::new();
                while let Some(s) = painter.next_stripe() {
                    assert!(s.y1 - s.y0 < max_rows.max(1), "{name:?} {max_rows}");
                    stripes.push(s);
                }
                // 各rectは、連続した帯の和であり、重なりも隙間も無い。
                let mut it = stripes.iter().peekable();
                for r in shapes(name).expect("定義済み") {
                    let mut y = r.y0;
                    while y <= r.y1 {
                        let s = it.next().expect("帯が足りる");
                        assert_eq!((s.x0, s.x1, s.color), (r.x0, r.x1, r.color));
                        assert_eq!(s.y0, y, "{name:?} {max_rows}");
                        y = s.y1 + 1;
                    }
                    assert_eq!(y, r.y1 + 1);
                }
                assert!(it.next().is_none(), "余分な帯が無い");
            }
        }
    }
}
