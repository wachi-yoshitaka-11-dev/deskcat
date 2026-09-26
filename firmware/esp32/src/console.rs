//! UART0を、debug logとPi–ESP32 protocol streamのどちらに使うかを切り替える。
//!
//! `pi-protocol-mode` feature（既定OFF）でbuildを分ける（[#446]）。
//!
//! - 既定: `init_log_mode`が`esp_idf_svc::log::EspLogger`を初期化する。UART0は
//!   stdio経由のまま（`UartDriver`は未install。VFS console層の実装詳細は未確認）。
//! - `pi-protocol-mode`: `silence_logging`がRust `log`facadeと`esp_log`を止める。
//!   **UART0のI/Oそのものは`main.rs`の`UartDriver`（`#446` PR B）へ一本化した。**
//!   送信も受信もこのdriver経由であり、stdio（`std::io::stdout()`／`stdin()`）は
//!   使わない。LCD／I2Cのbring-upは行わない（`main.rs`参照）。
//!
//! **`pi-protocol-mode`を有効にしたbuildを、実際のPi hostへ接続しないこと。**
//! 理由は3つある。
//!
//! (1) `boot`の受理確認・再送・`sid`選び直しは実装した（`crate::boot_session`）が、
//! `sid`の生成（`main.rs`の`generate_sid`）は衝突許容確率`0`を主張しない
//! （`generate_sid`のdoc参照）。`stale_session`受信時の`sid`選び直し
//! （`BootSession::reselect_sid`）は衝突からの回復手段だが、選び直しの回数には
//! 上限（`sid_reselect_limit`）があるため、counterが`0`から数え直された後に
//! 連続して選ぶ`sid`が、上限+1回分以上retired session集合に含まれていると
//! 衝突から抜けられないまま終端しうる。**これは設計上の性質であり、実機で
//! 確認して解消する類のものではない**（値を大きくしても上限が無くなる
//! わけではない）。
//!
//! (2) 受信経路がsoftware ring buffer（512 byte、
//! `config::PI_PROTOCOL_UART_RX_BUFFER_BYTES`）を溢れさせずに動くかは、buildでは
//! 示せず実機でしか確認できない（同定数のdoc「容量の根拠」参照）。**こちらは
//! 未確認のまま残っている項目である**（`#446` PR B時点。実機確認の状態はIssue #446
//! の追跡を見る）。
//!
//! (3) `pi-protocol-mode`はPi→ESP32方向のrequest（`hello`・`get_status`等）を
//! 一切処理しない（`crate::protocol`の`PiSession`は既定buildだけでcompileされ、
//! `pi-protocol-mode`のbuildには存在しない）。§8はidentityを復元できる要求に
//! 相関ACKを返すよう定めており、**これに反する既知の逸脱である**
//! （`crate::boot_session::BootSession::on_bytes`のdoc参照）。**これは設計上の
//! 性質であり、実機で確認して解消する類のものではない。**
//!
//! どの理由でも、実際のPi hostへの接続を安全と主張できる段階ではない。
//!
//! §2との関係、既知の逸脱、行長・line endingの扱いは
//! `docs/protocol/esp32-pi-protocol.md`§2が正本として持つ。
//!
//! [#446]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446

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
