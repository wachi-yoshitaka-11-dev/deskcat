//! 起動からの期限（watchdog）。
//!
//! `ESP32`のsessionの確立（`boot`の受理）と、`hello`の結果がそろわないまま期限を過ぎたら、
//! 外から見える形で終わらせる。`UnapprovedHelloAck`で`hello`の拒否が届かず、`boot`も再送されない場合
//! （`daemon`のmodule docの「起動直後の`hello`の拒否は届かない」）を閉じる。
//!
//! **時計を持たない。**呼び出し側が起動からの経過msを渡す（`Daemon::step`と同じ）。
//! **値を持たない。**期限は引数で与えられる（`cli`）。
//!
//! **解除は1回だけで、戻さない。**完了した後に再開などで`hello`が再び未解決になっても、判定しない。
//! 再開の回数は`RESUME_LIMIT`などが既に有限にしており、`daemon`のmodule docが「再開に期限は持たない」と
//! 定めている。
//!
//! 期限を過ぎたことは[`crate::daemon::ExitReason`]に足さない。終了コードは
//! [`crate::daemon::EXIT_SESSION_REDO`]（75）で、[`crate::runner::RunEnd::StartupDeadline`]が持つ。

/// 起動からの期限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartupWatchdog {
    deadline_ms: u64,
    released: bool,
}

impl StartupWatchdog {
    /// `deadline_ms`は起動からの期限。0は期限を過ぎたものとして扱う（`cli`は0を受け付けない）。
    #[must_use]
    pub const fn new(deadline_ms: u64) -> Self {
        Self {
            deadline_ms,
            released: false,
        }
    }

    /// 完了したか、期限を過ぎたかを調べる。期限を過ぎて未完了なら`true`。
    ///
    /// **完了を先に見る。**期限を過ぎた周で完了していれば、解除する。
    pub fn expired(&mut self, now_ms: u64, complete: bool) -> bool {
        if self.released {
            return false;
        }
        if complete {
            self.released = true;
            return false;
        }
        now_ms >= self.deadline_ms
    }

    /// 解除済みか。
    #[must_use]
    pub const fn is_released(&self) -> bool {
        self.released
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_before_the_deadline_continues() {
        let mut dog = StartupWatchdog::new(1000);
        assert!(!dog.expired(0, false));
        assert!(!dog.expired(999, false));
    }

    #[test]
    fn incomplete_at_or_after_the_deadline_expires() {
        let mut dog = StartupWatchdog::new(1000);
        assert!(dog.expired(1000, false));
        assert!(dog.expired(5000, false));
    }

    #[test]
    fn complete_releases_even_past_the_deadline() {
        let mut dog = StartupWatchdog::new(1000);
        assert!(!dog.expired(2000, true));
        assert!(dog.is_released());
    }

    #[test]
    fn a_release_is_not_undone_when_completion_is_lost() {
        let mut dog = StartupWatchdog::new(1000);
        assert!(!dog.expired(10, true));
        assert!(!dog.expired(10_000, false));
    }

    #[test]
    fn zero_deadline_expires_immediately() {
        let mut dog = StartupWatchdog::new(0);
        assert!(dog.expired(0, false));
    }
}
