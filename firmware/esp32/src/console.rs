//! UART0を、debug logとPi–ESP32 protocol streamのどちらに使うかを切り替える。
//!
//! `pi-protocol-mode` feature（既定OFF）でbuildを分ける（[#446]）。
//!
//! - 既定: `init_log_mode`が`esp_idf_svc::log::EspLogger`を初期化する。
//! - `pi-protocol-mode`: `silence_logging`がlogを止め、`write_line`がframeを
//!   直接書く。LCD／I2Cのbring-upは行わない（`main.rs`参照）。
//!
//! **`pi-protocol-mode`を有効にしたbuildを、実際のPi hostへ接続しないこと**
//! （`sid`が§3の再起動間の非衝突を満たさない。`main.rs`の
//! `send_boot_frame_once`参照）。
//!
//! §2との関係、既知の逸脱、行長・line endingの扱いは
//! `docs/protocol/esp32-pi-protocol.md`§2が正本として持つ。
//!
//! [#446]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446

#[cfg(feature = "pi-protocol-mode")]
use std::io::Write;

/// debug logモード（既定）を初期化する。
#[cfg(not(feature = "pi-protocol-mode"))]
pub fn init_log_mode() {
    esp_idf_svc::log::EspLogger::initialize_default();
}

/// Rust `log` facadeとESP-IDFの`esp_log`を止める（module doc参照）。
#[cfg(feature = "pi-protocol-mode")]
pub fn silence_logging() {
    log::set_max_level(log::LevelFilter::Off);
    esp_idf_svc::log::EspIdfLogFilter::new()
        .set_target_level("*", log::LevelFilter::Off)
        .expect("\"*\"はinterior NULを含まないため、to_cstring_argは失敗しない");
}

/// 1行（末尾の改行は呼び出し側が含める）をstdout経由でUART0へ書く。行末が
/// §2の規定と異なる点は`docs/protocol/esp32-pi-protocol.md`§2参照。
#[cfg(feature = "pi-protocol-mode")]
pub fn write_line(line: &str) {
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(line.as_bytes());
    let _ = stdout.flush();
}
