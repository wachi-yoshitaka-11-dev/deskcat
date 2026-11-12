//! linkを回す loop。時計と transport を注入できる形にしてある（実体は`main`が渡す）。
//!
//! `Daemon::step`の`Step::Exit`を**捨てない**。起動の期限（[`StartupWatchdog`]）を過ぎたときは、
//! [`RunEnd::StartupDeadline`]で返す。`ExitReason`には足さない（期限の判断は呼び出し側のものである）。

use std::io;
use std::num::NonZeroU64;
use std::time::Instant;

use deskcat_domain::{LocalSpeech, Millis, SpeechCounters};
use deskcat_serial::Transport;

use crate::cloud_speech::{CloudSpeech, Context, Failure};
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
    clock: C,
    watchdog: StartupWatchdog,
    speech_config: Option<SpeechConfig>,
) -> RunEnd
where
    T: Transport,
    C: FnMut() -> u64,
{
    run_loop_with_cloud(daemon, transport, clock, watchdog, speech_config, None)
}

/// クラウドは別workerで試し、結果が届くまでUARTの`step`を回す。
pub fn run_loop_with_cloud<T, C>(
    daemon: &mut Daemon,
    transport: &mut T,
    mut clock: C,
    mut watchdog: StartupWatchdog,
    speech_config: Option<SpeechConfig>,
    mut cloud: Option<CloudSpeech>,
) -> RunEnd
where
    T: Transport,
    C: FnMut() -> u64,
{
    let mut speech =
        speech_config.map(|config| LocalSpeech::new(config.min_interval_ms, Millis(0)));
    let mut fallback: Option<&'static str> = None;
    let mut ready_text: Option<String> = None;
    let origin = Instant::now();
    let mut waits = LinkWait::new(0);
    loop {
        let now_ms = clock();
        if let Some(worker) = cloud.as_mut()
            && let Some(result) = worker.poll()
        {
            match result {
                Ok(line) => ready_text = Some(line),
                Err(kind) => {
                    log::warn!("cloud_speech: {kind:?}; local fallback");
                    ready_text = fallback.map(str::to_owned);
                }
            }
            fallback = None;
        }
        if daemon.companion_asleep() {
            // 寝る直前に頼んだ独り言を、朝まで持ち越さない。
            ready_text = None;
        }
        if daemon.speech_ready()
            && let (Some(line), Some(config)) = (ready_text.take(), speech_config)
        {
            daemon.queue_speech(line, config.duration_ms, config.min_interval_ms);
        }
        if let (Some(speech), Some(config)) = (&mut speech, speech_config)
            && daemon.speech_ready()
            && ready_text.is_none()
            && fallback.is_none()
            && !cloud.as_ref().is_some_and(CloudSpeech::in_flight)
            && let Some(line) = speech.next_line(Millis(now_ms))
        {
            if let Some(worker) = cloud.as_mut() {
                let context = Context {
                    expression: daemon.speech_expression(),
                    time_band: crate::cloud_speech::time_band(std::time::SystemTime::now()),
                };
                if let Err(kind) = worker.submit(context, config.duration_ms) {
                    log::warn!("cloud_speech: {kind:?}; local fallback");
                    worker.note_failure(kind);
                    daemon.queue_speech(line, config.duration_ms, config.min_interval_ms);
                } else {
                    fallback = Some(line);
                }
            } else {
                daemon.queue_speech(line, config.duration_ms, config.min_interval_ms);
            }
        }
        daemon.set_local_minutes(crate::cloud_speech::local_minutes(
            std::time::SystemTime::now(),
        ));
        let step_started = micros_since(origin);
        let step = daemon.step(
            &mut Timed {
                inner: &mut *transport,
                waits: &mut waits,
                now_us: || micros_since(origin),
            },
            now_ms,
        );
        waits
            .step
            .note(micros_since(origin).saturating_sub(step_started));
        if let Some(line) = waits.take_if_due(now_ms) {
            log::info!("{line}");
        }
        if let Step::Exit(reason) = step {
            log::info!("{}", waits.finish(now_ms));
            report_speech_counters(speech.as_ref());
            report_cloud_counters(cloud.as_ref());
            return RunEnd::Exited(reason);
        }
        if watchdog.expired(now_ms, daemon.startup_complete()) {
            log::error!("起動の期限を過ぎた。sessionの確立と`hello`の結果がそろわない");
            log::info!("{}", waits.finish(now_ms));
            report_speech_counters(speech.as_ref());
            report_cloud_counters(cloud.as_ref());
            return RunEnd::StartupDeadline;
        }
    }
}

fn report_cloud_counters(cloud: Option<&CloudSpeech>) {
    if let Some(worker) = cloud {
        let counts = worker.counters();
        log::info!(
            "cloud_speech: success={} credential={} budget={} budget_io={} timeout={} http={} network={} input_tokens={} response={} display={} worker={}",
            counts.success,
            counts.count(Failure::Credential),
            counts.count(Failure::Budget),
            counts.count(Failure::BudgetIo),
            counts.count(Failure::Timeout),
            counts.count(Failure::Http),
            counts.count(Failure::Network),
            counts.count(Failure::InputTokens),
            counts.count(Failure::Response),
            counts.count(Failure::Display),
            counts.count(Failure::Worker)
        );
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

/// `link_wait`の窓の長さ（ms）。観測の粒度であり、安全値ではない。ESP32の
/// `firmware/esp32/src/config.rs`の`HEALTH_SNAPSHOT_PERIOD_MS`に揃えた。
pub const LINK_WAIT_WINDOW_MS: u64 = 10_000;

fn micros_since(origin: Instant) -> u64 {
    u64::try_from(origin.elapsed().as_micros()).unwrap_or(u64::MAX)
}

#[derive(Debug, Default, Clone, Copy)]
struct Max {
    n: u64,
    max_us: u64,
}

impl Max {
    fn note(&mut self, us: u64) {
        self.n += 1;
        self.max_us = self.max_us.max(us);
    }
}

#[derive(Debug)]
struct LinkWait {
    window_start_ms: u64,
    step: Max,
    read_idle: Max,
    read_data: Max,
    write: Max,
    flush: Max,
}

impl LinkWait {
    fn new(start_ms: u64) -> Self {
        Self {
            window_start_ms: start_ms,
            step: Max::default(),
            read_idle: Max::default(),
            read_data: Max::default(),
            write: Max::default(),
            flush: Max::default(),
        }
    }

    fn take_if_due(&mut self, now_ms: u64) -> Option<String> {
        (now_ms.saturating_sub(self.window_start_ms) >= LINK_WAIT_WINDOW_MS)
            .then(|| self.finish(now_ms))
    }

    fn finish(&mut self, now_ms: u64) -> String {
        let line = format!(
            "link_wait window_ms={} step_n={} step_max_us={} read_idle_n={} read_idle_max_us={} read_data_n={} read_data_max_us={} write_n={} write_max_us={} flush_n={} flush_max_us={}",
            now_ms.saturating_sub(self.window_start_ms),
            self.step.n,
            self.step.max_us,
            self.read_idle.n,
            self.read_idle.max_us,
            self.read_data.n,
            self.read_data.max_us,
            self.write.n,
            self.write.max_us,
            self.flush.n,
            self.flush.max_us,
        );
        *self = Self::new(now_ms);
        line
    }
}

/// `Transport`の戻り値を変えずに、呼び出しの時間を`LinkWait`へ記録する。
struct Timed<'a, T, U> {
    inner: &'a mut T,
    waits: &'a mut LinkWait,
    now_us: U,
}

impl<T: Transport, U: FnMut() -> u64> Transport for Timed<'_, T, U> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let started = (self.now_us)();
        let result = self.inner.read(buf);
        let us = (self.now_us)().saturating_sub(started);
        match &result {
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                self.waits.read_idle.note(us);
            }
            _ => self.waits.read_data.note(us),
        }
        result
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let started = (self.now_us)();
        let result = self.inner.write(buf);
        self.waits
            .write
            .note((self.now_us)().saturating_sub(started));
        result
    }

    fn flush(&mut self) -> io::Result<()> {
        let started = (self.now_us)();
        let result = self.inner.flush();
        self.waits
            .flush
            .note((self.now_us)().saturating_sub(started));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Script {
        read: Vec<io::Result<usize>>,
    }

    impl Transport for Script {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            self.read.remove(0)
        }
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len() - 1)
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "x"))
        }
    }

    /// 呼び出しごとに`by`だけ進む時計。1回の測定は2回読むので、差は`by`になる。
    fn ticking(by: u64) -> impl FnMut() -> u64 {
        let mut t = 0;
        move || {
            t += by;
            t
        }
    }

    #[test]
    fn results_pass_through_and_each_call_is_recorded() {
        let mut inner = Script {
            read: vec![
                Err(io::Error::new(io::ErrorKind::WouldBlock, "idle")),
                Ok(3),
                Ok(0),
            ],
        };
        let mut waits = LinkWait::new(0);
        let mut t = Timed {
            inner: &mut inner,
            waits: &mut waits,
            now_us: ticking(7),
        };
        let mut buf = [0_u8; 4];
        assert_eq!(
            t.read(&mut buf).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(t.read(&mut buf).unwrap(), 3);
        assert_eq!(t.read(&mut buf).unwrap(), 0);
        assert_eq!(t.write(&[1, 2]).unwrap(), 1);
        assert_eq!(t.flush().unwrap_err().kind(), io::ErrorKind::BrokenPipe);
        assert_eq!((waits.read_idle.n, waits.read_idle.max_us), (1, 7));
        assert_eq!((waits.read_data.n, waits.read_data.max_us), (2, 7));
        assert_eq!((waits.write.n, waits.write.max_us), (1, 7));
        assert_eq!((waits.flush.n, waits.flush.max_us), (1, 7));
    }

    #[test]
    fn max_keeps_the_largest_and_the_window_resets_when_reported() {
        let mut waits = LinkWait::new(100);
        waits.step.note(5);
        waits.step.note(9);
        waits.step.note(2);
        assert_eq!((waits.step.n, waits.step.max_us), (3, 9));
        assert!(waits.take_if_due(100 + LINK_WAIT_WINDOW_MS - 1).is_none());
        let line = waits.take_if_due(100 + LINK_WAIT_WINDOW_MS).unwrap();
        assert!(line.starts_with("link_wait window_ms=10000 step_n=3 step_max_us=9 "));
        assert_eq!((waits.step.n, waits.step.max_us), (0, 0));
        assert_eq!(waits.window_start_ms, 100 + LINK_WAIT_WINDOW_MS);
    }
}
