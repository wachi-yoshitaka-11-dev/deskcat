//! `ACCEL-01`（ADXL345）のI2C driver。
//!
//! [Issue #15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15) の受け入れ条件
//! （Device IDと正確な変換の検証、offsetとnoiseの測定など）のうち、この段階では
//! **Device ID (register `0x00`) の読み出しだけを扱う**（[AGENTS.md](../../../AGENTS.md)
//! 「最終形を一度に作らず、単体から統合へ進める」）。calibration・測定値の読み出しは別途進める。
//!
//! # 配線の根拠
//!
//! pin割り当ては
//! [`docs/hardware/gpio-assignment.md`](../../../docs/hardware/gpio-assignment.md)
//! の`信号inventory`が正本である。`ACCEL-SDA`はGPIO25、`ACCEL-SCL`はGPIO26であり、
//! いずれも`ENV-01`（[Issue #16](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/16)、
//! [`crate::env`]）と共有するbusである（同文書414行目「ACCEL-SDA | ACCEL-01 | I2C SDA |
//! Bidirectional | GPIO25」、415行目「ACCEL-SCL | ACCEL-01 | I2C SCL | Bidirectional |
//! GPIO26」、417行目「ENV-SDA | ENV-01 | ... | GPIO25（ACCEL-01と共有）」、418行目
//! 「ENV-SCL | ENV-01 | ... | GPIO26（ACCEL-01と共有）」）。**このmoduleはbus
//! (`I2cDriver`)を所有しない。**[`crate::env::Bme280`]と同じ理由（呼び出し側が1つの
//! busを作り、2つのdriverで共有する）で、[`Adxl345`]は自分のI2C addressだけを持ち、
//! 各methodは呼び出し側が渡す`&mut I2cDriver`を借りる。**`#16`のdriverと重複する
//! コードがあっても、2つしか無いうちは共通化しない**（3つ目が来たら検討する）。
//!
//! bus speedはStandard-mode（100 kHz）を使う（同文書「初回bring-upのmode決定」節、
//! 2026-09-06決定。[`crate::env`]と同じ根拠。**ここへ再掲しない**）。
//!
//! # I2C addressについて
//!
//! **このmoduleはaddressを定数で持たない。**`SDO`（`ALT ADDRESS`）の配線で
//! `0x1D`（`SDO`→VDD）／`0x53`（`SDO`→GND）のどちらになるかが決まり、まだ配線して
//! いない（同文書「I2C addressの選択」節。安全要件5項目に効かない一般値扱いであり、
//! 未確定でも着手を止めない）。呼び出し側が[`Adxl345::new`]へ渡す。
//!
//! # register・timingの根拠
//!
//! 一次資料は**Analog Devices ADXL345 Data Sheet Rev. G**
//! （<https://www.analog.com/media/en/technical-documentation/data-sheets/adxl345.pdf>、
//! `docs/hardware/sensor-datasheet-notes.md`が既に引用しているものと同一revision）。
//!
//! - Device ID register（`0x00`、`DEVID`、Read Only）。reset値は`0xE5`
//!   （`docs/hardware/sensor-datasheet-notes.md`165行目「`DEVID`（address `0x00`、
//!   Read Only）。reset値`11100101`＝`0xE5`（`The DEVID register holds a fixed device
//!   ID code of 0xE5 (345 octal)`）。Table 19 page 23、Register 0x00節 page 24」。
//!   **ここへ再掲しない**）。identify判定（`0xE5`との一致）は呼び出し側の責務とする
//!   （[`crate::env::Bme280::read_chip_id`]・`display.rs`の
//!   [`DisplayId`](crate::display::DisplayId)と同じ「捏造しない」形。このmoduleは
//!   生byteを返すだけで、ADXL345であると断定しない）。
//!
//! `main()`は`crate::run_i2c_bringup`からこのdriverを呼び、生byteをlogへ出す
//! （一致判定はしない。`main.rs`のmodule doc参照）。**実機通電はまだ行っていない。**bus共有pull-up(`ACCEL-SDA`/`ACCEL-SCL`の外部4.7 kΩ)はbreadboardへ実装済みだが(`docs/hardware/gpio-assignment.md`の`競合check`節の受け入れchecklist「すべての外部pull-upが3.3Vへ接続され」の項目、2026-09-07)、sensor module自体の接続(address/mode選択の配線)はまだ行っていない。
//! この呼び出しはcross-compileの確認までであり、実機での動作確認は別工程である
//! （[Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)
//! 「人間の監視が必要な操作」）。

use esp_idf_svc::hal::delay::{TickType, TickType_t};
use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::sys::EspError;

use crate::config;

/// Device ID register。Analog Devices ADXL345 Data Sheet Rev. G（module doc参照）。
const REG_DEVID: u8 = 0x00;

/// [`Adxl345::read_device_id`]の1 transactionのtimeout（tick）。
///
/// **`esp_idf_svc::hal::delay::BLOCK`（無期限）を使わない。**`SDA`がLowのまま固着した
/// 場合（配線ミス、jumper未設定など）に呼び出しが返らず、`main()`がheartbeatのloopへ
/// 到達しないため、人には「何も出ない」以外の情報が届かない。それでは「ハングした」
/// 「起動していない」「センサが無い」を区別できない
/// （[Issue #451](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/451)）。
///
/// 値と導出は[`crate::config::I2C_TRANSACTION_TIMEOUT_MS`]が正本である。
/// **ここへ再掲しない。**[`crate::env`]も同じ定数から同じ換算で作る。
const READ_TIMEOUT_TICKS: TickType_t =
    TickType::new_millis(config::I2C_TRANSACTION_TIMEOUT_MS).ticks();

/// `ACCEL-01`のI2C driver。
///
/// **busを所有しない。**`i2c`引数として呼び出し側の`&mut I2cDriver`を都度借りる
/// （module doc参照）。保持するのは自分のI2C addressだけである。
pub struct Adxl345 {
    address: u8,
}

impl Adxl345 {
    /// I2C addressを指定してdriverを作る。**まだ通信しない。**
    ///
    /// `address`は`0x1D`（`SDO`→VDD）／`0x53`（`SDO`→GND）のいずれかであり、`SDO`の
    /// 配線で決まる（module doc「I2C addressについて」参照）。呼び出し側が確定させる。
    #[must_use]
    pub const fn new(address: u8) -> Self {
        Self { address }
    }

    /// Device ID register（`0x00`）を読む。
    ///
    /// **生byteをそのまま返す。**ADXL345のreset値`0xE5`との一致判定は呼び出し側の
    /// 責務とする（module doc参照）。
    ///
    /// **有限時間で返る。**timeoutは[`READ_TIMEOUT_TICKS`]であり、超えると
    /// `Err(EspError)`（`ESP_ERR_TIMEOUT`）になる。応答しないsensorとbus固着を
    /// この関数は区別しない。**区別するのはlogを読む人間である。**
    pub fn read_device_id(&self, i2c: &mut I2cDriver<'_>) -> Result<u8, EspError> {
        let mut buf = [0u8; 1];
        i2c.write_read(self.address, &[REG_DEVID], &mut buf, READ_TIMEOUT_TICKS)?;
        Ok(buf[0])
    }
}
