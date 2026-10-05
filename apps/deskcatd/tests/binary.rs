//! `deskcatd`の実行体を、擬似端末（PTY）につないで確かめる。
//!
//! 子processとして実binaryを起動し、PTYのmaster側を偽の`ESP32`にする。確かめるのは、終了コードと、
//! 出力にある行の有無だけである。**確かめないもの**: 実port、Pi・ESP32の実機（#11の後半）、systemd上の起動。
//! Linux専用である（slaveのpathを`/proc/self/fd`から得る）。

#![cfg(target_os = "linux")]

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use deskcat_protocol::{
    Ack, AckStatus, Boot, DisplayStatus, Envelope, ErrorCode, ExpressionName, Frame, HeadTouched,
    Message, ProtocolCounters, SensorStatus, ServoStatus, Status, decode_line, encode_line, limits,
};
use serial2::SerialPort;

const ESP_SID: u32 = 2000;
/// 待ちの上限。判定のための長さではなく、止まったときに失敗させるための上限である。
const LIMIT: Duration = Duration::from_secs(30);

struct Run {
    code: Option<i32>,
    stderr: String,
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_deskcatd"))
}

fn args(port: &str, baud: &str, deadline_ms: &str) -> Vec<String> {
    [
        "--port",
        port,
        "--baud",
        baud,
        "--duplicate-capacity",
        "8",
        "--duplicate-retention-ms",
        "5000",
        "--startup-deadline-ms",
        deadline_ms,
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

fn wait(mut child: Child) -> Run {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("状態を読める") {
            let mut stderr = String::new();
            child
                .stderr
                .take()
                .expect("piped")
                .read_to_string(&mut stderr)
                .expect("読める");
            return Run {
                code: status.code(),
                stderr,
            };
        }
        if started.elapsed() > LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            panic!("子processが終わらない");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn run_plain(args: &[&str]) -> Run {
    wait(
        bin()
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("起動できる"),
    )
}

/// PTYの対。`master`は偽のESP32、`slave_path`はbinaryへ渡すpath。`_slave`は閉じない（閉じると切断に見える）。
struct Pty {
    master: SerialPort,
    _slave: SerialPort,
    slave_path: String,
}

fn pty() -> Pty {
    let (a, b) = SerialPort::pair().expect("PTYの対を開ける");
    let link = |p: &SerialPort| {
        std::fs::read_link(format!("/proc/self/fd/{}", p.as_raw_fd()))
            .expect("fdのpathを読める")
            .to_string_lossy()
            .into_owned()
    };
    let (la, lb) = (link(&a), link(&b));
    let (master, slave, slave_path) = if lb.starts_with("/dev/pts/") {
        (a, b, lb)
    } else {
        assert!(la.starts_with("/dev/pts/"), "slave側が見つからない");
        (b, a, la)
    };
    let mut master = master;
    master
        .set_read_timeout(Duration::from_millis(50))
        .expect("設定できる");
    Pty {
        master,
        _slave: slave,
        slave_path,
    }
}

fn spawn_on(pty: &Pty, deadline_ms: &str) -> Child {
    bin()
        .args(args(&pty.slave_path, "115200", deadline_ms))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("起動できる")
}

/// 偽のESP32の入出力。
struct Esp<'a> {
    master: &'a mut SerialPort,
    buf: Vec<u8>,
    next_id: u32,
}

impl<'a> Esp<'a> {
    fn new(master: &'a mut SerialPort) -> Self {
        Self {
            master,
            buf: Vec::new(),
            next_id: 1,
        }
    }

    fn send(&mut self, message: Message) {
        let frame = Frame::new(
            Envelope {
                v: limits::PROTOCOL_VERSION,
                sid: ESP_SID,
                id: self.next_id,
                ts_ms: 1,
            },
            message,
        );
        self.next_id += 1;
        let line = encode_line(&frame).expect("encodeできる");
        self.master.write_all(line.as_bytes()).expect("書ける");
    }

    /// `pick`が値を返すframeが来るまで読む。
    fn read_until<T>(&mut self, mut pick: impl FnMut(&Frame) -> Option<T>) -> T {
        let started = Instant::now();
        loop {
            while let Some(i) = self.buf.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = self.buf.drain(..=i).collect();
                let text = String::from_utf8_lossy(&line);
                if let Ok(frame) = decode_line(text.trim_end())
                    && let Some(found) = pick(&frame)
                {
                    return found;
                }
            }
            assert!(started.elapsed() < LIMIT, "期待したframeが来ない");
            let mut chunk = [0_u8; 512];
            if let Ok(n) = self.master.read(&mut chunk) {
                self.buf.extend_from_slice(&chunk[..n]);
            }
        }
    }

    fn hello(&mut self) -> (u32, u32) {
        self.read_until(|f| match f.message {
            Message::Hello(_) => Some(f.envelope.identity()),
            _ => None,
        })
    }
}

fn boot() -> Message {
    Message::Boot(Boot {
        firmware: "0.1.0".to_owned(),
        board: "esp32".to_owned(),
        reset_reason: "power_on".to_owned(),
    })
}

fn ack(pi_sid: u32, reply_to: u32, status: AckStatus, code: Option<ErrorCode>) -> Message {
    Message::Ack(Ack {
        reply_sid: pi_sid,
        reply_to,
        status,
        code,
        detail: None,
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

/// `boot`、`hello`のACK、`get_status`のACKと`status`まで進める。返すのはPiの`sid`。
fn establish(esp: &mut Esp<'_>) -> u32 {
    let (pi_sid, hello_id) = esp.hello();
    esp.send(boot());
    esp.send(ack(pi_sid, hello_id, AckStatus::Ok, None));
    let get_status = esp.read_until(|f| match f.message {
        Message::GetStatus => Some(f.envelope.id),
        _ => None,
    });
    esp.send(ack(pi_sid, get_status, AckStatus::Ok, None));
    esp.send(status("neutral"));
    pi_sid
}

#[test]
fn help_succeeds() {
    assert_eq!(run_plain(&["--help"]).code, Some(0));
}

#[test]
fn no_arguments_is_exit_1() {
    assert_eq!(run_plain(&[]).code, Some(1));
}

#[test]
fn zero_baud_is_exit_1() {
    let a = args("/dev/null", "0", "1000");
    let a: Vec<&str> = a.iter().map(String::as_str).collect();
    assert_eq!(run_plain(&a).code, Some(1));
}

#[test]
fn an_empty_port_is_exit_1() {
    let a = args("", "115200", "1000");
    let a: Vec<&str> = a.iter().map(String::as_str).collect();
    assert_eq!(run_plain(&a).code, Some(1));
}

#[test]
fn a_missing_port_is_exit_75() {
    let a = args("/nonexistent/deskcat-test", "115200", "1000");
    let a: Vec<&str> = a.iter().map(String::as_str).collect();
    assert_eq!(run_plain(&a).code, Some(75));
}

#[test]
fn a_silent_peer_ends_at_the_startup_deadline_with_75() {
    let p = pty();
    let run = wait(spawn_on(&p, "500"));
    assert_eq!(run.code, Some(75));
    assert!(run.stderr.contains("起動の期限を過ぎた"), "{}", run.stderr);
    assert!(!run.stderr.contains(&p.slave_path), "device名を出力しない");
}

#[test]
fn a_silent_peer_with_a_long_deadline_ends_by_the_hello_budget() {
    let p = pty();
    let run = wait(spawn_on(&p, "60000"));
    assert_eq!(run.code, Some(75));
    assert!(run.stderr.contains("HelloRetryExhausted"), "{}", run.stderr);
    assert!(!run.stderr.contains("StartupDeadline"), "{}", run.stderr);
}

#[test]
fn a_touch_reaches_the_esp32_as_happy_and_a_closed_link_is_75() {
    let mut p = pty();
    let child = spawn_on(&p, "60000");
    {
        let mut esp = Esp::new(&mut p.master);
        establish(&mut esp);
        esp.send(Message::HeadTouched(HeadTouched { duration_ms: 700 }));
        let name = esp.read_until(|f| match &f.message {
            Message::SetExpression(e) => Some(e.name),
            _ => None,
        });
        assert_eq!(name, ExpressionName::Happy);
    }
    let Pty {
        master,
        _slave: slave,
        ..
    } = p;
    drop(master);
    drop(slave);
    let run = wait(child);
    assert_eq!(run.code, Some(75));
    assert!(run.stderr.contains("LinkLost"), "{}", run.stderr);
}

#[test]
fn a_rejected_hello_is_exit_1() {
    let mut p = pty();
    let child = spawn_on(&p, "60000");
    {
        let mut esp = Esp::new(&mut p.master);
        let (pi_sid, hello_id) = esp.hello();
        esp.send(boot());
        esp.send(ack(
            pi_sid,
            hello_id,
            AckStatus::Rejected,
            Some(ErrorCode::InvalidPayload),
        ));
    }
    let run = wait(child);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains("HelloRejected"), "{}", run.stderr);
}

#[test]
fn a_new_esp32_session_after_establishing_is_75() {
    let mut p = pty();
    let child = spawn_on(&p, "60000");
    {
        let mut esp = Esp::new(&mut p.master);
        establish(&mut esp);
        let frame = Frame::new(
            Envelope {
                v: limits::PROTOCOL_VERSION,
                sid: ESP_SID + 1,
                id: 1,
                ts_ms: 1,
            },
            boot(),
        );
        let line = encode_line(&frame).expect("encodeできる");
        esp.master.write_all(line.as_bytes()).expect("書ける");
    }
    let run = wait(child);
    assert_eq!(run.code, Some(75));
    assert!(run.stderr.contains("EspRebooted"), "{}", run.stderr);
}

#[test]
fn a_hello_ack_without_a_boot_ends_at_the_startup_deadline() {
    let mut p = pty();
    // 期限は、helloの予算（約2秒）より長くする。ACKを捨てる実装なら、期限より先にHelloRetryExhaustedで終わる。
    let child = spawn_on(&p, "5000");
    {
        let mut esp = Esp::new(&mut p.master);
        let (pi_sid, hello_id) = esp.hello();
        esp.send(ack(pi_sid, hello_id, AckStatus::Ok, None));
    }
    let run = wait(child);
    assert_eq!(run.code, Some(75));
    assert!(run.stderr.contains("StartupDeadline"), "{}", run.stderr);
}
