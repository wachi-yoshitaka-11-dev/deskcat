//! 製品buildで、Piの`play_motion`とdebugの指示、`tick`／`stop`、`status`の`servo.limiter`が通る経路
//! （[Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)）。
//!
//! **値を持たない。**制限・制御周期・初期位置・name→targetの表は、HW-TBD-010／011／019／020と
//! PROTO-TBD-008が決まるまで`None`であり、[`product`]は`None`を返す（`LimitedServo`を作らない。
//! `Sg90`も、LEDC timer・channel・GPIO27も取らない）。**この経路の中で、サーボ出力を有効にする
//! 条件（`servo-safety-limits.md`の`サーボ出力を有効化してよい条件`）を変えない。**
//!
//! `stop`の後は[`ServoPath`]を`Stopped`のままにし、`resume`しない（再開は人の承認の後の別の手順）。

// 製品buildは`product()`が`None`を返すため、`ServoPath`の中身は呼ばれない。
#![allow(dead_code)]

use deskcat_servo::{
    ControlPeriod, Drive, LimiterCounters, MotionRequest, MotionSink, Rejection, Setpoint,
};
use esp_idf_svc::sys::EspError;

use crate::servo::{LimitedServo, Stopped, TickError};

/// 製品buildの制御周期の公称（ms）。**firmwareの制御周期であり、servoの実測値ではない。TBD。**
pub const CONTROL_PERIOD_NOMINAL_MS: Option<u32> = None;
/// 製品buildの制御周期の許容幅（ms）。TBD。
pub const CONTROL_PERIOD_TOLERANCE_MS: Option<u32> = None;

/// 製品buildの`ControlPeriod`。値が無い間は`None`。
#[must_use]
pub fn control_period() -> Option<ControlPeriod> {
    let nominal = CONTROL_PERIOD_NOMINAL_MS?;
    let tolerance = CONTROL_PERIOD_TOLERANCE_MS?;
    ControlPeriod::new(nominal as f32 / 1000.0, tolerance as f32 / 1000.0).ok()
}

/// Piの`play_motion`の名前からtargetを引く表。**表は無い**（PROTO-TBD-008とHW-TBD-010の後）。
#[must_use]
pub fn resolve_target(_name: &str) -> Option<f32> {
    None
}

/// 製品buildが持つ`ServoPath`。**値が無い間は`None`（`LimitedServo`を作らない）。**
#[must_use]
pub fn product() -> Option<ServoPath<'static>> {
    None
}

enum State<'d> {
    Running(LimitedServo<'d>),
    Stopped(Stopped<'d>),
    /// `stop`の途中の一時的な状態。
    Taken,
}

/// `LimitedServo`を所有し、`MotionSink`と`Drive`を実装する。
pub struct ServoPath<'d> {
    state: State<'d>,
    /// `stop`の直前の`Limiter`の累計（`Limiter`は`stop`で捨てるため）。
    counters_at_stop: LimiterCounters,
}

impl<'d> ServoPath<'d> {
    /// 作った`LimitedServo`を渡して作る。
    #[must_use]
    pub fn new(servo: LimitedServo<'d>) -> Self {
        Self {
            state: State::Running(servo),
            counters_at_stop: LimiterCounters::default(),
        }
    }

    /// `stop`の後か。
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        !matches!(self.state, State::Running(_))
    }
}

impl MotionSink for ServoPath<'_> {
    fn admit(&mut self, request: &MotionRequest<'_>) -> Result<(), Rejection> {
        match &mut self.state {
            State::Running(servo) => servo.admit(request),
            // `stop`の後は新しい動作を始めない。
            State::Stopped(_) | State::Taken => Err(Rejection::Busy),
        }
    }

    fn counters(&self) -> LimiterCounters {
        match &self.state {
            State::Running(servo) => servo.counters(),
            State::Stopped(_) | State::Taken => self.counters_at_stop,
        }
    }
}

impl Drive for ServoPath<'_> {
    type TickError = TickError;
    type StopError = EspError;

    fn tick(&mut self) -> Result<Option<Setpoint>, TickError> {
        match &mut self.state {
            State::Running(servo) => servo.tick(),
            State::Stopped(_) | State::Taken => Ok(None),
        }
    }

    fn stop(&mut self) -> Result<(), EspError> {
        match core::mem::replace(&mut self.state, State::Taken) {
            State::Running(servo) => {
                self.counters_at_stop = servo.counters();
                let (stopped, result) = servo.stop();
                self.state = State::Stopped(stopped);
                result
            }
            other => {
                self.state = other;
                Ok(())
            }
        }
    }
}
