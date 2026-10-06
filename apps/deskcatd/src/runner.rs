//! linkを回す loop。時計と transport を注入できる形にしてある（実体は`main`が渡す）。
//!
//! `Daemon::step`の`Step::Exit`を**捨てない**。起動の期限（[`StartupWatchdog`]）を過ぎたときは、
//! [`RunEnd::StartupDeadline`]で返す。`ExitReason`には足さない（期限の判断は呼び出し側のものである）。

use deskcat_serial::Transport;

use crate::daemon::{Daemon, EXIT_SESSION_REDO, ExitReason, Step};
use crate::watchdog::StartupWatchdog;

/// loopの終わり方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunEnd {
    /// `Daemon`が終了を求めた。
    Exited(ExitReason),
    /// 起動の期限を過ぎても、sessionの確立と`hello`の結果がそろわなかった。
    StartupDeadline,
}

impl RunEnd {
    /// 終了コード。期限の超過は75（sessionをやり直すための再起動）。
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Exited(reason) => reason.exit_code(),
            Self::StartupDeadline => EXIT_SESSION_REDO,
        }
    }
}

/// 終了するまで`step`を回す。`clock`は起動からの経過ms（単調）を返す。
///
/// 各周で、先に`step`の結果を見て、その後に期限を調べる。`Daemon`の終了が期限より優先される。
pub fn run_loop<T, C>(
    daemon: &mut Daemon,
    transport: &mut T,
    mut clock: C,
    mut watchdog: StartupWatchdog,
) -> RunEnd
where
    T: Transport,
    C: FnMut() -> u64,
{
    loop {
        let now_ms = clock();
        if let Step::Exit(reason) = daemon.step(transport, now_ms) {
            return RunEnd::Exited(reason);
        }
        if watchdog.expired(now_ms, daemon.startup_complete()) {
            log::error!("起動の期限を過ぎた。sessionの確立と`hello`の結果がそろわない");
            return RunEnd::StartupDeadline;
        }
    }
}
