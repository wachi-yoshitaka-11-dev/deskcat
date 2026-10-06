//! 同じcommandの再送でmotionを二重に始めない層（Issue #19の受け入れ条件4のうち、この層まで）。
//!
//! # 範囲
//!
//! [`MotionDedup::admit_once`]は、現在のsessionの`id`ごとに、`admit`（hostでは
//! [`crate::Limiter::admit`]、firmwareでは`LimitedServo::admit`）を**高々1回**しか呼ばない。
//!
//! - 初めての`id`（[`Lookup::New`]）: `admit`を1回呼び、結果を**`Ok`でも`Err`でも**記録する
//! - 処理済みの`id`（[`Lookup::Replay`]）: `admit`を呼ばず、記録した結果を返す
//! - 履歴から失われた`id`（[`Lookup::Expired`]）: `admit`を呼ばず、`duplicate_expired`で拒否する
//!
//! 根拠はProtocol §9の「ESP32は直近に処理した`(sid, id)`とresultを保持する」
//! 「Duplicateには…保持したresultのreplay経路を使い、非idempotentな動作を再実行しない」
//! 「Relativeまたは名前付きの物理motionにはduplicate suppressionが必要である」である。
//!
//! **wireの`play_motion`の受信と振り分けは含まない。**`play_motion`の型が
//! `deskcat-protocol`にまだ無い（理由は`deskcat-protocol`の`message`のmodule docにある）。
//! firmwareのsessionへの組み込みは、その型が決まった後に行う。
//!
//! `Expired`の扱いは[`DuplicateHistory`]のdocとProtocol §9による（受け入れ前の`TBD`が残っている）。
//!
//! # 拒否も記録する
//!
//! 記録しないと、再送のたびに`admit`が走り、rejection counterを二重に数える。
//!
//! [`crate::Limiter::admit`]が返す拒否（`UnknownMotion`、`NonFiniteTarget`、`TargetOutOfHardRange`）は、どれも最終的な拒否であり、記録する。§8が記録しないと
//! 定めるのは`hello`／`boot`の`rate_limited`だけで（手順9）、motionはそれに当たらない。
//! `busy`を記録するかは§8に定めが無く、ここでは扱わない。
//!
//! # 所有者との分担
//!
//! keyは`id`だけである。**`sid`が現在のsessionであることの照合と、session遷移を確定した
//! ときの[`MotionDedup::clear`]は、所有者（firmwareのsession）が行う**
//! （[`DuplicateHistory`]のdocと同じ分担。§8手順8、§9）。retiredな`sid`は照会の前に
//! `stale_session`で拒否されるので、ここへは届かない。
//!
//! # 時刻
//!
//! `now_ms`は単調に増加する値を渡す（[`DuplicateHistory::record`]の契約）。firmwareでは
//! `std::time::Instant`（`CLOCK_MONOTONIC`）から取る前提である（firmware側の呼び出しはまだ無い）。
//! 時刻が戻った場合の振る舞いは、[`DuplicateHistory::record`]のdocと`tests/dedup.rs`のtest 9による。
//!
//! # 値を持たない
//!
//! 保持件数と保持期間（`PROTO-TBD-005`）は[`DuplicatePolicy`]として呼び出し側が渡す。
//! **defaultを持たない。**

use deskcat_protocol::{DuplicateHistory, DuplicatePolicy, ErrorCode, Lookup};

use crate::limiter::Rejection;

/// [`MotionDedup::admit_once`]の結果。
///
/// **`#[non_exhaustive]`にしない。**[`Lookup`]と同じ理由である。別のcrate（firmware）の
/// `match`が網羅的であることを保ち、種類を足したときにcompileで止めるためである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupOutcome {
    /// 初めての`id`だった。`admit`を1回呼び、その結果を記録した。
    Admitted(Result<(), Rejection>),
    /// 処理済みの`id`だった。`admit`を呼ばず、記録した結果を返した。
    Replayed(Result<(), Rejection>),
    /// 履歴から失われた`id`だった。`admit`を呼ばずに拒否した（`duplicate_expired`）。
    Expired,
}

impl DedupOutcome {
    /// 相手へ返すerror code。成功なら`None`。
    #[must_use]
    pub const fn code(self) -> Option<ErrorCode> {
        match self {
            Self::Admitted(Ok(())) | Self::Replayed(Ok(())) => None,
            Self::Admitted(Err(rejection)) | Self::Replayed(Err(rejection)) => {
                Some(rejection.code())
            }
            Self::Expired => Some(ErrorCode::DuplicateExpired),
        }
    }
}

/// 現在のsessionのmotion commandの重複を判定し、`admit`を高々1回だけ呼ぶ（module doc参照）。
#[derive(Debug)]
pub struct MotionDedup {
    history: DuplicateHistory<Result<(), Rejection>>,
}

impl MotionDedup {
    /// 保持件数と保持期間を受け取って作る。**defaultを持たない。**
    #[must_use]
    pub fn new(policy: DuplicatePolicy) -> Self {
        Self {
            history: DuplicateHistory::new(policy),
        }
    }

    /// `id`が初めてなら`admit`を1回だけ呼び、結果を記録する（module doc参照）。
    ///
    /// `admit`が値（`Limiter::admit`の[`crate::AdmittedTarget`]など）を返す場合は、
    /// 呼び出し側がclosureの中で受け取る。**この層は結果の`Ok`／`Err`だけを記録する。**
    /// Replayのときに、その値を返す手段が無いためである。
    pub fn admit_once(
        &mut self,
        id: u32,
        now_ms: u64,
        admit: impl FnOnce() -> Result<(), Rejection>,
    ) -> DedupOutcome {
        match self.history.lookup(id, now_ms) {
            Lookup::New => {
                let result = admit();
                self.history.record(id, result, now_ms);
                DedupOutcome::Admitted(result)
            }
            Lookup::Replay(result) => DedupOutcome::Replayed(*result),
            Lookup::Expired => DedupOutcome::Expired,
        }
    }

    /// 履歴を破棄する。**所有者が、異なる`sid`へのsession遷移を確定したときだけ呼ぶ**（§9）。
    pub fn clear(&mut self) {
        self.history.clear();
    }
}
