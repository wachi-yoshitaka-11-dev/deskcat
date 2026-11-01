//! `play_motion`のsession判定と、動作受理前のduplicate判定。
//!
//! Firmwareの`pi_link.rs`とhost testが同じ判定を使う。`admit`は初めての現在sessionの
//! `(sid, id)`だけで呼ばれる。実際のservo出力へ接続する実行側は、`admit`が`Ok`を返す前に
//! motion名以外の`PROTO-TBD-008`の上限とservoの安全制限を検査する必要がある。
//! 現在のfirmwareは空catalogを渡すため、`admit`へ到達しない。
//! 保持件数・期間は`PROTO-TBD-005`、受理budgetとreplay quotaは`PROTO-TBD-012`、
//! command ageの拒否条件は`PROTO-TBD-013`のままである。この型は後二者を判定しない。
//! retired `sid`の保持は`PROTO-TBD-011`であり、firmwareの`PiSession`が扱う。

use deskcat_protocol::{DuplicatePolicy, ErrorCode, PlayMotion};

use crate::{CommandSource, DedupOutcome, MotionCatalog, MotionDedup, MotionRequest, Rejection};

/// Pi sessionと`ESP32` bootの現在状態。firmwareが受信ごとに渡す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionSession {
    current_sid: Option<u32>,
    boot_established: bool,
}

impl MotionSession {
    /// 判定に使う現在のPi `sid`と`ESP32`の`boot`確立状態を作る。
    #[must_use]
    pub const fn new(current_sid: Option<u32>, boot_established: bool) -> Self {
        Self {
            current_sid,
            boot_established,
        }
    }
}

/// Session判定またはduplicate判定を終えた結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionDecision {
    /// 現在のPi sessionでないか、ESP32の`boot`が未確立である。
    StaleSession,
    /// 現在sessionの履歴を照会した結果。
    Dedup(DedupOutcome),
}

impl MotionDecision {
    /// Wireの`ACK`へ載せるerror code。成功なら`None`。
    #[must_use]
    pub const fn code(self) -> Option<ErrorCode> {
        match self {
            Self::StaleSession => Some(ErrorCode::StaleSession),
            Self::Dedup(outcome) => outcome.code(),
        }
    }
}

/// [`MotionDispatch::handle_debug`]が指示を受理しなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DebugRefusal {
    /// ESP32の`boot`がACKされていない（Protocol §4.1の停止状態を含む）。Pi由来の`stale_session`と同じ基準である。
    BootNotEstablished,
    /// catalogの検査または`admit`が拒否した。
    Rejected(Rejection),
}

/// 現在のPi sessionのmotion受理を、session→duplicate→catalog→`admit`の順に扱う。
///
/// `MotionSession`の`current_sid`はfirmwareの`PiSession`が持つ値を毎回渡す。`on_new_session`は
/// `HelloOutcome::Established`のときだけ呼び、旧sessionの履歴を消す。
#[derive(Debug)]
pub struct MotionDispatch {
    dedup: MotionDedup,
    catalog: MotionCatalog,
}

impl MotionDispatch {
    /// 保持件数・期間と、受理可能なmotion名を明示して作る。既定値は持たない。
    #[must_use]
    pub fn new(policy: DuplicatePolicy, catalog: MotionCatalog) -> Self {
        Self {
            dedup: MotionDedup::new(policy),
            catalog,
        }
    }

    /// 新しいPi sessionが確立したとき、旧sessionの結果を破棄する。
    pub fn on_new_session(&mut self) {
        self.dedup.clear();
    }

    /// `admit`を、現在sessionの初回commandで、catalogに名前がある場合だけ呼ぶ。
    ///
    /// `session`の`boot_established`が偽の場合は履歴にも触れず`stale_session`を返す。
    /// `admit`は検査と動作開始を一つの呼び出しとして扱い、失敗も履歴に記録する。
    pub fn handle(
        &mut self,
        session: MotionSession,
        sid: u32,
        id: u32,
        command: &PlayMotion,
        now_ms: u64,
        admit: impl FnOnce(&PlayMotion) -> Result<(), Rejection>,
    ) -> MotionDecision {
        if session.current_sid != Some(sid) || !session.boot_established {
            return MotionDecision::StaleSession;
        }
        let catalog = &self.catalog;
        MotionDecision::Dedup(self.dedup.admit_once(id, now_ms, || {
            if !catalog.contains(&command.name) {
                return Err(Rejection::UnknownMotion);
            }
            admit(command)
        }))
    }

    /// debug由来のmotion指示を、ESP32の`boot`の確立の検査とcatalogの名前の検査の後で`admit`へ渡す
    /// （Issue #19の受け入れ条件3）。
    ///
    /// **`boot_established`とcatalogと`admit`は[`MotionDispatch::handle`]と同じ基準で通る。**
    /// `boot_established`が偽（`boot`がACKされていない間。Protocol §4.1の停止状態を含む）なら、
    /// Pi由来と同じく`admit`へ届かない。`admit`に渡す[`MotionRequest`]の`source`は
    /// [`CommandSource::Debug`]に固定する。`Limiter::admit`は`source`で判定を分けないため
    /// （範囲、clamp、counterはPi由来と同じ）、呼び出し側は、Pi由来と同じlimiter（同じcounter）へ届く
    /// `admit`を渡す。
    ///
    /// **通さない検査は2つだけである。**Piのsession（`current_sid`と、指示の`sid`の一致）と
    /// duplicate（`id`の履歴）は、Pi↔ESP32のwireの規則であり、debugの指示は`sid`も`id`も持たない。
    /// 偽の`sid`で`handle`を通すと、Piのsession規則がdebugの都合でゆがむ。duplicateの履歴にも
    /// 記録しない（Piの再送の判定へ影響しない）。
    ///
    /// # Errors
    ///
    /// `boot`が未確立なら[`DebugRefusal::BootNotEstablished`]、catalogに名前が無ければ
    /// `Rejected(`[`Rejection::UnknownMotion`]`)`（どちらも`admit`は呼ばない）。それ以外は`admit`の結果。
    pub fn handle_debug(
        &self,
        boot_established: bool,
        name: &str,
        target: f32,
        admit: impl FnOnce(&MotionRequest<'_>) -> Result<(), Rejection>,
    ) -> Result<(), DebugRefusal> {
        if !boot_established {
            return Err(DebugRefusal::BootNotEstablished);
        }
        if !self.catalog.contains(name) {
            return Err(DebugRefusal::Rejected(Rejection::UnknownMotion));
        }
        admit(&MotionRequest {
            name,
            target,
            source: CommandSource::Debug,
        })
        .map_err(DebugRefusal::Rejected)
    }
}
