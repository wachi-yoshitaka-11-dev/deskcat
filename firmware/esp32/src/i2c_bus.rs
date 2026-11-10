//! `ACCEL-01`（ADXL345）と`ENV-01`（BME280）が共有するI2C bus（`GPIO25`＝SDA、`GPIO26`＝SCL）の持ち方（[#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)）。
//!
//! 製品buildは、`crate::tap::TapDetector`と`crate::env_report::EnvReporter`が1つの`I2cDriver`を共有する。
//! **どちらもmain loopの同じtaskから呼ぶので、bus（`I2cDriver`）を取り合うのは、1つのtransactionの間だけである。**
//! `RefCell`の借用は、transactionの呼び出し1回で終わる（呼び出しをまたいで借りたままにしない）。
//! 2つのmoduleが入れ子に呼び合うことは無いので、借用が重なって`panic`することは無い。
//! **ISRからは呼ばない**（I2Cのtransactionはblockするため）。
//!
//! bus lockはESP-IDF側のI2C driverが持つ（`i2c_master_cmd_begin`のsemaphore）。**このmoduleは別のlockを足さない。**
//! 1 transactionは有限時間で返る（`config::I2C_TRANSACTION_TIMEOUT_MS`）ため、片方のmoduleがもう片方を無期限に待たせることは無い。
//! tapの`INT_SOURCE`のtap bitは読むまで残る（`crate::tap`）ので、envのtransactionが挟まっても、tapの検出は落ちない
//! （読みが遅れるだけである）。

// tapを持たない試験buildではdead_codeになる（`crate::tap`と同じく、moduleごと`#[cfg]`で落とさない）。
#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use esp_idf_svc::hal::i2c::I2cDriver;

/// 共有するI2C bus。`Rc`のcloneで、tapとenvが同じ`I2cDriver`を持つ。
pub type SharedI2c = Rc<RefCell<I2cDriver<'static>>>;

/// `driver`を共有できる形にする。
#[must_use]
pub fn share(driver: I2cDriver<'static>) -> SharedI2c {
    Rc::new(RefCell::new(driver))
}
