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
//!   詳細は[`crate::servo`]と[`run_servo_bench_test`]のdoc参照）。**#474で、このfeature付きbuildは
//!   `compile_error!`でcompileが止まる（下記）。**
//!
//! **上の一覧は既定buildの動作を述べる。**`pi-protocol-mode`はI2Cの
//! bring-upを行わず、`bringup-display-13`とは同時に有効にできない（下記
//! `compile_error!`）。build identity／board ID／reset reasonのlogと、
//! heartbeat／health snapshotのloopは`pi-protocol-mode`でも実行されるが、
//! loggingを止めているため出力は既定buildにしか出ない（`crate::console`
//! 参照）。`pi-protocol-mode`は代わりに`boot`のACK待ち・再送・`sid`選び直しを行う
//! （`crate::boot_session`参照。`#446` PR B。制約は`crate::console`のmodule docに
//! まとめてある）。
//!
//! 既定buildで`Peripherals::take()`の戻り値から実際にdriverへ渡すのは、I2C関連2本
//! （`crate::accel`・`crate::env`のmodule doc参照）だけである。
//! `bringup-display-13` feature付きbuildはLCD関連6+1本（`crate::display`のmodule doc参照）を、
//! `bench-servo-test-17` feature付きbuildは`SERVO-PWM`（GPIO27）と
//! `peripherals.ledc.timer0`／`channel0`を、それぞれ追加で渡す（`bench-servo-test-17`付きbuildは
//! #474でcompileが止まる。下記`compile_error!`）。`pi-protocol-mode`は
//! UART0関連（`peripherals.uart0`、GPIO1＝TX、GPIO3＝RX。`crate::boot_session`が
//! 使う`UartDriver`）だけを渡す（`#446` PR B。それ以前は`Peripherals::take()`自体を
//! 呼ばなかった）。
//!
//! **I2Cはこの版でも実機通電していない。**この版の検証は`cargo build`でのcross-compile
//! 確認までであり、実機へflashして確認するのは別工程である（[Hardware Safety
//! Policy](../../../docs/governance/hardware-safety-policy.md)「人間の監視が必要な
//! 操作」。初回配線revisionでの初回通電は人間監視下で行う）。
//! **2026-09-24追記（#472）: 上の2文は`4486de5`の時点の記述である。**2026-09-22に`35bcc36`の
//! buildで`ACCEL-01`／`ENV-01`へ初回通電し、Device ID読み出しに応答を得た記録が
//! [EXP-015](../../../docs/hardware/experiment-log.md)にある。それより後の変更を含むbuildは、
//! 実機で動かした記録が無い。
//!
//! **Protocol sessionはまだ確立を主張しない。**`pi-protocol-mode`は`boot`のACK待ち・
//! 再送・`stale_session`受信時の`sid`選び直しを実装した
//! （[#446](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446) PR B、
//! `crate::boot_session`参照）が、実機での動作確認（受信経路がring buffer溢れなく
//! 動くか、実際に`boot`→ACKが成立するか）はまだ無い。**既定buildは`boot`を一切
//! 組み立てない。**
//! `#446`より前は`boot=`というlog行を出していたが、その行は`#446`のPR Aで削除した。
//! `boot=`行を目視で確認していた作業（`#7`／`#13`等）があれば、`boot=`行が
//! 無くなったことを踏まえて確認し直すこと。
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
//! `run_display_bringup`は、二つの給電経路のいずれかで使う。**元々は**電流制限つき外部3.3 V
//! 電源（段階B-2b）を使った`DISP-01`（MSP2807）の初回通電手順に対応していた。**この段階B-2bが要求する「設定する
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
//! B-2bの前提2点は`3V3` pin経路には掛からない。**`3V3` pin経路での`DISP-01`
//! bring-up手順は[power-budget.md](../../../docs/hardware/power-budget.md)の
//! `DISP-01`追加接続のbring-upの手順（ESP32`3V3` pin給電、`#461`承認範囲）節が持つ**（`#13`が
//! 作成した。`ACCEL-01`／`ENV-01`が同じ`3V3` railへ既に接続済みの状態
//! （[EXP-015](../../../docs/hardware/experiment-log.md)）へ`DISP-01`を追加する場合を扱い、
//! `ACCEL-01`／`ENV-01`単体bring-upの手順とは別節である）。`#461`は接続そのものを
//! 許可するだけで、`run_display_bringup`（識別・backlight点灯・fill・四隅patternを行う）を
//! `3V3` pin経路で呼んでよいかは決めない。**`#461`の余裕解析はbacklightが点灯した状態を含む
//! （通常動作の合計にbacklightのtypical値とILI9341ロジック50 mAが入っている）。
//! `run_display_bringup`が追加で引く負荷は無い。**したがって上記の新設手順は、この余裕解析を
//! そのまま前提とし（[tbd-register.md](../../../docs/hardware/tbd-register.md)の`HW-TBD-024`行、
//! 2026-09-23追記）、余裕の再計算はしていない。**
//! `3V3` pin経路の初回接続も、B-2bと同じく人間の監視下で行い、異音・発熱・変色・異臭を
//! 認めたら直ちに停止する（[Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)
//! 「人間の監視が必要な操作」、[tbd-register.md](../../../docs/hardware/tbd-register.md)の
//! `HW-TBD-024`行が持つ停止基準）。結果は[Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)
//! 「ベンチ試験記録」の形式で記録する。**`#13`が書く手順にも、これらは`#461`を待たずに
//! 掛かっている**（§7は「新しい配線revisionの初回通電」、§10はbench試験記録の形式を、
//! いずれも条件なしで定めている）。
//!
//! **B-2b経由でこのfeatureを有効にする場合、B-2bの前提2つが解けたら、人間が次の手順で行う。**
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
//! **`3V3` pin経路でこのfeatureを有効にする場合は、上記1〜3の代わりに`power-budget.md`の
//! `DISP-01`追加接続のbring-upの手順（ESP32`3V3` pin給電、`#461`承認範囲）節が持つ実施前提条件と
//! 手順に従う。**同節もこのfeatureを要求する（条件(2)）。
//!
//! **このfeatureはB-2bのgateを開けない。**開けてよいかの判定は上記の正本文書が
//! 持つ。**このfeatureが変えるのは、既定buildが`DISP-01`へ触れるかどうかだけである**
//! （上のLCD関連6+1本のGPIOを駆動するか、`lcd.backlight_on()`を呼ぶか、`display_*`の
//! logを出すか）。回路側の制約も、`DISP-01`を接続してよいかの判定も、これで変わらない。

// `pi-protocol-mode`ではLCD／I2Cのbring-upとdemo用moduleをcompileしない
// （`crate::console`のmodule doc参照）。
#[cfg(not(feature = "pi-protocol-mode"))]
mod accel;
#[cfg(feature = "pi-protocol-mode")]
mod boot_session;
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

// `pi-protocol-mode`はUART0向けに`Peripherals::take()`を呼ぶが（`#446` PR B）、
// `run_servo_bench_test`の呼び出し自体が`#[cfg(not(feature = "pi-protocol-mode"))]`の
// blockの中にあるため、servo bench試験経路（`run_servo_bench_test`）は呼ばれない。
// 両方を有効にしてもbuildは通るが、servoのfeatureが黙って無効になる。それより、
// compile時に理由を示して止めるほうがよいと判断した。**#474以降、`bench-servo-test-17`は
// 単独でもcompileが止まる（下記）。**この排他は、#474の`compile_error!`を外したあとも
// 効くよう残す。
#[cfg(all(feature = "pi-protocol-mode", feature = "bench-servo-test-17"))]
compile_error!(
    "pi-protocol-modeとbench-servo-test-17は同時に有効にできない。\
     run_servo_bench_testの呼び出しはpi-protocol-mode以外のbuildでだけ実行される\
     blockの中にあり、servoのbench試験経路が呼ばれず、featureが黙って無効になる。\
     どちらか一方だけを有効にすること\
     （bench-servo-test-17は#474により単独でもcompileが止まる）。"
);

// `bringup-display-13`も同じ理由で`pi-protocol-mode`と排他にする。**servoと同じ形を
// 採ったのは、失敗の仕方が同じだからである。**`pi-protocol-mode`は`crate::display`を
// compileせず、`run_display_bringup`の呼び出しも`#[cfg(not(feature = "pi-protocol-mode"))]`の
// blockの中にあるため、両方を有効にしてもLCDのbring-upは実行されない。
// 「LCDを有効にしたつもりの構成が黙ってLCDを動かさない」状態を作らず、compile時に
// 理由を示して止める（`#451`）。
#[cfg(all(feature = "pi-protocol-mode", feature = "bringup-display-13"))]
compile_error!(
    "pi-protocol-modeとbringup-display-13は同時に有効にできない。\
     pi-protocol-modeはcrate::displayをcompileせず、run_display_bringupの呼び出しも\
     pi-protocol-mode以外のbuildでだけ実行されるblockの中にあるため、\
     LCDのbring-up経路が呼ばれず、featureが黙って無効になる。\
     どちらか一方だけを有効にすること。"
);

// `bench-servo-test-17`付きbuildは、#474でcompileを止めた（理由と承認の状態は
// `docs/hardware/servo-safety-limits.md`の`承認の状態`節が持つ。ここへ再掲しない）。
// **docへ書くだけでは、featureを付ければbuildでき動いてしまう。**
// codeとfeatureの定義は残す。
#[cfg(feature = "bench-servo-test-17")]
compile_error!(
    "bench-servo-test-17付きbuildは#474でcompileを止めている。\
     理由と承認の状態はdocs/hardware/servo-safety-limits.mdの承認の状態節を見ること。"
);

#[cfg(feature = "bringup-display-13")]
use std::time::Instant;

#[cfg(not(feature = "pi-protocol-mode"))]
use deskcat_protocol::{Hello, HelloReason};
#[cfg(not(feature = "pi-protocol-mode"))]
use esp_idf_svc::hal::delay::FreeRtos;
#[cfg(not(feature = "pi-protocol-mode"))]
use esp_idf_svc::hal::gpio::{InputPin, OutputPin};
#[cfg(not(feature = "pi-protocol-mode"))]
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver, I2C0};
use esp_idf_svc::hal::peripherals::Peripherals;
#[cfg(feature = "bringup-display-13")]
use esp_idf_svc::hal::spi::SpiAnyPins;
#[cfg(feature = "pi-protocol-mode")]
use esp_idf_svc::hal::uart::{config::Config as UartConfig, UartDriver};
use esp_idf_svc::hal::units::Hertz;

#[cfg(not(feature = "pi-protocol-mode"))]
use crate::accel::Adxl345;
#[cfg(feature = "pi-protocol-mode")]
use crate::boot_session::{BootRetryPolicy, BootSession};
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
///
/// `pi-protocol-mode`はこの関数の代わりに`UartDriver::read`のtimeoutで待つ
/// （main loop参照。`#446` PR B）。
#[cfg(not(feature = "pi-protocol-mode"))]
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

    // **両buildで`Peripherals::take()`を呼ぶ**（`#446` PR B）。既定buildはLCD／I2C／
    // servo benchのbring-upへ、`pi-protocol-mode`はUART0（`boot`のACK待ち・再送。
    // `crate::boot_session`参照）へ使う。1度しか成功しないため`expect`で即座に気付く。
    let peripherals = Peripherals::take().expect("Peripherals::take must succeed exactly once");

    // `pi-protocol-mode`ではLCD／I2C／servo benchのbring-upを一切行わない
    // （`crate::console`のmodule doc参照）。`pi-protocol-mode`と`bench-servo-test-17`を
    // 同時に有効にした場合も、servo benchは実行されない（上の`compile_error!`参照）。
    #[cfg(not(feature = "pi-protocol-mode"))]
    {
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

    // `pi-protocol-mode`では、**このfirmwareのcodeが行うUART0のI/Oを**
    // protocol専用の`UartDriver`へ一本化する（`crate::console`のmodule doc参照）。
    // 送信も受信もこのdriver経由に揃える。**ESP-IDFのROM／bootloader起動出力や
    // panic出力は対象外である**（`UartDriver`をinstallする前、またはこのfirmware
    // のcode外で書かれるため。§2の既知の逸脱として別途扱う）。
    // TX=GPIO1、RX=GPIO3はUART0のROM固定pin（`docs/hardware/gpio-assignment.md`の
    // `Pi–ESP32間のtransport`節。GPIO headerへの配線は無く内部USB-UARTブリッジへ
    // 接続する）。baudは既定build側のconsoleと同じ115200
    // （`CONFIG_ESP_CONSOLE_UART_BAUDRATE`）に揃える。ring buffer容量の根拠は
    // `config::PI_PROTOCOL_UART_RX_BUFFER_BYTES`のdoc参照。
    #[cfg(feature = "pi-protocol-mode")]
    let mut uart = {
        let uart_config = UartConfig::default()
            .baudrate(Hertz(config::PI_PROTOCOL_UART_BAUDRATE_HZ))
            .rx_fifo_size(config::PI_PROTOCOL_UART_RX_BUFFER_BYTES)
            .tx_fifo_size(config::PI_PROTOCOL_UART_TX_BUFFER_BYTES);
        UartDriver::new(
            peripherals.uart0,
            peripherals.pins.gpio1,
            peripherals.pins.gpio3,
            Option::<esp_idf_svc::hal::gpio::AnyIOPin>::None,
            Option::<esp_idf_svc::hal::gpio::AnyIOPin>::None,
            &uart_config,
        )
        .expect("UartDriver::new for UART0 must succeed exactly once")
    };

    // `pi-protocol-mode`でだけ`boot`のACK待ち・再送sessionを開始する（`#446` PR B、
    // `crate::boot_session`参照）。`sid`は起動のたびに1回だけ選ぶ（§3）。`stale_session`を
    // 受けたときの選び直しは`BootSession`内部が行うため、ここで選び直さない。
    #[cfg(feature = "pi-protocol-mode")]
    let mut boot_session = BootSession::start(
        BootRetryPolicy::provisional(),
        generate_sid(&health),
        reset_reason,
        &health,
        &mut uart,
    );
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

        #[cfg(feature = "pi-protocol-mode")]
        boot_session.on_deadline(&health, &mut uart);

        // 期限を積み直した後の時刻で残りを測る。log の所要時間を待ち時間から差し引く。
        #[cfg_attr(not(feature = "pi-protocol-mode"), allow(unused_mut))]
        let mut until = next_heartbeat.min(next_snapshot);
        #[cfg(feature = "pi-protocol-mode")]
        if let Some(retry_deadline) = boot_session.next_deadline_ms() {
            until = until.min(retry_deadline);
        }

        // `pi-protocol-mode`では、sleepの代わりに`UartDriver::read`へ残り時間を
        // timeoutとして渡す。旧設計（`sleep_ms_until`）はUARTを一切読まなかった
        // ため、`read`を導入すること自体が変更点である。data到着があれば
        // timeoutいっぱいまで待たず即座に戻る（`config::PI_PROTOCOL_UART_RX_BUFFER_BYTES`
        // のdoc「容量の根拠」参照）ため、busy-waitにならずACK受信への反応
        // latencyも下がる。ring bufferの容量とdata到着時の早期returnの関係は
        // 同定数のdocが持つ正本であり、ここでは繰り返さない。
        #[cfg(feature = "pi-protocol-mode")]
        {
            let now2 = health.uptime_ms();
            let remaining_ms = until.saturating_sub(now2);
            let ticks = boot_session::ticks_until(std::time::Duration::from_millis(remaining_ms));
            let mut buf = [0_u8; config::PI_PROTOCOL_UART_READ_CHUNK_BYTES];
            // **`Err`を無視する。**esp-idf-hal 0.46.2の`UartRxDriver::read`
            // （`uart.rs`1179〜1246行）は、timeout（data未到着、毎周回起こりうる
            // 正常系）と`uart_read_bytes`自体の失敗（`-1`）を、どちらも同じ
            // `Err(ESP_ERR_TIMEOUT)`に畳んでおり、この版のAPIでは型で区別できない
            // （`len`が`-1`でも`0`でも同じ分岐、doc commentも「timeoutならErr」と
            // 明記している）。両者を区別する手段（別のAPIや`unsafe`）は導入しない。
            // `uart_read_bytes`の`ESP_RETURN_ON_FALSE`（不正な`uart_num`／
            // null buffer／未installのdriver。`uart.c`1662〜1666行）は即時
            // returnであり、繰り返し起きればbusy-waitになる。ここではinstall
            // 済みの`uart`・固定長の`buf`しか渡さないためこの経路は実質
            // 到達しないという前提に立つ。`rx_mux`取得待ち（同1670〜1671行）は
            // `ticks`の間blockするため問題にならない。
            if let Ok(n) = uart.read(&mut buf, ticks) {
                if n > 0 {
                    boot_session.on_bytes(&buf[..n], &health, &mut uart);
                }
            }
        }
        #[cfg(not(feature = "pi-protocol-mode"))]
        sleep_ms_until(until, health.uptime_ms());
    }
}

/// NVS namespace。`bench17`（`run_servo_bench_test`が使うnamespace）とは別の名前にする。
/// 用途が異なるnamespaceを共有すると、片方のkey追加がもう片方の`erase_all`等を
/// 誤って巻き込みうる。
#[cfg(feature = "pi-protocol-mode")]
const SID_NVS_NAMESPACE: &str = "pi_session";
/// 次回起動が使う`sid`をこのkeyへ保存する。
#[cfg(feature = "pi-protocol-mode")]
const SID_NVS_KEY_NEXT: &str = "next_sid";

/// 起動のたびに新しい`sid`を選ぶ（`PROTO-TBD-011`のうち`sid`の生成方法に
/// 当たる部分。§3.1参照。生成方法は確定するが、衝突許容確率は未確定のまま
/// 残る。下の「衝突許容確率」節参照）。
///
/// # 生成方法: NVSの不揮発counter
///
/// **乱数ではなく不揮発counterを使う。**§3.1は生成方法として「乱数、不揮発カウンタ、
/// またはその併用」を挙げている。この crate は`Cargo.toml`で`unsafe_code = "forbid"`
/// としており、`esp_random()`／`bootloader_random_enable()`（ESP-IDF v5.5.3
/// `components/esp_hw_support/include/esp_random.h`・`bootloader_random.h`）は
/// `unsafe extern "C"` fnであるため直接呼べない。加えて、ESP-IDF公式資料
/// （`docs/en/api-reference/system/random.rst`）は、Wi-Fi／Bluetoothが有効か、
/// `bootloader_random_enable()`を呼んでいるか、second-stage bootloader実行中の
/// いずれかを満たさない限り、RNGの出力は「pseudo-random only」と明記する。
/// このfirmwareは`pi-protocol-mode`でWi-Fi／Bluetoothを一切初期化しない
/// （`Peripherals::take()`はUART0向けに呼ぶが、Wi-Fi／Bluetoothの初期化は
/// 別の話であり行わない。`#446` PR B）。`bootloader_random_enable()`も呼ばない
/// （呼ぶには`unsafe`が要る）。**したがって乱数側を使っても、
/// 上記の非衝突要件に対する根拠のある確率は示せない。**
///
/// 不揮発counterは`unsafe`もWi-Fi／Bluetoothの初期化も要らず、§3.1の要件
/// （再起動のたびに新しい値を選ぶ）を確率ではなく構造で満たそうとする。
/// [`EspNvs::get_u32`]で前回保存した値を読み、1加算し、**使う前に**
/// [`EspNvs::set_u32`]で保存する（保存後に停電しても、次回起動は保存済みの
/// 値から続くため、このboot分の値が失われるだけで、値の再利用は起きない）。
///
/// # 衝突許容確率
///
/// **0ではない。**`u32`一周（`u32::MAX`回のcommit）は実運用で起こらないが、
/// counterが`0`へ戻る経路が一周以外にもある。
///
/// `EspDefaultNvsPartition::take()`は`take_with(true)`（`reinit=true`）を呼ぶ
/// （esp-idf-svc 0.52.1 `src/nvs.rs`の`EspNvsPartition<NvsDefault>::take`）。
/// `NvsDefault::init(reinit=true)`は、`nvs_flash_init()`が
/// `ESP_ERR_NVS_NO_FREE_PAGES`または`ESP_ERR_NVS_NEW_VERSION_FOUND`を返すと、
/// **`nvs_flash_erase()`でdefault partition全体を消去してから再初期化する**
/// （同fileの`NvsDefault::init`）。この経路を通ると[`SID_NVS_KEY_NEXT`]も失われ、
/// counterは`0`から数え直しになり、以前使った小さい`sid`を再び選びうる。
/// 人がflash全体を消去した場合も同じ結果になる。**したがって「commitが
/// 成功する限り再利用しない」だけでは正しくない。**
///
/// counterが`0`から数え直された後にESP32が送る小さい`sid`が、受信側のretired
/// session集合に残っている値と一致すれば衝突する。この衝突は、protocol側の
/// `stale_session`回復（§3.1「`sid`が衝突した場合」。ACKで`stale_session`を
/// 受けたら新しい`sid`を選び直して再送する）で扱う対象である。**この回復経路は
/// `crate::boot_session::BootSession::reselect_sid`が実装している
/// （`#446` PR B）。**選び直しの回数には上限がある（`BootRetryPolicy`の
/// `sid_reselect_limit`）ため、counterが`0`から数え直された後に連続して選ぶ
/// `sid`が、上限+1回分以上retired session集合に含まれていると衝突から
/// 抜けられないまま終端しうる。
///
/// # NVSが使えない場合
///
/// `EspDefaultNvsPartition::take()`／`EspNvs::new`／`get_u32`／`set_u32`の
/// いずれかが失敗した場合、`health.uptime_ms()`の下位32 bitへ縮退する
/// （旧`sid_from_uptime`と同じ値）。**この経路では非衝突を主張しない。**
/// bring-upを行わないため起動ごとにほぼ同じ小さい値になり、§3の要件を
/// 満たさないまま`boot`を送る。エラーは`log::error!`で分類するが、
/// `pi-protocol-mode`はloggingを止めているため出力されない
/// （観測経路が無い。counterは持たない。`boot_session`のmodule doc「停止理由の区別」参照）。
#[cfg(feature = "pi-protocol-mode")]
fn generate_sid(health: &Health) -> u32 {
    use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};

    let nvs_partition = match EspDefaultNvsPartition::take() {
        Ok(partition) => partition,
        Err(err) => {
            log::error!("sid_nvs_partition_failed error={err}");
            return sid_from_uptime(health.uptime_ms());
        }
    };
    let nvs = match EspNvs::new(nvs_partition, SID_NVS_NAMESPACE, true) {
        Ok(nvs) => nvs,
        Err(err) => {
            log::error!("sid_nvs_open_failed error={err}");
            return sid_from_uptime(health.uptime_ms());
        }
    };

    // `None`（keyが無い＝初回起動）は`0`とみなす。protocolはsid=0を特別扱いしない
    // ため（Envelope §3、`u32`の正当な値）、「保存されていない」と「0が保存されている」を
    // 区別する必要は無い。どちらでも次のsidは`1`加算した値になる（最初の起動なら`1`）。
    let previous = match nvs.get_u32(SID_NVS_KEY_NEXT) {
        Ok(value) => value.unwrap_or(0),
        Err(err) => {
            log::error!("sid_nvs_get_failed error={err}");
            return sid_from_uptime(health.uptime_ms());
        }
    };
    // **`wrapping_add`のままにする。**`u32`一周（`generate_sid`のdoc「衝突許容確率」
    // 参照）は実運用では起こらないため、その先のsidが`0`や過去の小さい値と
    // 見た目上一致しても、`max(1)`のような補正は入れない。補正を入れると、
    // 一周した直後のsidが常に同じ値（`1`）へ寄せられ、実際に一周する状況では
    // 逆に衝突を作り込む。
    let sid = previous.wrapping_add(1);

    // **実際に返す前にcommitする。**`run_servo_bench_test`の「実際に動かす前に
    // latchをcommitする」と同じ順序（module doc参照）。
    if let Err(err) = nvs.set_u32(SID_NVS_KEY_NEXT, sid) {
        log::error!("sid_nvs_set_failed error={err}");
        return sid_from_uptime(health.uptime_ms());
    }

    sid
}

/// [`generate_sid`]のNVS不使用時の縮退経路が使う。`ts_ms`（`u64`）を`sid`（`u32`）へ
/// 切り詰める（下位32 bit）。**非衝突を主張しない**（[`generate_sid`]のdoc参照）。
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
/// **2026-09-24追記（#472）: この段落の配線についての記述は、2026-09-22の
/// [EXP-015](../../../docs/hardware/experiment-log.md)の記録と合わない。**同記録は、両moduleを
/// GPIO25／GPIO26・`3V3`・`GND`へ配線して通電したと書いている。同記録は、`module電源pinの独立性`／
/// `pin header対応`の確認（`docs/hardware/power-budget.md`の`ACCEL-01／ENV-01単体bring-upの手順`の
/// 条件(5)(a)）が未達のまま、`Cb`が未測定のまま通電したと書いている。
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
/// `bench-servo-test-17` feature付きbuildだけがこの関数を呼ぶ（#474で、そのbuildは
/// `compile_error!`でcompileが止まる）。承認の状態は
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
/// `boot`用に選ぶ`sid`は`generate_sid`参照。両者は別の判断である）。
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
