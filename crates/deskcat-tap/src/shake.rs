//! `ACCEL-01`（ADXL345）の揺れの検出（`shaken`。Protocol §4.3a。Issue #685）。
//!
//! tapと同じく、I2Cを知らない。Activityの設定の(address, 値)の列と、`INT_SOURCE`のActivity bitの読み方と、
//! 「窓の中でActivityがN回立ったら揺れ」の判定と、送出の条件を返すだけである（I2Cを動かすのは`firmware/esp32`の`crate::tap`）。
//!
//! # 方式
//!
//! ADXL345のActivityは、`THRESH_ACT`を超える加速度が参加軸のどれかに出たときに`INT_SOURCE`のD4を立てる（読むまで残る）。
//! **tapが読んでいる`INT_SOURCE`と同じ1回のreadでActivityも取れる**ので、I2Cの読みは増えず、tapの検出は遅れない。
//! 1回の叩きでもActivityは1回立ちうるため、**窓の中で[`ShakeConfig::required_hits`]回（2以上）立ったときだけ揺れとする**。
//!
//! # 出所
//!
//! `THRESH_ACT`（address 0x24、unsigned、62.5 mg/LSB、0は予期しない動作になりうる）、`ACT_INACT_CTL`（address 0x27。D7が
//! ACT ac/dc、D6〜D4がACT_X／Y／Z enable）、`INT_ENABLE`と`INT_SOURCE`のD4（Activity）は、**Analog Devices ADXL345 Data Sheet
//! Rev. E**（`datasheet.octopart.com`が配る販売店の写し。2026-11-11にPDFから文字を抽出して読んだ）による。
//! **公式のRev. Gとは照合していない**（CLIから取得できない）。`docs/hardware/sensor-datasheet-notes.md`にも同じことを書いた。
//!
//! # 値の扱い
//!
//! 値は[`SHAKE`]が持つ。**暫定の一般値であり、根拠・確定の手段・確かめていないことは[`SHAKE`]のdocに1か所だけ書く。**
//! 値は安全要件の5項目に効かない（外れても、揺れの誤検出か検出漏れになるだけで、何も壊れない）。

use crate::{
    BW_RATE_100_HZ, DUR, INT_ENABLE_SINGLE_TAP, POWER_CTL_MEASURE, REG_BW_RATE, REG_DUR,
    REG_INT_ENABLE, REG_POWER_CTL, REG_TAP_AXES, REG_THRESH_TAP, TAP_AXES, THRESH_TAP,
};

/// `THRESH_ACT`（address 0x24）。Activityの閾値。unsigned、62.5 mg/LSB。
pub const REG_THRESH_ACT: u8 = 0x24;
/// `ACT_INACT_CTL`（address 0x27）。D7がACT ac/dc（1でac結合）、D6〜D4がACT_X／Y／Zの有効。
pub const REG_ACT_INACT_CTL: u8 = 0x27;
/// `ACT_INACT_CTL`のD6〜D4（ACT_X／Y／Z enable）。
pub const ACT_AXES_MASK: u8 = 0b0111_0000;
/// `INT_ENABLE`のD4（Activity）。
pub const INT_ENABLE_ACTIVITY: u8 = 1 << 4;
/// `INT_SOURCE`のD4（Activity）。
pub const INT_SOURCE_ACTIVITY: u8 = 1 << 4;

/// `INT_SOURCE`のbyteが、Activityを示すか。
#[must_use]
pub const fn is_activity(int_source: u8) -> bool {
    int_source & INT_SOURCE_ACTIVITY != 0
}

/// 揺れの検出の設定。値は[`SHAKE`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShakeConfig {
    /// `THRESH_ACT`。62.5 mg/LSB。0は不可。
    pub thresh_act: u8,
    /// `ACT_INACT_CTL`。ac/dcと軸。ACTの軸（[`ACT_AXES_MASK`]）が1つ以上要る。
    pub act_inact_ctl: u8,
    /// 窓の長さ（ms）。最初のActivityからこの時間以内に[`Self::required_hits`]回立たなければ、数え直す。
    pub window_ms: u64,
    /// 揺れとするActivityの回数。**2以上**（1回の叩きでもActivityは立ちうるため）。
    pub required_hits: u32,
    /// `shaken`を続けて送る最小の間隔（ms）。
    pub min_interval_ms: u64,
}

impl ShakeConfig {
    /// 値が使える形か。`THRESH_ACT`が0でなく、ACTの軸が1つ以上あり、窓が正で、回数が2以上であること。
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.thresh_act != 0
            && self.act_inact_ctl & ACT_AXES_MASK != 0
            && self.window_ms > 0
            && self.required_hits >= 2
    }
}

/// 揺れの検出の値。**暫定の一般値である**（安全要件の5項目に効かない）。
///
/// - `thresh_act`＝3（3×62.5＝187.5 mg）。`act_inact_ctl`＝`0x60`（dc結合、X・Yのみ）。窓2000 ms、3回、最小間隔5000 ms。
///
/// **根拠は`docs/hardware/experiment-log.md`の`EXP-061`（`accel-sample-15`。full-res、±2 g、3.9 mg/LSB。187.5 mgは約48 LSB）。**
/// - 軸：静止のZは約−236 LSB（約1 g）で、dc結合でZを入れるとActivityが立ち続ける。X・Yは静止で平均−7.5／12.8、最大絶対値19。
///   机の叩きはX・Yのピークが最大30で、Zだけ大きい（−192〜−295）。
/// - 閾値：揺らすのYの最大は72〜81（窓1〜5。窓6は29。Xは−47〜23）。机の叩きのX・Yの最大30と静止の最大19の上、揺れのYのピークの下に置いた。2（約32 LSB）は叩きの30に近い。
/// - 叩きと揺れを分ける根拠は無い（EXP-061の叩きは机で最大約0.23 g、製品のtappedは1.5 g超）。
/// - 最小間隔：暫定の一般値。1回の揺らしと1件の対応は確かめていない。
///
/// **確かめていないこと。**窓の中でActivityが何回立つかはEXP-061に時系列が無く不明（揺らし方によっては届かず検出漏れ）。
/// dc結合なので基板が水平でなければX・Yに重力が乗る（約0.19 g、約11°で立ち続ける。取付が水平かは`docs`に無い）。
/// 製品buildは`DATA_FORMAT`を書かず（10 bit）、EXP-061はfull-resである（`THRESH_ACT`はg単位なので閾値は変わらないが、10 bitのLSBの重みは未確認）。
/// servoの振動での誤検出と、実機で`shaken`がPiへ届くことは未確認。
/// **確定させる手段：**servoの振動のsample（#18の段2の後）と、取付姿勢が決まった後の実機の揺らし。
pub const SHAKE: Option<ShakeConfig> = Some(ShakeConfig {
    thresh_act: 3,
    act_inact_ctl: 0x60,
    window_ms: 2_000,
    required_hits: 3,
    min_interval_ms: 5_000,
});

/// 検出に使える設定。`config`が`None`か、値が使えない形（[`ShakeConfig::is_valid`]）なら`None`。
#[must_use]
pub fn active(config: Option<ShakeConfig>) -> Option<ShakeConfig> {
    config.filter(ShakeConfig::is_valid)
}

/// 検出を始めるために書く(address, 値)の列（[`registers`]が返す）。最大8件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Writes {
    buf: [(u8, u8); 8],
    len: usize,
}

impl Writes {
    const fn new() -> Self {
        Self {
            buf: [(0, 0); 8],
            len: 0,
        }
    }

    fn push(&mut self, write: (u8, u8)) {
        self.buf[self.len] = write;
        self.len += 1;
    }

    /// 書く順の列。
    #[must_use]
    pub fn as_slice(&self) -> &[(u8, u8)] {
        &self.buf[..self.len]
    }
}

/// 検出を始めるために書く(address, 値)の列。**この順に書く。**
/// `shake`が`None`なら[`crate::SETUP`]と同じ（Activityを設定しない）。`Some`なら、`INT_ENABLE`へActivityを足し、
/// `THRESH_ACT`と`ACT_INACT_CTL`を割り込みの有効化より前に書く。**測定を始める書き込みは最後のまま**である。
#[must_use]
pub fn registers(shake: Option<&ShakeConfig>) -> Writes {
    let mut writes = Writes::new();
    writes.push((REG_BW_RATE, BW_RATE_100_HZ));
    writes.push((REG_THRESH_TAP, THRESH_TAP));
    writes.push((REG_DUR, DUR));
    writes.push((REG_TAP_AXES, TAP_AXES));
    let int_enable = match shake {
        Some(config) => {
            writes.push((REG_THRESH_ACT, config.thresh_act));
            writes.push((REG_ACT_INACT_CTL, config.act_inact_ctl));
            INT_ENABLE_SINGLE_TAP | INT_ENABLE_ACTIVITY
        }
        None => INT_ENABLE_SINGLE_TAP,
    };
    writes.push((REG_INT_ENABLE, int_enable));
    writes.push((REG_POWER_CTL, POWER_CTL_MEASURE));
    writes
}

/// Activityを見つけたときの扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShakeDecision {
    /// まだ回数に届かない。
    Counting,
    /// `shaken`を送る。
    Send,
    /// 揺れと判定したが、bootのACKを受けておらず、Piのsessionが確立していない。捨てる。
    DropNoSession,
    /// 揺れと判定したが、前の`shaken`から[`ShakeConfig::min_interval_ms`]未満である。捨てる。
    DropTooSoon,
}

/// 揺れの扱いの内訳。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShakeCounters {
    /// 見つけたActivityの数。
    pub activity_hits: u32,
    /// 送ると決めた数（wireへ書けたかは数えない）。
    pub sent: u32,
    /// sessionが確立していない間に揺れと判定して捨てた数。
    pub dropped_no_session: u32,
    /// 間隔が短くて捨てた数。
    pub dropped_too_soon: u32,
}

/// 「窓の中でActivityがN回」を数え、`shaken`を送ってよいかを決める。
///
/// 窓は固定である（最初のActivityから[`ShakeConfig::window_ms`]。過ぎたら数え直し、そのActivityを新しい窓の最初とする）。
/// 揺れと判定したら、送る・捨てるに関わらず数えを0へ戻す。
#[derive(Debug, Clone)]
pub struct ShakeFilter {
    config: ShakeConfig,
    window_start_ms: Option<u64>,
    hits: u32,
    last_sent_ms: Option<u64>,
    counters: ShakeCounters,
}

impl ShakeFilter {
    /// 使える設定（[`active`]が返したもの）から作る。
    #[must_use]
    pub const fn new(config: ShakeConfig) -> Self {
        Self {
            config,
            window_start_ms: None,
            hits: 0,
            last_sent_ms: None,
            counters: ShakeCounters {
                activity_hits: 0,
                sent: 0,
                dropped_no_session: 0,
                dropped_too_soon: 0,
            },
        }
    }

    /// `INT_SOURCE`にActivityがあったときに呼ぶ（読むたびに高々1回）。`now_ms`はuptime、
    /// `session_established`は`boot`のACKを受けたか。
    pub fn on_activity(&mut self, now_ms: u64, session_established: bool) -> ShakeDecision {
        self.counters.activity_hits = self.counters.activity_hits.saturating_add(1);
        // 窓を過ぎていたら、このActivityを新しい窓の最初にする。時計が戻ったとき（`now_ms < start`）は経過0として窓を続ける。
        let expired = self
            .window_start_ms
            .is_some_and(|start| now_ms.saturating_sub(start) > self.config.window_ms);
        if expired || self.window_start_ms.is_none() {
            self.window_start_ms = Some(now_ms);
            self.hits = 0;
        }
        self.hits = self.hits.saturating_add(1);
        if self.hits < self.config.required_hits {
            return ShakeDecision::Counting;
        }
        self.window_start_ms = None;
        self.hits = 0;
        if !session_established {
            self.counters.dropped_no_session = self.counters.dropped_no_session.saturating_add(1);
            return ShakeDecision::DropNoSession;
        }
        if self
            .last_sent_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < self.config.min_interval_ms)
        {
            self.counters.dropped_too_soon = self.counters.dropped_too_soon.saturating_add(1);
            return ShakeDecision::DropTooSoon;
        }
        self.last_sent_ms = Some(now_ms);
        self.counters.sent = self.counters.sent.saturating_add(1);
        ShakeDecision::Send
    }

    /// これまでの内訳。
    #[must_use]
    pub const fn counters(&self) -> ShakeCounters {
        self.counters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // **試験用の値である。製品の値ではない**（製品の値は`SHAKE`）。
    const TEST: ShakeConfig = ShakeConfig {
        thresh_act: 0x10,
        act_inact_ctl: 0b0111_0000,
        window_ms: 1_000,
        required_hits: 3,
        min_interval_ms: 2_000,
    };

    #[test]
    fn the_product_value_is_usable_and_keeps_z_out_of_the_axes() {
        let config = SHAKE.expect("製品の値が入っている");
        assert!(config.is_valid());
        assert_eq!(active(SHAKE), SHAKE);
        // dc結合（D7＝0）で、X・Yだけ（Zを入れない）。
        assert_eq!(config.act_inact_ctl & 0b1000_0000, 0);
        assert_eq!(config.act_inact_ctl & ACT_AXES_MASK, 0b0110_0000);
    }

    #[test]
    fn without_a_config_the_setup_is_the_tap_only_setup() {
        assert_eq!(registers(None).as_slice(), crate::SETUP.as_slice());
    }

    #[test]
    fn with_a_config_activity_is_set_before_the_interrupt_and_measure_stays_last() {
        let all = registers(Some(&TEST));
        let writes = all.as_slice();
        let pos = |reg: u8| writes.iter().position(|(r, _)| *r == reg).unwrap();
        assert_eq!(writes[pos(REG_THRESH_ACT)].1, 0x10);
        assert_eq!(writes[pos(REG_ACT_INACT_CTL)].1, 0b0111_0000);
        // INT_ENABLEはtapとActivityの両方。
        assert_eq!(writes[pos(REG_INT_ENABLE)].1, 0x40 | 0x10);
        assert!(pos(REG_THRESH_ACT) < pos(REG_INT_ENABLE));
        assert!(pos(REG_ACT_INACT_CTL) < pos(REG_INT_ENABLE));
        assert_eq!(writes.last().unwrap().0, REG_POWER_CTL);
    }

    #[test]
    fn unusable_values_are_not_activated() {
        let zero_threshold = ShakeConfig {
            thresh_act: 0,
            ..TEST
        };
        let no_axis = ShakeConfig {
            act_inact_ctl: 0b1000_0000,
            ..TEST
        };
        let one_hit = ShakeConfig {
            required_hits: 1,
            ..TEST
        };
        let no_window = ShakeConfig {
            window_ms: 0,
            ..TEST
        };
        for bad in [zero_threshold, no_axis, one_hit, no_window] {
            assert_eq!(active(Some(bad)), None, "{bad:?}");
        }
        assert_eq!(active(Some(TEST)), Some(TEST));
    }

    #[test]
    fn activity_is_bit_4_of_int_source_and_distinct_from_tap() {
        assert!(is_activity(0x10));
        assert!(!is_activity(0x40));
        assert!(!is_activity(0x80));
        assert!(!is_activity(0x00));
        assert!(is_activity(0xFF));
    }

    #[test]
    fn a_lone_activity_such_as_a_tap_is_not_a_shake() {
        let mut f = ShakeFilter::new(TEST);
        assert_eq!(f.on_activity(100, true), ShakeDecision::Counting);
        assert_eq!(f.counters().sent, 0);
        assert_eq!(f.counters().activity_hits, 1);
    }

    #[test]
    fn required_hits_within_the_window_send_once_and_reset_the_count() {
        let mut f = ShakeFilter::new(TEST);
        assert_eq!(f.on_activity(100, true), ShakeDecision::Counting);
        assert_eq!(f.on_activity(300, true), ShakeDecision::Counting);
        assert_eq!(f.on_activity(500, true), ShakeDecision::Send);
        // 数えは0へ戻る。続くActivityは新しい窓の1回目である。
        assert_eq!(f.on_activity(520, true), ShakeDecision::Counting);
        assert_eq!(f.counters().sent, 1);
    }

    #[test]
    fn hits_spread_over_more_than_the_window_do_not_add_up() {
        let mut f = ShakeFilter::new(TEST);
        assert_eq!(f.on_activity(0, true), ShakeDecision::Counting);
        assert_eq!(f.on_activity(600, true), ShakeDecision::Counting);
        // 窓（最初から1000 ms）を1 ms超えた。このActivityが新しい窓の最初になる。
        assert_eq!(f.on_activity(1_001, true), ShakeDecision::Counting);
        assert_eq!(f.on_activity(1_100, true), ShakeDecision::Counting);
        assert_eq!(f.on_activity(1_200, true), ShakeDecision::Send);
        // 窓の境界（ちょうど window_ms）は窓の中である。
        let mut g = ShakeFilter::new(TEST);
        g.on_activity(0, true);
        g.on_activity(500, true);
        assert_eq!(g.on_activity(1_000, true), ShakeDecision::Send);
    }

    #[test]
    fn a_shake_before_the_session_is_established_is_dropped_and_counted() {
        let mut f = ShakeFilter::new(TEST);
        f.on_activity(0, false);
        f.on_activity(10, false);
        assert_eq!(f.on_activity(20, false), ShakeDecision::DropNoSession);
        assert_eq!(f.counters().dropped_no_session, 1);
        assert_eq!(f.counters().sent, 0);
        // 捨てたものは「送った時刻」を作らない。確立後はすぐ送る。
        f.on_activity(30, true);
        f.on_activity(40, true);
        assert_eq!(f.on_activity(50, true), ShakeDecision::Send);
    }

    #[test]
    fn a_shake_within_the_minimum_interval_is_dropped_and_does_not_move_the_interval() {
        let mut f = ShakeFilter::new(TEST);
        for t in [0, 10, 20] {
            f.on_activity(t, true);
        }
        assert_eq!(f.counters().sent, 1);
        for t in [100, 110] {
            assert_eq!(f.on_activity(t, true), ShakeDecision::Counting);
        }
        assert_eq!(f.on_activity(120, true), ShakeDecision::DropTooSoon);
        assert_eq!(f.counters().dropped_too_soon, 1);
        // 前の送出（20 ms）から最小間隔（2000 ms）以上たてば送る。
        f.on_activity(2_020, true);
        f.on_activity(2_030, true);
        assert_eq!(f.on_activity(2_040, true), ShakeDecision::Send);
        assert_eq!(f.counters().sent, 2);
    }

    #[test]
    fn a_clock_that_goes_backwards_does_not_panic_or_expire_the_window() {
        let mut f = ShakeFilter::new(TEST);
        f.on_activity(5_000, true);
        f.on_activity(4_000, true);
        assert_eq!(f.on_activity(4_100, true), ShakeDecision::Send);
    }
}
