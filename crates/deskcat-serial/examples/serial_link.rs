//! 実serial portへ繋いでlinkを確かめる実行体。
//!
//! `SerialDevice`を開き、`Session`のpumpを回し、切断したら方針の範囲で再接続する。
//! **このcrateはloopを持たない**（`Session`はtransportを所有せず、pumpの引数で受け取る）
//! ため、その呼び出し側をここに1つ置く。[Issue #11]の後半（Pi実機）は、これを走らせる。
//!
//! # これで確かめられること／確かめられないこと
//!
//! **確かめられるのは「行が通ること」までである。**open、byteのread／write、行の復元、
//! 切断の観測、再接続の上限、partial I/Oはここで見える。
//!
//! **`protocol`が成立したことは確かめられない。**`boot`／`ping`／`status`／ACK／reconnect
//! 同期の実装は[Issue #12]であり、**`ESP32`側がprotocolを話すとは限らない。**
//! 起動時に`hello`を1件送るのは書き出し経路を通すためであって、handshakeではない。
//! 記録するときは「行が通った」と「protocolが成立した」を書き分ける。
//!
//! # 使い方
//!
//! ```text
//! cargo run --example serial_link -- --port <path> --baud <rate> [--seconds <n>] [--verbose]
//! ```
//!
//! `--port`と`--baud`は**どちらも必須である。**既定値を持たせない。device名は未確認で
//! あり（確定はIssue #11の後半）、baudの正本は`PROTO-TBD-001`でいずれも`Candidate`である。
//! **確認していない値を既定として固定しない。**渡した値は記録にそのまま残る。
//!
//! `--seconds`を省くと、再接続の上限に達してsessionが停止するまで走り続ける。
//!
//! `--verbose`を付けない限り`Info`までを出す。`Debug`まで上げるとread timeoutごとに
//! 1行出るため（既定50 msなので毎秒20行）、長時間の観察では本当のeventが埋まる。
//! 切り分けが要るときだけ上げる。
//!
//! # 記録に残さないもの
//!
//! **device名を出力へ書かない。**`Version Record Template`が禁じている項目であり、
//! この出力をそのまま記録へ貼れるようにしておく。開いた事実だけを出す。
//!
//! [Issue #11]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11
//! [Issue #12]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12

use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use deskcat_protocol::{Hello, HelloReason, Message, Outcome};
use deskcat_serial::{ConnectionState, Pump, SerialConfig, SerialDevice, Session, SessionCounters};

/// 呼び出し側の引数。
struct Args {
    port: String,
    baud: u32,
    seconds: Option<u64>,
    verbose: bool,
}

fn usage() -> &'static str {
    "usage: serial_link --port <path> --baud <rate> [--seconds <n>] [--verbose]\n\
     \n\
     --port と --baud は必須である。既定値を持たせない。\n\
     device名は未確認であり、baudの正本は PROTO-TBD-001 である。"
}

/// 引数の解析結果。
///
/// `--help`は**成功である。**`Err`へ畳むとexit codeが1になり、
/// script側から「使い方を聞いた」と「引数を間違えた」を区別できない。
enum Parsed {
    /// 実行する。
    Run(Box<Args>),
    /// 使い方を出して正常終了する。
    Help,
}

fn parse_args() -> Result<Parsed, String> {
    let mut port = None;
    let mut baud = None;
    let mut seconds = None;
    let mut verbose = false;
    let mut argv = std::env::args().skip(1);

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
            "--seconds" => {
                seconds = Some(
                    value()?
                        .parse::<u64>()
                        .map_err(|e| format!("--secondsが数値でない: {e}"))?,
                );
            }
            "--verbose" => verbose = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            other => return Err(format!("不明な引数: {other}\n\n{}", usage())),
        }
    }

    Ok(Parsed::Run(Box::new(Args {
        port: port.ok_or_else(|| format!("--portが必要である\n\n{}", usage()))?,
        baud: baud.ok_or_else(|| format!("--baudが必要である\n\n{}", usage()))?,
        seconds,
        verbose,
    })))
}

/// 起動からの経過（milliseconds）。仕様§3の`ts_ms`はwall-clock timeではない。
///
/// `u128`から`u64`へは飽和させる。切り捨てると値が巻き戻り、**単調増加という
/// 仕様上の性質が壊れる。**飽和なら止まるだけで、逆行しない。
fn uptime_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// 起動時に1件だけ送るmessage。**handshakeではない**（module docを読む）。
fn hello() -> Message {
    Message::Hello(Hello {
        host: "serial_link".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        reason: HelloReason::Startup,
    })
}

fn report(counters: SessionCounters, state: ConnectionState) {
    log::info!("state: {state:?}");
    log::info!(
        "counters: bytes_in={} bytes_out={} frames_in={} rejected_in={}",
        counters.bytes_in,
        counters.bytes_out,
        counters.frames_in,
        counters.rejected_in
    );
    log::info!(
        "counters: disconnects={} reconnect_attempts={} timeouts={} retries={}",
        counters.disconnects,
        counters.reconnect_attempts,
        counters.timeouts,
        counters.retries
    );
    log::info!(
        "counters: dropped_out={} discarded_on_disconnect={} encode_failed={}",
        counters.dropped_out,
        counters.discarded_on_disconnect,
        counters.encode_failed
    );
}

fn main() -> ExitCode {
    let args = match parse_args() {
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

    // **既定はInfoである。**`Debug`にすると、dataが来ていないだけの状態でも
    // read timeoutごとに1行出る（既定50 msなので毎秒20行）。長時間の実機観察では
    // それが本当のeventを埋めてしまう。切り分けが要るときだけ`--verbose`で上げる。
    logger::install(if args.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    });

    // **device名を出力しない。**開いた事実と設定値だけを出す。
    log::info!("baud={} で serial portを開く", args.baud);

    let config = match SerialConfig::new(&args.port, args.baud) {
        Ok(config) => config,
        Err(error) => {
            // loggerは導入済みである。ここだけeprintlnにすると出力先と整形が揃わない。
            log::error!("設定が不正である: {error}");
            return ExitCode::FAILURE;
        }
    };

    // sidは起動ごとに新しい値を選ぶ（仕様§3）。**自動で選び直さない。**
    // ここではprocessのidを種にする。乱数を持ち込まない。
    let sid = std::process::id();
    let mut session = Session::new(config.clone(), sid);
    let started = Instant::now();
    let deadline = args.seconds.map(|s| started + Duration::from_secs(s));

    log::info!("sid={sid} で開始する");

    loop {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            log::info!("--seconds に達した");
            break;
        }

        // **openのerrorをIoDisposition::classifyへ渡さない。**再列挙中の一時的な失敗
        // （ENOENT／EACCES／EBUSY）をFatalにすると、復帰しようとしている場面で止まる。
        let mut device = match SerialDevice::open(&config) {
            Ok(device) => device,
            Err(error) => {
                log::warn!("openに失敗した: {error}");
                let Some(backoff) = session.begin_reconnect() else {
                    log::error!("再接続の上限に達した");
                    break;
                };
                log::info!("{backoff:?} 待って再試行する");
                sleep(backoff);
                continue;
            }
        };

        log::info!("portを開いた");
        session.note_connected();

        // 書き出し経路を通すために1件だけ送る。**handshakeではない。**
        match session.send(hello(), uptime_ms(started)) {
            Ok(id) => log::info!("hello を queue へ入れた（id={id}）"),
            Err(error) => log::warn!("hello を送れない: {error}"),
        }

        let disconnected = pump_until_break(&mut session, &mut device, deadline);

        if !disconnected {
            break; // deadline到達、または停止済み。summaryはloopの外で1度だけ出す
        }

        // 切断ごとの区切りとして出す。**loopを抜けた後にもう一度出さない**
        // （同じ数字が2回並ぶと、どちらが最終値か読めない）。
        report(session.counters(), session.state());

        let Some(backoff) = session.begin_reconnect() else {
            log::error!("再接続の上限に達した。停止する");
            break;
        };
        log::info!("切断した。{backoff:?} 待って再接続する");
        sleep(backoff);
    }

    report(session.counters(), session.state());
    log::info!("経過 {:?}", started.elapsed());

    if matches!(session.state(), ConnectionState::Stopped(_)) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// linkが切れるか、期限に達するまでpumpを回す。切断で抜けたときだけ`true`。
fn pump_until_break(
    session: &mut Session,
    device: &mut SerialDevice,
    deadline: Option<Instant>,
) -> bool {
    loop {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return false;
        }

        let read = session.pump_read(device, |outcome| match outcome {
            Outcome::Frame(frame) => {
                let (sid, id) = frame.envelope.identity();
                // **payloadを出さない。**上位（Issue #12）が扱う。ここはlinkの確認である。
                log::info!("行を復元した: sid={sid} id={id} type={:?}", frame.message);
            }
            Outcome::Rejected(rejection) => {
                log::warn!(
                    "行を拒否した: code={:?} cause={:?} detail={}",
                    rejection.code(),
                    rejection.cause(),
                    rejection.detail()
                );
            }
        });
        let write = session.pump_write(device);

        for pump in [read, write] {
            match pump {
                Pump::Disconnected => {
                    log::warn!("切断を観測した");
                    return true;
                }
                Pump::Fatal => {
                    log::error!("再接続では直らないerrorである。停止する");
                    return false;
                }
                Pump::TimedOut => log::debug!("timeout（書き出しが詰まっている）"),
                Pump::Progress(_) | Pump::Idle => {}
                // `Pump`は`#[non_exhaustive]`である。増えたvariantを
                // 「進捗あり」と同じ扱いへ落とさない。見えるようにしておく。
                other => log::warn!("未知のPump: {other:?}"),
            }
        }

        if matches!(session.state(), ConnectionState::Stopped(_)) {
            return false;
        }
    }
}

/// 出力先を持たない最小のlogger。
///
/// **libraryは`log` facadeだけに依存し、実装の選択は呼び出し側に残す**という
/// `Cargo.toml`の方針に従い、ここで選ぶ。依存を増やさないため自前で書く。
mod logger {
    use std::io::Write as _;

    struct Stderr;

    impl log::Log for Stderr {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            let mut err = std::io::stderr().lock();
            let _ = writeln!(err, "[{:5}] {}", record.level(), record.args());
        }

        fn flush(&self) {
            let _ = std::io::stderr().lock().flush();
        }
    }

    static LOGGER: Stderr = Stderr;

    /// loggerを1度だけ入れる。
    ///
    /// `set_boxed_logger`ではなく`set_logger`を使うのは、`log`の`std` featureを
    /// 要求しないためである（`tests/simulator.rs`と同じ理由）。
    ///
    /// `level`より下は`log`側で捨てられる。
    pub fn install(level: log::LevelFilter) {
        let _ = log::set_logger(&LOGGER);
        log::set_max_level(level);
    }
}
