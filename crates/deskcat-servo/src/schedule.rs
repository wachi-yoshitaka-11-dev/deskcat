//! `tick`を制御周期で刻む時刻の判定（[Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)）。
//!
//! [`TickSchedule`]は「今`tick`を呼ぶ時刻か」と「次の期限」だけを答える。**値を持たない**
//! （周期は呼び出し側が渡す。製品buildの周期は`TBD`で、値が無い間は作らない）。
//! blockingも割り込みも使わず、main loopが次の期限まで待つ（UARTのread timeout）ために期限を返す。
//!
//! 期限の積み方は、1周期未満の遅れなら期限の列（前の期限 + period）へ揃え、1周期以上遅れたときだけ`now + period`へ積み直す
//! （取り戻すためにburstしない。積み直した回数は[`TickSchedule::overruns`]）。
//! 遅れた`tick`の間隔が制御周期の外になれば、`LimitedServo`が間隔を検査してlatchし、`stop`する（安全側）。

/// `tick`の期限の管理。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickSchedule {
    period_ms: u64,
    next_ms: u64,
    overruns: u32,
}

impl TickSchedule {
    /// `now_ms`を起点に作る。最初の期限は`now_ms + period_ms`。`period_ms`が0なら`None`。
    #[must_use]
    pub const fn new(period_ms: u32, now_ms: u64) -> Option<Self> {
        if period_ms == 0 {
            return None;
        }
        let period_ms = period_ms as u64;
        Some(Self {
            period_ms,
            next_ms: now_ms.saturating_add(period_ms),
            overruns: 0,
        })
    }

    /// 次の期限（単調に増える時刻、ms）。
    #[must_use]
    pub const fn next_deadline_ms(&self) -> u64 {
        self.next_ms
    }

    /// 期限に遅れた回数（次の期限が`now`以前になり、`now + period`へ積み直した回数）。
    #[must_use]
    pub const fn overruns(&self) -> u32 {
        self.overruns
    }

    /// `now_ms`が期限以後なら`true`を返し、次の期限を積む。まだなら`false`で何も変えない。
    pub fn due(&mut self, now_ms: u64) -> bool {
        if now_ms < self.next_ms {
            return false;
        }
        let next = self.next_ms.saturating_add(self.period_ms);
        if next > now_ms {
            self.next_ms = next;
        } else {
            self.next_ms = now_ms.saturating_add(self.period_ms);
            self.overruns = self.overruns.saturating_add(1);
        }
        true
    }
}
