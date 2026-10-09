//! `ACCEL-01`（ADXL345）のtap検出の設定と、`tapped`（Protocol §4.3）を送ってよいかの判定（Issue #21、F1）。
//!
//! このcrateはI2Cを知らない。レジスタへ書く(address, 値)の列と、`INT_SOURCE`のbyteの読み方と、
//! 送出の条件を返すだけであり、I2Cを動かすのは`firmware/esp32`の`crate::accel`である（hostでtestできるように分けた）。
//!
//! # 出所
//!
//! レジスタの`address`・`bit`・`scale factor`の出所は、**Analog Devices `ADXL345` Data Sheet Rev. 0**（`SparkFun`が
//! hostする版。`docs/hardware/sensor-datasheet-notes.md`が既に引用している）である。Table 16 Register Map（p.14）、
//! `THRESH_TAP`・`DUR`（p.15）、`BW_RATE`（Table 16、p.14）、`TAP_AXES`・`POWER_CTL`（p.16）、`INT_ENABLE`・`INT_SOURCE`（p.17）、
//! tap検出の節（p.20）による。**このcrateが使うレジスタの記述が、同文書が正とするRev. Gと同じかは、独立に確認していない**
//! （Rev. GをCLIから取得できなかった）。
//!
//! # 値の扱い
//!
//! 閾値・持続時間・最小間隔は**触覚の感度であり、安全要件の5項目に効かない**（外れても誤検出や検出漏れになるだけで、
//! 何も壊れない）。そのため一般値で始め、**暫定である**。実機で、指で軽く叩いて詰める（検出されなければ下げ、誤検出が多ければ上げる）。
//! 値は定数にして変えやすくしてある。

/// `THRESH_TAP`（address 0x1D）。tapの閾値。unsigned、62.5 mg/LSB（p.15）。
pub const REG_THRESH_TAP: u8 = 0x1D;
/// `DUR`（address 0x21）。閾値を超えていてよい最長の時間。625 µs/LSB。**0はtap機能を無効にする**（p.15）。
pub const REG_DUR: u8 = 0x21;
/// `TAP_AXES`（address 0x2A）。D2〜D0がX／Y／Zの有効（p.16）。
pub const REG_TAP_AXES: u8 = 0x2A;
/// `BW_RATE`（address 0x2C）。データレートと省電力modeの制御。reset値は`0b0000_1010`（`0x0A`。Table 16、p.14）。
pub const REG_BW_RATE: u8 = 0x2C;
/// `POWER_CTL`（address 0x2D）。D3が`Measure`（1で測定mode。電源投入時はstandby）（p.16）。
pub const REG_POWER_CTL: u8 = 0x2D;
/// `INT_ENABLE`（address 0x2E）。**tapの機能そのものの有効化を兼ねる**（`DATA_READY`等と違い「常に有効」ではない。p.17）。
pub const REG_INT_ENABLE: u8 = 0x2E;
/// `INT_SOURCE`（address 0x30）。読むとtapのbitが消える（p.17）。
pub const REG_INT_SOURCE: u8 = 0x30;

/// `BW_RATE`の値。`0x0A`は**100 Hz**に当たる（`docs/hardware/sensor-datasheet-notes.md`の`Output data rate`行が、Rev. Gの
/// Table 19 p.23でreset値`00001010`＝100 Hzと記録している。Rev. 0でもp.20が「100 Hz以上にするには`0x0A`以上を書く」と述べる）。reset値と同じだが、
/// ESP32だけが再起動してADXL345の電源が落ちない場合に、前の設定へ依らないよう明示して書く。
/// tapは未filterのデータで検出される（p.20）。`firmware/esp32/src/tap.rs`の読み出し間隔（20 ms）は、このデータレート（10 ms）の2倍である。
pub const BW_RATE_100_HZ: u8 = 0x0A;

/// `THRESH_TAP`の出発点。0x18＝24 × 62.5 mg＝**1.5 g**。**暫定の一般値である。**
///
/// **経緯。**当初の出発点は1 g（0x10）だった。決定の記録は
/// [Issue #21のcomment](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21#issuecomment-6075036433)にある。
///
/// **重力との干渉。**tapは**未filterのデータ**で検出される（p.20）ため、読みに重力の約1 gが含まれる。鉛直を向いた軸（基板を
/// 水平に置けばZ）の静止時の読みが閾値に近いと、noiseとoffsetで閾値を跨ぎ続け、何もしていないのに単tapが出うる。
/// 閾値は、そのため1 gより十分上に置く（データシートの出発点が「3 gを超える値から」（p.20。double tapを含む一般の出発点）
/// なのも、同じ理由と読める。**これは推論であり、実機では確かめていない**）。ただし3 gは、指で基板を軽く叩く衝撃には硬い見込みで、
/// 重力に0.5 gの余裕を取った1.5 gから始める。デモのジェスチャは「撫でる」ではなく「指で、ADXL345のモジュールを軽く叩く」である。
///
/// **g-range。**`DATA_FORMAT`（address 0x31）は**書かない**。reset値は`0x00`で、範囲は**±2 g**（10 bit。Table 16 p.14、range bitsはp.17）である。
/// 範囲を超える入力は飽和するため、**閾値が範囲の上限（2 g）を超えると届かない見込み**である（推論であり、データシートからは確かめていない）。
/// 上の「3 g超」の出発点は、より広い範囲を前提にした値と読める。このcrateは範囲を広げない（必要になったら`DATA_FORMAT`を書く）ため、
/// **閾値は2 g以下で調整する**（`0x20`が上限。testが固定する）。
///
/// **実機で詰める。**届かなければ`0x14`（1.25 g）まで下げる。静止中に`tapped`が出続けるなら上げる（上限は上のとおり2 g。`tap_detected`のlogの
/// `int_source`で、どの状態で立ったかを見る）。変えるたびにflashが要るため、最初の値を適当に選ばない。
/// `TAP_AXES`で軸を絞る案（基板の向きが決まってから）は、今は採らない。
pub const THRESH_TAP: u8 = 0x18;
/// `DUR`の出発点。0x10＝16 × 625 µs＝**10 ms**。**暫定の一般値である。**データシートはDURの例の値を挙げていない。
pub const DUR: u8 = 0x10;
/// `TAP_AXES`の出発点。0x07＝X・Y・Zの3軸。double tapの`Suppress`（D3）は使わない。
pub const TAP_AXES: u8 = 0x07;
/// `INT_ENABLE`の値。D6（`SINGLE_TAP`）だけを有効にする。double tapは使わない。
/// 割り込みの出力pinへ配線しない（`INT_SOURCE`をI2Cで読む）。
pub const INT_ENABLE_SINGLE_TAP: u8 = 1 << 6;
/// `POWER_CTL`の値。`Measure`（D3）だけを立てる。
pub const POWER_CTL_MEASURE: u8 = 1 << 3;

/// 検出を始めるために書く(address, 値)の列。**この順に書く。**データレート、閾値・持続時間・軸を先に決め、
/// 割り込みの有効化の後、最後に測定を始める（p.17の「割り込みは出力を有効にする前に設定する」に沿う）。
pub const SETUP: [(u8, u8); 6] = [
    (REG_BW_RATE, BW_RATE_100_HZ),
    (REG_THRESH_TAP, THRESH_TAP),
    (REG_DUR, DUR),
    (REG_TAP_AXES, TAP_AXES),
    (REG_INT_ENABLE, INT_ENABLE_SINGLE_TAP),
    (REG_POWER_CTL, POWER_CTL_MEASURE),
];

/// `INT_SOURCE`のD6（`SINGLE_TAP`）。
pub const INT_SOURCE_SINGLE_TAP: u8 = 1 << 6;

/// `INT_SOURCE`のbyteが、単tapを示すか。
#[must_use]
pub const fn is_single_tap(int_source: u8) -> bool {
    int_source & INT_SOURCE_SINGLE_TAP != 0
}

/// `tapped`を続けて送る最小の間隔（ms）。前の送出からこれ未満のtapは送らない。
/// **暫定の一般値である**（安全要件の5項目に効かない）。§8.2の送出の上限（`PROTO-TBD-012`）が決まれば置き換える。
pub const MIN_TAP_INTERVAL_MS: u64 = 500;

/// tapを見つけたときの扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// `tapped`を送る。
    Send,
    /// bootのACKを受けておらず、Piのsessionが確立していない。捨てる。
    DropNoSession,
    /// 前の`tapped`から[`MIN_TAP_INTERVAL_MS`]未満である。捨てる。
    DropTooSoon,
}

/// tapの扱いの内訳。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TapCounters {
    /// 送ると決めた数（`Send`を返した数。wireへ書けたかは数えない）。
    pub sent: u32,
    /// sessionが確立していない間（確立前と、終了した後を含む）に捨てた数。
    pub dropped_no_session: u32,
    /// 間隔が短くて捨てた数。
    pub dropped_too_soon: u32,
}

/// `tapped`を送ってよいかを決める。
#[derive(Debug, Clone, Default)]
pub struct TapFilter {
    last_sent_ms: Option<u64>,
    counters: TapCounters,
}

impl TapFilter {
    /// まだ何も送っていない状態で作る。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_sent_ms: None,
            counters: TapCounters {
                sent: 0,
                dropped_no_session: 0,
                dropped_too_soon: 0,
            },
        }
    }

    /// tapを1回見つけたときに呼ぶ。`now_ms`はuptime、`session_established`は`boot`のACKを受けたか。
    /// `Send`を返したときだけ、送った時刻として記録する。
    pub fn on_tap(&mut self, now_ms: u64, session_established: bool) -> Decision {
        if !session_established {
            self.counters.dropped_no_session = self.counters.dropped_no_session.saturating_add(1);
            return Decision::DropNoSession;
        }
        if self
            .last_sent_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < MIN_TAP_INTERVAL_MS)
        {
            self.counters.dropped_too_soon = self.counters.dropped_too_soon.saturating_add(1);
            return Decision::DropTooSoon;
        }
        self.last_sent_ms = Some(now_ms);
        self.counters.sent = self.counters.sent.saturating_add(1);
        Decision::Send
    }

    /// これまでの内訳。
    #[must_use]
    pub const fn counters(&self) -> TapCounters {
        self.counters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_writes_the_documented_registers_in_order() {
        assert_eq!(
            SETUP,
            [
                (0x2C, 0x0A),
                (0x1D, 0x18),
                (0x21, 0x10),
                (0x2A, 0x07),
                (0x2E, 0x40),
                (0x2D, 0x08),
            ]
        );
        // 測定を始める書き込みが最後である。
        assert_eq!(SETUP[SETUP.len() - 1].0, REG_POWER_CTL);
    }

    #[test]
    fn the_threshold_is_well_above_gravity_because_tap_uses_unfiltered_data() {
        // 62.5 mg/LSB（p.15）。重力の約1 g（1000 mg）に、少なくとも0.5 gの余裕を取る。
        let threshold_mg = u32::from(THRESH_TAP) * 625 / 10;
        assert!(threshold_mg >= 1500, "{threshold_mg} mg");
        // `DATA_FORMAT`を書かないので、範囲はreset値の±2 gである。閾値はその上限（2 g）以下でなければ届かない見込み。
        assert!(threshold_mg <= 2000, "{threshold_mg} mg");
    }

    #[test]
    fn dur_is_never_zero_because_zero_disables_tap_detection() {
        assert_ne!(DUR, 0);
        assert_ne!(
            THRESH_TAP, 0,
            "0はtap/double tapで予期しない動作になる（p.15）"
        );
    }

    #[test]
    fn single_tap_is_bit_6_of_int_source() {
        assert!(is_single_tap(0x40));
        assert!(is_single_tap(0xFF));
        assert!(!is_single_tap(0x00));
        // DATA_READY（D7）だけでは単tapではない。reset値の0x02（p.14）も違う。
        assert!(!is_single_tap(0x80));
        assert!(!is_single_tap(0x02));
    }

    #[test]
    fn taps_before_the_session_is_established_are_dropped_and_counted() {
        let mut f = TapFilter::new();
        assert_eq!(f.on_tap(100, false), Decision::DropNoSession);
        assert_eq!(f.on_tap(5_000, false), Decision::DropNoSession);
        assert_eq!(f.counters().dropped_no_session, 2);
        assert_eq!(f.counters().sent, 0);
        // 確立後の最初のtapは、捨てた時刻に関係なく送る。
        assert_eq!(f.on_tap(5_001, true), Decision::Send);
    }

    #[test]
    fn a_tap_within_the_minimum_interval_is_dropped_and_counted() {
        let mut f = TapFilter::new();
        assert_eq!(f.on_tap(1_000, true), Decision::Send);
        assert_eq!(
            f.on_tap(1_000 + MIN_TAP_INTERVAL_MS - 1, true),
            Decision::DropTooSoon
        );
        assert_eq!(f.counters().dropped_too_soon, 1);
        // 捨てたtapは「送った時刻」を更新しない。間隔は前の送出から数える。
        assert_eq!(f.on_tap(1_000 + MIN_TAP_INTERVAL_MS, true), Decision::Send);
        assert_eq!(f.counters().sent, 2);
    }

    #[test]
    fn an_uptime_that_goes_backwards_does_not_underflow() {
        let mut f = TapFilter::new();
        assert_eq!(f.on_tap(10_000, true), Decision::Send);
        assert_eq!(f.on_tap(5, true), Decision::DropTooSoon);
    }
}
