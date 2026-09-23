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
//! - `ACCEL-01`（ADXL345）／`ENV-01`（BME280）のDevice ID／Chip IDを読み、生byteを
//!   logへ出す（[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)／
//!   [#16](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/16)）。
//!   **一致判定はここでは行わない。**生byteをlogへ残すだけで、識別の断定は
//!   log を読む人間の責務とする（[`run_i2c_bringup`]参照）。
//! - **`DISP-01`（LCD）・`SERVO-PWM`・`ADC-*`・`TOUCH-*`は既定のbuildではdriveしない。**
//!   [`crate::display`]と[`crate::servo`]はcross-compile確認用にcompileするだけであり、
//!   `main()`からは呼ばない。`bringup-display-13` feature付きbuildだけが
//!   [`run_display_bringup`]経由でLCDを初期化し、識別・backlight点灯・単色fill・
//!   四隅patternを行う（[Issue #13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)。
//!   **既定offにした理由と有効化の手順は下の「`DISP-01`のbring-upを有効にする手順」節。**）。
//!   `bench-servo-test-17` feature付きbuildだけが[`run_servo_bench_test`]経由でservoを
//!   呼ぶ（[Issue #17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。
//!   詳細は[`crate::servo`]と[`run_servo_bench_test`]のdoc参照）。
//!
//! **上の一覧は既定buildの動作を述べる。**`pi-protocol-mode`はI2Cの
//! bring-upを行わず、`bringup-display-13`とは同時に有効にできない（下記
//! `compile_error!`）。build identity／board ID／reset reasonのlogと、
//! heartbeat／health snapshotのloopは`pi-protocol-mode`でも実行されるが、
//! loggingを止めているため出力は既定buildにしか出ない（`crate::console`
//! 参照）。`pi-protocol-mode`は代わりに`boot` frameの書き込みを1回だけ試みる
//! （`send_boot_frame_once`参照。制約は`crate::console`のmodule docに
//! まとめてある）。
//!
//! 既定buildで`Peripherals::take()`の戻り値から実際にdriverへ渡すのは、I2C関連2本
//! （`crate::accel`・`crate::env`のmodule doc参照）だけである。
//! `bringup-display-13` feature付きbuildはLCD関連6+1本（`crate::display`のmodule doc参照）を、
//! `bench-servo-test-17` feature付きbuildは`SERVO-PWM`（GPIO27）と
//! `peripherals.ledc.timer0`／`channel0`を、それぞれ追加で渡す。`pi-protocol-mode`は
//! `Peripherals::take()`自体を呼ばない（`main()`参照）。
//!
//! **I2Cはこの版でも実機通電していない。**この版の検証は`cargo build`でのcross-compile
//! 確認までであり、実機へflashして確認するのは別工程である（[Hardware Safety
//! Policy](../../../docs/governance/hardware-safety-policy.md)「人間の監視が必要な
//! 操作」。初回配線revisionでの初回通電は人間監視下で行う）。
//!
//! **Protocol sessionは確立しない。**`pi-protocol-mode`は`Boot` frameの書き込みを
//! 1回試みるだけで（[#446](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446)、
//! `send_boot_frame_once`参照）、受理確認・再送・受信loopを持たない。
//! **既定buildは`boot`を一切組み立てない。**`#446`より前は`boot=`というlog行を
//! 出していたが、その行はこの変更で削除した。`boot=`行を目視で確認していた
//! 作業（`#7`／`#13`等）があれば、この変更を踏まえて確認し直すこと。
//!
//! **Watchdog の設定を変えない。**Task Watchdog Timer は ESP-IDF の既定値のままである。
//! `sdkconfig.defaults` に watchdog の項目を足していない。heartbeat loop は
//! [`FreeRtos::delay_ms`] で待つ。同 API は
//! 「Delays bigger than `1000 /` `TICK_RATE_HZ` milliseconds … used in a loop would
//! starve the FreeRTOS IDLE tasks as they are low prio tasks and hence the IDLE task's
//! watchdog could trigger. **This delayer avoids that by yielding to the OS during the
//! delay.**」と doc に明記しており、これが「logging が watchdog の進行を block しない」
//! 根拠である。busy wait をしないため、待ち時間は必ず 1 ms 以上へ丸める
//! （[`sleep_ms_until`] 参照）。`run_display_bringup`（#13の LCD bring-up。
//! `bringup-display-13` feature付きbuildだけが呼ぶ）も同じ
//! [`FreeRtos::delay_ms`] を段階ごとに挟む（`service_bringup_step` 参照）。
//!
//! # `DISP-01`のbring-upを有効にする手順
//!
//! **既定buildでLCDを動かさないのは、恒久的な無効化ではない。**このfeatureが有効にする
//! `run_display_bringup`は、電流制限つき外部3.3 V電源（段階B-2b）を使った
//! `DISP-01`（MSP2807）の初回通電手順に対応する。**この段階B-2bが要求する「設定する
//! 電流制限値の上限を決めるためのmodule側の安全な上限」は引き続き未解決のままである**
//! （[tbd-register.md](../../../docs/hardware/tbd-register.md)の`HW-TBD-024`行）。
//! B-2bの給電構成も確定していない（[power-budget.md](../../../docs/hardware/power-budget.md)の
//! `DISP-01`初回通電の手順（給電構成の確定待ち）が、B-2bを採る場合の実行前提として
//! 2点を挙げている）。**[Issue #451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)より前、
//! `run_display_bringup`は既定buildでも無条件に呼ばれ、その中で`lcd.backlight_on()`を
//! 実行していた。**そのため`DISP-01`が配線されているだけでbacklightへ給電された。
//! **それを止めたのがこのfeatureである。**
//!
//! **`#461`（2026-09-23）が認めたのは、B-2bではなくESP32`3V3` pinからの通常接続である。**
//! `#445`が`ACCEL-01`／`ENV-01`に採った経路（B-2bではなく`3V3` pin）と同じであり、
//! B-2bの前提2点は`3V3` pin経路には掛からない。**ただし`3V3` pin経路での`DISP-01`
//! bring-up手順はまだ書かれていない**（`ACCEL-01`／`ENV-01`には
//! `ACCEL-01`／`ENV-01`単体bring-upの手順（`power-budget.md`）があるが、`DISP-01`向けの
//! 対応物は無い。作成は`#13`が引き受ける）。`#461`は接続そのものを許可するだけで、
//! `run_display_bringup`（識別・backlight点灯・fill・四隅patternを行う）を`3V3` pin
//! 経路で呼んでよいかは決めない。**`#461`の余裕解析はbacklightが点灯した状態を含む
//! （通常動作の合計にbacklightのtypical値とILI9341ロジック50 mAが入っている）。
//! `run_display_bringup`が追加で引く負荷は無い。**`#13`に残るのは`3V3` pin経路の
//! bring-up手順を書くことであり、余裕の再判定ではない
//! （[tbd-register.md](../../../docs/hardware/tbd-register.md)の`HW-TBD-024`行）。**
//! `3V3` pin経路の初回接続も、B-2bと同じく人間の監視下で行い、異音・発熱・変色・異臭を
//! 認めたら直ちに停止する（[Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)
//! 「人間の監視が必要な操作」、[tbd-register.md](../../../docs/hardware/tbd-register.md)の
//! `HW-TBD-024`行が持つ停止基準）。結果は[Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)
//! 「ベンチ試験記録」の形式で記録する。**`#13`が書く手順にも、これらは`#461`を待たずに
//! 掛かっている**（§7は「新しい配線revisionの初回通電」、§10はbench試験記録の形式を、
//! いずれも条件なしで定めている）。
//!
//! **B-2bの前提2つが解けたら、人間が次の手順でこのfeatureを有効にする。**
//!
//! 1. `power-budget.md`の`DISP-01`初回通電の手順（給電構成の確定待ち）が挙げる
//!    B-2bの前提2点を満たす。
//! 2. `--features bringup-display-13`を付けてbuildする。**commandの正本は
//!    [検証済みコマンド](../../../docs/toolchains/verified-commands.md)であり、
//!    ここへ写さない。**
//! 3. ESP32 Flash / HIL profileの端末で人間がflashし、人間の監視下で通電する
//!    （[Machine Profiles](../../../docs/toolchains/machine-profiles.md)、
//!    [Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)
//!    「人間の監視が必要な操作」）。
//!
//! **このfeatureはB-2bのgateを開けない。**開けてよいかの判定は上記の正本文書が
//! 持つ。**このfeatureが変えるのは、既定buildが`DISP-01`へ触れるかどうかだけである**
//! （上のLCD関連6+1本のGPIOを駆動するか、`lcd.backlight_on()`を呼ぶか、`display_*`の
//! logを出すか）。回路側の制約も、`DISP-01`を接続してよいかの判定も、これで変わらない。

// `pi-protocol-mode`ではLCD／I2Cのbring-upとdemo用moduleをcompileしない
// （`crate::console`のmodule doc参照）。
#[cfg(not(feature = "pi-protocol-mode"))]
mod accel;
mod config;
mod console;
#[cfg(not(feature = "pi-protocol-mode"))]
mod display;
#[cfg(not(feature = "pi-protocol-mode"))]
mod env;
mod health;
#[cfg(not(feature = "pi-protocol-mode"))]
mod protocol;
mod servo;

// `pi-protocol-mode`は`Peripherals::take()`を行わないため、`bench-servo-test-17`の
// servo bench試験経路（`run_servo_bench_test`）は呼ばれない。両方を有効にしても
// buildは通るが、servoのfeatureが黙って無効になる。それより、compile時に理由を
// 示して止めるほうがよいと判断した。
#[cfg(all(feature = "pi-protocol-mode", feature = "bench-servo-test-17"))]
compile_error!(
    "pi-protocol-modeとbench-servo-test-17は同時に有効にできない。\
     pi-protocol-modeはPeripherals::take()を行わないためservoのbench試験経路が\
     呼ばれず、featureが黙って無効になる。どちらか一方だけを有効にすること。"
);

// `bringup-display-13`も同じ理由で`pi-protocol-mode`と排他にする。**servoと同じ形を
// 採ったのは、失敗の仕方が同じだからである。**`pi-protocol-mode`は`Peripherals::take()`を
// 呼ばず`crate::display`もcompileしないため、両方を有効にしてもLCDのbring-upは実行され
// ない。「LCDを有効にしたつもりの構成が黙ってLCDを動かさない」状態を作らず、compile時に
// 理由を示して止める（`#451`）。
#[cfg(all(feature = "pi-protocol-mode", feature = "bringup-display-13"))]
compile_error!(
    "pi-protocol-modeとbringup-display-13は同時に有効にできない。\
     pi-protocol-modeはPeripherals::take()を行わずcrate::displayもcompileしないため\
     LCDのbring-up経路が呼ばれず、featureが黙って無効になる。\
     どちらか一方だけを有効にすること。"
);

#[cfg(feature = "bringup-display-13")]
use std::time::Instant;

#[cfg(feature = "pi-protocol-mode")]
use deskcat_protocol::{encode_line, limits, Boot, Envelope, Frame, Message};
#[cfg(not(feature = "pi-protocol-mode"))]
use deskcat_protocol::{Hello, HelloReason};
use esp_idf_svc::hal::delay::FreeRtos;
#[cfg(not(feature = "pi-protocol-mode"))]
use esp_idf_svc::hal::gpio::{InputPin, OutputPin};
#[cfg(not(feature = "pi-protocol-mode"))]
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver, I2C0};
#[cfg(not(feature = "pi-protocol-mode"))]
use esp_idf_svc::hal::peripherals::Peripherals;
#[cfg(feature = "bringup-display-13")]
use esp_idf_svc::hal::spi::SpiAnyPins;
#[cfg(not(feature = "pi-protocol-mode"))]
use esp_idf_svc::hal::units::Hertz;

#[cfg(not(feature = "pi-protocol-mode"))]
use crate::accel::Adxl345;
#[cfg(feature = "bringup-display-13")]
use crate::display::Ili9341;
#[cfg(not(feature = "pi-protocol-mode"))]
use crate::env::Bme280;
use crate::health::Health;
#[cfg(not(feature = "pi-protocol-mode"))]
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
#[cfg(not(feature = "pi-protocol-mode"))]
const ACCEL_I2C_ADDRESS: u8 = 0x53;

/// `ENV-01`（BME280）のI2C address。**`SDO`を`GND`へ配線する前提の値である。**
///
/// `SDO`をGNDへ配線すると`0x76`になる
/// （[`docs/hardware/sensor-datasheet-notes.md`](../../../docs/hardware/sensor-datasheet-notes.md)
/// 211行目）。module資料の既定でもある
/// （[`docs/hardware/gpio-assignment.md`](../../../docs/hardware/gpio-assignment.md)の
/// `I2C addressの選択`節「`0x76`はmodule資料が「既定」と記す側である」行）。
/// [`ACCEL_I2C_ADDRESS`]と同じ根拠（一般値で開始してよい側、`gpio-assignment.md`の
/// `I2C addressの選択`節「addressは一般値で開始してよい側である」行）で、GND側を
/// 前提にした。**現物確認まで確定しない点も`ACCEL_I2C_ADDRESS`と同じである。**
#[cfg(not(feature = "pi-protocol-mode"))]
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
#[cfg(not(feature = "pi-protocol-mode"))]
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

    // `crate::console`のmodule doc参照。
    #[cfg(feature = "pi-protocol-mode")]
    console::silence_logging();
    #[cfg(not(feature = "pi-protocol-mode"))]
    console::init_log_mode();

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

    // `pi-protocol-mode`ではLCD／I2C／servo benchのbring-up（`Peripherals::take()`を
    // 含む）を一切行わない（`crate::console`のmodule doc参照）。`pi-protocol-mode`と
    // `bench-servo-test-17`を同時に有効にした場合も、servo benchは実行されない。
    #[cfg(not(feature = "pi-protocol-mode"))]
    {
        // 実際にdriverへ渡す範囲はmodule doc参照。1度しか成功しないため`expect`で即座に気付く。
        let peripherals = Peripherals::take().expect("Peripherals::take must succeed exactly once");
        // **bring-up経路ごとに1 fieldで出す。**featureが2つになったため、行ごと`#[cfg]`で
        // 分けると組み合わせの数だけ同じ行を書くことになる。`cfg!`はcompile時に定数へ
        // 畳まれるため、有効でない経路の文字列が実行時に選ばれることはない。
        let display_state = if cfg!(feature = "bringup-display-13") {
            "bringup_enabled"
        } else {
            "not_driven"
        };
        let servo_state = if cfg!(feature = "bench-servo-test-17") {
            "bench_test_pending"
        } else {
            "not_driven"
        };
        log::info!(
            "peripherals=taken display={display_state} servo={servo_state} i2c=id_read_attempt adc=not_driven touch=not_driven"
        );
        log::info!(
            "i2c_addresses accel=0x{ACCEL_I2C_ADDRESS:02x} env=0x{ENV_I2C_ADDRESS:02x} baudrate_hz={I2C_BAUDRATE_HZ}"
        );

        // featureが無ければこのblockはbuildへ含まれない（`run_display_bringup`のdoc参照）。
        #[cfg(feature = "bringup-display-13")]
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
    }

    // 周期は `config` が持つ。**ここへ数値を直接書かない。**暫定値である根拠は
    // `config` の doc comment にある。
    log::info!(
        "heartbeat_period_ms={} health_snapshot_period_ms={}",
        config::HEARTBEAT_PERIOD_MS,
        config::HEALTH_SNAPSHOT_PERIOD_MS
    );

    // **`health.uptime_ms()`起点で最初の締切を積む。**`0`起点で固定すると、
    // 既定build（LCD／I2C bring-upを行う）ではbring-upの所要時間（複数のSPI
    // fillで実測1秒前後かかりうる）だけで最初のloop周回が即座に`overrun`と
    // 判定されてしまう。bring-up自体の遅延であってmain loopの遅延ではないため、
    // 混同しない。`pi-protocol-mode`はbring-upを行わないためこの遅延は生じないが、
    // 締切の起点をuptimeにする扱い自体は両buildで共通にしている。
    let bringup_done_ms = health.uptime_ms();
    let mut next_heartbeat = bringup_done_ms + u64::from(config::HEARTBEAT_PERIOD_MS);
    let mut next_snapshot = bringup_done_ms + u64::from(config::HEALTH_SNAPSHOT_PERIOD_MS);

    // `pi-protocol-mode`でだけ`boot`を1回試みる（`#446`。`send_boot_frame_once`参照）。
    #[cfg(feature = "pi-protocol-mode")]
    send_boot_frame_once(&health, reset_reason);
    #[cfg(not(feature = "pi-protocol-mode"))]
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

/// `boot` frameを1回だけ組み立て、`write_line`で直接UART0へ書き込みを試みる
/// （`pi-protocol-mode`でだけ呼ぶ。制約は`crate::console`のmodule doc参照）。
///
/// 受理確認・再送・recovery budget（§4.1）・受信loopを実装していないため
/// sessionは確立しない（`#12`の受け入れ条件は満たさない。残りは`#12`本文が
/// 引き続き追跡する）。
///
/// `sid`には`health.uptime_ms()`を使う。bring-upを行わないため起動ごとに
/// ほぼ同じ小さい値になり、§3が求める再起動間の非衝突を満たさない
/// （`PROTO-TBD-011`待ちの暫定値。`crate::console`の制約参照）。
///
/// `id`は1固定。`reset_reason`は実際の`ResetReason`から得た値である。
#[cfg(feature = "pi-protocol-mode")]
fn send_boot_frame_once(health: &Health, reset_reason: &str) {
    let boot = Boot {
        firmware: env!("CARGO_PKG_VERSION").to_owned(),
        board: config::BOARD.to_owned(),
        reset_reason: reset_reason.to_owned(),
    };
    let ts_ms = health.uptime_ms();
    let frame = Frame::new(
        Envelope {
            v: limits::PROTOCOL_VERSION,
            sid: sid_from_uptime(ts_ms),
            id: 1,
            ts_ms,
        },
        Message::Boot(boot),
    );
    // encode失敗はlog::warn!で分類する（`AGENTS.md`「エラーを握りつぶさず、分類、
    // ログ、カウンタを用意する」に沿った形）。ただし`pi-protocol-mode`では
    // loggingを止めているため、この`log::warn!`自体は出力されない。counterは
    // 持たない（同じ理由で増やしても観測できないため）。
    match encode_line(&frame) {
        Ok(line) => console::write_line(&line),
        Err(err) => {
            log::warn!("boot_encode_failed error={err}");
        }
    }
}

/// `send_boot_frame_once`のdoc参照。`ts_ms`（`u64`）を`sid`（`u32`）へ切り詰める（下位32 bit）。
#[cfg(feature = "pi-protocol-mode")]
const fn sid_from_uptime(ts_ms: u64) -> u32 {
    ts_ms as u32
}

/// `crate::protocol::PiSession`が`hello`／`ping`／`get_status`を仕様どおり処理できることを、
/// 自己完結した例で示す（実serial linkは無いため、入力もこの関数が作る）。
/// **既定buildでだけ呼ぶ**（`pi-protocol-mode`ではlog出力を止めており、この関数の
/// 目的である「log出力を目視できること」が成立しないため）。
///
/// **これはprotocolの成立を主張しない。**`crates/deskcat-serial`の`tests/simulator.rs`が
/// 持つ受け入れ条件のtestとは違い、これはbuildできることと、log出力を目視できることの
/// 実物である。実serial linkの受信loopが入ったら、この呼び出し元をそちらへ置き換える。
#[cfg(not(feature = "pi-protocol-mode"))]
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
#[cfg(not(feature = "pi-protocol-mode"))]
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
///
/// **`bringup-display-13` feature付きbuildだけがこの関数を持つ。**既定buildはこの関数を
/// compileせず、`main()`から呼ばない。したがって既定buildは`LCD-BL`（GPIO4）を含む
/// LCD関連pinへ一切触れず、`lcd.backlight_on()`も実行しない
/// （[#451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)。既定offにした
/// 理由と有効化の手順はmodule docの「`DISP-01`のbring-upを有効にする手順」節が持つ。
/// **ここへ再掲しない。**）。
#[cfg(feature = "bringup-display-13")]
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
/// （`docs/hardware/gpio-assignment.md`の`信号inventory`の`ACCEL-SDA`／`ACCEL-SCL`／
/// `ENV-SDA`／`ENV-SCL`各行）、`I2cDriver`は
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
/// 213行目「実装されているinterface（jumper設定）| TBD」）。BME280側の`J3`（`CSB`→`VDD`）は
/// **はんだ付けされた**（実施日・状態・根拠の水準は
/// [`docs/hardware/sensor-datasheet-notes.md`](../../../docs/hardware/sensor-datasheet-notes.md)の
/// `jumper（AE-BME280）`節が正であり、ここへ再掲しない。実施の記録は同dir の
/// `experiment-log.md`の`EXP-014`）。**ただしこの記述を書いたAIセッションは閉を確認しておらず、
/// 測定値も無い。**
/// **したがってこの関数の読み出しが失敗（`Err`）しても、原因をdriverやbus配線に限らない。**
/// 上に挙げた`CS`の配線要求が満たされていない場合がありうる。`J3`についても、はんだ付け済みという記述は
/// ユーザーの申告であって測定で確かめられておらず、**付いていない場合と、付いていても
/// 品質に問題がある場合の両方が残る。**下記のbus配線側の未完了も併せて見る。
///
/// **bus自体の配線も、まだ完了していない。**`ACCEL-SDA`／`ACCEL-SCL`の外部pull-upは
/// breadboardへ実装済みだが（`docs/hardware/gpio-assignment.md`の`競合check`節の
/// 受け入れchecklist「すべての外部pull-upが3.3Vへ接続され」の項目、2026-09-07）、
/// I2C実効pull-upが有効範囲内であることの確認（`Cb`未測定）と、ESP32電源投入前に
/// 外部moduleがpinを駆動していないことの非通電導通checkは、**moduleがESP32へ配線
/// されるまで検証対象が存在しない**として`#15`／`#16`側へ送られている
/// （同文書冒頭の`#2`のclose条件ではない項目一覧、(3)・(5)）。**したがってこの関数を
/// 実機で動かす前に、これらの現物確認が要る。**
///
/// 生byteの解釈は、この前提込みでlogを読む人間の判断とする。
///
/// # 通信timeout
///
/// **無期限（`esp_idf_svc::hal::delay::BLOCK`）は使わない。**`crate::accel`・`crate::env`が
/// [`crate::config::I2C_TRANSACTION_TIMEOUT_MS`]から換算したtick数を
/// `I2cDriver::write_read`へ渡す（[#451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)。
/// 値と導出は同定数のdocが正本であり、**ここへ再掲しない**）。**これにより、busが低のまま
/// 固着した状態（`SDA`の地絡、jumper未設定など）でもこの関数は`Err`を分類してlogへ出し、
/// `main()`のheartbeat loopへ到達する。**「ハングした」と「センサが応答しない」を、人が
/// logの有無から区別できる。
///
/// **上限が効くのはdriver呼び出し側のtick数である。**ESP-IDF v5.5.3の
/// `i2c_master_cmd_begin`（`components/driver/i2c/i2c.c`）は、`ticks_to_wait`を
/// `xSemaphoreTake`の待ち時間として使い、その後の完了待ちloopでも
/// `elapsed >= ticks_to_wait`で打ち切って`ESP_ERR_TIMEOUT`を返す。`ticks_start`は
/// semaphore取得より前に取るため、**この2区間を合わせた「待ち」の上限が渡したtick数**
/// になる。
///
/// **関数が戻るまでの時間は、その待ちより少し長い。**timeoutを検出した側は
/// `i2c_hw_fsm_reset`を呼び、同関数のbus clear待ちが最大`I2C_CLR_BUS_TIMEOUT_MS`
/// （同版で50 ms）かかりうる（同file）。**有限であることは変わらないが、
/// 「渡したtick数ちょうどで戻る」とは書かない。**
///
/// **`esp_idf_svc::hal::i2c::config::Config`の`timeout`フィールドは設定しない。**同
/// フィールドが書き換えるのはI2C周辺回路側のbus timeoutレジスタ（`i2c_set_timeout`）で
/// あり、ESP32ではこの値をbaudrateから導出して`i2c_param_config`の時点で既に設定して
/// いる（同版`components/hal/esp32/include/hal/i2c_ll.h`の`i2c_ll_cal_bus_clk`。
/// `clk_cal->tout = half_cycle * 20; //default we set the timeout value to 10 bus cycles.`）。
/// **ここへ定数を与えると、baudrateへ連動している既定値を、根拠の無い固定値で置き換える
/// ことになる。**上限の保証は上のtick数側で取るため、その必要が無い。
///
/// **実機で`Err`が返るまでの実測時間は、まだ取っていない。**上の上限はESP-IDF実装を
/// 読んだ結果であって実機観測ではない（この変更の検証は`cargo build`までである）。
#[cfg(not(feature = "pi-protocol-mode"))]
fn run_i2c_bringup(
    i2c0: I2C0<'static>,
    sda: impl InputPin + OutputPin + 'static,
    scl: impl InputPin + OutputPin + 'static,
    health: &mut Health,
) {
    // ESP32内蔵のweak pull-upは有効にしない。`gpio-assignment.md`の実効pull-up計算が
    // 外部pull-upだけを前提にしているため（`crate::env`のmodule doc「bus speedは
    // Standard-mode」節と同じ根拠。**ここへ再掲しない**）。
    // **`timeout`（hardware側のbus timeoutレジスタ）は設定しない。**理由はこの関数の
    // doc comment「通信timeout」節。**ここへ再掲しない。**
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
#[cfg(feature = "bringup-display-13")]
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
#[cfg(feature = "bringup-display-13")]
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
/// （health snapshotをwireへ送るにはsessionが要る。#12。`pi-protocol-mode`が
/// `boot`用に選ぶ`sid`は`send_boot_frame_once`参照。両者は別の判断である）。
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
