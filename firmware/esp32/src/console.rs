//! UART0（board上のUSB-UARTブリッジ）をdebug logに使う。
//!
//! **UART0は書き込みとdebug log専用である。**Pi–ESP32 protocol streamは
//! UART0を使わず、`main.rs`がUART0以外のUARTを`PI-UART-TX`（GPIO13）／
//! `PI-UART-RX`（GPIO14）へ割り当てて使う（[#487]。pinと接続の条件の正は
//! `docs/hardware/gpio-assignment.md`の`Pi–ESP32間のtransport`節）。そのため
//! 既定buildでも`pi-protocol-mode`でも、`init_log_mode`が
//! `esp_idf_svc::log::EspLogger`を初期化し、logはUART0へ出る（ESP-IDFのconsoleがUART0で
//! あることは、debug profileで生成された`sdkconfig`で確かめた。範囲は
//! `docs/protocol/esp32-pi-protocol.md`§2）。
//!
//! **[#487]より前、`pi-protocol-mode`はUART0をprotocol streamへ使い、loggingを
//! 止めていた**（[#446]、`silence_logging`）。Pi linkがUART0から外れたため、
//! loggingを止める理由は無くなり、`silence_logging`は削除した。
//!
//! # `pi-protocol-mode`のbuildを実際のPi hostへ接続する範囲
//!
//! **接続してよいのは、[#446]の受け入れ条件4（実機での`boot`→ACKによるsession確立）の
//! 試験として、`gpio-assignment.md`の`信号線をつないでよい条件`1〜4を守る場合に限る。**
//! 電気的な接続の安全はその条件1〜4が持つ。下の3つは電気の安全の制限ではなく、
//! sessionの確立や維持が失敗しうるというprotocolの既知の制約であり、試験はそれを観測する
//! 側にある。そのため試験としての接続は妨げない。ただし、常時接続する製品の構成として
//! 使える段階ではない。理由は次の3つである。
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
//! 未確認のまま残っている項目である。**溢れを数える経路もlogへ出す経路も無いため、
//! 実機で`boot`→ACKが成立しても、溢れなかったことの根拠にはならない。
//!
//! (3) `pi-protocol-mode`はPi→ESP32方向のrequest（`hello`・`get_status`等）を
//! 一切処理しない（`crate::protocol`の`PiSession`は既定buildだけでcompileされ、
//! `pi-protocol-mode`のbuildには存在しない）。§8はidentityを復元できる要求に
//! 相関ACKを返すよう定めており、**これに反する既知の逸脱である**
//! （`crate::boot_session::BootSession::on_bytes`のdoc参照）。**これは設計上の
//! 性質であり、実機で確認して解消する類のものではない。**同じ理由で、§4.1の表と
//! §5.1の手順5が定める、新しいPi `sid`の`hello`を受けたときの`boot`の再送
//! （[#12]、Revision履歴の`Draft 2 boot resend on new pi session`）も満たさない。
//! これらは`#487`の残りの作業（featureの排他をやめて1つのbuildへまとめ、`hello`の
//! 処理を受信の経路へつなぐ変更）で満たす。
//!
//! §2との関係、既知の逸脱、行長・line endingの扱いは
//! `docs/protocol/esp32-pi-protocol.md`§2が正本として持つ。
//!
//! [#12]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12
//! [#446]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446
//! [#487]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487

/// debug logを初期化する（module doc参照）。
pub fn init_log_mode() {
    esp_idf_svc::log::EspLogger::initialize_default();
}
