//! main taskをtask watchdog（TWDT）へ登録する処理と、その検証専用build（`bench-wdt-711`）の本体
//! （[Issue #711](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711)）。
//!
//! **servoを駆動するbuildは、[`register_main_task`]で登録する。**main taskが止まったまま、LEDCが最後のdutyを
//! 出し続けてservoを拘束し続けることを、timeoutで限るためである。`bench-servo-calib-18`
//! （`crate::servo_calib`）が呼ぶ。**製品build（`deskcat_product_mode`。既定、`no-touch-487`、
//! `led-no-touch-514`、`link-gap-20`）も、servoを駆動しないが、同じ関数で登録する**（[`ProductWatchdog`]。
//! [Issue #487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)の条件1。servoを出すより前に、
//! main loopが止まったときにresetする経路を、servoの無い構成で先に動かすためである）。
//!
//! **この関数は設定を決めない。**timeout・idle taskの監視・panicは、`TWDTConfig::new()`＝sdkconfigの値のままである
//! （`CONFIG_ESP_TASK_WDT_TIMEOUT_S`、`CONFIG_ESP_TASK_WDT_CHECK_IDLE_TASK_CPU*`、`CONFIG_ESP_TASK_WDT_PANIC`）。
//! `TWDTDriver::new`は、`CONFIG_ESP_TASK_WDT_INIT=y`のとき、ESP-IDFが起動時に初期化したTWDTを
//! `esp_task_wdt_reconfigure`で、渡した設定（ここでは同じ値）へ作り直す（esp-idf-hal 0.46.2 `task.rs`）。
//!
//! # `bench-wdt-711`
//!
//! 実機で「main taskを登録した後にfeedしないと、約timeout（5 s）でpanicしてresetし、再起動後の
//! `reset_reason`が`task_watchdog`になる」ことを見るためのbuildである。**servoに触れない。**`Sg90`・LEDC・
//! `SERVO-PWM`（GPIO27）を作らず、Pi link・I2C・LCDも持たない。**登録して`bench_wdt_711_registered`を出した後は、
//! 一度もfeedしない**（[`run_wdt_test`]）。製品buildと校正buildには、hangを起こすcommandを入れていない。
//! **`CONFIG_ESP_TASK_WDT_PANIC`が無効なら（`panic_on_trigger`がfalse）、登録せずに`bench_wdt_711_invalid_config`を出して止まる**
//! （発火してもresetしないので、このbuildの目的を果たせない）。
//! **resetの後、同じfirmwareが再び登録して、約timeoutごとにresetを繰り返す**（試験の終わりは人が決める。
//! flashし直すか、電源を切る）。
//! 実機で、約 timeout で panic して reset し、`reset_reason=task_watchdog` になることを確かめた（[EXP-060](../../../docs/hardware/experiment-log.md)の (A)）。

#[cfg(feature = "bench-wdt-711")]
use std::time::Instant;

#[cfg(feature = "bench-wdt-711")]
use esp_idf_svc::hal::delay::FreeRtos;
#[cfg(feature = "bench-wdt-711")]
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::task::watchdog::{TWDTConfig, TWDTDriver, WatchdogSubscription, TWDT};
use esp_idf_svc::sys::EspError;

/// 登録の失敗。**どちらの場合も、main taskは登録されていない。**
#[derive(Debug)]
pub enum RegisterError {
    /// `TWDTDriver::new`（TWDTの設定）が失敗した。
    Driver(EspError),
    /// `watch_current_task`（呼び出したtaskの登録）が失敗した。
    Subscribe(EspError),
}

impl core::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Driver(err) => write!(f, "TWDTDriver::new failed: {err}"),
            Self::Subscribe(err) => write!(f, "task watchdog subscription failed: {err}"),
        }
    }
}

/// 呼び出したtask（main task）をTWDTへ登録する。
///
/// `slot`へTWDTのdriverを置き、返す購読はそれを借りる（購読はdriverより長く生きられない）。
/// 購読を手放すと（drop）、登録が外れる。**登録した後は、timeoutより短い間隔で
/// [`WatchdogSubscription::feed`]を呼び続ける。**呼ばないと、TWDTが発火する（panicの設定はsdkconfigの値）。
///
/// # Errors
///
/// 設定または登録に失敗したとき（[`RegisterError`]）。**失敗した場合、main taskは登録されていない。**
pub fn register_main_task<'a>(
    slot: &'a mut Option<TWDTDriver<'static>>,
    twdt: TWDT<'static>,
) -> Result<WatchdogSubscription<'a>, RegisterError> {
    let driver =
        slot.insert(TWDTDriver::new(twdt, &TWDTConfig::new()).map_err(RegisterError::Driver)?);
    driver
        .watch_current_task()
        .map_err(RegisterError::Subscribe)
}

/// 製品buildのmain taskのwatchdog。**登録できなくても、製品buildは動き続ける**（servoを駆動しないため。
/// 登録できなかったことは`product_wdt_unavailable`としてlogへ出す）。**servoを駆動する製品buildにする
/// ときは、登録できなければservoを動かさない扱いへ変える**（`bench-servo-calib-18`と同じ。#20）。
///
/// 設定は`TWDTConfig::new()`＝sdkconfigの値であり、ここでtimeoutを決めない
/// （[`register_main_task`]）。**feedは、main loopの各周の初めに1回だけ行う。**
/// timeoutと、loopの1周の最悪の長さの関係は、`main.rs`のmodule docの`Task Watchdog Timer`の段落が持つ。
#[cfg(deskcat_product_mode)]
pub struct ProductWatchdog<'a> {
    subscription: Option<WatchdogSubscription<'a>>,
    feed_failure_logged: bool,
}

#[cfg(deskcat_product_mode)]
impl<'a> ProductWatchdog<'a> {
    /// main taskを登録する。失敗は握りつぶさず、1回だけlogへ出し、以後`feed`は何もしない。
    pub fn register(slot: &'a mut Option<TWDTDriver<'static>>, twdt: TWDT<'static>) -> Self {
        let panic_on_trigger = TWDTConfig::new().panic_on_trigger;
        let subscription = match register_main_task(slot, twdt) {
            Ok(subscription) => {
                log::info!(
                    "product_wdt_registered timeout_s={} panic_on_trigger={panic_on_trigger}",
                    esp_idf_svc::sys::CONFIG_ESP_TASK_WDT_TIMEOUT_S
                );
                Some(subscription)
            }
            Err(err) => {
                log::error!(
                    "product_wdt_unavailable error={err}. main taskは未登録で、製品buildはwatchdogなしで動く"
                );
                None
            }
        };
        Self {
            subscription,
            feed_failure_logged: false,
        }
    }

    /// 登録していれば餌付けする。失敗は最初の1回だけlogへ出す（毎周の出力にしない）。
    pub fn feed(&mut self) {
        if let Some(subscription) = self.subscription.as_mut() {
            if let Err(err) = subscription.feed() {
                if !self.feed_failure_logged {
                    self.feed_failure_logged = true;
                    log::error!("product_wdt_feed_failed error={err}. 以後の失敗はlogへ出さない");
                }
            }
        }
    }
}

/// `bench-wdt-711`のbuildの本体。**戻らない。**登録した後は一度もfeedしない。
#[cfg(feature = "bench-wdt-711")]
pub fn run_wdt_test() -> ! {
    let peripherals = Peripherals::take().expect("Peripherals::take must succeed exactly once");
    let panic_on_trigger = TWDTConfig::new().panic_on_trigger;
    log::info!(
        "bench_wdt_711_start timeout_s={} panic_on_trigger={panic_on_trigger}",
        esp_idf_svc::sys::CONFIG_ESP_TASK_WDT_TIMEOUT_S
    );
    // panicが無効だと、TWDTは発火しても警告するだけでresetしない。このbuildは「panicしてresetする」ことを
    // 見るためのものなので、登録の前に止める（値の出所はsdkconfigの`CONFIG_ESP_TASK_WDT_PANIC`のまま）。
    if !panic_on_trigger {
        log::error!(
            "bench_wdt_711_invalid_config panic_on_trigger=false. CONFIG_ESP_TASK_WDT_PANICが無効で、発火してもresetしない。main taskを登録せず、何も試験しない"
        );
        loop {
            FreeRtos::delay_ms(1000);
        }
    }
    let mut slot = None;
    // 購読を手放さない（dropすると登録が外れ、発火しない）。
    let _subscription = match register_main_task(&mut slot, peripherals.twdt) {
        Ok(subscription) => subscription,
        Err(err) => {
            log::error!(
                "bench_wdt_711_register_failed error={err}. main taskは未登録で、何も試験しない"
            );
            loop {
                FreeRtos::delay_ms(1000);
            }
        }
    };
    let registered_at = Instant::now();
    log::info!("bench_wdt_711_registered. これ以降feedしない。timeout後にpanicしてresetする見込み");
    // **feedしない。**待つ間もIDLE taskへ時間を渡す（`delay_ms`）ので、発火するのはmain taskの登録だけである。
    loop {
        FreeRtos::delay_ms(1000);
        log::info!(
            "bench_wdt_711_waiting elapsed_ms={}",
            registered_at.elapsed().as_millis()
        );
    }
}
