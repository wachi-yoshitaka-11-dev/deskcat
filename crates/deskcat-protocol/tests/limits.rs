//! 上限の不変条件。
//!
//! string上限を勘で置かないための検査である。各message typeについて、全string fieldを
//! 上限まで、全integerを最大値まで詰めたlineが[`limits::MAX_LINE_BYTES`]に収まることを
//! 確認する。上限定数を動かしたときに、この検査が破綻を検知する。

use deskcat_protocol::{
    Ack, AckStatus, Boot, DisplayStatus, Envelope, ErrorCode, Frame, Hello, HelloReason, Message,
    ProtocolCounters, SensorStatus, ServoStatus, Status, encode_line, limits,
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

fn filled(len: usize) -> String {
    "x".repeat(len)
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
    ]
}

#[test]
fn worst_case_lines_fit_in_the_line_limit() {
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

/// 上限ちょうどのstringは受理し、1 byte超えたら`out_of_range`で拒否する。
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
    let line = encode_line(&over_limit).expect("over limit still fits in a line");
    let err = deskcat_protocol::decode_line(&line).expect_err("over limit is rejected");
    assert_eq!(err.code(), ErrorCode::OutOfRange);
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
