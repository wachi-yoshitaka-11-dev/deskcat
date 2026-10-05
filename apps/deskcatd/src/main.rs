//! `deskcatd`の実行体。library（`Daemon`）を、実portと時計とloggerへつなぐだけにする。
//! 判断はlibraryに置く。引数は既定値を持たない（`deskcatd::cli`）。
//!
//! # 終了コード
//!
//! | 場面 | 終了コード |
//! |---|---|
//! | `--help` | 0 |
//! | 引数の誤り（UTF-8でない引数を含む）、`sid`を選べない、設定が不正（baudの0を含む） | 1 |
//! | `SerialDevice::open`に失敗した（**再接続しない**） | 75 |
//! | `Daemon`が終了を求めた | `ExitReason::exit_code`（75か1） |
//! | **起動の期限を過ぎた**（sessionの確立と`hello`の結果がそろわない） | 75（`EXIT_SESSION_REDO`） |
//!
//! 起動の期限の値は引数で与える。**値の根拠は無い**（`ESP32`の起動時間は未測定。対応するTBDの登録は無い）。
//! 置く場所は段階2b-iiiのunitの1か所だけである。ACK timeoutと再接続の方針は
//! `SerialConfig::new`が持つ暫定値（`RetryPolicy::provisional`など）を、`hello`の再送回数などは`daemon`の暫定の定数
//! （`HELLO_RETRY_LIMIT`など）を使い、ここでは値を足さない。いずれも根拠は無い。
//!
//! SIGTERMは既定の動作のままにする（handlerには`unsafe`か新しい依存が要る）。
//! 標準出力と標準エラーへの書き込みの失敗は捨てる（終了コードを変えない）。
//! 終了時にcounterを1行ずつ出す。**device名は出力しない。**

use std::io::Write as _;
use std::process::ExitCode;
use std::time::Instant;

use deskcat_domain::{Millis, Mind, Timing};
use deskcat_serial::{PeerSession, SerialConfig, SerialDevice, Session};
use deskcatd::cli::{self, Parsed};
use deskcatd::daemon::{Daemon, EXIT_FATAL, EXIT_SESSION_REDO, Identity};
use deskcatd::runner::run_loop;
use deskcatd::sid::sid_from_os;
use deskcatd::watchdog::StartupWatchdog;

/// 起動からの経過（ms）。単調で、wall-clock timeではない。
///
/// 飽和させる。切り捨てると値が巻き戻る。
fn uptime_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn main() -> ExitCode {
    // `std::env::args()`は、UTF-8でない引数でpanicする（終了101）。引数の誤りは終了1にする。
    let Ok(raw) = std::env::args_os()
        .skip(1)
        .map(std::ffi::OsString::into_string)
        .collect::<Result<Vec<_>, _>>()
    else {
        let _ = writeln!(std::io::stderr(), "UTF-8でない引数がある\n\n{}", cli::USAGE);
        return ExitCode::from(EXIT_FATAL);
    };
    let args = match cli::parse(raw) {
        Ok(Parsed::Run(args)) => *args,
        Ok(Parsed::Help) => {
            let _ = writeln!(std::io::stdout(), "{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error}\n\n{}", cli::USAGE);
            return ExitCode::from(EXIT_FATAL);
        }
    };

    let level = if args.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    if let Err(error) = logger::install(level) {
        let _ = writeln!(std::io::stderr(), "loggerを入れられない: {error}");
        return ExitCode::from(EXIT_FATAL);
    }

    // **固定値へ落とさない。**同じ`sid`を選ぶと、再起動しても同じ`sid`になる（仕様§3）。
    let sid = match sid_from_os() {
        Ok(sid) => sid,
        Err(error) => {
            log::error!("sidを選べない: {error}");
            return ExitCode::from(EXIT_FATAL);
        }
    };
    let config = match SerialConfig::new(&args.port, args.baud) {
        Ok(config) => config,
        Err(error) => {
            log::error!("設定が不正である: {error}");
            return ExitCode::from(EXIT_FATAL);
        }
    };

    // **device名を出力しない。**
    log::info!("baud={} で serial portを開く。sid={sid}", args.baud);
    let mut device = match SerialDevice::open(&config) {
        Ok(device) => device,
        Err(error) => {
            // 再接続しない。選び直しはsupervisorの再起動が行う。
            log::error!("openに失敗した: {error}");
            return ExitCode::from(EXIT_SESSION_REDO);
        }
    };

    let started = Instant::now();
    let mut daemon = Daemon::new(
        Session::new(config, sid),
        PeerSession::new(args.duplicate_policy),
        Mind::new(Timing::MVP, Millis(0)),
        Identity {
            host: "deskcatd".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        },
    );
    daemon.on_connected();

    let end = run_loop(
        &mut daemon,
        &mut device,
        || uptime_ms(started),
        StartupWatchdog::new(args.startup_deadline_ms.get()),
    );

    log::info!("終了する: {end:?}");
    log::info!("session: {:?}", daemon.session_counters());
    log::info!("peer: {:?}", daemon.peer_counters());
    log::info!("daemon: {:?}", daemon.counters());
    let failures = logger::write_failures();
    if failures > 0 {
        let _ = writeln!(std::io::stderr(), "log出力の失敗: {failures} 件");
    }
    ExitCode::from(end.exit_code())
}

/// stderrへ出すlogger。journaldがstderrを拾う。
///
/// **libraryは`log` facadeだけに依存し、実装の選択は呼び出し側に残す**（`Cargo.toml`の方針）。
/// 依存を増やさないため、`deskcat-serial`の`serial_link`と同じ形で自前で書く。
mod logger {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 出力に失敗した件数。loggerの失敗をlogへ書けないため、counterへ落として終了時に1度だけ読む。
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

    /// loggerを1度だけ入れる。`log`の`std` featureを要求しないため`set_logger`を使う。
    ///
    /// # Errors
    ///
    /// 既にloggerが入っている場合。
    pub fn install(level: log::LevelFilter) -> Result<(), log::SetLoggerError> {
        log::set_logger(&LOGGER)?;
        log::set_max_level(level);
        Ok(())
    }
}
