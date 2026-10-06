//! 現在sessionのduplicate履歴（§8手順8、§9、`PROTO-TBD-005`）。
//!
//! ACKを要する要求を受けた側が、同じ`(sid, id)`の再送で処理を二度実行しないために持つ。
//! 保持件数と保持期間は[`DuplicatePolicy`]として呼び出し側から受け取り、
//! **このmoduleは値を決めない**（同型のdoc参照）。
//!
//! # このcrateに置く理由
//!
//! 当初は`crates/deskcat-serial`（host専用）に置いていた（Issue #12）。Issue #19で、host
//! （`deskcat-serial`）とfirmware（`deskcat-servo`経由）の両方が同じ実装を使うために、
//! 両方が既に依っているこのcrateへ移した。**履歴の振る舞いは変えていない。**変えたのは、
//! crateの境界をまたぐために要った2点だけである（[`Lookup`]から`#[non_exhaustive]`を外した、
//! [`DuplicatePolicy::new`]の拒否を[`DuplicatePolicyError`]で返す）。`deskcat-serial`は
//! `deskcat_serial::duplicate`ごとre-exportし、以前のpathを保っている。
//! 値を呼び出し側から受け取る形（Issue #12）のまま移したので、`PROTO-TBD-005`の値を
//! 先取りしない。
//!
//! # `sid`を持たない理由
//!
//! 履歴は**現在sessionの分だけ**を持つ（§8手順8「duplicate照会は現在のsessionだけを対象とする」）。
//! したがってkeyは`id`だけで足りる。**`(sid, id)`の組で判定する責務は所有者にある。**
//! 所有者は、envelopeの`sid`が現在sessionであることを確かめてから[`DuplicateHistory::lookup`]を
//! 呼び、異なる`sid`へのsession遷移を確定したときに[`DuplicateHistory::clear`]を呼ぶ
//! （`deskcat_serial::PeerSession`がそうしている）。retiredな`sid`は照会の前に`stale_session`で
//! 拒否されるため（§8手順8）、ここへは届かない。
//!
//! # 履歴から失われたduplicate
//!
//! 保持件数を超えて捨てたentry、または保持期間を過ぎたentryへの再送は
//! [`Lookup::Expired`]になり、**新しい要求として実行しない**（§13の`PROTO-TBD-005`行
//! 「evictしたentryへの再送は新規commandとして実行しない（`duplicate_expired`で拒否する）」）。
//!
//! 判定は、これまでに記録した最大`id`以下で履歴に無い`id`を「処理したが失われた」と
//! みなす近似である。§3が`id`を同一session内で単調増加と定めているため、正規の送信側では
//! 成り立つ。**未処理の`id`が最大`id`より後に届いた場合も`Expired`になる**（例えば、送信側が
//! `id` 5の初回送出を失い、`id` 6が処理された後で5を再送した場合）。実行しない側へ倒れる
//! 誤判定であり、二重実行は起こさない。

use std::collections::{HashMap, VecDeque};

use core::num::NonZeroUsize;
use core::time::Duration;

/// [`DuplicatePolicy::new`]が拒否した理由。
///
/// **panicにしない。**呼び出し側から渡る値であり、分類して返す（`AGENTS.md`の
/// 「エラーを握りつぶさず、分類、ログ、カウンタを用意する」）。種類は、
/// `crates/deskcat-serial`の`ConfigError`の同名の2つと同じである（そちらは
/// `From<DuplicatePolicyError>`で変換する）。
///
/// **`#[non_exhaustive]`にしない。**種類を足したときに、`deskcat-serial`の変換の`match`が
/// compileで止まり、対応する`ConfigError`の種類を足し忘れないようにするためである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicatePolicyError {
    /// duplicate履歴の保持件数が0である。**0にすると、処理した直後の再送も
    /// 履歴に無く、保持した結果をreplayできない。**
    ZeroDuplicateCapacity,
    /// duplicate履歴の保持期間が0である。0件と同じく、記録した結果が即座に失われる。
    ZeroDuplicateRetention,
}

impl core::fmt::Display for DuplicatePolicyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::ZeroDuplicateCapacity => "duplicate履歴の保持件数が0である",
            Self::ZeroDuplicateRetention => "duplicate履歴の保持期間が0である",
        };
        f.write_str(text)
    }
}

impl core::error::Error for DuplicatePolicyError {}

/// 現在sessionのduplicate履歴の保持件数と保持期間（§9、`PROTO-TBD-005`）。
///
/// **既定値を持たない。**`provisional()`も用意しない。`PROTO-TBD-005`は保持期間、
/// retry window、保持件数の上限のいずれも未確定であり、仮の値もこのcrateでは決めない。
/// 呼び出し側が値と、その値を選んだ根拠を持つ。
///
/// 期間は§13の`PROTO-TBD-005`行が定める下限（遅延messageの最大生存時間＋再送window）を
/// 下回らないことが要求されているが、**この型はその下限を検査しない。**下限を構成する
/// 2つの値自体が未確定だからである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicatePolicy {
    capacity: NonZeroUsize,
    retention: Duration,
}

impl DuplicatePolicy {
    /// 保持件数と保持期間を指定して方針を作る。
    ///
    /// # Errors
    ///
    /// `capacity`が0なら[`DuplicatePolicyError::ZeroDuplicateCapacity`]、`retention`が0なら
    /// [`DuplicatePolicyError::ZeroDuplicateRetention`]を返す。
    pub fn new(capacity: usize, retention: Duration) -> Result<Self, DuplicatePolicyError> {
        let capacity =
            NonZeroUsize::new(capacity).ok_or(DuplicatePolicyError::ZeroDuplicateCapacity)?;
        if retention.is_zero() {
            return Err(DuplicatePolicyError::ZeroDuplicateRetention);
        }
        Ok(Self {
            capacity,
            retention,
        })
    }

    /// 保持する件数の上限。超えたら最も古いentryを捨てる。
    #[must_use]
    pub const fn capacity(&self) -> NonZeroUsize {
        self.capacity
    }

    /// 記録してから保持する期間。
    #[must_use]
    pub const fn retention(&self) -> Duration {
        self.retention
    }
}

/// [`DuplicateHistory::lookup`]の結果。
///
/// **`#[non_exhaustive]`にしない。**別のcrate（`deskcat-serial`、`deskcat-servo`）の`match`が
/// 網羅的であることを保ち、種類を足したときにcompileで止めるためである。`_`の腕で受けると、
/// 新しい種類を黙って「実行する」側か「実行しない」側へ倒すことになる。`deskcat-serial`に
/// あった間は同じcrateの中の`match`だけだったので、付いていても網羅的に書けた。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookup<'a, R> {
    /// 未処理の`id`である。処理してよい。処理したら[`DuplicateHistory::record`]で結果を残す。
    New,
    /// 処理済みである。**再実行せず**、保持した結果を返す（§9）。
    Replay(&'a R),
    /// 処理済みだが、保持件数または保持期間の上限で結果が失われた。
    /// **再実行しない。**`duplicate_expired`で拒否する（§7、`PROTO-TBD-005`）。
    Expired,
}

/// 記録した結果と、記録した時刻。
#[derive(Debug, Clone)]
struct Entry<R> {
    result: R,
    recorded_at_ms: u64,
}

/// 現在sessionのduplicate履歴。
///
/// `R`は保持する結果の型である（`boot`ならPiが返した[`crate::Ack`]）。
#[derive(Debug, Clone)]
pub struct DuplicateHistory<R> {
    policy: DuplicatePolicy,
    entries: HashMap<u32, Entry<R>>,
    /// 記録した順の`id`。保持件数と保持期間の両方で、先頭（最古）から捨てる。
    order: VecDeque<u32>,
    /// これまでに記録した最大`id`。履歴から消えた後も「処理済み」を判定するために残す。
    /// [`Self::clear`]だけが消す。
    highest_recorded: Option<u32>,
}

impl<R> DuplicateHistory<R> {
    /// 空の履歴を作る。
    #[must_use]
    pub fn new(policy: DuplicatePolicy) -> Self {
        Self {
            policy,
            entries: HashMap::new(),
            order: VecDeque::new(),
            highest_recorded: None,
        }
    }

    /// 保持件数と保持期間。
    #[must_use]
    pub const fn policy(&self) -> &DuplicatePolicy {
        &self.policy
    }

    /// 保持しているentryの件数。
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// entryを1件も保持していないか。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn is_past_retention(&self, recorded_at_ms: u64, now_ms: u64) -> bool {
        Duration::from_millis(now_ms.saturating_sub(recorded_at_ms)) >= self.policy.retention()
    }

    /// `id`が処理済みかを照会する。**非破壊である**（§8手順8）。履歴を読むだけで、
    /// 保持期間を過ぎたentryもここでは捨てない（捨てるのは[`Self::record`]）。
    ///
    /// `now_ms`は[`Self::record`]へ渡した時刻と同じ時計（受信側のuptime）の値を渡す。
    /// 経過時間が保持期間以上なら、entryが残っていても[`Lookup::Expired`]を返す。
    #[must_use]
    pub fn lookup(&self, id: u32, now_ms: u64) -> Lookup<'_, R> {
        match self.entries.get(&id) {
            Some(entry) if !self.is_past_retention(entry.recorded_at_ms, now_ms) => {
                Lookup::Replay(&entry.result)
            }
            Some(_) => Lookup::Expired,
            None if self.highest_recorded.is_some_and(|highest| id <= highest) => Lookup::Expired,
            None => Lookup::New,
        }
    }

    /// 処理した`id`の結果を記録する。
    ///
    /// 先に保持期間を過ぎたentryを捨て、記録後に保持件数を超えた分を古い順に捨てる。
    /// 既に記録済みの`id`なら結果と時刻を置き換える（順序は変えない）。
    ///
    /// `now_ms`は単調に増加する値を渡す。巻き戻すと、古いentryが保持期間内に見える。
    pub fn record(&mut self, id: u32, result: R, now_ms: u64) {
        while let Some(&oldest) = self.order.front() {
            let expired = self
                .entries
                .get(&oldest)
                .is_none_or(|entry| self.is_past_retention(entry.recorded_at_ms, now_ms));
            if !expired {
                break;
            }
            self.order.pop_front();
            self.entries.remove(&oldest);
        }

        let entry = Entry {
            result,
            recorded_at_ms: now_ms,
        };
        if self.entries.insert(id, entry).is_none() {
            self.order.push_back(id);
        }
        while self.order.len() > self.policy.capacity().get() {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.highest_recorded = Some(self.highest_recorded.map_or(id, |h| h.max(id)));
    }

    /// 履歴を破棄する。**異なる`sid`へのsession遷移を確定したときだけ呼ぶ**（§9）。
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.highest_recorded = None;
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::{DuplicateHistory, DuplicatePolicy, DuplicatePolicyError, Lookup};

    fn history(capacity: usize, retention_ms: u64) -> DuplicateHistory<&'static str> {
        DuplicateHistory::new(
            DuplicatePolicy::new(capacity, Duration::from_millis(retention_ms)).expect("0ではない"),
        )
    }

    /// 保持件数・保持期間の0は、どちらも記録した結果を即座に失う設定であり、受け付けない。
    #[test]
    fn a_duplicate_policy_rejects_zero_capacity_and_zero_retention() {
        assert_eq!(
            DuplicatePolicy::new(0, Duration::from_secs(1)),
            Err(DuplicatePolicyError::ZeroDuplicateCapacity)
        );
        assert_eq!(
            DuplicatePolicy::new(1, Duration::ZERO),
            Err(DuplicatePolicyError::ZeroDuplicateRetention)
        );
        assert!(DuplicatePolicy::new(1, Duration::from_millis(1)).is_ok());
    }

    /// 同じ`id`の再送は、保持した結果のreplayになる。処理は1回だけである。
    #[test]
    fn a_repeated_id_replays_the_recorded_result_instead_of_running_again() {
        let mut history = history(4, 1_000);
        let mut executed = 0;

        for attempt in 0..3_u64 {
            match history.lookup(7, attempt * 10) {
                Lookup::New => {
                    executed += 1;
                    history.record(7, "ok", attempt * 10);
                }
                Lookup::Replay(result) => assert_eq!(*result, "ok"),
                Lookup::Expired => panic!("保持期間内である"),
            }
        }

        assert_eq!(executed, 1, "二度目以降は実行しない");
    }

    /// 照会は非破壊である。保持期間を過ぎても、照会だけではentryを捨てない。
    #[test]
    fn lookup_does_not_mutate_the_history() {
        let mut history = history(4, 100);
        history.record(1, "ok", 0);

        assert_eq!(history.lookup(1, 100), Lookup::Expired);
        assert_eq!(history.len(), 1, "照会では捨てない");
    }

    /// 保持件数を超えて捨てた`id`は`Expired`であり、`New`へ戻らない。
    #[test]
    fn an_id_evicted_by_capacity_is_expired_not_new() {
        let mut history = history(2, 1_000);
        history.record(1, "a", 0);
        history.record(2, "b", 0);
        history.record(3, "c", 0);

        assert_eq!(history.len(), 2);
        assert_eq!(history.lookup(1, 0), Lookup::Expired);
        assert_eq!(history.lookup(3, 0), Lookup::Replay(&"c"));
        assert_eq!(history.lookup(4, 0), Lookup::New);
    }

    /// 保持期間を過ぎた`id`は`Expired`であり、次の記録で捨てられた後も`New`へ戻らない。
    #[test]
    fn an_id_past_retention_is_expired_and_stays_expired_after_pruning() {
        let mut history = history(4, 100);
        history.record(1, "a", 0);

        assert_eq!(history.lookup(1, 99), Lookup::Replay(&"a"));
        assert_eq!(history.lookup(1, 100), Lookup::Expired);

        history.record(2, "b", 150);
        assert_eq!(history.len(), 1, "保持期間を過ぎた1は記録の時点で捨てる");
        assert_eq!(history.lookup(1, 150), Lookup::Expired);
    }

    /// session遷移で破棄した後は、同じ`id`も`New`になる（新しいsessionの`id`空間）。
    #[test]
    fn clear_forgets_the_highest_recorded_id() {
        let mut history = history(4, 1_000);
        history.record(5, "a", 0);
        history.clear();

        assert!(history.is_empty());
        assert_eq!(history.lookup(5, 0), Lookup::New);
        assert_eq!(history.lookup(1, 0), Lookup::New);
    }
}
