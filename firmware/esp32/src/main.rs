//! DeskCat ESP32 の最小firmware。
//!
//! [Issue #6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)、
//! [Issue #7](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/7)、
//! [Issue #13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)
//! の受け入れ条件に対応する。
//!
//! - firmware build identity を出す
//! - board-configuration ID を出す
//! - reset reason を出す
//! - **rate limit 付きの heartbeat と health snapshot を出し続ける**（#7）
//! - `DISP-01`（LCD）を初期化し、識別・単色fill・四隅patternを描画する（#13）
//! - **servoも、それ以外の未検証GPIOもdriveしない**
//!
//! **`Peripherals::take()` を呼ぶのはLCD関連の6+1本（`crate::display`のmodule doc参照）
//! に限る。**`docs/hardware/gpio-assignment.md`の`信号inventory`のうち、
//! `SERVO-PWM`・`ACCEL-*`・`ENV-*`・`ADC-*`・`TOUCH-*`はこの版でも一切触れない
//! （同文書の`Blocked`状態は、servo出力gateなどLCD以外の項目が理由であり解除していない。
//! [#13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)本文が
//! `HW-TBD-023`のcloseを待たずに着手してよいとした根拠は、この範囲（LCDのみ）を前提にする）。
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
//! （[`sleep_ms_until`] 参照）。`run_display_bringup`（#13の LCD bring-up）も同じ
//! [`FreeRtos::delay_ms`] を段階ごとに挟む（[`service_bringup_step`] 参照）。

mod accel;
mod config;
mod display;
mod env;
mod health;
mod protocol;

use std::time::Instant;

use deskcat_protocol::{Boot, Hello, HelloReason};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::peripherals::Peripherals;

use crate::display::Ili9341;
use crate::health::Health;
use crate::protocol::PiSession;

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

    // **`Health`はdisplay bring-upより前に作る。**bring-up中もheartbeatを刻めるように
    // するためであり（下記`run_display_bringup`参照）、heartbeatのdeadline計算
    // （`next_heartbeat`／`next_snapshot`）は従来どおりbring-upの後で初期化する。
    let mut health = Health::new(reset_reason);

    // **`Peripherals::take()`はLCD配線の分だけ。**servoやI2C sensorは触らない
    // （module doc参照）。1度しか成功しないため`expect`で即座に気付く。
    let peripherals = Peripherals::take().expect("Peripherals::take must succeed exactly once");
    log::info!(
        "peripherals=display_only servo=not_driven i2c=not_driven adc=not_driven touch=not_driven"
    );

    run_display_bringup(peripherals, &mut health);

    // 周期は `config` が持つ。**ここへ数値を直接書かない。**暫定値である根拠は
    // `config` の doc comment にある。
    log::info!(
        "heartbeat_period_ms={} health_snapshot_period_ms={}",
        config::HEARTBEAT_PERIOD_MS,
        config::HEALTH_SNAPSHOT_PERIOD_MS
    );

    // **`health.uptime_ms()`起点で最初の締切を積む。**`0`起点で固定すると、
    // bring-upの所要時間（複数のSPI fillで実測1秒前後かかりうる）だけで最初の
    // loop周回が即座に`overrun`と判定されてしまう。bring-up自体の遅延であって
    // main loopの遅延ではないため、混同しない。
    let bringup_done_ms = health.uptime_ms();
    let mut next_heartbeat = bringup_done_ms + u64::from(config::HEARTBEAT_PERIOD_MS);
    let mut next_snapshot = bringup_done_ms + u64::from(config::HEALTH_SNAPSHOT_PERIOD_MS);

    // **`boot`はまだwireへ送らない。**GPIO割り当ての承認待ちではない
    // （`docs/hardware/gpio-assignment.md`の`Pi–ESP32間のtransport`節はUSB serialへ
    // 確定済みで、GPIO headerへの配線は無い）。止めているのは、そのUSB link上の
    // UART0が同文書のpin表で`firmware flashingとdebug log専用`と定められており、
    // ESP loggerの出力と、これから送るprotocolのJSON Lines streamが同じUART0を
    // 奪い合う点が未決なことである（`crate::protocol`のmodule doc参照）。ここで示すのは、
    // `crates/deskcat-protocol`の`Boot`をこのfirmwareが正しく組み立てられることと、
    // `crate::protocol::PiSession`が`hello`／`ping`／`get_status`を仕様どおり処理できる
    // ことであり、`health.rs`が`status`について既に示しているのと同じ範囲の主張である。
    log_boot_message(&mut health, reset_reason);
    demonstrate_pi_session(&mut health);

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

/// 起動時に送るはずの`boot`を組み立て、1行のJSONとしてlogへ出す（wireへは送らない）。
///
/// `firmware`／`board`は§4.1が要求するfieldであり、`reset_reason`は実際の
/// `ResetReason`から得た値である。**捏造しない。**envelopeを付けないのは、`sid`を
/// 選ぶ根拠（`PROTO-TBD-011`）も、選んだ`sid`を運ぶ相手も、まだ無いためである。
fn log_boot_message(health: &mut Health, reset_reason: &str) {
    let boot = Boot {
        firmware: env!("CARGO_PKG_VERSION").to_owned(),
        board: config::BOARD.to_owned(),
        reset_reason: reset_reason.to_owned(),
    };
    match serde_json::to_string(&boot) {
        Ok(payload) => log::info!("boot={payload}"),
        Err(err) => {
            health.record_boot_serialize_error();
            log::warn!(
                "boot_serialize_failed error={err} boot_serialize_errors={}",
                health.boot_serialize_errors()
            );
        }
    }
}

/// `crate::protocol::PiSession`が`hello`／`ping`／`get_status`を仕様どおり処理できることを、
/// 自己完結した例で示す（実serial linkは無いため、入力もこの関数が作る）。
///
/// **これはprotocolの成立を主張しない。**`crates/deskcat-serial`の`tests/simulator.rs`が
/// 持つ受け入れ条件のtestとは違い、これはbuildできることと、log出力を目視できることの
/// 実物である。UART0のlogとprotocol streamの分離が決まり実serial linkが入ったら、
/// この呼び出し元を受信loopへ置き換える。
fn demonstrate_pi_session(health: &mut Health) {
    let mut session = PiSession::new();
    // 例として使うだけのPi `sid`／`id`である。実際の値は相手が選ぶ。
    let pi_sid = 90_312;
    let hello = Hello {
        host: "deskcatd".to_owned(),
        version: "0.1.0".to_owned(),
        reason: HelloReason::Startup,
    };
    log::info!("protocol_demo pi_sid_before={:?}", session.pi_sid());
    let established = session.handle_hello(pi_sid, 1, &hello);
    log::info!(
        "protocol_demo hello_outcome={:?} pi_sid_after={:?}",
        established.outcome,
        session.pi_sid()
    );

    let ping_reply = session.handle_ping(pi_sid, 2);
    log::info!("protocol_demo ping_reply={ping_reply:?}");

    let status = health.to_status();
    let (get_status_ack, get_status_reply) = session.handle_get_status(pi_sid, 3, status);
    log::info!("protocol_demo get_status_ack={get_status_ack:?} status={get_status_reply:?}");
}

/// bring-up中の各段階の境界で1回、heartbeatを刻みOSへyieldする。
///
/// 受け入れ条件「更新中も通信とwatchdogがactiveである」に対応する。CodeRabbitの
/// review（[#415](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/415)）が、
/// `run_display_bringup`が`Health::new`とmain loopの開始より前に複数のSPI転送
/// （単色fill×5、四隅pattern）を連続実行しており、その間heartbeatが一度も
/// 出ないことを指摘した。`main`のloopが使う`next_heartbeat`によるdeadline
/// schedulingとは別の、bring-up専用の簡易版である。[`FreeRtos::delay_ms`]は
/// module doc冒頭が引用するとおりOSへyieldするため、SPI転送が連続する区間でも
/// idle taskのwatchdogに機会を与える。
fn service_bringup_step(health: &mut Health, step: &str) {
    FreeRtos::delay_ms(1);
    // **`bringup_hb`と`hb`は別の名前にする。**main loopの`hb seq=`（`config::HEARTBEAT_PERIOD_MS`
    // 周期の本来のheartbeat）とlog上の接頭辞を分け、読み手が混同しないようにする。
    // seqの連番自体は`Health`の同じcounterを共有するため単調増加のままである
    // （bring-up段階の分だけ、main loop側の最初のheartbeatのseqが0からは始まらない）。
    let seq = health.next_heartbeat_seq();
    log::info!(
        "bringup_hb seq={seq} uptime_ms={} bringup_step={step}",
        health.uptime_ms()
    );
}

/// `DISP-01`を初期化し、識別・単色fill・四隅patternを実行する。
///
/// **どの段階で失敗しても、この関数はpanicしない。**この関数はエラーを
/// `log::error!`へ分類して返すだけで、呼び出し元の`main`を止めない。
/// heartbeatは[`service_bringup_step`]で段階ごとに刻む（同関数のdoc参照）。
fn run_display_bringup(peripherals: Peripherals, health: &mut Health) {
    // pinは`docs/hardware/gpio-assignment.md`の`信号inventory`に従う
    // （`LCD-SCLK`=18, `LCD-MOSI`=23, `LCD-MISO`=19, `LCD-CS`=22, `LCD-DC`=17,
    // `LCD-RST`=16, `LCD-BL`=4）。`TOUCH-CS`(21)はbusを共有するが、このfirmwareは
    // touchへは触れない（`crate::display`のmodule doc参照）。
    let mut lcd = match Ili9341::new(
        peripherals.spi3,
        peripherals.pins.gpio18,
        peripherals.pins.gpio23,
        peripherals.pins.gpio19,
        peripherals.pins.gpio22,
        peripherals.pins.gpio17,
        peripherals.pins.gpio16,
        peripherals.pins.gpio4,
    ) {
        Ok(lcd) => lcd,
        Err(err) => {
            log::error!("display_driver_new_failed error={err}");
            return;
        }
    };

    let id = match lcd.init() {
        Ok(id) => id,
        Err(err) => {
            log::error!("display_init_failed error={err}");
            return;
        }
    };
    service_bringup_step(health, "init");

    // **識別結果を捏造しない。**読めた生byteと判定を両方logへ残す
    // （受け入れ条件「Controller識別情報と初期化の根拠を記録した」）。
    log::info!(
        "display_id raw={:02x?} matches_ili9341={}",
        id.raw,
        id.matches_ili9341()
    );
    if !id.matches_ili9341() {
        log::error!(
            "display_id_mismatch expected_id_hi=0x93 expected_id_lo=0x41 got_hi=0x{:02x} got_lo=0x{:02x}",
            id.raw[2],
            id.raw[3]
        );
    }

    if let Err(err) = lcd.backlight_on() {
        log::error!("display_backlight_on_failed error={err}");
    }

    run_fill_tests(&mut lcd, health);
    run_corner_pattern(&mut lcd, health);
}

/// 単色fillを既知のRGB565値で順に実行し、所要時間を計測してlogへ出す。
///
/// 受け入れ条件「単色fillが正しい」「Color orderが正しい」「更新timingを測定した」に
/// 対応する。**正しいかどうかの判定はこの関数では行わない。**実機のLCDを目視して
/// 判定するのは人間であり（`AGENTS.md`ハードウェア安全、初回通電は人間監視下）、
/// この関数は色と所要時間を機械可読な形でlogへ残すだけである。
fn run_fill_tests(lcd: &mut Ili9341<'_>, health: &mut Health) {
    let fills: [(&str, u16); 5] = [
        ("black", display::color::BLACK),
        ("red", display::color::RED),
        ("green", display::color::GREEN),
        ("blue", display::color::BLUE),
        ("white", display::color::WHITE),
    ];

    for (name, color) in fills {
        let start = Instant::now();
        match lcd.fill_screen(color) {
            Ok(()) => {
                let elapsed_us = start.elapsed().as_micros();
                log::info!("display_fill name={name} color=0x{color:04x} elapsed_us={elapsed_us}");
            }
            Err(err) => {
                log::error!("display_fill_failed name={name} error={err}");
            }
        }
        service_bringup_step(health, "fill");
    }
}

/// 四隅へ異なる色の正方形を描き、orientationとcolor orderを実機で確認できるようにする。
///
/// 受け入れ条件「四隅とorientationが正しい」に対応する。**MADCTLはreset時default
/// （`00h`）のままである**（`crate::display`のmodule doc参照）。この patternを見て
/// 向きと色順が期待どおりでなければ、`docs/hardware/gpio-assignment.md`の`MADCTL`欄と
/// `crate::display`のMADCTL定数を実測結果で更新する必要がある。
fn run_corner_pattern(lcd: &mut Ili9341<'_>, health: &mut Health) {
    if let Err(err) = lcd.fill_screen(display::color::BLACK) {
        log::error!("display_corner_background_failed error={err}");
        return;
    }
    service_bringup_step(health, "corner_background");

    const SQUARE: u16 = 24;
    let corners: [(&str, u16, u16, u16); 4] = [
        ("top_left", 0, 0, display::color::RED),
        (
            "top_right",
            display::WIDTH - SQUARE,
            0,
            display::color::GREEN,
        ),
        (
            "bottom_left",
            0,
            display::HEIGHT - SQUARE,
            display::color::BLUE,
        ),
        (
            "bottom_right",
            display::WIDTH - SQUARE,
            display::HEIGHT - SQUARE,
            display::color::WHITE,
        ),
    ];

    let start = Instant::now();
    for (name, x, y, color) in corners {
        if let Err(err) = lcd.fill_rect(x, y, x + SQUARE - 1, y + SQUARE - 1, color) {
            log::error!("display_corner_failed corner={name} error={err}");
        }
        service_bringup_step(health, "corner");
    }
    let elapsed_us = start.elapsed().as_micros();
    log::info!("display_corner_pattern elapsed_us={elapsed_us}");
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
