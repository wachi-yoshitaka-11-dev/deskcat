//! 不正・高rateの入力に対する受信器の上限と復帰のtest（Issue #24）。
//!
//! `framing.rs`は1件ずつの境界を、`conformance.rs`は1行ずつの分類を確かめる。このtestが足すのは、
//! **混ぜて大量に流したときの性質**である。
//!
//! - どれだけ流しても、組み立て中のbyte数が`capacity()`を超えない（bufferを伸ばさない）。
//! - 流した行の数と、得た結果（`Frame`か`Rejected`）の数が一致する（黙って落とさない）。
//! - 拒否は、期待した`ErrorCode`へ分類され、`counter_field()`が返すfieldへ集計できる。
//! - 流し終えた後に、有効な行が1件、通常どおりdecodeできる（復帰）。
//!
//! **このtestは、ESP32 firmwareの受信（UART、ring buffer、main loopの読み取り間隔）を試さない。**
//! hostの`LineReceiver`だけが対象であり、`status.protocol`のcounterはfirmwareが増やさない
//! （`firmware/esp32/src/pi_link.rs`のmodule doc）。実機の入力の試験は
//! `docs/runbooks/issue-24-fault-window.md`が持つ。
//!
//! 乱数は固定seedのxorshiftである。失敗を再現できるようにするため、実行ごとに変えない。

use deskcat_protocol::{Cause, ErrorCode, LineReceiver, Outcome, limits};
use serde::Deserialize;
use std::collections::BTreeMap;

const VALID: &str = include_str!("fixtures/valid.json");
const INVALID: &str = include_str!("fixtures/invalid.json");

#[derive(Deserialize)]
struct Doc<C> {
    cases: Vec<C>,
}

#[derive(Deserialize)]
struct ValidCase {
    name: String,
    line: String,
}

#[derive(Deserialize)]
struct InvalidCase {
    line: String,
    expect_error: String,
}

/// 期待する結果。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Expect {
    Frame,
    Rejected(&'static str),
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).expect("n fits in usize")
    }
}

fn valid_cases() -> Vec<ValidCase> {
    serde_json::from_str::<Doc<ValidCase>>(VALID)
        .expect("valid.json parses")
        .cases
}

fn invalid_cases() -> Vec<InvalidCase> {
    serde_json::from_str::<Doc<InvalidCase>>(INVALID)
        .expect("invalid.json parses")
        .cases
}

/// `ErrorCode`の文字列表現から、期待する`'static`の名前へ引く。
fn code_name(code: &str) -> &'static str {
    ErrorCode::ALL
        .iter()
        .map(|c| c.as_str())
        .find(|name| *name == code)
        .unwrap_or_else(|| panic!("unknown error code in fixture: {code}"))
}

/// 1行ぶんのbyte列と、その行で得るはずの結果を作る。
fn build_stream(rng: &mut Rng, rounds: usize) -> (Vec<u8>, Vec<Expect>) {
    let valid = valid_cases();
    let invalid = invalid_cases();
    let mut bytes = Vec::new();
    let mut expected = Vec::new();

    for _ in 0..rounds {
        match rng.below(5) {
            // 有効な行
            0 | 1 => {
                let case = &valid[rng.below(valid.len())];
                bytes.extend_from_slice(case.line.as_bytes());
                expected.push(Expect::Frame);
            }
            // 分類済みの不正な行
            2 => {
                let case = &invalid[rng.below(invalid.len())];
                bytes.extend_from_slice(case.line.as_bytes());
                expected.push(Expect::Rejected(code_name(&case.expect_error)));
            }
            // 無効なUTF-8を含む乱数の行。0xFFはUTF-8に現れないbyteで、必ず拒否される。
            3 => {
                let len = 1 + rng.below(2000);
                bytes.push(0xFF);
                for _ in 0..len {
                    // 改行を含めない。含めると、1つの行が複数の行になる。
                    let b = u8::try_from(rng.next() & 0xFF).expect("masked to u8");
                    bytes.push(if b == b'\n' { b' ' } else { b });
                }
                bytes.push(b'\n');
                // 長さにより、framerのoverflow（line_too_long）か、UTF-8不正（invalid_envelope）になる。
                let code = if len + 1 > limits::MAX_LINE_BODY_BYTES {
                    "line_too_long"
                } else {
                    "invalid_envelope"
                };
                expected.push(Expect::Rejected(code));
            }
            // 改行の無い長い入力の後に改行（上限を大きく超える行）
            _ => {
                let len = limits::MAX_LINE_BODY_BYTES + 1 + rng.below(20_000);
                bytes.extend(std::iter::repeat_n(b'x', len));
                bytes.push(b'\n');
                expected.push(Expect::Rejected("line_too_long"));
            }
        }
    }
    (bytes, expected)
}

fn classify(outcome: &Outcome) -> Expect {
    match outcome {
        Outcome::Frame(_) => Expect::Frame,
        Outcome::Rejected(rejection) => Expect::Rejected(rejection.code().as_str()),
    }
}

/// 混ぜた入力を、いろいろな大きさに刻んで流しても、結果の並びが期待と一致し、bufferが伸びない。
#[test]
fn mixed_stream_is_classified_in_order_and_the_buffer_stays_bounded() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (bytes, expected) = build_stream(&mut rng, 3_000);

    let mut receiver = LineReceiver::with_protocol_limit();
    let capacity = receiver.capacity();
    let mut observed = Vec::new();
    let mut high_water = 0;

    let mut rest = bytes.as_slice();
    while !rest.is_empty() {
        // 1〜200 byteずつ。UARTの読み取りが細切れに届く状況を模す。
        let take = (1 + rng.below(200)).min(rest.len());
        receiver.drain(&rest[..take], |outcome| observed.push(classify(&outcome)));
        rest = &rest[take..];
        high_water = high_water.max(receiver.pending());
        assert!(
            receiver.pending() <= capacity,
            "組み立て中のbyte数が上限を超えた: {} > {capacity}",
            receiver.pending()
        );
    }

    assert_eq!(
        observed.len(),
        expected.len(),
        "流した行の数と得た結果の数が違う"
    );
    for (i, (got, want)) in observed.iter().zip(&expected).enumerate() {
        assert_eq!(got, want, "{i}番目の行の結果が違う");
    }
    assert_eq!(receiver.pending(), 0, "最後の行の後にbyteが残っている");
    assert!(!receiver.is_discarding(), "読み捨て中のまま終わった");
    assert!(
        high_water <= capacity,
        "組み立て中の最大byte数が上限を超えた"
    );
}

/// 拒否は、`counter_field()`が返すfieldへ集計でき、全件がどれかのfieldへ入る。
#[test]
fn every_rejection_maps_to_a_counter_field() {
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    let (bytes, expected) = build_stream(&mut rng, 2_000);

    let mut receiver = LineReceiver::with_protocol_limit();
    let mut by_field: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut rejected = 0u32;
    let mut frames = 0u32;
    receiver.drain(&bytes, |outcome| match outcome {
        Outcome::Frame(_) => frames += 1,
        Outcome::Rejected(rejection) => {
            rejected += 1;
            let field = rejection
                .counter_field()
                .unwrap_or_else(|| panic!("counter fieldが無い拒否: {:?}", rejection.code()));
            *by_field.entry(field).or_default() += 1;
        }
    });

    let want_rejected = expected
        .iter()
        .filter(|e| matches!(e, Expect::Rejected(_)))
        .count();
    assert_eq!(rejected as usize, want_rejected);
    assert_eq!(frames as usize, expected.len() - want_rejected);
    assert_eq!(
        by_field.values().sum::<u32>(),
        rejected,
        "集計から漏れた拒否がある"
    );
}

/// 有効な行を高いrateで大量に流しても、1行も落とさず、bufferが残らない。
#[test]
fn a_flood_of_valid_lines_is_all_decoded() {
    const LINES: usize = 100_000;
    let get_status = valid_cases()
        .into_iter()
        .find(|c| c.name == "get_status")
        .expect("get_status fixture exists");

    let mut bytes = Vec::with_capacity(get_status.line.len() * LINES);
    for _ in 0..LINES {
        bytes.extend_from_slice(get_status.line.as_bytes());
    }

    let mut receiver = LineReceiver::with_protocol_limit();
    let mut frames = 0usize;
    // 4096 byteずつ。main loopが1回に読む量の大きめの想定である。
    for chunk in bytes.chunks(4096) {
        receiver.drain(chunk, |outcome| match outcome {
            Outcome::Frame(_) => frames += 1,
            other @ Outcome::Rejected(_) => panic!("有効な行が拒否された: {other:?}"),
        });
    }
    assert_eq!(frames, LINES);
    assert_eq!(receiver.pending(), 0);
}

/// 改行の無い入力を延々と流しても、bufferは伸びず、最初に1度だけ`line_too_long`を返し、
/// 改行の後の有効な行から復帰する（再接続や通信ノイズで、改行が来ない状態になる場合）。
#[test]
fn a_newline_free_flood_is_reported_once_and_recovers_at_the_next_line() {
    let ping = valid_cases()
        .into_iter()
        .find(|c| c.name == "ping")
        .expect("ping fixture exists");

    let mut receiver = LineReceiver::with_protocol_limit();
    let capacity = receiver.capacity();
    let mut oversize = 0;
    let mut frames = 0;
    let mut on = |outcome: Outcome| match outcome {
        Outcome::Rejected(r) => {
            assert!(matches!(r.cause(), Cause::Oversize), "{:?}", r.cause());
            oversize += 1;
        }
        Outcome::Frame(_) => frames += 1,
    };

    // 8 MiBを改行なしで流す。
    let chunk = [b'z'; 1024];
    for _ in 0..8 * 1024 {
        receiver.drain(&chunk, &mut on);
        assert!(receiver.pending() <= capacity);
    }
    receiver.drain(b"\n", &mut on);
    receiver.drain(ping.line.as_bytes(), &mut on);

    assert_eq!(oversize, 1, "line_too_longは1度だけ返す");
    assert_eq!(frames, 1, "改行の後の有効な行をdecodeできない");
    assert_eq!(receiver.pending(), 0);
}
