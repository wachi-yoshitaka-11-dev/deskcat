//! UART0（board上のUSB-UARTブリッジ）をdebug logに使う。
//!
//! **UART0は書き込みとdebug log専用である。**Pi–ESP32 protocol streamは
//! UART0を使わず、`main.rs`がUART0以外のUARTを`PI-UART-TX`（GPIO13）／
//! `PI-UART-RX`（GPIO14）へ割り当てて使う（[#487]。pinと接続の条件の正は
//! `docs/hardware/gpio-assignment.md`の`Pi–ESP32間のtransport`節）。そのため
//! どのbuildでも、`init_log_mode`が
//! `esp_idf_svc::log::EspLogger`を初期化し、logはUART0へ出る（ESP-IDFのconsoleがUART0で
//! あることは、debug profileで生成された`sdkconfig`で確かめた。範囲は
//! `docs/protocol/esp32-pi-protocol.md`§2）。
//!
//! **[#487]より前、Pi linkを持つbuild（`pi-protocol-mode`。#487で廃止した）はUART0をprotocol streamへ使い、loggingを
//! 止めていた**（[#446]、`silence_logging`）。Pi linkがUART0から外れたため、
//! loggingを止める理由は無くなり、`silence_logging`は削除した。
//!
//! # Pi linkを持つbuildを実際のPi hostへ接続する範囲
//!
//! Pi linkを持つのは、`bench-servo-test-17`以外のすべてのbuild（製品buildである既定buildを含む）
//! である（#487。`main.rs`のmodule doc「buildの構成」）。
//!
//! **接続してよい場面と条件は、`docs/hardware/gpio-assignment.md`の`信号線をつないでよい条件`が
//! 持つ**（とくに条件3（つないでよい試験）と条件5（給電の構成））。**ここへ書き写さない。**
//! 電気的な接続の安全もその条件が持つ。下の3つは電気の安全の制限ではなく、
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
//! 未確認のまま残っている項目である。**#487から、UART driverのevent（ring bufferの満杯、FIFOの
//! 溢れ）を数えてhealth snapshotの行へ出す**（`main.rs`の`drain_uart_events`）。ただし数えた回数は
//! 起きた回数の下限であり（同関数のdoc「数えられない場合」）、**0件でも溢れなかったことの証明には
//! ならない。**実機で`boot`→ACKが成立しても、同じく溢れなかったことの根拠にはならない。
//!
//! (3) Pi→ESP32方向の受信（`crate::pi_link`）は、decodeで拒否した行（未対応のtype、payloadや
//! envelopeの不正）の`(sid, id)`を復元しないため、相関ACKを返さない。§8は
//! identityを復元できる要求に相関ACKを返すよう定めており、**これに反する既知の逸脱である**
//! （正本は`docs/protocol/esp32-pi-protocol.md`§2）。§8.1／§8.2の流量制限も実装していない
//! （`crate::pi_link`のmodule doc「実装していないもの」）。
//!
//! §2との関係、既知の逸脱、行長・line endingの扱いは
//! `docs/protocol/esp32-pi-protocol.md`§2が正本として持つ。
//!
//! [#446]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446
//! [#487]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487

/// debug logを初期化する（module doc参照）。
pub fn init_log_mode() {
    esp_idf_svc::log::EspLogger::initialize_default();
}
