//! `ACCEL-01`（ADXL345）の連続測定の設定、生データの復号、静止時の統計
//! （[Issue #15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)）。
//!
//! I2Cは知らない。レジスタへ書く(address, 値)の列と、読んだbyteの読み方と、集計だけを持ち、
//! I2Cを動かすのは`firmware/esp32`の`crate::accel`・`crate::accel_sample`である（hostでtestできるように分けた）。
//! tap検出の設定は`deskcat-tap`が持つ。**`BW_RATE`・`POWER_CTL`・`INT_ENABLE`・`INT_SOURCE`のaddressと、`BW_RATE_100_HZ`・`POWER_CTL_MEASURE`の値は、
//! `deskcat-tap`の定数をそのまま再利用する**（二重に持たない。出所は同crateのcrate doc）。
//!
//! # 出所
//!
//! Analog Devices ADXL345 Data Sheet **Rev. E**（`datasheet.octopart.com`が配る版）を、PDFから文字を抽出して読んだ。
//! register map（`DATA_FORMAT`・`DATAX0`〜`DATAZ1`・`FIFO_CTL`・`FIFO_STATUS`。`INT_SOURCE`のD0＝Overrunの読み方も）、
//! Table 1（Sensitivity・Scale Factor・0 g Output）、Table 21（g range）、`FIFO`の節による。
//! **`docs/hardware/sensor-datasheet-notes.md`が正とするRev. Gは、ブラウザで取得済みだが、CLI（curl）からは取得できず、この crate の register は Rev. G で読んでいない。**
//! 同文書がRev. Gで確認済みとして持つのは`BW_RATE`・`DATA_FORMAT`・Device IDなどで、`FIFO_CTL`・`FIFO_STATUS`・`DATAX0`のaddressと`INT_SOURCE`のOverrun bitは載っていない。
//! Rev. E と Rev. G で、これらの記述が同じかは、独立に確認していない。
//!
//! # 値の扱い
//!
//! - **範囲は±2 g、`FULL_RES`＝1**（`DATA_FORMAT`＝`0x08`）。±2 gでは`FULL_RES`の10 bitと10 bit固定は同じ分解能で、
//!   sensitivity は typ 256 LSB/g（min 230／max 282）、scale factor は typ 3.9 mg/LSB（Table 1）。
//!   **mgへの換算は典型値256 LSB/gによる近似であり、個体の値ではない**（min〜maxで約±10 %の幅がある）。
//!   静止姿勢の重力の読み（約256 LSB）との比で、個体の感度の目安が得られるが、基板の傾きと分離できない。
//! - 出力は2の補数・右詰め・符号拡張（`Justify`＝0。`DATAx0`が下位byte）。±2 g・10 bitでは`-512..=511`に収まる。
//!   範囲外は[`SampleFault::OutOfRange`]にする。
//! - 0 g Output は X・Y が±150 mg、Z が±250 mg（Table 1）。静止の読みからこの値は直接出ない
//!   （重力が乗るため）。**offsetの主張は、姿勢が分かる向きでの読みに限る。**

#![no_std]

// `BW_RATE`（`0x2C`）・`POWER_CTL`（`0x2D`）・`INT_ENABLE`（`0x2E`）のaddressと、`BW_RATE`の100 Hz（`0x0A`）、`POWER_CTL`の`Measure`（D3＝`0x08`）は、
// `deskcat-tap`の定数を再利用する。`INT_SOURCE`（`0x30`）のaddressも同じ（`deskcat_tap::REG_INT_SOURCE`）。
// この crate が持つのは、D0の`Overrun`の読み方（[`is_overrun`]）だけである。
pub use deskcat_tap::{
    BW_RATE_100_HZ, POWER_CTL_MEASURE, REG_BW_RATE, REG_INT_ENABLE, REG_POWER_CTL,
};
/// `DATA_FORMAT`（address `0x31`）。reset値`0x00`。
pub const REG_DATA_FORMAT: u8 = 0x31;
/// `DATAX0`（address `0x32`）。ここから6 byte（X0,X1,Y0,Y1,Z0,Z1）。
pub const REG_DATAX0: u8 = 0x32;
/// `FIFO_CTL`（address `0x38`）。D7:D6がFIFO mode。
pub const REG_FIFO_CTL: u8 = 0x38;
/// `FIFO_STATUS`（address `0x39`）。D5:D0がentries。
pub const REG_FIFO_STATUS: u8 = 0x39;

/// 出力data rate（Hz）。[`BW_RATE_100_HZ`]に対応する。
pub const ODR_HZ: u32 = 100;
/// `DATA_FORMAT`の値。`FULL_RES`（D3）＝1、`Justify`（D2）＝0、`Range`（D1:D0）＝00（±2 g）。
pub const DATA_FORMAT_FULL_RES_2G: u8 = 0x08;
/// `FIFO_CTL`の値。Bypass（D7:D6＝00）。FIFOを空にするために先に書く。
pub const FIFO_CTL_BYPASS: u8 = 0x00;
/// `FIFO_CTL`の値。Stream（D7:D6＝10。最後の32件を保持）。`Samples`（D4:D0）は0。watermarkは使わない。
pub const FIFO_CTL_STREAM: u8 = 0x80;
/// `POWER_CTL`の値。standby（`Measure`＝0）。
pub const POWER_CTL_STANDBY: u8 = 0x00;
/// `INT_ENABLE`の値。全部無効。**tapの機能も止まる。**前のbuild（`tap-21`）が書いた値は電源が落ちるまで残るため、明示して書く。
pub const INT_ENABLE_NONE: u8 = 0x00;

/// 測定を始めるために書く(address, 値)の列。**この順に書く。**standbyで設定し、FIFOを空にしてからStreamにし、最後に測定を始める。
pub const SETUP: [(u8, u8); 7] = [
    (REG_POWER_CTL, POWER_CTL_STANDBY),
    (REG_INT_ENABLE, INT_ENABLE_NONE),
    (REG_BW_RATE, BW_RATE_100_HZ),
    (REG_DATA_FORMAT, DATA_FORMAT_FULL_RES_2G),
    (REG_FIFO_CTL, FIFO_CTL_BYPASS),
    (REG_FIFO_CTL, FIFO_CTL_STREAM),
    (REG_POWER_CTL, POWER_CTL_MEASURE),
];

/// 典型のsensitivity（LSB/g。Table 1 の typ）。
pub const TYPICAL_LSB_PER_G: i32 = 256;
/// ±2 g・10 bitの最小値と最大値（符号つき）。
pub const RAW_MIN: i16 = -512;
/// 同上の最大値。
pub const RAW_MAX: i16 = 511;

/// 1回の測定値（LSB。各軸）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// X軸。
    pub x: i16,
    /// Y軸。
    pub y: i16,
    /// Z軸。
    pub z: i16,
}

/// 復号できなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFault {
    /// ±2 g・10 bitで表せない値（`-512..=511`の外）。配線・設定・bus上の化けのいずれかの兆候だが、これだけでは区別できない。
    OutOfRange,
}

/// `DATAX0`から読んだ6 byteを復号する。
///
/// # Errors
///
/// いずれかの軸が`-512..=511`の外なら[`SampleFault::OutOfRange`]。
pub fn decode_sample(bytes: &[u8; 6]) -> Result<Sample, SampleFault> {
    let axis = |lo: u8, hi: u8| i16::from_le_bytes([lo, hi]);
    let s = Sample {
        x: axis(bytes[0], bytes[1]),
        y: axis(bytes[2], bytes[3]),
        z: axis(bytes[4], bytes[5]),
    };
    for v in [s.x, s.y, s.z] {
        if !(RAW_MIN..=RAW_MAX).contains(&v) {
            return Err(SampleFault::OutOfRange);
        }
    }
    Ok(s)
}

/// `FIFO_STATUS`のentries（D5:D0。最大33）。
#[must_use]
pub const fn fifo_entries(fifo_status: u8) -> u8 {
    fifo_status & 0x3F
}

/// `INT_SOURCE`のOverrun（D0）。Stream modeではFIFOが満杯になった（古い件が捨てられた）ことを示す。
#[must_use]
pub const fn is_overrun(int_source: u8) -> bool {
    int_source & 0x01 != 0
}

/// LSBを、典型のsensitivity（256 LSB/g）でmgへ換算する（四捨五入しない。0へ向けて切り捨て）。
#[must_use]
pub const fn lsb_to_mg(lsb: i32) -> i32 {
    lsb * 1000 / TYPICAL_LSB_PER_G
}

/// 窓の集計の結果（1軸）。平均と標準偏差はmLSB（LSBの1/1000）、最小と最大はLSB。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisSummary {
    /// 平均。
    pub mean_mlsb: i64,
    /// 標本標準偏差（n−1）。nが2未満なら0。
    pub stddev_mlsb: i64,
    /// 最小（LSB）。
    pub min: i16,
    /// 最大（LSB）。
    pub max: i16,
}

/// 3軸の和と二乗和を溜める。外れ値を捨てない（捨てる基準を持たない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    n: u32,
    sum: [i64; 3],
    sumsq: [i64; 3],
    min: [i16; 3],
    max: [i16; 3],
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl Stats {
    /// 空の集計。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            n: 0,
            sum: [0; 3],
            sumsq: [0; 3],
            min: [i16::MAX; 3],
            max: [i16::MIN; 3],
        }
    }

    /// 件数。
    #[must_use]
    pub const fn count(&self) -> u32 {
        self.n
    }

    /// 1件足す。
    pub fn push(&mut self, s: Sample) {
        for (i, v) in [s.x, s.y, s.z].into_iter().enumerate() {
            let w = i64::from(v);
            self.sum[i] += w;
            self.sumsq[i] += w * w;
            self.min[i] = self.min[i].min(v);
            self.max[i] = self.max[i].max(v);
        }
        self.n = self.n.saturating_add(1);
    }

    /// 軸（0＝X、1＝Y、2＝Z）の集計。1件も無ければ`None`。
    #[must_use]
    pub fn axis(&self, axis: usize) -> Option<AxisSummary> {
        if self.n == 0 || axis > 2 {
            return None;
        }
        let n = i128::from(self.n);
        let sum = i128::from(self.sum[axis]);
        let mean_mlsb = i64::try_from(sum * 1000 / n).unwrap_or(i64::MAX);
        let stddev_mlsb = if self.n < 2 {
            0
        } else {
            // 分散 = (n*Σx² − (Σx)²) / (n*(n−1))。mLSB²にするため 1_000_000 を掛ける。
            let num = n * i128::from(self.sumsq[axis]) - sum * sum;
            let var_m2 = u128::try_from(num.max(0) * 1_000_000 / (n * (n - 1))).unwrap_or(0);
            i64::try_from(isqrt(var_m2)).unwrap_or(i64::MAX)
        };
        Some(AxisSummary {
            mean_mlsb,
            stddev_mlsb,
            min: self.min[axis],
            max: self.max[axis],
        })
    }
}

/// 3軸の平均ベクトルの大きさ（mLSB）。静止なら重力1 g＝約256 LSBになるはずだが、
/// **これは個体の感度と0 g offsetの合成であり、どちらか一方の根拠にならない。**
#[must_use]
pub fn mean_magnitude_mlsb(stats: &Stats) -> Option<u64> {
    let x = i128::from(stats.axis(0)?.mean_mlsb);
    let y = i128::from(stats.axis(1)?.mean_mlsb);
    let z = i128::from(stats.axis(2)?.mean_mlsb);
    let sq = u128::try_from(x * x + y * y + z * z).unwrap_or(0);
    Some(u64::try_from(isqrt(sq)).unwrap_or(u64::MAX))
}

/// 整数の平方根（floor）。
fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = u128::midpoint(x, n / x);
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_order_standby_first_measure_last() {
        assert_eq!(SETUP[0], (REG_POWER_CTL, 0x00));
        assert_eq!(SETUP[SETUP.len() - 1], (REG_POWER_CTL, 0x08));
        // FIFOはBypassを書いてからStreamにする。
        assert_eq!(SETUP[4], (REG_FIFO_CTL, 0x00));
        assert_eq!(SETUP[5], (REG_FIFO_CTL, 0x80));
        // tapを止める。
        assert!(SETUP.contains(&(REG_INT_ENABLE, 0x00)));
    }

    #[test]
    fn decode_positive_and_negative() {
        // X=+256 (0x0100), Y=-1 (0xFFFF), Z=-512 (0xFE00)
        let s = decode_sample(&[0x00, 0x01, 0xFF, 0xFF, 0x00, 0xFE]).unwrap();
        assert_eq!(
            s,
            Sample {
                x: 256,
                y: -1,
                z: -512
            }
        );
    }

    #[test]
    fn decode_range_edges() {
        // +511 は通り、+512 と -513 は落ちる。
        assert!(decode_sample(&[0xFF, 0x01, 0, 0, 0, 0]).is_ok());
        assert_eq!(
            decode_sample(&[0x00, 0x02, 0, 0, 0, 0]),
            Err(SampleFault::OutOfRange)
        );
        assert_eq!(
            decode_sample(&[0, 0, 0xFF, 0xFD, 0, 0]),
            Err(SampleFault::OutOfRange)
        );
        // 16 bit全体の符号つきの端も範囲外。
        assert_eq!(
            decode_sample(&[0, 0, 0, 0, 0xFF, 0x7F]),
            Err(SampleFault::OutOfRange)
        );
    }

    #[test]
    fn fifo_and_overrun_bits() {
        assert_eq!(fifo_entries(0b1100_0000 | 0x21), 33);
        assert_eq!(fifo_entries(0), 0);
        assert!(is_overrun(0x01));
        assert!(!is_overrun(0x80));
    }

    #[test]
    fn lsb_to_mg_typical() {
        assert_eq!(lsb_to_mg(256), 1000);
        assert_eq!(lsb_to_mg(-256), -1000);
        assert_eq!(lsb_to_mg(0), 0);
    }

    #[test]
    fn stats_mean_and_stddev() {
        let mut st = Stats::new();
        assert!(st.axis(0).is_none());
        // X: 1,2,3,4 → mean 2.5, 標本分散 = 5/3 → σ=1.2909944
        for v in [1i16, 2, 3, 4] {
            st.push(Sample {
                x: v,
                y: 10,
                z: -256,
            });
        }
        let x = st.axis(0).unwrap();
        assert_eq!(x.mean_mlsb, 2500);
        assert_eq!(x.stddev_mlsb, 1290); // floor(1290.99)
        assert_eq!((x.min, x.max), (1, 4));
        let y = st.axis(1).unwrap();
        assert_eq!((y.mean_mlsb, y.stddev_mlsb), (10_000, 0));
        let z = st.axis(2).unwrap();
        assert_eq!(z.mean_mlsb, -256_000);
        assert!(st.axis(3).is_none());
        assert_eq!(st.count(), 4);
    }

    #[test]
    fn stats_single_sample_has_zero_stddev() {
        let mut st = Stats::new();
        st.push(Sample { x: 5, y: 0, z: 256 });
        assert_eq!(st.axis(0).unwrap().stddev_mlsb, 0);
    }

    #[test]
    fn magnitude_of_flat_board() {
        let mut st = Stats::new();
        for _ in 0..10 {
            st.push(Sample { x: 0, y: 0, z: 256 });
        }
        assert_eq!(mean_magnitude_mlsb(&st), Some(256_000));
        assert_eq!(mean_magnitude_mlsb(&Stats::new()), None);
    }

    #[test]
    fn isqrt_exact_and_floor() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(1_000_000_000_000), 1_000_000);
    }
}
