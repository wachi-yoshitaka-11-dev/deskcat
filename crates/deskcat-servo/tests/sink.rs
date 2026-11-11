//! `admit_pi`・`admit_debug`・`drive_tick`の振る舞い（Issue #19）。
//!
//! `TEST_`の値とmotion名はこのtestだけの入力であり、`servo-safety-limits.md`の`動作制限`表の
//! `承認値`でもhard boundでもない。このfileの値を正本へ写さない。

use core::time::Duration;

use deskcat_protocol::{DuplicatePolicy, ErrorCode, Frame, Message, PlayMotion, decode_line};
use deskcat_servo::{
    Cap, ControlPeriod, DebugRefusal, Drive, DriveOutcome, Limiter, LimiterSink, MotionCatalog,
    MotionDecision, MotionDispatch, MotionSession, MotionSink, PiMotion, PositionRange, Rejection,
    ServoLimits, admit_debug, admit_pi, drive_tick,
};

/// **test用の名前。**
const TEST_MOTION: &str = "test-motion";
/// **test用の値。**承認値とhard boundの間（clampして報告する側）。
const TEST_BETWEEN: f32 = 30.0;
/// **test用の値。**hard boundの外（rejectする側）。
const TEST_BEYOND: f32 = 100.0;
/// **test用の値。**承認値の内側。
const TEST_INSIDE: f32 = 5.0;

fn test_sink() -> LimiterSink {
    let limits = ServoLimits::new(
        PositionRange::new(-40.0, -30.0, 25.0, 35.0).expect("test range is ordered"),
        0.0,
        Cap::new(20.0, 25.0, "max_velocity").expect("test velocity cap"),
        Cap::new(40.0, 50.0, "max_acceleration").expect("test acceleration cap"),
        Cap::new(8.0, 12.0, "max_step").expect("test step cap"),
    )
    .expect("test limits are consistent");
    LimiterSink::new(
        Limiter::new(
            limits,
            ControlPeriod::new(0.02, 0.002).expect("test period"),
            MotionCatalog::new([TEST_MOTION]),
            0.0,
        )
        .expect("test limiter"),
    )
}

fn test_dispatch() -> MotionDispatch {
    let policy = DuplicatePolicy::new(4, Duration::from_secs(1)).expect("test policy");
    MotionDispatch::new(policy, MotionCatalog::new([TEST_MOTION]))
}

fn play(id: u32, name: &str) -> PlayMotion {
    let line = format!(
        r#"{{"v":1,"sid":10,"id":{id},"ts_ms":0,"type":"play_motion","payload":{{"name":"{name}","speed":0.5,"repeat":1}}}}"#
    );
    let Frame { message, .. } = decode_line(&line).expect("valid test wire line");
    let Message::PlayMotion(command) = message else {
        panic!("test line must be play_motion");
    };
    command
}

fn pi(
    dispatch: &mut MotionDispatch,
    sink: Option<&mut dyn MotionSink>,
    boot: bool,
    id: u32,
    name: &str,
    target: Option<f32>,
) -> MotionDecision {
    admit_pi(
        dispatch,
        MotionSession::new(Some(10), boot),
        PiMotion {
            sid: 10,
            id,
            command: &play(id, name),
            now_ms: 0,
        },
        |_| target,
        sink,
    )
}

#[test]
fn pi_and_debug_reach_the_same_sink_and_the_same_counters() {
    let mut dispatch = test_dispatch();
    let mut sink = test_sink();

    let decision = pi(
        &mut dispatch,
        Some(&mut sink),
        true,
        1,
        TEST_MOTION,
        Some(TEST_BETWEEN),
    );
    assert_eq!(decision.code(), None);
    assert_eq!(sink.counters().clamped_position, 1);

    admit_debug(&dispatch, true, TEST_MOTION, TEST_BETWEEN, Some(&mut sink)).expect("admitted");
    assert_eq!(sink.counters().clamped_position, 2);

    admit_debug(&dispatch, true, TEST_MOTION, TEST_INSIDE, Some(&mut sink)).expect("admitted");
    assert_eq!(sink.counters().clamped_position, 2);
    let held = sink.target().expect("target").target();
    assert!((held - TEST_INSIDE).abs() < f32::EPSILON);
}

#[test]
fn rejections_count_the_same_way_from_pi_and_debug() {
    let mut dispatch = test_dispatch();
    let mut sink = test_sink();

    let decision = pi(
        &mut dispatch,
        Some(&mut sink),
        true,
        1,
        TEST_MOTION,
        Some(TEST_BEYOND),
    );
    assert_eq!(decision.code(), Some(ErrorCode::OutOfRange));
    let after_pi = sink.counters();
    assert_eq!(
        admit_debug(&dispatch, true, TEST_MOTION, TEST_BEYOND, Some(&mut sink)),
        Err(DebugRefusal::Rejected(Rejection::TargetOutOfHardRange))
    );
    assert_ne!(sink.counters(), after_pi, "the debug reject is counted too");
}

#[test]
fn an_unresolved_name_or_a_missing_sink_is_unknown_motion() {
    let mut dispatch = test_dispatch();
    let mut sink = test_sink();

    let decision = pi(&mut dispatch, Some(&mut sink), true, 1, TEST_MOTION, None);
    assert_eq!(decision.code(), Some(ErrorCode::InvalidPayload));
    assert!(sink.target().is_none());

    let decision = pi(&mut dispatch, None, true, 2, TEST_MOTION, Some(TEST_INSIDE));
    assert_eq!(decision.code(), Some(ErrorCode::InvalidPayload));
    assert_eq!(
        admit_debug(&dispatch, true, TEST_MOTION, TEST_INSIDE, None),
        Err(DebugRefusal::Rejected(Rejection::UnknownMotion))
    );
}

#[test]
fn boot_not_established_never_reaches_the_sink() {
    let mut dispatch = test_dispatch();
    let mut sink = test_sink();

    let decision = pi(
        &mut dispatch,
        Some(&mut sink),
        false,
        1,
        TEST_MOTION,
        Some(TEST_INSIDE),
    );
    assert_eq!(decision.code(), Some(ErrorCode::StaleSession));
    assert_eq!(
        admit_debug(&dispatch, false, TEST_MOTION, TEST_INSIDE, Some(&mut sink)),
        Err(DebugRefusal::BootNotEstablished)
    );
    assert!(sink.target().is_none());
}

#[test]
fn a_motion_name_outside_the_catalog_never_reaches_the_sink() {
    let mut dispatch = test_dispatch();
    let mut sink = test_sink();
    let decision = pi(
        &mut dispatch,
        Some(&mut sink),
        true,
        1,
        "other",
        Some(TEST_INSIDE),
    );
    assert_eq!(decision.code(), Some(ErrorCode::InvalidPayload));
    assert!(sink.target().is_none());
}

/// `tick`と`stop`の呼び出しを数える。
struct FakeDrive {
    ticks: Vec<Result<Option<()>, &'static str>>,
    stop_result: Result<(), &'static str>,
    stops: u32,
}

impl Drive for FakeDrive {
    type TickError = &'static str;
    type StopError = &'static str;

    fn tick(&mut self) -> Result<Option<deskcat_servo::Setpoint>, &'static str> {
        match self.ticks.remove(0) {
            Ok(_) => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn stop(&mut self) -> Result<(), &'static str> {
        self.stops += 1;
        self.stop_result
    }
}

#[test]
fn a_tick_error_always_calls_stop_and_keeps_a_stop_failure() {
    let mut drive = FakeDrive {
        ticks: vec![Ok(None), Err("faulted")],
        stop_result: Err("stop failed"),
        stops: 0,
    };
    assert!(matches!(drive_tick(&mut drive), DriveOutcome::Idle));
    assert_eq!(drive.stops, 0);
    match drive_tick(&mut drive) {
        DriveOutcome::Stopped { tick_error, stop } => {
            assert_eq!(tick_error, "faulted");
            assert_eq!(stop, Err("stop failed"));
        }
        other => panic!("expected Stopped, got {other:?}"),
    }
    assert_eq!(drive.stops, 1);
}
