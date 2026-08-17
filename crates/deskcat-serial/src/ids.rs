//! 送信側の`id`採番（仕様§3、`PROTO-TBD-003`）。
//!
//! 仕様が確定した規則をそのまま実装する。
//!
//! - **`id`をwrapさせない。**wrapは`(sid, id)`の一意性を壊し、§9が
//!   duplicate判定の前提にしている性質を失わせる。
//! - **次に割り当てる新規`id`が`u32`の上限値そのものになった時点で、
//!   その上限値を終端報告のために予約する。**予約した`id`は`protocol_fault`
//!   ちょうど1件にだけ使う。
//! - **予約と払い出しは不可分に行う。**仕様は「採番は単一の点で直列化し、
//!   上限値に達した採番要求は『予約済み』として拒否する」と定める。
//!   ここでは`&mut self`がその直列化そのものである。採番点を複数持たないため、
//!   境界で別の送出元が先に上限値を取ることが起こらない。
//! - **上限値そのものは正当な`id`である。**受信側に予約値の判定を足さない。
//!   予約はこの送信側の内部規則にとどまる。
//!
//! 既に送出したmessageの再送は同じ`(sid, id)`で行うため、新しい`id`を消費しない。
//! したがって上限に達した後もretryは実行でき、止まるのは新しい`(sid, id)`を
//! 要する送出だけである。

/// `id`空間を使い切ったために新しい`(sid, id)`を作れないこと。
///
/// 仕様§3の停止状態に対応する。復帰はprocessの再起動、または運用者の明示的な
/// session resetによる（§3.1）。**このcrateは自動で`sid`を選び直さない。**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdSpaceExhausted;

impl core::fmt::Display for IdSpaceExhausted {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("idの上限に達したため新しい(sid, id)を作れない")
    }
}

impl core::error::Error for IdSpaceExhausted {}

/// 同一session内の`id`を単調増加で採番する。
///
/// 単一の点で直列化するため、`allocate`と`take_terminal`は`&mut self`を取る。
#[derive(Debug)]
pub struct IdAllocator {
    /// 次に払い出す`id`。`None`は終端報告用の`id`も使い切ったことを表す。
    next: Option<u32>,
    /// 終端報告用に予約した`id`がまだ残っているか。
    terminal_available: bool,
}

impl IdAllocator {
    /// 初期値`1`から採番を始める。
    ///
    /// 仕様の例（§3、§5.1）は`id: 1`から始まる`hello`を示している。
    #[must_use]
    pub const fn new() -> Self {
        Self::starting_at(1)
    }

    /// 初期値を指定して採番を始める。
    ///
    /// `sid`を選び直したときは`id`を初期値へ戻す（§3.1）。その用途で使う。
    #[must_use]
    pub const fn starting_at(first: u32) -> Self {
        Self {
            next: Some(first),
            terminal_available: true,
        }
    }

    /// 通常のmessage用に`id`を1つ払い出す。
    ///
    /// 次に払い出す値が`u32::MAX`になった時点で、その値を終端報告のために予約し、
    /// **以降の通常の採番をすべて拒否する。**command、event、`status`、完了event、
    /// fault eventのいずれであっても、新しい`(sid, id)`を要するものはここを通る。
    ///
    /// # Errors
    ///
    /// 予約に達している場合に[`IdSpaceExhausted`]を返す。
    pub fn allocate(&mut self) -> Result<u32, IdSpaceExhausted> {
        match self.next {
            Some(id) if id < u32::MAX => {
                self.next = Some(id + 1);
                Ok(id)
            }
            // `u32::MAX`は終端報告用に予約済みである。通常の採番へは払い出さない。
            _ => Err(IdSpaceExhausted),
        }
    }

    /// 終端報告用に予約した`id`を、消費せずに覗く。
    ///
    /// 送出の準備（encodeやqueueへの投入）が失敗しうる場合、[`Self::take_terminal`]を
    /// 先に呼ぶと**予約を失ったまま送れない**状態になる。予約は1件しかないため
    /// 取り戻せない。準備が成功してから消費する。
    #[must_use]
    pub const fn peek_terminal(&self) -> Option<u32> {
        if self.terminal_available {
            self.next
        } else {
            None
        }
    }

    /// 終端報告（`protocol_fault`）用に予約した`id`を取り出す。
    ///
    /// **ちょうど1件にだけ使う。**2回目以降は`None`を返す。
    pub fn take_terminal(&mut self) -> Option<u32> {
        if !self.terminal_available {
            return None;
        }
        let id = self.next?;
        self.terminal_available = false;
        self.next = None;
        Some(id)
    }

    /// 通常の採番が停止しているか。
    ///
    /// `true`のとき、新しい`(sid, id)`を要する送出は行えない。既存の
    /// `(sid, id)`による再送は影響を受けない。
    #[must_use]
    pub const fn is_exhausted(&self) -> bool {
        matches!(self.next, None | Some(u32::MAX))
    }

    /// 終端報告用の`id`がまだ残っているか。
    #[must_use]
    pub const fn terminal_available(&self) -> bool {
        self.terminal_available
    }
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_monotonically_from_one() {
        let mut ids = IdAllocator::new();
        assert_eq!(ids.allocate(), Ok(1));
        assert_eq!(ids.allocate(), Ok(2));
        assert_eq!(ids.allocate(), Ok(3));
    }

    /// 上限値は通常の採番へ払い出さない。仕様が「予約」と定めた点である。
    #[test]
    fn the_maximum_id_is_reserved_and_never_allocated_as_an_ordinary_id() {
        let mut ids = IdAllocator::starting_at(u32::MAX - 1);
        assert_eq!(ids.allocate(), Ok(u32::MAX - 1));
        assert!(ids.is_exhausted(), "次の採番は上限値であり予約済みである");
        assert_eq!(ids.allocate(), Err(IdSpaceExhausted));
    }

    /// 終端報告は上限値ちょうど1件で行う。
    #[test]
    fn the_terminal_report_uses_the_reserved_maximum_exactly_once() {
        let mut ids = IdAllocator::starting_at(u32::MAX);
        assert_eq!(ids.allocate(), Err(IdSpaceExhausted));
        assert_eq!(ids.take_terminal(), Some(u32::MAX));
        assert_eq!(ids.take_terminal(), None, "2件目は無い");
        assert!(!ids.terminal_available());
    }

    /// 停止後も通常の採番は再開しない。wrapさせないことの帰結である。
    #[test]
    fn allocation_never_wraps_back_to_the_start() {
        let mut ids = IdAllocator::starting_at(u32::MAX);
        assert_eq!(ids.allocate(), Err(IdSpaceExhausted));
        let _ = ids.take_terminal();
        for _ in 0..3 {
            assert_eq!(
                ids.allocate(),
                Err(IdSpaceExhausted),
                "0や1へ戻ってはならない"
            );
        }
    }

    /// `sid`を選び直したときは`id`を初期値へ戻す（§3.1）。
    #[test]
    fn a_new_session_restarts_the_id_sequence() {
        let mut ids = IdAllocator::starting_at(u32::MAX);
        assert!(ids.allocate().is_err());

        let mut ids = IdAllocator::new();
        assert_eq!(ids.allocate(), Ok(1), "新しいsessionでは初期値から始まる");
    }
}
