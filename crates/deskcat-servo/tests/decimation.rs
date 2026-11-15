//! 出力の間引き（Issue #708、ADR-0027）の網羅test。
//!
//! # ここに出てくる数値は test 用である
//!
//! `trajectory.rs`と同じく、`TEST_`で始まる値はtestを走らせるためだけに選んだ値であり、
//! `servo-safety-limits.md`の承認値ではない。**このfileの値を正本へ写さない。**
//!
//! # 何を確かめるか
//!
//! firmwareの`LimitedServo::tick`と同じ形を、時計を模擬して再現する。
//!
//! - 各tickの間隔は`dt_tick`（公称±許容の内側で揺れる）。`OutputDecimator`が出力すると答えたtickでだけ、
//!   `Limiter::step`を呼ぶ。`dt`は**最後の出力から今回の出力までの和（窓）**である
//! - `Limiter`の`ControlPeriod`は、各tickの`ControlPeriod`をN倍した窓の許容である
//!
//! 出力の列は`Limiter`が測った間隔（窓）で制限した列そのものなので、**窓の間隔`T_k`で数えた加速度が上限を超えない**
//! （`|v_k − v_(k−1)| / T_k`。`v_k`は`Setpoint::velocity()`、`Limiter`の内部状態）。f32の位置の差分から求めると、
//! 位置の分解能（100付近で約7.6e-6）の丸めで、公称の周期でも上限を外れるため、内部状態で数える。
//! 速度の分解能（`VELOCITY_ULP`）の丸めは引いて比べる。

#![allow(clippy::float_cmp)]

use deskcat_servo::{
    Cap, CommandSource, ControlPeriod, Limiter, MotionCatalog, MotionRequest, OutputDecimator,
    PositionRange, ServoLimits,
};

/// **test用の値。**
const TEST_HARD_MIN: f32 = -40.0;
const TEST_APPROVED_MIN: f32 = -30.0;
const TEST_APPROVED_MAX: f32 = 25.0;
const TEST_HARD_MAX: f32 = 35.0;
const TEST_MAX_VELOCITY: f32 = 20.0;
const TEST_MAX_ACCELERATION_F32: f32 = 40.0;
/// f64で比べる上限。
const TEST_MAX_ACCELERATION: f64 = 40.0;
const TEST_MAX_STEP: f32 = 8.0;
const TEST_DT_S: f32 = 0.02;
const TEST_MOTION: &str = "test-motion";

/// 速度のf32の分解能。`TEST_MAX_VELOCITY`（20）は[16, 32)にあり、そこのf32の間隔は2^-19である。
/// `Limiter`は速度をf32で持つので、速度差はこの丸めを含む。間引かない（n = 1）でも`|Δv|/dt`は40.00001と出る
/// （f32の丸めであり、上限の破れではない）。`(|Δv| − 2·VELOCITY_ULP) / T_k`で数える。
const VELOCITY_ULP: f64 = 1.0 / 524_288.0;

/// f64の累積誤差の許容（相対）。**制限値ではない。**
const F64_SLACK: f64 = 1e-9;

fn limits() -> ServoLimits {
    ServoLimits::new(
        PositionRange::new(
            TEST_HARD_MIN,
            TEST_APPROVED_MIN,
            TEST_APPROVED_MAX,
            TEST_HARD_MAX,
        )
        .expect("ordered"),
        0.0,
        Cap::new(TEST_MAX_VELOCITY, 25.0, "max_velocity").expect("cap"),
        Cap::new(TEST_MAX_ACCELERATION_F32, 50.0, "max_acceleration").expect("cap"),
        Cap::new(TEST_MAX_STEP, 12.0, "max_step").expect("cap"),
    )
    .expect("consistent")
}

const TARGETS: [f32; 7] = [25.0, -30.0, 0.0, 12.5, -7.0, 35.0, -40.0];

/// 決定的な擬似乱数（依存を足さない）。
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

/// 1つの走査の結果。
#[derive(Default)]
struct Worst {
    /// `max(|v_k − v_(k−1)| − 2·VELOCITY_ULP, 0) / T_k`の最大（f32の丸めを引いた値）。
    acceleration: f64,
    /// 参考：`|v_k − v_(k−1)| / ((T_k + T_(k−1)) / 2)`の最大（窓の中心どうしの間隔）。
    acceleration_by_mean: f64,
    /// `|v_k|`の最大。
    velocity: f64,
    /// 出力した回数。
    outputs: usize,
}

/// `dt`の選び方。
#[derive(Clone, Copy)]
enum Jitter {
    /// 公称の間隔だけ。
    None,
    /// 許容幅の内側（許容の90%）の両端と公称を擬似乱数で選ぶ。
    Within(f32),
}

/// `ticks`回のtickを進め、出力するtickで`Limiter`を窓の間隔で進める。
fn run(n: u32, jitter: Jitter, seed: u64, switch_every: usize, ticks: usize) -> Worst {
    let tolerance = match jitter {
        Jitter::None => 0.0,
        Jitter::Within(t) => t,
    };
    // firmwareと同じく、窓の許容は各tickの許容のN倍。
    let window_period = ControlPeriod::new(
        TEST_DT_S * f32::from(u8::try_from(n).expect("small")),
        tolerance * f32::from(u8::try_from(n).expect("small")),
    )
    .expect("window period is valid");
    let mut limiter = Limiter::new(
        limits(),
        window_period,
        MotionCatalog::new([TEST_MOTION]),
        0.0,
    )
    .expect("start inside the range");
    let mut decimator = OutputDecimator::new(n).expect("n >= 1");
    let mut rng = Lcg(seed + 1);
    let mut worst = Worst::default();
    let mut window = 0.0f64;
    let mut previous: Option<(f64, f64)> = None;
    for index in 0..ticks {
        let slot = (index / switch_every) as u64 + seed;
        let target = TARGETS[usize::try_from(slot % TARGETS.len() as u64).expect("< 7")];
        // targetはいつでも受理される（出力しないtickでも）。`admit`は`Limiter`を進めない。
        let admitted = limiter
            .admit(&MotionRequest {
                name: TEST_MOTION,
                target,
                source: CommandSource::Pi,
            })
            .expect("target inside the hard range");
        let dt = match jitter {
            Jitter::None => TEST_DT_S,
            Jitter::Within(t) => match rng.next() % 3 {
                0 => TEST_DT_S - t * 0.9,
                1 => TEST_DT_S + t * 0.9,
                _ => TEST_DT_S,
            },
        };
        window += f64::from(dt);
        if !decimator.advance() {
            continue;
        }
        // 窓の間隔は、各tickの間隔の和をf32へ丸めて渡す（firmwareは`Instant`の差をf32秒にする）。
        #[allow(clippy::cast_possible_truncation)]
        let window_s = window as f32;
        let setpoint = limiter
            .step(admitted, window_s)
            .expect("the window is inside the window period");
        assert!(
            !setpoint.clamps().acceleration_bound_conceded,
            "n {n}: the acceleration bound was conceded"
        );
        let output = f64::from(setpoint.position());
        assert!(
            (f64::from(TEST_APPROVED_MIN)..=f64::from(TEST_APPROVED_MAX)).contains(&output),
            "output {output} left the approved range"
        );
        let elapsed = f64::from(window_s);
        let velocity = f64::from(setpoint.velocity());
        let (previous_velocity, previous_elapsed) = previous.unwrap_or((0.0, elapsed));
        let delta = (velocity - previous_velocity).abs();
        worst.acceleration = worst
            .acceleration
            .max((delta - 2.0 * VELOCITY_ULP).max(0.0) / elapsed);
        worst.acceleration_by_mean = worst
            .acceleration_by_mean
            .max(delta / f64::midpoint(elapsed, previous_elapsed));
        worst.velocity = worst.velocity.max(velocity.abs());
        worst.outputs += 1;
        previous = Some((velocity, elapsed));
        window = 0.0;
    }
    worst
}

#[test]
fn zero_interval_is_refused_and_outputs_fall_on_every_nth_tick() {
    assert!(OutputDecimator::new(0).is_none());
    for n in 1u32..=6 {
        let mut decimator = OutputDecimator::new(n).expect("n >= 1");
        assert_eq!(decimator.interval(), n);
        for tick in 1..=60u32 {
            assert_eq!(decimator.advance(), tick % n == 0, "n {n}, tick {tick}");
        }
    }
}

/// 各tickの間隔が公称で一定のとき。N、動き始めの位相、再targetの位相を総当たりする。
#[test]
fn nominal_interval_never_exceeds_the_bounds_for_any_phase() {
    let mut checked = 0usize;
    let mut reached = 0.0f64;
    for n in 1u32..=6 {
        for seed in 0..TARGETS.len() as u64 {
            for switch_every in 1usize..60 {
                let worst = run(n, Jitter::None, seed, switch_every, 400);
                assert!(
                    worst.acceleration <= TEST_MAX_ACCELERATION * (1.0 + F64_SLACK),
                    "n {n}, seed {seed}, switch {switch_every}: acceleration {}",
                    worst.acceleration
                );
                assert!(
                    worst.velocity <= f64::from(TEST_MAX_VELOCITY) * (1.0 + F64_SLACK),
                    "n {n}: velocity {}",
                    worst.velocity
                );
                assert!(worst.outputs > 0);
                reached = reached.max(worst.acceleration);
                checked += 1;
            }
        }
    }
    assert!(checked > 2000);
    // 総当たりが実際に上限まで加速度を使っていること（上限に届かない列だけでは、上の検査が空になる）。
    assert!(
        reached >= TEST_MAX_ACCELERATION * 0.99,
        "sweepの最大の加速度 {reached} が上限に届いていない"
    );
}

/// **各tickの間隔が許容幅の内側で揺れても、窓の間隔`T_k`で数えた加速度は上限を超えない**
/// （ADR-0027の決定3。`Limiter`は窓の間隔で制限するため）。譲り（`acceleration_bound_conceded`）も立たない。
/// 許容の90%まで揺らす3値（公称・公称±90%）で、N・揺れ幅・seed・再targetの位相を総当たりする。
#[test]
fn jitter_inside_the_tolerance_never_exceeds_the_bounds() {
    let mut reached = 0.0f64;
    for tolerance in [0.002f32, 0.005, 0.01] {
        for n in 1u32..=6 {
            for seed in 0..40u64 {
                for switch_every in 3usize..40 {
                    let worst = run(n, Jitter::Within(tolerance), seed, switch_every, 300);
                    assert!(
                        worst.acceleration <= TEST_MAX_ACCELERATION * (1.0 + F64_SLACK),
                        "tolerance {tolerance}, n {n}, seed {seed}, switch {switch_every}: acceleration {}",
                        worst.acceleration
                    );
                    assert!(
                        worst.velocity <= f64::from(TEST_MAX_VELOCITY) * (1.0 + F64_SLACK),
                        "tolerance {tolerance}, n {n}: velocity {}",
                        worst.velocity
                    );
                    reached = reached.max(worst.acceleration);
                }
            }
        }
    }
    assert!(
        reached >= TEST_MAX_ACCELERATION * 0.99,
        "sweepの最大の加速度 {reached} が上限に届いていない"
    );
}

/// 参考：窓の中心の間隔で数えた加速度。`Limiter`は窓の間隔`T_k`で制限するので、中心の間隔で数えると
/// 上限を超えうる（間引かない`Limiter`も、step自身の間隔で制限するため同じ）。値を出すだけで、上限は主張しない。
#[test]
fn centre_interval_acceleration_is_reported_for_reference() {
    for tolerance in [0.002f32, 0.01] {
        let mut worst_mean = 0.0f64;
        for n in [1u32, 3] {
            for seed in 0..40u64 {
                for switch_every in 3usize..40 {
                    let worst = run(n, Jitter::Within(tolerance), seed, switch_every, 300);
                    worst_mean = worst_mean.max(worst.acceleration_by_mean);
                }
            }
        }
        println!(
            "tolerance {tolerance}: worst acceleration by the centre interval {worst_mean:.3}"
        );
    }
}
