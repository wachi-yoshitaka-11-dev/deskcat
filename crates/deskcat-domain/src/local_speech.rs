//! #23 のローカル独り言候補と頻度制限。
//!
//! 文候補は暫定。LCD の文字と layout は `PROTO-TBD-007` の測定後に確認する。
//! この module は表示時間と wire の上限を決めず、`show_text` への変換も行わない。

use std::num::NonZeroU64;

use crate::Millis;

/// ネット取得なしで使える短い文候補。秘密情報や外部データを含まない。
///
/// 表示可能な文字・行数・layout 処理量は `PROTO-TBD-007` で未確定であり、
/// ここに並ぶことは LCD での表示確認を意味しない。
pub const LOCAL_SPEECH_LINES: &[&str] = &[
    "きょうもいっしょにいよう。",
    "ちょっとひとやすみ。",
    "のんびりしていこう。",
];

/// ローカル独り言の計数。抑制した呼び出しも数える。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpeechCounters {
    /// 候補を返した回数。
    pub emitted: u64,
    /// 最短間隔に達していないため候補を返さなかった回数。
    pub rate_limited: u64,
    /// 前回より前の時刻が渡された回数。
    pub clock_regressions: u64,
}

/// 呼び出し側の単調時刻に対して候補を順番に返す状態。
///
/// 最短間隔に正本の数値はない。`NonZeroU64` で呼び出し側に設定を要求し、
/// この crate には暫定の既定値も置かない。最初の呼び出しは直ちに返し、
/// 以後は最後に返した時刻から最短間隔が経つまで抑制する。
/// 呼び出し側が定期的に [`Self::next_line`] を呼ぶ必要がある。
#[derive(Debug, Clone)]
pub struct LocalSpeech {
    min_interval_ms: NonZeroU64,
    last_emitted: Option<Millis>,
    last_now: Millis,
    next_index: usize,
    counters: SpeechCounters,
}

impl LocalSpeech {
    /// `start` を時刻の起点として作る。`min_interval_ms` の運用値は `TBD`。
    #[must_use]
    pub const fn new(min_interval_ms: NonZeroU64, start: Millis) -> Self {
        Self {
            min_interval_ms,
            last_emitted: None,
            last_now: start,
            next_index: 0,
            counters: SpeechCounters {
                emitted: 0,
                rate_limited: 0,
                clock_regressions: 0,
            },
        }
    }

    /// 候補を返す。最短間隔内または時刻逆行時は `None` を返す。
    ///
    /// 抑えた呼び出しでは候補を進めない。時刻逆行も最後に返した時刻を変えない。
    pub fn next_line(&mut self, now: Millis) -> Option<&'static str> {
        if now < self.last_now {
            self.counters.clock_regressions = self.counters.clock_regressions.saturating_add(1);
            return None;
        }
        self.last_now = now;

        if self
            .last_emitted
            .is_some_and(|last| now.0 - last.0 < self.min_interval_ms.get())
        {
            self.counters.rate_limited = self.counters.rate_limited.saturating_add(1);
            return None;
        }

        let line = LOCAL_SPEECH_LINES[self.next_index];
        self.next_index += 1;
        if self.next_index == LOCAL_SPEECH_LINES.len() {
            self.next_index = 0;
        }
        self.last_emitted = Some(now);
        self.counters.emitted = self.counters.emitted.saturating_add(1);
        Some(line)
    }

    /// 現在の計数。
    #[must_use]
    pub const fn counters(&self) -> SpeechCounters {
        self.counters
    }
}
