//! byte列を上限付きでlineへ切り出すframer（§8手順1〜4、6）。
//!
//! この層はJSONもUTF-8も解釈しない。byteを蓄え、`\n`でlineを切り、上限を超えた行を
//! 次の`\n`まで破棄するところまでを持つ。UTF-8の検証と[`crate::decode_line`]への
//! 受け渡しは[`crate::receiver`]が行う。
//!
//! # 上限
//!
//! bufferは構築時に[`Box`]で確保し、それ以降**伸びない**。`push`を持たない型を使うことで、
//! 「上限のないbufferを作らない」を規律ではなく型の性質にしている。
//! 容量の意味は[`crate::limits::MAX_LINE_BODY_BYTES`]を参照する。
//!
//! # `\r`の除去
//!
//! §8手順4「改行受信時に、直前にある任意のcarriage returnを除去する」はこの層が持つ。
//! [`crate::decode_line`]も末尾の`\r`を落とすが、あちらは1 lineだけを単体で渡された場合の
//! ための処理である。framerを通した行には`\r`が残らないため、二重には効かない。

use crate::limits;

/// framerが切り出した事象。
///
/// 借用元は[`LineFramer`]の内部bufferであり、次に`feed`を呼ぶまで有効である。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framed<'a> {
    /// 1 lineぶんのbody。終端の`\n`と、その直前の`\r`は取り除いてある。
    Line(&'a [u8]),
    /// 行長上限を超えたため、この行を次の`\n`まで破棄する。
    ///
    /// **検知した時点で1回だけ返す。**終端の`\n`を待たない。待つと、相手が`\n`を
    /// 送ってこない限り報告が出ず、§4.1の`boot`送信側がrecovery budgetを無応答のまま
    /// 使い切ってしまう。
    Oversize {
        /// 破棄する行のうち、identity復元のために保持していたprefix。
        ///
        /// 長さは[`LineFramer::prefix_budget`]までである。行の残りは保持しない。
        prefix: &'a [u8],
    },
}

/// [`LineFramer::feed`]の結果。
///
/// **保証は片方向である。`event`が`None`なら、`consumed`は必ず入力長に等しい。**
/// 逆は成り立たない。`feed`は事象を1件返した時点で止まるが、その事象が入力の最後の
/// byteで完成した場合は`consumed`も入力長になる（`b"abcd\n"`など）。
///
/// この保証があるため、呼び出し側のloopは「消費した分だけ進めて、事象が無くなったら抜ける」
/// だけでよく、入力を取りこぼす書き方にならない。
///
/// **`event`が`Some`のとき、`consumed`は0でありうる。**bufferが容量ちょうどで止まった直後に
/// 本文が届いた場合、1 byteも消費せずにoverflowを報告する。このとき内部stateは
/// 破棄中へ遷移しているため、次の`feed`が必ず入力を進める。loopは止まる。
#[derive(Debug)]
#[must_use = "`consumed`が入力長に満たないことがある。残りを捨てると入力を落とす"]
pub struct Progress<'a> {
    /// 入力の先頭から消費したbyte数。
    pub consumed: usize,
    /// 切り出せた事象。
    pub event: Option<Framed<'a>>,
}

/// 内部bufferの範囲として持つ事象。借用を後から張り直すための中間表現。
enum Pending {
    Line(usize),
    Oversize(usize),
}

/// 上限付きのincremental line framer。
///
/// 使い方は[`crate::receiver::LineReceiver`]を参照する。byte列だけを扱いたい場合に直接使う。
#[derive(Debug)]
pub struct LineFramer {
    /// 構築後は伸びない。`Box<[u8]>`は`push`を持たないため、上限が型の性質になる。
    buf: Box<[u8]>,
    len: usize,
    prefix_budget: usize,
    /// overflowを検知し、次の`\n`まで読み捨てている最中か。
    discarding: bool,
}

impl LineFramer {
    /// bodyの最大byte数を指定して作る。
    ///
    /// prefix予算は初期値として`capacity`（保持している行buffer全体）になる。
    /// `PROTO-TBD-002`が候補値を示していないため、ここで新しい数を決めない。
    /// 縮める場合は[`Self::with_prefix_budget`]を使う。
    ///
    /// # Panics
    ///
    /// `capacity`が0の場合にpanicする。1 byteも蓄えられないframerは、
    /// すべての入力をoversizeにするため、設定の誤りとして扱う。
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "line framer capacity must be at least 1 byte");
        Self {
            buf: vec![0; capacity].into_boxed_slice(),
            len: 0,
            prefix_budget: capacity,
            discarding: false,
        }
    }

    /// 仕様§2の候補値（[`limits::MAX_LINE_BODY_BYTES`]）で作る。
    #[must_use]
    pub fn with_protocol_limit() -> Self {
        Self::new(limits::MAX_LINE_BODY_BYTES)
    }

    /// oversize時にidentity復元へ渡すprefixのbyte数を縮める。
    ///
    /// `PROTO-TBD-002`が確定したときの入口である。`capacity`より大きい値はcapacityへ丸める。
    /// **縮めることは純粋な制限**であり、いま復元できない行が復元できるようにはならない。
    #[must_use]
    pub fn with_prefix_budget(mut self, bytes: usize) -> Self {
        self.prefix_budget = bytes.min(self.buf.len());
        self
    }

    /// bodyの最大byte数。
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// oversize時に保持するprefixのbyte数。
    #[must_use]
    pub fn prefix_budget(&self) -> usize {
        self.prefix_budget
    }

    /// 組み立て途中のbodyのbyte数。
    #[must_use]
    pub fn pending(&self) -> usize {
        self.len
    }

    /// overflowを検知して次の`\n`まで読み捨てている最中か。
    #[must_use]
    pub fn is_discarding(&self) -> bool {
        self.discarding
    }

    /// 組み立て途中のstateを捨てる。
    ///
    /// §10のport再openやreconnectで使う。これを呼ばないと、切断前の途中までのlineが
    /// 再接続後の最初のlineへ連結される。**事象は返さない。**
    pub fn reset(&mut self) {
        self.len = 0;
        self.discarding = false;
    }

    /// 入力の先頭から、事象を1件切り出せるところまで取り込む。
    ///
    /// 戻り値の不変条件は[`Progress`]を参照する。
    pub fn feed<'s>(&'s mut self, input: &[u8]) -> Progress<'s> {
        let mut consumed = 0;
        let mut pending = None;

        while consumed < input.len() {
            let rest = &input[consumed..];

            if self.discarding {
                if let Some(i) = find_newline(rest) {
                    self.discarding = false;
                    consumed += i + 1;
                } else {
                    consumed += rest.len();
                }
                continue;
            }

            let room = self.capacity() - self.len;
            match find_newline(rest) {
                // bodyが容量に収まる。`i == room`（上限ちょうど）も収まる側である。
                Some(i) if i <= room => {
                    self.store(&rest[..i]);
                    consumed += i + 1;
                    pending = Some(Pending::Line(body_len_without_carriage_return(
                        &self.buf[..self.len],
                    )));
                    self.len = 0;
                    break;
                }
                // `\n`がまだ来ないが、容量には収まる。入力を使い切って次のreadを待つ。
                None if rest.len() <= room => {
                    self.store(rest);
                    consumed += rest.len();
                }
                // `\n`が容量の先にある、または`\n`が無いまま容量を超える。どちらもoverflowである。
                _ => {
                    // 容量まで詰めてからoverflowとする。詰めたぶんがprefixになる。
                    let fill = room.min(rest.len());
                    self.store(&rest[..fill]);
                    consumed += fill;
                    pending = Some(Pending::Oversize(self.prefix_budget.min(self.len)));
                    self.len = 0;
                    self.discarding = true;
                    break;
                }
            }
        }

        let event = pending.map(|pending| match pending {
            Pending::Line(end) => Framed::Line(&self.buf[..end]),
            Pending::Oversize(end) => Framed::Oversize {
                prefix: &self.buf[..end],
            },
        });
        Progress { consumed, event }
    }

    fn store(&mut self, bytes: &[u8]) {
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
    }
}

/// `\n`の位置を返す。
fn find_newline(bytes: &[u8]) -> Option<usize> {
    bytes.iter().position(|&byte| byte == b'\n')
}

/// 末尾の`\r`を1つだけ落としたあとのbody長を返す（§2、§8手順4）。
fn body_len_without_carriage_return(body: &[u8]) -> usize {
    match body {
        [.., b'\r'] => body.len() - 1,
        _ => body.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Framed, LineFramer};

    /// 入力を1回で流し、事象を順に集める。`consumed`を必ず尊重する。
    fn drain(framer: &mut LineFramer, input: &[u8]) -> Vec<Result<Vec<u8>, Vec<u8>>> {
        let mut events = Vec::new();
        let mut rest = input;
        loop {
            let progress = framer.feed(rest);
            let consumed = progress.consumed;
            match progress.event {
                Some(Framed::Line(line)) => events.push(Ok(line.to_vec())),
                Some(Framed::Oversize { prefix }) => events.push(Err(prefix.to_vec())),
                None => {
                    assert_eq!(consumed, rest.len(), "eventが無いのに入力が残っている");
                    break;
                }
            }
            rest = &rest[consumed..];
        }
        events
    }

    #[test]
    fn splits_on_newline_and_strips_crlf() {
        let mut framer = LineFramer::new(16);
        assert_eq!(
            drain(&mut framer, b"ab\ncd\r\n"),
            vec![Ok(b"ab".to_vec()), Ok(b"cd".to_vec())]
        );
        assert_eq!(framer.pending(), 0);
    }

    /// 改行の直前ではない`\r`は落とさない。JSONの空白として意味を持つ。
    #[test]
    fn keeps_a_carriage_return_that_is_not_before_the_newline() {
        let mut framer = LineFramer::new(16);
        assert_eq!(drain(&mut framer, b"a\rb\n"), vec![Ok(b"a\rb".to_vec())]);
    }

    /// `\r`が前のchunkの末尾、`\n`が次のchunkの先頭でも同じ結果になる。
    #[test]
    fn handles_crlf_split_across_chunks() {
        let mut framer = LineFramer::new(16);
        assert!(drain(&mut framer, b"ab\r").is_empty());
        assert_eq!(drain(&mut framer, b"\n"), vec![Ok(b"ab".to_vec())]);
    }

    #[test]
    fn empty_input_makes_no_progress() {
        let mut framer = LineFramer::new(4);
        let progress = framer.feed(b"");
        assert_eq!(progress.consumed, 0);
        assert!(progress.event.is_none());
    }

    /// 容量ちょうどのbodyは受理し、1 byte超えたらoversizeにする。
    #[test]
    fn enforces_the_body_limit_at_the_boundary() {
        let mut framer = LineFramer::new(4);
        assert_eq!(drain(&mut framer, b"abcd\n"), vec![Ok(b"abcd".to_vec())]);
        assert_eq!(
            drain(&mut framer, b"abcde\n"),
            vec![Err(b"abcd".to_vec())],
            "1 byte超過はoversize"
        );
        assert!(!framer.is_discarding(), "終端の改行で破棄を抜ける");
    }

    /// CRLFの行は`\r`のぶんだけbody予算を多く使う。
    #[test]
    fn a_carriage_return_consumes_body_budget() {
        let mut framer = LineFramer::new(4);
        assert_eq!(drain(&mut framer, b"abc\r\n"), vec![Ok(b"abc".to_vec())]);
        assert_eq!(drain(&mut framer, b"abcd\r\n"), vec![Err(b"abcd".to_vec())]);
    }

    /// overflowは検知時に1回だけ返す。終端の`\n`が何chunk先でも増えない。
    #[test]
    fn reports_an_oversize_line_exactly_once() {
        let mut framer = LineFramer::new(4);
        assert_eq!(drain(&mut framer, b"abcdefg"), vec![Err(b"abcd".to_vec())]);
        assert!(framer.is_discarding());
        assert!(drain(&mut framer, b"hijklmn").is_empty());
        assert!(drain(&mut framer, b"opq").is_empty());
        assert!(drain(&mut framer, b"\n").is_empty());
        assert!(!framer.is_discarding());
        assert_eq!(drain(&mut framer, b"ok\n"), vec![Ok(b"ok".to_vec())]);
    }

    #[test]
    fn resumes_at_the_next_line_after_an_oversize_line() {
        let mut framer = LineFramer::new(4);
        assert_eq!(
            drain(&mut framer, b"toolong\nab\ntoolong2\ncd\n"),
            vec![
                Err(b"tool".to_vec()),
                Ok(b"ab".to_vec()),
                Err(b"tool".to_vec()),
                Ok(b"cd".to_vec()),
            ]
        );
        assert_eq!(framer.pending(), 0);
    }

    /// 破棄中にさらにoversizeが続いても、2回目のoversizeを返さない。
    #[test]
    fn does_not_report_again_while_discarding() {
        let mut framer = LineFramer::new(2);
        let events = drain(&mut framer, b"aaaaaaaaaaaaaaaaaaaa\n");
        assert_eq!(events, vec![Err(b"aa".to_vec())]);
    }

    /// bufferが容量ちょうどで止まった直後に`\n`が来た場合を、oversizeにしない。
    #[test]
    fn a_full_buffer_followed_by_a_newline_is_a_valid_line() {
        let mut framer = LineFramer::new(4);
        assert!(drain(&mut framer, b"abcd").is_empty());
        assert_eq!(framer.pending(), 4);
        assert_eq!(drain(&mut framer, b"\n"), vec![Ok(b"abcd".to_vec())]);
    }

    /// bufferが容量ちょうどで止まった直後に本文が来た場合は、oversizeにする。
    #[test]
    fn a_full_buffer_followed_by_more_body_overflows() {
        let mut framer = LineFramer::new(4);
        assert!(drain(&mut framer, b"abcd").is_empty());
        assert_eq!(drain(&mut framer, b"e\n"), vec![Err(b"abcd".to_vec())]);
    }

    #[test]
    fn prefix_budget_narrows_the_retained_prefix() {
        let mut framer = LineFramer::new(8).with_prefix_budget(3);
        assert_eq!(framer.prefix_budget(), 3);
        assert_eq!(
            drain(&mut framer, b"abcdefghij\n"),
            vec![Err(b"abc".to_vec())]
        );
    }

    #[test]
    fn prefix_budget_is_clamped_to_capacity() {
        let framer = LineFramer::new(4).with_prefix_budget(999);
        assert_eq!(framer.prefix_budget(), 4);
    }

    #[test]
    fn reset_drops_a_partial_line_without_prepending_it() {
        let mut framer = LineFramer::new(16);
        assert!(drain(&mut framer, b"partial").is_empty());
        framer.reset();
        assert_eq!(framer.pending(), 0);
        assert_eq!(drain(&mut framer, b"fresh\n"), vec![Ok(b"fresh".to_vec())]);
    }

    #[test]
    fn reset_clears_the_discarding_state() {
        let mut framer = LineFramer::new(2);
        assert_eq!(drain(&mut framer, b"toolong"), vec![Err(b"to".to_vec())]);
        assert!(framer.is_discarding());
        framer.reset();
        assert!(!framer.is_discarding());
        assert_eq!(drain(&mut framer, b"ok\n"), vec![Ok(b"ok".to_vec())]);
    }

    #[test]
    fn empty_lines_are_produced_as_empty_bodies() {
        let mut framer = LineFramer::new(4);
        assert_eq!(
            drain(&mut framer, b"\n\r\n"),
            vec![Ok(Vec::new()), Ok(Vec::new())]
        );
    }

    /// 切り方を変えても事象の並びは変わらない。
    #[test]
    fn the_outcome_does_not_depend_on_the_chunking() {
        let input = b"ab\ncdefghij\n\nkl\r\n";
        let mut whole = LineFramer::new(4);
        let expected = drain(&mut whole, input);

        for chunk in 1..=input.len() {
            let mut framer = LineFramer::new(4);
            let mut events = Vec::new();
            for part in input.chunks(chunk) {
                events.extend(drain(&mut framer, part));
            }
            assert_eq!(events, expected, "chunk size {chunk}");
        }
    }

    /// 改行の無い長大な入力でも、bufferは容量を超えず、確保も1回きりである。
    #[test]
    fn a_newline_free_flood_stays_bounded() {
        let mut framer = LineFramer::new(64);
        let address = framer.buf.as_ptr();
        let mut oversize = 0;

        for _ in 0..16_384 {
            let events = drain(&mut framer, &[b'x'; 64]);
            oversize += events.len();
            assert!(framer.pending() <= framer.capacity());
        }

        assert_eq!(oversize, 1, "改行が来るまでoversizeは1回だけ");
        assert_eq!(framer.buf.as_ptr(), address, "bufferを再確保していない");
    }

    #[test]
    #[should_panic(expected = "capacity must be at least 1 byte")]
    fn a_zero_capacity_framer_is_rejected() {
        let _ = LineFramer::new(0);
    }
}
