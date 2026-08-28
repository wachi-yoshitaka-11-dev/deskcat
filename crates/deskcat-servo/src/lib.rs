//! Servoのhard limit強制とtrajectory limiting（Issue #19）。
//!
//! # このcrateは境界値を持たない
//!
//! 可動域、速度、加速度、`単一commandの最大変化量`、`Neutral位置`、受理するmotion名は
//! **すべて呼び出し側から受け取る。**`docs/hardware/servo-safety-limits.md`の
//! `動作制限`表はこれらの行が`TBD`であり、`docs/governance/hardware-safety-policy.md`の
//! 対応表が`サーボPWM、可動域、速度、加速度`を「一次資料または実測」の側へ置いている。
//! 確定は`HW-TBD-010`／`011`／`020`と Issue #18 の calibration である。
//!
//! **[`Default`]を数値付きで実装しない。**[`MotionCatalog::empty`]と
//! [`LimiterCounters`]の`Default`は数値を選んでいない（前者は「何も受理しない」、
//! 後者は全counterが0）。
//!
//! # 呼び出し側の前提
//!
//! **[`Limiter::step`]を呼ぶ間隔は[`ControlPeriod`]で受け取り、範囲外はrejectする。**
//! doc で頼むのではなく検査で強制する。境界に速度を持って着いた状態では、必要な減速量が
//! 固定される一方で1 stepの減速量は`最大加速度 × 間隔`なので、**間隔が縮むと
//! `最大加速度`を守れなくなる**（危険なのは**小さくする**方向である）。
//!
//! 周期も許容変動幅も**呼び出し側から受け取る。**`動作制限`表に制御周期の行は無く、
//! ここで値を決めない。**許容幅が0なら`最大加速度`を譲ることは無い。**幅を広げた場合の
//! 譲りは[`ClampReport::acceleration_bound_conceded`]で観測できる。
//! **位置の bound は、どの場合も破らない。**
//!
//! # 範囲
//!
//! 含むもの（`servo-safety-limits.md`の`Command処理`の中央3段）:
//!
//! - `motion-name/target validation`と`hard range clamp or rejection`（[`Limiter::admit`]）
//! - `velocity and acceleration limiting`（[`Limiter::step`]）
//! - `state and clamp-counter report`のcounter（[`LimiterCounters`]、[`ClampReport`]）
//!
//! 含まないもの:
//!
//! - `protocol validation`（`crates/deskcat-protocol`）
//! - `calibrated pulse conversion`と`hardware PWM`（calibration値が`TBD`、PWMは実機）
//! - `単位時間あたりの受理数`（`rate_limited`）と`実行中trajectoryの占有`（`busy`）
//! - duplicate suppression。**Issue #19 の受け入れ条件4 だが、この作業の範囲外である。**
//!   `PROTO-TBD-005`（履歴の保持件数・期間）が未確定であり、
//!   [`deskcat_protocol::ProtocolCounters::duplicate_expired`]自身が
//!   「発火条件は未確定である。このcounterはそれらの値を先取りしない」と定めている。
//!   **後続のIssueはまだ立っていない**（2026-08-28時点）。想定している形は、
//!   `crates/deskcat-serial`の`BootHistory`と`firmware/esp32`の`processed_hello`から
//!   共通の履歴型を抽出して共用することであり、**このcrateへ3つ目の実装を作らない**
//! - servoの駆動、GPIO、通電に関わる一切
//!
//! # 例
//!
//! 値はすべて呼び出し側が渡す。**次の数値はdoc test用であり、正本の値ではない。**
//!
//! ```
//! use deskcat_servo::{
//!     Cap, CommandSource, ControlPeriod, Limiter, MotionCatalog, MotionRequest, PositionRange,
//!     ServoLimits,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let position = PositionRange::new(-40.0, -30.0, 30.0, 40.0)?;
//! let limits = ServoLimits::new(
//!     position,
//!     0.0,
//!     Cap::new(20.0, 25.0, "max_velocity")?,
//!     Cap::new(40.0, 50.0, "max_acceleration")?,
//!     Cap::new(10.0, 12.0, "max_step")?,
//! )?;
//! // 制御周期と許容幅も呼び出し側が渡す。**この数値もdoc test用である。**
//! let period = ControlPeriod::new(0.02, 0.002)?;
//! let mut limiter = Limiter::new(limits, period, MotionCatalog::new(["doc-test-motion"]), 0.0)?;
//!
//! // hard boundの外はreject。
//! let refused = limiter.admit(&MotionRequest {
//!     name: "doc-test-motion",
//!     target: 100.0,
//!     source: CommandSource::Pi,
//! });
//! assert!(refused.is_err());
//!
//! // `承認値`とhard boundの間はclampして報告する。
//! let admitted = limiter.admit(&MotionRequest {
//!     name: "doc-test-motion",
//!     target: 35.0,
//!     source: CommandSource::Pi,
//! })?;
//! assert!(admitted.clamped());
//! assert_eq!(admitted.target(), 30.0);
//! assert_eq!(limiter.counters().clamped_position, 1);
//! # Ok(())
//! # }
//! ```

pub mod counters;
pub mod limiter;
pub mod limits;

pub use counters::{ClampReport, LimiterCounters};
pub use limiter::{
    AdmittedTarget, CommandSource, Limiter, MotionCatalog, MotionRequest, Rejection, Setpoint,
};
pub use limits::{Cap, ControlPeriod, LimitsError, PositionRange, ServoLimits};
