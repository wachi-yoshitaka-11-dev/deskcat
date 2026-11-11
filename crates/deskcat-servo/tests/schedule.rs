//! `TickSchedule`と`StopLatch`の振る舞い（Issue #19）。
//!
//! `TEST_`の値はこのtestだけの入力であり、制御周期の値の出所ではない。

use deskcat_servo::{StopLatch, TickSchedule};

/// **test用の値。**
const TEST_PERIOD_MS: u32 = 7;

#[test]
fn zero_period_makes_no_schedule() {
    assert!(TickSchedule::new(0, 100).is_none());
}

#[test]
fn not_due_before_the_first_deadline() {
    let mut schedule = TickSchedule::new(TEST_PERIOD_MS, 100).expect("period");
    assert_eq!(schedule.next_deadline_ms(), 107);
    assert!(!schedule.due(100));
    assert!(!schedule.due(106));
    assert_eq!(schedule.next_deadline_ms(), 107);
}

#[test]
fn due_at_the_deadline_then_next_period_is_aligned_to_the_schedule() {
    let mut schedule = TickSchedule::new(TEST_PERIOD_MS, 100).expect("period");
    assert!(schedule.due(107));
    assert_eq!(schedule.next_deadline_ms(), 114);
    assert!(!schedule.due(109));
    // 少し遅れて呼んでも、次は期限の列（114の次は121。116 + 7ではない）へ整列する。
    assert!(schedule.due(116));
    assert!(!schedule.due(116));
    assert_eq!(schedule.next_deadline_ms(), 121);
    assert_eq!(schedule.overruns(), 0);
}

#[test]
fn late_call_rebases_without_burst_and_counts_the_overrun() {
    let mut schedule = TickSchedule::new(TEST_PERIOD_MS, 0).expect("period");
    // 2周期以上遅れた。取り戻すためのburstはしない。
    assert!(schedule.due(30));
    assert_eq!(schedule.next_deadline_ms(), 37);
    assert!(!schedule.due(30));
    assert_eq!(schedule.overruns(), 1);
}

#[test]
fn deadline_saturates_instead_of_wrapping() {
    let mut schedule = TickSchedule::new(TEST_PERIOD_MS, u64::MAX - 1).expect("period");
    assert_eq!(schedule.next_deadline_ms(), u64::MAX);
    assert!(schedule.due(u64::MAX));
}

#[test]
fn stop_latch_stays_pending_until_a_stop_succeeds() {
    let mut latch = StopLatch::new();
    assert!(!latch.is_pending());
    latch.record::<()>(&Err(()));
    latch.record::<()>(&Err(()));
    assert!(latch.is_pending());
    assert_eq!(latch.failures(), 2);
    latch.record::<()>(&Ok(()));
    assert!(!latch.is_pending());
    // 成功しても、失敗の累計は消えない。
    assert_eq!(latch.failures(), 2);
}

#[test]
fn stop_latch_ok_from_the_start_records_no_failure() {
    let mut latch = StopLatch::default();
    latch.record::<()>(&Ok(()));
    assert!(!latch.is_pending());
    assert_eq!(latch.failures(), 0);
}
