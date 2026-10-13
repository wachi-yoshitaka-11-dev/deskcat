//! `ENV-01` の BME280 生値と補正係数から温度・気圧・相対湿度を計算する。
//!
//! I2C、レジスタの読み出しと byte の組み立て、測定設定は firmware の責務。
//! この crate は読み出し済みの数値だけを受け取り、内部状態を持たない。
//!
//! 出所: [Bosch Sensortec BME280 Data sheet, BST-BME280-DS001-24,
//! Revision 1.24 (February 2024)](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf)。
//! §4.2.2 Table 16 が補正係数の符号・bit 幅、§4.2.3 (pp.25–26) が整数補償式と
//! 出力形式、§5.4.7–5.4.9 が生値の形式を定める。

#![no_std]

/// BME280 の読み出し済み補正係数。各 field の型はデータシート §4.2.2 Table 16 に従う。
///
/// `dig_h4` と `dig_h5` は 12 bit の符号付き値として firmware 側で組み立て、
/// `i16` へ符号拡張して渡す。ここでレジスタ byte を解釈しない。
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
}
