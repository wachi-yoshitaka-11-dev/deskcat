//! 上限のある送信queue。
//!
//! 受け入れ条件は「**上限のないmessage queueが存在しない**」ことである。
//! したがって`VecDeque::push_back`を無条件に呼ぶ経路を1つも残さない。
//! 溢れは黙って捨てず、**明示的にdropしてcounterを増やす**。

use core::num::NonZeroUsize;
use std::collections::VecDeque;

/// [`Outbox::enqueue`]の結果。
///
/// `#[must_use]`にしているのは、dropを戻り値で返すだけにして呼び出し側が
/// 無視できると、「握りつぶさない」という要求が実質的に効かなくなるためである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "溢れたかどうかを見ずに捨てると、dropを握りつぶすことになる"]
pub enum Enqueued {
    /// queueへ入った。
    Accepted,
    /// **空のpayloadだったため受け付けなかった。**
    ///
    /// 空を通すと[`Outbox::peek`]が空sliceを返し、`write(&[])`が契約どおり
    /// `Ok(0)`を返す。呼び出し側はそれをEOF＝切断と読み、偽の切断を記録して
    /// queueを捨てる。**`Outbox`は公開APIなので、境界で弾く。**
    Empty,
    /// 容量に達していたため、**この送信を捨てた**。
    ///
    /// 古い側ではなく新しい側を捨てる。既にqueueへ入ったmessageは送信順序の
    /// 一部であり、後から来たものを優先して押し出すと、送出済みのものとの
    /// 前後関係が壊れる。捨てたことは呼び出し側へ返し、counterにも残す。
    Dropped,
}

/// 固定容量の送信queue。
#[derive(Debug)]
pub struct Outbox {
    queue: VecDeque<Vec<u8>>,
    capacity: usize,
    dropped: u64,
    /// 先頭messageのうち、既に書き出したbyte数。partial writeの進捗である。
    written: usize,
}

impl Outbox {
    /// 容量を指定して作る。
    ///
    /// **容量を[`NonZeroUsize`]で受け取る。**`Outbox`は公開APIであり、外部から
    /// 任意の値を渡せる。0を`assert!`で弾くと、公開APIへの不正入力をprocessの
    /// 終了で扱うことになる。**0を表現できない型にして、失敗する経路そのものを
    /// 無くす。**呼び出し側の検証は[`crate::SerialConfig::with_outbox_capacity`]が
    /// 型付きerrorで行う。
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity.get()),
            capacity: capacity.get(),
            dropped: 0,
            written: 0,
        }
    }

    /// 送信するbyte列を入れる。容量を超える場合は捨ててcounterを増やす。
    ///
    /// **空のbyte列は受け付けない**（[`Enqueued::Empty`]）。溢れとは別に扱う。
    /// 空はcounterへ計上しない。捨てた送信ではなく、呼び出し側の誤りである。
    pub fn enqueue(&mut self, bytes: impl Into<Vec<u8>>) -> Enqueued {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Enqueued::Empty;
        }
        if self.queue.len() >= self.capacity {
            self.dropped += 1;
            return Enqueued::Dropped;
        }
        self.queue.push_back(bytes);
        Enqueued::Accepted
    }

    /// 次に書き出すbyte列。まだ書けていない部分だけを返す。
    #[must_use]
    pub fn peek(&self) -> Option<&[u8]> {
        self.queue.front().map(|front| &front[self.written..])
    }

    /// `n` byteを書けたことを記録する。先頭を書き切ったらqueueから外す。
    ///
    /// # Panics
    ///
    /// queueが空のとき、または先頭の残りより多いbyte数を渡したときにpanicする。
    /// **`write`が要求より多くを返すことはない。**それが起きたなら下層の実装が
    /// 契約を破っており、進捗の記録を続けると送信内容が壊れる。
    pub fn advance(&mut self, n: usize) {
        let front_len = self.queue.front().expect("空のoutboxへadvanceした").len();
        let remaining = front_len - self.written;
        assert!(
            n <= remaining,
            "書けた量が残りを超えている: {n} > {remaining}"
        );
        self.written += n;
        if self.written == front_len {
            self.queue.pop_front();
            self.written = 0;
        }
    }

    /// 保留中のmessage数。
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// 保留が無いか。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// 容量。
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// 溢れて捨てた件数。
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }

    /// 先頭messageの書き出し済みbyte数。
    #[must_use]
    pub const fn partial_write_len(&self) -> usize {
        self.written
    }

    /// 保留分をすべて捨て、捨てた件数を返す。
    ///
    /// 切断時に使う。再接続後に古いmessageを送り直すと、相手から見て新しい
    /// commandとして扱われうる。再送するかどうかは上位（`PROTO-TBD-013`の
    /// stale command判定）の話であり、この層が勝手に送り直さない。
    ///
    /// **件数を[`Self::dropped`]へは足さない。**溢れによるdropと切断による破棄は
    /// 原因が違う（前者は輻輳、後者はlinkの障害）。1つのcounterへ畳むと、
    /// どちらが起きているのか外から分けられない。
    pub fn clear(&mut self) -> u64 {
        let discarded = self.queue.len() as u64;
        self.queue.clear();
        self.written = 0;
        discarded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).expect("testの容量は0ではない")
    }

    /// 空のpayloadは受け付けない。通すと偽の切断を作る。
    #[test]
    fn an_empty_payload_is_rejected_at_the_boundary() {
        let mut outbox = Outbox::new(cap(2));
        assert_eq!(outbox.enqueue(Vec::new()), Enqueued::Empty);
        assert!(outbox.is_empty(), "queueへ入らない");
        assert_eq!(outbox.dropped(), 0, "溢れではないのでcounterを増やさない");
        assert_eq!(outbox.peek(), None);
    }

    #[test]
    fn accepts_up_to_capacity_then_drops_and_counts() {
        let mut outbox = Outbox::new(cap(2));
        assert_eq!(outbox.enqueue(b"a".to_vec()), Enqueued::Accepted);
        assert_eq!(outbox.enqueue(b"b".to_vec()), Enqueued::Accepted);
        assert_eq!(outbox.enqueue(b"c".to_vec()), Enqueued::Dropped);
        assert_eq!(outbox.len(), 2, "容量を超えて保持しない");
        assert_eq!(outbox.dropped(), 1);
    }

    /// 新しい側を捨てる。既にqueueへ入った送信順序を壊さない。
    #[test]
    fn dropping_removes_the_new_message_not_the_queued_one() {
        let mut outbox = Outbox::new(cap(1));
        let _ = outbox.enqueue(b"first".to_vec());
        assert_eq!(outbox.enqueue(b"second".to_vec()), Enqueued::Dropped);
        assert_eq!(outbox.peek(), Some(&b"first"[..]));
    }

    #[test]
    fn advance_tracks_partial_writes_across_calls() {
        let mut outbox = Outbox::new(cap(2));
        let _ = outbox.enqueue(b"hello".to_vec());

        outbox.advance(2);
        assert_eq!(outbox.peek(), Some(&b"llo"[..]));
        assert_eq!(outbox.partial_write_len(), 2);

        outbox.advance(3);
        assert!(outbox.is_empty(), "書き切ったらqueueから外れる");
        assert_eq!(outbox.partial_write_len(), 0);
    }

    /// 切断による破棄は、溢れによるdropとは別に数える。
    #[test]
    fn clear_reports_the_discarded_count_without_touching_the_overflow_counter() {
        let mut outbox = Outbox::new(cap(4));
        let _ = outbox.enqueue(b"a".to_vec());
        let _ = outbox.enqueue(b"b".to_vec());
        assert_eq!(outbox.clear(), 2, "捨てた件数を返す");
        assert!(outbox.is_empty());
        assert_eq!(outbox.dropped(), 0, "溢れは起きていない");
    }

    #[test]
    #[should_panic(expected = "書けた量が残りを超えている")]
    fn advancing_past_the_front_is_a_contract_violation() {
        let mut outbox = Outbox::new(cap(1));
        let _ = outbox.enqueue(b"ab".to_vec());
        outbox.advance(3);
    }
}
