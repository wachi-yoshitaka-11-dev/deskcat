//! `run_loop`と`StartupWatchdog`のtest。実portは開かず、台本どおりに振る舞うfake transportを注入する。
//!
//! 時計も注入する。閉じるのは、**`Step::Exit`を捨てないこと**と、**起動の期限が終了75になること**である。
//! 実portと`main`の引数・終了コードの写像は、ここでは確かめない。

use std::collections::VecDeque;
use std::io;
use std::num::NonZeroU64;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::{self, Receiver, Sender},
};
use std::time::Duration;

use deskcat_domain::{Expression, LOCAL_SPEECH_LINES, Millis, Mind, Timing};
use deskcat_protocol::{
    Ack, AckStatus, Boot, DisplayStatus, Envelope, Frame, Message, ProtocolCounters, SensorStatus,
    ServoStatus, Status, decode_line, encode_line, limits,
};
use deskcat_serial::{DuplicatePolicy, PeerSession, SerialConfig, Session, Transport};
use deskcatd::cloud_speech::{Client, CloudSpeech, Context, Failure};
use deskcatd::daemon::{Daemon, EXIT_SESSION_REDO, ExitReason, Identity, Step};
use deskcatd::runner::{RunEnd, SpeechConfig, run_loop, run_loop_with_cloud, run_loop_with_speech};
use deskcatd::watchdog::StartupWatchdog;

const PI_SID: u32 = 1000;
const ESP_SID: u32 = 2000;

/// 台本が尽きたら、`eof`ならEOF（切断）、でなければ`WouldBlock`を返す。
#[derive(Default)]
struct Fake {
    reads: VecDeque<Vec<u8>>,
    written: Vec<u8>,
    eof: bool,
    eof_after_idle_reads: Option<usize>,
    idle_reads: usize,
    cloud_request_started: Option<Receiver<()>>,
    uart_progress: Option<Sender<()>>,
    auto_ack_text: bool,
    acked_text_id: Option<u32>,
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
            None => {
                if self.auto_ack_text {
                    let written = std::str::from_utf8(&self.written).expect("UTF-8");
                    if let Some(id) = written
                        .lines()
                        .filter_map(|line| {
                            let frame = decode_line(line).expect("frame");
                            matches!(frame.message, Message::ShowText(_))
                                .then_some(frame.envelope.id)
                        })
                        .next_back()
                        && self.acked_text_id != Some(id)
                    {
                        self.acked_text_id = Some(id);
                        let answer = Frame::new(
                            Envelope {
                                v: limits::PROTOCOL_VERSION,
                                sid: ESP_SID,
                                id: 100 + id,
                                ts_ms: 1,
                            },
                            Message::Ack(Ack {
                                reply_sid: PI_SID,
                                reply_to: id,
                                status: AckStatus::Ok,
                                code: None,
                                detail: None,
                            }),
                        );
                        let line = encode_line(&answer).expect("ACKはencodeできる");
                        let bytes = line.as_bytes();
                        buf[..bytes.len()].copy_from_slice(bytes);
                        return Ok(bytes.len());
                    }
                }
                self.idle_reads += 1;
                if let (Some(started), Some(progress)) =
                    (&self.cloud_request_started, &self.uart_progress)
                    && started.try_recv().is_ok()
                {
                    progress.send(()).expect("workerへUART進行を通知");
                }
                if self
                    .eof_after_idle_reads
                    .is_some_and(|limit| self.idle_reads >= limit)
                {
                    Ok(0)
                } else {
                    Err(io::Error::from(io::ErrorKind::WouldBlock))
                }
            }
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

fn ready() -> (Daemon, Fake) {
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
    t.feed(
        Message::Boot(Boot {
            firmware: "0.1.0".to_owned(),
            board: "esp32".to_owned(),
            reset_reason: "power_on".to_owned(),
        }),
        ESP_SID,
        2,
    );
    for _ in 0..4 {
        assert_eq!(d.step(&mut t, 10), Step::Continue);
    }
    let sent = String::from_utf8(core::mem::take(&mut t.written)).unwrap();
    let status_id = sent
        .lines()
        .map(|line| decode_line(line).unwrap())
        .find(|f| matches!(f.message, Message::GetStatus))
        .unwrap()
        .envelope
        .id;
    t.feed(
        Message::Ack(Ack {
            reply_sid: PI_SID,
            reply_to: status_id,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        }),
        ESP_SID,
        3,
    );
    t.feed(
        Message::Status(Box::new(Status {
            firmware: "0.1.0".to_owned(),
            reset_reason: "power_on".to_owned(),
            display: DisplayStatus {
                state: "ready".to_owned(),
                expression: "neutral".to_owned(),
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
        })),
        ESP_SID,
        4,
    );
    for _ in 0..4 {
        assert_eq!(d.step(&mut t, 20), Step::Continue);
    }
    assert!(d.speech_ready());
    t.written.clear();
    t.idle_reads = 0;
    t.eof_after_idle_reads = Some(2);
    (d, t)
}

#[test]
fn runner_uses_local_speech_only_with_explicit_config() {
    let (mut d, mut t) = ready();
    assert_eq!(
        run_loop(&mut d, &mut t, clock(100, 100), StartupWatchdog::new(1_000)),
        RunEnd::Exited(ExitReason::LinkLost)
    );
    assert!(t.written.is_empty(), "既定はoff");

    let (mut d, mut t) = ready();
    let config = SpeechConfig {
        min_interval_ms: NonZeroU64::new(200).unwrap(),
        duration_ms: 500,
    };
    assert_eq!(
        run_loop_with_speech(
            &mut d,
            &mut t,
            clock(100, 100),
            StartupWatchdog::new(1_000),
            Some(config)
        ),
        RunEnd::Exited(ExitReason::LinkLost)
    );
    let sent = String::from_utf8(t.written).unwrap();
    let frames: Vec<_> = sent
        .lines()
        .map(|line| decode_line(line).unwrap())
        .collect();
    assert_eq!(frames.len(), 1);
    assert!(
        matches!(&frames[0].message, Message::ShowText(text) if text.text == LOCAL_SPEECH_LINES[0] && text.duration_ms == 500)
    );
    assert_eq!(d.counters().show_text_sent, 1);
}

struct StubClaude {
    result: Result<String, Failure>,
    cloud_request_started: Sender<()>,
    uart_progress: Receiver<()>,
    progress_during_request: Arc<AtomicBool>,
}

impl Client for StubClaude {
    fn count_tokens(&mut self, _: Context) -> Result<u32, Failure> {
        self.cloud_request_started
            .send(())
            .expect("UARTへ要求開始を通知");
        self.progress_during_request.store(
            self.uart_progress
                .recv_timeout(Duration::from_secs(1))
                .is_ok(),
            Ordering::Relaxed,
        );
        Ok(20)
    }

    fn generate(&mut self, _: Context) -> Result<String, Failure> {
        self.result.clone()
    }
}

#[test]
fn cloud_worker_keeps_uart_running_and_failure_shows_local_line() {
    let (mut d, mut t) = ready();
    t.eof_after_idle_reads = Some(200);
    let (request_started_tx, request_started_rx) = mpsc::channel();
    let (uart_progress_tx, uart_progress_rx) = mpsc::channel();
    let progress_during_request = Arc::new(AtomicBool::new(false));
    t.cloud_request_started = Some(request_started_rx);
    t.uart_progress = Some(uart_progress_tx);
    let dir =
        std::env::temp_dir().join(format!("deskcat-runner-cloud-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("test dir");
    let worker = CloudSpeech::start(
        StubClaude {
            result: Err(Failure::Timeout),
            cloud_request_started: request_started_tx,
            uart_progress: uart_progress_rx,
            progress_during_request: Arc::clone(&progress_during_request),
        },
        dir.join("budget.json"),
    )
    .expect("worker");
    let config = SpeechConfig {
        min_interval_ms: NonZeroU64::new(60_000).expect("nonzero"),
        duration_ms: 500,
    };
    let mut now = 100;
    let end = run_loop_with_cloud(
        &mut d,
        &mut t,
        || {
            std::thread::sleep(Duration::from_millis(1));
            now += 1;
            now
        },
        StartupWatchdog::new(1_000),
        Some(config),
        Some(worker),
    );
    assert_eq!(end, RunEnd::Exited(ExitReason::LinkLost));
    let frames: Vec<_> = String::from_utf8(t.written)
        .expect("UTF-8")
        .lines()
        .map(|line| decode_line(line).expect("frame"))
        .collect();
    assert!(
        progress_during_request.load(Ordering::Relaxed),
        "HTTP処理中もUARTを読む"
    );
    assert!(
        matches!(&frames[0].message, Message::ShowText(text) if text.text == LOCAL_SPEECH_LINES[0])
    );
    std::fs::remove_dir_all(dir).expect("cleanup");
}

/// 依頼のたびにworker threadを落とす。意図したpanicであり、test出力に`PanicClaude`のpanicが1回出る。
/// 呼ばれた回数を数え、2回目の依頼がworkerへ届いていないことを示す。
struct PanicClaude {
    calls: Arc<AtomicUsize>,
}

impl Client for PanicClaude {
    fn count_tokens(&mut self, _: Context) -> Result<u32, Failure> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        panic!("PanicClaude: worker threadを落とす意図したpanic");
    }

    fn generate(&mut self, _: Context) -> Result<String, Failure> {
        unreachable!("count_tokensで落ちる");
    }
}

fn panic_worker(name: &str) -> (CloudSpeech, Arc<AtomicUsize>, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("deskcat-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("test dir");
    let calls = Arc::new(AtomicUsize::new(0));
    let worker = CloudSpeech::start(
        PanicClaude {
            calls: Arc::clone(&calls),
        },
        dir.join("budget.json"),
    )
    .expect("worker");
    (worker, calls, dir)
}

/// #23の「cloudのfailure時にローカルへfallbackする」のうち、依頼の時点の失敗（`submit`のErr）の根拠は、
/// このtest（A）、下のrunnerのtest（B）、`runner.rs`の`submit`がErrなら`note_failure`してローカルの行を
/// `queue_speech`する1行の分岐の3つを合わせたものである。**Bだけでは根拠にしない。**
///
/// A：`submit`が返すErrは`Failure::Worker`だけである（処理中の再依頼、または送信queueの満杯・切断）。
/// Budgetの失敗はworker側で起き、`poll`のErrで届くため、この経路には来ない。workerが落ちた後の`submit`が
/// `Err(Worker)`を返し、`in_flight()`がfalseである（処理中の守りに当たったのではない）ことを直接確かめる。
/// workerが終わってからqueueが切れるまでの一瞬に`submit`がOkを返した場合は、`poll`で結果（Err）を受けて
/// `in_flight`を戻してから、回数を決めた再試行でやり直す。
#[test]
fn cloud_submit_returns_worker_failure_after_the_worker_died() {
    let (mut worker, calls, dir) = panic_worker("cloud-submit-api-test");
    let context = Context {
        expression: Expression::Neutral,
        time_band: Some(2),
    };
    worker
        .submit(context, 500)
        .expect("生きているworkerは受け付ける");
    let mut polled = None;
    for _ in 0..2_000 {
        if let Some(result) = worker.poll() {
            polled = Some(result);
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        polled,
        Some(Err(Failure::Worker)),
        "落ちたworkerは`poll`でErr"
    );
    assert!(!worker.in_flight());
    let mut submitted = Ok(());
    for _ in 0..2_000 {
        submitted = worker.submit(context, 500);
        if submitted.is_err() {
            break;
        }
        while worker.poll().is_none() {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert_eq!(submitted, Err(Failure::Worker), "落ちたworkerへの依頼");
    assert!(!worker.in_flight(), "処理中の守りではなくqueueの切断である");
    assert_eq!(
        calls.load(Ordering::Relaxed),
        1,
        "2回目の依頼はworkerへ届かない"
    );
    std::fs::remove_dir_all(dir).expect("cleanup");
}

/// B：runnerの振る舞い。workerが最初の依頼で落ちると、1回目は`poll`のErr、2回目は`submit`のErrで、
/// どちらもローカルの行がそのまま送られ、UARTは止まらない。stubは1回だけ呼ばれる。
/// 2行目だけでは`submit`のErrと`poll`のErrを区別できない（counterも同じ）。区別はAが受け持つ。
#[test]
fn cloud_submit_failure_after_worker_death_shows_local_line_and_keeps_uart_running() {
    let (mut d, mut t) = ready();
    t.auto_ack_text = true;
    t.eof_after_idle_reads = Some(1_500);
    let (worker, calls, dir) = panic_worker("runner-cloud-submit-test");
    let config = SpeechConfig {
        min_interval_ms: NonZeroU64::new(50).expect("nonzero"),
        duration_ms: 500,
    };
    let mut now = 100;
    let end = run_loop_with_cloud(
        &mut d,
        &mut t,
        || {
            std::thread::sleep(Duration::from_millis(1));
            now += 1;
            now
        },
        StartupWatchdog::new(1_000),
        Some(config),
        Some(worker),
    );
    assert_eq!(end, RunEnd::Exited(ExitReason::LinkLost));
    let texts: Vec<_> = String::from_utf8(t.written)
        .expect("UTF-8")
        .lines()
        .map(|line| decode_line(line).expect("frame"))
        .filter_map(|frame| match frame.message {
            Message::ShowText(text) => Some(text.text),
            _ => None,
        })
        .collect();
    assert!(texts.len() >= 2, "pollのErrとsubmitのErrの2回とも送る");
    assert_eq!(texts[0], LOCAL_SPEECH_LINES[0]);
    assert_eq!(texts[1], LOCAL_SPEECH_LINES[1]);
    assert_eq!(
        calls.load(Ordering::Relaxed),
        1,
        "2回目の依頼はworkerへ届かない"
    );
    assert!(
        d.counters().show_text_accepted >= 2,
        "どちらのあともUARTを回し、ACKを処理する"
    );
    std::fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn runner_keeps_the_interval_after_each_correlated_ack() {
    let (mut d, mut t) = ready();
    t.auto_ack_text = true;
    t.eof_after_idle_reads = Some(8);
    let config = SpeechConfig {
        min_interval_ms: NonZeroU64::new(300).unwrap(),
        duration_ms: 500,
    };
    assert_eq!(
        run_loop_with_speech(
            &mut d,
            &mut t,
            clock(100, 100),
            StartupWatchdog::new(1_000),
            Some(config)
        ),
        RunEnd::Exited(ExitReason::LinkLost)
    );
    let sent = String::from_utf8(t.written).unwrap();
    let texts: Vec<_> = sent
        .lines()
        .map(|line| decode_line(line).unwrap())
        .filter(|frame| matches!(frame.message, Message::ShowText(_)))
        .collect();
    assert!(texts.len() >= 2);
    assert!(
        matches!(&texts[0].message, Message::ShowText(text) if text.text == LOCAL_SPEECH_LINES[0])
    );
    assert!(
        matches!(&texts[1].message, Message::ShowText(text) if text.text == LOCAL_SPEECH_LINES[1])
    );
    for pair in texts.windows(2) {
        assert!(pair[1].envelope.ts_ms - pair[0].envelope.ts_ms >= 300);
    }
    assert!(
        d.counters().show_text_accepted >= 2,
        "ACK後にも次の候補が出る"
    );
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
