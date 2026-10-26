//! 任意の`show_text`を1件ESP32へ送り、ACKを表示する試験用の送り手。
//!
//! [Issue #23]の残作業である`PROTO-TBD-007`（textの上限とLCDのlayout・memoryの計測。
//! `docs/protocol/esp32-pi-protocol.md` §13）を実機の窓で測るために、`deskcatd`の固定の独り言3文
//! （10・10・13字）では出せない入力を送る。**`deskcatd`のoptionにはしない。**製品の動作を変えず、
//! 試験の入力だけをここで作る。`serial_link`と同じく、このcrateが持たないloopの呼び出し側をここに置く。
//!
//! # 動作
//!
//! 1. portを開き、`hello`（`Startup`）を送る。再接続はしない（切断したら失敗で終わる）。
//! 2. ESP32の`boot`を受けてACKを返し、ESP32 sessionが確立してから（仕様§10.1。確立の直後の
//!    `get_status`は`handle_frame`が送る）`show_text`を**1件だけ**送る。確立の前には送らない。
//! 3. `show_text`へのACK（`ok`または`rejected`）を相関したら、その内容を1行で表示して終わる。
//!    `rejected`も「ACKを得た」であり、終了codeは0である。拒否が期待どおりかは、呼び出し側が
//!    表示された`code`で判断する。ACK timeoutでは同じ`id`で再送する（仕様§9）。再送予算を使い切るか
//!    `--seconds`が尽きたら失敗（終了code 1）で終わる。
//!
//! 表示するのは`show_text_result: status=… code=… detail=…`と、送った`text`の文字数・byte数・
//! `duration_ms`である。**textそのものとdevice名は出力しない**（記録へそのまま貼れるように
//! するため。`Version Record Template`が禁じている）。
//!
//! **ESP32が応えるのは、`firmware/esp32`の`show_text`を処理する版をPi linkを持つbuildで書き込んだ場合である**
//! （`pi_link.rs`のmodule doc）。`serial_link`と同じく、ESP32が`hello`を受理した後に再起動していないことが
//! 前提である。違えば`stale_session`の拒否になる。その場合は実行体を起動し直す。
//!
//! # 使い方
//!
//! ```text
//! cargo run --example show_text_probe -- --port <path> --baud <rate> \
//!     --duplicate-capacity <n> --duplicate-retention-ms <ms> --seconds <n> \
//!     --text <string> [--repeat <n>] --duration-ms <ms> [--verbose]
//! ```
//!
//! `--port`、`--baud`、`--duplicate-capacity`、`--duplicate-retention-ms`は、`serial_link`と同じ理由で
//! **必須である。**既定値を持たせない（baudは`PROTO-TBD-001`、duplicate履歴は`PROTO-TBD-005`）。
//! `--seconds`（`show_text`のACKを待つ時間を含む全体の上限）、`--text`、`--duration-ms`も必須である。
//! `--repeat <n>`は`--text`をn回つないだものを送る（884 byteなどをshellで書かずに作るため。既定は1）。
//!
//! # 実機の窓で送る入力（`PROTO-TBD-007`）
//!
//! 送り手は`--text`をそのまま送るだけで、字を選ばない。送るときの注意は次のとおり。
//!
//! - **glyph表に無い字を含む`text`は、`MissingGlyph`で丸ごと拒否される**（`crates/deskcat-face/src/text.rs`の
//!   `TextLayout::new`。描画の途中までは出さない）。`TextLayout::new`は先頭から1字ずつ、`MAX_TEXT_CHARS`を超える前に
//!   表で引くため、21字の入力でも、途中に未収録の字があると`TooLong`ではなく`MissingGlyph`の拒否になり、境界を測れない。
//!   境界を測る入力には、収録済みの字（下の表は`醐`と`M`）だけを使う。
//! - **JSON で escape が起きる文字（`"`と`\`）は使わない。**escapeされると行が1024 byteを超えうる（仕様§5.4は
//!   「`line_too_long`になりうる」）。超えた行は送り手の`encode`が`SendError::Encode`で拒否し、ESP32へは出ない。
//!   その場合は`show_text を送れない`を出し、ACKは得られず、終了code 1になる。
//! - 884 byteを日本語で作るなら、`醐`を294字（882 byte）に`MM`を足す。
//!
//! 窓で送る4点（`MAX_TEXT_CHARS`と`MAX_TEXT_DURATION_MS`は`crates/deskcat-face/src/text.rs`の値で、現版は
//! 320 ÷ (8×2) = 20と30000。値が変わっていたら読み直す）:
//!
//! | 目的 | 引数 | 期待するACK（LCDが使える場合） |
//! |---|---|---|
//! | 受理の最大（20字） | `--text 醐 --repeat 20 --duration-ms 30000` | `ok` |
//! | `TooLong`の境界（21字） | `--text 醐 --repeat 21 --duration-ms 30000` | `rejected`／`out_of_range` |
//! | wireの最大（884 byte） | `--text M --repeat 884 --duration-ms 30000` | `rejected`／`out_of_range` |
//! | 表示時間の上限（30000 ms） | 上の20字の行 | `ok`（30001以上は`out_of_range`） |
//!
//! 4点目は、20字の行の`--duration-ms`で測る。
//! **期待するACKは、LCDが使える場合に限る。**firmwareは`display_state`を先に見るため
//! （`firmware/esp32/src/pi_link.rs`）、使えない間は上のどの行も`hardware_unavailable`か`busy`で返り、
//! `out_of_range`は来ない。
//! **期待するACKは、現版のfirmwareを読んだ予想であり、実機で測った結果ではない。**測るのが目的である。
//!
//! [Issue #23]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/23

use std::io::Read;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use deskcat_protocol::{AckStatus, Frame, Hello, HelloReason, Message, Outcome, ShowText, limits};
use deskcat_serial::{
    ConnectionState, DuplicatePolicy, OutstandingKind, PeerSession, Pump, Received, RetryOutcome,
    SendError, SerialConfig, SerialDevice, Session, handle_frame, retry_due_requests,
};

/// 呼び出し側の引数。
struct Args {
    port: String,
    baud: u32,
    duplicate_policy: DuplicatePolicy,
    seconds: u64,
    message: ShowText,
    verbose: bool,
}

fn usage() -> &'static str {
    "usage: show_text_probe --port <path> --baud <rate> \
     --duplicate-capacity <n> --duplicate-retention-ms <ms> --seconds <n> \
     --text <string> [--repeat <n>] --duration-ms <ms> [--verbose]\n\
     \n\
     --port、--baud、--duplicate-capacity、--duplicate-retention-ms、--seconds、--text、--duration-ms は必須である。既定値を持たせない。\n\
     --repeat は --text をn回つないで送る（既定は1）。"
}

/// 引数の解析結果。`--help`は成功である（`serial_link`と同じ）。
enum Parsed {
    Run(Box<Args>),
    Help,
}

/// `--text`を`repeat`回つないだ`show_text`を作る。
///
/// 上限（`text`のbyte長、制御文字）はここで検査しない。**検査は`Session::send`のencodeが持つ。**
/// 窓で拒否を測る入力を、こちらの都合で先に弾かないためである。ただし、巨大な`--repeat`で
/// 確保する量だけは、1行の上限（[`limits::MAX_LINE_BYTES`]）で止める。
fn build_show_text(text: &str, repeat: usize, duration_ms: u32) -> Result<ShowText, String> {
    if repeat == 0 {
        return Err("--repeatは1以上である".to_owned());
    }
    if text.is_empty() {
        return Err("--textが空である".to_owned());
    }
    let total = text
        .len()
        .checked_mul(repeat)
        .filter(|total| *total <= limits::MAX_LINE_BYTES)
        .ok_or_else(|| {
            format!(
                "--textと--repeatの積が{} byteを超える（1行の上限）",
                limits::MAX_LINE_BYTES
            )
        })?;
    let mut repeated = String::with_capacity(total);
    for _ in 0..repeat {
        repeated.push_str(text);
    }
    Ok(ShowText {
        text: repeated,
        duration_ms,
    })
}

fn parse_args(argv: impl IntoIterator<Item = String>) -> Result<Parsed, String> {
    let mut port = None;
    let mut baud = None;
    let mut duplicate_capacity = None;
    let mut duplicate_retention_ms = None;
    let mut seconds = None;
    let mut text = None;
    let mut repeat = 1_usize;
    let mut duration_ms = None;
    let mut verbose = false;
    let mut argv = argv.into_iter();

    while let Some(flag) = argv.next() {
        let mut value = || argv.next().ok_or_else(|| format!("{flag}に値が無い"));
        match flag.as_str() {
            "--port" => port = Some(value()?),
            "--baud" => {
                baud = Some(
                    value()?
                        .parse::<u32>()
                        .map_err(|e| format!("--baudが数値でない: {e}"))?,
                );
            }
            "--duplicate-capacity" => {
                duplicate_capacity = Some(
                    value()?
                        .parse::<usize>()
                        .map_err(|e| format!("--duplicate-capacityが数値でない: {e}"))?,
                );
            }
            "--duplicate-retention-ms" => {
                duplicate_retention_ms = Some(
                    value()?
                        .parse::<u64>()
                        .map_err(|e| format!("--duplicate-retention-msが数値でない: {e}"))?,
                );
            }
            "--seconds" => {
                seconds = Some(
                    value()?
                        .parse::<u64>()
                        .map_err(|e| format!("--secondsが数値でない: {e}"))?,
                );
            }
            "--text" => text = Some(value()?),
            "--repeat" => {
                repeat = value()?
                    .parse::<usize>()
                    .map_err(|e| format!("--repeatが数値でない: {e}"))?;
            }
            "--duration-ms" => {
                duration_ms = Some(
                    value()?
                        .parse::<u32>()
                        .map_err(|e| format!("--duration-msが数値でない: {e}"))?,
                );
            }
            "--verbose" => verbose = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            other => return Err(format!("不明な引数: {other}\n\n{}", usage())),
        }
    }

    let missing = |name: &str| format!("{name}が必要である\n\n{}", usage());
    let message = build_show_text(
        &text.ok_or_else(|| missing("--text"))?,
        repeat,
        duration_ms.ok_or_else(|| missing("--duration-ms"))?,
    )?;
    Ok(Parsed::Run(Box::new(Args {
        port: port.ok_or_else(|| missing("--port"))?,
        baud: baud.ok_or_else(|| missing("--baud"))?,
        duplicate_policy: DuplicatePolicy::new(
            duplicate_capacity.ok_or_else(|| missing("--duplicate-capacity"))?,
            Duration::from_millis(
                duplicate_retention_ms.ok_or_else(|| missing("--duplicate-retention-ms"))?,
            ),
        )
        .map_err(|e| format!("duplicate履歴の設定が不正である: {e}"))?,
        seconds: seconds.ok_or_else(|| missing("--seconds"))?,
        message,
        verbose,
    })))
}

/// 起動からの経過（milliseconds）。仕様§3の`ts_ms`はwall-clock timeではない。飽和させる（`serial_link`と同じ理由）。
fn uptime_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// `/dev/urandom`の4 byteから0以外の`sid`を選ぶ（`deskcatd`の`sid_from_os`と同じ出所）。
fn random_sid() -> std::io::Result<u32> {
    let mut random = std::fs::File::open("/dev/urandom")?;
    loop {
        let mut bytes = [0_u8; 4];
        random.read_exact(&mut bytes)?;
        let sid = u32::from_le_bytes(bytes);
        if sid != 0 {
            return Ok(sid);
        }
    }
}

fn hello() -> Message {
    Message::Hello(Hello {
        host: "show_text_probe".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        reason: HelloReason::Startup,
    })
}

/// `show_text`を1件だけ送る状態。結果が決まったら`Finished`で止まる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// ESP32 sessionの確立を待っている。
    WaitSession,
    /// `show_text`を送り、ACKを待っている（`id`）。
    Sent(u32),
    /// ACKを相関した。
    Acked,
    /// 再送予算を使い切って取り下げた。
    GaveUp,
}

/// 送った`show_text`のACKを表示するために持つ結果。
#[derive(Debug, Clone, PartialEq, Eq)]
struct AckResult {
    status: AckStatus,
    code: String,
    detail: String,
}

/// 状態を進める。副作用（queueへの送出）は`main`が持ち、ここは判断だけである。
struct Probe {
    step: Step,
    result: Option<AckResult>,
}

impl Probe {
    fn new() -> Self {
        Self {
            step: Step::WaitSession,
            result: None,
        }
    }

    /// ESP32 sessionが確立していて、まだ送っていないなら`true`。
    fn wants_send(&self, peer: &PeerSession) -> bool {
        self.step == Step::WaitSession && peer.esp32_sid().is_some()
    }

    fn on_sent(&mut self, id: u32) {
        self.step = Step::Sent(id);
    }

    /// 受信を見て、送った`show_text`のACKなら結果を記録する。
    fn observe(&mut self, received: &Received) {
        let Received::Ack(correlated) = received else {
            return;
        };
        let Step::Sent(id) = self.step else {
            return;
        };
        if correlated.request != OutstandingKind::ShowText || correlated.ack.reply_to != id {
            return;
        }
        self.step = Step::Acked;
        self.result = Some(AckResult {
            status: correlated.ack.status,
            code: correlated
                .ack
                .code
                .map_or_else(|| "-".to_owned(), |code| code.as_str().to_owned()),
            detail: correlated
                .ack
                .detail
                .clone()
                .unwrap_or_else(|| "-".to_owned()),
        });
    }

    /// 再送予算を使い切った`show_text`を拾う。
    fn observe_retries(&mut self, outcomes: &[RetryOutcome]) {
        for outcome in outcomes {
            if let RetryOutcome::GaveUp(id, OutstandingKind::ShowText) = outcome
                && self.step == Step::Sent(*id)
            {
                self.step = Step::GaveUp;
            }
        }
    }

    fn finished(&self) -> bool {
        matches!(self.step, Step::Acked | Step::GaveUp)
    }
}

fn log_received(received: &Received) {
    match received {
        Received::Boot(handled) => log::info!("bootを処理した: outcome={:?}", handled.outcome),
        Received::Ack(correlated) => log::info!(
            "ackを相関した: request={:?} reply_to={} status={:?} code={:?}",
            correlated.request,
            correlated.ack.reply_to,
            correlated.ack.status,
            correlated.ack.code
        ),
        Received::Status(accepted) => {
            log::info!("statusを受理した: solicited={}", accepted.solicited);
        }
        Received::Event(frame) => log::info!(
            "eventを受けた: type={} id={}",
            frame.message.type_str(),
            frame.envelope.id
        ),
        // 拒否と無視は`handle_frame`自身が`log::warn!`へ残している。
        Received::Rejected { .. } | Received::UndefinedType(_) => {}
        // `Received`は`#[non_exhaustive]`である。増えたvariantを黙って捨てない。
        other => log::warn!("未知のReceived: {other:?}"),
    }
}

fn log_retries(outcomes: &[RetryOutcome]) {
    for outcome in outcomes {
        match outcome {
            RetryOutcome::Resent(id, kind) => log::info!("再送した: id={id} kind={kind:?}"),
            RetryOutcome::ResendFailed(..) | RetryOutcome::GaveUp(..) => {}
            other => log::warn!("未知のRetryOutcome: {other:?}"),
        }
    }
}

/// `show_text`をqueueへ入れ、ACKを相関させるために記録する。
fn send_show_text(
    session: &mut Session,
    peer: &mut PeerSession,
    probe: &mut Probe,
    message: &ShowText,
    now_ms: u64,
) {
    let message = Message::ShowText(message.clone());
    match session.send(message.clone(), now_ms) {
        Ok(id) => {
            // `Message::ShowText`は必ず追跡対象に分類される（`OutstandingKind::classify`）ため、`None`は来ない。
            let _ = peer.note_sent(id, message, now_ms);
            probe.on_sent(id);
            log::info!("show_text を queue へ入れた（id={id}）");
        }
        // queueが満杯なだけである。書き出せば空くので、次の周回で再試行する。
        Err(SendError::Dropped) => log::warn!("show_text を送れない。次の周回で再試行する"),
        Err(error) => {
            log::error!("show_text を送れない: {error}");
            probe.step = Step::GaveUp;
        }
    }
}

/// 受信したframeを`handle_frame`へ渡し、結果を`probe`へ見せる。
fn process_frames(
    frames: Vec<Frame>,
    session: &mut Session,
    peer: &mut PeerSession,
    probe: &mut Probe,
    started: Instant,
) {
    for frame in frames {
        let received = handle_frame(session, peer, frame, uptime_ms(started));
        if matches!(received, Received::Ack(_)) {
            session.note_peer_responsive();
        }
        log_received(&received);
        probe.observe(&received);
    }
}

fn install_logger(verbose: bool) -> Result<(), ()> {
    let level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    logger::install(level).map_err(|error| {
        eprintln!("loggerを入れられない: {error}");
    })
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(Parsed::Run(args)) => *args,
        Ok(Parsed::Help) => {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };
    if install_logger(args.verbose).is_err() {
        return ExitCode::FAILURE;
    }

    // **device名を出力しない。**開いた事実と設定値だけを出す。
    log::info!("baud={} で serial portを開く", args.baud);
    let config = match SerialConfig::new(&args.port, args.baud) {
        Ok(config) => config,
        Err(error) => {
            log::error!("設定が不正である: {error}");
            return ExitCode::FAILURE;
        }
    };
    let started = Instant::now();
    let Some(deadline) = started.checked_add(Duration::from_secs(args.seconds)) else {
        log::error!("--seconds が大きすぎる: {}", args.seconds);
        return ExitCode::FAILURE;
    };
    let mut device = match SerialDevice::open(&config) {
        Ok(device) => device,
        Err(error) => {
            log::error!("openに失敗した: {error}");
            return ExitCode::FAILURE;
        }
    };
    log::info!("portを開いた");

    // Piの`sid`は起動ごとのOS乱数である（仕様§3.1、`PROTO-TBD-011`）。PIDは使わない。
    let sid = match random_sid() {
        Ok(sid) => sid,
        Err(error) => {
            log::error!("sidを選べない: {error}");
            return ExitCode::FAILURE;
        }
    };
    log::info!("sid={sid} で開始する");
    let mut session = Session::new(config, sid);
    let mut peer = PeerSession::new(args.duplicate_policy.clone());
    let mut probe = Probe::new();
    session.note_connected();
    match session.send(hello(), uptime_ms(started)) {
        Ok(id) => {
            peer.note_hello_sent(id);
            log::info!("hello を queue へ入れた（id={id}, reason=Startup）");
        }
        Err(error) => {
            log::error!("hello を送れない: {error}");
            return ExitCode::FAILURE;
        }
    }

    log::info!(
        "show_text: chars={} bytes={} duration_ms={}",
        args.message.text.chars().count(),
        args.message.text.len(),
        args.message.duration_ms
    );

    pump_until_done(
        &mut session,
        &mut peer,
        &mut probe,
        &mut device,
        &args.message,
        started,
        deadline,
    );

    finish(&session, &probe, started)
}

/// 結果が決まるか、切断か、期限に達するまでpumpを回す。
fn pump_until_done(
    session: &mut Session,
    peer: &mut PeerSession,
    probe: &mut Probe,
    device: &mut SerialDevice,
    message: &ShowText,
    started: Instant,
    deadline: Instant,
) {
    while Instant::now() < deadline && !probe.finished() {
        // frameを集めてから`handle_frame`へ渡す（`session`の二重可変借用を避ける。`serial_link`と同じ）。
        let mut frames: Vec<Frame> = Vec::new();
        let read = session.pump_read(device, |outcome| match outcome {
            Outcome::Frame(frame) => {
                let (sid, id) = frame.envelope.identity();
                log::info!(
                    "行を復元した: sid={sid} id={id} type={}",
                    frame.message.type_str()
                );
                frames.push(frame);
            }
            Outcome::Rejected(rejection) => log::warn!(
                "行を拒否した: code={:?} cause={:?} detail={}",
                rejection.code(),
                rejection.cause(),
                rejection.detail()
            ),
        });
        process_frames(frames, session, peer, probe, started);

        if !matches!(read, Pump::Disconnected) {
            let now_ms = uptime_ms(started);
            let retries = retry_due_requests(session, peer, now_ms);
            log_retries(&retries);
            probe.observe_retries(&retries);
            if probe.wants_send(peer) {
                send_show_text(session, peer, probe, message, now_ms);
            }
        }

        let write = session.pump_write(device);
        let mut stop = false;
        for pump in [read, write] {
            match pump {
                Pump::Disconnected => {
                    log::error!("切断を観測した。再接続はしない");
                    stop = true;
                }
                Pump::Fatal => {
                    log::error!("再接続では直らないerrorである。停止する");
                    stop = true;
                }
                Pump::TimedOut => log::debug!("timeout（書き出しが詰まっている）"),
                Pump::Progress(_) | Pump::Idle => {}
                other => log::warn!("未知のPump: {other:?}"),
            }
        }
        if stop || matches!(session.state(), ConnectionState::Stopped(_)) {
            break;
        }
        // 書き出すものも読むものも無い周回で空回りしない。`pump_read`のread timeout（既定50 ms）が通常は待つ。
        if matches!(read, Pump::Idle) && matches!(write, Pump::Idle) {
            sleep(Duration::from_millis(1));
        }
    }
}

fn finish(session: &Session, probe: &Probe, started: Instant) -> ExitCode {
    let counters = session.counters();
    log::info!(
        "counters: bytes_in={} bytes_out={} frames_in={} rejected_in={} retries={} timeouts={}",
        counters.bytes_in,
        counters.bytes_out,
        counters.frames_in,
        counters.rejected_in,
        counters.retries,
        counters.timeouts
    );
    log::info!("経過 {:?}", started.elapsed());
    let code = match (&probe.step, &probe.result) {
        (Step::Acked, Some(result)) => {
            log::info!(
                "show_text_result: status={:?} code={} detail={}",
                result.status,
                result.code,
                result.detail
            );
            ExitCode::SUCCESS
        }
        (Step::GaveUp, _) => {
            log::error!("show_text_result: 応答が無く、再送予算を使い切った（または送れなかった）");
            ExitCode::FAILURE
        }
        (Step::WaitSession, _) => {
            log::error!("show_text_result: ESP32 sessionが確立せず、送らなかった");
            ExitCode::FAILURE
        }
        (Step::Sent(_), _) | (Step::Acked, None) => {
            log::error!("show_text_result: --secondsの間にACKを相関できなかった");
            ExitCode::FAILURE
        }
    };

    // **握りつぶしていないことをここで示す。**0件でも出す。
    let failures = logger::write_failures();
    if failures > 0 {
        eprintln!("log出力の失敗: {failures} 件");
    } else {
        log::info!("log出力の失敗: 0 件");
    }
    code
}

/// 出力先を持たない最小のlogger（`serial_link`と同じ。libraryは`log` facadeだけに依存し、実装は呼び出し側が選ぶ）。
mod logger {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 出力に失敗した件数。loggerの失敗をlogへ書けないため、counterへ落として終了時に1度読む。
    static WRITE_FAILURES: AtomicU64 = AtomicU64::new(0);

    pub fn write_failures() -> u64 {
        WRITE_FAILURES.load(Ordering::Relaxed)
    }

    struct Stderr;

    impl log::Log for Stderr {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            let mut err = std::io::stderr().lock();
            if writeln!(err, "[{:5}] {}", record.level(), record.args()).is_err() {
                WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
            }
        }

        fn flush(&self) {
            if std::io::stderr().lock().flush().is_err() {
                WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    static LOGGER: Stderr = Stderr;

    /// loggerを1度だけ入れる。
    ///
    /// # Errors
    ///
    /// 既にloggerが入っている場合に[`log::SetLoggerError`]を返す。
    pub fn install(level: log::LevelFilter) -> Result<(), log::SetLoggerError> {
        log::set_logger(&LOGGER)?;
        log::set_max_level(level);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use deskcat_protocol::{Ack, Envelope, ErrorCode, decode_line};

    use super::*;

    /// 窓で送る入力（module docの表）。
    const CJK: &str = "醐";

    fn session() -> Session {
        let config = SerialConfig::new("/dev/simulated", 115_200).unwrap();
        let mut session = Session::new(config, 7);
        session.note_connected();
        session
    }

    /// `session`が書き出した1行をdecodeして返す。行の長さ（改行を含む）も返す。
    fn written(session: &mut Session) -> (Frame, usize) {
        let mut wire = Cursor::new(Vec::new());
        assert!(matches!(session.pump_write(&mut wire), Pump::Progress(_)));
        let bytes = wire.into_inner();
        assert_eq!(bytes.last(), Some(&b'\n'), "1行は改行で終わる");
        let line = String::from_utf8(bytes.clone()).expect("UTF-8のJSONL");
        (
            decode_line(line.trim_end()).expect("送出した行をdecodeできる"),
            bytes.len(),
        )
    }

    fn args(extra: &[&str]) -> Vec<String> {
        [
            "--port",
            "/dev/simulated",
            "--baud",
            "115200",
            "--duplicate-capacity",
            "8",
            "--duplicate-retention-ms",
            "30000",
            "--seconds",
            "20",
        ]
        .iter()
        .chain(extra)
        .map(|s| (*s).to_owned())
        .collect()
    }

    fn parsed(extra: &[&str]) -> Args {
        match parse_args(args(extra)) {
            Ok(Parsed::Run(args)) => *args,
            _ => panic!("解析に成功するはずである"),
        }
    }

    #[test]
    fn the_four_window_inputs_go_out_as_one_show_text_line_each() {
        // (text, repeat, 期待する文字数, 期待するbyte数)
        for (text, repeat, chars, bytes) in [
            (CJK, 20, 20, 60),
            (CJK, 21, 21, 63),
            ("M", 884, 884, 884),
            (CJK, 20, 20, 60), // 表示時間の上限30000 ms。durationは下で確かめる。
        ] {
            let args = parsed(&[
                "--text",
                text,
                "--repeat",
                &repeat.to_string(),
                "--duration-ms",
                "30000",
            ]);
            assert_eq!(args.message.text.chars().count(), chars);
            assert_eq!(args.message.text.len(), bytes);
            assert_eq!(args.message.duration_ms, 30_000);

            let mut session = session();
            let id = session
                .send(Message::ShowText(args.message.clone()), 123)
                .expect("上限内のshow_textはencodeできる");
            let (frame, line_bytes) = written(&mut session);
            assert_eq!(frame.envelope.identity(), (7, id));
            assert_eq!(frame.envelope.ts_ms, 123);
            assert_eq!(frame.message, Message::ShowText(args.message));
            assert!(line_bytes <= limits::MAX_LINE_BYTES, "{line_bytes} byte");
        }
    }

    #[test]
    fn the_wire_line_has_the_documented_shape() {
        let args = parsed(&[
            "--text",
            "なでてくれて、ありがと。",
            "--duration-ms",
            "5000",
        ]);
        let mut session = session();
        session
            .send(Message::ShowText(args.message), 52_300)
            .unwrap();
        let mut wire = Cursor::new(Vec::new());
        let _ = session.pump_write(&mut wire);
        assert_eq!(
            String::from_utf8(wire.into_inner()).unwrap(),
            "{\"v\":1,\"sid\":7,\"id\":1,\"ts_ms\":52300,\"type\":\"show_text\",\"payload\":{\"text\":\"なでてくれて、ありがと。\",\"duration_ms\":5000}}\n"
        );
    }

    #[test]
    fn text_over_the_wire_limit_is_not_filtered_here() {
        // 885 byteはbuild_show_textを通り、送出側のencodeが拒否する（窓で拒否を測る入力を先に弾かない）。
        let args = parsed(&["--text", "M", "--repeat", "885", "--duration-ms", "30000"]);
        assert_eq!(args.message.text.len(), limits::MAX_TEXT_BYTES + 1);
        let mut session = session();
        assert!(matches!(
            session.send(Message::ShowText(args.message), 0),
            Err(SendError::Encode(_))
        ));
        assert_eq!(session.pending_out(), 0);
    }

    #[test]
    fn arguments_that_cannot_make_a_line_are_refused() {
        assert!(build_show_text("M", 0, 1).is_err());
        assert!(build_show_text("", 1, 1).is_err());
        assert!(build_show_text("M", limits::MAX_LINE_BYTES + 1, 1).is_err());
        assert!(build_show_text("M", usize::MAX, 1).is_err());
        // 必須の引数を省くと、既定値で補わずに拒否する。
        assert!(parse_args(args(&["--text", "M"])).is_err());
        assert!(parse_args(args(&["--duration-ms", "1"])).is_err());
        assert!(parse_args(args(&["--text", "M", "--duration-ms", "-1"])).is_err());
        assert!(matches!(
            parse_args(["--help".to_owned()]),
            Ok(Parsed::Help)
        ));
    }

    fn ack_frame(reply_to: u32, status: AckStatus, code: Option<ErrorCode>) -> Frame {
        Frame::new(
            Envelope {
                v: limits::PROTOCOL_VERSION,
                sid: 42,
                id: 100,
                ts_ms: 0,
            },
            Message::Ack(Ack {
                reply_sid: 7,
                reply_to,
                status,
                code,
                detail: None,
            }),
        )
    }

    fn boot_frame() -> Frame {
        Frame::new(
            Envelope {
                v: limits::PROTOCOL_VERSION,
                sid: 42,
                id: 1,
                ts_ms: 0,
            },
            Message::Boot(deskcat_protocol::Boot {
                firmware: "fake".to_owned(),
                board: "fake".to_owned(),
                reset_reason: "power_on".to_owned(),
            }),
        )
    }

    #[test]
    fn sends_once_only_after_the_esp32_session_is_established_and_reports_the_ack() {
        let args = parsed(&["--text", CJK, "--duration-ms", "1000"]);
        let mut session = session();
        let mut peer = PeerSession::new(args.duplicate_policy.clone());
        let mut probe = Probe::new();
        let started = Instant::now();

        assert!(!probe.wants_send(&peer), "boot前は送らない");
        process_frames(
            vec![boot_frame()],
            &mut session,
            &mut peer,
            &mut probe,
            started,
        );
        assert!(probe.wants_send(&peer), "boot確立後は送る");

        send_show_text(&mut session, &mut peer, &mut probe, &args.message, 10);
        let Step::Sent(id) = probe.step else {
            panic!("送ったはずである");
        };
        assert!(!probe.wants_send(&peer), "2件目は送らない");

        // 別の要求（`get_status`・`hello`）へのACKや、別のidへのACKは結果にしない。
        for (request, reply_to) in [
            (OutstandingKind::GetStatus, id),
            (OutstandingKind::Hello, id),
            (OutstandingKind::ShowText, id + 99),
        ] {
            let Message::Ack(ack) = ack_frame(reply_to, AckStatus::Ok, None).message else {
                unreachable!();
            };
            probe.observe(&Received::Ack(deskcat_serial::CorrelatedAck {
                request,
                ack,
            }));
            assert_eq!(probe.step, Step::Sent(id));
        }

        process_frames(
            vec![ack_frame(
                id,
                AckStatus::Rejected,
                Some(ErrorCode::OutOfRange),
            )],
            &mut session,
            &mut peer,
            &mut probe,
            started,
        );
        assert!(probe.finished());
        let result = probe.result.expect("結果がある");
        assert_eq!(result.status, AckStatus::Rejected);
        assert_eq!(result.code, "out_of_range");
    }
}
