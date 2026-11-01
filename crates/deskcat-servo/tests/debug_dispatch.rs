//! debugの指示が、Piの指示と同じcatalog・同じlimiter（同じcounter）を通ること（Issue #19の受け入れ条件3）。
//!
//! `TEST_`の値とmotion名はこのtestだけの入力であり、`servo-safety-limits.md`の`動作制限`表の
//! `承認値`でもhard boundでもない。このfileの値を正本へ写さない。

use core::time::Duration;

use deskcat_protocol::{DuplicatePolicy, Frame, Message, decode_line};
use deskcat_servo::{
    Cap, CommandSource, ControlPeriod, DebugRefusal, DedupOutcome, Limiter, MotionCatalog,
    MotionDecision, MotionDispatch, MotionRequest, MotionSession, PositionRange, Rejection,
    ServoLimits, parse_debug_line,
};

/// **test用の値。**
const TEST_HARD_MIN: f32 = -40.0;
/// **test用の値。**
const TEST_APPROVED_MIN: f32 = -30.0;
/// **test用の値。**
const TEST_APPROVED_MAX: f32 = 25.0;
/// **test用の値。**
const TEST_HARD_MAX: f32 = 35.0;
/// **test用の名前。**Protocol §5.3は受理するmotion名を`TBD`としている。
const TEST_MOTION: &str = "test-motion";
/// **test用の値。**`承認値`とhard boundの間（clampして報告する側）。
const TEST_BETWEEN_APPROVED_AND_HARD: f32 = 30.0;
/// **test用の値。**hard boundの外（rejectする側）。
const TEST_BEYOND_HARD: f32 = 100.0;
/// **test用の値。**`承認値`の内側。
const TEST_INSIDE: f32 = 5.0;

fn test_limiter() -> Limiter {
    let limits = ServoLimits::new(
        PositionRange::new(
            TEST_HARD_MIN,
            TEST_APPROVED_MIN,
            TEST_APPROVED_MAX,
            TEST_HARD_MAX,
        )
        .expect("test range is ordered"),
        0.0,
        Cap::new(20.0, 25.0, "max_velocity").expect("test velocity cap"),
        Cap::new(40.0, 50.0, "max_acceleration").expect("test acceleration cap"),
        Cap::new(8.0, 12.0, "max_step").expect("test step cap"),
    )
    .expect("test limits are consistent");
    Limiter::new(
        limits,
        ControlPeriod::new(0.02, 0.002).expect("test period"),
        MotionCatalog::new([TEST_MOTION]),
        0.0,
    )
    .expect("test limiter")
}

fn test_dispatch() -> MotionDispatch {
    let policy = DuplicatePolicy::new(2, Duration::from_secs(1)).expect("test policy");
    MotionDispatch::new(policy, MotionCatalog::new([TEST_MOTION]))
}

/// Piの`play_motion`を、同じlimiterへ通す。**wireの`PlayMotion`はtargetを持たない**ため、
/// targetは呼び出し側が渡す（名前からtargetを解決する表は未決。`PROTO-TBD-008`）。
fn pi_play(
    dispatch: &mut MotionDispatch,
    limiter: &mut Limiter,
    id: u32,
    name: &str,
    target: f32,
) -> MotionDecision {
    let line = format!(
        r#"{{"v":1,"sid":10,"id":{id},"ts_ms":0,"type":"play_motion","payload":{{"name":"{name}","speed":0.5,"repeat":1}}}}"#
    );
    let Frame { message, .. } = decode_line(&line).expect("valid test wire line");
    let Message::PlayMotion(command) = message else {
        panic!("test line must be play_motion");
    };
    dispatch.handle(
        MotionSession::new(Some(10), true),
        10,
        id,
        &command,
        0,
        |command| {
            limiter
                .admit(&MotionRequest {
                    name: &command.name,
                    target,
                    source: CommandSource::Pi,
                })
                .map(|_| ())
        },
    )
}

/// debugの1行を、同じlimiterへ通す。
fn debug_play(
    dispatch: &MotionDispatch,
    limiter: &mut Limiter,
    line: &str,
) -> Result<(), DebugRefusal> {
    let motion = parse_debug_line(line).expect("test line is well formed");
    dispatch.handle_debug(true, motion.name, motion.target, |request| {
        assert_eq!(
            request.source,
            CommandSource::Debug,
            "handle_debug fixes the source"
        );
        limiter.admit(request).map(|_| ())
    })
}

/// debugのclampは、Piが読むのと同じcounterを増やす。
#[test]
fn a_debug_clamp_and_a_pi_clamp_count_in_the_same_limiter() {
    let mut dispatch = test_dispatch();
    let mut limiter = test_limiter();

    assert_eq!(
        pi_play(
            &mut dispatch,
            &mut limiter,
            1,
            TEST_MOTION,
            TEST_BETWEEN_APPROVED_AND_HARD
        ),
        MotionDecision::Dedup(DedupOutcome::Admitted(Ok(())))
    );
    assert_eq!(limiter.counters().clamped_position, 1);

    let line = format!("MOTION {TEST_MOTION} {TEST_BETWEEN_APPROVED_AND_HARD}");
    assert_eq!(debug_play(&dispatch, &mut limiter, &line), Ok(()));
    assert_eq!(
        limiter.counters().clamped_position,
        2,
        "the debug clamp is counted where the Pi reads it"
    );
}

/// debugのrejectも、同じlimiterの同じ種類のcounterを増やす。
#[test]
fn debug_rejections_count_by_kind_in_the_same_limiter() {
    let mut dispatch = test_dispatch();
    let mut limiter = test_limiter();

    // hard boundの外はrejectする（clampしない）。
    let line = format!("MOTION {TEST_MOTION} {TEST_BEYOND_HARD}");
    assert_eq!(
        debug_play(&dispatch, &mut limiter, &line),
        Err(DebugRefusal::Rejected(Rejection::TargetOutOfHardRange))
    );
    assert_eq!(limiter.counters().rejected_out_of_range, 1);

    // 有限でないtargetは、parserではなくlimiterがrejectする。
    let line = format!("MOTION {TEST_MOTION} nan");
    assert_eq!(
        debug_play(&dispatch, &mut limiter, &line),
        Err(DebugRefusal::Rejected(Rejection::NonFiniteTarget))
    );
    assert_eq!(limiter.counters().rejected_invalid_payload, 1);

    // Piの経路も同じcounterに数える。
    let decision = pi_play(
        &mut dispatch,
        &mut limiter,
        1,
        TEST_MOTION,
        TEST_BEYOND_HARD,
    );
    assert_eq!(
        decision,
        MotionDecision::Dedup(DedupOutcome::Admitted(Err(Rejection::TargetOutOfHardRange)))
    );
    assert_eq!(limiter.counters().rejected_out_of_range, 2);
}

/// catalogに無い名前は、debugでも`admit`に届かず、limiterのcounterを動かさない。
#[test]
fn debug_cannot_skip_the_catalog() {
    let dispatch = test_dispatch();
    let mut limiter = test_limiter();
    let mut called = false;
    let motion = parse_debug_line("MOTION unknown-motion 1").expect("well formed");
    let result = dispatch.handle_debug(true, motion.name, motion.target, |request| {
        called = true;
        limiter.admit(request).map(|_| ())
    });
    assert_eq!(
        result,
        Err(DebugRefusal::Rejected(Rejection::UnknownMotion))
    );
    assert!(
        !called,
        "admit is not reached for a name outside the catalog"
    );
    assert_eq!(
        limiter.counters(),
        deskcat_servo::LimiterCounters::default()
    );

    // 空のcatalogでは、debugも全て拒否される。
    let policy = DuplicatePolicy::new(2, Duration::from_secs(1)).expect("test policy");
    let empty = MotionDispatch::new(policy, MotionCatalog::empty());
    let result = empty.handle_debug(true, TEST_MOTION, TEST_INSIDE, |_| {
        panic!("admit must not be reached with an empty catalog")
    });
    assert_eq!(
        result,
        Err(DebugRefusal::Rejected(Rejection::UnknownMotion))
    );
}

/// debugの指示はPiのduplicateの履歴に触れない。
#[test]
fn debug_does_not_touch_the_pi_duplicate_history() {
    let mut dispatch = test_dispatch();
    let mut limiter = test_limiter();

    let line = format!("MOTION {TEST_MOTION} {TEST_INSIDE}");
    assert_eq!(debug_play(&dispatch, &mut limiter, &line), Ok(()));

    // 同じ`(sid, id)`のPiの初回は、debugの後でも初回として扱われる。
    assert_eq!(
        pi_play(&mut dispatch, &mut limiter, 1, TEST_MOTION, TEST_INSIDE),
        MotionDecision::Dedup(DedupOutcome::Admitted(Ok(())))
    );
}

/// ESP32の`boot`がACKされていない間は、debugもPi由来と同じく`admit`へ届かない（Protocol §4.1の停止状態を含む）。
#[test]
fn debug_is_refused_while_boot_is_not_established() {
    let dispatch = test_dispatch();
    let mut limiter = test_limiter();
    let result = dispatch.handle_debug(false, TEST_MOTION, TEST_INSIDE, |request| {
        limiter.admit(request).map(|_| ())
    });
    assert_eq!(result, Err(DebugRefusal::BootNotEstablished));
    assert_eq!(
        limiter.counters(),
        deskcat_servo::LimiterCounters::default(),
        "a refused debug command does not reach the limiter"
    );

    // Piの経路も、同じ状態で`admit`へ届かない（同じ基準である）。
    let mut pi_dispatch = test_dispatch();
    let line = format!(
        r#"{{"v":1,"sid":10,"id":1,"ts_ms":0,"type":"play_motion","payload":{{"name":"{TEST_MOTION}","speed":0.5,"repeat":1}}}}"#
    );
    let Frame { message, .. } = decode_line(&line).expect("valid test wire line");
    let Message::PlayMotion(command) = message else {
        panic!("test line must be play_motion");
    };
    let decision = pi_dispatch.handle(
        MotionSession::new(Some(10), false),
        10,
        1,
        &command,
        0,
        |_| panic!("admit must not be reached before boot is established"),
    );
    assert_eq!(decision, MotionDecision::StaleSession);
}
