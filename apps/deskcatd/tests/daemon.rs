//! `Daemon`のtest。実portは開かず、台本どおりに振る舞うfake transportを注入する。
//!
//! ここで閉じるのは、受け入れ条件1のうち**表情と、時間でneutralへ戻ること**である。
//! 首の動き（`PROTO-TBD-008`）と、Pi実機での確認は閉じない。

use core::time::Duration;
use std::collections::VecDeque;
use std::io;

use deskcat_domain::{Millis, Mind, Timing};
use deskcat_protocol::{
    Ack, AckStatus, Boot, DisplayStatus, Envelope, ErrorCode, ExpressionName, Frame, HeadTouched,
    HelloReason, Message, ProtocolCounters, SensorStatus, ServoStatus, Status, decode_line,
    encode_line, limits,
};
use deskcat_serial::{
    ConnectionState, DuplicatePolicy, PeerSession, SerialConfig, Session, StopReason, Transport,
};
use deskcatd::daemon::{
    Daemon, EXIT_FATAL, EXIT_SESSION_REDO, ExitReason, HELLO_RETRY_LIMIT, Identity, RESUME_LIMIT,
    STATUS_RETRY_LIMIT, Step,
};
use deskcatd::expression_sync::{SILENT_RETRY_LIMIT, TRANSIENT_RETRY_LIMIT};

const PI_SID: u32 = 1000;
const ESP_SID: u32 = 2000;

#[derive(Default)]
struct Fake {
    reads: VecDeque<io::Result<Vec<u8>>>,
    written: Vec<u8>,
}

impl Transport for Fake {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.reads.pop_front() {
            Some(Ok(bytes)) => {
                let n = bytes.len().min(buf.len());
                buf[..n].copy_from_slice(&bytes[..n]);
                if n < bytes.len() {
                    self.reads.push_front(Ok(bytes[n..].to_vec()));
                }
                Ok(n)
            }
            Some(Err(err)) => Err(err),
            None => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        }
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Fake {
    fn feed(&mut self, message: Message, sid: u32, id: u32) {
        let frame = Frame::new(
            Envelope {
                v: limits::PROTOCOL_VERSION,
                sid,
                id,
                ts_ms: 1,
            },
            message,
        );
        let line = encode_line(&frame).expect("encodeできる");
        self.reads.push_back(Ok(line.into_bytes()));
    }

    fn sent(&mut self) -> Vec<Frame> {
        let text = String::from_utf8(core::mem::take(&mut self.written)).expect("UTF-8である");
        text.lines()
            .map(|line| decode_line(line).expect("復元できる"))
            .collect()
    }
}

fn ack(reply_to: u32, status: AckStatus, code: Option<ErrorCode>) -> Message {
    Message::Ack(Ack {
        reply_sid: PI_SID,
        reply_to,
        status,
        code,
        detail: None,
    })
}

fn boot() -> Message {
    Message::Boot(Boot {
        firmware: "0.1.0".to_owned(),
        board: "esp32".to_owned(),
        reset_reason: "power_on".to_owned(),
    })
}

fn status(expression: &str) -> Message {
    Message::Status(Box::new(Status {
        firmware: "0.1.0".to_owned(),
        reset_reason: "power_on".to_owned(),
        display: DisplayStatus {
            state: "ready".to_owned(),
            expression: expression.to_owned(),
        },
        servo: ServoStatus {
            state: "disabled".to_owned(),
        },
        sensors: SensorStatus {
            touch: "unknown".to_owned(),
            acceleration: "unknown".to_owned(),
            environment: "unknown".to_owned(),
        },
        protocol: ProtocolCounters::default(),
    }))
}

fn daemon() -> Daemon {
    daemon_with(Timing::MVP)
}

fn daemon_with(timing: Timing) -> Daemon {
    let config = SerialConfig::new("/dev/simulated", 115_200).expect("設定は妥当である");
    let policy = DuplicatePolicy::new(8, Duration::from_secs(1)).expect("0ではない");
    Daemon::new(
        Session::new(config, PI_SID),
        PeerSession::new(policy),
        Mind::new(timing, Millis(0)),
        Identity {
            host: "deskcatd".to_owned(),
            version: "0.1.0".to_owned(),
        },
    )
}

/// 読む台本が尽きるまで回し、送信queueが空くまで数周足す。`Continue`以外が出たらそこで返す。
///
/// `Session::pump_write`は1周に1件のmessageしか書かない。
fn run(d: &mut Daemon, t: &mut Fake, now: u64) -> Step {
    let mut idle = 0;
    while idle < 4 {
        if t.reads.is_empty() {
            idle += 1;
        }
        let step = d.step(t, now);
        if step != Step::Continue {
            return step;
        }
    }
    Step::Continue
}

fn step_code(step: Step) -> u8 {
    match step {
        Step::Exit(reason) => reason.exit_code(),
        other => panic!("終了ではない: {other:?}"),
    }
}

fn find_id(frames: &[Frame], pick: impl Fn(&Message) -> bool) -> Option<u32> {
    frames
        .iter()
        .find(|f| pick(&f.message))
        .map(|f| f.envelope.id)
}

fn expressions(frames: &[Frame]) -> Vec<ExpressionName> {
    frames
        .iter()
        .filter_map(|f| match &f.message {
            Message::SetExpression(e) => Some(e.name),
            _ => None,
        })
        .collect()
}

/// `boot`→`get_status`→`status`まで進める。`status`の表情を`shown`にする。
fn established(shown: &str) -> (Daemon, Fake) {
    established_with(daemon(), shown)
}

fn established_with(mut d: Daemon, shown: &str) -> (Daemon, Fake) {
    let mut t = Fake::default();
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let sent = t.sent();
    let hello = sent
        .iter()
        .find_map(|f| match &f.message {
            Message::Hello(h) => Some(h.clone()),
            _ => None,
        })
        .expect("helloを送る");
    assert_eq!(hello.reason, HelloReason::Startup);

    // 通常の起動順では、helloのACKはbootより先に届く。ESP32のsidが未承認なので受理されない（§6）。
    let hello_id = find_id(&sent, |m| matches!(m, Message::Hello(_))).expect("hello");
    t.feed(ack(hello_id, AckStatus::Ok, None), ESP_SID, 90);
    t.feed(boot(), ESP_SID, 1);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);
    let sent = t.sent();
    let get_status =
        find_id(&sent, |m| matches!(m, Message::GetStatus)).expect("確立の後にget_statusを送る");
    t.feed(ack(get_status, AckStatus::Ok, None), ESP_SID, 2);
    t.feed(status(shown), ESP_SID, 3);
    assert_eq!(run(&mut d, &mut t, 20), Step::Continue);
    (d, t)
}

#[test]
fn a_touch_sends_happy_and_time_returns_it_to_neutral() {
    let (mut d, mut t) = established("neutral");
    let _ = t.sent();

    t.feed(
        Message::HeadTouched(HeadTouched { duration_ms: 700 }),
        ESP_SID,
        4,
    );
    assert_eq!(run(&mut d, &mut t, 100), Step::Continue);
    let sent = t.sent();
    assert_eq!(expressions(&sent), [ExpressionName::Happy]);
    let id = find_id(&sent, |m| matches!(m, Message::SetExpression(_))).expect("送った");
    assert!(
        sent.iter()
            .all(|f| !matches!(f.message, Message::SetExpression(ref e) if e.transition_ms != 0)),
        "transition_msは0"
    );

    t.feed(ack(id, AckStatus::Ok, None), ESP_SID, 5);
    assert_eq!(run(&mut d, &mut t, 150), Step::Continue);
    assert!(expressions(&t.sent()).is_empty(), "ACK済みなので送らない");

    // 保つ時間（3000 ms）が過ぎる。
    assert_eq!(run(&mut d, &mut t, 3_200), Step::Continue);
    assert_eq!(expressions(&t.sent()), [ExpressionName::Neutral]);
    assert_eq!(d.counters().motions_not_sent, 1, "首の動きは送らない");
}

#[test]
fn only_one_set_expression_is_in_flight_and_the_latest_is_sent_after_the_ack() {
    let (mut d, mut t) = established("neutral");
    let _ = t.sent();

    t.feed(
        Message::HeadTouched(HeadTouched { duration_ms: 700 }),
        ESP_SID,
        4,
    );
    assert_eq!(run(&mut d, &mut t, 100), Step::Continue);
    let first = t.sent();
    assert_eq!(expressions(&first), [ExpressionName::Happy]);
    let id = find_id(&first, |m| matches!(m, Message::SetExpression(_))).expect("送った");

    t.feed(Message::Tapped, ESP_SID, 5);
    assert_eq!(run(&mut d, &mut t, 200), Step::Continue);
    assert!(expressions(&t.sent()).is_empty(), "応答待ちの間は送らない");

    t.feed(ack(id, AckStatus::Ok, None), ESP_SID, 6);
    assert_eq!(run(&mut d, &mut t, 250), Step::Continue);
    assert_eq!(expressions(&t.sent()), [ExpressionName::Surprised]);
    assert_eq!(
        d.peer_counters().unmatched_acks,
        0,
        "古い追跡を外さないので、ACKは不一致にならない"
    );
}

#[test]
fn nothing_is_sent_before_the_status_arrives() {
    let mut d = daemon();
    let mut t = Fake::default();
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    t.feed(boot(), ESP_SID, 1);
    t.feed(
        Message::HeadTouched(HeadTouched { duration_ms: 700 }),
        ESP_SID,
        2,
    );
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);
    assert!(
        expressions(&t.sent()).is_empty(),
        "表示が分かるまで送らない"
    );
}

#[test]
fn a_status_that_differs_from_the_desired_expression_is_corrected() {
    // process起動直後の欲しい表情はNeutralである。ESP32がHappyを出していれば戻す（§10.1手順6〜7）。
    let (d, mut t) = established("happy");
    assert_eq!(expressions(&t.sent()), [ExpressionName::Neutral]);
    assert_eq!(d.connection_state(), ConnectionState::Connected);
}

#[test]
fn a_second_boot_with_another_sid_asks_for_a_restart() {
    let (mut d, mut t) = established("neutral");
    t.feed(boot(), ESP_SID + 1, 1);
    let step = run(&mut d, &mut t, 500);
    assert_eq!(step, Step::Exit(ExitReason::EspRebooted));
    assert_eq!(ExitReason::EspRebooted.exit_code(), EXIT_SESSION_REDO);
}

#[test]
fn a_replayed_boot_does_not_restart() {
    let (mut d, mut t) = established("neutral");
    t.feed(boot(), ESP_SID, 1);
    assert_eq!(run(&mut d, &mut t, 500), Step::Continue);
}

#[test]
fn stale_session_on_a_command_resumes_with_a_startup_hello_and_a_new_id() {
    // ESP32がPiのhelloを受けていない（または再起動してPiのsidを失った）。
    let mut d = daemon();
    let mut t = Fake::default();
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let first_hello = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("hello");

    t.feed(ack(first_hello, AckStatus::Ok, None), ESP_SID, 90);
    t.feed(boot(), ESP_SID, 1);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);
    let get_status = find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("get_status");

    t.feed(
        ack(
            get_status,
            AckStatus::Rejected,
            Some(ErrorCode::StaleSession),
        ),
        ESP_SID,
        2,
    );
    assert_eq!(run(&mut d, &mut t, 20), Step::Continue, "終了しない");
    let sent = t.sent();
    let resumed = sent
        .iter()
        .find_map(|f| match &f.message {
            Message::Hello(h) => Some((f.envelope.sid, f.envelope.id, h.reason)),
            _ => None,
        })
        .expect("helloを送り直す");
    assert_eq!(resumed.0, PI_SID, "sidは選び直さない");
    assert_ne!(resumed.1, first_hello, "新しいid");
    assert_eq!(resumed.2, HelloReason::Startup);
    assert_eq!(d.counters().resumes, 1);

    // helloが受理され、get_statusとstatusを経て表情を送れるようになる。
    t.feed(ack(resumed.1, AckStatus::Ok, None), ESP_SID, 3);
    assert_eq!(run(&mut d, &mut t, 30), Step::Continue);
    let get_status = find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("再び要求する");
    t.feed(ack(get_status, AckStatus::Ok, None), ESP_SID, 4);
    t.feed(status("neutral"), ESP_SID, 5);
    t.feed(
        Message::HeadTouched(HeadTouched { duration_ms: 700 }),
        ESP_SID,
        6,
    );
    assert_eq!(run(&mut d, &mut t, 40), Step::Continue);
    assert_eq!(expressions(&t.sent()), [ExpressionName::Happy]);
}

#[test]
fn resuming_more_than_the_limit_asks_for_a_restart() {
    let mut d = daemon();
    let mut t = Fake::default();
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let hello_id = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("hello");
    t.feed(ack(hello_id, AckStatus::Ok, None), ESP_SID, 90);
    t.feed(boot(), ESP_SID, 1);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);

    let mut step = Step::Continue;
    for round in 0..=RESUME_LIMIT {
        let sent = t.sent();
        let get_status = find_id(&sent, |m| matches!(m, Message::GetStatus)).expect("要求する");
        t.feed(
            ack(
                get_status,
                AckStatus::Rejected,
                Some(ErrorCode::StaleSession),
            ),
            ESP_SID,
            10 + round,
        );
        step = run(&mut d, &mut t, 20 + u64::from(round));
        if step != Step::Continue {
            break;
        }
        let hello = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("再開のhello");
        t.feed(ack(hello, AckStatus::Ok, None), ESP_SID, 100 + round);
        assert_eq!(run(&mut d, &mut t, 30 + u64::from(round)), Step::Continue);
    }
    assert_eq!(step, Step::Exit(ExitReason::ResumeExhausted));
}

#[test]
fn exit_codes_follow_the_documented_table() {
    assert_eq!(
        ExitReason::SessionStopped(StopReason::IdSpaceExhausted).exit_code(),
        EXIT_SESSION_REDO
    );
    // I/Oの致命的なerrorも、再接続せずに再起動で扱う。
    assert_eq!(
        ExitReason::SessionStopped(StopReason::Fatal).exit_code(),
        EXIT_SESSION_REDO
    );
    assert_eq!(ExitReason::LinkLost.exit_code(), EXIT_SESSION_REDO);
    assert_eq!(ExitReason::GetStatusRejected(None).exit_code(), EXIT_FATAL);
    assert_eq!(
        ExitReason::HelloRetryExhausted.exit_code(),
        EXIT_SESSION_REDO
    );
    assert_eq!(ExitReason::StatusUnanswered.exit_code(), EXIT_SESSION_REDO);
    assert_eq!(ExitReason::HelloRejected(None).exit_code(), EXIT_FATAL);
    assert_eq!(ExitReason::HelloUnsendable.exit_code(), EXIT_FATAL);
    assert_eq!(ExitReason::ResumeExhausted.exit_code(), EXIT_SESSION_REDO);
}

#[test]
fn a_fatal_io_error_stops_the_daemon() {
    let (mut d, mut t) = established("neutral");
    t.reads
        .push_back(Err(io::Error::from(io::ErrorKind::PermissionDenied)));
    assert_eq!(
        d.step(&mut t, 50),
        Step::Exit(ExitReason::SessionStopped(StopReason::Fatal))
    );
}

#[test]
fn an_unanswered_set_expression_is_resent_with_the_same_id_then_replaced_with_a_new_one() {
    // 既定のACK timeoutは500 ms、再送は1回（`RetryPolicy::provisional`）。
    let (mut d, mut t) = established("neutral");
    let _ = t.sent();
    t.feed(
        Message::HeadTouched(HeadTouched { duration_ms: 700 }),
        ESP_SID,
        4,
    );
    assert_eq!(run(&mut d, &mut t, 100), Step::Continue);
    let first = find_id(&t.sent(), |m| matches!(m, Message::SetExpression(_))).expect("送った");

    // timeout後、同じidで再送する。
    assert_eq!(run(&mut d, &mut t, 700), Step::Continue);
    let resent = find_id(&t.sent(), |m| matches!(m, Message::SetExpression(_))).expect("再送した");
    assert_eq!(resent, first);

    // 再送予算を使い切って取り下げる。表示は分からなくなり、新しいidで送り直す。
    assert_eq!(run(&mut d, &mut t, 1_300), Step::Continue);
    let again = find_id(&t.sent(), |m| matches!(m, Message::SetExpression(_))).expect("送り直した");
    assert_ne!(again, first);
    assert_eq!(d.counters().set_expression_gave_up, 1);
}

/// 書き出されたframeのうち、ACK以外（command）の種類の並び。
fn command_order(frames: &[Frame]) -> Vec<&'static str> {
    frames
        .iter()
        .filter(|f| !matches!(f.message, Message::Ack(_)))
        .map(|f| f.message.type_str())
        .collect()
}

#[test]
fn the_hello_is_the_first_command_on_the_wire_even_when_the_boot_arrives_first() {
    // §5.1: helloは、Piがprocessを起動した直後に、他のcommandより先に一度送る。
    // Piが起動する前にESP32が出した`boot`が、portに残っている。
    let mut d = daemon();
    let mut t = Fake::default();
    t.feed(boot(), ESP_SID, 1);
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let sent = t.sent();
    // `boot`へのACKはcommandではないので、helloの前後どちらでもよい。commandはhelloが最初である。
    assert_eq!(command_order(&sent), ["hello", "get_status"]);
}

#[test]
fn a_boot_that_arrives_before_the_hello_is_sent_does_not_double_the_hello() {
    let mut d = daemon();
    let mut t = Fake::default();
    t.feed(boot(), ESP_SID, 1);
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let sent = t.sent();
    let get_status = find_id(&sent, |m| matches!(m, Message::GetStatus)).expect("get_status");
    let hello = find_id(&sent, |m| matches!(m, Message::Hello(_))).expect("hello");

    // ESP32は、wireの順に応答する。helloを受理し、get_statusへ`ok`とstatusを返す。
    // helloのACKは`boot`の後に届くが、最初の確立では`PeerSession`がhelloの待ちを残している。
    t.feed(ack(hello, AckStatus::Ok, None), ESP_SID, 2);
    t.feed(ack(get_status, AckStatus::Ok, None), ESP_SID, 3);
    t.feed(status("neutral"), ESP_SID, 4);
    t.feed(
        Message::HeadTouched(HeadTouched { duration_ms: 700 }),
        ESP_SID,
        5,
    );
    assert_eq!(run(&mut d, &mut t, 20), Step::Continue);
    let sent = t.sent();
    assert!(
        hello_ids(&sent).is_empty(),
        "helloは1つだけ。ACKは残した待ちと相関する"
    );
    assert_eq!(d.peer_counters().unmatched_acks, 0);
    assert_eq!(d.counters().resumes, 0);
    assert_eq!(expressions(&sent), [ExpressionName::Happy]);
}

#[test]
fn a_stale_set_expression_is_sent_again_after_the_session_is_resumed() {
    let (mut d, mut t) = established("neutral");
    let _ = t.sent();
    t.feed(
        Message::HeadTouched(HeadTouched { duration_ms: 700 }),
        ESP_SID,
        4,
    );
    assert_eq!(run(&mut d, &mut t, 100), Step::Continue);
    let id = find_id(&t.sent(), |m| matches!(m, Message::SetExpression(_))).expect("送った");

    t.feed(
        ack(id, AckStatus::Rejected, Some(ErrorCode::StaleSession)),
        ESP_SID,
        5,
    );
    assert_eq!(run(&mut d, &mut t, 110), Step::Continue);
    let hello = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("再開のhello");
    assert_eq!(
        d.counters().set_expression_rejected,
        0,
        "内容の拒否ではない"
    );

    t.feed(ack(hello, AckStatus::Ok, None), ESP_SID, 6);
    assert_eq!(run(&mut d, &mut t, 120), Step::Continue);
    let get_status = find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("要求");
    t.feed(ack(get_status, AckStatus::Ok, None), ESP_SID, 7);
    t.feed(status("neutral"), ESP_SID, 8);
    assert_eq!(run(&mut d, &mut t, 130), Step::Continue);
    assert_eq!(expressions(&t.sent()), [ExpressionName::Happy]);
}

#[test]
fn a_startup_hello_rejected_as_invalid_payload_is_terminal() {
    // §5.1の拒否code表: `invalid_payload`は終端。同じsidのままcommandを送らず、processを再起動する。
    let mut d = daemon();
    let mut t = Fake::default();
    t.feed(boot(), ESP_SID, 1);
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let hello = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("hello");
    let _ = t.sent();

    t.feed(
        ack(hello, AckStatus::Rejected, Some(ErrorCode::InvalidPayload)),
        ESP_SID,
        2,
    );
    let step = run(&mut d, &mut t, 10);
    assert_eq!(
        step,
        Step::Exit(ExitReason::HelloRejected(Some(ErrorCode::InvalidPayload)))
    );
    assert_eq!(step_code(step), EXIT_FATAL);
    assert!(
        find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).is_none(),
        "同じsidのまま、stateを取り直さない"
    );
}

/// `boot`→`get_status`まで進め、`get_status`のidを返す。
fn asked_for_status() -> (Daemon, Fake, u32) {
    let mut d = daemon();
    let mut t = Fake::default();
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let hello = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("hello");
    t.feed(ack(hello, AckStatus::Ok, None), ESP_SID, 90);
    t.feed(boot(), ESP_SID, 1);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);
    let id = find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("get_status");
    (d, t, id)
}

#[test]
fn a_get_status_rejected_for_another_reason_is_asked_again_later() {
    let (mut d, mut t, id) = asked_for_status();
    t.feed(
        ack(id, AckStatus::Rejected, Some(ErrorCode::RateLimited)),
        ESP_SID,
        2,
    );
    assert_eq!(run(&mut d, &mut t, 20), Step::Continue);
    assert!(
        find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).is_none(),
        "理由が解ける時間を待つ"
    );
    assert_eq!(run(&mut d, &mut t, 700), Step::Continue);
    let again = find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("送り直す");
    assert_ne!(again, id);
    assert_eq!(d.counters().resumes, 0, "helloは送り直さない");
}

#[test]
fn a_status_that_never_follows_the_ack_is_asked_for_again() {
    let (mut d, mut t, id) = asked_for_status();
    t.feed(ack(id, AckStatus::Ok, None), ESP_SID, 2);
    assert_eq!(run(&mut d, &mut t, 20), Step::Continue);
    assert!(find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).is_none());
    assert_eq!(run(&mut d, &mut t, 700), Step::Continue);
    let again = find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("送り直す");
    assert_ne!(again, id);

    // 届けば、以後は待たない。
    t.feed(ack(again, AckStatus::Ok, None), ESP_SID, 3);
    t.feed(status("neutral"), ESP_SID, 4);
    assert_eq!(run(&mut d, &mut t, 710), Step::Continue);
    assert_eq!(run(&mut d, &mut t, 5_000), Step::Continue);
    assert!(find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).is_none());
}

// ---- 表のセルごとのtest（`daemon.rs`のmodule docの表） ----

fn rejected(reply_to: u32, code: ErrorCode) -> Message {
    ack(reply_to, AckStatus::Rejected, Some(code))
}

/// `boot`が先に届いた状態で、`hello`を送った直後。`hello`のidを返す。
fn hello_pending_after_boot() -> (Daemon, Fake, u32) {
    let mut d = daemon();
    let mut t = Fake::default();
    t.feed(boot(), ESP_SID, 1);
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let hello = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("hello");
    (d, t, hello)
}

/// `hello`が終端として終了する拒否code（`stale_session`と`rate_limited`を除く9つ）。
const HELLO_FATAL_CODES: [ErrorCode; 9] = [
    ErrorCode::InvalidPayload,
    ErrorCode::Busy,
    ErrorCode::DuplicateExpired,
    ErrorCode::UnsupportedVersion,
    ErrorCode::UnknownType,
    ErrorCode::InvalidEnvelope,
    ErrorCode::OutOfRange,
    ErrorCode::LineTooLong,
    ErrorCode::HardwareUnavailable,
];

#[test]
fn cell_hello_rate_limited_is_resent_with_the_same_id_after_the_cooldown() {
    let (mut d, mut t, hello) = hello_pending_after_boot();
    t.feed(rejected(hello, ErrorCode::RateLimited), ESP_SID, 2);
    assert_eq!(run(&mut d, &mut t, 20), Step::Continue);
    assert!(
        t.sent()
            .iter()
            .all(|f| !matches!(f.message, Message::Hello(_))),
        "cooldownの間は再送しない"
    );
    assert_eq!(run(&mut d, &mut t, 700), Step::Continue);
    let resent: Vec<_> = t
        .sent()
        .into_iter()
        .filter(|f| matches!(f.message, Message::Hello(_)))
        .collect();
    assert_eq!(resent.len(), 1);
    assert_eq!(resent[0].envelope.id, hello, "同じ(sid, id)");
    assert_eq!(resent[0].envelope.sid, PI_SID);

    // 受理されれば、stateの取得へ進む。
    t.feed(ack(hello, AckStatus::Ok, None), ESP_SID, 3);
    assert_eq!(run(&mut d, &mut t, 710), Step::Continue);
    assert!(find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).is_some());
}

#[test]
fn cell_hello_rate_limited_budget_is_finite() {
    let (mut d, mut t, hello) = hello_pending_after_boot();
    let mut now = 0;
    for round in 0..HELLO_RETRY_LIMIT {
        t.feed(rejected(hello, ErrorCode::RateLimited), ESP_SID, 10 + round);
        now += 700;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
        now += 700;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
        assert!(
            t.sent()
                .iter()
                .any(|f| matches!(f.message, Message::Hello(_))),
            "{round}回目の再送"
        );
    }
    t.feed(rejected(hello, ErrorCode::RateLimited), ESP_SID, 99);
    let step = run(&mut d, &mut t, now + 1);
    assert_eq!(step, Step::Exit(ExitReason::HelloRetryExhausted));
    assert_eq!(step_code(step), EXIT_SESSION_REDO);
}

#[test]
fn cell_hello_with_every_other_code_is_fatal() {
    for code in HELLO_FATAL_CODES {
        let (mut d, mut t, hello) = hello_pending_after_boot();
        t.feed(rejected(hello, code), ESP_SID, 2);
        let step = run(&mut d, &mut t, 20);
        assert_eq!(
            step,
            Step::Exit(ExitReason::HelloRejected(Some(code))),
            "{code:?}"
        );
        assert_eq!(step_code(step), EXIT_FATAL);
    }
}

#[test]
fn cell_hello_rejection_before_the_boot_is_lost_and_does_not_stop() {
    // 起動直後は、ESP32のsidが未承認である。拒否のcodeは届かない（module docの「起動直後のhelloの拒否は届かない」）。
    let mut d = daemon();
    let mut t = Fake::default();
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let hello = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("hello");
    t.feed(rejected(hello, ErrorCode::StaleSession), ESP_SID, 1);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue, "届かない");
    assert_eq!(d.peer_counters().unapproved_hello_acks, 1);
    assert!(
        t.sent()
            .iter()
            .all(|f| !matches!(f.message, Message::Hello(_)))
    );
}

#[test]
fn cell_set_expression_with_a_transient_code_is_resent_after_the_cooldown() {
    for code in [
        ErrorCode::Busy,
        ErrorCode::RateLimited,
        ErrorCode::DuplicateExpired,
    ] {
        let (mut d, mut t) = established("neutral");
        let _ = t.sent();
        t.feed(
            Message::HeadTouched(HeadTouched { duration_ms: 700 }),
            ESP_SID,
            4,
        );
        assert_eq!(run(&mut d, &mut t, 100), Step::Continue);
        let id = find_id(&t.sent(), |m| matches!(m, Message::SetExpression(_))).expect("送った");

        t.feed(rejected(id, code), ESP_SID, 5);
        assert_eq!(run(&mut d, &mut t, 110), Step::Continue);
        assert!(
            expressions(&t.sent()).is_empty(),
            "{code:?}: cooldownの間は送らない"
        );
        assert_eq!(run(&mut d, &mut t, 700), Step::Continue);
        let sent = t.sent();
        assert_eq!(expressions(&sent), [ExpressionName::Happy], "{code:?}");
        let again = find_id(&sent, |m| matches!(m, Message::SetExpression(_))).expect("送り直す");
        assert_ne!(again, id, "{code:?}: 新しいid（§9）");
        assert_eq!(d.counters().set_expression_rejected, 0, "{code:?}");
        assert_eq!(d.counters().set_expression_retried_later, 1, "{code:?}");
    }
}

#[test]
fn cell_set_expression_transient_retries_are_finite() {
    let (mut d, mut t) = established("neutral");
    let _ = t.sent();
    t.feed(
        Message::HeadTouched(HeadTouched { duration_ms: 700 }),
        ESP_SID,
        4,
    );
    let mut now = 100;
    assert_eq!(run(&mut d, &mut t, now), Step::Continue);
    let mut sent_count = 0;
    for round in 0..=TRANSIENT_RETRY_LIMIT {
        let id = find_id(&t.sent(), |m| matches!(m, Message::SetExpression(_))).expect("送った");
        sent_count += 1;
        t.feed(rejected(id, ErrorCode::Busy), ESP_SID, 10 + round);
        now += 100;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
        now += 600;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
    }
    // 上限を超えた。以後は、欲しい表情が変わるまで送らない。
    assert_eq!(sent_count, TRANSIENT_RETRY_LIMIT + 1);
    assert!(expressions(&t.sent()).is_empty());
    // 保つ時間（3000 ms）の内で、時間が過ぎても送らない（過ぎるとneutralへ戻る別の送信が起こる）。
    now += 100;
    assert!(now < 100 + 3_000);
    assert_eq!(run(&mut d, &mut t, now), Step::Continue);
    assert!(expressions(&t.sent()).is_empty());
}

#[test]
fn cell_set_expression_with_a_content_code_stops_until_the_desire_changes() {
    for code in [
        ErrorCode::UnsupportedVersion,
        ErrorCode::UnknownType,
        ErrorCode::InvalidEnvelope,
        ErrorCode::InvalidPayload,
        ErrorCode::OutOfRange,
        ErrorCode::LineTooLong,
        ErrorCode::HardwareUnavailable,
    ] {
        let (mut d, mut t) = established("neutral");
        let _ = t.sent();
        t.feed(
            Message::HeadTouched(HeadTouched { duration_ms: 700 }),
            ESP_SID,
            4,
        );
        assert_eq!(run(&mut d, &mut t, 100), Step::Continue);
        let id = find_id(&t.sent(), |m| matches!(m, Message::SetExpression(_))).expect("送った");

        t.feed(rejected(id, code), ESP_SID, 5);
        assert_eq!(run(&mut d, &mut t, 110), Step::Continue);
        assert_eq!(run(&mut d, &mut t, 900), Step::Continue);
        assert!(expressions(&t.sent()).is_empty(), "{code:?}: 送り直さない");
        assert_eq!(d.counters().set_expression_rejected, 1, "{code:?}");
        assert_eq!(d.counters().set_expression_retried_later, 0, "{code:?}");
    }
}

// ---- link断と再開（再開に期限は持たない） ----

#[test]
fn a_disconnect_asks_for_a_restart_and_never_reconnects() {
    let (mut d, mut t) = established("neutral");
    t.reads.push_back(Ok(Vec::new())); // EOF
    let step = d.step(&mut t, 50);
    assert_eq!(step, Step::Exit(ExitReason::LinkLost));
    assert_eq!(step_code(step), EXIT_SESSION_REDO);
}

#[test]
fn a_fatal_io_error_asks_for_a_restart() {
    let (mut d, mut t) = established("neutral");
    t.reads
        .push_back(Err(io::Error::from(io::ErrorKind::PermissionDenied)));
    let step = d.step(&mut t, 50);
    assert_eq!(step_code(step), EXIT_SESSION_REDO);
}

#[test]
fn waiting_for_the_status_after_an_ok_ack_has_no_timeout_and_asks_again_with_new_ids() {
    // §5.6: Piは、応答のstatusを待つtimeoutを持たない。失敗として数えず、終了もしない。
    let (mut d, mut t, mut id) = asked_for_status();
    let mut ids = vec![id];
    let mut now = 10;
    for round in 0..(STATUS_RETRY_LIMIT * 3) {
        t.feed(ack(id, AckStatus::Ok, None), ESP_SID, 100 + round);
        now += 100;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
        now += 600;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue, "{round}回目");
        assert_eq!(
            d.counters().status_reasked,
            u64::from(round) + 1,
            "送り直しを数える"
        );
        id = find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("新しいidで要求する");
        assert!(!ids.contains(&id));
        ids.push(id);
    }
    assert_eq!(d.counters().resumes, 0);
}

#[test]
fn cell_hello_stale_session_is_a_sid_collision() {
    let (mut d, mut t, hello) = hello_pending_after_boot();
    t.feed(rejected(hello, ErrorCode::StaleSession), ESP_SID, 2);
    let step = run(&mut d, &mut t, 20);
    assert_eq!(step, Step::Exit(ExitReason::HelloCollided));
    assert_eq!(step_code(step), EXIT_SESSION_REDO);
}

// ---- 不変条件「`hello`は同時に1つだけ」: 再開の要求と`hello`の結果の交差 ----

/// `boot`が先に届いた状態で、`hello`と`get_status`を送った直後。両方のidを返す。
///
/// 以下の`invariant_*`の台本は、ESP32が`get_status`を`stale_session`で拒否するなど、wireの順どおりに
/// 応答すれば起こらない事象も含む。起こったときの状態機械（再開の見送りと`hello`の結果の交差）を固定する。
fn boot_first_with_ids() -> (Daemon, Fake, u32, u32) {
    let mut d = daemon();
    let mut t = Fake::default();
    t.feed(boot(), ESP_SID, 1);
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let sent = t.sent();
    let get_status = find_id(&sent, |m| matches!(m, Message::GetStatus)).expect("get_status");
    let hello = find_id(&sent, |m| matches!(m, Message::Hello(_))).expect("hello");
    (d, t, get_status, hello)
}

/// 書き出された`hello`のid（順）。
fn hello_ids(frames: &[Frame]) -> Vec<u32> {
    frames
        .iter()
        .filter(|f| matches!(f.message, Message::Hello(_)))
        .map(|f| f.envelope.id)
        .collect()
}

#[test]
fn invariant_a_resume_wanted_while_the_hello_is_pending_waits_for_its_rate_limited_resend() {
    let (mut d, mut t, get_status, hello) = boot_first_with_ids();
    // 先に送ったget_statusのstale拒否が届く。helloのACKはまだ無いので、再開は見送る。
    t.feed(rejected(get_status, ErrorCode::StaleSession), ESP_SID, 2);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);
    assert!(hello_ids(&t.sent()).is_empty());
    // 続いて、helloが`rate_limited`で拒否される。新しいidのhelloを送らない。
    t.feed(rejected(hello, ErrorCode::RateLimited), ESP_SID, 3);
    assert_eq!(run(&mut d, &mut t, 20), Step::Continue);
    assert!(hello_ids(&t.sent()).is_empty(), "cooldownの間は送らない");
    // cooldownの後は、同じ(sid, id)の再送だけである。
    assert_eq!(run(&mut d, &mut t, 700), Step::Continue);
    assert_eq!(hello_ids(&t.sent()), [hello]);
    assert_eq!(d.counters().resumes, 0, "再開の回数を消費しない");
    // 受理されれば、見送りは解ける。余分なhelloは出ず、stateを取り直す。
    t.feed(ack(hello, AckStatus::Ok, None), ESP_SID, 4);
    assert_eq!(run(&mut d, &mut t, 710), Step::Continue);
    let sent = t.sent();
    assert!(hello_ids(&sent).is_empty());
    assert!(find_id(&sent, |m| matches!(m, Message::GetStatus)).is_some());
    assert_eq!(run(&mut d, &mut t, 5_000), Step::Continue);
    assert!(hello_ids(&t.sent()).is_empty(), "見送りは解けている");
    assert_eq!(d.counters().resumes, 0);
}

#[test]
fn invariant_a_resume_wanted_while_the_hello_is_pending_ends_with_the_retry_budget() {
    let (mut d, mut t, get_status, hello) = boot_first_with_ids();
    t.feed(rejected(get_status, ErrorCode::StaleSession), ESP_SID, 2);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);
    let mut now = 10;
    let mut seen = Vec::new();
    for round in 0..HELLO_RETRY_LIMIT {
        t.feed(rejected(hello, ErrorCode::RateLimited), ESP_SID, 10 + round);
        now += 700;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
        now += 700;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
        seen.extend(hello_ids(&t.sent()));
    }
    t.feed(rejected(hello, ErrorCode::RateLimited), ESP_SID, 99);
    let step = run(&mut d, &mut t, now + 1);
    assert_eq!(step, Step::Exit(ExitReason::HelloRetryExhausted));
    seen.extend(hello_ids(&t.sent()));
    assert!(
        seen.iter().all(|id| *id == hello),
        "新しいidのhelloは出ない: {seen:?}"
    );
    assert_eq!(d.counters().resumes, 0);
}

#[test]
fn invariant_a_stale_command_while_the_rate_limited_resend_is_pending_does_not_send_a_new_hello() {
    let (mut d, mut t, get_status, hello) = boot_first_with_ids();
    t.feed(rejected(hello, ErrorCode::RateLimited), ESP_SID, 2);
    assert_eq!(run(&mut d, &mut t, 20), Step::Continue);
    let _ = t.sent();
    // 再送待ちのあいだに、通常commandのstale拒否が届く。
    t.feed(rejected(get_status, ErrorCode::StaleSession), ESP_SID, 3);
    assert_eq!(run(&mut d, &mut t, 30), Step::Continue);
    assert!(hello_ids(&t.sent()).is_empty(), "再送待ちのhelloがある");
    assert_eq!(run(&mut d, &mut t, 700), Step::Continue);
    assert_eq!(hello_ids(&t.sent()), [hello], "同じ(sid, id)の再送だけ");
    t.feed(ack(hello, AckStatus::Ok, None), ESP_SID, 4);
    assert_eq!(run(&mut d, &mut t, 710), Step::Continue);
    assert!(hello_ids(&t.sent()).is_empty());
    assert_eq!(d.counters().resumes, 0);
}

#[test]
fn invariant_a_resume_wanted_while_the_hello_is_pending_ends_with_its_stale_session() {
    let (mut d, mut t, get_status, hello) = boot_first_with_ids();
    t.feed(rejected(get_status, ErrorCode::StaleSession), ESP_SID, 2);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);
    t.feed(rejected(hello, ErrorCode::StaleSession), ESP_SID, 3);
    let step = run(&mut d, &mut t, 20);
    assert_eq!(step, Step::Exit(ExitReason::HelloCollided));
    assert!(hello_ids(&t.sent()).is_empty());
    assert_eq!(d.counters().resumes, 0);
}

#[test]
fn an_unanswered_get_status_is_asked_again_without_a_hello_and_then_asks_for_a_restart() {
    let (mut d, mut t, id) = asked_for_status();
    let mut asked = vec![id];

    // ACKが一度も来ない。取り下げるたびに新しいidで要求し直し、helloは送らない。
    let mut now = 10;
    let mut step = Step::Continue;
    for _ in 0..200 {
        now += 600;
        step = run(&mut d, &mut t, now);
        let sent = t.sent();
        assert!(hello_ids(&sent).is_empty(), "無応答でhelloを送り直さない");
        asked.extend(
            sent.iter()
                .filter(|f| matches!(f.message, Message::GetStatus))
                .map(|f| f.envelope.id),
        );
        if step != Step::Continue {
            break;
        }
    }
    assert_eq!(step, Step::Exit(ExitReason::StatusUnanswered));
    assert_eq!(d.counters().resumes, 0);
    asked.sort_unstable();
    asked.dedup();
    assert_eq!(
        asked.len(),
        usize::try_from(STATUS_RETRY_LIMIT).expect("小さい") + 1,
        "上限までは、新しいidで要求し直す"
    );
}

#[test]
fn invariant_a_silent_hello_is_resent_with_the_same_id_even_when_a_resume_is_wanted() {
    // helloのACKが失われた。ESP32は処理済みかもしれないので、新しいidのstartupは送らない
    // （「startupなのにsidが現在のPi sessionと同じ」でinvalid_payloadになる）。
    let (mut d, mut t, get_status, hello) = boot_first_with_ids();
    t.feed(rejected(get_status, ErrorCode::StaleSession), ESP_SID, 2);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);
    assert!(hello_ids(&t.sent()).is_empty());
    assert_eq!(run(&mut d, &mut t, 700), Step::Continue);
    assert_eq!(hello_ids(&t.sent()), [hello], "同じ(sid, id)の再送");
    assert_eq!(d.counters().resumes, 0);

    // 再送のACKが届けば（保持ACK）、見送りは解け、stateを取り直す。
    t.feed(ack(hello, AckStatus::Ok, None), ESP_SID, 3);
    assert_eq!(run(&mut d, &mut t, 710), Step::Continue);
    let sent = t.sent();
    assert!(hello_ids(&sent).is_empty());
    assert!(find_id(&sent, |m| matches!(m, Message::GetStatus)).is_some());
    assert_eq!(run(&mut d, &mut t, 5_000), Step::Continue);
    assert!(hello_ids(&t.sent()).is_empty());
    assert_eq!(d.counters().resumes, 0);
}

#[test]
fn invariant_a_silent_hello_ends_with_the_retry_budget_and_never_sends_a_new_id() {
    let (mut d, mut t, get_status, hello) = boot_first_with_ids();
    t.feed(rejected(get_status, ErrorCode::StaleSession), ESP_SID, 2);
    assert_eq!(run(&mut d, &mut t, 10), Step::Continue);
    let mut now = 10;
    let mut seen = Vec::new();
    let mut step = Step::Continue;
    for _ in 0..50 {
        now += 600;
        step = run(&mut d, &mut t, now);
        seen.extend(hello_ids(&t.sent()));
        if step != Step::Continue {
            break;
        }
    }
    assert_eq!(step, Step::Exit(ExitReason::HelloRetryExhausted));
    assert_eq!(step_code(step), EXIT_SESSION_REDO);
    assert_eq!(
        seen.len(),
        usize::try_from(HELLO_RETRY_LIMIT).expect("小さい"),
        "予算の分だけ再送する"
    );
    assert!(
        seen.iter().all(|id| *id == hello),
        "新しいidは出ない: {seen:?}"
    );
    assert_eq!(d.counters().resumes, 0);
}

#[test]
fn an_unanswered_startup_hello_is_resent_with_the_same_id_then_asks_for_a_restart() {
    // ESP32が何も返さない。起動直後のhelloも、同じ(sid, id)で有限回だけ再送し、使い切れば終了する。
    let mut d = daemon();
    let mut t = Fake::default();
    d.on_connected();
    assert_eq!(run(&mut d, &mut t, 0), Step::Continue);
    let hello = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("hello");
    let mut now = 0;
    let mut seen = Vec::new();
    let mut step = Step::Continue;
    for _ in 0..50 {
        now += 600;
        step = run(&mut d, &mut t, now);
        seen.extend(hello_ids(&t.sent()));
        if step != Step::Continue {
            break;
        }
    }
    assert_eq!(step, Step::Exit(ExitReason::HelloRetryExhausted));
    assert_eq!(
        seen.len(),
        usize::try_from(HELLO_RETRY_LIMIT).expect("小さい")
    );
    assert!(seen.iter().all(|id| *id == hello));
}

#[test]
fn cell_get_status_with_a_transient_code_is_asked_again_with_a_new_id() {
    for code in [
        ErrorCode::Busy,
        ErrorCode::RateLimited,
        ErrorCode::DuplicateExpired,
    ] {
        let (mut d, mut t, id) = asked_for_status();
        t.feed(rejected(id, code), ESP_SID, 2);
        assert_eq!(run(&mut d, &mut t, 20), Step::Continue, "{code:?}");
        assert!(find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).is_none());
        assert_eq!(run(&mut d, &mut t, 700), Step::Continue, "{code:?}");
        let again = find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("送り直す");
        assert_ne!(again, id, "{code:?}");
        assert_eq!(d.counters().resumes, 0, "{code:?}: helloは送り直さない");
    }
}

#[test]
fn cell_get_status_with_a_fixed_rejection_asks_for_a_restart() {
    // 決まった拒否（再送でも結果が変わらない）は、繰り返さずに終了する。
    for code in [
        ErrorCode::UnsupportedVersion,
        ErrorCode::UnknownType,
        ErrorCode::InvalidEnvelope,
        ErrorCode::InvalidPayload,
        ErrorCode::OutOfRange,
        ErrorCode::LineTooLong,
        ErrorCode::HardwareUnavailable,
    ] {
        let (mut d, mut t, id) = asked_for_status();
        t.feed(rejected(id, code), ESP_SID, 2);
        let step = run(&mut d, &mut t, 20);
        assert_eq!(
            step,
            Step::Exit(ExitReason::GetStatusRejected(Some(code))),
            "{code:?}"
        );
        assert_eq!(step_code(step), EXIT_FATAL);
    }
}

#[test]
fn a_resume_has_no_deadline_and_keeps_asking_for_a_status_that_never_arrives() {
    // 再開のhelloがokで、get_statusのACKもokだが、statusの行だけが失われ続ける。
    // §5.6: Piはstatusを待つtimeoutを持たない。終了せず、新しいidで再要求し続ける。
    let (mut d, mut t, id) = asked_for_status();
    t.feed(rejected(id, ErrorCode::StaleSession), ESP_SID, 2);
    assert_eq!(run(&mut d, &mut t, 20), Step::Continue);
    let hello = find_id(&t.sent(), |m| matches!(m, Message::Hello(_))).expect("再開のhello");
    t.feed(ack(hello, AckStatus::Ok, None), ESP_SID, 3);
    assert_eq!(run(&mut d, &mut t, 30), Step::Continue);
    let mut asked = find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("要求");
    let mut now = 30;
    for round in 0..(STATUS_RETRY_LIMIT * 4) {
        t.feed(ack(asked, AckStatus::Ok, None), ESP_SID, 100 + round);
        now += 100;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
        now += 600;
        assert_eq!(run(&mut d, &mut t, now), Step::Continue, "{round}回目");
        asked =
            find_id(&t.sent(), |m| matches!(m, Message::GetStatus)).expect("新しいidで要求する");
    }
    assert!(now > 5_000, "再開から十分に長い時間が過ぎても終了しない");
    assert_eq!(d.counters().resumes, 1);
    assert_eq!(
        d.counters().status_reasked,
        u64::from(STATUS_RETRY_LIMIT * 4)
    );
}

#[test]
fn cell_set_expression_without_any_ack_stops_after_the_limit_and_resumes_when_the_desire_changes() {
    // 現行のfirmwareはset_expressionにACKを返さない（pi_link.rsのother腕。処理を足すのは#21）。
    // 保つ時間を長くして、時間でneutralへ戻る送信と区別する。
    let timing = Timing {
        happy_hold_ms: 600_000,
        surprised_hold_ms: 600_000,
        same_kind_cooldown_ms: 1_000,
    };
    let (mut d, mut t) = established_with(daemon_with(timing), "neutral");
    let _ = t.sent();
    t.feed(
        Message::HeadTouched(HeadTouched { duration_ms: 700 }),
        ESP_SID,
        4,
    );
    let mut ids = Vec::new();
    let mut now = 100;
    for _ in 0..60 {
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
        ids.extend(
            t.sent()
                .iter()
                .filter(|f| matches!(f.message, Message::SetExpression(_)))
                .map(|f| f.envelope.id),
        );
        now += 600;
    }
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        usize::try_from(SILENT_RETRY_LIMIT).expect("小さい") + 1,
        "上限までは新しいidで送り、その後は止める: {ids:?}"
    );
    assert_eq!(d.counters().set_expression_stopped_silent, 1);

    // 欲しい表情が変わる（tapped = surprised）と、また送る（上限まで）。
    t.feed(Message::Tapped, ESP_SID, 5);
    let mut again = Vec::new();
    for _ in 0..60 {
        assert_eq!(run(&mut d, &mut t, now), Step::Continue);
        again.extend(
            t.sent()
                .iter()
                .filter(|f| matches!(f.message, Message::SetExpression(_)))
                .map(|f| f.envelope.id),
        );
        now += 600;
    }
    again.sort_unstable();
    again.dedup();
    assert_eq!(
        again.len(),
        usize::try_from(SILENT_RETRY_LIMIT).expect("小さい") + 1
    );
    assert_eq!(d.counters().set_expression_stopped_silent, 2);
}
