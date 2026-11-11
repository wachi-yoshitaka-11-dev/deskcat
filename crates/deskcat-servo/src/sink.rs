//! Piの`play_motion`とdebugの指示が同じ入口（[`MotionSink`]）へ届く共通の関数と、
//! `tick`のエラーで必ず`stop`を呼ぶ関数（[Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)）。
//!
//! firmwareの`LimitedServo`と、host testの[`LimiterSink`]が[`MotionSink`]を実装する。
//! **このmoduleは値を持たない。**name→targetの表は呼び出し側が渡す（表の値はPROTO-TBD-008と
//! HW-TBD-010の後）。sinkが無い（`None`）ときは、catalogが空でないbuildでも`UnknownMotion`で拒否する。

use deskcat_protocol::PlayMotion;

use crate::{
    AdmittedTarget, CommandSource, DebugRefusal, Limiter, LimiterCounters, MotionDecision,
    MotionDispatch, MotionRequest, MotionSession, Rejection, Setpoint,
};

/// motion commandを`Limiter::admit`へ通す入口。Piとdebugは同じ実装へ届く。
pub trait MotionSink {
    /// `Limiter::admit`の結果をそのまま返す。出力はしない。
    ///
    /// # Errors
    ///
    /// `Limiter::admit`のreject、またはsinkが新しいcommandを受けられない状態でのreject。
    fn admit(&mut self, request: &MotionRequest<'_>) -> Result<(), Rejection>;

    /// 累計counter（`status`の`servo.limiter`へ渡す）。
    fn counters(&self) -> LimiterCounters;
}

/// host test用の[`MotionSink`]。`Limiter`と受理したtargetを持つ。
#[derive(Debug)]
pub struct LimiterSink {
    limiter: Limiter,
    target: Option<AdmittedTarget>,
}

impl LimiterSink {
    /// `Limiter`を所有して作る。
    #[must_use]
    pub const fn new(limiter: Limiter) -> Self {
        Self {
            limiter,
            target: None,
        }
    }

    /// 最後に受理したtarget。
    #[must_use]
    pub const fn target(&self) -> Option<AdmittedTarget> {
        self.target
    }
}

impl MotionSink for LimiterSink {
    fn admit(&mut self, request: &MotionRequest<'_>) -> Result<(), Rejection> {
        self.target = Some(self.limiter.admit(request)?);
        Ok(())
    }

    fn counters(&self) -> LimiterCounters {
        self.limiter.counters()
    }
}

/// Piから受けた`play_motion`1件（envelopeの`sid`・`id`と、受信時刻）。
#[derive(Debug, Clone, Copy)]
pub struct PiMotion<'a> {
    /// envelopeの`sid`。
    pub sid: u32,
    /// envelopeの`id`。
    pub id: u32,
    /// payload。
    pub command: &'a PlayMotion,
    /// 単調に増える受信時刻（ms）。
    pub now_ms: u64,
}

/// Piの`play_motion`を`MotionDispatch::handle`へ通し、受理したものを`sink`へ渡す。
///
/// `resolve`は名前からtargetを引く表である（`PlayMotion`はtargetを持たない）。引けなければ
/// `UnknownMotion`。`sink`が`None`でも`UnknownMotion`で拒否する（動かさない）。
pub fn admit_pi(
    dispatch: &mut MotionDispatch,
    session: MotionSession,
    motion: PiMotion<'_>,
    resolve: impl FnOnce(&str) -> Option<f32>,
    sink: Option<&mut dyn MotionSink>,
) -> MotionDecision {
    let PiMotion {
        sid,
        id,
        command,
        now_ms,
    } = motion;
    dispatch.handle(session, sid, id, command, now_ms, |command| {
        let (Some(sink), Some(target)) = (sink, resolve(&command.name)) else {
            return Err(Rejection::UnknownMotion);
        };
        sink.admit(&MotionRequest {
            name: &command.name,
            target,
            source: CommandSource::Pi,
        })
    })
}

/// debugの指示を`MotionDispatch::handle_debug`へ通し、受理したものを`sink`へ渡す。
/// `sink`が`None`なら`UnknownMotion`で拒否する。
///
/// # Errors
///
/// `handle_debug`の拒否。
pub fn admit_debug(
    dispatch: &MotionDispatch,
    boot_established: bool,
    name: &str,
    target: f32,
    sink: Option<&mut dyn MotionSink>,
) -> Result<(), DebugRefusal> {
    dispatch.handle_debug(boot_established, name, target, |request| {
        sink.ok_or(Rejection::UnknownMotion)?.admit(request)
    })
}

/// [`drive_tick`]が駆動する対象。
pub trait Drive {
    /// `tick`の失敗。
    type TickError;
    /// `stop`の失敗。
    type StopError;

    /// 1 tick進める。`Some`は出力した位置。
    ///
    /// # Errors
    ///
    /// 出力できず、以後も出さない失敗。
    fn tick(&mut self) -> Result<Option<Setpoint>, Self::TickError>;

    /// 出力を止める。
    ///
    /// # Errors
    ///
    /// 止められなかった失敗。
    fn stop(&mut self) -> Result<(), Self::StopError>;
}

/// [`drive_tick`]の結果。
#[derive(Debug)]
pub enum DriveOutcome<T, S> {
    /// 出力しなかった（targetが無い、または間引いた）。
    Idle,
    /// 出力した。
    Output(Setpoint),
    /// `tick`が失敗したので`stop`を呼んだ。**`stop`の失敗は握りつぶさずここに残す。**
    Stopped {
        /// `tick`の失敗。
        tick_error: T,
        /// `stop`の結果。
        stop: Result<(), S>,
    },
}

/// `tick`を1回呼び、失敗したら**必ず**`stop`を呼ぶ。
pub fn drive_tick<D: Drive>(drive: &mut D) -> DriveOutcome<D::TickError, D::StopError> {
    match drive.tick() {
        Ok(None) => DriveOutcome::Idle,
        Ok(Some(setpoint)) => DriveOutcome::Output(setpoint),
        Err(tick_error) => DriveOutcome::Stopped {
            tick_error,
            stop: drive.stop(),
        },
    }
}

/// `stop`の結果を握りつぶさず記録する。**失敗している間は「止めた」と言わない。**
///
/// `stop`が失敗した（duty を0へ戻せなかった）後に、次の`stop`を無条件の成功にすると、
/// 出力が残ったまま「止めた」と読める。呼び出し側は、[`StopLatch::is_pending`]の間は`stop`を
/// 成功するまで試し直し、結果を[`StopLatch::record`]へ渡す。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StopLatch {
    pending: bool,
    failures: u32,
}

impl StopLatch {
    /// 失敗の記録が無い状態で作る。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending: false,
            failures: 0,
        }
    }

    /// `stop`1回の結果を記録する。失敗は数え、成功は`pending`を下ろす。
    pub fn record<E>(&mut self, result: &Result<(), E>) {
        if result.is_err() {
            self.pending = true;
            self.failures = self.failures.saturating_add(1);
        } else {
            self.pending = false;
        }
    }

    /// 直近の`stop`が失敗したままか（試し直しが要る）。
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        self.pending
    }

    /// `stop`の失敗の累計。
    #[must_use]
    pub const fn failures(&self) -> u32 {
        self.failures
    }
}
