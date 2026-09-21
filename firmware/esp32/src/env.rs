//! `ENV-01`（BME280）のI2C driver。
//!
//! [Issue #16](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/16) の受け入れ条件
//! （Device識別情報とcalibration処理の検証、raw値と変換値の観測）のうち、この段階では
//! **Chip ID (register `0xD0`) の読み出しだけを扱う**（[AGENTS.md](../../../AGENTS.md)
//! 「最終形を一度に作らず、単体から統合へ進める」）。calibration・測定値の読み出しは別途進める。
//!
//! `main()`は`crate::run_i2c_bringup`からこのdriverを呼び、生byteをlogへ出す
//! （一致判定はしない。`main.rs`のmodule doc参照）。**実機通電はまだ行っていない。**bus共有pull-up(`ACCEL-SDA`/`ACCEL-SCL`の外部4.7 kΩ)はbreadboardへ実装済みだが(`docs/hardware/gpio-assignment.md`710行目、2026-09-07)、sensor module自体の接続(address/mode選択の配線)はまだ行っていない。
//! この呼び出しはcross-compileの確認までであり、実機での動作確認は別工程である
//! （[Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)
//! 「人間の監視が必要な操作」）。
//!
//! # 配線の根拠
//!
//! pin割り当ては
//! [`docs/hardware/gpio-assignment.md`](../../../docs/hardware/gpio-assignment.md)
//! の`信号inventory`が正本である。`ENV-SDA`はGPIO25、`ENV-SCL`はGPIO26であり、いずれも
//! `ACCEL-01`（[Issue #15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)）
//! と共有するbusである（同文書418行目「ENV-SCL | ENV-01 | I2C SCL | Bidirectional |
//! GPIO26（ACCEL-01と共有）」、415行目「ACCEL-SCL | ACCEL-01 | ... | GPIO26」と一致）。
//! **このmoduleはbus(`I2cDriver`)を所有しない。**呼び出し側（`main.rs`）が1つのbusを
//! 作り、`Adxl345`（`#15`側のdriver、`crate::accel`）と共有する設計であるため、
//! [`Bme280`]は自分のI2C addressだけを持ち、各methodは呼び出し側が渡す
//! `&mut I2cDriver`を借りる。
//!
//! bus speedはStandard-mode（100 kHz）を使う（同文書「初回bring-upのmode決定」節、
//! 2026-09-06決定）。ESP32内蔵のweak pull-upは有効にしない。同文書の`I2C busの実効
//! pull-up`節が導出したpull-up値（ADXL345側`01C`並列5.00 kΩ ∥ 外部4.7 kΩ ≈ 2.42 kΩ）は
//! 外部pull-upだけを前提にしており、ESP32内蔵pull-up（typ 45 kΩ）を含めていないため。
//!
//! # I2C addressについて
//!
//! **このmoduleはaddressを定数で持たない。**`0x76`（`SDO`→GND）／`0x77`（`SDO`→VDD）の
//! どちらになるかは配線で決まり、まだ配線していない（同文書「I2C addressの選択」節。
//! 安全要件5項目に効かない一般値扱いであり、未確定でも着手を止めない）。呼び出し側が
//! [`Bme280::new`]へ渡す。
//!
//! # register・timingの根拠
//!
//! 一次資料は**Bosch BME280 Data Sheet Revision 1.24**
//! （<https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf>、
//! `docs/hardware/sensor-datasheet-notes.md`が既に引用しているものと同一revision）。
//!
//! - Chip ID register（`0xD0`）。reset値は`0x60`（同文書`sensor-datasheet-notes.md`が
//!   既に記録済み。**ここへ再掲しない**）。identify判定（`0x60`との一致）は呼び出し側の
//!   責務とする（`display.rs`の[`DisplayId`](crate::display::DisplayId)と同じ「捏造しない」
//!   形。このmoduleは生byteを返すだけで、BME280であると断定しない）。

use esp_idf_svc::hal::delay::BLOCK;
use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::sys::EspError;

/// Chip ID register。Bosch BME280 Data Sheet Revision 1.24（module doc参照）。
const REG_CHIP_ID: u8 = 0xD0;

/// `ENV-01`（BME280）のI2C driver。
///
/// **busを所有しない。**`i2c`引数として呼び出し側の`&mut I2cDriver`を都度借りる
/// （module doc参照）。保持するのは自分のI2C addressだけである。
pub struct Bme280 {
    address: u8,
}

impl Bme280 {
    /// I2C addressを指定してdriverを作る。**まだ通信しない。**
    ///
    /// `address`は`0x76`／`0x77`のいずれかであり、`SDO`の配線で決まる
    /// （module doc「I2C addressについて」参照）。呼び出し側が確定させる。
    #[must_use]
    pub const fn new(address: u8) -> Self {
        Self { address }
    }

    /// Chip ID register（`0xD0`）を読む。
    ///
    /// **生byteをそのまま返す。**BME280のreset値`0x60`との一致判定は呼び出し側の
    /// 責務とする（module doc参照）。
    pub fn read_chip_id(&self, i2c: &mut I2cDriver<'_>) -> Result<u8, EspError> {
        let mut buf = [0u8; 1];
        i2c.write_read(self.address, &[REG_CHIP_ID], &mut buf, BLOCK)?;
        Ok(buf[0])
    }
}
