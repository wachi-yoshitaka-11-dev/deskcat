//! byte列からframeを取り出す上限付きreceiver（§8手順1〜7）。
//!
//! [`crate::framing::LineFramer`]がbyteをlineへ切り、この層がUTF-8を検証して
//! [`crate::decode_line`]へ渡す。**`decode_line`が`&str`を取るため、byte列から`str`への
//! 境界はここにある。**
//!
//! # 分類であって、送出の判断ではない
//!
//! [`Rejection`]は§7の分類と、復元できた`(sid, id)`を持つだけである。それを相手へ返すか、
//! 計数だけにするかは方向と§8.2の送出上限に従い、session stateを持つ層（#11、#12）が決める。
//!
//! # 例
//!
//! ```
//! use deskcat_protocol::{LineReceiver, Outcome};
//!
//! let mut receiver = LineReceiver::with_protocol_limit();
//! let mut frames = 0;
//!
//! // 1 lineが2回のreadに分かれ、2 line目が同じreadに入っている。
//! for chunk in [
//!     &br#"{"v":1,"sid":90312,"id":907,"ts_"#[..],
//!     &br#"ms":54000,"type":"ping","payload":{}}
//! {"v":1,"sid":90312,"id":908,"ts_ms":54100,"type":"get_status","payload":{}}
//! "#[..],
//! ] {
//!     receiver.drain(chunk, |outcome| {
//!         if let Outcome::Frame(frame) = outcome {
//!             frames += 1;
//!             assert_eq!(frame.envelope.sid, 90312);
//!         }
//!     });
//! }
//!
//! assert_eq!(frames, 2);
//! assert_eq!(receiver.pending(), 0);
//! ```

use crate::decode::decode_line;
use crate::envelope::Frame;
use crate::error::{DecodeError, ErrorCode};
use crate::framing::{Framed, LineFramer};
use crate::prefix::{self, PrefixEnvelope};

/// 1 lineぶんの受信結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// 検証を通ったframe（§8手順7を通過）。
    Frame(Frame),
    /// 分類済みのerrorで拒否した。
    Rejected(Rejection),
}

/// parser counterを分けるための、拒否の起因（§7末尾）。
///
/// wire上の[`ErrorCode`]は§4.6の対応表どおり`invalid_envelope`へまとめるため、
/// これだけではUTF-8の不正とJSONの不正を区別できない。§7が「parser counterでは
/// invalid UTF-8、invalid JSONを区別する」と定めているので、分類をここに持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Cause {
    /// lineがUTF-8として解釈できなかった。
    InvalidUtf8 {
        /// UTF-8として妥当だった先頭からのbyte数。
        valid_up_to: usize,
    },
    /// UTF-8としては読めたが、[`crate::decode_line`]の検証で落ちた。
    ///
    /// **JSONの不正とenvelopeの不正は、ここでは区別が付かない。**`decode_line`が
    /// どちらも`invalid_envelope`として返すためである。区別が必要になった時点で、
    /// `decode_line`側の分類を細かくする。
    Decode,
    /// 行長上限を超えたため、次の改行まで破棄した。
    Oversize,
}

/// lineを拒否した理由と、そこから分かった相関情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    error: DecodeError,
    cause: Cause,
    identity: Option<(u32, u32)>,
    type_name: Option<&'static str>,
}

impl Rejection {
    /// 相手へ返す分類（§7）。
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.error.code()
    }

    /// parser counterを分けるための起因。
    #[must_use]
    pub const fn cause(&self) -> Cause {
        self.cause
    }

    /// 診断用の説明。wireへそのまま載せない。
    #[must_use]
    pub fn detail(&self) -> &str {
        self.error.detail()
    }

    /// 復元できた`(sid, id)`。
    ///
    /// oversizeの場合は上限付きprefixからの復元結果である（§8手順6）。
    /// `None`のときは相関応答を組み立てられないため、呼び出し側は計数だけを行う。
    ///
    /// **oversize以外では常に`None`である。**[`crate::decode_line`]はerror時に
    /// envelopeを返さないため、この層からは分からない。§8手順7が要求する
    /// 「identityを復元できる場合の拒否ACK」に必要な情報であり、#12で
    /// [`crate::prefix::recover_identity`]を拡張点として使う。
    #[must_use]
    pub const fn identity(&self) -> Option<(u32, u32)> {
        self.identity
    }

    /// 復元できた既知のmessage type。
    ///
    /// §7は、oversizeした行が`boot`のときだけ`line_too_long`を通常のerrorではなく
    /// `status: rejected`のACKの`code`として返すよう定めている。その判定に使う。
    #[must_use]
    pub const fn type_name(&self) -> Option<&'static str> {
        self.type_name
    }

    /// この拒否を計上する`status.payload.protocol`のfield名。
    #[must_use]
    pub const fn counter_field(&self) -> Option<&'static str> {
        self.error.counter_field()
    }
}

/// [`LineReceiver::feed`]の結果。
///
/// 不変条件は[`crate::framing::Progress`]と同じである。**`outcome`が`None`であることと、
/// `consumed`が入力長に等しいことは同値である。**
#[derive(Debug)]
#[must_use = "`consumed`が入力長に満たないことがある。残りを捨てると入力を落とす"]
pub struct Received {
    /// 入力の先頭から消費したbyte数。
    pub consumed: usize,
    /// 取り出せた結果。
    pub outcome: Option<Outcome>,
}

/// 上限付きのincremental JSON Lines receiver。
#[derive(Debug)]
pub struct LineReceiver {
    framer: LineFramer,
}

impl LineReceiver {
    /// bodyの最大byte数を指定して作る。
    ///
    /// # Panics
    ///
    /// `capacity`が0の場合にpanicする。
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            framer: LineFramer::new(capacity),
        }
    }

    /// 仕様§2の候補値（[`crate::limits::MAX_LINE_BODY_BYTES`]）で作る。
    ///
    /// この容量なら、bufferを通ったlineは必ず[`crate::decode_line`]の行長判定も通る。
    /// `line_too_long`の出所がframerの1箇所だけになる。
    #[must_use]
    pub fn with_protocol_limit() -> Self {
        Self {
            framer: LineFramer::with_protocol_limit(),
        }
    }

    /// oversize時にidentity復元へ渡すprefixのbyte数を縮める。
    ///
    /// 既定値は行buffer全体である。`PROTO-TBD-002`が確定したときの入口である。
    #[must_use]
    pub fn with_prefix_budget(mut self, bytes: usize) -> Self {
        self.framer = self.framer.with_prefix_budget(bytes);
        self
    }

    /// bodyの最大byte数。
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.framer.capacity()
    }

    /// 組み立て途中のbodyのbyte数。
    #[must_use]
    pub fn pending(&self) -> usize {
        self.framer.pending()
    }

    /// overflowを検知して次の改行まで読み捨てている最中か。
    #[must_use]
    pub fn is_discarding(&self) -> bool {
        self.framer.is_discarding()
    }

    /// 組み立て途中のstateを捨てる（§10のport再open、reconnect）。
    pub fn reset(&mut self) {
        self.framer.reset();
    }

    /// 入力の先頭から、結果を1件取り出せるところまで取り込む。
    pub fn feed(&mut self, input: &[u8]) -> Received {
        let capacity = self.framer.capacity();
        let progress = self.framer.feed(input);
        let consumed = progress.consumed;
        let outcome = progress.event.map(|event| interpret(event, capacity));
        Received { consumed, outcome }
    }

    /// 入力を最後まで処理し、結果ごとに`on_outcome`を呼ぶ。
    ///
    /// [`Self::feed`]のloopをここに1回だけ書く。呼び出し側が`consumed`の扱いを
    /// 間違える余地を無くすためである。
    pub fn drain(&mut self, input: &[u8], mut on_outcome: impl FnMut(Outcome)) {
        let mut rest = input;
        loop {
            let received = self.feed(rest);
            rest = &rest[received.consumed..];
            match received.outcome {
                Some(outcome) => on_outcome(outcome),
                None => break,
            }
        }
    }
}

/// framerの事象を、分類済みの結果へ変える。
fn interpret(event: Framed<'_>, capacity: usize) -> Outcome {
    match event {
        Framed::Line(body) => match core::str::from_utf8(body) {
            Ok(line) => match decode_line(line) {
                Ok(frame) => Outcome::Frame(frame),
                Err(error) => Outcome::Rejected(Rejection {
                    // `with_protocol_limit`の容量では`LineTooLong`は返らない。framerが先に
                    // 検知するためである。容量をそれより大きく取った場合だけここへ来るので、
                    // 分類はoversizeへ寄せる。ただしidentityは復元していない。
                    cause: if error.code() == ErrorCode::LineTooLong {
                        Cause::Oversize
                    } else {
                        Cause::Decode
                    },
                    error,
                    identity: None,
                    type_name: None,
                }),
            },
            Err(error) => Outcome::Rejected(Rejection {
                error: DecodeError::new(
                    // §4.6がUTF-8／JSON／envelope不正をまとめて`parse_errors`とするため、
                    // wire上の分類は`invalid_envelope`になる。UTF-8固有の情報は`cause`が持つ。
                    ErrorCode::InvalidEnvelope,
                    format!(
                        "line is not valid UTF-8: {} bytes were valid, then {} invalid byte(s)",
                        error.valid_up_to(),
                        error.error_len().map_or_else(
                            || "an unexpected end of".to_owned(),
                            |len| len.to_string()
                        )
                    ),
                ),
                cause: Cause::InvalidUtf8 {
                    valid_up_to: error.valid_up_to(),
                },
                identity: None,
                type_name: None,
            }),
        },
        Framed::Oversize { prefix } => {
            let recovered = prefix::recover_identity(prefix);
            Outcome::Rejected(Rejection {
                error: DecodeError::new(
                    ErrorCode::LineTooLong,
                    format!(
                        "line body exceeded {capacity} bytes and was discarded up to the next newline"
                    ),
                ),
                cause: Cause::Oversize,
                identity: recovered.map(|PrefixEnvelope { sid, id, .. }| (sid, id)),
                type_name: recovered.and_then(|recovered| recovered.type_name),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cause, LineReceiver, Outcome};
    use crate::error::ErrorCode;
    use crate::limits;

    fn collect(receiver: &mut LineReceiver, input: &[u8]) -> Vec<Outcome> {
        let mut outcomes = Vec::new();
        receiver.drain(input, |outcome| outcomes.push(outcome));
        outcomes
    }

    fn codes(outcomes: &[Outcome]) -> Vec<(ErrorCode, Cause)> {
        outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                Outcome::Rejected(rejection) => Some((rejection.code(), rejection.cause())),
                Outcome::Frame(_) => None,
            })
            .collect()
    }

    const PING: &[u8] = br#"{"v":1,"sid":90312,"id":907,"ts_ms":54000,"type":"ping","payload":{}}"#;

    #[test]
    fn decodes_a_line_split_across_reads() {
        let mut receiver = LineReceiver::with_protocol_limit();
        for split in 0..=PING.len() {
            receiver.reset();
            let mut outcomes = collect(&mut receiver, &PING[..split]);
            outcomes.extend(collect(&mut receiver, &PING[split..]));
            outcomes.extend(collect(&mut receiver, b"\n"));

            match outcomes.as_slice() {
                [Outcome::Frame(frame)] => {
                    assert_eq!(frame.envelope.identity(), (90312, 907), "split {split}");
                }
                other => panic!("split {split}: {other:?}"),
            }
        }
    }

    #[test]
    fn classifies_invalid_utf8_separately_from_json() {
        let mut receiver = LineReceiver::with_protocol_limit();

        // `0xff`はUTF-8のどの列にも現れない。
        let outcomes = collect(&mut receiver, b"{\"v\":1,\xff}\n");
        assert_eq!(
            codes(&outcomes),
            vec![(
                ErrorCode::InvalidEnvelope,
                Cause::InvalidUtf8 { valid_up_to: 7 }
            )]
        );

        let outcomes = collect(&mut receiver, b"not json\n");
        assert_eq!(
            codes(&outcomes),
            vec![(ErrorCode::InvalidEnvelope, Cause::Decode)]
        );
    }

    /// 途中で切れたmulti-byte列も、完全なUTF-8でないので同じ分類になる。
    #[test]
    fn classifies_a_truncated_multibyte_sequence_as_invalid_utf8() {
        let mut receiver = LineReceiver::with_protocol_limit();
        let outcomes = collect(&mut receiver, b"{\"a\":\"\xe3\x81\"}\n");
        assert!(matches!(
            codes(&outcomes).as_slice(),
            [(ErrorCode::InvalidEnvelope, Cause::InvalidUtf8 { .. })]
        ));
    }

    /// UTF-8としてもJSONとしても不正な行は、UTF-8として1件だけ報告する。
    #[test]
    fn invalid_utf8_takes_precedence_over_invalid_json() {
        let mut receiver = LineReceiver::with_protocol_limit();
        let outcomes = collect(&mut receiver, b"garbage \xff\xfe\n");
        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            codes(&outcomes).as_slice(),
            [(_, Cause::InvalidUtf8 { .. })]
        ));
    }

    #[test]
    fn resumes_after_invalid_utf8() {
        let mut receiver = LineReceiver::with_protocol_limit();
        let mut input = b"\xff\xff\n".to_vec();
        input.extend_from_slice(PING);
        input.push(b'\n');

        let outcomes = collect(&mut receiver, &input);
        assert!(matches!(outcomes[0], Outcome::Rejected(_)));
        assert!(matches!(outcomes[1], Outcome::Frame(_)));
        assert_eq!(outcomes.len(), 2);
    }

    /// 4 byteのemojiをchunk境界で割っても、組み立て後に検証するので通る。
    #[test]
    fn a_multibyte_character_split_across_reads_still_decodes() {
        let line = format!(
            r#"{{"v":1,"sid":1,"id":1,"ts_ms":0,"type":"ack","payload":{{"reply_sid":1,"reply_to":1,"status":"rejected","code":"busy","detail":"{}"}}}}"#,
            "\u{1f431}"
        );
        let bytes = line.as_bytes();
        let emoji_start = bytes
            .windows(4)
            .position(|window| window == "\u{1f431}".as_bytes())
            .expect("emojiが含まれている");

        for offset in 1..4 {
            let mut receiver = LineReceiver::with_protocol_limit();
            let mut outcomes = collect(&mut receiver, &bytes[..emoji_start + offset]);
            outcomes.extend(collect(&mut receiver, &bytes[emoji_start + offset..]));
            outcomes.extend(collect(&mut receiver, b"\n"));
            assert!(
                matches!(outcomes.as_slice(), [Outcome::Frame(_)]),
                "offset {offset}: {outcomes:?}"
            );
        }
    }

    #[test]
    fn an_empty_line_is_rejected_as_an_envelope_problem() {
        let mut receiver = LineReceiver::with_protocol_limit();
        assert_eq!(
            codes(&collect(&mut receiver, b"\n")),
            vec![(ErrorCode::InvalidEnvelope, Cause::Decode)]
        );
    }

    /// BOMを黙って落とさない。UTF-8としては妥当だが、JSONとしては不正である。
    #[test]
    fn a_byte_order_mark_is_not_stripped() {
        let mut receiver = LineReceiver::with_protocol_limit();
        let mut input = b"\xef\xbb\xbf".to_vec();
        input.extend_from_slice(PING);
        input.push(b'\n');
        assert_eq!(
            codes(&collect(&mut receiver, &input)),
            vec![(ErrorCode::InvalidEnvelope, Cause::Decode)]
        );
    }

    /// 行の途中の生の`\n`は、string内であってもlineを切る。framerはJSONを解釈しない。
    #[test]
    fn a_raw_newline_inside_a_json_string_splits_the_line() {
        let mut receiver = LineReceiver::with_protocol_limit();
        let outcomes = collect(&mut receiver, b"{\"a\":\"x\ny\"}\n");
        assert_eq!(outcomes.len(), 2, "2件の不正lineになる: {outcomes:?}");
        assert!(
            outcomes
                .iter()
                .all(|outcome| matches!(outcome, Outcome::Rejected(_)))
        );
    }

    #[test]
    fn oversize_recovers_the_identity_from_the_prefix() {
        let mut receiver = LineReceiver::new(48);
        let mut input =
            br#"{"v":1,"sid":90312,"id":907,"ts_ms":54000,"type":"boot","payload":{}}"#.to_vec();
        input.push(b'\n');

        let outcomes = collect(&mut receiver, &input);
        match outcomes.as_slice() {
            [Outcome::Rejected(rejection)] => {
                assert_eq!(rejection.code(), ErrorCode::LineTooLong);
                assert_eq!(rejection.cause(), Cause::Oversize);
                assert_eq!(rejection.identity(), Some((90312, 907)));
                assert_eq!(rejection.type_name(), None, "typeはprefixの外にある");
                assert_eq!(rejection.counter_field(), Some("oversize_lines"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn oversize_without_a_recoverable_identity_reports_none() {
        let mut receiver = LineReceiver::new(8);
        let outcomes = collect(&mut receiver, b"aaaaaaaaaaaaaaaaaaaa\n");
        match outcomes.as_slice() {
            [Outcome::Rejected(rejection)] => {
                assert_eq!(rejection.code(), ErrorCode::LineTooLong);
                assert_eq!(rejection.identity(), None);
            }
            other => panic!("{other:?}"),
        }
    }

    /// prefixにtypeまで収まれば、§7の`boot`特例を判断できる。
    #[test]
    fn oversize_recovers_a_known_type_when_the_prefix_reaches_it() {
        let mut receiver = LineReceiver::new(64);
        let mut input =
            br#"{"v":1,"sid":1,"id":2,"ts_ms":0,"type":"boot","payload":{"firmware":"0.1.0","board":"esp32","reset_reason":"power_on"}}"#
                .to_vec();
        input.push(b'\n');

        match collect(&mut receiver, &input).as_slice() {
            [Outcome::Rejected(rejection)] => {
                assert_eq!(rejection.identity(), Some((1, 2)));
                assert_eq!(rejection.type_name(), Some("boot"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn resumes_at_the_next_valid_line_after_an_oversize_line() {
        // PING（69 bytes）は収まり、200 byteの行は収まらない容量を選ぶ。
        let mut receiver = LineReceiver::new(80);
        let mut input = vec![b'a'; 200];
        input.push(b'\n');
        input.extend_from_slice(PING);
        input.push(b'\n');

        let outcomes = collect(&mut receiver, &input);
        assert_eq!(outcomes.len(), 2);
        assert!(matches!(outcomes[0], Outcome::Rejected(_)));
        assert!(matches!(outcomes[1], Outcome::Frame(_)));
        assert_eq!(receiver.pending(), 0);
        assert!(!receiver.is_discarding());
    }

    /// 上限ちょうどの行は受理し、1 byte超過は`line_too_long`になる。
    /// **`decode_line`側の行長判定は、この容量では到達しない。**
    #[test]
    fn the_protocol_limit_is_the_only_source_of_line_too_long() {
        let head = br#"{"v":1,"sid":1,"id":1,"ts_ms":0,"type":"ping","payload":{"pad":""#;
        let tail = br#""}}"#;

        let mut at_limit = head.to_vec();
        at_limit.resize(limits::MAX_LINE_BODY_BYTES - tail.len(), b'a');
        at_limit.extend_from_slice(tail);
        assert_eq!(at_limit.len(), limits::MAX_LINE_BODY_BYTES);

        let mut over_limit = at_limit.clone();
        over_limit.insert(head.len(), b'a');

        let mut receiver = LineReceiver::with_protocol_limit();
        let mut with_newline = at_limit.clone();
        with_newline.push(b'\n');
        assert!(matches!(
            collect(&mut receiver, &with_newline).as_slice(),
            [Outcome::Frame(_)]
        ));

        let mut with_newline = over_limit;
        with_newline.push(b'\n');
        match collect(&mut receiver, &with_newline).as_slice() {
            [Outcome::Rejected(rejection)] => {
                assert_eq!(rejection.code(), ErrorCode::LineTooLong);
                assert_eq!(
                    rejection.cause(),
                    Cause::Oversize,
                    "framerが検知したもので、decode_lineが返したものではない"
                );
                assert_eq!(rejection.identity(), Some((1, 1)));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn every_rejection_maps_to_a_status_counter() {
        let mut receiver = LineReceiver::new(32);
        let inputs: [&[u8]; 5] = [
            b"\n",
            b"not json\n",
            b"\xff\xff\n",
            b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
            br#"{"v":2,"sid":1,"id":1,"ts_ms":0,"type":"ping","payload":{}}"#,
        ];
        for input in inputs {
            for outcome in collect(&mut receiver, input) {
                if let Outcome::Rejected(rejection) = outcome {
                    assert!(
                        rejection.counter_field().is_some(),
                        "{} has no status counter",
                        rejection.code()
                    );
                }
            }
        }
    }

    #[test]
    fn drain_terminates_on_empty_input() {
        let mut receiver = LineReceiver::with_protocol_limit();
        assert!(collect(&mut receiver, b"").is_empty());
    }
}
