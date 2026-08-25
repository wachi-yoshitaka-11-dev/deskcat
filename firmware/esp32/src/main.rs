//! DeskCat ESP32 の最小firmware。
//!
//! [Issue #6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6) と
//! [Issue #7](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/7) の受け入れ条件に
//! 対応し、**未検証のperipheralを初期化しない。**
//!
//! - firmware build identity を出す
//! - board-configuration ID を出す
//! - reset reason を出す
//! - **rate limit 付きの heartbeat と health snapshot を出し続ける**（#7）
//! - **servoも未知のoutputもdriveしない**
//!
//! **`Peripherals::take()` を呼ばない。**これがGPIOを一切駆動しない根拠である。
//! GPIO割り当ては `docs/hardware/gpio-assignment.md` が導通checkなどを待って
//! `Blocked` であり、この firmware は pin へ触れない。
//!
//! **Protocol sessionは確立しない。**`crates/deskcat-protocol` の `Boot` message を
//! 送るのは別の作業であり、ここでは log へ出すだけである。ただし `reset_reason` の
//! 文字列は同 crate の fixture が使う snake_case へ揃えてある。health snapshot も
//! 同 crate の `Status` を組み立てて log へ出すだけである。
//!
//! **「log へ出す」は「serial へ出ない」ではない。**ESP logger の出力は UART を通って
//! serial monitor に現れる。送らないのは、protocol の message として
//! application の serial link へ流すことである（serial device は #11、
//! session state は #12）。
//!
//! **Watchdog の設定を変えない。**Task Watchdog Timer は ESP-IDF の既定値のままである。
//! `sdkconfig.defaults` に watchdog の項目を足していない。heartbeat loop は
//! [`FreeRtos::delay_ms`] で待つ。同 API は
//! 「Delays bigger than `1000 /` `TICK_RATE_HZ` milliseconds … used in a loop would
//! starve the FreeRTOS IDLE tasks as they are low prio tasks and hence the IDLE task's
//! watchdog could trigger. **This delayer avoids that by yielding to the OS during the
//! delay.**」と doc に明記しており、これが「logging が watchdog の進行を block しない」
//! 根拠である。busy wait をしないため、待ち時間は必ず 1 ms 以上へ丸める
//! （[`sleep_ms_until`] 参照）。

mod config;
mod health;

use esp_idf_svc::hal::delay::FreeRtos;

use crate::health::Health;

/// `ResetReason` を Protocol の語彙（snake_case）へ写す。
///
/// **列挙値の意味を推測で足さない。**`esp_idf_svc::hal::reset::ResetReason` は
/// `#[non_exhaustive]` ではないため、variant が増えたときは compile error で気付く。
/// **`_ =>` で受けない。**受けると、この性質が失われる。
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

/// 次の期限を返す。**これが rate limit の実装である。**
///
/// 通常は `deadline + period` を返す。log の所要時間で周期が漸進的にずれないよう、
/// 経過時間ではなく期限を基準に積む。
///
/// `deadline + period` が既に `now` を過ぎている場合（出力が 1 周期以上かかった場合）は、
/// **遅れた分を取り戻そうとしない。**`now + period` へ整列し直し、`overrun` へ
/// `true` を返す。
///
/// **`now` には出力が終わった後の時刻を渡す。**出力の前の時刻を渡すと、出力自体が
/// 1 周期以上かかっても `deadline + period` が未来に見えるため、overrun を検出できない。
///
/// # 保証の範囲
///
/// 保証するのは **slot ごとに 1 回**（burst 1）である。slot は `period` の倍数で区切る。
/// 長期の出力 rate は `1 / period` を超えない。**ただし、ある回が 1 周期未満だけ遅れた
/// 場合、その次の回は前回の出力から 1 周期未満で来うる**（schedule へ整列し直すため）。
/// 「連続する 2 回の間隔が必ず `period` 以上」までは保証しない。そこまで保証するには
/// 出力時刻を基準に積むことになり、毎回の出力費用が周期へ積み上がって drift する。
fn next_deadline(deadline: u64, period_ms: u32, now: u64) -> (u64, bool) {
    let period = u64::from(period_ms);
    let next = deadline.saturating_add(period);
    if next > now {
        (next, false)
    } else {
        (now.saturating_add(period), true)
    }
}

/// `until` まで待つ。
///
/// **必ず 1 ms 以上待つ。**`delay_ms(0)` は yield せずに戻るため、loop に置くと
/// busy wait になり、優先度の低い IDLE task を starve させる。IDLE task が回らないと
/// Task Watchdog Timer が進まないため、これは watchdog の前提を壊す。
///
/// 待ち時間が `u32` に収まらない場合は `u32::MAX` で頭打ちにする。頭打ちにしても
/// 次の周回で残りを待ち直すだけであり、期限を飛ばさない。
fn sleep_ms_until(until: u64, now: u64) {
    let remaining = until.saturating_sub(now);
    let ms = u32::try_from(remaining).unwrap_or(u32::MAX).max(1);
    FreeRtos::delay_ms(ms);
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

    log::info!("board={}", config::BOARD);

    // `ResetReason::get()` は safe fn である。`unsafe` は esp-idf-hal 内部にあり、
    // `unsafe_code` は crate 単位の lint なのでこの crate には効かない。
    let reason = esp_idf_svc::hal::reset::ResetReason::get();
    let reset_reason = reset_reason_str(reason);
    log::info!("reset_reason={reset_reason} raw={reason:?}");

    // **peripheralを取らない。**GPIOもservoも触らない。
    log::info!("peripherals=untouched servo=not_driven");

    // 周期は `config` が持つ。**ここへ数値を直接書かない。**暫定値である根拠は
    // `config` の doc comment にある。
    log::info!(
        "heartbeat_period_ms={} health_snapshot_period_ms={}",
        config::HEARTBEAT_PERIOD_MS,
        config::HEALTH_SNAPSHOT_PERIOD_MS
    );

    let mut health = Health::new(reset_reason);
    let mut next_heartbeat = u64::from(config::HEARTBEAT_PERIOD_MS);
    let mut next_snapshot = u64::from(config::HEALTH_SNAPSHOT_PERIOD_MS);

    // **`main()` から戻らない。**#6 の firmware は戻っていたため、task が進み続けて
    // いるかを外から確認できなかった。
    loop {
        let now = health.uptime_ms();

        if now >= next_heartbeat {
            let seq = health.next_heartbeat_seq();
            log::info!("hb seq={seq} uptime_ms={now}");
            // **出力の後の時刻で積む。**出力前の時刻では、出力自体が 1 周期以上
            // かかっても overrun を検出できない。
            let after = health.uptime_ms();
            let (next, overrun) = next_deadline(next_heartbeat, config::HEARTBEAT_PERIOD_MS, after);
            next_heartbeat = next;
            if overrun {
                health.record_overrun();
                log::warn!(
                    "heartbeat_overrun uptime_ms={now} period_ms={} overrun_ticks={}",
                    config::HEARTBEAT_PERIOD_MS,
                    health.overrun_ticks()
                );
            }
        }

        if now >= next_snapshot {
            emit_health_snapshot(&mut health, now);
            let after = health.uptime_ms();
            let (next, overrun) =
                next_deadline(next_snapshot, config::HEALTH_SNAPSHOT_PERIOD_MS, after);
            next_snapshot = next;
            if overrun {
                health.record_overrun();
                log::warn!(
                    "health_snapshot_overrun uptime_ms={now} period_ms={} overrun_ticks={}",
                    config::HEALTH_SNAPSHOT_PERIOD_MS,
                    health.overrun_ticks()
                );
            }
        }

        // 期限を積み直した後の時刻で残りを測る。log の所要時間を待ち時間から差し引く。
        let until = next_heartbeat.min(next_snapshot);
        sleep_ms_until(until, health.uptime_ms());
    }
}

/// Health snapshot を 1 行の JSON として log へ出す。
///
/// `crates/deskcat-protocol` の `Status` をそのまま serialize する。
/// **これが「counter schema を protocol status へ使用できる」ことの実物である。**
/// 出力するのは `status` の payload であり、envelope を付けた wire line ではない
/// （`sid` を選ぶには session が要る。#12）。
///
/// **error を握りつぶさない。**serialize は事実上失敗しないが、`expect()` で潰さず
/// 分類して log し、counter を進める。
fn emit_health_snapshot(health: &mut Health, now: u64) {
    let status = health.to_status();
    match serde_json::to_string(&status) {
        Ok(payload) => log::info!(
            "health uptime_ms={now} overrun_ticks={} snapshot_errors={} status={payload}",
            health.overrun_ticks(),
            health.snapshot_errors(),
        ),
        Err(err) => {
            health.record_snapshot_error();
            log::warn!(
                "health_snapshot_serialize_failed uptime_ms={now} snapshot_errors={} error={err}",
                health.snapshot_errors()
            );
        }
    }
}
