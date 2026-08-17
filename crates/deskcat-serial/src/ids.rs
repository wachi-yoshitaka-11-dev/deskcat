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
/// 単一の点で直列化するため、状態を変える操作は`&mut self`を取る。
#[derive(Debug)]
pub struct IdAllocator {
    /// 次に払い出す`id`。`None`は終端報告用の`id`も使い切ったことを表す。
    next: Option<u32>,
    /// 終端報告用の`id`を既に取り出したか。
    terminal_taken: bool,
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
            terminal_taken: false,
        }
    }

    /// 次に払い出す通常の`id`を、消費せずに覗く。
    ///
    /// 送出の準備（encodeやqueueへの投入）が失敗しうる場合、先に消費すると
    /// **失敗のたびに`id`が減り、上限へ早く到達する。**仕様は`id`の単調増加を
    /// 求めるが連続は求めないため、飛びは問題にならない。**枯渇が問題である。**
    /// 準備が成功してから[`Self::commit`]で消費する。
    ///
    /// # Errors
    ///
    /// 上限値が終端報告用に予約されている場合に[`IdSpaceExhausted`]を返す。
    pub const fn peek_next(&self) -> Result<u32, IdSpaceExhausted> {
        match self.next {
            Some(id) if id < u32::MAX => Ok(id),
            // `u32::MAX`は終端報告用に予約済みである。通常の採番へは払い出さない。
            _ => Err(IdSpaceExhausted),
        }
    }

    /// [`Self::peek_next`]で覗いた`id`を消費する。
    ///
    /// # Panics
    ///
    /// 覗いた値と違う`id`を渡したときにpanicする。採番点が1つであることを
    /// 崩した呼び出しであり、そのまま進めると`id`の単調性が壊れる。
    pub fn commit(&mut self, id: u32) {
        assert_eq!(
            self.peek_next().ok(),
            Some(id),
            "覗いた`id`と違う値をcommitした"
        );
        self.next = Some(id + 1);
    }

    /// 通常のmessage用に`id`を1つ払い出す。
    ///
    /// 失敗しない送出でだけ使う。encodeやqueueへの投入が失敗しうる場合は
    /// [`Self::peek_next`]と[`Self::commit`]へ分ける。
    ///
    /// # Errors
    ///
    /// 予約に達している場合に[`IdSpaceExhausted`]を返す。
    pub fn allocate(&mut self) -> Result<u32, IdSpaceExhausted> {
        let id = self.peek_next()?;
        self.commit(id);
        Ok(id)
    }

    /// 終端報告用に予約した`id`を、消費せずに覗く。
    ///
    /// **予約が成立するのは、次に払い出す`id`が`u32::MAX`になったときだけである。**
    /// 枯渇前に取り出せてしまうと、通常の`id`を終端報告に使ったうえで採番が
    /// 止まる。予約はあくまで「上限値を通常の送出へ渡さないための取り置き」である。
    #[must_use]
    pub const fn peek_terminal(&self) -> Option<u32> {
        if self.terminal_taken {
            return None;
        }
        match self.next {
            Some(u32::MAX) => Some(u32::MAX),
            _ => None,
        }
    }

    /// 終端報告（`protocol_fault`）用に予約した`id`を取り出す。
    ///
    /// **ちょうど1件にだけ使う。**2回目以降と、枯渇前の呼び出しは`None`を返す。
    pub fn take_terminal(&mut self) -> Option<u32> {
        let id = self.peek_terminal()?;
        self.terminal_taken = true;
        self.next = None;
        Some(id)
    }

    /// 通常の採番が停止しているか。
    ///
    /// `true`のとき、新しい`(sid, id)`を要する送出は行えない。既存の
    /// `(sid, id)`による再送は影響を受けない。
    #[must_use]
    pub const fn is_exhausted(&self) -> bool {
        self.peek_next().is_err()
    }

    /// 終端報告用の`id`が今すぐ取り出せるか。
    ///
    /// 枯渇前は`false`である。予約は上限に達してはじめて成立する。
    #[must_use]
    pub const fn terminal_available(&self) -> bool {
        self.peek_terminal().is_some()
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

    /// **枯渇前に終端報告用の`id`を取り出せてはならない。**
    ///
    /// 取り出せると、通常の`id`（例えば`1`）を終端報告に使ったうえで採番が
    /// 止まる。予約は「上限値を通常の送出へ渡さないための取り置き」であって、
    /// 「いつでも1件引ける枠」ではない。
    #[test]
    fn the_terminal_id_is_unavailable_until_the_space_is_actually_exhausted() {
        let mut ids = IdAllocator::new();
        assert_eq!(ids.peek_terminal(), None, "枯渇前は覗けない");
        assert!(!ids.terminal_available());
        assert_eq!(ids.take_terminal(), None, "枯渇前は取り出せない");

        // 採番は壊れていない。
        assert_eq!(ids.allocate(), Ok(1));
        assert_eq!(ids.allocate(), Ok(2));

        let mut ids = IdAllocator::starting_at(u32::MAX - 1);
        assert_eq!(ids.take_terminal(), None, "上限の1つ手前でもまだ早い");
        assert_eq!(ids.allocate(), Ok(u32::MAX - 1));
        assert_eq!(ids.peek_terminal(), Some(u32::MAX), "ここで予約が成立する");
    }

    /// 準備が失敗した場合に`id`を消費しない。飛びではなく枯渇を避けるためである。
    #[test]
    fn peeking_does_not_consume_the_id_until_committed() {
        let mut ids = IdAllocator::new();
        assert_eq!(ids.peek_next(), Ok(1));
        assert_eq!(ids.peek_next(), Ok(1), "覗くだけでは進まない");
        ids.commit(1);
        assert_eq!(ids.peek_next(), Ok(2));
    }

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
