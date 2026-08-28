//! 純粋なtrajectory test（Issue #19 の受け入れ条件6）。
//!
//! # ここに出てくる数値は test 用である
//!
//! **`TEST_`で始まる定数は、このtestを走らせるためだけに選んだ値である。**
//! `docs/hardware/servo-safety-limits.md`の`動作制限`表の`承認値`でも
//! `設定可能なhard bound`でもなく、`HW-TBD-010`／`011`／`020`の値でもない。
//! 同表は`最大連続電流`を除く全行が`TBD`であり、確定は Issue #18 の calibration と
//! 実測による。**このfileの値を正本へ写さない。**
//!
//! 単位も決めていない。位置は「command単位」、速度は「command単位毎秒」である。
//! degreeでもpulse widthでもない。
//!
//! 左右非対称に選んであるのは、符号の取り違えをtestが見逃さないようにするためである。

// 厳密比較を使う。clampの行き先、rejectで状態が動かないこと、Pi経路とdebug経路の
// 一致は、いずれも「ちょうどその値か」を検査するものであり、許容誤差を入れると
// 検査が緩くなる。近似で足りる箇所では`EPSILON`を明示的に使っている。
#![allow(clippy::float_cmp)]

use deskcat_servo::{
    Cap, CommandSource, ControlPeriod, Limiter, LimiterCounters, MotionCatalog, MotionRequest,
    PositionRange, Rejection, ServoLimits,
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
/// **test用の値。**`最大速度`ではない。
const TEST_MAX_VELOCITY: f32 = 20.0;
/// **test用の値。**`最大速度`のhard bound側ではない。
const TEST_MAX_VELOCITY_HARD: f32 = 25.0;
/// **test用の値。**`最大加速度`ではない。
const TEST_MAX_ACCELERATION: f32 = 40.0;
/// **test用の値。**`最大加速度`のhard bound側ではない。
const TEST_MAX_ACCELERATION_HARD: f32 = 50.0;
/// **test用の値。**`単一commandの最大変化量`ではない。
const TEST_MAX_STEP: f32 = 8.0;
/// **test用の値。**`単一commandの最大変化量`のhard bound側ではない。
const TEST_MAX_STEP_HARD: f32 = 12.0;
/// **test用の値。**制御周期ではない。`HEARTBEAT_PERIOD_MS`とも無関係である。
const TEST_DT_S: f32 = 0.02;
/// **test用の名前。**Protocol §5.3 は受理するmotion名を`TBD`としている。
const TEST_MOTION: &str = "test-motion";

/// f32比較の許容誤差。**制限値ではない。**
const EPSILON: f32 = 1e-4;

/// 加速度を比較するときの上限。**制限値ではない。**
///
/// 加速度は `Δv / dt` で求めるため、`dt` が小さいと f32 の丸め誤差が `1/dt` 倍に拡大する。
/// `dt = 0.005` で実測 40.000153（上限 40）が出た。**その誤差を許すためだけの余裕であり、
/// 実際の超過は桁が違う**（`dt`を変える場合で 15000 に達する）。
fn acceleration_ceiling(dt: f32) -> f32 {
    TEST_MAX_ACCELERATION + EPSILON / dt
}

/// 2値の中点。**制限値ではない。**testの範囲を作るためだけに使う。
fn midpoint(a: f32, b: f32) -> f32 {
    a + (b - a) / 2.0
}

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
        Cap::new(TEST_MAX_VELOCITY, TEST_MAX_VELOCITY_HARD, "max_velocity")
            .expect("test velocity cap"),
        Cap::new(
            TEST_MAX_ACCELERATION,
            TEST_MAX_ACCELERATION_HARD,
            "max_acceleration",
        )
        .expect("test acceleration cap"),
        Cap::new(TEST_MAX_STEP, TEST_MAX_STEP_HARD, "max_step").expect("test step cap"),
    )
    .expect("test limits are consistent")
}

/// sweepで使うtarget。hard範囲の外・境界・内側・反転を含む。**test用の値である。**
const SWEEP_TARGETS: [f32; 7] = [
    TEST_APPROVED_MAX,
    TEST_APPROVED_MIN,
    TEST_NEUTRAL,
    TEST_HARD_MAX,
    TEST_HARD_MIN,
    12.5,
    -7.0,
];

/// `単一commandの最大変化量`だけ差し替えたlimiter。**値はtest用である。**
/// `単一commandの最大変化量`を大きくした制限。**test用の値である。**
/// 1 stepで境界へ届かせ、境界での挙動を試すために使う。
fn wide_step_limits() -> ServoLimits {
    ServoLimits::new(
        PositionRange::new(
            TEST_HARD_MIN,
            TEST_APPROVED_MIN,
            TEST_APPROVED_MAX,
            TEST_HARD_MAX,
        )
        .expect("ordered"),
        TEST_NEUTRAL,
        Cap::new(TEST_MAX_VELOCITY, TEST_MAX_VELOCITY_HARD, "max_velocity").expect("cap"),
        Cap::new(
            TEST_MAX_ACCELERATION,
            TEST_MAX_ACCELERATION_HARD,
            "max_acceleration",
        )
        .expect("cap"),
        Cap::new(80.0, 90.0, "max_step").expect("cap"),
    )
    .expect("consistent")
}

/// 許容変動幅を0にした制御周期。**test用であり、実機の周期でもjitter幅でもない。**
fn exact_period(nominal_s: f32) -> ControlPeriod {
    ControlPeriod::new(nominal_s, 0.0).expect("test period is positive and finite")
}

fn limiter_with_max_step(max_step: f32, period_s: f32) -> Limiter {
    let limits = ServoLimits::new(
        PositionRange::new(
            TEST_HARD_MIN,
            TEST_APPROVED_MIN,
            TEST_APPROVED_MAX,
            TEST_HARD_MAX,
        )
        .expect("test range is ordered"),
        TEST_NEUTRAL,
        Cap::new(TEST_MAX_VELOCITY, TEST_MAX_VELOCITY_HARD, "max_velocity").expect("cap"),
        Cap::new(
            TEST_MAX_ACCELERATION,
            TEST_MAX_ACCELERATION_HARD,
            "max_acceleration",
        )
        .expect("cap"),
        Cap::new(max_step, max_step + 10.0, "max_step").expect("cap"),
    )
    .expect("test limits are consistent");
    Limiter::new(
        limits,
        exact_period(period_s),
        MotionCatalog::new([TEST_MOTION]),
        TEST_NEUTRAL,
    )
    .expect("start is inside the approved range")
}

fn limiter_at(position: f32) -> Limiter {
    Limiter::new(
        test_limits(),
        exact_period(TEST_DT_S),
        MotionCatalog::new([TEST_MOTION]),
        position,
    )
    .expect("test start position is inside the approved range")
}

fn limiter() -> Limiter {
    limiter_at(TEST_NEUTRAL)
}

fn request(target: f32, source: CommandSource) -> MotionRequest<'static> {
    MotionRequest {
        name: TEST_MOTION,
        target,
        source,
    }
}

/// `admit`と`step`を`steps`回まわし、毎stepで安全不変条件を検査する。
///
/// 返すのは実際に進んだstep数ではなく、走らせたあとのlimiterである。
fn drive(limiter: &mut Limiter, target: f32, steps: usize) {
    let mut previous_velocity = limiter.velocity();
    for index in 0..steps {
        let admitted = limiter
            .admit(&request(target, CommandSource::Pi))
            .expect("target is inside the hard range");
        let setpoint = limiter.step(admitted, TEST_DT_S).expect("dt is positive");

        assert_invariants(
            setpoint.position(),
            setpoint.velocity(),
            previous_velocity,
            index,
        );
        previous_velocity = setpoint.velocity();
    }
}

/// 受け入れ条件2 の不変条件。position、velocity、accelerationがhard boundを超えない。
fn assert_invariants(position: f32, velocity: f32, previous_velocity: f32, index: usize) {
    assert!(
        (TEST_HARD_MIN - EPSILON..=TEST_HARD_MAX + EPSILON).contains(&position),
        "step {index}: position {position} left the hard range"
    );
    assert!(
        (TEST_APPROVED_MIN - EPSILON..=TEST_APPROVED_MAX + EPSILON).contains(&position),
        "step {index}: position {position} left the approved range"
    );
    assert!(
        velocity.abs() <= TEST_MAX_VELOCITY + EPSILON,
        "step {index}: velocity {velocity} exceeded the approved max velocity"
    );
    assert!(
        velocity.abs() <= TEST_MAX_VELOCITY_HARD + EPSILON,
        "step {index}: velocity {velocity} exceeded the hard max velocity"
    );
    let acceleration = (velocity - previous_velocity).abs() / TEST_DT_S;
    assert!(
        acceleration <= TEST_MAX_ACCELERATION + EPSILON,
        "step {index}: acceleration {acceleration} exceeded the approved max acceleration"
    );
    assert!(
        acceleration <= TEST_MAX_ACCELERATION_HARD + EPSILON,
        "step {index}: acceleration {acceleration} exceeded the hard max acceleration"
    );
}

// ---------------------------------------------------------------------------
// 受け入れ条件1: 不正な motion を reject する
// ---------------------------------------------------------------------------

#[test]
fn rejects_a_motion_name_that_is_not_in_the_catalog() {
    let mut limiter = limiter();
    let rejection = limiter
        .admit(&request(TEST_NEUTRAL, CommandSource::Pi).with_name("not-in-catalog"))
        .expect_err("unknown motion name");

    assert_eq!(rejection, Rejection::UnknownMotion);
    assert_eq!(
        rejection.code(),
        deskcat_protocol::ErrorCode::InvalidPayload,
        "Protocol §7は「列挙外の値」をinvalid_payloadとしている"
    );
    assert_eq!(limiter.counters().rejected_invalid_payload, 1);
}

#[test]
fn an_empty_catalog_rejects_every_name() {
    // motion名がTBDである間の安全側の状態。Protocol §5.3 は受理するmotion名を
    // 「calibrationが完了するまでTBD」としている。
    let mut limiter = Limiter::new(
        test_limits(),
        exact_period(TEST_DT_S),
        MotionCatalog::empty(),
        TEST_NEUTRAL,
    )
    .expect("start position is inside the approved range");

    assert!(MotionCatalog::empty().is_empty());
    for name in [TEST_MOTION, "", "nod", "neutral"] {
        assert_eq!(
            limiter.admit(&request(TEST_NEUTRAL, CommandSource::Pi).with_name(name)),
            Err(Rejection::UnknownMotion),
            "empty catalog must reject {name:?}"
        );
    }
}

#[test]
fn rejects_targets_that_are_not_finite() {
    for target in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut limiter = limiter();
        let rejection = limiter
            .admit(&request(target, CommandSource::Pi))
            .expect_err("non-finite target");

        assert_eq!(rejection, Rejection::NonFiniteTarget);
        assert_eq!(
            rejection.code(),
            deskcat_protocol::ErrorCode::InvalidPayload,
            "構造的に不正であり、clampよりrejectを優先する"
        );
        assert_eq!(limiter.counters().rejected_invalid_payload, 1);
    }
}

#[test]
fn rejects_intervals_that_are_not_positive() {
    for dt in [0.0, -TEST_DT_S, f32::NAN, f32::INFINITY] {
        let mut limiter = limiter();
        let admitted = limiter
            .admit(&request(TEST_APPROVED_MAX, CommandSource::Pi))
            .expect("target is valid");
        assert_eq!(
            limiter.step(admitted, dt),
            Err(Rejection::NonPositiveInterval),
            "dt {dt} must be rejected"
        );
        assert_eq!(limiter.counters().rejected_invalid_payload, 1);
    }
}

#[test]
fn the_name_check_runs_before_the_target_check() {
    // `Command処理`の順序は motion-name/target validation → hard range clamp or rejection。
    // 名前もtargetも不正なときは、先に走る名前の判定で落ちる。
    let mut limiter = limiter();
    let rejection = limiter
        .admit(&request(f32::NAN, CommandSource::Pi).with_name("not-in-catalog"))
        .expect_err("both are invalid");

    assert_eq!(rejection, Rejection::UnknownMotion);
}

// ---------------------------------------------------------------------------
// 受け入れ条件2: hard bound を超えない。clamp より reject を優先する
// ---------------------------------------------------------------------------

#[test]
fn rejects_targets_outside_the_hard_range_instead_of_clamping_them() {
    // `Command処理`: 「構造的に不正または明らかに危険なcommandは、clampよりrejectを優先する」。
    for target in [
        TEST_HARD_MAX + EPSILON,
        TEST_HARD_MIN - EPSILON,
        f32::MAX,
        f32::MIN,
    ] {
        let mut limiter = limiter();
        let rejection = limiter
            .admit(&request(target, CommandSource::Pi))
            .expect_err("target is outside the hard range");

        assert_eq!(rejection, Rejection::TargetOutOfHardRange);
        assert_eq!(
            rejection.code(),
            deskcat_protocol::ErrorCode::OutOfRange,
            "Protocol §5.3は「値そのものが許容範囲外」をout_of_rangeとしている"
        );
        assert_eq!(limiter.counters().rejected_out_of_range, 1);
        assert_eq!(
            limiter.counters().total_clamps(),
            0,
            "rejectした要求をclampとして数えない"
        );
        assert_eq!(limiter.position(), TEST_NEUTRAL, "rejectで状態を動かさない");
    }
}

#[test]
fn clamps_targets_between_the_approved_bound_and_the_hard_bound() {
    for (target, expected) in [
        (TEST_HARD_MAX, TEST_APPROVED_MAX),
        (TEST_HARD_MIN, TEST_APPROVED_MIN),
        (
            midpoint(TEST_APPROVED_MAX, TEST_HARD_MAX),
            TEST_APPROVED_MAX,
        ),
        (
            midpoint(TEST_APPROVED_MIN, TEST_HARD_MIN),
            TEST_APPROVED_MIN,
        ),
    ] {
        let mut limiter = limiter();
        let admitted = limiter
            .admit(&request(target, CommandSource::Pi))
            .expect("target is inside the hard range");

        assert!(
            admitted.clamped(),
            "target {target} must be reported as clamped"
        );
        assert_eq!(admitted.target(), expected);
        assert_eq!(limiter.counters().clamped_position, 1);
        assert_eq!(limiter.counters().total_rejections(), 0);
    }
}

#[test]
fn accepts_targets_on_the_approved_bounds_without_clamping() {
    for target in [TEST_APPROVED_MIN, TEST_APPROVED_MAX, TEST_NEUTRAL] {
        let mut limiter = limiter();
        let admitted = limiter
            .admit(&request(target, CommandSource::Pi))
            .expect("target is inside the approved range");

        assert!(
            !admitted.clamped(),
            "target {target} is inside and must not clamp"
        );
        assert_eq!(admitted.target(), target);
        assert_eq!(limiter.counters().total_clamps(), 0);
    }
}

#[test]
fn a_full_travel_to_each_bound_never_leaves_the_bounds() {
    // 受け入れ条件2 の中心。端から端まで走らせても、position・velocity・acceleration が
    // hard bound を超えない。`drive`が毎stepで不変条件を検査する。
    let mut limiter = limiter();
    drive(&mut limiter, TEST_APPROVED_MAX, 200);
    assert!((limiter.position() - TEST_APPROVED_MAX).abs() < EPSILON);
    assert!(limiter.velocity().abs() < EPSILON, "境界で止まりきる");

    drive(&mut limiter, TEST_APPROVED_MIN, 400);
    assert!((limiter.position() - TEST_APPROVED_MIN).abs() < EPSILON);
    assert!(limiter.velocity().abs() < EPSILON, "境界で止まりきる");
}

#[test]
fn braking_starts_before_the_bound_so_the_acceleration_bound_holds() {
    // 減速を入れないと、境界に達したstepで位置clampが速度を一気に落とし、
    // `最大加速度`を超える。`drive`の不変条件がその回帰を捕まえる。
    let mut limiter = limiter();
    drive(&mut limiter, TEST_APPROVED_MAX, 200);

    assert!(
        limiter.counters().clamped_braking > 0,
        "境界へ向かう走行で減速が作用する"
    );
    assert_eq!(
        limiter.counters().clamped_position,
        0,
        "減速が効いていれば、安全網の位置clampは作用しない"
    );
}

#[test]
fn hard_bounds_hold_across_a_sweep_of_targets_and_reversals() {
    // 境界の網羅。hard boundの外・境界上・承認値の境界上・内側を順に往復させる。
    let mut limiter = limiter();
    let targets = [
        TEST_APPROVED_MAX,
        TEST_APPROVED_MIN,
        TEST_NEUTRAL,
        TEST_HARD_MAX, // clampされてTEST_APPROVED_MAXへ
        TEST_HARD_MIN, // clampされてTEST_APPROVED_MINへ
        midpoint(TEST_APPROVED_MIN, TEST_NEUTRAL),
        TEST_APPROVED_MAX,
    ];
    for target in targets {
        drive(&mut limiter, target, 60);
    }

    // 範囲外は毎回rejectされ、状態を動かさない。
    for target in [TEST_HARD_MAX + 1.0, TEST_HARD_MIN - 1.0] {
        let before = limiter.position();
        assert!(limiter.admit(&request(target, CommandSource::Pi)).is_err());
        assert_eq!(limiter.position(), before);
    }
}

#[test]
fn a_single_step_never_moves_further_than_the_max_step() {
    let mut limiter = limiter();
    let mut previous = limiter.position();
    for _ in 0..200 {
        let admitted = limiter
            .admit(&request(TEST_APPROVED_MAX, CommandSource::Pi))
            .expect("valid target");
        let setpoint = limiter.step(admitted, TEST_DT_S).expect("dt is positive");
        assert!(
            (setpoint.position() - previous).abs() <= TEST_MAX_STEP + EPSILON,
            "1 stepの変化量が`単一commandの最大変化量`を超えた"
        );
        previous = setpoint.position();
    }
}

#[test]
fn a_long_interval_is_bounded_by_velocity_not_by_the_interval() {
    // 制御周期そのものが長い場合。1 stepの移動量は`最大速度`×周期と
    // `単一commandの最大変化量`の小さい方を超えない。
    let long_dt = 10.0;
    let mut limiter = limiter_with_max_step(TEST_MAX_STEP, long_dt);
    let admitted = limiter
        .admit(&request(TEST_APPROVED_MAX, CommandSource::Pi))
        .expect("valid target");
    let setpoint = limiter
        .step(admitted, long_dt)
        .expect("dt is the control period");

    assert!(setpoint.position() <= TEST_APPROVED_MAX + EPSILON);
    assert!(setpoint.velocity().abs() <= TEST_MAX_VELOCITY + EPSILON);
    assert!(
        (setpoint.velocity() - 0.0).abs() <= TEST_MAX_ACCELERATION * long_dt + EPSILON,
        "加速度の制限はdtに比例して効く"
    );
}

/// 制御周期が一定なら、`最大加速度`を1度も超えず、譲りもしない。
///
/// `dt`・target・`単一commandの最大変化量`を変えた組み合わせを総当たりし、
/// 境界への突入と反転を含む長い列を回す。
#[test]
fn a_constant_interval_never_exceeds_the_acceleration_bound() {
    let mut worst = 0.0f32;
    for max_step in [TEST_MAX_STEP, 80.0, 0.5] {
        for dt in [10.0f32, 1.0, 0.25, 0.05, TEST_DT_S, 0.005, 0.001] {
            let mut limiter = limiter_with_max_step(max_step, dt);
            let mut previous_velocity = 0.0f32;
            for k in 0..120 {
                let target = SWEEP_TARGETS[(k / 11) % SWEEP_TARGETS.len()];
                let Ok(admitted) = limiter.admit(&request(target, CommandSource::Pi)) else {
                    continue;
                };
                let setpoint = limiter
                    .step(admitted, dt)
                    .expect("dt is the control period");
                let acceleration = (setpoint.velocity() - previous_velocity).abs() / dt;
                if acceleration > worst {
                    worst = acceleration;
                }
                assert!(
                    acceleration <= acceleration_ceiling(dt),
                    "max_step {max_step}, dt {dt}, step {k}: acceleration {acceleration} exceeded"
                );
                assert!(
                    !setpoint.clamps().acceleration_bound_conceded,
                    "max_step {max_step}, dt {dt}, step {k}: conceded under a constant interval"
                );
                previous_velocity = setpoint.velocity();
            }
            assert_eq!(limiter.counters().acceleration_bound_conceded, 0);
        }
    }
    assert!(worst > 0.0, "sweepが実際に加速度を動かしていること");
}

/// 制御周期の外にある間隔は reject する。**doc の依頼ではなく検査で強制する。**
///
/// review が挙げた再現手順（長い間隔の直後に短い間隔）が、ここで止まる。
#[test]
fn an_interval_outside_the_control_period_is_rejected() {
    let mut limiter = limiter_with_max_step(80.0, TEST_DT_S);
    let admitted = limiter
        .admit(&request(TEST_APPROVED_MAX, CommandSource::Pi))
        .expect("valid target");

    // review の再現手順の1歩目そのもの。
    let rejection = limiter
        .step(admitted, 10.0)
        .expect_err("10.0 is not the control period");
    assert_eq!(rejection, Rejection::IntervalOutsideControlPeriod);
    assert_eq!(
        rejection.code(),
        deskcat_protocol::ErrorCode::OutOfRange,
        "§5.3は「値そのものが許容範囲外」をout_of_rangeとしている"
    );
    assert_eq!(limiter.counters().rejected_out_of_range, 1);
    assert_eq!(limiter.position(), TEST_NEUTRAL, "rejectで状態を動かさない");

    // 小さすぎる方向も同じ。**危険なのはこちらの方向である。**
    for dt in [0.001f32, 0.005, 0.019, 0.021, 1.0] {
        let mut limiter = limiter_with_max_step(80.0, TEST_DT_S);
        let admitted = limiter
            .admit(&request(TEST_APPROVED_MAX, CommandSource::Pi))
            .expect("valid target");
        assert_eq!(
            limiter.step(admitted, dt),
            Err(Rejection::IntervalOutsideControlPeriod),
            "dt {dt} は許容幅0の制御周期の外である"
        );
    }
}

/// 許容変動幅の内側の jitter は受理する。**厳密一致にすると正常な系が止まる。**
#[test]
fn jitter_inside_the_tolerance_is_accepted() {
    let nominal = TEST_DT_S;
    // **test用の幅。**実機のjitter実測値ではない。
    let tolerance = 0.002_f32;
    let period = ControlPeriod::new(nominal, tolerance).expect("valid period");
    let mut limiter = Limiter::new(
        test_limits(),
        period,
        MotionCatalog::new([TEST_MOTION]),
        TEST_NEUTRAL,
    )
    .expect("start is inside the approved range");

    // **境界ちょうどは検査しない。**`|dt - nominal|`を f32 で求めるため、境界上の値は
    // どちらに転ぶか決まらない。内側と、明確に外側だけを検査する。
    for dt in [
        nominal,
        nominal - tolerance * 0.5,
        nominal + tolerance * 0.5,
    ] {
        let admitted = limiter
            .admit(&request(TEST_APPROVED_MAX, CommandSource::Pi))
            .expect("valid target");
        let setpoint = limiter.step(admitted, dt).expect("inside the tolerance");
        assert!(
            (TEST_HARD_MIN - EPSILON..=TEST_HARD_MAX + EPSILON).contains(&setpoint.position()),
            "dt {dt}: position left the hard range"
        );
    }

    // 幅のすぐ外は reject する。
    let admitted = limiter
        .admit(&request(TEST_APPROVED_MAX, CommandSource::Pi))
        .expect("valid target");
    assert_eq!(
        limiter.step(admitted, nominal + tolerance * 2.0),
        Err(Rejection::IntervalOutsideControlPeriod)
    );
}

/// 許容変動幅を広く取っても、**受理したすべての step で3つの bound を保ち、譲らない。**
///
/// 減速の見積もりに、契約が許す**最短**の間隔（1 stepで確実に使える減速量）と
/// **最長**の間隔（1 stepで進みうる距離）を使っているためである。実際の間隔が幅の
/// どこに来ても、見積もりより楽になる。
///
/// **代償は、幅を広く取るほど境界への接近が保守的になること**である。
/// 違反ではなく遅さとして現れる。
#[test]
fn a_wide_tolerance_stays_within_every_bound_and_never_concedes() {
    // **test用の組。**実機の周期でもjitter幅でもない。幅を周期の9割まで広げてある。
    for (nominal, tolerance) in [(1.0f32, 0.9f32), (0.1, 0.09), (TEST_DT_S, 0.018)] {
        let period = ControlPeriod::new(nominal, tolerance).expect("valid period");
        let mut limiter = Limiter::new(
            wide_step_limits(),
            period,
            MotionCatalog::new([TEST_MOTION]),
            TEST_NEUTRAL,
        )
        .expect("start is inside the approved range");

        let mut previous_velocity = 0.0f32;
        for k in 0..250 {
            // 幅の端から端まで揺らす。
            let dt = if k % 2 == 0 {
                nominal + tolerance * 0.99
            } else {
                nominal - tolerance * 0.99
            };
            let target = SWEEP_TARGETS[(k / 11) % SWEEP_TARGETS.len()];
            let Ok(admitted) = limiter.admit(&request(target, CommandSource::Pi)) else {
                continue;
            };
            let setpoint = limiter.step(admitted, dt).expect("inside the tolerance");

            assert!(
                (TEST_HARD_MIN - EPSILON..=TEST_HARD_MAX + EPSILON).contains(&setpoint.position()),
                "nominal {nominal}, step {k}: position left the hard range"
            );
            assert!(
                setpoint.velocity().abs() <= TEST_MAX_VELOCITY_HARD + EPSILON,
                "nominal {nominal}, step {k}: velocity left the hard bound"
            );
            let acceleration = (setpoint.velocity() - previous_velocity).abs() / dt;
            assert!(
                acceleration <= acceleration_ceiling(dt),
                "nominal {nominal}, step {k}: acceleration {acceleration} exceeded"
            );
            assert!(
                !setpoint.clamps().acceleration_bound_conceded,
                "nominal {nominal}, step {k}: conceded despite the conservative estimate"
            );
            previous_velocity = setpoint.velocity();
        }
        assert_eq!(
            limiter.counters().acceleration_bound_conceded,
            0,
            "nominal {nominal}: 譲りは起きないはずである"
        );
    }
}

/// 広い許容幅でも境界へ到達し、速度が0まで落ちきる。**保守的にしすぎて手前で止まらない。**
#[test]
fn a_wide_tolerance_still_reaches_the_bound_and_stops() {
    // 幅を周期の9割にした極端な設定。到達は遅くなるが、止まる。
    let (nominal, tolerance) = (0.001_f32, 0.0009_f32);
    let period = ControlPeriod::new(nominal, tolerance).expect("valid period");
    let mut limiter = Limiter::new(
        wide_step_limits(),
        period,
        MotionCatalog::new([TEST_MOTION]),
        TEST_NEUTRAL,
    )
    .expect("start is inside the approved range");

    for k in 0..20_000 {
        let dt = if k % 2 == 0 {
            nominal + tolerance * 0.99
        } else {
            nominal - tolerance * 0.99
        };
        let admitted = limiter
            .admit(&request(TEST_APPROVED_MAX, CommandSource::Pi))
            .expect("valid target");
        limiter.step(admitted, dt).expect("inside the tolerance");
        if (limiter.position() - TEST_APPROVED_MAX).abs() < EPSILON
            && limiter.velocity().abs() < EPSILON
        {
            assert_eq!(limiter.counters().acceleration_bound_conceded, 0);
            return;
        }
    }
    panic!(
        "境界へ到達しなかった: pos={} vel={}",
        limiter.position(),
        limiter.velocity()
    );
}

/// 制御周期そのものの整合検査。
#[test]
fn an_inconsistent_control_period_is_refused() {
    for (nominal, tolerance) in [
        (0.0_f32, 0.0_f32),
        (-TEST_DT_S, 0.0),
        (TEST_DT_S, -0.001),
        (TEST_DT_S, TEST_DT_S),
        (TEST_DT_S, TEST_DT_S * 2.0),
        (f32::NAN, 0.0),
        (TEST_DT_S, f32::INFINITY),
    ] {
        assert!(
            matches!(
                ControlPeriod::new(nominal, tolerance),
                Err(deskcat_servo::LimitsError::InvalidControlPeriod)
            ),
            "period ({nominal}, {tolerance}) must be refused"
        );
    }
    let ok = ControlPeriod::new(TEST_DT_S, 0.0).expect("zero tolerance is allowed");
    assert_eq!(ok.nominal_s(), TEST_DT_S);
    assert_eq!(ok.tolerance_s(), 0.0);
    assert!(ok.accepts(TEST_DT_S));
    assert!(!ok.accepts(TEST_DT_S * 2.0));
    assert!(!ok.accepts(f32::NAN));
}

// ---------------------------------------------------------------------------
// 受け入れ条件3: Debug command が同じ limiter を使用する
// ---------------------------------------------------------------------------

#[test]
fn a_debug_command_gets_the_same_decision_as_a_pi_command() {
    // `AGENTS.md`: 「サーボ安全制限をデバッグ経路からも迂回させない」。
    // `servo-safety-limits.md`: 「AIが生成したcommandやdebug commandでも
    // hard limitを迂回できない」。
    let probes = [
        TEST_HARD_MAX + 1.0,
        TEST_HARD_MAX,
        TEST_APPROVED_MAX,
        TEST_NEUTRAL,
        TEST_APPROVED_MIN,
        TEST_HARD_MIN,
        TEST_HARD_MIN - 1.0,
        f32::NAN,
    ];

    for target in probes {
        let mut from_pi = limiter();
        let mut from_debug = limiter();

        let pi = from_pi.admit(&request(target, CommandSource::Pi));
        let debug = from_debug.admit(&request(target, CommandSource::Debug));

        match (pi, debug) {
            (Ok(pi), Ok(debug)) => {
                assert_eq!(pi.target(), debug.target(), "target {target}");
                assert_eq!(pi.clamped(), debug.clamped(), "target {target}");
                assert_eq!(pi.source(), CommandSource::Pi);
                assert_eq!(debug.source(), CommandSource::Debug, "出所は報告のため残る");
            }
            (Err(pi), Err(debug)) => assert_eq!(pi, debug, "target {target}"),
            (pi, debug) => panic!("target {target}: pi={pi:?} debug={debug:?} が食い違った"),
        }
        assert_eq!(
            from_pi.counters(),
            from_debug.counters(),
            "target {target}: counterも同じでなければならない"
        );
    }
}

#[test]
fn a_debug_trajectory_is_bounded_exactly_like_a_pi_trajectory() {
    let mut from_pi = limiter();
    let mut from_debug = limiter();

    for _ in 0..200 {
        let pi = from_pi
            .admit(&request(TEST_HARD_MAX, CommandSource::Pi))
            .expect("clamped, not rejected");
        let debug = from_debug
            .admit(&request(TEST_HARD_MAX, CommandSource::Debug))
            .expect("clamped, not rejected");

        let pi = from_pi.step(pi, TEST_DT_S).expect("dt is positive");
        let debug = from_debug.step(debug, TEST_DT_S).expect("dt is positive");

        assert_eq!(pi.position(), debug.position());
        assert_eq!(pi.velocity(), debug.velocity());
        assert_eq!(pi.clamps(), debug.clamps());
    }
    assert_eq!(from_pi.counters(), from_debug.counters());
}

// ---------------------------------------------------------------------------
// 受け入れ条件5: Clamp／rejection counter を観測できる
// ---------------------------------------------------------------------------

#[test]
fn counters_start_at_zero() {
    let limiter = limiter();
    assert_eq!(limiter.counters(), LimiterCounters::default());
    assert_eq!(limiter.counters().total_clamps(), 0);
    assert_eq!(limiter.counters().total_rejections(), 0);
}

#[test]
fn each_clamp_kind_is_counted_separately() {
    let mut limiter = limiter();

    // 位置clamp（admit側）。
    let admitted = limiter
        .admit(&request(TEST_HARD_MAX, CommandSource::Pi))
        .expect("inside the hard range");
    assert!(admitted.clamped());
    assert_eq!(limiter.counters().clamped_position, 1);

    // step clampと速度clampは、遠いtargetへの最初のstepで同時に立つ。
    let setpoint = limiter.step(admitted, TEST_DT_S).expect("dt is positive");
    assert!(
        setpoint.clamps().step,
        "変化量が`単一commandの最大変化量`を超える"
    );
    assert!(setpoint.clamps().velocity, "速度が`最大速度`を超える");
    assert!(
        setpoint.clamps().acceleration,
        "静止からの立ち上がりで加速度が効く"
    );
    assert!(setpoint.clamps().any());
    let counters = limiter.counters();
    assert_eq!(counters.clamped_step, 1);
    assert_eq!(counters.clamped_velocity, 1);
    assert_eq!(counters.clamped_acceleration, 1);
}

#[test]
fn rejection_counters_are_split_by_wire_error_code() {
    let mut limiter = limiter();

    limiter
        .admit(&request(TEST_NEUTRAL, CommandSource::Pi).with_name("not-in-catalog"))
        .expect_err("unknown motion");
    limiter
        .admit(&request(f32::NAN, CommandSource::Pi))
        .expect_err("non-finite target");
    limiter
        .admit(&request(TEST_HARD_MAX + 1.0, CommandSource::Pi))
        .expect_err("outside the hard range");

    let counters = limiter.counters();
    assert_eq!(counters.rejected_invalid_payload, 2);
    assert_eq!(counters.rejected_out_of_range, 1);
    assert_eq!(counters.total_rejections(), 3);
    assert_eq!(counters.total_clamps(), 0);
}

#[test]
fn a_setpoint_reports_the_clamps_of_its_own_step() {
    // `Command処理`: 「有効なtargetを保守的にclampした場合は、machine-readableな
    // statusまたはeventで報告する」。単発の報告がこれである。
    let mut limiter = limiter_at(TEST_NEUTRAL);
    let admitted = limiter
        .admit(&request(TEST_NEUTRAL, CommandSource::Pi))
        .expect("already at the target");
    let setpoint = limiter.step(admitted, TEST_DT_S).expect("dt is positive");

    assert!(!setpoint.clamps().any(), "動かない要求では何もclampしない");
    assert_eq!(setpoint.position(), TEST_NEUTRAL);
    assert_eq!(limiter.counters().total_clamps(), 0);
}

#[test]
fn counters_saturate_instead_of_wrapping() {
    let counters = LimiterCounters {
        clamped_position: u32::MAX,
        clamped_step: u32::MAX,
        clamped_velocity: u32::MAX,
        clamped_braking: u32::MAX,
        clamped_acceleration: u32::MAX,
        acceleration_bound_conceded: u32::MAX,
        rejected_invalid_payload: u32::MAX,
        rejected_out_of_range: u32::MAX,
    };
    assert_eq!(counters.total_clamps(), u32::MAX);
    assert_eq!(counters.total_rejections(), u32::MAX);
}

// ---------------------------------------------------------------------------
// 制限そのものの整合検査
// ---------------------------------------------------------------------------

#[test]
fn a_cap_whose_approved_value_exceeds_its_hard_bound_is_refused() {
    assert!(matches!(
        Cap::new(TEST_MAX_VELOCITY_HARD, TEST_MAX_VELOCITY, "max_velocity"),
        Err(deskcat_servo::LimitsError::ApprovedExceedsHard {
            field: "max_velocity"
        })
    ));
}

#[test]
fn a_negative_or_non_finite_cap_is_refused() {
    assert!(matches!(
        Cap::new(-1.0, TEST_MAX_VELOCITY, "max_velocity"),
        Err(deskcat_servo::LimitsError::Negative { .. })
    ));
    assert!(matches!(
        Cap::new(f32::NAN, TEST_MAX_VELOCITY, "max_velocity"),
        Err(deskcat_servo::LimitsError::NotFinite { .. })
    ));
}

#[test]
fn a_position_range_that_is_not_nested_is_refused() {
    // 承認値の範囲はhard boundの内側でなければならない。
    assert!(matches!(
        PositionRange::new(
            TEST_APPROVED_MIN,
            TEST_HARD_MIN,
            TEST_APPROVED_MAX,
            TEST_HARD_MAX
        ),
        Err(deskcat_servo::LimitsError::RangeOutOfOrder)
    ));
    assert!(matches!(
        PositionRange::new(
            TEST_HARD_MIN,
            TEST_APPROVED_MIN,
            TEST_HARD_MAX,
            TEST_APPROVED_MAX
        ),
        Err(deskcat_servo::LimitsError::RangeOutOfOrder)
    ));
    assert!(matches!(
        PositionRange::new(
            TEST_HARD_MIN,
            TEST_APPROVED_MAX,
            TEST_APPROVED_MIN,
            TEST_HARD_MAX
        ),
        Err(deskcat_servo::LimitsError::RangeOutOfOrder)
    ));
}

#[test]
fn a_neutral_outside_the_approved_range_is_refused() {
    let range = PositionRange::new(
        TEST_HARD_MIN,
        TEST_APPROVED_MIN,
        TEST_APPROVED_MAX,
        TEST_HARD_MAX,
    )
    .expect("ordered");
    // hard boundの内側であっても、承認値の範囲の外なら受け付けない。
    assert!(matches!(
        ServoLimits::new(
            range,
            TEST_HARD_MAX,
            Cap::new(TEST_MAX_VELOCITY, TEST_MAX_VELOCITY_HARD, "v").expect("cap"),
            Cap::new(TEST_MAX_ACCELERATION, TEST_MAX_ACCELERATION_HARD, "a").expect("cap"),
            Cap::new(TEST_MAX_STEP, TEST_MAX_STEP_HARD, "s").expect("cap"),
        ),
        Err(deskcat_servo::LimitsError::NeutralOutsideApprovedRange)
    ));
}

#[test]
fn a_start_position_outside_the_approved_range_is_refused() {
    assert!(
        Limiter::new(
            test_limits(),
            exact_period(TEST_DT_S),
            MotionCatalog::new([TEST_MOTION]),
            TEST_HARD_MAX
        )
        .is_err()
    );
    assert!(
        Limiter::new(
            test_limits(),
            exact_period(TEST_DT_S),
            MotionCatalog::new([TEST_MOTION]),
            f32::NAN
        )
        .is_err()
    );
}

/// testの読みやすさのためだけの補助。名前を差し替えた要求を作る。
trait WithName<'a> {
    fn with_name(self, name: &'a str) -> MotionRequest<'a>;
}

impl<'a> WithName<'a> for MotionRequest<'a> {
    fn with_name(self, name: &'a str) -> MotionRequest<'a> {
        MotionRequest { name, ..self }
    }
}
