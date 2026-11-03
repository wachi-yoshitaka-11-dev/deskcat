//! 環境・揺れ・休憩・寝起きの決定的なtest。時刻はすべてtestが渡す。

use deskcat_domain::{
    Ambient, Bubble, Companion, Config, Face, LocalMinutes, Millis, NeckRequest, Output,
};

// `Config::MVP`は実機で観察して変える値なので、testは自分の値を持つ。
const C: Config = Config {
    hot_on_c: 30.0,
    hot_off_c: 28.0,
    cold_on_c: 15.0,
    cold_off_c: 17.0,
    dry_on_pct: 30.0,
    dry_off_pct: 35.0,
    pressure_drop_on_hpa: 4.0,
    pressure_drop_off_hpa: 2.0,
    pressure_sample_ms: 1_000,
    bubble_gap_ms: 100,
    ambient_hold_ms: 5_000,
    dizzy_hold_ms: 3_000,
    motion_settle_ms: 2_000,
    shake_cooldown_ms: 500,
    rest_interval_ms: 60_000,
    sleep_at: match LocalMinutes::new(22, 0) {
        Some(m) => m,
        None => panic!(),
    },
    wake_at: match LocalMinutes::new(7, 0) {
        Some(m) => m,
        None => panic!(),
    },
};

fn day() -> LocalMinutes {
    LocalMinutes::new(12, 0).unwrap()
}

fn night() -> LocalMinutes {
    LocalMinutes::new(23, 30).unwrap()
}

fn amb(t: f32, h: f32, p: f32) -> Ambient {
    Ambient {
        temperature_c: t,
        humidity_pct: h,
        pressure_hpa: p,
    }
}

fn mild(t: f32) -> Ambient {
    amb(t, 50.0, 1013.0)
}

fn bubble_of(o: Output) -> Option<Bubble> {
    o.bubble
}

#[test]
fn hot_is_announced_once_and_not_repeated_near_threshold() {
    let mut c = Companion::new(C, Millis(0));
    let o = c.on_ambient(mild(30.0), Millis(1_000));
    assert_eq!(o.bubble, Some(Bubble::Hot));
    assert_eq!(o.face, Some(Face::Hot));
    // しきい値の上下で揺れても、戻る側（28.0）を下回らない限り出し直さない。
    for (i, t) in [29.9, 30.1, 29.0, 30.5, 28.0, 31.0].into_iter().enumerate() {
        let now = Millis(2_000 + i as u64 * 1_000);
        assert_eq!(bubble_of(c.on_ambient(mild(t), now)), None, "t={t}");
    }
    // 戻る側を下回ってから再び越えれば、もう一度出す。
    assert_eq!(c.on_ambient(mild(27.9), Millis(20_000)), Output::NONE);
    assert_eq!(
        bubble_of(c.on_ambient(mild(30.0), Millis(21_000))),
        Some(Bubble::Hot)
    );
}

#[test]
fn cold_dry_and_low_pressure_are_announced_once_each() {
    let mut c = Companion::new(C, Millis(0));
    assert_eq!(
        bubble_of(c.on_ambient(mild(15.0), Millis(0))),
        Some(Bubble::Cold)
    );
    assert_eq!(c.on_ambient(mild(16.0), Millis(1_000)), Output::NONE);

    let mut c = Companion::new(C, Millis(0));
    let o = c.on_ambient(amb(22.0, 30.0, 1013.0), Millis(0));
    assert_eq!((o.bubble, o.face), (Some(Bubble::Dry), Some(Face::Worried)));
    assert_eq!(
        c.on_ambient(amb(22.0, 33.0, 1013.0), Millis(1_000)),
        Output::NONE
    );

    let mut c = Companion::new(C, Millis(0));
    assert_eq!(
        c.on_ambient(amb(22.0, 50.0, 1013.0), Millis(0)),
        Output::NONE
    );
    assert_eq!(
        c.on_ambient(amb(22.0, 50.0, 1010.0), Millis(1_000)),
        Output::NONE
    );
    let o = c.on_ambient(amb(22.0, 50.0, 1009.0), Millis(2_000));
    assert_eq!(o.bubble, Some(Bubble::LowPressure));
    // 下がり幅が戻る側（2 hPa）まで縮むまでは出し直さない。
    assert_eq!(
        c.on_ambient(amb(22.0, 50.0, 1010.5), Millis(3_000)),
        Output::NONE
    );
    assert_eq!(
        c.on_ambient(amb(22.0, 50.0, 1008.0), Millis(4_000)),
        Output::NONE
    );
}

#[test]
fn simultaneous_conditions_are_announced_in_turn_with_a_gap() {
    let mut c = Companion::new(C, Millis(0));
    let o = c.on_ambient(amb(31.0, 20.0, 1013.0), Millis(0));
    assert_eq!(o.bubble, Some(Bubble::Hot));
    assert_eq!(c.tick(Millis(50), day()), Output::NONE);
    let o = c.tick(Millis(100), day());
    assert_eq!(o.bubble, Some(Bubble::Dry));
}

#[test]
fn invalid_samples_are_counted_and_ignored() {
    let mut c = Companion::new(C, Millis(0));
    assert_eq!(c.on_ambient(mild(f32::NAN), Millis(0)), Output::NONE);
    assert_eq!(c.on_ambient(mild(200.0), Millis(1)), Output::NONE);
    assert_eq!(c.counters().invalid_samples, 2);
}

#[test]
fn ambient_face_returns_to_neutral() {
    let mut c = Companion::new(C, Millis(0));
    c.on_ambient(mild(31.0), Millis(0));
    assert_eq!(c.tick(Millis(4_999), day()).face, None);
    assert_eq!(c.tick(Millis(5_000), day()).face, Some(Face::Neutral));
}

#[test]
fn shake_makes_dizzy_and_sways_the_neck() {
    let mut c = Companion::new(C, Millis(0));
    let o = c.on_shake(Millis(10_000));
    assert_eq!(o.face, Some(Face::Dizzy));
    assert_eq!(o.neck, Some(NeckRequest::Sway));
    assert_eq!(c.tick(Millis(13_000), day()).face, Some(Face::Neutral));
}

#[test]
fn shake_right_after_own_neck_motion_is_ignored() {
    let mut c = Companion::new(C, Millis(0));
    c.on_shake(Millis(10_000));
    // 自分の首振りの振動を、揺れとして拾い直さない。
    assert_eq!(c.on_shake(Millis(11_999)), Output::NONE);
    assert_eq!(c.counters().ignored_shakes, 1);
    assert_eq!(c.on_shake(Millis(12_000)).neck, Some(NeckRequest::Sway));
}

#[test]
fn rest_reminder_comes_at_the_interval_and_repeats() {
    let mut c = Companion::new(C, Millis(0));
    assert_eq!(c.tick(Millis(59_999), day()), Output::NONE);
    let o = c.tick(Millis(60_000), day());
    assert_eq!(o.bubble, Some(Bubble::TakeBreak));
    assert_eq!(o.neck, Some(NeckRequest::Sway));
    assert_eq!(c.tick(Millis(119_999), day()).bubble, None);
    assert_eq!(
        c.tick(Millis(120_000), day()).bubble,
        Some(Bubble::TakeBreak)
    );
}

#[test]
fn night_sleeps_without_neck_motion_and_wakes_in_the_morning() {
    let mut c = Companion::new(C, Millis(0));
    let o = c.tick(Millis(1_000), night());
    assert_eq!(o.face, Some(Face::Sleeping));
    assert_eq!(o.bubble, Some(Bubble::GoodNight));
    assert!(c.is_asleep());

    // 夜は休憩・環境・揺れのどれも首を動かさず、吹き出しも出さない。
    for i in 1..=5u64 {
        let now = Millis(1_000 + i * 60_000);
        assert_eq!(c.tick(now, night()), Output::NONE);
    }
    assert_eq!(c.on_ambient(mild(35.0), Millis(400_000)), Output::NONE);
    assert_eq!(c.on_shake(Millis(401_000)), Output::NONE);
    assert_eq!(c.face(), Face::Sleeping);

    let morning = LocalMinutes::new(7, 0).unwrap();
    let o = c.tick(Millis(500_000), morning);
    assert_eq!(o.face, Some(Face::Neutral));
    assert_eq!(o.bubble, Some(Bubble::GoodMorning));
    assert_eq!(o.neck, None);
    assert!(!c.is_asleep());
}

#[test]
fn sleep_window_wraps_midnight() {
    let mut c = Companion::new(C, Millis(0));
    let after_midnight = LocalMinutes::new(3, 0).unwrap();
    assert_eq!(c.tick(Millis(0), after_midnight).face, Some(Face::Sleeping));
    let mut c = Companion::new(C, Millis(0));
    let before = LocalMinutes::new(21, 59).unwrap();
    assert_eq!(c.tick(Millis(0), before), Output::NONE);
}

#[test]
fn rest_timer_restarts_after_waking() {
    let mut c = Companion::new(C, Millis(0));
    c.tick(Millis(0), night());
    c.tick(Millis(1_000_000), LocalMinutes::new(7, 0).unwrap());
    assert_eq!(c.tick(Millis(1_059_999), day()), Output::NONE);
    assert_eq!(
        c.tick(Millis(1_060_000), day()).bubble,
        Some(Bubble::TakeBreak)
    );
}

#[test]
fn time_going_backwards_is_counted_and_ignored() {
    let mut c = Companion::new(C, Millis(0));
    c.tick(Millis(10_000), day());
    assert_eq!(c.on_shake(Millis(5_000)), Output::NONE);
    assert_eq!(c.counters().clock_regressions, 1);
}
