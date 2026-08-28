//! Hard limitの強制とtrajectory limiting。
//!
//! # 処理の順序
//!
//! [`servo-safety-limits.md`の`Command処理`]が順序を定めている。**変えない。**
//!
//! ```text
//! received command
//!   → protocol validation
//!   → motion-name/target validation
//!   → hard range clamp or rejection
//!   → velocity and acceleration limiting
//!   → calibrated pulse conversion
//!   → hardware PWM
//!   → state and clamp-counter report
//! ```
//!
//! この moduleが担うのは中央の3段である。
//!
//! - `motion-name/target validation`と`hard range clamp or rejection` → [`Limiter::admit`]
//! - `velocity and acceleration limiting` → [`Limiter::step`]
//! - `state and clamp-counter report`のcounter → [`Limiter::counters`]と[`Setpoint::clamps`]
//!
//! **含まないもの。**`protocol validation`（`crates/deskcat-protocol`）、
//! `calibrated pulse conversion`（calibration値が`HW-TBD-010`で未確定）、
//! `hardware PWM`（実機）。
//!
//! **`単位時間あたりの受理数`と`実行中trajectoryによる占有`も含まない。**
//! Protocol §5.3 はそれぞれ`rate_limited`と`busy`を返すと定めているが、
//! `動作制限`表の`秒あたり受理motion command数`は`HW-TBD-020`で未確定であり、
//! trajectoryの実行管理はこの型の範囲外である。
//!
//! **正規化speedとrepeat countの上限も含まない。**Protocol §5.3 が上限を要求しているが、
//! 値の持ち主が`動作制限`表に無く、受理するmotion名自体が同§で`TBD`である。
//!
//! # clampとrejectの分かれ目は推論である
//!
//! `Command処理`節は「構造的に不正または明らかに危険なcommandは、clampよりrejectを優先する。
//! 有効なtargetを保守的にclampした場合は、machine-readableなstatusまたはeventで報告する」
//! とだけ定める。**境目の定義は書かれていない。**
//!
//! この型は`動作制限`表が`承認値`と`設定可能なhard bound`を**別の列**として持つ構造から、
//! **hard boundの外＝「明らかに危険」→reject、`承認値`とhard boundの間＝「有効なtargetを
//! 保守的に」→clampして報告**と読んでいる。**これは実装側の推論であり、
//! `servo-safety-limits.md`にそう書いてあるわけではない。**
//! 正本が境目を明示したら、この doc commentごと差し替える。
//!
//! # 境界への近づき方も推論である
//!
//! **正本は「境界へどう近づくか」を定めていない。**`Command処理`節が定めるのは処理の順序と
//! clampよりrejectを優先することだけであり、`動作制限`表が持つのは上限の値（すべて`TBD`）だけである。
//!
//! [`Limiter::step`]が使う減速則——`承認値`の位置範囲の端へ向かう速度を、
//! 「いま`最大加速度`で減速を始めれば残り距離の内側で止まれる速度」で抑える——は、
//! `最大加速度`と`承認値`の位置範囲から導いたものだが、**その具体形は実装側の選択である。**
//! とくに次の2点は正本に根拠を持たない。
//!
//! - 連続時間の `sqrt(2*a*d)` を使わず、**1 stepで変えられる速度 `a*dt` の整数倍へ量子化**したこと。
//!   連続時間の式は離散時間では緩すぎ、最後のstepが境界を越える。越えた分を位置clampが
//!   切り落とすと、その減速が`最大加速度`を超える
//! - 残り距離が `a*dt^2` 未満のときだけ `distance/dt` で詰めること。これが無いと
//!   targetの手前で止まったまま着かない
//!
//! **導かれるのは「減速が要る」ことまでであり、この式そのものではない。**
//! 正本が接近の仕方を定めたら、**この実装ごと差し替える。**
//!
//! [`servo-safety-limits.md`の`Command処理`]: https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/docs/hardware/servo-safety-limits.md#command処理

use core::fmt;

use deskcat_protocol::ErrorCode;

use crate::counters::{ClampReport, LimiterCounters};
use crate::limits::{ControlPeriod, LimitsError, ServoLimits};

/// Commandの出所。
///
/// **制限の判断でこの値を読まない。**`AGENTS.md`が
/// 「サーボ安全制限をデバッグ経路からも迂回させない」と定め、
/// `servo-safety-limits.md`の`確定しているproject規則`が
/// `AIが生成したcommandやdebug commandでもhard limitを迂回できない`と定めている。
/// 出所は報告のために持ち回るだけである（受け入れ条件3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandSource {
    /// Raspberry Piからのcommand。
    Pi,
    /// Debug経路（local console、test hook等）からのcommand。
    Debug,
}

/// 受理するmotion名の集合。
///
/// **既定の集合を持たない。**Protocol §5.3 が「初期に受け入れるmotion名は、
/// サーボ機構のcalibrationが完了するまで`TBD`とする」と定めているためである。
/// [`MotionCatalog::empty`]は何も受理しない。**motion名が未確定な間の安全側の状態がこれである。**
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MotionCatalog {
    names: Vec<String>,
}

impl MotionCatalog {
    /// 何も受理しない空のcatalog。
    #[must_use]
    pub const fn empty() -> Self {
        Self { names: Vec::new() }
    }

    /// 受理する名前を列挙して作る。
    pub fn new<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            names: names.into_iter().map(Into::into).collect(),
        }
    }

    /// この名前を受理するか。
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.names.iter().any(|known| known == name)
    }

    /// 何も受理しない状態か。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Commandをrejectした理由。
///
/// wire error codeへの対応は[`Rejection::code`]が持つ。対応の正本は
/// Protocol §5.3 の表と §7 の「単一lineの検証で決まるcodeの対応付け」である。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Rejection {
    /// [`MotionCatalog`]に無いmotion名。
    ///
    /// §7 が「列挙外の値」を`invalid_payload`としている。
    UnknownMotion,
    /// targetが有限でない（NaN、±∞）。構造的に不正である。
    NonFiniteTarget,
    /// 経過時間が有限でない、または正でない。構造的に不正である。
    NonPositiveInterval,
    /// 呼び出し間隔が[`ControlPeriod`]の許容範囲の外にある。
    ///
    /// §5.3 が「値そのものが許容範囲外」を`out_of_range`としており、§7 が
    /// `invalid_payload`（型と必須fieldの問題）と`out_of_range`（型は正しいが値が
    /// 範囲外）を分けている。**間隔は型としては正しい正の秒数であり、範囲の側で外れる。**
    /// したがって`out_of_range`である。**新しいcodeは作っていない。**
    IntervalOutsideControlPeriod,
    /// targetが`設定可能なhard bound`の外にある。
    ///
    /// §5.3 が「値そのものが許容範囲外」を`out_of_range`としている。
    TargetOutOfHardRange,
}

impl Rejection {
    /// 相手へ返す[`ErrorCode`]。
    #[must_use]
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::UnknownMotion | Self::NonFiniteTarget | Self::NonPositiveInterval => {
                ErrorCode::InvalidPayload
            }
            Self::TargetOutOfHardRange | Self::IntervalOutsideControlPeriod => {
                ErrorCode::OutOfRange
            }
        }
    }
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::UnknownMotion => "motion name is not in the catalog",
            Self::NonFiniteTarget => "target is not finite",
            Self::NonPositiveInterval => "interval is not a positive, finite number of seconds",
            Self::TargetOutOfHardRange => "target lies outside the configurable hard bound",
            Self::IntervalOutsideControlPeriod => {
                "interval lies outside the configured control period"
            }
        };
        write!(f, "{reason} ({})", self.code().as_str())
    }
}

impl std::error::Error for Rejection {}

/// [`Limiter::admit`]を通ったtarget。
///
/// **この型を呼び出し側で作れない。**fieldはprivateであり、公開constructorも無い。
/// [`Limiter::step`]はこの型しか受け取らないため、`admit`を飛ばしてtrajectoryを
/// 進める経路が存在しない（受け入れ条件3）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdmittedTarget {
    target: f32,
    source: CommandSource,
    clamped: bool,
}

impl AdmittedTarget {
    /// `承認値`の範囲へ収めたあとのtarget。
    #[must_use]
    pub const fn target(self) -> f32 {
        self.target
    }

    /// Commandの出所。
    #[must_use]
    pub const fn source(self) -> CommandSource {
        self.source
    }

    /// 受理にあたってtargetをclampしたか。
    #[must_use]
    pub const fn clamped(self) -> bool {
        self.clamped
    }
}

/// Motion command 1件。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionRequest<'a> {
    /// Motion名。[`MotionCatalog`]で検証する。
    pub name: &'a str,
    /// 目標位置（command単位）。
    ///
    /// **名前付きmotionからこの値を解決するのは呼び出し側である。**
    /// 解決表はcalibration（`HW-TBD-010`）と受理するmotion名（Protocol §5.3）が
    /// 未確定なため、このcrateが持たない。
    pub target: f32,
    /// Commandの出所。**制限の判断では読まない。**
    pub source: CommandSource,
}

/// 1 step分の出力。
///
/// **この型を呼び出し側で作れない。**[`Limiter::step`]だけが作る。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Setpoint {
    position: f32,
    velocity: f32,
    source: CommandSource,
    clamps: ClampReport,
}

impl Setpoint {
    /// このstepの終端位置（command単位）。
    ///
    /// **`承認値`の位置範囲の内側であることが型の契約である。**
    /// `承認値`の範囲は`設定可能なhard bound`の内側なので、hard boundも超えない
    /// （受け入れ条件2）。
    #[must_use]
    pub const fn position(self) -> f32 {
        self.position
    }

    /// このstepの終端速度（command単位毎秒）。
    ///
    /// 大きさが`最大速度`の`承認値`を超えないことが型の契約である。
    #[must_use]
    pub const fn velocity(self) -> f32 {
        self.velocity
    }

    /// Commandの出所。
    #[must_use]
    pub const fn source(self) -> CommandSource {
        self.source
    }

    /// このstepで作用したclamp。machine-readableな報告に使う。
    #[must_use]
    pub const fn clamps(self) -> ClampReport {
        self.clamps
    }
}

/// Servoのhard limitを強制するlimiter。
///
/// **唯一の入口が[`Limiter::admit`]である。**module docの「clampとrejectの分かれ目は推論である」
/// も参照する。
#[derive(Debug, Clone)]
pub struct Limiter {
    limits: ServoLimits,
    period: ControlPeriod,
    catalog: MotionCatalog,
    position: f32,
    velocity: f32,
    counters: LimiterCounters,
}

impl Limiter {
    /// 制限、受理するmotion名、開始位置から作る。
    ///
    /// `initial_position`を呼び出し側から受け取るのは、**起動時のservo位置が
    /// firmwareから分からない**ためである。`servo-safety-limits.md`の
    /// `起動時とdriver故障時の動作`は`HW-TBD-019`で未確定であり、同節は
    /// 「承認されるまで、安全状態は『未検証の動作出力を行わない』とする」としている。
    /// **既定の開始位置を選ばない。**
    ///
    /// # Errors
    ///
    /// `initial_position`が有限でない場合、または`承認値`の位置範囲の外にある場合に失敗する。
    pub fn new(
        limits: ServoLimits,
        period: ControlPeriod,
        catalog: MotionCatalog,
        initial_position: f32,
    ) -> Result<Self, LimitsError> {
        if !initial_position.is_finite() {
            return Err(LimitsError::NotFinite {
                field: "initial_position",
            });
        }
        let range = limits.position();
        if initial_position < range.approved_min() || initial_position > range.approved_max() {
            return Err(LimitsError::StartOutsideApprovedRange);
        }
        Ok(Self {
            limits,
            period,
            catalog,
            position: initial_position,
            velocity: 0.0,
            counters: LimiterCounters::default(),
        })
    }

    /// 累計counter（受け入れ条件5）。
    #[must_use]
    pub const fn counters(&self) -> LimiterCounters {
        self.counters
    }

    /// 現在位置（command単位）。
    #[must_use]
    pub const fn position(&self) -> f32 {
        self.position
    }

    /// 現在速度（command単位毎秒）。
    #[must_use]
    pub const fn velocity(&self) -> f32 {
        self.velocity
    }

    /// 適用中の制限。
    #[must_use]
    pub const fn limits(&self) -> ServoLimits {
        self.limits
    }

    /// 適用中の制御周期。
    #[must_use]
    pub const fn period(&self) -> ControlPeriod {
        self.period
    }

    /// `motion-name/target validation`と`hard range clamp or rejection`。
    ///
    /// **[`CommandSource`]で分岐しない。**Pi由来でもdebug由来でも同じ判定を通る
    /// （受け入れ条件3）。
    ///
    /// # Errors
    ///
    /// motion名がcatalogに無い、targetが有限でない、またはtargetが
    /// `設定可能なhard bound`の外にある場合にrejectする。
    pub fn admit(&mut self, request: &MotionRequest<'_>) -> Result<AdmittedTarget, Rejection> {
        if !self.catalog.contains(request.name) {
            return Err(self.reject(Rejection::UnknownMotion));
        }
        if !request.target.is_finite() {
            return Err(self.reject(Rejection::NonFiniteTarget));
        }
        let range = self.limits.position();
        if !range.within_hard(request.target) {
            return Err(self.reject(Rejection::TargetOutOfHardRange));
        }
        let clamped_target = range.clamp_to_approved(request.target);
        // 「clampが作用したか」の判定なので、厳密比較が正しい。近似で比べると、
        // 値を変えていないのにclampしたと報告する。
        #[allow(clippy::float_cmp)]
        let clamped = clamped_target != request.target;
        if clamped {
            self.counters.record_clamps(ClampReport {
                position: true,
                ..ClampReport::default()
            });
        }
        Ok(AdmittedTarget {
            target: clamped_target,
            source: request.source,
            clamped,
        })
    }

    /// `velocity and acceleration limiting`を`dt_s`秒分だけ進める。
    ///
    /// 適用順は`単一commandの最大変化量` → `最大速度` → 位置境界へ向けた減速 →
    /// `最大加速度` → `承認値`の位置範囲である。
    ///
    /// 減速を`最大加速度`より前に置くのは、境界に達したstepで位置clampが速度を
    /// 一気に落とし`最大加速度`を超えるのを防ぐためである。最後の位置範囲は安全網であり、
    /// 減速が効いている限り作用しない。
    ///
    /// # `dt_s`は[`ControlPeriod`]の範囲内でなければならない
    ///
    /// **範囲外の間隔はrejectする。**doc で「安定させること」と頼むのではなく、型と検査で
    /// 強制する（`AGENTS.md`「サーボ安全制限をデバッグ経路からも迂回させない」）。
    ///
    /// 境界に速度を持って着いた状態では、**必要な減速量は持っている速度で固定される**一方、
    /// 1 stepで使える減速量は`最大加速度 × dt_s`である。**`dt_s`が縮むと後者だけが縮む。**
    /// 契約を持たずに任意の間隔を受けると、ここで`最大加速度`が破れた
    /// （`10.0`の次に`0.02`で加速度125、`0.001`で2500。逆に大きくした場合は40のまま。
    /// `tests/trajectory.rs`のtest値で上限は40。**正本の値ではない**）。
    ///
    /// # 許容幅の内側では3つのboundをすべて保つ
    ///
    /// 減速の上限を、契約が許す**最短**の間隔（1 stepで確実に使える減速量）と
    /// **最長**の間隔（1 stepで進みうる距離）で見積もる。**両方を保守側に倒す必要がある。**
    /// 離散の停止距離は`v²/(2a) + v·dt/2`で`dt`とともに伸びるため、最短側だけでは
    /// 距離の見積もりが楽観側に残る。
    ///
    /// これにより、実際の間隔が許容幅のどこに来ても見積もりより楽になり、
    /// **境界へ速度を持ち越さず、位置clampが発火せず、`最大加速度`も破れない。**
    /// 幅11通り×49,440 stepの総当たりで、位置の逸脱0、譲り0回だった。
    ///
    /// **代償は速さである。**幅を広げるほど境界への接近が保守的になる。
    /// **違反ではなく遅さとして現れる。**
    ///
    /// [`ClampReport::acceleration_bound_conceded`]が立つのは
    /// **この見積もりの前提が破れたときだけである**（同 flag の doc を参照）。
    /// その場合でも譲るのは加速度の側だけで、**位置の bound は破らない。**
    ///
    /// # Errors
    ///
    /// `dt_s`が有限でない、正でない、または[`ControlPeriod`]の許容範囲の外にある場合に
    /// rejectする。
    pub fn step(&mut self, admitted: AdmittedTarget, dt_s: f32) -> Result<Setpoint, Rejection> {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return Err(self.reject(Rejection::NonPositiveInterval));
        }
        if !self.period.accepts(dt_s) {
            return Err(self.reject(Rejection::IntervalOutsideControlPeriod));
        }

        let mut clamps = ClampReport::default();
        let start = self.position;
        let range = self.limits.position();

        // `単一commandの最大変化量`。
        let max_step = self.limits.max_step().approved();
        let mut delta = admitted.target - start;
        if delta.abs() > max_step {
            delta = delta.signum() * max_step;
            clamps.step = true;
        }

        // `最大速度`。
        let max_velocity = self.limits.max_velocity().approved();
        let mut velocity = delta / dt_s;
        if velocity.abs() > max_velocity {
            velocity = velocity.signum() * max_velocity;
            clamps.velocity = true;
        }

        // 位置境界へ向けた減速。
        //
        // これを入れないと、`承認値`の位置範囲の端に達したstepで最後の位置clampが
        // 速度を一気に落とし、**`最大加速度`を超える**。受け入れ条件2が
        // 「Position、velocity、acceleration が hard bound を超えない」を要求しているため、
        // 端へ着いてから止まるのでは足りず、着く前に減速を始める必要がある。
        //
        // 上限は「いま`最大加速度`で減速を始めれば残り距離の内側で止まれる速度」である。
        // **新しい制限値を持ち込んでいない。**`最大加速度`と`承認値`の位置範囲だけから決まる。
        let max_braking = self.braking_velocity(start, velocity, dt_s);
        if velocity.abs() > max_braking {
            velocity = velocity.signum() * max_braking;
            clamps.braking = true;
        }

        // `最大加速度`。
        let max_delta_v = self.limits.max_acceleration().approved() * dt_s;
        let delta_v = velocity - self.velocity;
        if delta_v.abs() > max_delta_v {
            velocity = self.velocity + delta_v.signum() * max_delta_v;
            clamps.acceleration = true;
        }

        // `承認値`の位置範囲。ここを最後に当てる。
        let unclamped_position = start + velocity * dt_s;
        let position = range.clamp_to_approved(unclamped_position);
        // 上と同じ理由で厳密比較を使う。
        #[allow(clippy::float_cmp)]
        let position_clamped = position != unclamped_position;
        if position_clamped {
            clamps.position = true;
            // 位置をclampしたら速度も実際の移動量と整合させる。
            velocity = (position - start) / dt_s;

            // **ここだけが`最大加速度`を破りうる経路である。**上の加速度clampを通った速度を
            // 位置clampが切り落とすため、切り落とし分が`最大加速度`を超えることがある。
            // 境界ではこれ以上位置を進められないので、**位置の bound を優先する以外に無い。**
            // **握りつぶさず、譲ったことを数える**（`dt_s`の項に発生条件がある）。
            let realized = (velocity - self.velocity).abs();
            if realized > max_delta_v {
                clamps.acceleration_bound_conceded = true;
            }
        }

        self.position = position;
        self.velocity = velocity;
        self.counters.record_clamps(clamps);

        Ok(Setpoint {
            position,
            velocity,
            source: admitted.source,
            clamps,
        })
    }

    /// 進行方向の`承認値`の境界までに、`最大加速度`で止まりきれる速度の上限。
    ///
    /// # 連続時間の `sqrt(2*a*d)` を使わない理由
    ///
    /// 連続時間の式は離散時間では**緩すぎる。**1 stepが`dt`秒の刻みで減速すると、
    /// 最後のstepが境界をわずかに越え、そこで位置clampが速度を切り落とす。
    /// その切り落としは`最大加速度`を超える減速になる
    /// （`tests/trajectory.rs`のtest値で 45 対 上限 40。**正本の値ではない**）。
    ///
    /// # 使う式
    ///
    /// 1 stepで変えられる速度を `unit = a*dt` とすると、速度 `n*unit` から
    /// 毎step `unit` ずつ落として止まるまでに進む距離は
    /// `a*dt^2 * n*(n+1)/2` である。残り距離 `d` に収まる最大の整数 `n` を採り、
    /// 上限を `n*unit` とする。
    ///
    /// `n` を整数へ落とすことで、減速中の各stepの速度差がちょうど `unit` になり、
    /// `最大加速度`のclampが減速側で作用しなくなる。境界も越えない
    /// （`n*unit*dt <= d` が上の不等式から従う）。
    ///
    /// `n` が丸めで1つ小さく出ても安全側である。上限が1 step分きつくなるだけで、
    /// `最大加速度`のclampが不足分を埋める。
    ///
    /// `direction`は符号だけを使う。0 のときは動かないので上限を課さない。
    fn braking_velocity(&self, position: f32, direction: f32, dt_s: f32) -> f32 {
        if direction == 0.0 {
            return f32::INFINITY;
        }
        let range = self.limits.position();
        let distance = if direction > 0.0 {
            range.approved_max() - position
        } else {
            position - range.approved_min()
        };
        let distance = distance.max(0.0);

        let acceleration = self.limits.max_acceleration().approved();
        // 1 stepで確実に使える減速量は、契約が許す**最小**の間隔で決まる。
        let unit = acceleration * self.period.shortest_s();
        if unit <= 0.0 {
            return 0.0;
        }
        // 1 stepで進みうる距離は、契約が許す**最大**の間隔で決まる。
        let quantum = unit * self.period.longest_s();
        let steps = (((1.0 + 8.0 * distance / quantum).sqrt() - 1.0) / 2.0).floor();
        let ramp = steps.max(0.0) * unit;

        // 今回の step で境界を越えないための上限は、実際の間隔で決まる。
        let approach = (distance / dt_s).min(unit);
        ramp.max(approach)
    }

    fn reject(&mut self, rejection: Rejection) -> Rejection {
        match rejection.code() {
            ErrorCode::OutOfRange => {
                self.counters.rejected_out_of_range =
                    self.counters.rejected_out_of_range.saturating_add(1);
            }
            _ => {
                self.counters.rejected_invalid_payload =
                    self.counters.rejected_invalid_payload.saturating_add(1);
            }
        }
        rejection
    }
}
