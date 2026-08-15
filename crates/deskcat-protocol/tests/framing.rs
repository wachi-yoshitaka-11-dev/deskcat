//! `tests/fixtures/framing.json`に対するtest。
//!
//! §12が要求するFraming／parse群のうち、byte単位の分割受信とinvalid UTF-8を担う（Issue #10）。
//! 期待値はRust側へ埋め込まず、言語に依存しないJSON fileへ置く。firmware側の実装も
//! 同じfileで検証できるようにするためである（ADR-0001）。
//!
//! **各caseは複数の切り方で回す。**期待する結果はchunkの切り方に依存しない、というのが
//! この群の中心的な不変条件である（受け入れ条件「任意位置で分割したmessageをparseできる」）。

use deskcat_protocol::{Cause, LineReceiver, Outcome, Rejection, limits};
use serde::Deserialize;

const FRAMING: &str = include_str!("fixtures/framing.json");

/// このtestが解釈できるfixture file形式のversion。
const SUPPORTED_FIXTURE_SCHEMA: u32 = 1;

#[derive(Deserialize)]
struct FramingDoc {
    fixture_schema: u32,
    cases: Vec<FramingCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FramingCase {
    name: String,
    #[allow(dead_code, reason = "fixtureの意図を人が読むためのfieldである")]
    note: String,
    /// bodyの最大byte数。省略時は仕様§2の候補値を使う。
    capacity: Option<usize>,
    /// oversize時にidentity復元へ渡すprefixのbyte数。省略時は行buffer全体。
    prefix_budget: Option<usize>,
    chunks: Vec<Chunk>,
    expect: Vec<ExpectedOutcome>,
    expect_pending_bytes: usize,
}

/// 1回のreadに相当するbyte列。`utf8`と`hex`のどちらか一方だけを持つ。
///
/// ASCIIのcaseを読めるまま保ちつつ、invalid UTF-8も表現できるようにするための形である。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Chunk {
    utf8: Option<String>,
    hex: Option<String>,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
enum ExpectedOutcome {
    Frame {
        #[serde(rename = "type")]
        type_name: String,
        sid: u32,
        id: u32,
    },
    Rejected {
        code: String,
        cause: String,
        #[serde(default)]
        identity: Option<[u32; 2]>,
        #[serde(default, rename = "type")]
        type_name: Option<String>,
    },
}

impl Chunk {
    fn bytes(&self, case: &str) -> Vec<u8> {
        match (&self.utf8, &self.hex) {
            (Some(text), None) => text.as_bytes().to_vec(),
            (None, Some(hex)) => decode_hex(hex, case),
            _ => panic!("{case}: chunkは`utf8`と`hex`のどちらか一方だけを持つ"),
        }
    }
}

fn decode_hex(hex: &str, case: &str) -> Vec<u8> {
    assert!(hex.len().is_multiple_of(2), "{case}: hexの桁数が奇数である");
    hex.as_bytes()
        .chunks(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("hexはASCIIである");
            u8::from_str_radix(text, 16).unwrap_or_else(|_| panic!("{case}: hexではない: {text}"))
        })
        .collect()
}

fn framing_doc() -> FramingDoc {
    let doc: FramingDoc = serde_json::from_str(FRAMING).expect("framing.json parses");
    assert_eq!(
        doc.fixture_schema, SUPPORTED_FIXTURE_SCHEMA,
        "unsupported fixture_schema in framing.json"
    );
    doc
}

fn cause_name(cause: Cause) -> &'static str {
    match cause {
        Cause::InvalidUtf8 { .. } => "invalid_utf8",
        Cause::Decode => "decode",
        Cause::Oversize => "oversize",
        other => panic!("fixture runnerが知らないcause: {other:?}"),
    }
}

fn observed(outcome: &Outcome) -> ExpectedOutcome {
    match outcome {
        Outcome::Frame(frame) => ExpectedOutcome::Frame {
            type_name: frame.message.type_str().to_owned(),
            sid: frame.envelope.sid,
            id: frame.envelope.id,
        },
        Outcome::Rejected(rejection) => ExpectedOutcome::Rejected {
            code: rejection.code().as_str().to_owned(),
            cause: cause_name(rejection.cause()).to_owned(),
            identity: rejection.identity().map(|(sid, id)| [sid, id]),
            type_name: rejection.type_name().map(str::to_owned),
        },
    }
}

/// 1つのcaseを、与えられたchunk列で回す。
fn run(case: &FramingCase, chunks: &[Vec<u8>], label: &str) {
    let mut receiver = LineReceiver::new(case.capacity.unwrap_or(limits::MAX_LINE_BODY_BYTES));
    if let Some(budget) = case.prefix_budget {
        receiver = receiver.with_prefix_budget(budget);
    }

    let mut outcomes = Vec::new();
    for chunk in chunks {
        receiver.drain(chunk, |outcome| outcomes.push(observed(&outcome)));
    }

    assert_eq!(
        outcomes, case.expect,
        "{}: {label}: 結果の並びが期待と違う",
        case.name
    );
    assert_eq!(
        receiver.pending(),
        case.expect_pending_bytes,
        "{}: {label}: bufferの残り",
        case.name
    );
}

/// fixtureが定めた切り方、まとめて1回、1 byteずつ、そして全ての2分割で回す。
#[test]
fn framing_fixtures_hold_for_every_chunking() {
    for case in framing_doc().cases {
        let chunks: Vec<Vec<u8>> = case
            .chunks
            .iter()
            .map(|chunk| chunk.bytes(&case.name))
            .collect();
        run(&case, &chunks, "as given");

        let whole: Vec<u8> = chunks.concat();
        run(&case, std::slice::from_ref(&whole), "single chunk");

        let single_bytes: Vec<Vec<u8>> = whole.iter().map(|byte| vec![*byte]).collect();
        run(&case, &single_bytes, "one byte at a time");

        for split in 0..=whole.len() {
            let halves = vec![whole[..split].to_vec(), whole[split..].to_vec()];
            run(&case, &halves, &format!("split at {split}"));
        }
    }
}

#[test]
fn framing_fixtures_are_not_empty() {
    assert!(
        framing_doc().cases.len() >= 20,
        "framing fixture cases were removed"
    );
}

#[test]
fn framing_fixture_case_names_are_unique() {
    let mut names: Vec<String> = framing_doc()
        .cases
        .into_iter()
        .map(|case| case.name)
        .collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len(), "duplicate framing fixture case name");
}

/// chunkは`utf8`と`hex`のどちらか一方だけを持つ。両方書くと正本が2つになる。
#[test]
fn framing_fixture_chunks_have_exactly_one_encoding() {
    for case in framing_doc().cases {
        for chunk in &case.chunks {
            assert!(
                chunk.utf8.is_some() ^ chunk.hex.is_some(),
                "{}: chunkは`utf8`と`hex`のどちらか一方だけを持つ",
                case.name
            );
        }
    }
}

/// 拒否はすべて`status`のcounterへ対応づく。
#[test]
fn framing_fixture_rejections_map_to_a_status_counter() {
    for case in framing_doc().cases {
        let mut receiver = LineReceiver::new(case.capacity.unwrap_or(limits::MAX_LINE_BODY_BYTES));
        for chunk in &case.chunks {
            receiver.drain(&chunk.bytes(&case.name), |outcome| {
                if let Outcome::Rejected(rejection) = outcome {
                    let rejection: Rejection = rejection;
                    assert!(
                        rejection.counter_field().is_some(),
                        "{}: {} has no status counter",
                        case.name,
                        rejection.code()
                    );
                }
            });
        }
    }
}
