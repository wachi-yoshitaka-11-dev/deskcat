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
}
