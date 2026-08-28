//! Limiterへ渡す動作制限の入力。
//!
//! # この moduleは境界値を1つも持たない
//!
//! 正本は[`docs/hardware/servo-safety-limits.md`]の`動作制限`表であり、同表は
//! 12行のうち`最大連続電流`を除く全行が`承認値 TBD`／`設定可能なhard bound TBD`である
//! （その`最大連続電流`も「**firmwareへ直接は設定しない**」と同表が定めている）。
//! [Hardware Safety Policy]の対応表は`サーボPWM、可動域、速度、加速度`を
//! 「一次資料または実測」の側に置いており、**一般値で開始してよい側ではない。**
//!
//! したがってこの moduleが持つのは**型と検査だけ**である。値は呼び出し側が渡す。
//! **[`Default`]を実装しない。**実装すると数値を選ぶことになる。
//!
//! # `承認値`と`設定可能なhard bound`を別に持つ理由
//!
//! `動作制限`表が2列を別に持っているためである。[`Limiter`]はこの2列を
//! [`servo-safety-limits.md`の`Command処理`]が定める
//! 「構造的に不正または明らかに危険なcommandは、clampよりrejectを優先する」の
//! 分かれ目に使う。**hard boundの外はreject、`承認値`とhard boundの間はclampして報告**である。
//! **この対応付けは同文書が明示していない読みである。**文書が定めているのは優先順位と
//! 報告義務だけであり、境目の定義は書かれていない。変える場合はこの doc commentも変える。
//!
//! # 単位
//!
//! **この moduleは単位を決めない。**位置は「command単位」、速度は「command単位毎秒」、
//! 加速度は「command単位毎秒毎秒」として扱い、**degreeともpulse widthとも解釈しない。**
//! `calibrated pulse conversion`（`Command処理`の後段）はこのcrateの範囲外であり、
//! calibration値（`HW-TBD-010`）が未確定なため単位を確定できない。
//!
//! [`Limiter`]: crate::Limiter
//! [`docs/hardware/servo-safety-limits.md`]: https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/docs/hardware/servo-safety-limits.md
//! [Hardware Safety Policy]: https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/docs/governance/hardware-safety-policy.md
//! [`servo-safety-limits.md`の`Command処理`]: https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/docs/hardware/servo-safety-limits.md#command処理

use core::fmt;

/// 制限の組み立てに失敗した理由。
///
/// **値の妥当性そのものは判定しない。**判定できるのは、渡された値どうしの整合
/// （有限であること、大小関係、符号）だけである。「その値が安全か」は実測
/// （`HW-TBD-010`／`011`／`020`）が決めることであり、この型は答えを持たない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LimitsError {
    /// 有限でない値（NaN、±∞）を渡された。
    NotFinite {
        /// 対象のfield名。
        field: &'static str,
    },
    /// 負の値を渡された。上限は大きさであり、負にならない。
    Negative {
        /// 対象のfield名。
        field: &'static str,
    },
    /// `承認値`が`設定可能なhard bound`を超えている。
    ///
    /// `承認値`はhard boundの内側にある保守的な運用値であり、外側に置けない。
    ApprovedExceedsHard {
        /// 対象のfield名。
        field: &'static str,
    },
    /// 位置範囲の大小関係が
    /// `hard_min <= approved_min <= approved_max <= hard_max`を満たさない。
    RangeOutOfOrder,
    /// `Neutral位置`が`承認値`の位置範囲の外にある。
    NeutralOutsideApprovedRange,
    /// 制御周期または許容変動幅が、正の有限な秒数として成り立たない。
    ///
    /// 許容変動幅は0以上で、制御周期より小さくなければならない。等しいか大きいと
    /// 受理する間隔の下限が0以下になり、間隔の検査が意味を失う。
    InvalidControlPeriod,
    /// 開始位置が`承認値`の位置範囲の外にある。
    ///
    /// [`NeutralOutsideApprovedRange`]と分けている。呼び出し側が直せるのは
    /// 別々の入力であり、同じcodeで返すとどちらを直せばよいか分からない。
    ///
    /// [`NeutralOutsideApprovedRange`]: Self::NeutralOutsideApprovedRange
    StartOutsideApprovedRange,
}

impl fmt::Display for LimitsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite { field } => write!(f, "`{field}` must be finite"),
            Self::Negative { field } => write!(f, "`{field}` must not be negative"),
            Self::ApprovedExceedsHard { field } => {
                write!(f, "`{field}` approved value exceeds its hard bound")
            }
            Self::RangeOutOfOrder => f.write_str(
                "position range must satisfy hard_min <= approved_min <= approved_max <= hard_max",
            ),
            Self::NeutralOutsideApprovedRange => {
                f.write_str("neutral position lies outside the approved position range")
            }
            Self::StartOutsideApprovedRange => {
                f.write_str("start position lies outside the approved position range")
            }
            Self::InvalidControlPeriod => f.write_str(
                "control period must be positive and finite, with a tolerance in [0, period)",
            ),
        }
    }
}

impl std::error::Error for LimitsError {}

/// 上限1つ分の、`承認値`と`設定可能なhard bound`。
///
/// 大きさの上限（最大速度、最大加速度、単一commandの最大変化量など）に使う。
/// 符号を持たないため、`0 <= approved <= hard`を要求する。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cap {
    approved: f32,
    hard: f32,
}

impl Cap {
    /// `承認値`と`設定可能なhard bound`から作る。
    ///
    /// `field`は失敗したときに[`LimitsError`]へ載せる名前である。どの入力を直せばよいかを
    /// 呼び出し側へ返すためだけに使い、検査そのものには影響しない。
    ///
    /// # Errors
    ///
    /// 有限でない、負である、または`承認値`がhard boundを超える場合に失敗する。
    pub fn new(approved: f32, hard: f32, field: &'static str) -> Result<Self, LimitsError> {
        if !approved.is_finite() || !hard.is_finite() {
            return Err(LimitsError::NotFinite { field });
        }
        if approved < 0.0 || hard < 0.0 {
            return Err(LimitsError::Negative { field });
        }
        if approved > hard {
            return Err(LimitsError::ApprovedExceedsHard { field });
        }
        Ok(Self { approved, hard })
    }

    /// 保守的な運用値。clampの行き先である。
    #[must_use]
    pub const fn approved(self) -> f32 {
        self.approved
    }

    /// 設定可能なhard bound。これを超える要求はrejectする。
    #[must_use]
    pub const fn hard(self) -> f32 {
        self.hard
    }
}

/// 位置の範囲。`承認値`側とhard bound側の2組を持つ。
///
/// `動作制限`表の`最小位置`／`最大位置`／`最大command範囲`に対応する。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionRange {
    hard_min: f32,
    approved_min: f32,
    approved_max: f32,
    hard_max: f32,
}

impl PositionRange {
    /// 4つの境界から作る。
    ///
    /// 引数は数直線上の並び順、すなわち
    /// `hard_min` → `approved_min` → `approved_max` → `hard_max` で渡す。
    /// **「外側2つ、内側2つ」の順ではない。**`承認値`の範囲がhard boundの範囲に
    /// 内包されることが不変条件である。
    ///
    /// # Errors
    ///
    /// 有限でない値がある場合、または
    /// `hard_min <= approved_min <= approved_max <= hard_max`を満たさない場合に失敗する。
    pub fn new(
        hard_min: f32,
        approved_min: f32,
        approved_max: f32,
        hard_max: f32,
    ) -> Result<Self, LimitsError> {
        for (value, field) in [
            (hard_min, "position.hard_min"),
            (approved_min, "position.approved_min"),
            (approved_max, "position.approved_max"),
            (hard_max, "position.hard_max"),
        ] {
            if !value.is_finite() {
                return Err(LimitsError::NotFinite { field });
            }
        }
        if !(hard_min <= approved_min && approved_min <= approved_max && approved_max <= hard_max) {
            return Err(LimitsError::RangeOutOfOrder);
        }
        Ok(Self {
            hard_min,
            approved_min,
            approved_max,
            hard_max,
        })
    }

    /// hard boundの下限。
    #[must_use]
    pub const fn hard_min(self) -> f32 {
        self.hard_min
    }

    /// hard boundの上限。
    #[must_use]
    pub const fn hard_max(self) -> f32 {
        self.hard_max
    }

    /// `承認値`側の下限。clampの行き先である。
    #[must_use]
    pub const fn approved_min(self) -> f32 {
        self.approved_min
    }

    /// `承認値`側の上限。clampの行き先である。
    #[must_use]
    pub const fn approved_max(self) -> f32 {
        self.approved_max
    }

    /// hard boundの内側にあるか。境界上は内側として扱う。
    #[must_use]
    pub fn within_hard(self, value: f32) -> bool {
        value >= self.hard_min && value <= self.hard_max
    }

    /// `承認値`の範囲へ収める。
    ///
    /// [`f32::clamp`]は`min > max`と非有限のboundでpanicするが、
    /// [`PositionRange::new`]が有限性と`approved_min <= approved_max`を保証するため、
    /// この呼び出しはpanicしない。
    #[must_use]
    pub fn clamp_to_approved(self, value: f32) -> f32 {
        value.clamp(self.approved_min, self.approved_max)
    }
}

/// [`Limiter`]が強制する制限一式。
///
/// `動作制限`表の行との対応は次のとおりである。**値はここに無い。**
///
/// - `position` — `最小位置`／`最大位置`／`最大command範囲`（同表は「最小位置と最大位置で決まる」としている）
/// - `neutral` — `Neutral位置`
/// - `max_velocity` — `最大速度`
/// - `max_acceleration` — `最大加速度`
/// - `max_step` — `単一commandの最大変化量`
///
/// 同表の残りの行（`最大連続電流`、`最大連続動作時間`、`最大duty cycle`、
/// `秒あたり受理motion command数`、`Command timeout`）は**この型が持たない。**
/// 時間と電流に関わる強制はこのcrateの範囲外である。
///
/// [`Limiter`]: crate::Limiter
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServoLimits {
    position: PositionRange,
    neutral: f32,
    max_velocity: Cap,
    max_acceleration: Cap,
    max_step: Cap,
}

impl ServoLimits {
    /// 制限一式を組み立てる。
    ///
    /// `neutral`は`Neutral位置`（`HW-TBD-010`）である。`承認値`の位置範囲の内側を要求する。
    /// hard boundの内側で足りるとはしない。neutralはfail-safeの行き先の候補であり、
    /// 保守的な運用範囲の外に置くと、安全側へ倒す動作が運用範囲を出る。
    ///
    /// # Errors
    ///
    /// `neutral`が有限でない場合、または`承認値`の位置範囲の外にある場合に失敗する。
    pub fn new(
        position: PositionRange,
        neutral: f32,
        max_velocity: Cap,
        max_acceleration: Cap,
        max_step: Cap,
    ) -> Result<Self, LimitsError> {
        if !neutral.is_finite() {
            return Err(LimitsError::NotFinite { field: "neutral" });
        }
        if neutral < position.approved_min() || neutral > position.approved_max() {
            return Err(LimitsError::NeutralOutsideApprovedRange);
        }
        Ok(Self {
            position,
            neutral,
            max_velocity,
            max_acceleration,
            max_step,
        })
    }

    /// 位置の範囲。
    #[must_use]
    pub const fn position(self) -> PositionRange {
        self.position
    }

    /// `Neutral位置`。
    #[must_use]
    pub const fn neutral(self) -> f32 {
        self.neutral
    }

    /// `最大速度`。
    #[must_use]
    pub const fn max_velocity(self) -> Cap {
        self.max_velocity
    }

    /// `最大加速度`。
    #[must_use]
    pub const fn max_acceleration(self) -> Cap {
        self.max_acceleration
    }

    /// `単一commandの最大変化量`。
    #[must_use]
    pub const fn max_step(self) -> Cap {
        self.max_step
    }
}

/// 制御周期と、その許容変動幅。
///
/// # なぜ型で持つのか
///
/// [`Limiter::step`]が守れる保証は、**呼び出される間隔が安定していることに依存する。**
/// 境界に速度を持って着いた状態では、必要な減速量は持っている速度で固定される一方、
/// 1 stepで使える減速量は`最大加速度 × 間隔`である。**間隔が縮むと後者だけが縮み、
/// `最大加速度`を守れなくなる。**
///
/// doc へ「安定させること」と書くだけでは、呼び出し側が守らなければ破れる。
/// `AGENTS.md`は「サーボ安全制限をデバッグ経路からも迂回させない」と定めており、
/// **迂回できる強制を残さないために型で受け取り、範囲外の間隔をrejectする。**
///
/// # 値はここで決めない
///
/// `docs/hardware/servo-safety-limits.md`の`動作制限`表に制御周期の行は無い。
/// **既定値を持たず、[`Default`]も実装しない。**周期も許容幅も呼び出し側が渡す。
///
/// # 許容変動幅について
///
/// **0にできるが、実際の制御loopにはjitterがある。**厳密一致でrejectすると正常な系で
/// servoが止まるため、呼び出し側が実測に基づく幅を渡せるようにしてある。
///
/// **幅を広げても`最大加速度`は破れない。**[`Limiter::step`]は減速の上限を、
/// 幅が許す**最短**の間隔（1 stepで確実に使える減速量）と**最長**の間隔
/// （1 stepで進みうる距離）で見積もる。実際の間隔が幅のどこに来ても見積もりより楽になる。
/// 幅11通り×49,440 stepの総当たりで、譲りは1度も起きなかった。
///
/// **代償は速さである。**幅を広げるほど見積もりが保守的になり、境界への接近が遅くなる。
/// 幅を周期の9割にしたtestでは、到達に模擬5.0秒かかった（`tests/trajectory.rs`のtest値。
/// **正本の値ではない**）。**違反ではなく遅さとして現れるので、実測のjitterに見合う
/// 最小の幅を渡すこと。**
///
/// [`Limiter::step`]: crate::Limiter::step
/// [`ClampReport::acceleration_bound_conceded`]: crate::ClampReport::acceleration_bound_conceded
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlPeriod {
    nominal_s: f32,
    tolerance_s: f32,
}

impl ControlPeriod {
    /// 公称の制御周期（秒）と許容変動幅（秒）から作る。
    ///
    /// # Errors
    ///
    /// 有限でない、`nominal_s`が正でない、`tolerance_s`が負、または
    /// `tolerance_s >= nominal_s`の場合に失敗する。
    pub fn new(nominal_s: f32, tolerance_s: f32) -> Result<Self, LimitsError> {
        if !nominal_s.is_finite() || !tolerance_s.is_finite() {
            return Err(LimitsError::InvalidControlPeriod);
        }
        if nominal_s <= 0.0 || tolerance_s < 0.0 || tolerance_s >= nominal_s {
            return Err(LimitsError::InvalidControlPeriod);
        }
        Ok(Self {
            nominal_s,
            tolerance_s,
        })
    }

    /// 公称の制御周期（秒）。
    #[must_use]
    pub const fn nominal_s(self) -> f32 {
        self.nominal_s
    }

    /// 許容変動幅（秒）。
    #[must_use]
    pub const fn tolerance_s(self) -> f32 {
        self.tolerance_s
    }

    /// 契約が許す最短の間隔（秒）。減速で確実に使える`最大加速度 × 間隔`を決める。
    #[must_use]
    pub fn shortest_s(self) -> f32 {
        self.nominal_s - self.tolerance_s
    }

    /// 契約が許す最長の間隔（秒）。1 stepで進みうる距離を決める。
    #[must_use]
    pub fn longest_s(self) -> f32 {
        self.nominal_s + self.tolerance_s
    }

    /// この間隔を受理するか。境界上は受理する。
    #[must_use]
    pub fn accepts(self, dt_s: f32) -> bool {
        dt_s.is_finite() && (dt_s - self.nominal_s).abs() <= self.tolerance_s
    }
}
