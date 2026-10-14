//! #23 の日本語独り言を、LCD/SPI を知らずに 8×12 glyph と画面上の cell へ変換する。
//!
//! k8x12 の ASCII・JIS X 0208 非漢字・第一水準を使う。画面全体に1行で出す仮 layout と最大表示時間は
//! PROTO-TBD-007 の実機測定前の暫定値であり、protocol の最終制限ではない。

use deskcat_protocol::{ErrorCode, ExpressionName};

use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

#[path = "speech_glyphs.rs"]
mod glyphs;

/// k8x12 の cell 幅。
pub const GLYPH_WIDTH: u16 = 8;
/// k8x12 の cell 高。
pub const GLYPH_HEIGHT: u16 = 12;
/// 実 LCD の可読性を確認するまでの暫定拡大率。
pub const SCALE: u16 = 2;
/// 暫定の1行 layout に入る最大文字数。
pub const MAX_TEXT_CHARS: usize = (SCREEN_WIDTH / (GLYPH_WIDTH * SCALE)) as usize;
/// 表示時間の暫定上限。最終値は PROTO-TBD-007 と実測で決める。
pub const MAX_TEXT_DURATION_MS: u32 = 30_000;

/// 表示できない入力の分類。wire の上限以外はまだ最終仕様ではない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextError {
    /// 空文字は画面を黒くするだけなので受理しない。
    Empty,
    /// 1行の仮 layout に入らない。
    TooLong,
    /// 収録していない文字。表示を途中で切らず、全体を拒否する。
    MissingGlyph(char),
    /// 暫定の表示時間上限を超えた、または0である。
    Duration,
}

impl TextError {
    /// 入力の表示可能範囲外として protocol §7 の `out_of_range` に分類する（暫定）。
    #[must_use]
    pub const fn code(self) -> ErrorCode {
        ErrorCode::OutOfRange
    }
}

/// 1文字の描画指示。1 row の最上位 bit が左端 pixel である。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphCell {
    /// 左端。
    pub x: u16,
    /// 上端。
    pub y: u16,
    /// 8×12 の行 bitmap。
    pub rows: [u8; GLYPH_HEIGHT as usize],
}

/// 1行の配置。文字列は保持せず、静的 glyph table の index だけを固定長配列に入れる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLayout {
    indices: [u16; MAX_TEXT_CHARS],
    len: usize,
    start_x: u16,
}

impl TextLayout {
    /// 全文字を事前検査し、1行に中央配置する。失敗時は描画の途中までを返さない。
    ///
    /// # Errors
    ///
    /// 空文字、最大文字数超過、未収録文字。
    pub fn new(text: &str) -> Result<Self, TextError> {
        let mut indices = [0; MAX_TEXT_CHARS];
        let mut len = 0;
        for char in text.chars() {
            if len == MAX_TEXT_CHARS {
                return Err(TextError::TooLong);
            }
            let index = glyphs::GLYPHS
                .binary_search_by_key(&char, |(c, _)| *c)
                .map_err(|_| TextError::MissingGlyph(char))?;
            indices[len] = u16::try_from(index).map_err(|_| TextError::MissingGlyph(char))?;
            len += 1;
        }
        if len == 0 {
            return Err(TextError::Empty);
        }
        let width = u16::try_from(len).map_err(|_| TextError::TooLong)? * GLYPH_WIDTH * SCALE;
        Ok(Self {
            indices,
            len,
            start_x: (SCREEN_WIDTH - width) / 2,
        })
    }

    /// 文字数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// 空かどうか。正常な構築では空にはならない。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// index 番目の cell を返す。
    #[must_use]
    pub fn cell(&self, index: usize) -> Option<GlyphCell> {
        if index >= self.len {
            return None;
        }
        Some(GlyphCell {
            x: self.start_x + u16::try_from(index).ok()? * GLYPH_WIDTH * SCALE,
            y: (SCREEN_HEIGHT - GLYPH_HEIGHT * SCALE) / 2,
            rows: glyphs::GLYPHS[usize::from(self.indices[index])].1,
        })
    }
}

/// 文字表示中は表情の最新要求を保持し、期限後にそれを描き直す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPresentation {
    expression: ExpressionName,
    deadline_ms: Option<u64>,
}

impl TextPresentation {
    /// 初期状態は neutral の表情で、文字表示は無い。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            expression: ExpressionName::Neutral,
            deadline_ms: None,
        }
    }

    /// 最新の表情を記録する。文字表示中は画面を変えず、復元時に使う。
    pub fn show_expression(&mut self, name: ExpressionName) -> Option<ExpressionName> {
        self.expression = name;
        if self.deadline_ms.is_none() {
            Some(name)
        } else {
            None
        }
    }

    /// 検証済みの文字表示を開始する。期限は要求時刻から数える。
    ///
    /// # Errors
    ///
    /// 表示時間が0か暫定上限を超えるとき。
    pub fn show_text(&mut self, duration_ms: u32, now_ms: u64) -> Result<(), TextError> {
        if duration_ms == 0 || duration_ms > MAX_TEXT_DURATION_MS {
            return Err(TextError::Duration);
        }
        self.deadline_ms = Some(now_ms.saturating_add(u64::from(duration_ms)));
        Ok(())
    }

    /// 期限が来たら、復元すべき最新の表情を1回だけ返す。
    pub fn expire(&mut self, now_ms: u64) -> Option<ExpressionName> {
        if self.deadline_ms.is_some_and(|deadline| now_ms >= deadline) {
            self.deadline_ms = None;
            Some(self.expression)
        } else {
            None
        }
    }

    /// 描画が失敗した場合に文字表示を終了し、復元対象を返す。
    pub fn cancel(&mut self) -> Option<ExpressionName> {
        self.deadline_ms.take().map(|_| self.expression)
    }

    /// 現在の文字表示の終了時刻。
    #[must_use]
    pub const fn deadline_ms(&self) -> Option<u64> {
        self.deadline_ms
    }
}

impl Default for TextPresentation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deskcat_domain::LOCAL_SPEECH_LINES;

    #[test]
    fn every_local_line_has_a_glyph_and_fits_the_provisional_row() {
        for line in LOCAL_SPEECH_LINES {
            let layout = TextLayout::new(line).unwrap();
            assert_eq!(layout.len(), line.chars().count());
            for i in 0..layout.len() {
                let cell = layout.cell(i).unwrap();
                assert!(cell.x + GLYPH_WIDTH * SCALE <= SCREEN_WIDTH);
                assert!(cell.y + GLYPH_HEIGHT * SCALE <= SCREEN_HEIGHT);
                assert!(cell.rows.iter().any(|row| *row != 0));
            }
            assert!(layout.cell(layout.len()).is_none());
        }
    }

    #[test]
    fn missing_glyph_is_classified_before_any_drawing() {
        assert_eq!(
            TextLayout::new("きょう🐈"),
            Err(TextError::MissingGlyph('🐈'))
        );
        assert_eq!(TextError::MissingGlyph('🐈').code(), ErrorCode::OutOfRange);
    }

    #[test]
    fn expanded_table_is_sorted_and_reaches_beyond_u8_indices() {
        assert!(glyphs::GLYPHS.len() > usize::from(u8::MAX));
        assert!(glyphs::GLYPHS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (character, _) in [glyphs::GLYPHS[256], *glyphs::GLYPHS.last().unwrap()] {
            let layout = TextLayout::new(&character.to_string()).unwrap();
            assert_eq!(
                layout.cell(0).unwrap().rows,
                glyphs::GLYPHS[glyphs::GLYPHS
                    .binary_search_by_key(&character, |(c, _)| *c)
                    .unwrap()]
                .1
            );
        }
    }

    #[test]
    fn kana_kanji_punctuation_and_ascii_are_displayable() {
        let layout = TextLayout::new("猫、こんにちは！").unwrap();
        assert_eq!(layout.len(), 8);
        assert!(TextLayout::new("Cat 123").is_ok());
    }

    #[test]
    fn one_line_limit_rejects_overflow_without_truncation() {
        assert!(TextLayout::new(&"い".repeat(MAX_TEXT_CHARS)).is_ok());
        assert_eq!(
            TextLayout::new(&"い".repeat(MAX_TEXT_CHARS + 1)),
            Err(TextError::TooLong)
        );
        assert_eq!(TextLayout::new(""), Err(TextError::Empty));
    }

    #[test]
    fn expression_changes_during_text_are_restored_at_expiry() {
        let mut state = TextPresentation::new();
        assert_eq!(
            state.show_expression(ExpressionName::Happy),
            Some(ExpressionName::Happy)
        );
        assert_eq!(state.show_text(500, 100), Ok(()));
        assert_eq!(state.show_expression(ExpressionName::Surprised), None);
        assert_eq!(state.expire(599), None);
        assert_eq!(state.expire(600), Some(ExpressionName::Surprised));
        assert_eq!(state.expire(601), None);
        assert_eq!(
            state.show_expression(ExpressionName::Neutral),
            Some(ExpressionName::Neutral)
        );
    }

    #[test]
    fn duration_is_bounded_and_new_text_replaces_old_deadline() {
        let mut state = TextPresentation::new();
        assert_eq!(state.show_text(0, 0), Err(TextError::Duration));
        assert_eq!(
            state.show_text(MAX_TEXT_DURATION_MS + 1, 0),
            Err(TextError::Duration)
        );
        state.show_text(MAX_TEXT_DURATION_MS, 100).unwrap();
        state.show_text(10, 200).unwrap();
        assert_eq!(state.deadline_ms(), Some(210));
        assert_eq!(state.expire(210), Some(ExpressionName::Neutral));
    }
}
