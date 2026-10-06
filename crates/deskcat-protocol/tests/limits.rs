//! 上限の不変条件。
//!
//! string上限を勘で置かないための検査である。各message typeについて、全string fieldを
//! 上限まで、全integerを最大値まで詰めたlineが[`limits::MAX_LINE_BYTES`]に収まることを
//! 確認する。上限定数を動かしたときに、この検査が破綻を検知する。

use deskcat_protocol::{
    Ack, AckStatus, Boot, DisplayStatus, Envelope, Environment, ErrorCode, ExpressionName,
    FiniteF32, Frame, HeadTouched, Hello, HelloReason, Lifted, Message, ProtocolCounters,
    SensorStatus, ServoStatus, SetExpression, ShowText, Status, encode_line, limits,
};

/// 最悪のenvelope。`v`は受理される値ではなく、宣言した幅の最大値を使う。
fn worst_case_envelope() -> Envelope {
    Envelope {
        v: u16::MAX,
        sid: u32::MAX,
        id: u32::MAX,
        ts_ms: u64::MAX,
    }
}

/// JSON encodeで長さが変わらない文字で埋める。
fn filled(len: usize) -> String {
    "x".repeat(len)
}

/// JSON encodeで1 byteが6 byteへ広がる文字で埋める。
///
/// `U+0001`は、6文字のunicode escapeへ広げられる。引用符やbackslashは
/// 2 byteへしか広がらないため、膨張率が最大のこの文字をworst caseに使う。
fn filled_with_escapes(len: usize) -> String {
    "\u{1}".repeat(len)
}

fn worst_case_messages() -> Vec<Message> {
    vec![
        Message::Boot(Boot {
            firmware: filled(limits::MAX_FIRMWARE_BYTES),
            board: filled(limits::MAX_BOARD_BYTES),
            reset_reason: filled(limits::MAX_RESET_REASON_BYTES),
        }),
        Message::Hello(Hello {
            host: filled(limits::MAX_HOST_BYTES),
            version: filled(limits::MAX_VERSION_BYTES),
            // 最も長い列挙値を選ぶ。
            reason: HelloReason::PortReopen,
        }),
        Message::Ping,
        Message::GetStatus,
        Message::Status(Box::new(Status {
            firmware: filled(limits::MAX_FIRMWARE_BYTES),
            reset_reason: filled(limits::MAX_RESET_REASON_BYTES),
            display: DisplayStatus {
                state: filled(limits::MAX_STATE_NAME_BYTES),
                expression: filled(limits::MAX_STATE_NAME_BYTES),
            },
            servo: ServoStatus {
                state: filled(limits::MAX_STATE_NAME_BYTES),
            },
            sensors: SensorStatus {
                touch: filled(limits::MAX_STATE_NAME_BYTES),
                acceleration: filled(limits::MAX_STATE_NAME_BYTES),
                environment: filled(limits::MAX_STATE_NAME_BYTES),
            },
            protocol: ProtocolCounters {
                parse_errors: u32::MAX,
                invalid_payloads: u32::MAX,
                unsupported_versions: u32::MAX,
                oversize_lines: u32::MAX,
                unknown_types: u32::MAX,
                rate_limited: u32::MAX,
                busy: u32::MAX,
                out_of_range: u32::MAX,
                stale_sessions: u32::MAX,
                hardware_unavailable: u32::MAX,
                duplicate_expired: u32::MAX,
                session_switches: u32::MAX,
                suppressed_responses: u32::MAX,
            },
        })),
        Message::Ack(Ack {
            reply_sid: u32::MAX,
            reply_to: u32::MAX,
            status: AckStatus::Rejected,
            // 最も長いerror codeを選ぶ。
            code: Some(ErrorCode::HardwareUnavailable),
            detail: Some(filled(limits::MAX_DETAIL_BYTES)),
        }),
        Message::SetExpression(SetExpression {
            // 最も長い列挙値を選ぶ。
            name: ExpressionName::Surprised,
            transition_ms: u32::MAX,
        }),
        worst_case_show_text(limits::MAX_TEXT_BYTES),
        Message::HeadTouched(HeadTouched {
            duration_ms: u32::MAX,
        }),
        Message::Tapped,
        Message::Lifted(Lifted {
            duration_ms: u32::MAX,
        }),
        Message::Environment(Environment {
            temperature_c: Some(longest_finite_f32()),
            humidity_pct: Some(longest_finite_f32()),
            pressure_hpa: Some(longest_finite_f32()),
        }),
    ]
}

/// `text`を`text_bytes`まで詰め、`duration_ms`を最大値にした`show_text`。
fn worst_case_show_text(text_bytes: usize) -> Message {
    Message::ShowText(ShowText {
        text: filled(text_bytes),
        duration_ms: u32::MAX,
    })
}

/// 長くencodeされる有限の`f32`。
///
/// 手で選んだ候補を実際にencodeし、その中で最長のものを選ぶ。**全有限値の中の最長である
/// ことは示していない。**
fn longest_finite_f32() -> FiniteF32 {
    [
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        -1.175_494_2e-38,
        -1.234_567_9e-7,
        -1.234_567_9e7,
        -123_456.79,
    ]
    .into_iter()
    .map(|value| FiniteF32::new(value).expect("finite"))
    .max_by_key(|value| serde_json::to_string(value).expect("serializes").len())
    .expect("candidates are not empty")
}

/// escapeが起きない文字であれば、全fieldを上限まで詰めても行長に収まる。
///
/// **これは「string上限を守れば行長も収まる」という意味ではない。**escapeを含む場合は
/// 収まらないことがあり、それは下の`strings_needing_json_escapes_can_exceed_the_line_limit`が
/// 固定している。
#[test]
fn worst_case_lines_without_escapes_fit_in_the_line_limit() {
    for message in worst_case_messages() {
        let type_name = message.type_str();
        let frame = Frame::new(worst_case_envelope(), message);

        let line = encode_line(&frame)
            .unwrap_or_else(|err| panic!("{type_name}: worst case exceeds the line limit: {err}"));

        assert!(
            line.len() <= limits::MAX_LINE_BYTES,
            "{type_name}: worst case is {} bytes, limit is {}",
            line.len(),
            limits::MAX_LINE_BYTES
        );
    }
}

/// string上限を全て満たしていても、JSON escapeで膨らんだ行は上限を超えうる。
///
/// このとき`encode_line`が[`ErrorCode::LineTooLong`]を返し、上限を超えた行が黙って
/// wireへ出ることはない。**field上限は行長の十分条件ではない**という事実をここで固定する。
/// escape後のwire sizeまで含めた上限の確定は`PROTO-TBD-002`に含める。
#[test]
fn strings_needing_json_escapes_can_exceed_the_line_limit() {
    let message = Message::Status(Box::new(Status {
        firmware: filled_with_escapes(limits::MAX_FIRMWARE_BYTES),
        reset_reason: filled_with_escapes(limits::MAX_RESET_REASON_BYTES),
        display: DisplayStatus {
            state: filled_with_escapes(limits::MAX_STATE_NAME_BYTES),
            expression: filled_with_escapes(limits::MAX_STATE_NAME_BYTES),
        },
        servo: ServoStatus {
            state: filled_with_escapes(limits::MAX_STATE_NAME_BYTES),
        },
        sensors: SensorStatus {
            touch: filled_with_escapes(limits::MAX_STATE_NAME_BYTES),
            acceleration: filled_with_escapes(limits::MAX_STATE_NAME_BYTES),
            environment: filled_with_escapes(limits::MAX_STATE_NAME_BYTES),
        },
        protocol: ProtocolCounters::default(),
    }));

    // 各fieldは上限内であり、値の範囲検査は通る。
    message
        .check_bounds()
        .expect("every field is within its byte limit");

    let frame = Frame::new(
        Envelope {
            v: limits::PROTOCOL_VERSION,
            sid: 1,
            id: 1,
            ts_ms: 0,
        },
        message,
    );

    // それでもencode後の行は上限を超え、errorとして検出される。
    let err = encode_line(&frame).expect_err("escaped worst case must not fit silently");
    assert_eq!(err.code(), ErrorCode::LineTooLong);
}

/// 上限ちょうどのstringは受理し、1 byte超えたら`out_of_range`で拒否する。
///
/// **拒否は送信側で起きる。**`encode_line`が`check_bounds`を通すため、上限を超えたframeは
/// そもそもwireへ出ない。以前はencodeが通り、同じcrateのdecoderだけが拒否していた。
/// 公開APIが自分で受理しないframeを作れる状態だったのを、ここで固定し直している。
#[test]
fn string_limits_are_enforced_at_the_boundary() {
    let at_limit = Frame::new(
        Envelope {
            v: limits::PROTOCOL_VERSION,
            sid: 1,
            id: 1,
            ts_ms: 0,
        },
        Message::Boot(Boot {
            firmware: filled(limits::MAX_FIRMWARE_BYTES),
            board: filled(limits::MAX_BOARD_BYTES),
            reset_reason: filled(limits::MAX_RESET_REASON_BYTES),
        }),
    );
    let line = encode_line(&at_limit).expect("at limit encodes");
    assert_eq!(
        deskcat_protocol::decode_line(&line).expect("at limit decodes"),
        at_limit
    );

    let over_limit = Frame::new(
        at_limit.envelope,
        Message::Boot(Boot {
            firmware: filled(limits::MAX_FIRMWARE_BYTES + 1),
            board: filled(limits::MAX_BOARD_BYTES),
            reset_reason: filled(limits::MAX_RESET_REASON_BYTES),
        }),
    );
    let err = encode_line(&over_limit).expect_err("over limit must not reach the wire");
    assert_eq!(err.code(), ErrorCode::OutOfRange);
}

/// 上限超過について、`encode_line`と`decode_line`が同じcodeを返す。
///
/// **encodeに検査を足したことで、decode側の判定が要らなくなったわけではない。**
/// 他実装が送ってきたlineは受信側でしか止められない。送信側の検査は追加であって
/// 置き換えではないことを、両経路で同じcodeが返ることとして固定する。
#[test]
fn encode_and_decode_reject_over_limit_fields_with_the_same_code() {
    let over_limit = Frame::new(
        Envelope {
            v: limits::PROTOCOL_VERSION,
            sid: 1,
            id: 1,
            ts_ms: 0,
        },
        Message::Boot(Boot {
            firmware: filled(limits::MAX_FIRMWARE_BYTES + 1),
            board: filled(limits::MAX_BOARD_BYTES),
            reset_reason: filled(limits::MAX_RESET_REASON_BYTES),
        }),
    );
    let encode_err = encode_line(&over_limit).expect_err("over limit must not reach the wire");
    assert_eq!(encode_err.code(), ErrorCode::OutOfRange);

    // 同じ内容をwire lineとして手で組む。encodeできない以上、こうしないと
    // decode側の経路を通せない。
    let over_limit_line = format!(
        r#"{{"v":1,"sid":1,"id":1,"ts_ms":0,"type":"boot","payload":{{"firmware":"{}","board":"{}","reset_reason":"{}"}}}}"#,
        filled(limits::MAX_FIRMWARE_BYTES + 1),
        filled(limits::MAX_BOARD_BYTES),
        filled(limits::MAX_RESET_REASON_BYTES),
    );
    let decode_err =
        deskcat_protocol::decode_line(&over_limit_line).expect_err("over limit is rejected");
    assert_eq!(decode_err.code(), encode_err.code());
}

/// `rejected`で`code`が無いACKは、encode時点で`invalid_payload`として拒否する。
///
/// §6は「`rejected`の場合はmachine-readableな`code`を含める」と定めている。decode側は
/// 以前からこれを検査していた。encode側にも同じ検査を通し、規則を破ったACKを
/// wireへ出さない。
#[test]
fn rejected_acks_without_a_code_are_refused_by_encode() {
    let frame = Frame::new(
        Envelope {
            v: limits::PROTOCOL_VERSION,
            sid: 1,
            id: 1,
            ts_ms: 0,
        },
        Message::Ack(Ack {
            reply_sid: 1,
            reply_to: 1,
            status: AckStatus::Rejected,
            code: None,
            detail: None,
        }),
    );

    let err = encode_line(&frame).expect_err("a rejected ack without a code must not be encoded");
    assert_eq!(err.code(), ErrorCode::InvalidPayload);
}

/// `ok`に`code`が付いていても拒否しない。
///
/// §6が要求するのは「`rejected`の場合はcodeを含める」ことだけである。**これ以上に
/// 厳しくすると、仕様上妥当なpeerが送るACKをencodeできなくなる。**送信前検証を
/// 足したことで規則が強くなっていないことを、ここで固定する。
#[test]
fn ok_acks_may_carry_a_code() {
    let frame = Frame::new(
        Envelope {
            v: limits::PROTOCOL_VERSION,
            sid: 1,
            id: 1,
            ts_ms: 0,
        },
        Message::Ack(Ack {
            reply_sid: 1,
            reply_to: 1,
            status: AckStatus::Ok,
            code: Some(ErrorCode::Busy),
            detail: None,
        }),
    );

    let line = encode_line(&frame).expect("an ok ack with a code is still valid");
    assert_eq!(
        deskcat_protocol::decode_line(&line).expect("and decodes back"),
        frame
    );
}

/// shape違反と上限超過を同時に持つframeでは、encodeとdecodeが同じcodeを返す。
///
/// **検証順序を揃えていることの検査である。**`check_bounds`を先に呼ぶと、encodeだけが
/// `out_of_range`を返し、同じ内容のlineをdecodeすると`invalid_payload`になる。
/// 送受信で分類が食い違うと、counterの計上先も食い違う。
#[test]
fn shape_is_checked_before_bounds_on_both_paths() {
    let frame = Frame::new(
        Envelope {
            v: limits::PROTOCOL_VERSION,
            sid: 1,
            id: 1,
            ts_ms: 0,
        },
        Message::Ack(Ack {
            reply_sid: 1,
            reply_to: 1,
            status: AckStatus::Rejected,
            code: None,
            detail: Some(filled(limits::MAX_DETAIL_BYTES + 1)),
        }),
    );

    let encode_err = encode_line(&frame).expect_err("both rules are violated");
    assert_eq!(encode_err.code(), ErrorCode::InvalidPayload);

    // 同じ内容をwire lineとして手で組み、decode側の分類と突き合わせる。
    let line = format!(
        r#"{{"v":1,"sid":1,"id":1,"ts_ms":0,"type":"ack","payload":{{"reply_sid":1,"reply_to":1,"status":"rejected","detail":"{}"}}}}"#,
        filled(limits::MAX_DETAIL_BYTES + 1),
    );
    let decode_err = deskcat_protocol::decode_line(&line).expect_err("both rules are violated");
    assert_eq!(decode_err.code(), encode_err.code());
}

/// 改行を含めてちょうど上限のlineは受理し、1 byte超えたら`line_too_long`で拒否する。
#[test]
fn line_length_is_enforced_at_the_boundary() {
    let head = r#"{"v":1,"sid":1,"id":1,"ts_ms":0,"type":"boot","payload":{"firmware":"0.1.0","board":"esp32","reset_reason":"power_on","unknown_pad":""#;
    let tail = r#""}}"#;

    // 改行1 byteを含めてちょうど上限になるよう詰める。
    let pad = limits::MAX_LINE_BYTES - 1 - head.len() - tail.len();
    let at_limit = format!("{head}{}{tail}\n", "a".repeat(pad));
    assert_eq!(at_limit.len(), limits::MAX_LINE_BYTES);
    deskcat_protocol::decode_line(&at_limit).expect("a line at the limit is accepted");

    let over_limit = format!("{head}{}{tail}\n", "a".repeat(pad + 1));
    assert_eq!(over_limit.len(), limits::MAX_LINE_BYTES + 1);
    let err = deskcat_protocol::decode_line(&over_limit).expect_err("one byte over is rejected");
    assert_eq!(err.code(), ErrorCode::LineTooLong);
}

/// 改行が無い入力でも、改行1 byteを数に入れて判定する。
#[test]
fn the_newline_counts_toward_the_line_limit() {
    let head = r#"{"v":1,"sid":1,"id":1,"ts_ms":0,"type":"boot","payload":{"firmware":"0.1.0","board":"esp32","reset_reason":"power_on","unknown_pad":""#;
    let tail = r#""}}"#;

    let pad = limits::MAX_LINE_BYTES - head.len() - tail.len();
    let without_newline = format!("{head}{}{tail}", "a".repeat(pad));
    assert_eq!(without_newline.len(), limits::MAX_LINE_BYTES);

    let err = deskcat_protocol::decode_line(&without_newline)
        .expect_err("the newline is counted even when it is absent");
    assert_eq!(err.code(), ErrorCode::LineTooLong);
}

/// `MAX_TEXT_BYTES`は行長から計算した最大値である。
///
/// 上の`worst_case_lines_without_escapes_fit_in_the_line_limit`は「収まる」ことだけを見る。
/// ここでは**ちょうど`MAX_LINE_BYTES`になり、1 byte増やすと収まらない**ことを見る。
/// 勘で置いた値ではなく、行長とenvelopeのworst caseから決まる値であることの検査である。
#[test]
fn max_text_bytes_is_the_largest_that_fits_in_the_line_limit() {
    let at_limit = Frame::new(
        worst_case_envelope(),
        worst_case_show_text(limits::MAX_TEXT_BYTES),
    );
    let line = encode_line(&at_limit).expect("the worst case at the text limit fits");
    assert_eq!(line.len(), limits::MAX_LINE_BYTES);

    // textを1 byte増やした同じworst caseを、wire lineとして手で組む。encode_lineはbyte上限で
    // 先に止めるため、こうしないと行長を確かめられない。decodeは行長を最初に見る。
    let one_more = format!(
        "{{\"v\":{},\"sid\":{},\"id\":{},\"ts_ms\":{},\"type\":\"show_text\",\"payload\":{{\"text\":\"{}\",\"duration_ms\":{}}}}}\n",
        u16::MAX,
        u32::MAX,
        u32::MAX,
        u64::MAX,
        filled(limits::MAX_TEXT_BYTES + 1),
        u32::MAX,
    );
    assert_eq!(one_more.len(), limits::MAX_LINE_BYTES + 1);
    let err = deskcat_protocol::decode_line(&one_more).expect_err("one byte over the line limit");
    assert_eq!(err.code(), ErrorCode::LineTooLong);
}

/// `show_text`のbyte上限と制御文字は、encodeとdecodeで同じ`out_of_range`になる。
///
/// 送信側で止めるだけでなく、他実装が送ってきたlineを受信側でも止める。
#[test]
fn show_text_bounds_are_enforced_on_both_paths() {
    let envelope = Envelope {
        v: limits::PROTOCOL_VERSION,
        sid: 1,
        id: 1,
        ts_ms: 0,
    };

    for text in [filled(limits::MAX_TEXT_BYTES + 1), "a\nb".to_owned()] {
        let frame = Frame::new(
            envelope,
            Message::ShowText(ShowText {
                text: text.clone(),
                duration_ms: 0,
            }),
        );
        let encode_err = encode_line(&frame).expect_err("must not reach the wire");
        assert_eq!(encode_err.code(), ErrorCode::OutOfRange);

        let line = format!(
            r#"{{"v":1,"sid":1,"id":1,"ts_ms":0,"type":"show_text","payload":{{"text":{},"duration_ms":0}}}}"#,
            serde_json::to_string(&text).expect("a string serializes"),
        );
        let decode_err = deskcat_protocol::decode_line(&line).expect_err("is rejected");
        assert_eq!(decode_err.code(), encode_err.code());
    }
}

/// `FiniteF32`はNaNと無限大を持てない。encodeへ非有限値が届く経路が無いことの検査である。
#[test]
fn finite_f32_refuses_non_finite_values() {
    assert!(FiniteF32::new(f32::NAN).is_none());
    assert!(FiniteF32::new(f32::INFINITY).is_none());
    assert!(FiniteF32::new(f32::NEG_INFINITY).is_none());
    assert_eq!(FiniteF32::new(27.4).map(FiniteF32::get), Some(27.4));
}
