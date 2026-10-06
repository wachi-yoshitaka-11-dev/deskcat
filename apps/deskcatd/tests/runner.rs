//! `run_loop`と`StartupWatchdog`のtest。実portは開かず、台本どおりに振る舞うfake transportを注入する。
//!
//! 時計も注入する。閉じるのは、**`Step::Exit`を捨てないこと**と、**起動の期限が終了75になること**である。
//! 実portと`main`の引数・終了コードの写像は、ここでは確かめない。

use std::collections::VecDeque;
use std::io;

use deskcat_domain::{Millis, Mind, Timing};
use deskcat_protocol::{
    Ack, AckStatus, Boot, Envelope, Frame, Message, decode_line, encode_line, limits,
};
use deskcat_serial::{DuplicatePolicy, PeerSession, SerialConfig, Session, Transport};
use deskcatd::daemon::{Daemon, EXIT_SESSION_REDO, ExitReason, Identity, Step};
use deskcatd::runner::{RunEnd, run_loop};
use deskcatd::watchdog::StartupWatchdog;

const PI_SID: u32 = 1000;
const ESP_SID: u32 = 2000;

/// 台本が尽きたら、`eof`ならEOF（切断）、でなければ`WouldBlock`を返す。
#[derive(Default)]
struct Fake {
    reads: VecDeque<Vec<u8>>,
    written: Vec<u8>,
    eof: bool,
}

impl Transport for Fake {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.reads.pop_front() {
            Some(bytes) => {
                let n = bytes.len().min(buf.len());
                buf[..n].copy_from_slice(&bytes[..n]);
                if n < bytes.len() {
                    self.reads.push_front(bytes[n..].to_vec());
                }
                Ok(n)
            }
            None if self.eof => Ok(0),
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
        self.reads
            .push_back(encode_line(&frame).expect("encodeできる").into_bytes());
    }

    fn hello_id(&mut self) -> u32 {
        let text = String::from_utf8(core::mem::take(&mut self.written)).expect("UTF-8である");
        text.lines()
            .map(|line| decode_line(line).expect("復元できる"))
            .find(|f| matches!(f.message, Message::Hello(_)))
            .map(|f| f.envelope.id)
            .expect("helloを送る")
    }
}

fn daemon() -> Daemon {
    let config = SerialConfig::new("/dev/simulated", 115_200).expect("設定は妥当である");
    let policy = DuplicatePolicy::new(8, core::time::Duration::from_secs(1)).expect("0ではない");
    let mut d = Daemon::new(
        Session::new(config, PI_SID),
        PeerSession::new(policy),
        Mind::new(Timing::MVP, Millis(0)),
        Identity {
            host: "deskcatd".to_owned(),
            version: "0.1.0".to_owned(),
        },
    );
    d.on_connected();
    d
}

/// `step`ごとに`start`から`by`ずつ進む時計。
fn clock(start: u64, by: u64) -> impl FnMut() -> u64 {
    let mut now = start;
    move || {
        let v = now;
        now += by;
        v
    }
}

#[test]
fn startup_is_incomplete_until_established_and_hello_resolved() {
    let mut d = daemon();
    let mut t = Fake::default();
    assert!(!d.startup_complete(), "helloを送る前");
    assert_eq!(d.step(&mut t, 0), Step::Continue);
    assert!(!d.startup_complete(), "helloの結果を待っている");

    // `boot`だけが届いた（確立済み）。helloは未解決のまま。
    t.feed(
        Message::Boot(Boot {
            firmware: "0.1.0".to_owned(),
            board: "esp32".to_owned(),
            reset_reason: "power_on".to_owned(),
        }),
        ESP_SID,
        1,
    );
    assert_eq!(d.step(&mut t, 10), Step::Continue);
    assert!(!d.startup_complete(), "確立済みでも、helloが未解決");

    // helloのACKが届く（確立済みなので相関する）。
    let hello_id = t.hello_id();
    t.feed(
        Message::Ack(Ack {
            reply_sid: PI_SID,
            reply_to: hello_id,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        }),
        ESP_SID,
        2,
    );
    assert_eq!(d.step(&mut t, 20), Step::Continue);
    assert!(d.startup_complete(), "確立済みで、helloの結果が出た");
}

#[test]
fn the_watchdog_ends_a_silent_start_with_75_before_the_hello_budget_does() {
    // ACK timeoutは500 ms。helloの再送の予算を使い切るより前に、期限（300 ms）が来る。
    let mut d = daemon();
    let mut t = Fake::default();
    let end = run_loop(&mut d, &mut t, clock(0, 100), StartupWatchdog::new(300));
    assert_eq!(end, RunEnd::StartupDeadline);
    assert_eq!(end.exit_code(), EXIT_SESSION_REDO);
}

#[test]
fn a_daemon_exit_is_returned_not_dropped() {
    // 期限が遠ければ、helloの予算が尽きて`Daemon`が終了を求める。`Step::Exit`を捨てない。
    let mut d = daemon();
    let mut t = Fake::default();
    let end = run_loop(
        &mut d,
        &mut t,
        clock(0, 100),
        StartupWatchdog::new(u64::MAX),
    );
    assert_eq!(end, RunEnd::Exited(ExitReason::HelloRetryExhausted));
    assert_eq!(end.exit_code(), EXIT_SESSION_REDO);
}

#[test]
fn a_link_loss_is_returned_as_the_daemons_own_exit() {
    let mut d = daemon();
    let mut t = Fake {
        eof: true,
        ..Fake::default()
    };
    let end = run_loop(
        &mut d,
        &mut t,
        clock(0, 100),
        StartupWatchdog::new(u64::MAX),
    );
    assert_eq!(end, RunEnd::Exited(ExitReason::LinkLost));
}

#[test]
fn a_start_that_completes_in_the_step_past_the_deadline_is_not_expired() {
    let mut d = daemon();
    let mut t = Fake::default();
    assert_eq!(d.step(&mut t, 0), Step::Continue);
    let hello_id = t.hello_id();
    t.feed(
        Message::Boot(Boot {
            firmware: "0.1.0".to_owned(),
            board: "esp32".to_owned(),
            reset_reason: "power_on".to_owned(),
        }),
        ESP_SID,
        1,
    );
    t.feed(
        Message::Ack(Ack {
            reply_sid: PI_SID,
            reply_to: hello_id,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        }),
        ESP_SID,
        2,
    );
    // 2つのframeを1回のreadで返す。別々のreadだと、1つ目の周で未完了のまま期限を過ぎて終わる（正しい動き）。
    let both: Vec<u8> = t.reads.drain(..).flatten().collect();
    t.reads.push_back(both);
    // 台本が尽きたら切断する。**完了が立つのは、期限（300 ms）を過ぎた周である**（最初のnowは5000）。
    // 判定は`step`の後に完了を先に見るので、期限切れにならない。
    t.eof = true;
    let end = run_loop(&mut d, &mut t, clock(5000, 1000), StartupWatchdog::new(300));
    assert_eq!(end, RunEnd::Exited(ExitReason::LinkLost));
}

#[test]
fn a_daemon_exit_wins_over_the_deadline_in_the_same_step() {
    // 期限は0（最初の周から期限切れ）。同じ周で`Daemon`が切断を観測して終了を求める。終了が先に返る。
    let mut d = daemon();
    let mut t = Fake {
        eof: true,
        ..Fake::default()
    };
    let end = run_loop(&mut d, &mut t, clock(0, 100), StartupWatchdog::new(0));
    assert_eq!(end, RunEnd::Exited(ExitReason::LinkLost));
}

#[test]
fn the_deadline_ends_a_start_whose_hello_was_answered_but_boot_never_came() {
    // 通常の起動順の欠けた形: helloのACKが`boot`より先に届く。ESP32の`sid`が未承認なので、相関できない
    // ACKとして扱われ（`UnapprovedHelloAck`）、helloは済んだものになる。`boot`は来ない。
    // 期限（10 s）は、helloの再送の予算を使い切る時間（約2秒）より長い。helloが未解決のままなら、
    // `HelloRetryExhausted`で先に終わる。`StartupDeadline`で終わることが、helloが済んだ印である。
    let mut d = daemon();
    let mut t = Fake::default();
    assert_eq!(d.step(&mut t, 0), Step::Continue);
    let hello_id = t.hello_id();
    t.feed(
        Message::Ack(Ack {
            reply_sid: PI_SID,
            reply_to: hello_id,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        }),
        ESP_SID,
        1,
    );
    let end = run_loop(
        &mut d,
        &mut t,
        clock(100, 100),
        StartupWatchdog::new(10_000),
    );
    assert_eq!(end, RunEnd::StartupDeadline);
    assert!(!d.startup_complete(), "確立していない");
}
