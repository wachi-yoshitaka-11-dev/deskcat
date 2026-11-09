//! main taskをtask watchdog（TWDT）へ登録し、**発火したらpanicしてresetする**ようにする
//! （[Issue #711](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/711)）。
//!
//! **servoを駆動するbuildは、[`register_main_task_with_panic`]で登録してpanicを有効にする。**
//! main taskが止まったまま、LEDCが最後のdutyを出し続けてservoを拘束し続けることを、timeoutで限るためである。
//! 後で製品buildがservoを駆動するようになったら（#19・#20）、そのときにこの関数を使う。
//!
//! **`sdkconfig.defaults`は`CONFIG_ESP_TASK_WDT_PANIC`を変えていない**（既定の「警告だけ」）。そのため、
//! servoを駆動しないbuild（製品buildを含む）のTWDTは従来どおり警告だけで、resetしない。
//! panicを有効にするのは、この関数を呼んだbuildだけである。
//!
//! # 設定の根拠（ESP-IDF v5.5.3のsourceと、esp-idf-hal 0.46.2のsourceを読んだ結果）
//!
//! - `TWDTDriver::new`は、`CONFIG_ESP_TASK_WDT_INIT=y`のとき（生成された`sdkconfig`で確認済み）、
//!   ESP-IDFが起動時に初期化したTWDTを`esp_task_wdt_reconfigure`で、渡した設定へ作り直す
//!   （esp-idf-hal `src/task.rs`の`TWDTDriver::new`と`watchdog_is_init_by_idf`）。
//! - `esp_task_wdt_reconfigure`は`p_twdt_obj->panic = config->trigger_panic`を書く
//!   （`components/esp_system/task_wdt/task_wdt.c`）。timeout時の処理（`task_wdt_timeout_handling`）は
//!   この**TWDT全体の1つの値**を読むので、panicはidle taskの監視にも効く。
//! - timeoutと監視するidle taskは、`TWDTConfig::new()`＝sdkconfigの値（`CONFIG_ESP_TASK_WDT_TIMEOUT_S`、
//!   `CONFIG_ESP_TASK_WDT_CHECK_IDLE_TASK_CPU*`）のままにする。**ここでtimeoutを決めない。**
//! - `esp_task_wdt_reconfigure`は、内部のtimerの再設定が失敗しても`ESP_OK`を返す（同sourceの`err:`の行）。
//!   そのため、この関数の成功は、設定が効いたことの証拠ではない。**実機では確かめていない。**

use esp_idf_svc::hal::task::watchdog::{TWDTConfig, TWDTDriver, WatchdogSubscription, TWDT};
use esp_idf_svc::sys::EspError;

/// panicを有効にしたTWDTへ、呼び出したtask（main task）を登録する。
///
/// `slot`へTWDTのdriverを置き、返す購読はそれを借りる（購読はdriverより長く生きられない）。
/// 購読を手放すと（drop）、登録が外れる。**登録した後は、timeoutより短い間隔で
/// [`WatchdogSubscription::feed`]を呼び続ける。**呼ばないと、panicしてresetする。
///
/// # Errors
///
/// TWDTの設定または登録に失敗したとき。**失敗した場合、main taskは登録されていない。**
/// `TWDTDriver::new`が成功した後で登録に失敗したときは、panicの設定だけが効いている
/// （idle taskの監視が発火すればpanicする）。
pub fn register_main_task_with_panic<'a>(
    slot: &'a mut Option<TWDTDriver<'static>>,
    twdt: TWDT<'static>,
) -> Result<WatchdogSubscription<'a>, EspError> {
    let mut config = TWDTConfig::new();
    config.panic_on_trigger = true;
    let driver = slot.insert(TWDTDriver::new(twdt, &config)?);
    driver.watch_current_task()
}
