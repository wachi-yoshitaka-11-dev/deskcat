//! Piからの受信が途切れたことの判定と、途切れたときに`stop`を呼ぶ関数
//! （[Issue #20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20)）。
//!
//! **値を持たない。**途切れたと判定するまでの時間は呼び出し側が渡す（製品buildの値は`TBD`。HW-TBD-017）。
//! `None`の間は、途切れたと判定しない。復帰後の再開はこのmoduleの範囲外である（HW-TBD-018）。

use crate::Drive;

/// [`LinkWatch::check`]の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkVerdict {
    /// 見張っていない（時間が決まっていない、またはまだ1度も受信していない）。
    Idle,
    /// 受信が続いている。
    Alive,
    /// 受信が途切れている。`first`は、この途切れで最初の判定のときだけ`true`。
    Lost {
        /// この途切れで最初の判定か。
        first: bool,
    },
}

/// 受信の途切れの見張り。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkWatch {
    timeout_ms: Option<u32>,
    last_rx_ms: Option<u64>,
    lost: bool,
    losses: u32,
}

impl LinkWatch {
    /// `timeout_ms`が`None`または0の間は、見張らない。
    #[must_use]
    pub const fn new(timeout_ms: Option<u32>) -> Self {
        Self {
            timeout_ms: match timeout_ms {
                Some(0) => None,
                other => other,
            },
            last_rx_ms: None,
            lost: false,
            losses: 0,
        }
    }

    /// 見張る時間が決まっているか。
    #[must_use]
    pub const fn is_armed(&self) -> bool {
        self.timeout_ms.is_some()
    }

    /// Piから受信した時刻を記録する。途切れの状態は解く（stopした出力は戻さない）。
    pub fn note_rx(&mut self, now_ms: u64) {
        self.last_rx_ms = Some(self.last_rx_ms.map_or(now_ms, |last| last.max(now_ms)));
        self.lost = false;
    }

    /// 途切れたと判定する時刻（単調に増える時刻、ms）。見張っていなければ`None`。
    #[must_use]
    pub fn deadline_ms(&self) -> Option<u64> {
        if self.lost {
            return None;
        }
        Some(self.last_rx_ms?.saturating_add(u64::from(self.timeout_ms?)))
    }

    /// 途切れの判定を返す。
    pub fn check(&mut self, now_ms: u64) -> LinkVerdict {
        let (Some(timeout), Some(last)) = (self.timeout_ms, self.last_rx_ms) else {
            return LinkVerdict::Idle;
        };
        if now_ms.saturating_sub(last) < u64::from(timeout) {
            return LinkVerdict::Alive;
        }
        let first = !self.lost;
        if first {
            self.lost = true;
            self.losses = self.losses.saturating_add(1);
        }
        LinkVerdict::Lost { first }
    }

    /// 途切れと判定した累計回数。
    #[must_use]
    pub const fn losses(&self) -> u32 {
        self.losses
    }
}

/// `watch`が途切れを最初に判定したときだけ`drive.stop()`を呼び、その結果を返す。
/// **`stop`の失敗は握りつぶさず返す**（試し直しは呼び出し側の`StopLatch`）。
pub fn fail_safe_on_loss<D: Drive>(
    watch: &mut LinkWatch,
    drive: &mut D,
    now_ms: u64,
) -> Option<Result<(), D::StopError>> {
    match watch.check(now_ms) {
        LinkVerdict::Lost { first: true } => Some(drive.stop()),
        _ => None,
    }
}
