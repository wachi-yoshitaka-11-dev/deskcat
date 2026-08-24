//! DeskCat ESP32 の最小firmware。
//!
//! [Issue #6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6) の受け入れ条件に
//! 対応し、**未検証のperipheralを初期化しない。**
//!
//! - firmware build identity を出す
//! - board-configuration ID を出す
//! - reset reason を出す
//! - **servoも未知のoutputもdriveしない**
//!
//! [Issue #7](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/7) で heartbeat を足した。
//!
//! - **rate limit を持つ** heartbeat loop（周期は [`HEARTBEAT_PERIOD`]）
//! - **単調増加する uptime**（`ts_ms` と同じ `u64` ミリ秒）
//! - reset reason の取得を維持する（回帰）
//! - **watchdog の進行を妨げない**（周期ごとに `std::thread::sleep` で明示的に譲る）
//! - counter は `deskcat-protocol` の [`ProtocolCounters`] を再利用する。**新しい counter 型を作らない**
//!
//! **`Peripherals::take()` を呼ばない。**これがGPIOを一切駆動しない根拠である。
//! GPIO割り当ては `docs/hardware/gpio-assignment.md` が導通checkなどを待って
//! `Blocked` であり、この firmware は pin へ触れない。
//!
//! **Protocol sessionは確立しない。**`crates/deskcat-protocol` の `Boot` message を
//! 送るのは別の作業であり、ここでは log へ出すだけである。ただし `reset_reason` の
//! 文字列は同 crate の fixture が使う snake_case へ揃えてある。

/// Board-configuration ID。
///
/// `crates/deskcat-protocol` の fixture が `"esp32"` を使っており、それへ揃える。
/// 値の意味は同 crate の `Boot` message の `board` field である。
const BOARD: &str = "esp32";

/// `ResetReason` を Protocol の語彙（snake_case）へ写す。
///
/// **列挙値の意味を推測で足さない。**`esp_idf_svc::hal::reset::ResetReason` は
/// `#[non_exhaustive]` ではないため、variant が増えたときは compile error で気付く。
fn reset_reason_str(reason: esp_idf_svc::hal::reset::ResetReason) -> &'static str {
    use esp_idf_svc::hal::reset::ResetReason;

    match reason {
        ResetReason::Software => "software",
        ResetReason::ExternalPin => "external_pin",
        ResetReason::Watchdog => "watchdog",
        ResetReason::Sdio => "sdio",
        ResetReason::Panic => "panic",
        ResetReason::InterruptWatchdog => "interrupt_watchdog",
        ResetReason::PowerOn => "power_on",
        ResetReason::Unknown => "unknown",
        ResetReason::Brownout => "brownout",
        ResetReason::TaskWatchdog => "task_watchdog",
        ResetReason::DeepSleep => "deep_sleep",
        ResetReason::USBPeripheral => "usb_peripheral",
        ResetReason::JTAG => "jtag",
        ResetReason::EfuseError => "efuse_error",
        ResetReason::PowerGlitch => "power_glitch",
        ResetReason::CPULockup => "cpu_lockup",
    }
}

fn main() {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // build identity。profile は `Cargo.toml` の `[profile.dev]`／`[profile.release]` に対応する。
    // **`esp_app_desc!()` は使わない。**`#[no_mangle]`／`#[link_section]` を展開するため
    // `Cargo.toml` の `unsafe_code = "forbid"` に触れる。
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    log::info!(
        "firmware={} version={} profile={}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        profile
    );

    log::info!("board={BOARD}");

    // `ResetReason::get()` は safe fn である。`unsafe` は esp-idf-hal 内部にあり、
    // `unsafe_code` は crate 単位の lint なのでこの crate には効かない。
    let reason = esp_idf_svc::hal::reset::ResetReason::get();
    log::info!("reset_reason={} raw={:?}", reset_reason_str(reason), reason);

    // **peripheralを取らない。**GPIOもservoも触らない。
    log::info!("peripherals=untouched servo=not_driven");

    run_heartbeat(reason);
}

/// Heartbeat の周期。
///
/// **この値は暫定である。**一次資料にも実測にも基づいていない。**確定値として扱わない。**
/// 5 秒は、(a) log が serial monitor を埋めない、(b) Task Watchdog Timer の既定値に対して
/// 十分に短い、の2点だけを満たす見当である。**どちらも根拠として測っていない。**
///
/// **hardcode ではなく、この定数を唯一の設定点として持つ。**周期を変えるときはここだけを
/// 変える。Pi 側から provision できる仕組みができた時点で、この定数はその既定値へ移る。
/// **provisioning は本 Issue の範囲ではない。**
const HEARTBEAT_PERIOD: core::time::Duration = core::time::Duration::from_secs(5);

/// Heartbeat の状態。
///
/// **`ProtocolCounters` を再利用し、新しい counter 型を作らない。**各 field の意味の正本は
/// protocol §4.6 の counter 対応表であり、**ここへ再掲しない。**
struct Heartbeat {
    /// 単調時計の起点。`std::time::Instant` は monotonic であり、
    /// **`gettimeofday` 由来の壁時計を使わない**（SNTP で飛ぶと uptime の単調性が崩れる）。
    started: std::time::Instant,
    /// 直前に出した uptime。単調性の検査に使う。
    last_uptime_ms: u64,
    /// Heartbeat の連番。
    seq: u64,
    /// Protocol status へそのまま渡せる counter 群。
    ///
    /// **protocol session を確立していないため、現時点ではすべて 0 のままである。**
    /// 0 であることは「数えていない」ではなく「該当事象が起きていない」を表す。
    counters: deskcat_protocol::ProtocolCounters,
    /// 単調時計が逆行した回数。
    ///
    /// **`ProtocolCounters` には該当する field が無い。**§4.6 の counter はいずれも
    /// message 処理の事象であり、時計の異常を表す field は定義されていない。
    /// **そこへ無理に押し込むと counter の意味が壊れるため、firmware local に持つ。**
    /// **これは protocol status へ出す値ではない。**
    clock_regressions: u32,
}

impl Heartbeat {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            last_uptime_ms: 0,
            seq: 0,
            counters: deskcat_protocol::ProtocolCounters::default(),
            clock_regressions: 0,
        }
    }

    /// 単調増加を保証した uptime をミリ秒で返す。
    ///
    /// **型は `u64` である。**protocol §3 が `ts_ms` を `u64` にした理由（`u32` は約 49.7 日で
    /// wrap し、長時間動作で単調性が崩れる）はここにも当たる。
    ///
    /// **逆行を握りつぶさない。**検知したら分類して log と counter へ出し、
    /// **返す値は直前値に留める**（下げない）。
    fn uptime_ms(&mut self) -> u64 {
        // `u128` から `u64` への変換。`u128` のミリ秒が `u64` を超えるのは約 5.8 億年後で
        // あり実機では到達しないが、**`as` による暗黙の切り捨てを作らない。**
        // `try_from` で明示し、あり得ない側へ落ちたときは飽和させる。
        let raw = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);

        if raw < self.last_uptime_ms {
            self.clock_regressions = self.clock_regressions.saturating_add(1);
            log::warn!(
                "clock_regression uptime_ms={} last_uptime_ms={} clock_regressions={}",
                raw,
                self.last_uptime_ms,
                self.clock_regressions
            );
            return self.last_uptime_ms;
        }

        self.last_uptime_ms = raw;
        raw
    }
}

/// Heartbeat loop。**返らない。**
///
/// **watchdog の進行を妨げない。**周期ごとに `std::thread::sleep` で main task を明示的に
/// 譲るため、idle task が走り Task Watchdog Timer が餌を得る。
/// **ISR ではないため、この loop に ISR の禁止事項（allocation、blocking、JSON、長い I/O）は
/// 当たらない。**`sdkconfig.defaults` の watchdog 設定は触っていない（既定値のまま）。
fn run_heartbeat(reason: esp_idf_svc::hal::reset::ResetReason) -> ! {
    let mut hb = Heartbeat::new();
    let reason_str = reset_reason_str(reason);

    log::info!(
        "heartbeat_start period_ms={} board={}",
        HEARTBEAT_PERIOD.as_millis(),
        BOARD
    );

    loop {
        // 先に譲る。起動直後に1件出してから待つのではなく、周期を必ず1回挟む。
        std::thread::sleep(HEARTBEAT_PERIOD);

        hb.seq = hb.seq.saturating_add(1);
        let uptime_ms = hb.uptime_ms();

        // reset reason を毎回添える。**再取得はしない。**起動時に取った値を保持して出す。
        // 取得可否の回帰は起動時の log で見る。
        //
        // **`ProtocolCounters` の 13 field のうち 3 つだけを出している。**全部出すと
        // 5 秒ごとに 13 個の 0 が並び、serial monitor が読めなくなる。
        // **選んだ 3 つに意味の優劣は無い。**protocol session を確立していない現時点では
        // 全 field が 0 であり、**どれを出しても同じである。**session を張った後に
        // 何を出すかは、status message を実装する作業で決める。**ここで決めない。**
        log::info!(
            "heartbeat seq={} uptime_ms={} reset_reason={} \
             parse_errors={} invalid_payloads={} rate_limited={} \
             clock_regressions={} peripherals=untouched servo=not_driven",
            hb.seq,
            uptime_ms,
            reason_str,
            hb.counters.parse_errors,
            hb.counters.invalid_payloads,
            hb.counters.rate_limited,
            hb.clock_regressions
        );
    }
}
