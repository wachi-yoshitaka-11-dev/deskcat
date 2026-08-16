//! 共有conformance fixtureに対するtest。
//!
//! fixtureは`tests/fixtures/`のJSON fileであり、Rust側に期待値を埋め込まない。
//! firmware側の実装（Issue #10）が同じfileで検証できるようにするためである
//! （ADR-0001「両側が共通JSON fixtureとprotocol conformance testに合格しなければならない」）。

use deskcat_protocol::{Cause, LineReceiver, Message, Outcome, decode_line, encode_line};
use serde::Deserialize;

const VALID: &str = include_str!("fixtures/valid.json");
const INVALID: &str = include_str!("fixtures/invalid.json");

/// このtestが解釈できるfixture file形式のversion。
const SUPPORTED_FIXTURE_SCHEMA: u32 = 1;

#[derive(Deserialize)]
struct ValidDoc {
    fixture_schema: u32,
    cases: Vec<ValidCase>,
}

#[derive(Deserialize)]
struct ValidCase {
    name: String,
    line: String,
    canonical: bool,
    expect: Expect,
}

#[derive(Deserialize)]
struct Expect {
    #[serde(rename = "type")]
    type_name: String,
    sid: u32,
    id: u32,
}

#[derive(Deserialize)]
struct InvalidDoc {
    fixture_schema: u32,
    cases: Vec<InvalidCase>,
}

#[derive(Deserialize)]
struct InvalidCase {
    name: String,
    line: String,
    expect_error: String,
}

fn valid_doc() -> ValidDoc {
    let doc: ValidDoc = serde_json::from_str(VALID).expect("valid.json parses");
    assert_eq!(
        doc.fixture_schema, SUPPORTED_FIXTURE_SCHEMA,
        "unsupported fixture_schema in valid.json"
    );
    doc
}

fn invalid_doc() -> InvalidDoc {
    let doc: InvalidDoc = serde_json::from_str(INVALID).expect("invalid.json parses");
    assert_eq!(
        doc.fixture_schema, SUPPORTED_FIXTURE_SCHEMA,
        "unsupported fixture_schema in invalid.json"
    );
    doc
}

/// fixtureが空でも成功してしまうtestにしない。
///
/// 件数の下限は、#9の受け入れ条件が要求するcase群を数えたものである。
/// #10と#12がcaseを追加するため上限は設けない。
#[test]
fn fixtures_are_not_empty() {
    assert!(
        valid_doc().cases.len() >= 15,
        "valid fixture cases were removed"
    );
    assert!(
        invalid_doc().cases.len() >= 28,
        "invalid fixture cases were removed"
    );
}

#[test]
fn fixture_case_names_are_unique() {
    let mut names: Vec<String> = valid_doc()
        .cases
        .into_iter()
        .map(|case| case.name)
        .chain(invalid_doc().cases.into_iter().map(|case| case.name))
        .collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len(), "duplicate fixture case name");
}

#[test]
fn valid_fixtures_decode_as_expected() {
    for case in valid_doc().cases {
        let frame = decode_line(&case.line).unwrap_or_else(|err| panic!("{}: {err}", case.name));

        assert_eq!(
            frame.message.type_str(),
            case.expect.type_name,
            "{}: message type",
            case.name
        );
        assert_eq!(
            frame.envelope.identity(),
            (case.expect.sid, case.expect.id),
            "{}: (sid, id)",
            case.name
        );
    }
}

/// `line → decode → encode → decode`が同じ値に戻ることを確認する。
///
/// byte一致まで要求するのは`canonical`なcaseだけである。未知fieldを含むcaseや
/// CRLFのcaseは、再encodeで正規形になるため値の一致だけを要求する。
#[test]
fn valid_fixtures_round_trip() {
    for case in valid_doc().cases {
        let frame = decode_line(&case.line).unwrap_or_else(|err| panic!("{}: {err}", case.name));

        let encoded =
            encode_line(&frame).unwrap_or_else(|err| panic!("{}: encode: {err}", case.name));
        let decoded = decode_line(&encoded)
            .unwrap_or_else(|err| panic!("{}: decode after encode: {err}", case.name));

        assert_eq!(
            frame, decoded,
            "{}: round trip changed the value",
            case.name
        );

        if case.canonical {
            assert_eq!(encoded, case.line, "{}: canonical bytes", case.name);
        }
    }
}

#[test]
fn invalid_fixtures_fail_with_the_expected_code() {
    for case in invalid_doc().cases {
        let err = decode_line(&case.line)
            .err()
            .unwrap_or_else(|| panic!("{}: expected a decode error", case.name));

        assert_eq!(
            err.code().as_str(),
            case.expect_error,
            "{}: error code (detail: {})",
            case.name,
            err.detail()
        );
    }
}

/// 分類が`status`のcounterへ対応づけられていることを確認する。
///
/// AGENTS.md「エラーを握りつぶさず、分類、ログ、カウンタを用意する」に対応する。
#[test]
fn invalid_fixtures_map_to_a_status_counter() {
    for case in invalid_doc().cases {
        let err = decode_line(&case.line)
            .err()
            .unwrap_or_else(|| panic!("{}: expected a decode error", case.name));

        assert!(
            err.counter_field().is_some(),
            "{}: {} has no status counter",
            case.name,
            err.code()
        );
    }
}

// ---------------------------------------------------------------------------
// byte経路（Issue #10）を通しても、`decode_line`と同じ結論になることを固定する。
//
// #10が足したのはbyte列からlineを組み立てる層だけであり、schemaの解釈は変えていない。
// 同じfixtureを両方の入口へ流して、そのことを検査する。
// ---------------------------------------------------------------------------

/// 入力を受信器へ流し、結果を順に集める。
fn receive(receiver: &mut LineReceiver, input: &[u8]) -> Vec<Outcome> {
    let mut outcomes = Vec::new();
    receiver.drain(input, |outcome| outcomes.push(outcome));
    outcomes
}

#[test]
fn valid_fixtures_decode_through_the_byte_path() {
    for case in valid_doc().cases {
        let expected = decode_line(&case.line).unwrap_or_else(|err| panic!("{}: {err}", case.name));

        let mut receiver = LineReceiver::with_protocol_limit();
        match receive(&mut receiver, case.line.as_bytes()).as_slice() {
            [Outcome::Frame(frame)] => assert_eq!(*frame, expected, "{}", case.name),
            other => panic!("{}: {other:?}", case.name),
        }
        assert_eq!(receiver.pending(), 0, "{}: bufferが残っている", case.name);
    }
}

/// すべてのbyte境界で分割しても同じframeになる（§12「すべてのbyte境界で分割したmessage」）。
#[test]
fn valid_fixtures_decode_when_split_at_every_byte_boundary() {
    for case in valid_doc().cases {
        let expected = decode_line(&case.line).unwrap_or_else(|err| panic!("{}: {err}", case.name));
        let bytes = case.line.as_bytes();

        for split in 0..=bytes.len() {
            let mut receiver = LineReceiver::with_protocol_limit();
            let mut outcomes = receive(&mut receiver, &bytes[..split]);
            outcomes.extend(receive(&mut receiver, &bytes[split..]));

            match outcomes.as_slice() {
                [Outcome::Frame(frame)] => {
                    assert_eq!(*frame, expected, "{}: split {split}", case.name);
                }
                other => panic!("{}: split {split}: {other:?}", case.name),
            }
            // frameが正しくても、bufferにbyteが残る分割固有の不具合がありうる。
            assert_eq!(
                receiver.pending(),
                0,
                "{}: split {split}: bufferが残っている",
                case.name
            );
        }
    }
}

/// 1回のreadに全fixtureのlineを詰めても、順番どおりに取り出せる。
#[test]
fn valid_fixtures_decode_when_concatenated_into_one_read() {
    let cases = valid_doc().cases;
    let mut input = String::new();
    for case in &cases {
        input.push_str(&case.line);
    }

    let mut receiver = LineReceiver::with_protocol_limit();
    let outcomes = receive(&mut receiver, input.as_bytes());
    assert_eq!(outcomes.len(), cases.len(), "取り出せた件数");

    for (outcome, case) in outcomes.iter().zip(&cases) {
        match outcome {
            Outcome::Frame(frame) => {
                assert_eq!(
                    frame.message.type_str(),
                    case.expect.type_name,
                    "{}",
                    case.name
                );
                assert_eq!(
                    frame.envelope.identity(),
                    (case.expect.sid, case.expect.id),
                    "{}",
                    case.name
                );
            }
            other @ Outcome::Rejected(_) => panic!("{}: {other:?}", case.name),
        }
    }
    assert_eq!(receiver.pending(), 0);
}

#[test]
fn invalid_fixtures_fail_the_same_way_through_the_byte_path() {
    for case in invalid_doc().cases {
        let mut receiver = LineReceiver::with_protocol_limit();
        match receive(&mut receiver, case.line.as_bytes()).as_slice() {
            [Outcome::Rejected(rejection)] => {
                assert_eq!(
                    rejection.code().as_str(),
                    case.expect_error,
                    "{}: error code (detail: {})",
                    case.name,
                    rejection.detail()
                );
                // 行長超過は必ずframerが検知したものである。`decode_line`側の
                // 行長判定は、この容量では到達しない。
                if rejection.code().as_str() == "line_too_long" {
                    assert_eq!(rejection.cause(), Cause::Oversize, "{}", case.name);
                }
            }
            other => panic!("{}: {other:?}", case.name),
        }
    }
}

/// `type_str`と`known_type_name`が同じ綴りを使っていることを、fixtureの`expect.type`で固定する。
#[test]
fn fixture_type_names_round_trip_through_known_type_name() {
    for case in valid_doc().cases {
        assert_eq!(
            Message::known_type_name(case.expect.type_name.as_bytes()),
            Some(case.expect.type_name.as_str()),
            "{}: 既知typeとして解決できない",
            case.name
        );
    }
}
