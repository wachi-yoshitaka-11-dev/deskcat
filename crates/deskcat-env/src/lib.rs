//! `ENV-01` の BME280 生値と補正係数から温度・気圧・相対湿度を計算する。
//!
//! I2C と測定設定は firmware の責務。レジスタの byte の組み立ては
//! host で境界を試験できるよう、この crate の純粋な関数に置く。
//!
//! 出所: [Bosch Sensortec BME280 Data sheet, BST-BME280-DS001-24,
//! Revision 1.24 (February 2024)](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf)。
//! §4.2.2 Table 16 が補正係数の符号・bit 幅、§4.2.3 (pp.25–26) が整数補償式と
//! 出力形式、§5.4.7–5.4.9 が生値の形式を定める。

#![no_std]

/// BME280 の読み出し済み補正係数。各 field の型はデータシート §4.2.2 Table 16 に従う。
///
/// `dig_h4` と `dig_h5` は [`decode_calibration`] が 12 bit から符号拡張する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Calibration {
    /// Unsigned temperature coefficient 1.
    pub dig_t1: u16,
    /// Signed temperature coefficient 2.
    pub dig_t2: i16,
    /// Signed temperature coefficient 3.
    pub dig_t3: i16,
    /// Unsigned pressure coefficient 1. Zero is rejected to avoid division by zero.
    pub dig_p1: u16,
    /// Signed pressure coefficient 2.
    pub dig_p2: i16,
    /// Signed pressure coefficient 3.
    pub dig_p3: i16,
    /// Signed pressure coefficient 4.
    pub dig_p4: i16,
    /// Signed pressure coefficient 5.
    pub dig_p5: i16,
    /// Signed pressure coefficient 6.
    pub dig_p6: i16,
    /// Signed pressure coefficient 7.
    pub dig_p7: i16,
    /// Signed pressure coefficient 8.
    pub dig_p8: i16,
    /// Signed pressure coefficient 9.
    pub dig_p9: i16,
    /// Unsigned humidity coefficient 1.
    pub dig_h1: u8,
    /// Signed humidity coefficient 2.
    pub dig_h2: i16,
    /// Unsigned humidity coefficient 3.
    pub dig_h3: u8,
    /// Signed, 12-bit humidity coefficient 4, sign-extended to `i16`.
    pub dig_h4: i16,
    /// Signed, 12-bit humidity coefficient 5, sign-extended to `i16`.
    pub dig_h5: i16,
    /// Signed humidity coefficient 6.
    pub dig_h6: i8,
}

/// 取得済み ADC 生値。温度・気圧は 20 bit、湿度は 16 bit（§4.2.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawSample {
    /// Temperature ADC value (20 bit).
    pub temperature: u32,
    /// Pressure ADC value (20 bit).
    pub pressure: u32,
    /// Humidity ADC value (16 bit).
    pub humidity: u16,
}

/// Bosch §4.2.2 Table 16 の 0x88..0xA1 の長さ。
pub const CALIB_TP_LEN: usize = 26;
/// Bosch §4.2.2 Table 16 の 0xE1..0xE7 の長さ。
pub const CALIB_H_LEN: usize = 7;
/// Bosch §4.1, §5.4.7–5.4.9 の 0xF7..0xFE の burst read 長。
pub const RAW_LEN: usize = 8;
/// Bosch §5.4.1 Table 18 の Chip ID。
pub const CHIP_ID: u8 = 0x60;

/// register の観測値から判定できる fault。I2C 自身の fault は firmware が分類する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterFault {
    /// Chip ID が BME280 ではない。
    WrongChipId,
    /// NVM image copy 中。
    CalibrationNotReady,
    /// 変換中。
    MeasurementNotReady,
    /// 測定を skip した channel の sentinel。
    SkippedChannel,
}

/// I2C error の分類。コードの由来と具体値は firmware 側の ESP-IDF が持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoFault {
    /// transaction の有限時間上限に達した。
    Timeout,
    /// timeout 以外。NACK と物理的 disconnect は、この情報だけでは区別できない。
    Other,
}

/// ESP-IDF の error code と timeout code の一致だけを純粋に判定する。
/// どの code が timeout かは firmware が SDK の定数として渡す。
#[must_use]
pub const fn classify_io_error(code: i32, timeout_code: i32) -> IoFault {
    if code == timeout_code {
        IoFault::Timeout
    } else {
        IoFault::Other
    }
}

/// §5.4.1 の Chip ID を確認する。
///
/// # Errors
///
/// `id` が [`CHIP_ID`] と異なる場合。
pub const fn check_identity(id: u8) -> Result<(), RegisterFault> {
    if id == CHIP_ID {
        Ok(())
    } else {
        Err(RegisterFault::WrongChipId)
    }
}

/// NVM image copy 中には補正係数を使わない（§5.4.4 Table 21）。
///
/// # Errors
///
/// `status.im_update` が 1 の場合。
pub const fn check_calibration_ready(status: u8) -> Result<(), RegisterFault> {
    if status & 0x01 == 0 {
        Ok(())
    } else {
        Err(RegisterFault::CalibrationNotReady)
    }
}

/// 変換中または NVM image copy 中は data register を読まない（§5.4.4 Table 21）。
///
/// # Errors
///
/// `status.measuring` または `status.im_update` が 1 の場合。
pub const fn check_measurement_ready(status: u8) -> Result<(), RegisterFault> {
    if status & 0x09 == 0 {
        Ok(())
    } else {
        Err(RegisterFault::MeasurementNotReady)
    }
}

/// Bosch §4.2.2 Table 16 の byte 配置を `Calibration` へ復号する。
/// 配列はそれぞれ 0x88..0xA1、0xE1..0xE7 の連続 read。
#[must_use]
pub fn decode_calibration(tp: &[u8; CALIB_TP_LEN], h: &[u8; CALIB_H_LEN]) -> Calibration {
    fn le_u16(data: &[u8], i: usize) -> u16 {
        u16::from_le_bytes([data[i], data[i + 1]])
    }
    fn le_i16(data: &[u8], i: usize) -> i16 {
        i16::from_le_bytes([data[i], data[i + 1]])
    }
    // 12 bit の符号 bit を i16 の bit 15 へ移して算術右 shift。
    fn signed12(bits: u16) -> i16 {
        (bits << 4).cast_signed() >> 4
    }
    Calibration {
        dig_t1: le_u16(tp, 0),
        dig_t2: le_i16(tp, 2),
        dig_t3: le_i16(tp, 4),
        dig_p1: le_u16(tp, 6),
        dig_p2: le_i16(tp, 8),
        dig_p3: le_i16(tp, 10),
        dig_p4: le_i16(tp, 12),
        dig_p5: le_i16(tp, 14),
        dig_p6: le_i16(tp, 16),
        dig_p7: le_i16(tp, 18),
        dig_p8: le_i16(tp, 20),
        dig_p9: le_i16(tp, 22),
        dig_h1: tp[25],
        dig_h2: le_i16(h, 0),
        dig_h3: h[2],
        dig_h4: signed12((u16::from(h[3]) << 4) | u16::from(h[4] & 0x0f)),
        dig_h5: signed12((u16::from(h[5]) << 4) | u16::from(h[4] >> 4)),
        dig_h6: h[6].cast_signed(),
    }
}

/// Bosch §4.1, §5.4.7–5.4.9 の連続 8 byte を復号する。
/// skipped channel の sentinel は測定値として補償しない（Tables 20, 23, 24）。
///
/// # Errors
///
/// 温度・気圧・湿度のいずれかが skipped channel の sentinel である場合。
pub fn decode_raw(bytes: &[u8; RAW_LEN]) -> Result<RawSample, RegisterFault> {
    fn adc20(msb: u8, lsb: u8, xlsb: u8) -> u32 {
        (u32::from(msb) << 12) | (u32::from(lsb) << 4) | u32::from(xlsb >> 4)
    }
    let raw = RawSample {
        pressure: adc20(bytes[0], bytes[1], bytes[2]),
        temperature: adc20(bytes[3], bytes[4], bytes[5]),
        humidity: u16::from_be_bytes([bytes[6], bytes[7]]),
    };
    if raw.pressure == 0x80000 || raw.temperature == 0x80000 || raw.humidity == 0x8000 {
        Err(RegisterFault::SkippedChannel)
    } else {
        Ok(raw)
    }
}

/// 補償後の値。単位・Q 形式はデータシート §4.2.3 に従う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// 温度、0.01 °C 単位。例: `5123` は 51.23 °C。
    pub temperature_centi_c: i32,
    /// 気圧、Q24.8 Pa。Pa への換算は 256 で割る。
    pub pressure_q24_8_pa: u32,
    /// 相対湿度、Q22.10 %RH。%RH への換算は 1024 で割る。
    pub humidity_q22_10_percent: u32,
}

impl Reading {
    /// 温度を °C で返す（§4.2.3 の 0.01 °C 単位から換算）。
    #[must_use]
    pub fn temperature_celsius(self) -> f64 {
        f64::from(self.temperature_centi_c) / 100.0
    }

    /// 気圧を Pa で返す（§4.2.3 の Q24.8 から換算）。
    #[must_use]
    pub fn pressure_pa(self) -> f64 {
        f64::from(self.pressure_q24_8_pa) / 256.0
    }

    /// 相対湿度を %RH で返す（§4.2.3 の Q22.10 から換算）。
    #[must_use]
    pub fn humidity_percent(self) -> f64 {
        f64::from(self.humidity_q22_10_percent) / 1024.0
    }
}

/// 入力または計算結果が式の定義域を外れた。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 温度または気圧の生値が 20 bit に収まらない。
    RawOutOfRange,
    /// `dig_h4` または `dig_h5` が符号付き 12 bit に収まらない。
    HumidityCoefficientOutOfRange,
    /// 気圧補償式の除数が 0 になる。
    ZeroPressureDivisor,
    /// 中間の整数演算が `i128` を超えた、または結果の整数形式へ収まらない。
    ArithmeticOverflow,
    /// 気圧の最終値が unsigned Q24.8 に収まらない。
    PressureOutOfRange,
}

/// Bosch §4.2.3 の整数補償式（code revision 1.1）を適用する。
///
/// 気圧・湿度は同じ sample の温度から計算した `t_fine` を使う。符号付き係数と
/// 中間値の算術右 shift を保ちつつ、式の途中では `i128` を用いて overflow を防ぐ。
/// 原式の `dig_p1 == 0` 時の `0` は測定値と区別できないため、[`Error`] を返す。
/// 呼び出し側はこの分類をログ・カウンタへ反映できる。
///
/// # Errors
///
/// 生値・補正係数の bit 幅、気圧式の除数、整数演算、気圧の出力範囲のいずれかが不正な場合。
pub fn compensate(raw: RawSample, c: &Calibration) -> Result<Reading, Error> {
    if raw.temperature >= 1 << 20 || raw.pressure >= 1 << 20 {
        return Err(Error::RawOutOfRange);
    }
    if !(-2048..=2047).contains(&c.dig_h4) || !(-2048..=2047).contains(&c.dig_h5) {
        return Err(Error::HumidityCoefficientOutOfRange);
    }
    if c.dig_p1 == 0 {
        return Err(Error::ZeroPressureDivisor);
    }

    let adc_t = i128::from(raw.temperature);
    let t1 = i128::from(c.dig_t1);
    let t2 = i128::from(c.dig_t2);
    let t3 = i128::from(c.dig_t3);
    let var1 = (((adc_t >> 3) - (t1 << 1)) * t2) >> 11;
    let delta = (adc_t >> 4) - t1;
    let var2 = (((delta * delta) >> 12) * t3) >> 14;
    let t_fine = var1 + var2;
    let temperature_centi_c =
        i32::try_from((t_fine * 5 + 128) >> 8).map_err(|_| Error::ArithmeticOverflow)?;

    let mut var1 = t_fine - 128_000;
    let mut var2 = var1 * var1 * i128::from(c.dig_p6);
    var2 += (var1 * i128::from(c.dig_p5)) << 17;
    var2 += i128::from(c.dig_p4) << 35;
    var1 = ((var1 * var1 * i128::from(c.dig_p3)) >> 8) + ((var1 * i128::from(c.dig_p2)) << 12);
    var1 = (((1_i128 << 47) + var1) * i128::from(c.dig_p1)) >> 33;
    if var1 == 0 {
        return Err(Error::ZeroPressureDivisor);
    }
    let mut p = 1_048_576 - i128::from(raw.pressure);
    p = (((p << 31) - var2) * 3125) / var1;
    var1 = i128::from(c.dig_p9)
        .checked_mul(p >> 13)
        .and_then(|value| value.checked_mul(p >> 13))
        .ok_or(Error::ArithmeticOverflow)?
        >> 25;
    var2 = (i128::from(c.dig_p8) * p) >> 19;
    p = ((p + var1 + var2) >> 8) + (i128::from(c.dig_p7) << 4);
    let pressure_q24_8_pa = u32::try_from(p).map_err(|_| Error::PressureOutOfRange)?;

    let h_base = t_fine - 76_800;
    let h_first = ((i128::from(raw.humidity) << 14)
        - (i128::from(c.dig_h4) << 20)
        - i128::from(c.dig_h5) * h_base
        + 16_384)
        >> 15;
    let h_second = (((((h_base * i128::from(c.dig_h6)) >> 10)
        * (((h_base * i128::from(c.dig_h3)) >> 11) + 32_768))
        >> 10)
        + 2_097_152)
        .checked_mul(i128::from(c.dig_h2))
        .and_then(|value| value.checked_add(8192))
        .ok_or(Error::ArithmeticOverflow)?
        >> 14;
    let mut h = h_first * h_second;
    h -= ((((h >> 15) * (h >> 15)) >> 7) * i128::from(c.dig_h1)) >> 4;
    let humidity_q22_10_percent =
        u32::try_from(h.clamp(0, 419_430_400) >> 12).map_err(|_| Error::ArithmeticOverflow)?;

    Ok(Reading {
        temperature_centi_c,
        pressure_q24_8_pa,
        humidity_q22_10_percent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_register_boundaries_and_signed_nibbles() {
        // 人工 byte。位置と符号の根拠は Bosch Rev. 1.24 §4.2.2 Table 16。
        let mut tp = [0u8; CALIB_TP_LEN];
        tp[0..4].copy_from_slice(&[0x34, 0x12, 0xfe, 0xff]);
        tp[6..8].copy_from_slice(&[0x78, 0x56]);
        tp[22..24].copy_from_slice(&[0x02, 0x80]);
        tp[25] = 0xa5;
        let h = [0x01, 0x80, 0x5a, 0x80, 0xf0, 0x7f, 0xff];
        let c = decode_calibration(&tp, &h);
        assert_eq!((c.dig_t1, c.dig_t2, c.dig_p1), (0x1234, -2, 0x5678));
        assert_eq!(
            (c.dig_p9, c.dig_h1, c.dig_h2, c.dig_h3),
            (-32766, 0xa5, -32767, 0x5a)
        );
        assert_eq!((c.dig_h4, c.dig_h5, c.dig_h6), (-2048, 2047, -1));

        let h = [0, 0, 0, 0x7f, 0x0f, 0x80, 0];
        let c = decode_calibration(&tp, &h);
        assert_eq!((c.dig_h4, c.dig_h5), (2047, -2048));
    }

    #[test]
    fn raw_burst_boundaries_and_skipped_channel_faults() {
        // 人工 byte。Bosch Rev. 1.24 §5.4.7–5.4.9 Tables 29–31 の配置。
        let bytes = [0x12, 0x34, 0x5f, 0xab, 0xcd, 0xe1, 0x34, 0x56];
        assert_eq!(
            decode_raw(&bytes),
            Ok(RawSample {
                pressure: 0x12345,
                temperature: 0xabcde,
                humidity: 0x3456,
            })
        );
        let mut skipped = bytes;
        skipped[0..3].copy_from_slice(&[0x80, 0x00, 0x00]);
        assert_eq!(decode_raw(&skipped), Err(RegisterFault::SkippedChannel));
        skipped = bytes;
        skipped[3..6].copy_from_slice(&[0x80, 0x00, 0x00]);
        assert_eq!(decode_raw(&skipped), Err(RegisterFault::SkippedChannel));
        skipped = bytes;
        skipped[6..8].copy_from_slice(&[0x80, 0x00]);
        assert_eq!(decode_raw(&skipped), Err(RegisterFault::SkippedChannel));
    }

    #[test]
    fn status_and_identity_faults_follow_register_bits() {
        // Bosch Rev. 1.24 §5.4.1 Table 18、§5.4.4 Table 21。
        assert_eq!(check_identity(CHIP_ID), Ok(()));
        assert_eq!(check_identity(0x00), Err(RegisterFault::WrongChipId));
        assert_eq!(
            check_calibration_ready(0x01),
            Err(RegisterFault::CalibrationNotReady)
        );
        assert_eq!(check_calibration_ready(0x08), Ok(()));
        assert_eq!(
            check_measurement_ready(0x08),
            Err(RegisterFault::MeasurementNotReady)
        );
        assert_eq!(
            check_measurement_ready(0x01),
            Err(RegisterFault::MeasurementNotReady)
        );
        assert_eq!(check_measurement_ready(0x00), Ok(()));
    }

    #[test]
    fn io_faults_separate_timeout_from_other_errors() {
        // 人工的な code。ESP-IDF の数値や実機の error として扱わない。
        assert_eq!(classify_io_error(7, 7), IoFault::Timeout);
        assert_eq!(classify_io_error(8, 7), IoFault::Other);
    }

    // 人工的な入力。メーカーの校正値や実測値として扱わない。
    // 公式 §4.2.3 の式で手計算可能な係数だけを立て、3経路を試す。
    const SIMPLE: Calibration = Calibration {
        dig_t1: 0,
        dig_t2: 5120,
        dig_t3: 0,
        dig_p1: 1,
        dig_p2: 0,
        dig_p3: 0,
        dig_p4: 0,
        dig_p5: 0,
        dig_p6: 0,
        dig_p7: 0,
        dig_p8: 0,
        dig_p9: 0,
        dig_h1: 0,
        dig_h2: 1,
        dig_h3: 0,
        dig_h4: 0,
        dig_h5: 0,
        dig_h6: 0,
    };

    const RAW: RawSample = RawSample {
        temperature: 16_384,
        pressure: 1_048_575,
        humidity: 16_384,
    };

    #[test]
    fn simple_calculation_covers_all_three_channels() {
        assert_eq!(
            compensate(RAW, &SIMPLE),
            Ok(Reading {
                temperature_centi_c: 100,
                pressure_q24_8_pa: 1_600_000,
                humidity_q22_10_percent: 256,
            })
        );
    }

    #[test]
    fn datasheet_output_examples_have_documented_units() {
        // Bosch §4.2.3 の出力例。raw 値と係数の一式ではなく、単位の例である。
        let example = Reading {
            temperature_centi_c: 5123,
            pressure_q24_8_pa: 24_674_867,
            humidity_q22_10_percent: 47_445,
        };
        assert!((example.temperature_celsius() - 51.23).abs() < 0.001);
        assert!((example.pressure_pa() - 96_386.2).abs() < 0.01);
        assert!((example.humidity_percent() - 46.333).abs() < 0.001);
    }

    #[test]
    fn rejects_raw_values_outside_the_datasheet_bit_width() {
        assert_eq!(
            compensate(
                RawSample {
                    temperature: 1 << 20,
                    ..RAW
                },
                &SIMPLE
            ),
            Err(Error::RawOutOfRange)
        );
        assert_eq!(
            compensate(
                RawSample {
                    pressure: 1 << 20,
                    ..RAW
                },
                &SIMPLE
            ),
            Err(Error::RawOutOfRange)
        );
    }

    #[test]
    fn rejects_zero_pressure_divisor_instead_of_returning_a_plausible_zero() {
        let c = Calibration {
            dig_p1: 0,
            ..SIMPLE
        };
        assert_eq!(compensate(RAW, &c), Err(Error::ZeroPressureDivisor));
    }

    #[test]
    fn rejects_humidity_coefficients_before_sign_extension_errors_propagate() {
        let c = Calibration {
            dig_h4: 2048,
            ..SIMPLE
        };
        assert_eq!(
            compensate(RAW, &c),
            Err(Error::HumidityCoefficientOutOfRange)
        );
    }

    #[test]
    fn humidity_is_clamped_to_the_documented_range() {
        let raw = RawSample {
            humidity: u16::MAX,
            ..RAW
        };
        let c = Calibration {
            dig_h2: i16::MAX,
            ..SIMPLE
        };
        assert_eq!(
            compensate(raw, &c).unwrap().humidity_q22_10_percent,
            100 * 1024
        );
    }

    #[test]
    fn each_sample_uses_its_own_temperature_for_pressure_and_humidity() {
        let c = Calibration {
            dig_p2: 20_000,
            dig_h5: 2047,
            ..SIMPLE
        };
        let another = RawSample {
            temperature: 32_768,
            ..RAW
        };
        let first = compensate(RAW, &c).unwrap();
        let second = compensate(another, &c).unwrap();
        assert_ne!(first.pressure_q24_8_pa, second.pressure_q24_8_pa);
        assert_ne!(
            first.humidity_q22_10_percent,
            second.humidity_q22_10_percent
        );
        assert_eq!(compensate(RAW, &c), Ok(first));
    }

    #[test]
    fn signed_pressure_and_humidity_coefficients_change_the_result() {
        let positive = Calibration {
            dig_p2: 20_000,
            dig_h4: 1,
            ..SIMPLE
        };
        let negative = Calibration {
            dig_p2: -20_000,
            dig_h4: -1,
            ..SIMPLE
        };
        let a = compensate(RAW, &positive).unwrap();
        let b = compensate(RAW, &negative).unwrap();
        assert_ne!(a.pressure_q24_8_pa, b.pressure_q24_8_pa);
        assert!(a.humidity_q22_10_percent < b.humidity_q22_10_percent);
    }

    /// 係数の組。`dig_p1`・`dig_h4`・`dig_h5` だけが EXP-031 の実機の記録値
    /// （`dig_p1=37064`、`dig_h4=391`、`dig_h5=50`）。**他の係数は記録に無いので人工値**であり、
    /// この組から出る温度・気圧・湿度は実機の値ではない。
    const EXP031_P1_H4_H5: Calibration = Calibration {
        dig_t1: 28_000,
        dig_t2: 26_000,
        dig_t3: -1_000,
        dig_p1: 37_064,
        dig_p2: -10_500,
        dig_p3: 3_024,
        dig_p4: 8_000,
        dig_p5: -100,
        dig_p6: -7,
        dig_p7: 9_900,
        dig_p8: -10_230,
        dig_p9: 4_285,
        dig_h1: 75,
        dig_h2: 370,
        dig_h3: 0,
        dig_h4: 391,
        dig_h5: 50,
        dig_h6: 30,
    };

    /// 上と同じ実機の3値に、`dig_h3`・`dig_h6`（湿度式の入れ子の項）が 0 でない人工値を足した組。
    const WITH_H3_H6: Calibration = Calibration {
        dig_t1: 27_504,
        dig_t2: 26_435,
        dig_t3: 50,
        dig_p2: -10_685,
        dig_p4: 2_855,
        dig_p5: 140,
        dig_p7: 15_500,
        dig_p8: -14_600,
        dig_p9: 6_000,
        dig_h2: 360,
        dig_h3: 40,
        dig_h6: -20,
        ..EXP031_P1_H4_H5
    };

    /// 独立に求めた期待値。1 行 = (係数, raw 値, 温度 0.01 °C, 気圧 Q24.8 Pa, 湿度 Q22.10 %RH,
    /// Bosch driver の 0.01 Pa 単位の気圧, Bosch driver の湿度 Q22.10)。
    ///
    /// **出し方**（どちらも Python の任意精度整数。この crate の Rust 実装とは別の実装）:
    ///
    /// 1. 前 5 列: Bosch BME280 Data sheet Rev. 1.24 §4.2.3 の 32 bit 温度・64 bit 気圧・32 bit 湿度の
    ///    整数補償式を、`>>`（負数は床方向）で Python に書いて計算した。**datasheet 本文はこの追加の
    ///    作業では開き直していない。**式の構造は、下の 2. の driver と、各列の差が負の中間値の
    ///    丸めだけに収まることで確かめた。
    /// 2. 後 2 列と温度の検算: Bosch Sensortec の公開 driver
    ///    （<https://github.com/boschsensortec/BME280_SensorAPI> の `bme280.c` v3.5.1 2020-12-17、
    ///    BSD-3-Clause）の、`BME280_DOUBLE_ENABLE`・`BME280_32BIT_ENABLE` を定義しない既定の整数版
    ///    `compensate_temperature`・`compensate_pressure`（64 bit）・`compensate_humidity` を 1 行ずつ
    ///    写して計算した。driver は `>>` の代わりに `/`（0 方向への切り捨て）を使うので、負の中間値
    ///    （`dig_t3 < 0`、`dig_p8 < 0`、`dig_h6 < 0` など）で datasheet 式と最下位 bit がずれうる。
    ///    そのため driver の値は完全一致でなく、**温度は一致、気圧は 0.05 Pa 以内、湿度は 2 LSB 以内**
    ///    で比べる（下の test）。driver は気圧を 0.01 Pa 単位、−40..85 °C・0..100 %RH に clamp する
    ///    ので、入力は clamp に当たらず、int32 の途中値も溢れない範囲にしてある。
    ///
    /// 参考（test には含めない）: driver の `BME280_DOUBLE_ENABLE` 版は、`dig_h3 == 0` の組では上の
    /// 値と温度 0.01 °C・気圧 0.1 Pa・湿度 0.01 %RH 以内で一致した。`dig_h3 != 0` では湿度が約
    /// 2.7 %RH ずれる（その版の湿度式は `var5` を 1 回余分に掛けており、同 driver の整数版と式が
    /// 揃わない）ので使わなかった。
    ///
    /// 入力の raw 値のうち最初の 3 つは EXP-031 が記録した実機の 3 回の raw 値。
    #[allow(clippy::type_complexity)]
    const GOLDEN: &[(&Calibration, RawSample, i32, u32, u32, u32, u32)] = &[
        (
            &EXP031_P1_H4_H5,
            raw(537_744, 303_840, 36_195),
            2_772,
            26_725_823,
            64_683,
            10_439_778,
            64_683,
        ),
        (
            &EXP031_P1_H4_H5,
            raw(537_776, 303_712, 36_142),
            2_773,
            26_731_824,
            64_381,
            10_442_121,
            64_381,
        ),
        (
            &EXP031_P1_H4_H5,
            raw(537_776, 303_760, 36_145),
            2_773,
            26_729_729,
            64_392,
            10_441_303,
            64_392,
        ),
        (
            &EXP031_P1_H4_H5,
            raw(500_000, 400_000, 30_000),
            1_609,
            22_113_231,
            28_630,
            8_637_982,
            28_630,
        ),
        (
            &WITH_H3_H6,
            raw(537_744, 303_840, 36_195),
            3_079,
            30_477_164,
            59_394,
            11_905_142,
            59_395,
        ),
        (
            &WITH_H3_H6,
            raw(480_000, 420_000, 30_000),
            1_259,
            24_666_675,
            28_194,
            9_635_419,
            28_194,
        ),
        (
            &WITH_H3_H6,
            raw(530_000, 330_000, 40_000),
            2_835,
            29_212_921,
            80_385,
            11_411_296,
            80_387,
        ),
    ];

    const fn raw(temperature: u32, pressure: u32, humidity: u16) -> RawSample {
        RawSample {
            temperature,
            pressure,
            humidity,
        }
    }

    #[test]
    fn compensation_matches_independent_calculations() {
        for &(c, sample, temperature, pressure, humidity, bosch_centi_pa, bosch_humidity) in GOLDEN
        {
            let r = compensate(sample, c).unwrap();
            assert_eq!(r.temperature_centi_c, temperature, "{sample:?}");
            assert_eq!(r.pressure_q24_8_pa, pressure, "{sample:?}");
            assert_eq!(r.humidity_q22_10_percent, humidity, "{sample:?}");
            // Bosch driver との比較。気圧は Q24.8 を driver の 0.01 Pa 単位へ換算する。
            let centi_pa = i64::from(r.pressure_q24_8_pa) * 100 / 256;
            assert!(
                (centi_pa - i64::from(bosch_centi_pa)).abs() <= 5,
                "{sample:?}"
            );
            let h = i64::from(r.humidity_q22_10_percent) - i64::from(bosch_humidity);
            assert!(h.abs() <= 2, "{sample:?}");
        }
    }
}
