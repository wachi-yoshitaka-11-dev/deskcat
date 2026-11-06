//! 出力の間引き（Issue #708、ADR-0027）の網羅test。
//!
//! # ここに出てくる数値は test 用である
//!
//! `trajectory.rs`と同じく、`TEST_`で始まる値はtestを走らせるためだけに選んだ値であり、
//! `servo-safety-limits.md`の承認値ではない。**このfileの値を正本へ写さない。**
//!
//! # 何を数えるか
//!
//! `Limiter`は毎step進み、PWMへ渡すのはN stepに1回の位置だけである。servoに届くのは**出力の列**なので、
//! 速度と加速度は**出力の間隔で**数える。窓`k`（前の出力の後から今回の出力まで、間隔`T_k`）の平均速度を
//! `v_k = Σ(v_i·dt_i) / T_k`とする。`v_k`が表す時刻は窓の中心なので、**加速度は窓の中心どうしの間隔で数える**：
//! `|v_k − v_(k−1)| / ((T_k + T_(k−1)) / 2)`（ADR-0027の決定3の「出力の間隔で数えて」の読み。`acceleration_by_mean`）。
//! 間隔が公称で一定なら`T_k`で割った値（`acceleration_by_current`）と同じになり、公称の周期の検査はそれを使う。
//! 間隔が揺れる場合、`T_k`だけで割った値は参考として別に出す。
//!
//! **`v_i`は`Setpoint::velocity()`（`Limiter`の内部状態）を使い、出力した位置の差分から求めない。**位置は
//! f32で、100付近の分解能が約7.6e-6あるため、差分から求めた加速度は丸めで上限を外れる
//! （ADR-0027の観察では、20 msの全stepでも50.011が出た）。内部状態から数えれば丸めが入らない。
//! 代わりに、**出力した位置の差分が内部状態の積分に丸めの範囲で一致すること**を別に検査する
//! （`POSITION_ROUNDING_PER_STEP`）。この2つで、内部状態の数え方が実際の出力から離れていないことを押さえる。

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
const TEST_MAX_STEP: f32 = 8.0;
/// f64で比べる上限。
const TEST_MAX_ACCELERATION: f64 = 40.0;
const TEST_DT_S: f32 = 0.02;
const TEST_MOTION: &str = "test-motion";

/// 1 stepの位置の丸めの上限。|位置|≦40のf32の分解能（2^-18≒3.8e-6）の2倍
/// （`start + velocity*dt`の積と和のそれぞれの丸め）。**制限値ではない。**
const POSITION_ROUNDING_PER_STEP: f64 = 8e-6;

/// f64の累積誤差の許容（相対）。**制限値ではない。**
const F64_SLACK: f64 = 1e-9;

/// 速度のf32の分解能。`TEST_MAX_VELOCITY`（20）は[16, 32)にあり、そこのf32の間隔は2^-19である。
/// `Limiter`は速度をf32で持つので、1 stepの速度差も、窓の平均速度の差も、この丸めを含む。
/// **間引かない（n = 1）でも、`|Δv|/dt`は40.00001と出る**（f32の丸めであり、上限の破れではない）。
/// 窓の差には最大2回分（今回と前回）の丸めを許し、`(|Δv| − 2·VELOCITY_ULP) / T_k`で数える。
const VELOCITY_ULP: f64 = 1.0 / 524_288.0;

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
    acceleration_by_current: f64,
    /// `|v_k − v_(k−1)| / ((T_k + T_(k−1)) / 2)`の最大。
    acceleration_by_mean: f64,
    /// `|v_k|`の最大。
    velocity: f64,
    /// 出力した位置の差分と、内部状態の積分との差の最大（窓内のstep数で割った、1 stepあたりの値）。
    position_mismatch_per_step: f64,
    /// 出力した回数。
    outputs: usize,
}

/// `dt`の選び方。
#[derive(Clone, Copy)]
enum Jitter {
    /// 公称の間隔だけ。
    None,
    /// 許容幅の内側の両端と公称を擬似乱数で選ぶ。
    Within(f32),
}

/// `steps`回`Limiter`を進め、`decimator`が出力するstepで窓を閉じて数える。
fn run(n: u32, offset: u32, jitter: Jitter, seed: u64, switch_every: usize, steps: usize) -> Worst {
    let tolerance = match jitter {
        Jitter::None => 0.0,
        Jitter::Within(t) => t,
    };
    let period = ControlPeriod::new(TEST_DT_S, tolerance).expect("period");
    let mut limiter = Limiter::new(limits(), period, MotionCatalog::new([TEST_MOTION]), 0.0)
        .expect("start inside the range");
    let mut decimator = OutputDecimator::new(n).expect("n >= 1");
    for _ in 0..offset {
        decimator.advance();
    }
    let mut rng = Lcg(seed + 1);
    let mut worst = Worst::default();
    // 窓の途中の積算。
    let (mut integral, mut elapsed, mut window_steps) = (0.0f64, 0.0f64, 0u32);
    let mut start_position = 0.0f64;
    let mut previous: Option<(f64, f64)> = None;
    for index in 0..steps {
        let slot = (index / switch_every) as u64 + seed;
        let target = TARGETS[usize::try_from(slot % TARGETS.len() as u64).expect("< 7")];
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
        let setpoint = limiter.step(admitted, dt).expect("dt is inside the period");
        assert!(!setpoint.clamps().acceleration_bound_conceded);
        integral += f64::from(setpoint.velocity()) * f64::from(dt);
        elapsed += f64::from(dt);
        window_steps += 1;
        if !decimator.advance() {
            continue;
        }
        // 出力する。窓を閉じる。
        let output = f64::from(setpoint.position());
        assert!(
            (f64::from(TEST_APPROVED_MIN)..=f64::from(TEST_APPROVED_MAX)).contains(&output),
            "output {output} left the approved range"
        );
        let mismatch = ((output - start_position) - integral).abs() / f64::from(window_steps);
        worst.position_mismatch_per_step = worst.position_mismatch_per_step.max(mismatch);
        let velocity = integral / elapsed;
        let (previous_velocity, previous_elapsed) = previous.unwrap_or((0.0, elapsed));
        let delta = (velocity - previous_velocity).abs();
        worst.acceleration_by_current = worst
            .acceleration_by_current
            .max((delta - 2.0 * VELOCITY_ULP).max(0.0) / elapsed);
        worst.acceleration_by_mean = worst
            .acceleration_by_mean
            .max(delta / f64::midpoint(elapsed, previous_elapsed));
        worst.velocity = worst.velocity.max(velocity.abs());
        worst.outputs += 1;
        previous = Some((velocity, elapsed));
        start_position = output;
        integral = 0.0;
        elapsed = 0.0;
        window_steps = 0;
    }
    worst
}

#[test]
fn zero_interval_is_refused_and_outputs_fall_on_every_nth_step() {
    assert!(OutputDecimator::new(0).is_none());
    for n in 1u32..=6 {
        let mut decimator = OutputDecimator::new(n).expect("n >= 1");
        assert_eq!(decimator.interval(), n);
        for step in 1..=60u32 {
            assert_eq!(decimator.advance(), step % n == 0, "n {n}, step {step}");
        }
    }
}

/// 公称の間隔だけで、Nと、静止からの動き始めの位相と、再targetの位相を総当たりする。
/// **出力の列の加速度が、どの位相でも上限を超えない**（Issue #708の受け入れ条件）。
#[test]
fn nominal_interval_never_exceeds_the_bounds_for_any_phase() {
    let mut checked = 0usize;
    let mut reached = 0.0f64;
    for n in 1u32..=6 {
        for seed in 0..TARGETS.len() as u64 {
            for switch_every in 1usize..60 {
                let worst = run(n, 0, Jitter::None, seed, switch_every, 400);
                assert!(
                    worst.acceleration_by_current <= TEST_MAX_ACCELERATION * (1.0 + F64_SLACK),
                    "n {n}, seed {seed}, switch {switch_every}: acceleration {}",
                    worst.acceleration_by_current
                );
                assert!(
                    worst.velocity <= f64::from(TEST_MAX_VELOCITY) * (1.0 + F64_SLACK),
                    "n {n}: velocity {}",
                    worst.velocity
                );
                assert!(
                    worst.position_mismatch_per_step <= POSITION_ROUNDING_PER_STEP,
                    "n {n}: output position drifted from the integral by {}",
                    worst.position_mismatch_per_step
                );
                assert!(worst.outputs > 0);
                reached = reached.max(worst.acceleration_by_current);
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

/// 出力の列がすでに途中の位相にある場合（最初の出力までのstep数が1〜N−1）も、上限を超えない。
/// 設計は最初の出力をちょうどNstep目にするが、位相がずれた場合の余裕も押さえる。
#[test]
fn a_shifted_first_window_does_not_exceed_the_bounds_either() {
    for n in 2u32..=6 {
        for offset in 1..n {
            for seed in 0..TARGETS.len() as u64 {
                let worst = run(n, offset, Jitter::None, seed, 17, 400);
                assert!(
                    worst.acceleration_by_current <= TEST_MAX_ACCELERATION * (1.0 + F64_SLACK),
                    "n {n}, offset {offset}, seed {seed}: acceleration {}",
                    worst.acceleration_by_current
                );
            }
        }
    }
}

/// 間隔が許容幅の内側で揺れる場合。**揺れた間隔では`Limiter`自身が、step間の加速度の差を
/// 間隔の平均で割ると上限を超える**（`step`は`最大加速度 × そのstepの間隔`で制限するため）。
/// 間引いた出力が、窓の中心どうしの間隔（2つの窓の間隔の平均）で数えて、間引かない`n = 1`より悪くならないこと
/// （同じ間隔の列で比べる。参考の比較であり、揺れる間隔での上限以下は保証しない）と、速度の上限を守ること、
/// 参考として、今回の窓の間隔`T_k`だけで数えた超過が間隔の最大と最小の比以内であることを確かめる。
#[test]
fn jitter_inside_the_tolerance_is_no_worse_than_the_undecimated_limiter() {
    for tolerance in [0.002f32, 0.01] {
        let mut baseline = 0.0f64;
        let mut by_n = Vec::new();
        // 今回の窓の間隔`T_k`で割った値（公称の周期の検査と同じ数え方）の最大。
        let mut worst_current = 0.0f64;
        let shortest = f64::from(TEST_DT_S - tolerance * 0.9);
        let longest = f64::from(TEST_DT_S + tolerance * 0.9);
        for n in 1u32..=5 {
            let mut worst_mean = 0.0f64;
            for seed in 0..40u64 {
                for switch_every in 3usize..40 {
                    let worst = run(n, 0, Jitter::Within(tolerance), seed, switch_every, 300);
                    assert!(
                        worst.velocity <= f64::from(TEST_MAX_VELOCITY) * (1.0 + F64_SLACK),
                        "tolerance {tolerance}, n {n}: velocity {}",
                        worst.velocity
                    );
                    assert!(
                        worst.position_mismatch_per_step <= POSITION_ROUNDING_PER_STEP,
                        "tolerance {tolerance}, n {n}: position drift"
                    );
                    worst_mean = worst_mean.max(worst.acceleration_by_mean);
                    worst_current = worst_current.max(worst.acceleration_by_current);
                }
            }
            if n == 1 {
                baseline = worst_mean;
            }
            by_n.push(worst_mean);
        }
        // **揺れた間隔では、`T_k`で割ると上限を超えうる**（間引かない`Limiter`は`最大加速度 × そのstepの間隔`で
        // 制限するので超えない。窓の間隔が窓ごとに違うため、窓をまたぐと超える）。超過の大きさは、
        // 間隔の最大と最小の比以内に収まることを、この総当たりの範囲で確かめる（**証明ではない**）。
        assert!(
            worst_current <= TEST_MAX_ACCELERATION * (longest / shortest),
            "tolerance {tolerance}: acceleration by T_k {worst_current} exceeds the interval ratio"
        );
        println!(
            "tolerance {tolerance}: worst acceleration by T_k {worst_current:.3} ({:.3}x), by mean interval {:.3}, undecimated {baseline:.3}",
            worst_current / TEST_MAX_ACCELERATION,
            by_n.iter().copied().fold(0.0f64, f64::max)
        );
        // 参考の比較の前提：`n = 1`（間引かない）でも、窓の中心の間隔で数えると上限を超える。
        assert!(
            baseline > TEST_MAX_ACCELERATION,
            "tolerance {tolerance}: the undecimated baseline {baseline} did not exceed the bound"
        );
        for (index, worst_mean) in by_n.iter().enumerate() {
            assert!(
                *worst_mean <= baseline * (1.0 + F64_SLACK),
                "tolerance {tolerance}, n {}: decimated acceleration {worst_mean} exceeds the undecimated {baseline}",
                index + 1
            );
        }
    }
}
