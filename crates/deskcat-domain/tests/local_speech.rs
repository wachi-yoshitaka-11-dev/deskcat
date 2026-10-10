//! #23 のローカル独り言。時計と間隔は test が与える。

use std::num::NonZeroU64;

use deskcat_domain::{LOCAL_SPEECH_LINES, LocalSpeech, Millis, SpeechCounters};

// 運用値ではない。境界を調べるためだけの test 入力。
fn speech() -> LocalSpeech {
    LocalSpeech::new(NonZeroU64::new(100).unwrap(), Millis(1_000))
}

#[test]
fn first_line_is_available_without_waiting_and_lines_rotate() {
    let mut speech = speech();
    for (index, &line) in LOCAL_SPEECH_LINES.iter().enumerate() {
        assert_eq!(
            speech.next_line(Millis(1_000 + index as u64 * 100)),
            Some(line)
        );
    }
    assert_eq!(
        speech.next_line(Millis(1_000 + LOCAL_SPEECH_LINES.len() as u64 * 100)),
        Some(LOCAL_SPEECH_LINES[0])
    );
}

#[test]
fn calls_before_interval_are_suppressed_without_consuming_a_line() {
    let mut speech = speech();
    assert_eq!(speech.next_line(Millis(1_000)), Some(LOCAL_SPEECH_LINES[0]));
    assert_eq!(speech.next_line(Millis(1_000)), None);
    assert_eq!(speech.next_line(Millis(1_099)), None);
    assert_eq!(speech.next_line(Millis(1_100)), Some(LOCAL_SPEECH_LINES[1]));
    assert_eq!(
        speech.counters(),
        SpeechCounters {
            emitted: 2,
            rate_limited: 2,
            clock_regressions: 0,
        }
    );
}

#[test]
fn clock_regression_does_not_consume_a_line_or_reset_the_interval() {
    let mut speech = speech();
    assert_eq!(speech.next_line(Millis(1_010)), Some(LOCAL_SPEECH_LINES[0]));
    assert_eq!(speech.next_line(Millis(1_009)), None);
    assert_eq!(speech.next_line(Millis(1_109)), None);
    assert_eq!(speech.next_line(Millis(1_110)), Some(LOCAL_SPEECH_LINES[1]));
    assert_eq!(speech.counters().clock_regressions, 1);
    assert_eq!(speech.counters().rate_limited, 1);
}

#[test]
fn clock_before_start_is_ignored() {
    let mut speech = speech();
    assert_eq!(speech.next_line(Millis(999)), None);
    assert_eq!(speech.next_line(Millis(1_000)), Some(LOCAL_SPEECH_LINES[0]));
    assert_eq!(speech.counters().clock_regressions, 1);
}

#[test]
fn candidates_are_nonempty_and_have_no_control_characters() {
    assert!(!LOCAL_SPEECH_LINES.is_empty());
    for line in LOCAL_SPEECH_LINES {
        assert!(!line.is_empty());
        assert!(!line.chars().any(char::is_control));
    }
}
