//! Clampとrejectionのcounter（受け入れ条件5）。
//!
//! # Protocol counterではない
//!
//! [`deskcat_protocol::ProtocolCounters`]を流用しない。同型に**clampを数えるfieldは無く**、
//! Protocol §4.6のcounter対応表もclamp用のfieldを定義していない。
//! 追加すればwire schemaの変更になり、正本は`docs/protocol/esp32-pi-protocol.md`である。
//! **このcrateでwire schemaを変えない。**
//!
//! `firmware/esp32/src/health.rs`が`overrun_ticks`などを「Protocol counterではない」と
//! 明記して別に持っているのと同じ形である。
//!
//! # rejectionはwire error codeへ対応づく
//!
//! clampと違い、rejectionはProtocol §5.3・§7が既にcodeを定めている。
//! [`Rejection::code`]がその対応を持つ。ここではcodeごとに分けて数える。
//!
//! # 飽和
//!
//! すべて`u32`で、`saturating_add`で加算する。wrapさせない。wrapすると
//! 「clampが起きていない」と読める値へ戻り、安全側の判断ができなくなる。
//! 幅と飽和時の扱いの正本は`PROTO-TBD-006`であり、**このcrateでは決めない。**
//! ここで`u32`を採るのは[`deskcat_protocol::ProtocolCounters`]の各fieldに揃えるためである。
//!
//! [`Rejection::code`]: crate::Rejection::code

/// 1回のstepで作用した制限。
///
/// `servo-safety-limits.md`の`Command処理`が
/// 「有効なtargetを保守的にclampした場合は、machine-readableなstatusまたはeventで
/// 報告する」と定めている。**その報告の中身がこの型である。**
/// [`LimiterCounters`]が累計、この型が単発のeventに対応する。
// clampの種類ごとに独立したflagを持つ。まとめると「どの制限が効いたか」が
// 報告から落ち、`Command処理`が要求する machine-readable な報告にならない。
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClampReport {
    /// 位置を`承認値`の位置範囲へ収めた。
    ///
    /// 立つ箇所が2つある。**どちらも「位置を承認値の範囲へ収めた」ことを表すので、
    /// 同じflagと同じcounterで数える。**
    ///
    /// - `Limiter::admit`が、hard boundの内側にある要求targetを`承認値`の範囲へ収めたとき
    /// - `Limiter::step`の最後の安全網が、算出した位置を`承認値`の範囲へ収めたとき。
    ///   位置境界へ向けた減速が効いていれば、こちらは作用しない
    pub position: bool,
    /// 1 stepの変化量を`単一commandの最大変化量`へ収めた。
    pub step: bool,
    /// 速度を`最大速度`へ収めた。
    pub velocity: bool,
    /// 位置境界へ向けた減速のために速度を制限した。
    ///
    /// `最大加速度`のまま減速して`承認値`の位置範囲の内側で止まれる速度が上限である。
    /// [`Self::velocity`]と分けているのは、上限の出どころが違うためである
    /// （`最大速度`ではなく、残り距離と`最大加速度`から決まる）。
    pub braking: bool,
    /// 加速度を`最大加速度`へ収めた。
    pub acceleration: bool,
}

impl ClampReport {
    /// 1つでもclampが作用したか。
    #[must_use]
    pub const fn any(self) -> bool {
        self.position || self.step || self.velocity || self.braking || self.acceleration
    }
}

/// Clampとrejectionの累計。
///
/// **module docのとおりProtocol counterではない。**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LimiterCounters {
    /// `承認値`の位置範囲へclampした回数。[`ClampReport::position`]の累計である。
    pub clamped_position: u32,
    /// `単一commandの最大変化量`へclampした回数。
    pub clamped_step: u32,
    /// `最大速度`へclampした回数。
    pub clamped_velocity: u32,
    /// 位置境界へ向けた減速のために速度を制限した回数。
    pub clamped_braking: u32,
    /// `最大加速度`へclampした回数。
    pub clamped_acceleration: u32,
    /// `invalid_payload`でrejectした回数。
    pub rejected_invalid_payload: u32,
    /// `out_of_range`でrejectした回数。
    pub rejected_out_of_range: u32,
}

impl LimiterCounters {
    /// 1回のstepのclampを累計へ加える。
    pub fn record_clamps(&mut self, report: ClampReport) {
        if report.position {
            self.clamped_position = self.clamped_position.saturating_add(1);
        }
        if report.step {
            self.clamped_step = self.clamped_step.saturating_add(1);
        }
        if report.velocity {
            self.clamped_velocity = self.clamped_velocity.saturating_add(1);
        }
        if report.braking {
            self.clamped_braking = self.clamped_braking.saturating_add(1);
        }
        if report.acceleration {
            self.clamped_acceleration = self.clamped_acceleration.saturating_add(1);
        }
    }

    /// clamp counterの合計。飽和加算する。
    #[must_use]
    pub const fn total_clamps(&self) -> u32 {
        self.clamped_position
            .saturating_add(self.clamped_step)
            .saturating_add(self.clamped_velocity)
            .saturating_add(self.clamped_braking)
            .saturating_add(self.clamped_acceleration)
    }

    /// rejection counterの合計。飽和加算する。
    #[must_use]
    pub const fn total_rejections(&self) -> u32 {
        self.rejected_invalid_payload
            .saturating_add(self.rejected_out_of_range)
    }
}
