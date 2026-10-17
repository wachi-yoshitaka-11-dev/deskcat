//! Firmwareが呼ぶ`MotionDispatch`を、wire decodeと仮のrelative動作でhostから試す。
//!
//! `TEST_`の値とmotion名はこのtestだけの入力であり、`PROTO-TBD-005`／`008`の
//! 確定値ではない。仮の動作は整数を足すだけで、servoの出力、速度、安全値を試さない。

use core::time::Duration;

use deskcat_protocol::{DuplicatePolicy, ErrorCode, Frame, Message, decode_line};
use deskcat_servo::{DedupOutcome, MotionCatalog, MotionDecision, MotionDispatch, Rejection};

const TEST_MOTION: &str = "test-relative";
const TEST_CAPACITY: usize = 2;
const TEST_RETENTION_MS: u64 = 1_000;

fn dispatch(catalog: MotionCatalog) -> MotionDispatch {
    let policy = DuplicatePolicy::new(TEST_CAPACITY, Duration::from_millis(TEST_RETENTION_MS))
        .expect("test-only duplicate policy");
    MotionDispatch::new(policy, catalog)
}

fn wire(sid: u32, id: u32, name: &str) -> String {
    format!(
        r#"{{"v":1,"sid":{sid},"id":{id},"ts_ms":0,"type":"play_motion","payload":{{"name":"{name}","speed":0.5,"repeat":1}}}}"#
    )
}

/// `position`への加算はhostだけの相対動作。closureが呼ばれた回数を結果で観測する。
fn run(
    dispatch: &mut MotionDispatch,
    current_sid: Option<u32>,
    boot_established: bool,
    line: &str,
    now_ms: u64,
    position: &mut i32,
) -> MotionDecision {
    let Frame { envelope, message } = decode_line(line).expect("valid test wire line");
    let Message::PlayMotion(command) = message else {
        panic!("test line must be play_motion");
    };
    dispatch.handle(
        current_sid,
        boot_established,
        envelope.sid,
        envelope.id,
        &command,
        now_ms,
        |_| {
            *position += 1;
            Ok(())
        },
    )
}

#[test]
fn retry_of_same_sid_and_id_runs_relative_motion_once() {
    let mut dispatch = dispatch(MotionCatalog::new([TEST_MOTION]));
    let mut position = 0;
    let first = wire(10, 1, TEST_MOTION);
    let changed_retry = wire(10, 1, "different-name");

    assert_eq!(
        run(&mut dispatch, Some(10), true, &first, 0, &mut position),
        MotionDecision::Dedup(DedupOutcome::Admitted(Ok(())))
    );
    assert_eq!(
        run(&mut dispatch, Some(10), true, &first, 1, &mut position),
        MotionDecision::Dedup(DedupOutcome::Replayed(Ok(())))
    );
    assert_eq!(
        run(
            &mut dispatch,
            Some(10),
            true,
            &changed_retry,
            2,
            &mut position,
        ),
        MotionDecision::Dedup(DedupOutcome::Replayed(Ok(())))
    );
    assert_eq!(position, 1, "same (sid, id) must have one effect");
}

#[test]
fn old_sid_is_stale_after_a_session_transition() {
    let mut dispatch = dispatch(MotionCatalog::new([TEST_MOTION]));
    let mut position = 0;
    let old = wire(10, 1, TEST_MOTION);
    let current = wire(11, 1, TEST_MOTION);

    assert_eq!(
        run(&mut dispatch, Some(10), true, &old, 0, &mut position),
        MotionDecision::Dedup(DedupOutcome::Admitted(Ok(())))
    );
    dispatch.on_new_session();
    assert_eq!(
        run(&mut dispatch, Some(11), true, &current, 1, &mut position),
        MotionDecision::Dedup(DedupOutcome::Admitted(Ok(())))
    );
    let stale = run(&mut dispatch, Some(11), true, &old, 2, &mut position);
    assert_eq!(stale, MotionDecision::StaleSession);
    assert_eq!(stale.code(), Some(ErrorCode::StaleSession));
    assert_eq!(position, 2, "old relative command must have no new effect");
}

#[test]
fn evicted_id_returns_duplicate_expired_without_running_again() {
    let mut dispatch = dispatch(MotionCatalog::new([TEST_MOTION]));
    let mut position = 0;

    for id in 1..=3 {
        let decision = run(
            &mut dispatch,
            Some(10),
            true,
            &wire(10, id, TEST_MOTION),
            u64::from(id),
            &mut position,
        );
        assert_eq!(
            decision,
            MotionDecision::Dedup(DedupOutcome::Admitted(Ok(())))
        );
    }
    let expired = run(
        &mut dispatch,
        Some(10),
        true,
        &wire(10, 1, TEST_MOTION),
        4,
        &mut position,
    );
    assert_eq!(expired, MotionDecision::Dedup(DedupOutcome::Expired));
    assert_eq!(expired.code(), Some(ErrorCode::DuplicateExpired));
    assert_eq!(position, 3, "evicted ID must not run again");
}

#[test]
fn expired_retention_returns_duplicate_expired_without_running_again() {
    let mut dispatch = dispatch(MotionCatalog::new([TEST_MOTION]));
    let mut position = 0;
    let command = wire(10, 1, TEST_MOTION);

    assert_eq!(
        run(&mut dispatch, Some(10), true, &command, 0, &mut position),
        MotionDecision::Dedup(DedupOutcome::Admitted(Ok(())))
    );
    let expired = run(
        &mut dispatch,
        Some(10),
        true,
        &command,
        TEST_RETENTION_MS,
        &mut position,
    );
    assert_eq!(expired, MotionDecision::Dedup(DedupOutcome::Expired));
    assert_eq!(expired.code(), Some(ErrorCode::DuplicateExpired));
    assert_eq!(position, 1);
}

#[test]
fn boot_pending_returns_stale_without_recording_the_id() {
    let mut dispatch = dispatch(MotionCatalog::new([TEST_MOTION]));
    let mut position = 0;
    let command = wire(10, 1, TEST_MOTION);

    let pending = run(&mut dispatch, Some(10), false, &command, 0, &mut position);
    assert_eq!(pending, MotionDecision::StaleSession);
    assert_eq!(position, 0);
    assert_eq!(
        run(&mut dispatch, Some(10), true, &command, 1, &mut position),
        MotionDecision::Dedup(DedupOutcome::Admitted(Ok(())))
    );
    assert_eq!(position, 1);
}

#[test]
fn empty_catalog_never_reaches_admit() {
    let mut dispatch = dispatch(MotionCatalog::empty());
    let command = wire(10, 1, TEST_MOTION);
    let frame = decode_line(&command).expect("valid test wire line");
    let Message::PlayMotion(command) = frame.message else {
        panic!("test line must be play_motion");
    };

    let first = dispatch.handle(Some(10), true, 10, 1, &command, 0, |_| {
        panic!("empty catalog must not reach an actuator")
    });
    let retry = dispatch.handle(Some(10), true, 10, 1, &command, 1, |_| {
        panic!("replay must not reach an actuator")
    });
    assert_eq!(
        first,
        MotionDecision::Dedup(DedupOutcome::Admitted(Err(Rejection::UnknownMotion)))
    );
    assert_eq!(first.code(), Some(ErrorCode::InvalidPayload));
    assert_eq!(
        retry,
        MotionDecision::Dedup(DedupOutcome::Replayed(Err(Rejection::UnknownMotion)))
    );
}
