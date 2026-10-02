//! 状態遷移の決定的なtest。時刻はすべてtestが渡す。

use deskcat_domain::{
    ContactEvent, EventCounts, Expression, Millis, Mind, MotionIntent, Reaction, Timing,
};

// `Timing::MVP`は実機で観察して変える値なので、testは自分の値を持つ。
// 下の時刻はこの値から決めている。
const T: Timing = Timing {
    happy_hold_ms: 3_000,
    surprised_hold_ms: 2_000,
    same_kind_cooldown_ms: 1_000,
};

fn mind() -> Mind {
    Mind::new(T, Millis(0))
}

fn shows(expression: Expression, motion: Option<MotionIntent>) -> Reaction {
    Reaction {
        expression: Some(expression),
        motion,
    }
}

#[test]
fn starts_neutral_and_idles_without_output() {
    let mut m = mind();
    assert_eq!(m.expression(), Expression::Neutral);
    assert_eq!(m.tick(Millis(100_000)), Reaction::NONE);
    assert_eq!(m.expression(), Expression::Neutral);
}

#[test]
fn head_touch_makes_happy_and_nods() {
    let mut m = mind();
    assert_eq!(
        m.on_event(ContactEvent::HeadTouched, Millis(10)),
        shows(Expression::Happy, Some(MotionIntent::Nod))
    );
    assert_eq!(m.expression(), Expression::Happy);
}

#[test]
fn tap_and_lift_make_surprised_without_motion() {
    for event in [ContactEvent::Tapped, ContactEvent::Lifted] {
        let mut m = mind();
        assert_eq!(
            m.on_event(event, Millis(10)),
            shows(Expression::Surprised, None)
        );
    }
}

#[test]
fn happy_returns_to_neutral_exactly_at_hold() {
    let mut m = mind();
    m.on_event(ContactEvent::HeadTouched, Millis(1_000));
    let end = 1_000 + T.happy_hold_ms;
    assert_eq!(m.tick(Millis(end - 1)), Reaction::NONE);
    assert_eq!(m.tick(Millis(end)), shows(Expression::Neutral, None));
    assert_eq!(m.tick(Millis(end + 1)), Reaction::NONE);
}

#[test]
fn surprised_returns_to_neutral_exactly_at_hold() {
    let mut m = mind();
    m.on_event(ContactEvent::Tapped, Millis(1_000));
    let end = 1_000 + T.surprised_hold_ms;
    assert_eq!(m.tick(Millis(end - 1)), Reaction::NONE);
    assert_eq!(m.tick(Millis(end)), shows(Expression::Neutral, None));
}

#[test]
fn same_kind_within_cooldown_is_suppressed_and_counted() {
    let mut m = mind();
    m.on_event(ContactEvent::HeadTouched, Millis(0));
    let just_before = T.same_kind_cooldown_ms - 1;
    assert_eq!(
        m.on_event(ContactEvent::HeadTouched, Millis(just_before)),
        Reaction::NONE
    );
    assert_eq!(
        m.counters().event(ContactEvent::HeadTouched),
        EventCounts {
            accepted: 1,
            suppressed: 1
        }
    );
}

#[test]
fn same_kind_at_cooldown_is_accepted_and_restarts_hold() {
    let mut m = mind();
    m.on_event(ContactEvent::HeadTouched, Millis(0));
    let again = T.same_kind_cooldown_ms;
    // 表情は変わらないので送り直さない。首の意図は新しく受け付けたeventの分だけ出す。
    assert_eq!(
        m.on_event(ContactEvent::HeadTouched, Millis(again)),
        Reaction {
            expression: None,
            motion: Some(MotionIntent::Nod)
        }
    );
    // 保つ時間は2回目から測り直す。
    assert_eq!(m.tick(Millis(T.happy_hold_ms)), Reaction::NONE);
    assert_eq!(
        m.tick(Millis(again + T.happy_hold_ms)),
        shows(Expression::Neutral, None)
    );
    assert_eq!(m.counters().event(ContactEvent::HeadTouched).accepted, 2);
}

#[test]
fn cooldown_is_per_kind() {
    let mut m = mind();
    m.on_event(ContactEvent::HeadTouched, Millis(0));
    assert_eq!(
        m.on_event(ContactEvent::Tapped, Millis(1)),
        shows(Expression::Surprised, None)
    );
    assert_eq!(
        m.on_event(ContactEvent::Lifted, Millis(2)),
        Reaction::NONE,
        "表情はSurprisedのままで、受け付けてはいる"
    );
    assert_eq!(m.counters().event(ContactEvent::Lifted).accepted, 1);
}

#[test]
fn later_event_overrides_current_expression() {
    let mut m = mind();
    m.on_event(ContactEvent::Tapped, Millis(0));
    assert_eq!(
        m.on_event(ContactEvent::HeadTouched, Millis(500)),
        shows(Expression::Happy, Some(MotionIntent::Nod))
    );
}

#[test]
fn suppressed_event_still_reports_pending_return_to_neutral() {
    // 保つ時間がcooldownより短い設定で、tickを呼ばずに次のeventが来た場合。
    let timing = Timing {
        happy_hold_ms: 100,
        surprised_hold_ms: 100,
        same_kind_cooldown_ms: 1_000,
    };
    let mut m = Mind::new(timing, Millis(0));
    m.on_event(ContactEvent::HeadTouched, Millis(0));
    assert_eq!(
        m.on_event(ContactEvent::HeadTouched, Millis(500)),
        shows(Expression::Neutral, None)
    );
    assert_eq!(m.counters().event(ContactEvent::HeadTouched).suppressed, 1);
}

#[test]
fn expired_expression_is_replaced_by_event_in_one_reaction() {
    let mut m = mind();
    m.on_event(ContactEvent::Tapped, Millis(0));
    // Surprisedは期限切れだがtickを呼んでいない。Neutralを挟まずHappyだけを返す。
    assert_eq!(
        m.on_event(ContactEvent::HeadTouched, Millis(T.surprised_hold_ms + 5)),
        shows(Expression::Happy, Some(MotionIntent::Nod))
    );
}

#[test]
fn clock_regression_is_ignored_and_counted() {
    let mut m = Mind::new(T, Millis(1_000));
    m.on_event(ContactEvent::HeadTouched, Millis(2_000));
    assert_eq!(
        m.on_event(ContactEvent::Tapped, Millis(1_999)),
        Reaction::NONE
    );
    assert_eq!(m.tick(Millis(1_500)), Reaction::NONE);
    assert_eq!(m.expression(), Expression::Happy);
    assert_eq!(m.counters().clock_regressions, 2);
    assert_eq!(
        m.counters().event(ContactEvent::Tapped),
        EventCounts::default()
    );
}

#[test]
fn same_input_sequence_gives_same_output_sequence() {
    let inputs = [
        (Some(ContactEvent::HeadTouched), 0),
        (Some(ContactEvent::HeadTouched), 300),
        (None, 2_000),
        (Some(ContactEvent::Tapped), 2_500),
        (None, 4_499),
        (None, 4_500),
        (Some(ContactEvent::Lifted), 9_000),
        (None, 20_000),
    ];
    let run = || {
        let mut m = mind();
        inputs
            .iter()
            .map(|&(event, t)| match event {
                Some(e) => m.on_event(e, Millis(t)),
                None => m.tick(Millis(t)),
            })
            .collect::<Vec<_>>()
    };
    let first = run();
    assert_eq!(first, run());
    assert_eq!(
        first,
        vec![
            shows(Expression::Happy, Some(MotionIntent::Nod)),
            Reaction::NONE,
            Reaction::NONE,
            shows(Expression::Surprised, None),
            Reaction::NONE,
            shows(Expression::Neutral, None),
            shows(Expression::Surprised, None),
            shows(Expression::Neutral, None),
        ]
    );
}
