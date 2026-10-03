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
//! ここで値を決めない。
//!
//! **許容幅の内側であれば、幅の広さによらず位置・速度・加速度の3つを保つ。**
//! 減速の上限を、幅が許す**最短**の間隔（1 stepで使える減速量）と**最長**の間隔
//! （1 stepで進みうる距離）の両方で保守側に見積もるためである。
//! 幅を広げると**違反ではなく遅さ**として現れる。
//! [`ClampReport::acceleration_bound_conceded`]が立つのは**この前提が破れたときだけ**である。
//!
//! # 範囲
//!
//! 含むもの（`servo-safety-limits.md`の`Command処理`の中央3段）:
//!
//! - `motion-name/target validation`と`hard range clamp or rejection`（[`Limiter::admit`]）
//! - `velocity and acceleration limiting`（[`Limiter::step`]）
//! - `state and clamp-counter report`のcounter（[`LimiterCounters`]、[`ClampReport`]）
//! - 同じcommandの再送でmotionを二重に始めない層（[`MotionDedup`]。Issue #19の受け入れ条件4の
//!   うち、この層まで）。**このcrateは履歴を実装しない。**`deskcat-protocol`の`DuplicateHistory`を
//!   使う側である
//!
//! duplicate suppressionは、PR #251のときにはこのcrateの範囲外とし、「`PROTO-TBD-005`
//! （履歴の保持件数・期間）が未確定なうちは、共通の履歴型の抽出に着手しない」としていた。
//! 理由は値を先取りしないことだった。Issue #12で`crates/deskcat-serial`の`DuplicateHistory`が
//! 保持件数と保持期間を`DuplicatePolicy`として呼び出し側から受け取る形になり、その理由は
//! 抽出しても守れるようになった。そこでIssue #19で、`DuplicateHistory`を`deskcat-protocol`へ
//! 移してhostとfirmwareで共用し、このcrateはそれを使う層だけを持つ。**このcrateへ3つ目の
//! 履歴の実装を作らない**ことは変わらない
//!
//! 含まないもの:
//!
//! - `protocol validation`（`crates/deskcat-protocol`）
//! - `calibrated pulse conversion`と`hardware PWM`（calibration値が`TBD`、PWMは実機）
//! - `単位時間あたりの受理数`（`rate_limited`）と`実行中trajectoryの占有`（`busy`）
//! - wireの`play_motion`の受信と振り分け。`play_motion`の型が`deskcat-protocol`にまだ無い
//!   （上限値が`PROTO-TBD-007`／`008`／`009`／`014`で未確定）
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
pub mod dedup;
pub mod limiter;
pub mod limits;

pub use counters::{ClampReport, LimiterCounters};
pub use dedup::{DedupOutcome, MotionDedup};
pub use limiter::{
    AdmittedTarget, CommandSource, Limiter, MotionCatalog, MotionRequest, Rejection, Setpoint,
};
pub use limits::{Cap, ControlPeriod, LimitsError, PositionRange, ServoLimits};
