//! `LCD-*`と`TOUCH-*`が使うSPI hostへの書き込み口。bus（`SpiDriver`）の持ち方が2通りある。
//!
//! - [`SpiPort::Held`]: `SpiBusDriver`。作った時点でbus lockを取り、手放すまで握り続ける。
//!   **`touch-sample-14`だけがこちらである**。1つのdriverがhostを専有し、転送ごとのbusの取り直しがない。
//! - [`SpiPort::Shared`]: `SpiDeviceDriver`（hardware CSなし）。busを転送ごとに取り、`Rc<SpiDriver>`を共有する。
//!   LCDを使うbuild（製品build、`bringup-display-13`、`face-21`、`touch-lcd-14`）が使い、LCDとタッチが同じhostにdeviceを持つ。
//!   `touch-lcd-14`と製品buildはタッチを読み、`bringup-display-13`・`face-21`はタッチのdeviceを持つが読まない（`crate::touch`）。
//!
//! どちらも`spi_transmit`へ渡す転送は同じで、CSは持たない（呼び出し側が自前のGPIOで扱う）。
//! 違うのはbus lockを握り続けるか、転送ごとに取るかだけである。

// `touch-sample-14`は`Shared`を、それ以外のbuildは`Held`を作らない。`crate::display`と同じく、moduleごと`#[cfg]`で落とさず全buildでcompileする。
#![allow(dead_code)]

use std::rc::Rc;

use esp_idf_svc::hal::spi::config::{Config as SpiConfig, MODE_0};
use esp_idf_svc::hal::spi::{SpiBusDriver, SpiDeviceDriver, SpiDriver};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::EspError;

pub enum SpiPort<'d> {
    Held(SpiBusDriver<'d, SpiDriver<'d>>),
    Shared(SpiDeviceDriver<'d, Rc<SpiDriver<'d>>>),
}

fn config(clock_hz: u32) -> SpiConfig {
    SpiConfig::new().baudrate(Hertz(clock_hz)).data_mode(MODE_0)
}

impl<'d> SpiPort<'d> {
    /// `driver`を専有し、bus lockを握り続ける。
    pub fn held(driver: SpiDriver<'d>, clock_hz: u32) -> Result<Self, EspError> {
        Ok(Self::Held(SpiBusDriver::new(driver, &config(clock_hz))?))
    }

    /// 共有する`driver`に、`clock_hz`のdeviceを1つ足す。
    pub fn shared(driver: Rc<SpiDriver<'d>>, clock_hz: u32) -> Result<Self, EspError> {
        Ok(Self::Shared(SpiDeviceDriver::new(
            driver,
            Option::<esp_idf_svc::hal::gpio::AnyOutputPin>::None,
            &config(clock_hz),
        )?))
    }

    pub fn write(&mut self, words: &[u8]) -> Result<(), EspError> {
        match self {
            Self::Held(bus) => bus.write(words),
            Self::Shared(device) => device.write(words),
        }
    }

    pub fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), EspError> {
        match self {
            Self::Held(bus) => bus.transfer(read, write),
            Self::Shared(device) => device.transfer(read, write),
        }
    }
}
