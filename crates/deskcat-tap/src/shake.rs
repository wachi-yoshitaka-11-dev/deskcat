//! `ACCEL-01`（ADXL345）の揺れの検出（`shaken`。Protocol §4.3a。Issue #685）。
//!
//! tapと同じく、I2Cを知らない。Activityの設定の(address, 値)の列と、`INT_SOURCE`のActivity bitの読み方と、
//! 「窓の中でActivityがN回立ったら揺れ」の判定と、送出の条件を返すだけである（I2Cを動かすのは`firmware/esp32`の`crate::tap`）。
//!
//! # 方式
//!
//! `ADXL345`のActivityは、`THRESH_ACT`を超える加速度が参加軸のどれかに出たときに`INT_SOURCE`のD4を立てる（読むまで残る）。
//! **tapが読んでいる`INT_SOURCE`と同じ1回のreadでActivityも取れる**ので、I2Cの読みは増えず、tapの検出は遅れない。
//! 1回の叩きでもActivityは1回立ちうるため、**窓の中で[`ShakeConfig::required_hits`]回（2以上）立ったときだけ揺れとする**。
//! Activityだけが続けて立った読み（Inactivityが無い）は1回と数え、Activityなしの読み、またはInactivityが立った読み（Linkでは、Activityと同じ読みでもよい）で再武装して、別の回とする（傾いて静止していても揺れにしない）。
//!
//! # 出所
//!
//! `THRESH_ACT`（address 0x24、unsigned、62.5 mg/LSB、0は予期しない動作になりうる）、`ACT_INACT_CTL`（address 0x27。D7が
//! ACT ac/dc、D6〜D4が`ACT_X`／`Y`／`Z` enable）、`INT_ENABLE`と`INT_SOURCE`のD4（Activity）は、**Analog Devices ADXL345 Data Sheet
//! Rev. E**（`datasheet.octopart.com`が配る販売店の写し。2026-11-11にPDFから文字を抽出して読んだ）による。
//! ac結合の記述（`ACT_INACT_CTL`の節。Issue #764）は、同じRev. Eを2026-11-15に読み直した。**公式のRev. Gとは照合していない**（CLIから取得できない）。`docs/hardware/sensor-datasheet-notes.md`にも同じことを書いた。
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
/// `ACT_INACT_CTL`（address 0x27）。D7がACT ac/dc（1でac結合）、D6〜D4が`ACT_X`／`Y`／`Z`の有効。
pub const REG_ACT_INACT_CTL: u8 = 0x27;
/// `THRESH_INACT`（address 0x25）。Inactivityの閾値。unsigned、62.5 mg/LSB。0は予期しない動作になりうる。
pub const REG_THRESH_INACT: u8 = 0x25;
/// `TIME_INACT`（address 0x26）。加速度が`THRESH_INACT`未満である時間。1 s/LSB。0なら、出力が閾値未満になった時点でInactivity。
pub const REG_TIME_INACT: u8 = 0x26;
/// `ACT_INACT_CTL`のD6〜D4（`ACT_X`／`Y`／`Z` enable）。
pub const ACT_AXES_MASK: u8 = 0b0111_0000;
/// `ACT_INACT_CTL`のD2〜D0（`INACT_X`／`Y`／`Z` enable）。
pub const INACT_AXES_MASK: u8 = 0b0000_0111;
/// `POWER_CTL`のD5（Link）。ActivityとInactivityを直列につなぐ。
pub const POWER_CTL_LINK: u8 = 1 << 5;
/// `INT_ENABLE`のD3（Inactivity）。
pub const INT_ENABLE_INACTIVITY: u8 = 1 << 3;
/// `INT_SOURCE`のD3（Inactivity）。
pub const INT_SOURCE_INACTIVITY: u8 = 1 << 3;
/// `INT_ENABLE`のD4（Activity）。
pub const INT_ENABLE_ACTIVITY: u8 = 1 << 4;
/// `INT_SOURCE`のD4（Activity）。
pub const INT_SOURCE_ACTIVITY: u8 = 1 << 4;

/// `INT_SOURCE`のbyteが、Activityを示すか。
#[must_use]
pub const fn is_activity(int_source: u8) -> bool {
    int_source & INT_SOURCE_ACTIVITY != 0
}

/// `INT_SOURCE`のbyteが、Inactivityを示すか。
#[must_use]
pub const fn is_inactivity(int_source: u8) -> bool {
    int_source & INT_SOURCE_INACTIVITY != 0
}

/// 揺れの検出の設定。値は[`SHAKE`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShakeConfig {
    /// `THRESH_ACT`。62.5 mg/LSB。0は不可。
    pub thresh_act: u8,
    /// `ACT_INACT_CTL`。ac/dcと軸。ACTの軸（[`ACT_AXES_MASK`]）が1つ以上要る。
    pub act_inact_ctl: u8,
    /// `POWER_CTL`のLink（D5）を立てるか。立てると、Activityの検出はInactivityを検出した後に始まる
    /// （ac結合のActivityの基準値を、Inactivityのたびに取り直す）。立てるなら、下の2つと
    /// `act_inact_ctl`のINACTの軸（[`INACT_AXES_MASK`]）が要る。
    pub link: bool,
    /// `THRESH_INACT`。62.5 mg/LSB。`link`のとき0は不可。
    pub thresh_inact: u8,
    /// `TIME_INACT`。1 s/LSB。0なら、出力が`THRESH_INACT`未満になった時点でInactivityとする。
    pub time_inact: u8,
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
            && (!self.link || (self.thresh_inact != 0 && self.act_inact_ctl & INACT_AXES_MASK != 0))
            && self.window_ms > 0
            && self.required_hits >= 2
    }
}

/// 揺れの検出の値。**暫定の一般値である**（安全要件の5項目に効かない）。
///
/// - `thresh_act`＝3（187.5 mg）。`act_inact_ctl`＝`0xEE`（`Activity`も`Inactivity`も**ac結合**・X・Y）。`link`＝true。`thresh_inact`＝2（125 mg）。`time_inact`＝0。
///   窓2000 ms、3回、最小間隔5000 ms。
///
/// **根拠（Issue #764）。**
/// - 出所：Analog Devices ADXL345 Data Sheet Rev. E（モジュール冒頭の「出所」）。ac結合のActivityは、検出の開始時の加速度を基準値として、差が`THRESH_ACT`を超えたら立つ
///   （`ACT_INACT_CTL`の節）。dc結合は重力が閾値を超える姿勢で立ち続けうる。`EXP-065`の(c)は、傾けて揺らしたとき多くは検出せず、原因を切り分けていない。
/// - 基準値の取り直し：datasheetはActivityの基準値の更新を直接書かない。`POWER_CTL`のLink（Link Bitの節）は、Activityの検出の開始をInactivityの検出まで遅らせる。
///   ac結合のInactivityの基準値は閾値を超えるたびに更新される（`ACT_INACT_CTL`の節）。この2つを合わせた**読み**として、置き直した後に静止すれば基準値が取り直される。
///   基準値を取る瞬間そのものは書かれていない。`TIME_INACT`＝0は「出力が閾値未満になったら割り込む」値で、1以上では窓2000 msで3回に届かない。
/// - 軸：X・Y。机の叩きはZだけ大きく（`EXP-061`）、Zを入れると叩きで`Activity`が立つ。
/// - 閾値：`EXP-061`（full-res、3.9 mg/LSB。187.5 mgは約48 LSB）で、揺らすYの最大72〜81は静止の平均12.8から約59〜68 LSB、叩きのX・Yの最大30は約17 LSB、静止の最大19は約6 LSB。
///   `THRESH_INACT`＝2（約32 LSB）は、静止の揺らぎ（最大約9 LSB）の上、`THRESH_ACT`の下に置いた一般値。
/// - 叩きと揺れを分ける根拠は無い。最小間隔は暫定の一般値で、1回の揺らしと1件の対応は確かめていない。
///
/// **確かめていないこと（実機）。**
/// - 置き直した後の静止で0件になること、揺らすと検出すること（Link下の基準値の取り直しは上の読みであり、host testの模擬はその読みから作った）。
/// - 実機のInactivityはフィルタ後の出力で比べる（`TIME_INACT`の節）。揺らしの間に何回Inactivityが立つか（検出漏れの可能性）は不明。
/// - `EXP-061`は水平のsampleで、立てた姿勢のsampleは無い。製品buildは`DATA_FORMAT`を書かず（10 bit）、`EXP-061`はfull-resである。
/// - servoの振動での誤検出と、`shaken`がPiへ届くこと。
///
/// **確定させる手段：**servoの振動のsample（#18の段2の後）と、B3を立てた姿勢での実機の静止・置き直し・揺らし（#764の受け入れ条件4）。
pub const SHAKE: Option<ShakeConfig> = Some(ShakeConfig {
    thresh_act: 3,
    act_inact_ctl: 0xEE,
    link: true,
    thresh_inact: 2,
    time_inact: 0,
    window_ms: 2_000,
    required_hits: 3,
    min_interval_ms: 5_000,
});

/// 検出に使える設定。`config`が`None`か、値が使えない形（[`ShakeConfig::is_valid`]）なら`None`。
#[must_use]
pub fn active(config: Option<ShakeConfig>) -> Option<ShakeConfig> {
    config.filter(ShakeConfig::is_valid)
}

/// 検出を始めるために書く(address, 値)の列（[`registers`]が返す）。最大10件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Writes {
    buf: [(u8, u8); 10],
    len: usize,
}

impl Writes {
    const fn new() -> Self {
        Self {
            buf: [(0, 0); 10],
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
            if config.link {
                writes.push((REG_THRESH_INACT, config.thresh_inact));
                writes.push((REG_TIME_INACT, config.time_inact));
                INT_ENABLE_SINGLE_TAP | INT_ENABLE_ACTIVITY | INT_ENABLE_INACTIVITY
            } else {
                INT_ENABLE_SINGLE_TAP | INT_ENABLE_ACTIVITY
            }
        }
        None => INT_ENABLE_SINGLE_TAP,
    };
    writes.push((REG_INT_ENABLE, int_enable));
    let link = if shake.is_some_and(|config| config.link) {
        POWER_CTL_LINK
    } else {
        0
    };
    writes.push((REG_POWER_CTL, POWER_CTL_MEASURE | link));
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
/// Activityだけが続けて立った読みは、1回と数える。Activityなしの読み、またはInactivityが立った読みで再武装する（[`ShakeFilter::on_int_source`]）。
/// 窓は固定である（最初のActivityから[`ShakeConfig::window_ms`]。過ぎたら数え直し、そのActivityを新しい窓の最初とする）。
/// 揺れと判定したら、送る・捨てるに関わらず数えを0へ戻す。
#[derive(Debug, Clone)]
pub struct ShakeFilter {
    config: ShakeConfig,
    window_start_ms: Option<u64>,
    hits: u32,
    /// 直前の読みがActivityなし、またはInactivityありだったか（再武装）。基準値との差（ac結合）や重力を含む加速度（dc結合）が閾値を超えたままだと、読むたびにActivityが立ち続ける。
    armed: bool,
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
            armed: true,
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
        // 前の読みもActivityだったなら、同じ立ちっぱなしの続きである。数えない（揺れは立ったり消えたりする）。
        if !std::mem::replace(&mut self.armed, false) {
            return ShakeDecision::Counting;
        }
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

    /// `INT_SOURCE`の1回の読みを扱う（Activityが立っていれば数え、立っていなければ再武装する）。
    /// Inactivityが立った読みは、間に静かな時間があったものとして再武装する。Linkを使うと、Activityは
    /// Inactivityを挟んだ回にしか立たない。**1回の読みに両方があるとき、どちらが先かは分からない**ので、
    /// 数える前と数えた後の両方で再武装する（Activity→Inactivityの順でも、次の読みのActivityを数え漏らさない）。
    pub fn on_int_source(
        &mut self,
        now_ms: u64,
        session_established: bool,
        int_source: u8,
    ) -> Option<ShakeDecision> {
        let activity = is_activity(int_source);
        let inactivity = is_inactivity(int_source);
        if !activity || inactivity {
            self.on_quiet();
        }
        let decision = activity.then(|| self.on_activity(now_ms, session_established));
        if activity && inactivity {
            self.on_quiet();
        }
        decision
    }

    /// 再武装する（Activityが立っていなかった読み、またはInactivityが立った読み。[`ShakeFilter::on_int_source`]）。
    pub fn on_quiet(&mut self) {
        self.armed = true;
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
        link: false,
        thresh_inact: 0,
        time_inact: 0,
        window_ms: 1_000,
        required_hits: 3,
        min_interval_ms: 2_000,
    };

    #[test]
    fn the_product_value_is_usable_and_keeps_z_out_of_the_axes() {
        let config = SHAKE.expect("製品の値が入っている");
        assert!(config.is_valid());
        assert_eq!(active(SHAKE), SHAKE);
        // ac結合（D7＝1。重力に依らない。Issue #764）で、X・Yだけ（Zを入れない）。
        assert_eq!(config.act_inact_ctl & 0b1000_0000, 0b1000_0000);
        assert_eq!(config.act_inact_ctl & ACT_AXES_MASK, 0b0110_0000);
        // Link＋ac結合のInactivity（X・Y）で、置き直しの後に基準値を取り直す（Issue #764）。
        assert!(config.link);
        assert_eq!(config.act_inact_ctl & 0b0000_1000, 0b0000_1000);
        assert_eq!(config.act_inact_ctl & INACT_AXES_MASK, 0b0000_0110);
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
        // Linkなし：`POWER_CTL`はMeasureだけで、Inactivityは設定しない。
        assert_eq!(writes.last().unwrap().1, POWER_CTL_MEASURE);
        assert!(writes.iter().all(|(r, _)| *r != REG_THRESH_INACT));
    }

    #[test]
    fn with_link_the_inactivity_is_set_before_the_interrupt_and_link_is_written_with_measure_last()
    {
        let link = ShakeConfig {
            act_inact_ctl: 0b1110_1110,
            link: true,
            thresh_inact: 2,
            time_inact: 0,
            ..TEST
        };
        let all = registers(Some(&link));
        let writes = all.as_slice();
        let pos = |reg: u8| writes.iter().position(|(r, _)| *r == reg).unwrap();
        assert_eq!(writes[pos(REG_THRESH_INACT)].1, 2);
        assert_eq!(writes[pos(REG_TIME_INACT)].1, 0);
        assert_eq!(writes[pos(REG_INT_ENABLE)].1, 0x40 | 0x10 | 0x08);
        assert!(pos(REG_THRESH_INACT) < pos(REG_INT_ENABLE));
        assert!(pos(REG_TIME_INACT) < pos(REG_INT_ENABLE));
        let last = writes.last().unwrap();
        assert_eq!(*last, (REG_POWER_CTL, POWER_CTL_MEASURE | POWER_CTL_LINK));
        assert!(is_inactivity(0x08) && !is_inactivity(0x10));
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
        let link_without_threshold = ShakeConfig {
            act_inact_ctl: 0b1110_1110,
            link: true,
            thresh_inact: 0,
            ..TEST
        };
        let link_without_axis = ShakeConfig {
            act_inact_ctl: 0b1110_0000,
            link: true,
            thresh_inact: 2,
            ..TEST
        };
        for bad in [
            zero_threshold,
            no_axis,
            one_hit,
            no_window,
            link_without_threshold,
            link_without_axis,
        ] {
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

    /// 静かな読み（Activityなし）を挟んでからActivityを見つけた、という1回。揺れは立ったり消えたりする。
    fn hit(f: &mut ShakeFilter, now_ms: u64, session_established: bool) -> ShakeDecision {
        f.on_quiet();
        f.on_activity(now_ms, session_established)
    }

    #[test]
    fn sustained_activity_without_a_quiet_read_is_not_a_shake() {
        // 基準との差が閾値を超えたままだと、`INT_SOURCE`を読むたびにActivityが立っている（Issue #751の5）。
        let mut f = ShakeFilter::new(TEST);
        for t in (0..1_000).step_by(10) {
            assert_eq!(f.on_activity(t, true), ShakeDecision::Counting);
        }
        assert_eq!(f.counters().sent, 0);
    }

    #[test]
    fn activity_counts_again_after_a_quiet_read() {
        let mut f = ShakeFilter::new(TEST);
        assert_eq!(f.on_activity(0, true), ShakeDecision::Counting);
        // 静かな読みが無いActivityは数えない。
        assert_eq!(f.on_activity(10, true), ShakeDecision::Counting);
        assert_eq!(f.on_activity(20, true), ShakeDecision::Counting);
        f.on_quiet();
        assert_eq!(f.on_activity(30, true), ShakeDecision::Counting);
        f.on_quiet();
        assert_eq!(f.on_activity(40, true), ShakeDecision::Send);
    }

    #[test]
    fn with_link_every_read_that_has_activity_and_inactivity_counts_once() {
        // 1回の読みにActivityとInactivityの両方が立つ場合（模擬の仮定。順序は不明）は、毎回数える。
        let both = INT_SOURCE_ACTIVITY | INT_SOURCE_INACTIVITY;
        let mut f = ShakeFilter::new(TEST);
        assert_eq!(
            f.on_int_source(0, true, both),
            Some(ShakeDecision::Counting)
        );
        // Activity→Inactivityの順だった場合の、次の読みのActivityだけの読み。
        assert_eq!(
            f.on_int_source(20, true, INT_SOURCE_ACTIVITY),
            Some(ShakeDecision::Counting)
        );
        assert_eq!(f.on_int_source(40, true, both), Some(ShakeDecision::Send));
        assert_eq!(f.counters().sent, 1);
    }

    #[test]
    fn sustained_activity_reads_without_inactivity_are_still_one_hit() {
        let mut f = ShakeFilter::new(TEST);
        for t in (0..1_000).step_by(20) {
            f.on_int_source(t, true, INT_SOURCE_ACTIVITY);
        }
        assert_eq!(f.counters().sent, 0);
        assert_eq!(f.on_int_source(1_000, true, 0), None);
    }

    #[test]
    fn a_lone_activity_such_as_a_tap_is_not_a_shake() {
        let mut f = ShakeFilter::new(TEST);
        assert_eq!(hit(&mut f, 100, true), ShakeDecision::Counting);
        assert_eq!(f.counters().sent, 0);
        assert_eq!(f.counters().activity_hits, 1);
    }

    #[test]
    fn required_hits_within_the_window_send_once_and_reset_the_count() {
        let mut f = ShakeFilter::new(TEST);
        assert_eq!(hit(&mut f, 100, true), ShakeDecision::Counting);
        assert_eq!(hit(&mut f, 300, true), ShakeDecision::Counting);
        assert_eq!(hit(&mut f, 500, true), ShakeDecision::Send);
        // 数えは0へ戻る。続くActivityは新しい窓の1回目である。
        assert_eq!(hit(&mut f, 520, true), ShakeDecision::Counting);
        assert_eq!(f.counters().sent, 1);
    }

    #[test]
    fn hits_spread_over_more_than_the_window_do_not_add_up() {
        let mut f = ShakeFilter::new(TEST);
        assert_eq!(hit(&mut f, 0, true), ShakeDecision::Counting);
        assert_eq!(hit(&mut f, 600, true), ShakeDecision::Counting);
        // 窓（最初から1000 ms）を1 ms超えた。このActivityが新しい窓の最初になる。
        assert_eq!(hit(&mut f, 1_001, true), ShakeDecision::Counting);
        assert_eq!(hit(&mut f, 1_100, true), ShakeDecision::Counting);
        assert_eq!(hit(&mut f, 1_200, true), ShakeDecision::Send);
        // 窓の境界（ちょうど window_ms）は窓の中である。
        let mut g = ShakeFilter::new(TEST);
        hit(&mut g, 0, true);
        hit(&mut g, 500, true);
        assert_eq!(hit(&mut g, 1_000, true), ShakeDecision::Send);
    }

    #[test]
    fn a_shake_before_the_session_is_established_is_dropped_and_counted() {
        let mut f = ShakeFilter::new(TEST);
        hit(&mut f, 0, false);
        hit(&mut f, 10, false);
        assert_eq!(hit(&mut f, 20, false), ShakeDecision::DropNoSession);
        assert_eq!(f.counters().dropped_no_session, 1);
        assert_eq!(f.counters().sent, 0);
        // 捨てたものは「送った時刻」を作らない。確立後はすぐ送る。
        hit(&mut f, 30, true);
        hit(&mut f, 40, true);
        assert_eq!(hit(&mut f, 50, true), ShakeDecision::Send);
    }

    #[test]
    fn a_shake_within_the_minimum_interval_is_dropped_and_does_not_move_the_interval() {
        let mut f = ShakeFilter::new(TEST);
        for t in [0, 10, 20] {
            hit(&mut f, t, true);
        }
        assert_eq!(f.counters().sent, 1);
        for t in [100, 110] {
            assert_eq!(hit(&mut f, t, true), ShakeDecision::Counting);
        }
        assert_eq!(hit(&mut f, 120, true), ShakeDecision::DropTooSoon);
        assert_eq!(f.counters().dropped_too_soon, 1);
        // 前の送出（20 ms）から最小間隔（2000 ms）以上たてば送る。
        hit(&mut f, 2_020, true);
        hit(&mut f, 2_030, true);
        assert_eq!(hit(&mut f, 2_040, true), ShakeDecision::Send);
        assert_eq!(f.counters().sent, 2);
    }

    #[test]
    fn a_clock_that_goes_backwards_does_not_panic_or_expire_the_window() {
        let mut f = ShakeFilter::new(TEST);
        hit(&mut f, 5_000, true);
        hit(&mut f, 4_000, true);
        assert_eq!(hit(&mut f, 4_100, true), ShakeDecision::Send);
    }

    // ---- 立てた（傾けた）姿勢の模擬（Issue #764）。製品の値（`SHAKE`）でADXL345のActivityを模擬し、`ShakeFilter`へ通す。 ----
    //
    // **模擬はdatasheet Rev. Eの記述の読みであり、実機の挙動ではない。**読み：
    // - ac結合のActivity：検出の開始時の加速度を基準値とし、差の大きさが`THRESH_ACT`を超えたら立つ（`ACT_INACT_CTL`の節）。
    // - ac結合のInactivity：基準値は閾値を超えるたびに更新し、差が`THRESH_INACT`未満の状態が`TIME_INACT`続いたら立つ（同節）。
    // - Link（`POWER_CTL`のD5）：Activityの検出の開始をInactivityの検出まで遅らせ、Activityの検出後はInactivityの検出が始まり、Activityは立たない（Link Bitの節）。
    // 実機のInactivityは**フィルタ後の出力**で比べる（`TIME_INACT`の節）が、模擬は生の標本で比べる。

    /// ADXL345の模擬。10 ms（100 Hz）ごとに標本を渡し、`INT_SOURCE`相当のbit（読むまで残る）をためる。
    struct Sim {
        config: ShakeConfig,
        /// `true`ならActivityの検出中。`false`ならInactivityの検出中（Linkのときだけ`false`になる）。
        detecting: bool,
        /// Activityの基準値（ac結合）。
        act_ref: [f64; 3],
        /// Inactivityの基準値。
        inact_ref: [f64; 3],
        quiet_ms: u32,
        latched: u8,
    }

    impl Sim {
        /// 起動時の標本`first`から始める。Linkなら、Inactivityの検出から始まる。
        fn new(config: ShakeConfig, first: [f64; 3]) -> Self {
            Self {
                config,
                detecting: !config.link,
                act_ref: first,
                inact_ref: first,
                quiet_ms: 0,
                latched: 0,
            }
        }

        fn exceeds(
            ctl: u8,
            mask_top: u8,
            thr_mg: f64,
            v: [f64; 3],
            reference: [f64; 3],
            ac: bool,
        ) -> bool {
            (0..3).any(|axis| {
                ctl & (mask_top >> axis) != 0 && {
                    let d = if ac {
                        v[axis] - reference[axis]
                    } else {
                        v[axis]
                    };
                    d.abs() > thr_mg
                }
            })
        }

        fn sample(&mut self, a: [f64; 3]) {
            let ctl = self.config.act_inact_ctl;
            let act_ac = ctl & 0b1000_0000 != 0;
            let act_thr = f64::from(self.config.thresh_act) * 62.5;
            if self.detecting {
                if Self::exceeds(ctl, 0b0100_0000, act_thr, a, self.act_ref, act_ac) {
                    self.latched |= INT_SOURCE_ACTIVITY;
                    if self.config.link {
                        // Activityの後はInactivityの検出が始まり、Activityは立たない。
                        self.detecting = false;
                        self.inact_ref = a;
                        self.quiet_ms = 0;
                    }
                }
                return;
            }
            let inact_thr = f64::from(self.config.thresh_inact) * 62.5;
            if Self::exceeds(ctl, 0b0000_0100, inact_thr, a, self.inact_ref, true) {
                // 閾値を超えたら基準値を更新し、静かな時間を数え直す。
                self.inact_ref = a;
                self.quiet_ms = 0;
                return;
            }
            self.quiet_ms += 10;
            if self.quiet_ms >= u32::from(self.config.time_inact) * 1_000 {
                self.latched |= INT_SOURCE_INACTIVITY;
                self.detecting = true;
                self.act_ref = a; // Activityの検出の開始時の加速度。
            }
        }

        /// `INT_SOURCE`を読む（読むと消える）。
        fn read(&mut self) -> u8 {
            std::mem::take(&mut self.latched)
        }
    }

    /// 20 ms（`firmware/esp32`のpoll間隔）ごとに`INT_SOURCE`を読み、`firmware/esp32/src/tap.rs`と同じ`on_int_source`で`ShakeFilter`へ通す。
    /// 列は10 ms刻みの標本。`start_ms`はこの列の最初の標本の時刻。揺れの検出（`Send`）の数を返す。
    fn feed(sim: &mut Sim, filter: &mut ShakeFilter, start_ms: u64, samples: &[[f64; 3]]) -> u32 {
        let mut sent = 0;
        for (i, a) in samples.iter().enumerate() {
            sim.sample(*a);
            if i % 2 == 1 {
                let now = start_ms + i as u64 * 10;
                let src = sim.read();
                if filter.on_int_source(now, true, src) == Some(ShakeDecision::Send) {
                    sent += 1;
                }
            }
        }
        sent
    }

    /// 10 ms刻みの標本で、`gravity`（mg）にY軸の振幅`amp_mg`（mg）・4 Hzの揺れを足した列。
    fn posture(gravity: [f64; 3], amp_mg: f64, samples: u32) -> Vec<[f64; 3]> {
        posture_with_phase(gravity, amp_mg, 0.0, samples)
    }

    /// `posture`の、揺れの開始位相（rad）を指定する版。
    fn posture_with_phase(
        gravity: [f64; 3],
        amp_mg: f64,
        phase: f64,
        samples: u32,
    ) -> Vec<[f64; 3]> {
        (0..samples)
            .map(|i| {
                let t = f64::from(i) * 0.01;
                let y = amp_mg * (2.0 * std::f64::consts::PI * 4.0 * t + phase).sin();
                [gravity[0], gravity[1] + y, gravity[2]]
            })
            .collect()
    }

    /// 平ら・立てた（Yが鉛直）・傾けた（X・Yに約0.7 gずつ）。
    const POSTURES: [[f64; 3]; 3] = [[0.0, 0.0, 1000.0], [0.0, 1000.0, 0.0], [707.0, 707.0, 0.0]];

    /// 起動時の姿勢`from`から始め、`to`へ置き直し（ステップ）、その姿勢で`rest_s`秒静止してから`shake_s`秒揺らす。
    /// （置き直しの最中の検出を除き）静止の間の検出数と、揺らしの間の検出数を返す。
    fn replaced(
        config: ShakeConfig,
        from: [f64; 3],
        to: [f64; 3],
        rest_s: u32,
        shake_s: u32,
    ) -> (u32, u32) {
        let mut sim = Sim::new(config, from);
        let mut f = ShakeFilter::new(config);
        // 起動直後の静止1秒。
        feed(&mut sim, &mut f, 0, &posture(from, 0.0, 100));
        // 置き直しの瞬間を含む静止。置き直した直後の1秒は、置き直しの動き（ステップ）の分として数えない。
        let t0 = 1_000;
        feed(&mut sim, &mut f, t0, &posture(to, 0.0, 100));
        let t1 = t0 + 1_000;
        let at_rest = feed(&mut sim, &mut f, t1, &posture(to, 0.0, rest_s * 100));
        let t2 = t1 + u64::from(rest_s) * 1_000;
        let shaken = feed(&mut sim, &mut f, t2, &posture(to, 300.0, shake_s * 100));
        (at_rest, shaken)
    }

    #[test]
    fn at_rest_in_any_posture_nothing_is_counted() {
        let config = SHAKE.unwrap();
        for g in POSTURES {
            let mut sim = Sim::new(config, g);
            let mut f = ShakeFilter::new(config);
            // 3分。
            assert_eq!(
                feed(&mut sim, &mut f, 0, &posture(g, 0.0, 18_000)),
                0,
                "{g:?}"
            );
        }
    }

    #[test]
    fn shaking_is_counted_in_any_posture_including_the_tilted_and_upright_ones() {
        let config = SHAKE.unwrap();
        for g in POSTURES {
            let mut sim = Sim::new(config, g);
            let mut f = ShakeFilter::new(config);
            feed(&mut sim, &mut f, 0, &posture(g, 0.0, 100));
            // 振幅300 mg（閾値187.5 mgの上。EXP-061の揺らすの幅より大きい側の模擬）。
            assert!(
                feed(&mut sim, &mut f, 1_000, &posture(g, 300.0, 150)) >= 1,
                "{g:?}"
            );
        }
    }

    #[test]
    fn a_small_wobble_below_the_threshold_is_not_counted_in_any_posture_or_phase() {
        let config = SHAKE.unwrap();
        for g in POSTURES {
            // Link下では基準値がInactivityのたびに取り直されるので、実効の閾値は振れの山と谷の差である。
            // 振幅80 mg（山と谷の差160 mg）は、閾値187.5 mgの下。開始位相の偶然で通らないよう、位相を振る。
            for phase in [0.0, 0.8, 1.6, 2.4, 3.2, 4.0, 4.8, 5.6] {
                let mut sim = Sim::new(config, g);
                let mut f = ShakeFilter::new(config);
                let samples = posture_with_phase(g, 80.0, phase, 300);
                assert_eq!(feed(&mut sim, &mut f, 0, &samples), 0, "{g:?} {phase}");
            }
        }
    }

    #[test]
    fn after_being_put_down_in_another_posture_rest_counts_nothing_and_shaking_is_still_counted() {
        // 持ち上げて置き直す（#22）。起動時と別の姿勢になっても、静止は数えず、揺らすと数える。
        let config = SHAKE.unwrap();
        for from in POSTURES {
            for to in POSTURES {
                let (at_rest, shaken) = replaced(config, from, to, 180, 2);
                assert_eq!(at_rest, 0, "{from:?} -> {to:?}");
                assert!(shaken >= 1, "{from:?} -> {to:?}");
            }
        }
    }

    #[test]
    fn without_link_a_reference_taken_at_boot_goes_stale_after_being_put_down() {
        // 模擬の振る舞い（Linkなしでは、起動時の基準値が置き直した後も古いまま）は仮定で、実機では確かめていない。
        let no_link = ShakeConfig {
            act_inact_ctl: 0xE0,
            link: false,
            ..SHAKE.unwrap()
        };
        let (at_rest, shaken) = replaced(no_link, POSTURES[0], POSTURES[1], 180, 2);
        assert_eq!(at_rest, 0);
        assert_eq!(shaken, 0);
    }
}
