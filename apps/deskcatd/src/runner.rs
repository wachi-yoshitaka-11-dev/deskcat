//! linkを回す loop。時計と transport を注入できる形にしてある（実体は`main`が渡す）。
//!
//! `Daemon::step`の`Step::Exit`を**捨てない**。起動の期限（[`StartupWatchdog`]）を過ぎたときは、
//! [`RunEnd::StartupDeadline`]で返す。`ExitReason`には足さない（期限の判断は呼び出し側のものである）。

use std::num::NonZeroU64;

use deskcat_domain::{LocalSpeech, Millis, SpeechCounters};
use deskcat_serial::Transport;

use crate::daemon::{Daemon, EXIT_SESSION_REDO, ExitReason, Step};
use crate::watchdog::StartupWatchdog;

/// 独り言の運用値。正本に数値が無いため、CLIから両方受け取った場合だけ使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeechConfig {
    /// 文を送る最短間隔。
    pub min_interval_ms: NonZeroU64,
    /// ESP32に要求する表示時間。
    pub duration_ms: u32,
}

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
    clock: C,
    watchdog: StartupWatchdog,
) -> RunEnd
where
    T: Transport,
    C: FnMut() -> u64,
{
    run_loop_with_speech(daemon, transport, clock, watchdog, None)
}

/// 独り言を有効にしたrunner。設定が無ければ送らない。
pub fn run_loop_with_speech<T, C>(
    daemon: &mut Daemon,
    transport: &mut T,
    mut clock: C,
    mut watchdog: StartupWatchdog,
    speech_config: Option<SpeechConfig>,
) -> RunEnd
where
    T: Transport,
    C: FnMut() -> u64,
{
    let mut speech =
        speech_config.map(|config| LocalSpeech::new(config.min_interval_ms, Millis(0)));
    loop {
        let now_ms = clock();
        if let (Some(speech), Some(config)) = (&mut speech, speech_config)
            && daemon.speech_ready()
            && let Some(line) = speech.next_line(Millis(now_ms))
        {
            daemon.queue_speech(line, config.duration_ms, config.min_interval_ms);
        }
        if let Step::Exit(reason) = daemon.step(transport, now_ms) {
            report_speech_counters(speech.as_ref());
            return RunEnd::Exited(reason);
        }
        if watchdog.expired(now_ms, daemon.startup_complete()) {
            log::error!("起動の期限を過ぎた。sessionの確立と`hello`の結果がそろわない");
            report_speech_counters(speech.as_ref());
            return RunEnd::StartupDeadline;
        }
    }
}

fn report_speech_counters(speech: Option<&LocalSpeech>) {
    if let Some(speech) = speech {
        let SpeechCounters {
            emitted,
            rate_limited,
            clock_regressions,
        } = speech.counters();
        log::info!(
            "local_speech: emitted={emitted} rate_limited={rate_limited} clock_regressions={clock_regressions}"
        );
    }
}
