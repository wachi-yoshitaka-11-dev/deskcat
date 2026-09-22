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
//! - `ACCEL-01`（ADXL345）／`ENV-01`（BME280）のDevice ID／Chip IDを読み、生byteを
//!   logへ出す（[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)／
//!   [#16](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/16)）。
//!   **一致判定はここでは行わない。**生byteをlogへ残すだけで、識別の断定は
//!   log を読む人間の責務とする（[`run_i2c_bringup`]参照）。
//! - `SERVO-PWM`・`ADC-*`・`TOUCH-*`は既定のbuildではdriveしない。[`crate::servo`]は
//!   cross-compile確認用に加えたのみ。`bench-servo-test-17` feature付きbuildだけが
//!   [`run_servo_bench_test`]経由で呼ぶ（[Issue #17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。
//!   詳細は[`crate::servo`]と[`run_servo_bench_test`]のdoc参照）。
//!
//! 既定buildで`Peripherals::take()`が束縛するのはLCD関連6+1本と、I2C関連2本
//! （`crate::display`・`crate::accel`・`crate::env`のmodule doc参照）だけである。
//! `bench-servo-test-17` feature付きbuildだけは例外で`SERVO-PWM`（GPIO27）と
//! `peripherals.ledc.timer0`／`channel0`も束縛する。
//!
//! **I2Cはこの版でも実機通電していない。**この版の検証は`cargo build`でのcross-compile
//! 確認までであり、実機へflashして確認するのは別工程である（[Hardware Safety
//! Policy](../../../docs/governance/hardware-safety-policy.md)「人間の監視が必要な
//! 操作」。初回配線revisionでの初回通電は人間監視下で行う）。
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
mod servo;

use std::time::Instant;

use deskcat_protocol::{Boot, Hello, HelloReason};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{InputPin, OutputPin};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver, I2C0};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::spi::SpiAnyPins;
use esp_idf_svc::hal::units::Hertz;

use crate::accel::Adxl345;
use crate::display::Ili9341;
use crate::env::Bme280;
use crate::health::Health;
use crate::protocol::PiSession;

/// `ACCEL-01`（ADXL345）のI2C address。**`SDO`を`GND`へ配線する前提の値である。**
///
/// `SDO`／`ALT ADDRESS`をGNDへ配線すると`0x53`になる
/// （[`docs/hardware/sensor-datasheet-notes.md`](../../../docs/hardware/sensor-datasheet-notes.md)
/// 170行目。**ただしどちらになるかはmodule board上の実装で決まり、現物確認まで確定しない**
/// （[`HW-TBD-004`](../../../docs/hardware/tbd-register.md)）。この定数は`SDO`→GND前提の値である）。
/// addressは一般値で開始してよい側であり
/// （[`docs/hardware/gpio-assignment.md`](../../../docs/hardware/gpio-assignment.md)
/// 372行目「addressは一般値で開始してよい側である」（hardware-safety-policy.mdの
/// 対応表に基づく分類）、388行目「上の材料には電気的な優劣が無く、実装コストの差だけ
/// である」）、`SDO`配線を決める側（現物作業）がこの値と異なる配線を選ぶ場合は、この
/// 定数を実際の配線へ合わせて直す。`ENV-01`側も`GND`側を前提にした
/// （[`ENV_I2C_ADDRESS`]参照）。
const ACCEL_I2C_ADDRESS: u8 = 0x53;

/// `ENV-01`（BME280）のI2C address。**`SDO`を`GND`へ配線する前提の値である。**
///
/// `SDO`をGNDへ配線すると`0x76`になる
/// （[`docs/hardware/sensor-datasheet-notes.md`](../../../docs/hardware/sensor-datasheet-notes.md)
/// 211行目）。module資料の既定でもある
/// （[`docs/hardware/gpio-assignment.md`](../../../docs/hardware/gpio-assignment.md)
/// 382行目「`0x76`はmodule資料が「既定」と記す側である」）。[`ACCEL_I2C_ADDRESS`]と
/// 同じ根拠（一般値で開始してよい側、`gpio-assignment.md`372行目）で、GND側を
/// 前提にした。**現物確認まで確定しない点も`ACCEL_I2C_ADDRESS`と同じである。**
const ENV_I2C_ADDRESS: u8 = 0x76;

/// I2C busのbaudrate。Standard-mode（100 kHz）。
///
/// [`docs/hardware/gpio-assignment.md`](../../../docs/hardware/gpio-assignment.md)
/// 329行目「2026-09-06に、初回bring-upで採るmodeをStandard-mode（100 kHz）と決定した」。
/// `初回bring-upのmode決定`節が根拠（実効pull-up 約2.42 kΩは規定`Cb`上限でもStandard-modeの
/// rise time制約を満たす。Fast-modeは成立しない）。**ただし同節が明記するとおり、
/// 「rise timeの制約に余裕がある」ことと「実効抵抗がStandard-modeの規定範囲内にあることの
/// 確認」は別であり、後者はこの変更の時点でも未達のまま残る**（[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)
/// の受け入れchecklist項目。この変更はその項目を閉じない）。
const I2C_BAUDRATE_HZ: u32 = 100_000;

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

    // 束縛範囲はmodule doc参照。1度しか成功しないため`expect`で即座に気付く。
    let peripherals = Peripherals::take().expect("Peripherals::take must succeed exactly once");
    #[cfg(not(feature = "bench-servo-test-17"))]
    log::info!(
        "peripherals=display_and_i2c servo=not_driven i2c=id_read_attempt adc=not_driven touch=not_driven"
    );
    #[cfg(feature = "bench-servo-test-17")]
    log::info!(
        "peripherals=display_and_i2c_and_servo_bench_test servo=bench_test_pending i2c=id_read_attempt adc=not_driven touch=not_driven"
    );
    log::info!(
        "i2c_addresses accel=0x{ACCEL_I2C_ADDRESS:02x} env=0x{ENV_I2C_ADDRESS:02x} baudrate_hz={I2C_BAUDRATE_HZ}"
    );

    run_display_bringup(
        peripherals.spi3,
        peripherals.pins.gpio18,
        peripherals.pins.gpio23,
        peripherals.pins.gpio19,
        peripherals.pins.gpio22,
        peripherals.pins.gpio17,
        peripherals.pins.gpio16,
        peripherals.pins.gpio4,
        &mut health,
    );

    run_i2c_bringup(
        peripherals.i2c0,
        peripherals.pins.gpio25,
        peripherals.pins.gpio26,
        &mut health,
    );

    // featureが無ければこのblockはbuildへ含まれない（`run_servo_bench_test`のdoc参照）。
    #[cfg(feature = "bench-servo-test-17")]
    run_servo_bench_test(
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
        peripherals.pins.gpio27,
    );

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
#[allow(clippy::too_many_arguments)]
fn run_display_bringup<SPI: SpiAnyPins + 'static>(
    spi3: SPI,
    sclk: impl OutputPin + 'static,
    mosi: impl OutputPin + 'static,
    miso: impl InputPin + 'static,
    cs: impl OutputPin + 'static,
    dc: impl OutputPin + 'static,
    rst: impl OutputPin + 'static,
    bl: impl OutputPin + 'static,
    health: &mut Health,
) {
    // pinは`docs/hardware/gpio-assignment.md`の`信号inventory`に従う
    // （`LCD-SCLK`=18, `LCD-MOSI`=23, `LCD-MISO`=19, `LCD-CS`=22, `LCD-DC`=17,
    // `LCD-RST`=16, `LCD-BL`=4）。`TOUCH-CS`(21)はbusを共有するが、このfirmwareは
    // touchへは触れない（`crate::display`のmodule doc参照）。
    // **`Peripherals`全体ではなく個々のfieldを受け取る。**`main()`がI2C用の
    // fieldも同じ`Peripherals`から取り出す必要があるため（`run_i2c_bringup`参照）、
    // 呼び出し元でfieldを分けてから渡す。
    let mut lcd = match Ili9341::new(spi3, sclk, mosi, miso, cs, dc, rst, bl) {
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

/// `ACCEL-01`（ADXL345）と`ENV-01`（BME280）のDevice ID／Chip IDを読み、生byteをlogへ出す。
///
/// [Issue #15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)／
/// [Issue #16](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/16)の
/// bring-up手順の1工程である。**一致判定はここでは行わない。**`crate::accel::Adxl345`・
/// `crate::env::Bme280`が返す生byteをそのままlogへ出すだけであり、期待値
/// （`0xE5`／`0x60`）との一致は、logを読む人間の判断とする（`display_id`の
/// `matches_ili9341()`とは異なる扱いである。**この関数へ判定を持ち込まない。**）。
///
/// 2つのsensorは同じI2C bus（`GPIO25`＝SDA、`GPIO26`＝SCL）を共有するため
/// （`docs/hardware/gpio-assignment.md`414・415・417・418行目）、`I2cDriver`は
/// この関数の中で1つだけ作り、両方のdriverへ順に貸す（`crate::accel`・`crate::env`
/// のmodule docが定める設計）。**どの段階で失敗しても、この関数はpanicしない。**
///
/// **両deviceがI2Cモードでbusに応答する前提は、まだ現物で確定していない。**
/// ADXL345は`CS` pinを`VDD I/O`へ配線する必要がある（Analog Devices ADXL345 Data
/// Sheet **Rev. 0**（SparkFunがhostする版、`docs/hardware/sensor-datasheet-notes.md`
/// 134行目が「Revision 4までの出典」と記録する版と同一）「I2C mode is enabled if
/// the CS pin is tied high to VDD I/O... there is no default mode if the CS pin
/// is left unconnected」page 8・10。**この版とRev. G（同文書が正とする版）の既知の
/// 差異一覧（同文書140行目「Rev. 0とRev. Gには差がある」以下のtable）にSerial Communications／I2Cの記載は含まれない
/// が、CLIからRev. Gを取得できないため、Rev. G側でこの記述が同一であることは
/// 独立に確認していない**）。
/// 裏面はんだジャンパ2箇所は開放だが何を選ぶ設定かboard資料が無く不明
/// （[`docs/hardware/sensor-datasheet-notes.md`](../../../docs/hardware/sensor-datasheet-notes.md)
/// 182行目「実装されているinterface（jumper設定）| TBD」）。BME280側は`J3`のはんだ付けが
/// 要る（`docs/hardware/gpio-assignment.md`417行目）。**したがってこの関数の読み出しが
/// 失敗（`Err`）しても、driverやbus配線ではなく、これらの未配線が原因でありうる。**
///
/// **bus自体の配線も、まだ完了していない。**`ACCEL-SDA`／`ACCEL-SCL`の外部pull-upは
/// breadboardへ実装済みだが（`docs/hardware/gpio-assignment.md`710行目、2026-09-07）、
/// I2C実効pull-upが有効範囲内であることの確認（`Cb`未測定）と、ESP32電源投入前に
/// 外部moduleがpinを駆動していないことの非通電導通checkは、**moduleがESP32へ配線
/// されるまで検証対象が存在しない**として`#15`／`#16`側へ送られている
/// （同文書冒頭の`#2`のclose条件ではない項目一覧、(3)・(5)）。**したがってこの関数を
/// 実機で動かす前に、これらの現物確認が要る。**
///
/// **通信timeoutには`esp_idf_svc::hal::delay::BLOCK`（無期限）を使う**（`crate::accel`・
/// `crate::env`と同じ値。一次資料に無い値を推測しない）。**busが低のまま固着する
/// 状態（jumper未設定など）で、この呼び出しがどのくらいの時間で`Err`を返すか、
/// あるいは返さないままになりうるかは、このPRでは検証していない。**esp-idf-hal・
/// esp-idfのI2C driver実装には複数の内部timeout機構（`i2c_master_cmd_begin`の
/// alive-check polling、I2Cハードウェアのbus timeoutレジスタ）があるが、
/// `esp_idf_svc::hal::i2c::config::Config`の`timeout`フィールドを設定していない
/// 場合の既定挙動と、実際に`Err(ESP_ERR_TIMEOUT)`が返るまでの時間は未確認である。
/// **したがってこの呼び出しが返る時間の上限は、このPRの時点で未確定として残す。**
///
/// 生byteの解釈は、この前提込みでlogを読む人間の判断とする。
fn run_i2c_bringup(
    i2c0: I2C0<'static>,
    sda: impl InputPin + OutputPin + 'static,
    scl: impl InputPin + OutputPin + 'static,
    health: &mut Health,
) {
    // ESP32内蔵のweak pull-upは有効にしない。`gpio-assignment.md`の実効pull-up計算が
    // 外部pull-upだけを前提にしているため（`crate::env`のmodule doc「bus speedは
    // Standard-mode」節と同じ根拠。**ここへ再掲しない**）。
    let config = I2cConfig::new()
        .baudrate(Hertz(I2C_BAUDRATE_HZ))
        .sda_enable_pullup(false)
        .scl_enable_pullup(false);

    let mut i2c = match I2cDriver::new(i2c0, sda, scl, &config) {
        Ok(i2c) => i2c,
        Err(err) => {
            log::error!("i2c_driver_new_failed error={err}");
            return;
        }
    };

    let accel = Adxl345::new(ACCEL_I2C_ADDRESS);
    match accel.read_device_id(&mut i2c) {
        Ok(raw) => log::info!("accel_device_id raw=0x{raw:02x}"),
        Err(err) => log::error!("accel_device_id_read_failed error={err}"),
    }
    service_bringup_step(health, "accel_device_id");

    let env = Bme280::new(ENV_I2C_ADDRESS);
    match env.read_chip_id(&mut i2c) {
        Ok(raw) => log::info!("env_chip_id raw=0x{raw:02x}"),
        Err(err) => log::error!("env_chip_id_read_failed error={err}"),
    }
    service_bringup_step(health, "env_chip_id");
}

/// `SERVO-01`（SG90）の単発bench試験（[Issue #17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)）。
/// `bench-servo-test-17` feature付きbuildだけがこの関数を呼ぶ。承認の状態は
/// [servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
/// `承認の状態`節が正本（ここへ再掲しない）。
///
/// # NVSによる単発latch
///
/// この関数はESP32起動のたびに呼ばれ、手動triggerは無い。brownout resetでの
/// 意図しない再実行を防ぐため、**実際にPWMで動かす前に**NVSへ実行済みflagをcommitする
/// （`EspNvs::set_u8`。`unsafe`は増やさない）。commit済みなら次回起動時にskipする。
/// flagのcommit自体に失敗した場合はservoを動かさずに抜ける（誤判定を避けるため
/// 動かさない側へ倒す）。再武装（NVS消去して再実行）には改めて承認が要る
/// （[servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
/// `再武装`step参照）。
///
/// `config::SERVO_BENCH_TEST_ARM_DELAY_MS`の間、heartbeatは出ない。人間は
/// `servo_bench_test_arm_delay_start`等のlogで状況を判断する
/// （検討経緯・前提の検証状況はPR本文参照）。
///
/// 中央（`config::SERVO_ANGLE_CONVENTION_NEUTRAL_DEG`）から
/// `config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG`だけ偏った角度へ1回動かし、
/// `config::SERVO_BENCH_TEST_EXPOSURE_MS`を挟んでdutyを0へ戻す。
#[cfg(feature = "bench-servo-test-17")]
fn run_servo_bench_test(
    timer0: esp_idf_svc::hal::ledc::TIMER0<'static>,
    channel0: esp_idf_svc::hal::ledc::CHANNEL0<'static>,
    pin: impl OutputPin + 'static,
) {
    use crate::servo::Sg90;
    use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};

    const NVS_NAMESPACE: &str = "bench17";
    const NVS_KEY_RAN: &str = "ran";

    let nvs_partition = match EspDefaultNvsPartition::take() {
        Ok(partition) => partition,
        Err(err) => {
            log::error!("servo_bench_test_nvs_partition_failed error={err}");
            return;
        }
    };
    let nvs = match EspNvs::new(nvs_partition, NVS_NAMESPACE, true) {
        Ok(nvs) => nvs,
        Err(err) => {
            log::error!("servo_bench_test_nvs_open_failed error={err}");
            return;
        }
    };

    match nvs.get_u8(NVS_KEY_RAN) {
        Ok(Some(1)) => {
            log::warn!(
                "servo_bench_test_skipped_already_ran reason=nvs_latch_set. \
                 このESP32は既に本試験を1回実行済みである（reset後の再実行を防ぐ）。\
                 改めて承認を得てNVSを消去するまで再実行しない"
            );
            return;
        }
        Ok(_) => {}
        Err(err) => {
            log::error!("servo_bench_test_nvs_get_failed error={err}");
            return;
        }
    }

    // **実際に動かす前にlatchをcommitする。**動作中・動作直後のbrownout resetでも
    // 次回起動でこのflagが読め、再実行しない（module doc「NVSによる単発latch」参照）。
    if let Err(err) = nvs.set_u8(NVS_KEY_RAN, 1) {
        log::error!("servo_bench_test_nvs_set_failed error={err}");
        return;
    }
    log::info!("servo_bench_test_nvs_latch_set");

    log::info!(
        "servo_bench_test_arm_delay_start ms={}",
        config::SERVO_BENCH_TEST_ARM_DELAY_MS
    );
    FreeRtos::delay_ms(config::SERVO_BENCH_TEST_ARM_DELAY_MS);
    log::info!("servo_bench_test_arm_delay_done");

    let mut servo = match Sg90::new(timer0, channel0, pin) {
        Ok(servo) => servo,
        Err(err) => {
            log::error!("servo_bench_test_new_failed error={err}");
            return;
        }
    };

    let target_deg =
        config::SERVO_ANGLE_CONVENTION_NEUTRAL_DEG + config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG;
    match servo.move_to_angle_once(target_deg) {
        Ok(()) => log::info!("servo_bench_test_move target_deg={target_deg}"),
        Err(err) => log::error!("servo_bench_test_move_failed error={err}"),
    }

    // 露出時間の技術的な最小化。「この秒数まで安全」の主張ではない（定数doc参照）。
    FreeRtos::delay_ms(config::SERVO_BENCH_TEST_EXPOSURE_MS);

    match servo.stop() {
        Ok(()) => log::info!("servo_bench_test_stop"),
        Err(err) => log::error!("servo_bench_test_stop_failed error={err}"),
    }
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
