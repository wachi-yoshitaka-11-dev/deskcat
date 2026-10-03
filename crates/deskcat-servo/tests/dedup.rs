//! 同じcommandの再送でmotionを二重に始めない層のtest（Issue #19の受け入れ条件4のうち、この層まで）。
//!
//! # ここに出てくる数値はtest用である
//!
//! **`TEST_`で始まる定数は、このtestを走らせるためだけに選んだ値である。**
//! `docs/hardware/servo-safety-limits.md`の`動作制限`表の値でも、`PROTO-TBD-005`
//! （duplicate履歴の保持件数・期間）の値でもない。**このfileの値を正本へ写さない。**
//!
//! # 何を確かめるか
//!
//! [`MotionDedup::admit_once`]が、同じ`id`に対して`admit`を高々1回しか呼ばないことである。
//! `admit`には[`Limiter::admit`]を包んだclosureを渡し、呼ばれた回数と[`Limiter::counters`]で
//! 確かめる。**wireの`play_motion`の受信と、firmwareのsessionへの組み込みは試していない**
//! （`play_motion`の型がまだ無い）。

use core::time::Duration;

use deskcat_protocol::{DuplicatePolicy, ErrorCode};
use deskcat_servo::{
    AdmittedTarget, Cap, CommandSource, ControlPeriod, DedupOutcome, Limiter, LimiterCounters,
    MotionCatalog, MotionDedup, MotionRequest, PositionRange, Rejection, ServoLimits,
};

/// **test用の値。**`最小位置`のhard bound側ではない。
const TEST_HARD_MIN: f32 = -40.0;
/// **test用の値。**`最小位置`の`承認値`ではない。
const TEST_APPROVED_MIN: f32 = -30.0;
/// **test用の値。**`最大位置`の`承認値`ではない。
const TEST_APPROVED_MAX: f32 = 25.0;
/// **test用の値。**`最大位置`のhard bound側ではない。
const TEST_HARD_MAX: f32 = 35.0;
/// **test用の値。**`Neutral位置`ではない。
const TEST_NEUTRAL: f32 = 0.0;
/// **test用の値。**制御周期ではない。
const TEST_DT_S: f32 = 0.02;
/// **test用の名前。**Protocol §5.3 は受理するmotion名を`TBD`としている。
const TEST_MOTION: &str = "test-motion";
/// **test用の値。**`PROTO-TBD-005`の保持件数ではない。
const TEST_CAPACITY: usize = 4;
/// **test用の値。**`PROTO-TBD-005`の保持期間ではない。
const TEST_RETENTION_MS: u64 = 1_000;

fn test_limits() -> ServoLimits {
    ServoLimits::new(
        PositionRange::new(
            TEST_HARD_MIN,
            TEST_APPROVED_MIN,
            TEST_APPROVED_MAX,
            TEST_HARD_MAX,
        )
        .expect("test range is ordered"),
        TEST_NEUTRAL,
        Cap::new(20.0, 25.0, "max_velocity").expect("test velocity cap"),
        Cap::new(40.0, 50.0, "max_acceleration").expect("test acceleration cap"),
        Cap::new(8.0, 12.0, "max_step").expect("test step cap"),
    )
    .expect("test limits are consistent")
}

fn policy(capacity: usize, retention_ms: u64) -> DuplicatePolicy {
    DuplicatePolicy::new(capacity, Duration::from_millis(retention_ms)).expect("0ではない")
}

/// 履歴を持つ所有者の代わり。`admit`が呼ばれた回数と、受理したtargetを持つ。
struct Owner {
    dedup: MotionDedup,
    limiter: Limiter,
    admitted: Option<AdmittedTarget>,
    admit_calls: u32,
}

impl Owner {
    fn new(capacity: usize, retention_ms: u64) -> Self {
        Self {
            dedup: MotionDedup::new(policy(capacity, retention_ms)),
            limiter: Limiter::new(
                test_limits(),
                ControlPeriod::new(TEST_DT_S, 0.0).expect("test period"),
                MotionCatalog::new([TEST_MOTION]),
                TEST_NEUTRAL,
            )
            .expect("test limiter"),
            admitted: None,
            admit_calls: 0,
        }
    }

    /// `id`のmotion commandを、重複の層を通して受理する。
    fn command(&mut self, id: u32, now_ms: u64, name: &str, target: f32) -> DedupOutcome {
        let Self {
            dedup,
            limiter,
            admitted,
            admit_calls,
        } = self;
        dedup.admit_once(id, now_ms, || {
            *admit_calls += 1;
            let request = MotionRequest {
                name,
                target,
                source: CommandSource::Pi,
            };
            limiter.admit(&request).map(|target| {
                *admitted = Some(target);
            })
        })
    }

    fn counters(&self) -> LimiterCounters {
        self.limiter.counters()
    }
}

/// 1. 初めての`id`は、`admit`が1回呼ばれ、成功が記録される。
#[test]
fn a_new_id_calls_admit_once_and_records_ok() {
    let mut owner = Owner::new(TEST_CAPACITY, TEST_RETENTION_MS);

    let outcome = owner.command(1, 0, TEST_MOTION, 10.0);

    assert_eq!(outcome, DedupOutcome::Admitted(Ok(())));
    assert_eq!(outcome.code(), None);
    assert_eq!(owner.admit_calls, 1);
    assert_eq!(owner.admitted.map(AdmittedTarget::target), Some(10.0));
}

/// 2. 同じ`id`の再送は、`admit`を呼ばずに記録した成功を返す。counterもtargetも変わらない。
#[test]
fn a_repeated_id_replays_without_calling_admit_or_counting_again() {
    let mut owner = Owner::new(TEST_CAPACITY, TEST_RETENTION_MS);
    // `承認値`の外でhard boundの内側のtargetは、clampされて受理される（counterが1増える）。
    owner.command(1, 0, TEST_MOTION, 30.0);
    let counters_after_first = owner.counters();
    assert_eq!(counters_after_first.clamped_position, 1);
    let target_after_first = owner.admitted;

    for attempt in 1..=3_u64 {
        let outcome = owner.command(1, attempt * 10, TEST_MOTION, 30.0);
        assert_eq!(outcome, DedupOutcome::Replayed(Ok(())));
    }

    assert_eq!(owner.admit_calls, 1, "再送ではadmitを呼ばない");
    assert_eq!(
        owner.counters(),
        counters_after_first,
        "再送でcounterが増えない"
    );
    assert_eq!(
        owner.admitted, target_after_first,
        "再送でtargetが置き換わらない"
    );
}

/// 3. 拒否した`id`の再送は、`admit`を呼ばずに同じ拒否を返す。rejection counterは1回分だけである。
#[test]
fn a_rejected_id_replays_the_same_rejection_without_counting_twice() {
    let mut owner = Owner::new(TEST_CAPACITY, TEST_RETENTION_MS);

    let first = owner.command(1, 0, "not-in-catalog", 10.0);
    assert_eq!(first, DedupOutcome::Admitted(Err(Rejection::UnknownMotion)));
    let second = owner.command(1, 10, "not-in-catalog", 10.0);
    assert_eq!(
        second,
        DedupOutcome::Replayed(Err(Rejection::UnknownMotion))
    );

    assert_eq!(second.code(), Some(ErrorCode::InvalidPayload));
    assert_eq!(owner.admit_calls, 1);
    assert_eq!(
        owner.counters().rejected_invalid_payload,
        1,
        "二重に数えない"
    );
}

/// 4. 保持件数を超えて捨てられた`id`の再送は、`admit`を呼ばずに`duplicate_expired`で拒否する。
#[test]
fn an_id_evicted_by_capacity_is_expired_and_never_admitted() {
    let mut owner = Owner::new(2, TEST_RETENTION_MS);
    owner.command(1, 0, TEST_MOTION, 1.0);
    owner.command(2, 0, TEST_MOTION, 2.0);
    owner.command(3, 0, TEST_MOTION, 3.0);
    assert_eq!(owner.admit_calls, 3);

    let outcome = owner.command(1, 0, TEST_MOTION, 1.0);

    assert_eq!(outcome, DedupOutcome::Expired);
    assert_eq!(outcome.code(), Some(ErrorCode::DuplicateExpired));
    assert_eq!(
        owner.admit_calls, 3,
        "失われたidを新しいcommandとして実行しない"
    );
}

/// 5. 保持期間を過ぎた`id`の再送も、`admit`を呼ばずに`duplicate_expired`で拒否する。
#[test]
fn an_id_past_retention_is_expired_and_never_admitted() {
    let mut owner = Owner::new(TEST_CAPACITY, TEST_RETENTION_MS);
    owner.command(1, 0, TEST_MOTION, 1.0);

    assert_eq!(
        owner.command(1, TEST_RETENTION_MS - 1, TEST_MOTION, 1.0),
        DedupOutcome::Replayed(Ok(()))
    );
    assert_eq!(
        owner.command(1, TEST_RETENTION_MS, TEST_MOTION, 1.0),
        DedupOutcome::Expired
    );
    assert_eq!(owner.admit_calls, 1);
}

/// 6. 最大の`id`より小さく、履歴に無い`id`（初回の送出が失われた場合）も実行しない。
///
/// `DuplicateHistory`のdocの近似（記録した最大`id`以下で履歴に無い`id`を「処理したが
/// 失われた」とみなす）どおりであり、実行しない側へ倒れる。
#[test]
fn a_lost_first_send_below_the_highest_id_is_expired_not_admitted() {
    let mut owner = Owner::new(TEST_CAPACITY, TEST_RETENTION_MS);
    owner.command(6, 0, TEST_MOTION, 6.0);

    let outcome = owner.command(5, 10, TEST_MOTION, 5.0);

    assert_eq!(outcome, DedupOutcome::Expired);
    assert_eq!(owner.admit_calls, 1);
}

/// 7. 所有者がsession遷移で履歴を破棄した後は、同じ`id`も新しいcommandとして受理される。
#[test]
fn after_the_owner_clears_on_a_session_transition_the_same_id_is_admitted_again() {
    let mut owner = Owner::new(TEST_CAPACITY, TEST_RETENTION_MS);
    owner.command(1, 0, TEST_MOTION, 1.0);

    owner.dedup.clear();
    let outcome = owner.command(1, 10, TEST_MOTION, 2.0);

    assert_eq!(outcome, DedupOutcome::Admitted(Ok(())));
    assert_eq!(owner.admit_calls, 2);
    assert_eq!(owner.admitted.map(AdmittedTarget::target), Some(2.0));
}

/// 8. 別々の`id`は、それぞれ1回ずつ受理される。
#[test]
fn distinct_ids_are_each_admitted_once() {
    let mut owner = Owner::new(TEST_CAPACITY, TEST_RETENTION_MS);

    for id in 1..=3_u32 {
        let target = f32::from(u8::try_from(id).expect("small id"));
        assert_eq!(
            owner.command(id, 0, TEST_MOTION, target),
            DedupOutcome::Admitted(Ok(()))
        );
    }
    for id in 1..=3_u32 {
        let target = f32::from(u8::try_from(id).expect("small id"));
        assert_eq!(
            owner.command(id, 10, TEST_MOTION, target),
            DedupOutcome::Replayed(Ok(()))
        );
    }

    assert_eq!(owner.admit_calls, 3);
}

/// 10. 時刻が記録より前に戻っても、同じ`id`の再送で`admit`は呼ばれない（Replayになる）。
///
/// **単調な時刻を渡す契約に依る**（`DuplicateHistory::record`のdoc）。契約が破られて時刻が
/// 戻ったときに変わるのは、`duplicate_expired`で拒否する代わりに記録した結果を返すことだけで、
/// どちらでも`admit`は呼ばれない。
#[test]
fn a_clock_that_moves_backwards_replays_instead_of_expiring_under_the_monotonic_contract() {
    let mut owner = Owner::new(TEST_CAPACITY, TEST_RETENTION_MS);
    owner.command(1, 5_000, TEST_MOTION, 1.0);

    let outcome = owner.command(1, 0, TEST_MOTION, 1.0);

    assert_eq!(outcome, DedupOutcome::Replayed(Ok(())));
    assert_eq!(owner.admit_calls, 1);
}
