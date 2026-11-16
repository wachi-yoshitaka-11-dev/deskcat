//! `LinkWatch`と`fail_safe_on_loss`の振る舞い（Issue #20）。
//!
//! `TEST_`の値はこのtestだけの入力であり、途切れの判定時間の値の出所ではない。

use deskcat_servo::{
    CommandSource, Drive, LimiterCounters, LinkVerdict, LinkWatch, MotionRequest, MotionSink,
    Rejection, Setpoint, StopLatch, fail_safe_on_loss,
};

/// **test用の値。**
const TEST_TIMEOUT_MS: u32 = 50;

#[test]
fn unset_or_zero_timeout_never_judges_a_loss() {
    for timeout in [None, Some(0)] {
        let mut watch = LinkWatch::new(timeout);
        watch.note_rx(0);
        assert!(!watch.is_armed());
        assert_eq!(watch.check(u64::MAX), LinkVerdict::Idle);
        assert_eq!(watch.deadline_ms(), None);
        assert_eq!(watch.losses(), 0);
    }
}

#[test]
fn idle_until_the_first_rx() {
    let mut watch = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    assert!(watch.is_armed());
    assert_eq!(watch.check(1_000_000), LinkVerdict::Idle);
    assert_eq!(watch.deadline_ms(), None);
}

#[test]
fn alive_before_the_timeout_and_lost_at_it() {
    let mut watch = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    watch.note_rx(100);
    assert_eq!(watch.deadline_ms(), Some(150));
    assert_eq!(watch.check(149), LinkVerdict::Alive);
    assert_eq!(watch.check(150), LinkVerdict::Lost { first: true });
    assert_eq!(watch.check(151), LinkVerdict::Lost { first: false });
    assert_eq!(watch.losses(), 1);
    assert_eq!(watch.deadline_ms(), None);
}

#[test]
fn rx_after_a_loss_makes_it_alive_and_a_new_loss_counts_again() {
    let mut watch = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    watch.note_rx(0);
    assert_eq!(watch.check(60), LinkVerdict::Lost { first: true });
    watch.note_rx(70);
    assert_eq!(watch.check(80), LinkVerdict::Alive);
    assert_eq!(watch.check(120), LinkVerdict::Lost { first: true });
    assert_eq!(watch.losses(), 2);
}

#[test]
fn time_going_backwards_does_not_panic_or_lose() {
    let mut watch = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    watch.note_rx(100);
    watch.note_rx(10);
    assert_eq!(watch.check(5), LinkVerdict::Alive);
    assert_eq!(watch.deadline_ms(), Some(150));
}

#[test]
fn deadline_saturates_instead_of_wrapping() {
    let mut watch = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    watch.note_rx(u64::MAX - 1);
    assert_eq!(watch.deadline_ms(), Some(u64::MAX));
}

/// stopが失敗できる偽のdrive。stop後は新しい動作を受けない。
struct FakeDrive {
    stops: u32,
    fail_stops: u32,
    stopped: bool,
    latch: StopLatch,
}

impl FakeDrive {
    fn new(fail_stops: u32) -> Self {
        Self {
            stops: 0,
            fail_stops,
            stopped: false,
            latch: StopLatch::new(),
        }
    }
}

impl MotionSink for FakeDrive {
    fn admit(&mut self, _request: &MotionRequest<'_>) -> Result<(), Rejection> {
        if self.stopped {
            Err(Rejection::Busy)
        } else {
            Ok(())
        }
    }
    fn counters(&self) -> LimiterCounters {
        LimiterCounters::default()
    }
}

impl Drive for FakeDrive {
    type TickError = ();
    type StopError = ();
    fn tick(&mut self) -> Result<Option<Setpoint>, ()> {
        Ok(None)
    }
    fn stop(&mut self) -> Result<(), ()> {
        self.stops += 1;
        self.stopped = true;
        let result = if self.fail_stops > 0 {
            self.fail_stops -= 1;
            Err(())
        } else {
            Ok(())
        };
        self.latch.record(&result);
        result
    }
}

#[test]
fn loss_stops_the_drive_once() {
    let mut watch = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    let mut drive = FakeDrive::new(0);
    watch.note_rx(0);
    assert!(fail_safe_on_loss(&mut watch, &mut drive, 49).is_none());
    assert_eq!(fail_safe_on_loss(&mut watch, &mut drive, 50), Some(Ok(())));
    assert!(fail_safe_on_loss(&mut watch, &mut drive, 500).is_none());
    assert_eq!(drive.stops, 1);
    assert!(drive.stopped);
}

#[test]
fn a_failed_stop_is_returned_and_left_pending_for_the_caller_to_retry() {
    let mut watch = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    let mut drive = FakeDrive::new(1);
    watch.note_rx(0);
    assert_eq!(fail_safe_on_loss(&mut watch, &mut drive, 50), Some(Err(())));
    assert!(drive.latch.is_pending());
    // 試し直しは呼び出し側（StopLatchが保留の間）。
    assert_eq!(drive.stop(), Ok(()));
    assert!(!drive.latch.is_pending());
}

#[test]
fn unset_timeout_never_stops_the_drive() {
    let mut watch = LinkWatch::new(None);
    let mut drive = FakeDrive::new(0);
    watch.note_rx(0);
    assert!(fail_safe_on_loss(&mut watch, &mut drive, u64::MAX).is_none());
    assert_eq!(drive.stops, 0);
}

#[test]
fn after_a_loss_a_new_motion_is_refused_even_when_the_link_returns() {
    let mut watch = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    let mut drive = FakeDrive::new(0);
    watch.note_rx(0);
    let _ = fail_safe_on_loss(&mut watch, &mut drive, 50);
    watch.note_rx(60);
    let request = MotionRequest {
        name: "test",
        target: 0.0,
        source: CommandSource::Pi,
    };
    assert_eq!(drive.admit(&request), Err(Rejection::Busy));
}

// --- RxGapStats（HW-TBD-017の実測用。観測だけで、判定を変えない） ---

use deskcat_servo::{RX_GAP_BIN_UPPER_MS, RX_GAP_BINS, RxGapStats};

#[test]
fn gap_stats_first_frame_is_only_a_baseline() {
    let mut stats = RxGapStats::new();
    stats.note_rx(1_000);
    assert_eq!(stats.gaps(), 0);
    assert_eq!(stats.max_ms(), 0);
    assert_eq!(stats.bins(), [0; RX_GAP_BINS]);
}

#[test]
fn gap_stats_records_max_sum_count_and_where_the_max_happened() {
    let mut stats = RxGapStats::new();
    for t in [0, 10, 110, 120] {
        stats.note_rx(t);
    }
    assert_eq!(stats.gaps(), 3);
    assert_eq!(stats.max_ms(), 100);
    assert_eq!(stats.max_at_ms(), 110);
    assert_eq!(stats.sum_ms(), 120);
}

#[test]
fn gap_stats_bins_are_inclusive_at_the_upper_edge_and_overflow_goes_last() {
    let mut stats = RxGapStats::new();
    let mut t = 0u64;
    stats.note_rx(t);
    // 各区間の上端ちょうど、上端+1、最後の区間の外。
    for gap in [16, 17, 4096, 4097, 1_000_000] {
        t += gap;
        stats.note_rx(t);
    }
    let bins = stats.bins();
    assert_eq!(bins[0], 1); // 16
    assert_eq!(bins[1], 1); // 17
    assert_eq!(bins[RX_GAP_BINS - 2], 1); // 4096
    assert_eq!(bins[RX_GAP_BINS - 1], 2); // 4097, 1_000_000
    assert_eq!(bins.iter().sum::<u32>(), stats.gaps());
    assert_eq!(RX_GAP_BIN_UPPER_MS.len(), RX_GAP_BINS - 1);
}

#[test]
fn gap_stats_same_instant_frames_count_as_zero_gaps() {
    let mut stats = RxGapStats::new();
    stats.note_rx(50);
    stats.note_rx(50);
    assert_eq!(stats.gaps(), 1);
    assert_eq!(stats.max_ms(), 0);
    assert_eq!(stats.bins()[0], 1);
}

#[test]
fn gap_stats_clock_going_back_is_a_zero_gap_and_keeps_the_latest_baseline() {
    let mut stats = RxGapStats::new();
    stats.note_rx(100);
    stats.note_rx(90);
    assert_eq!(stats.max_ms(), 0);
    stats.note_rx(130);
    assert_eq!(stats.max_ms(), 30);
}

#[test]
fn gap_stats_window_max_resets_but_total_max_stays() {
    let mut stats = RxGapStats::new();
    for t in [0, 200, 210] {
        stats.note_rx(t);
    }
    assert_eq!(stats.take_window_max_ms(), 200);
    stats.note_rx(240);
    assert_eq!(stats.take_window_max_ms(), 30);
    assert_eq!(stats.take_window_max_ms(), 0);
    assert_eq!(stats.max_ms(), 200);
}

#[test]
fn gap_stats_restart_session_does_not_count_the_gap_between_sessions() {
    let mut stats = RxGapStats::new();
    stats.note_rx(0);
    stats.note_rx(10);
    stats.restart_session();
    stats.note_rx(100_000);
    stats.note_rx(100_020);
    assert_eq!(stats.gaps(), 2);
    assert_eq!(stats.max_ms(), 20);
    assert_eq!(stats.sum_ms(), 30);
}

#[test]
fn gap_stats_does_not_touch_the_loss_judgment() {
    // 観測は`LinkWatch`と独立で、同じ時刻を渡しても判定は変わらない。
    let mut with = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    let mut without = LinkWatch::new(Some(TEST_TIMEOUT_MS));
    let mut stats = RxGapStats::new();
    for t in [0, 10, 20] {
        with.note_rx(t);
        stats.note_rx(t);
        without.note_rx(t);
    }
    for now in [20, 69, 70, 71, 500] {
        assert_eq!(with.check(now), without.check(now));
    }
    assert_eq!(with.losses(), without.losses());
}
