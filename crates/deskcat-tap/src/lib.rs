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
//! 製品buildの`tapped`は、タッチの接触中と離れた直後も送らない（[`touch_blocks_tap`]。画面を撫でる力が加速度に伝わると推定しているため
//! （`EXP-044`は原因を確かめていない）。離れてからの時間は`firmware/esp32/src/touch.rs`の定数が持つ）。さらに、tapを見つけても
//! すぐには送らず、[`TAP_CONFIRM_MS`]待つ（[`TapFilter::with_confirmation`]。待つ間に接触が始まれば捨てる）。接触が「触れた」と判定されるより
//! 前（10〜260 ms）に出るtapがあるため。待たないのは`tap-21`試験build（[`TapFilter::new`]。タッチを読まない）。
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
/// **この範囲と閾値の関係は、データシートから確かめていない。**`THRESH_TAP`は独自のscale（0xFF＝約16 g。p.15）を持つが、
/// 範囲（2 g）を超える閾値が実際に届くかは分からない（範囲を超える入力が飽和するなら届かない、という推論はあるが、確かめていない）。
/// **2 gを超える調整が届くかは、実機で確かめる**（このcrateは範囲を広げない。必要になったら`DATA_FORMAT`を書く）。
///
/// **実機で詰める。**届かなければ`0x14`（1.25 g）まで下げる。静止中に`tapped`が出続けるなら上げる（2 gを超える調整の可否は上のとおり未確認。`tap_detected`のlogの
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

/// 製品buildが、tapを見つけてから`tapped`を送るまで待つ時間（ms）。待つ間にタッチの接触が始まれば（[`touch_blocks_tap`]が`true`になれば）
/// そのtapは捨てる（`DropTouch`）。**暫定の一般値である**（安全要件の5項目に効かない。外れても、撫でに`Surprised`が混ざるか、
/// 叩いてから`Surprised`までが遅れるだけで、何も壊れない）。
/// **根拠は机上の再集計である。**`EXP-044`の旧buildのlogを実機側が再集計したところ、タッチが「触れた」と判定される前の10〜260 ms
/// （多くは60〜90 ms）にtapが出ていた（11件）。集計のscriptと原本は、この値を決めた時点で他の担当は開いていない。**実測の窓ではない。**
/// 叩いてから`Surprised`までが、この分だけ遅れる（遅れは許容と判断した）。**送出は、見つけてから300 ms以上後である。**
/// 超える分は、tapを読む周期（`firmware/esp32/src/tap.rs`の`POLL_INTERVAL_MS`＝20 ms。実際にpollした時刻から次を積む）とmain loopの遅れによる（上限は保証しない）。**実機の窓で詰める。**
/// [`MIN_TAP_INTERVAL_MS`]以下でなければならない（待つ間の2件目のtapを、最小間隔の判定と矛盾なく捨てるため。下の`const`の検査）。
pub const TAP_CONFIRM_MS: u64 = 300;

const _: () = assert!(TAP_CONFIRM_MS <= MIN_TAP_INTERVAL_MS);

/// タッチが触れている間、または離れてから`holdoff_ms`未満のとき`true`。製品buildはこの間のtapを送らない
/// （画面を撫でる力が加速度センサーに伝わって単tapと判定され、撫でたのに`Surprised`が混ざる、と推定している。`EXP-044`は原因を確かめていない。#21）。
/// `released_at_ms`は、接触が終わった時刻（`None`は、まだ一度も離れていない）。
#[must_use]
pub const fn touch_blocks_tap(
    touching: bool,
    released_at_ms: Option<u64>,
    now_ms: u64,
    holdoff_ms: u64,
) -> bool {
    if touching {
        return true;
    }
    match released_at_ms {
        Some(released) => now_ms.saturating_sub(released) < holdoff_ms,
        None => false,
    }
}

/// tapを見つけたときの扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// `tapped`を送る。
    Send,
    /// 保留した。[`TAP_CONFIRM_MS`]の間に接触が始まらなければ、[`TapFilter::on_tick`]が`Send`を返す（始まれば`DropTouch`）。
    Hold,
    /// bootのACKを受けておらず、Piのsessionが確立していない。捨てる。保留中にsessionが入れ替わって（[`TapFilter::on_session`]。確立し直し済みの場合を含む）捨てた場合も、この扱いで数える。
    DropNoSession,
    /// タッチの接触中か、離れた直後である（[`touch_blocks_tap`]）。画面を撫でる力が加速度に伝わったtapとみなして（推定）捨てる。
    /// 保留中のtapが、待つ間に接触が始まって捨てられた場合も含む。
    DropTouch,
    /// 前の`tapped`から[`MIN_TAP_INTERVAL_MS`]未満である。捨てる。**保留中のtapがあるときに来た2件目も、これで捨てる**
    /// （先の1件の送出は、2件目から見て[`TAP_CONFIRM_MS`]（≦最小間隔）以内に決まるため。先の1件は残す）。
    DropTooSoon,
}

/// tapの扱いの内訳。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TapCounters {
    /// 送ると決めた数（`Send`を返した数。wireへ書けたかは数えない。保留しただけ（`Hold`）は数えない）。
    pub sent: u32,
    /// sessionが確立していない間（確立前と、終了した後を含む。保留中にsessionが終わった場合も）に捨てた数。
    pub dropped_no_session: u32,
    /// タッチの接触中か離れた直後で捨てた数。
    pub dropped_touch: u32,
    /// 間隔が短くて捨てた数。
    pub dropped_too_soon: u32,
    /// 保留中に、tap検出が止まって（読み出しの失敗が続いて）送れなくなり、捨てた数。
    pub dropped_detection_stopped: u32,
}

/// `tapped`を送ってよいかを決める。
#[derive(Debug, Clone, Default)]
pub struct TapFilter {
    last_sent_ms: Option<u64>,
    /// 保留中のtapを見つけた時刻。**1件だけ**持つ。
    pending_since_ms: Option<u64>,
    /// 0なら保留せず、見つけたtapをすぐ`Send`する。
    confirm_ms: u64,
    /// 最後に見たsessionの世代（[`Self::on_session`]）。比べるのは「等しいか」だけである。
    session_epoch: u32,
    counters: TapCounters,
}

impl TapFilter {
    /// まだ何も送っていない状態で作る。**保留しない**（見つけたtapをすぐ`Send`する。`tap-21`試験build）。
    #[must_use]
    pub const fn new() -> Self {
        Self::with_confirmation(0)
    }

    /// まだ何も送っていない状態で作る。tapを見つけたら`confirm_ms`保留し、その間に接触が始まらなければ送る（製品build。
    /// 値は[`TAP_CONFIRM_MS`]）。保留は1件だけである。
    #[must_use]
    pub const fn with_confirmation(confirm_ms: u64) -> Self {
        Self {
            last_sent_ms: None,
            pending_since_ms: None,
            confirm_ms,
            session_epoch: 0,
            counters: TapCounters {
                sent: 0,
                dropped_no_session: 0,
                dropped_touch: 0,
                dropped_too_soon: 0,
                dropped_detection_stopped: 0,
            },
        }
    }

    /// tapを1回見つけたときに呼ぶ。`now_ms`はuptime、`session_established`は`boot`のACKを受けたか、
    /// `touch_blocks`は[`touch_blocks_tap`]の結果（タッチを読まないbuildは`false`）。
    /// `Send`を返したときだけ、送った時刻として記録する（`DropTouch`は記録しない）。保留する構成では、見つけたtapは`Hold`を返し、送るのは
    /// [`Self::on_tick`]である。最小間隔は、保留を始めるときに確かめる（保留中は2件目を捨てるので、送るときに破れることはない）。
    /// 送った時刻は`on_tick`が`Send`を返した時刻である。
    pub fn on_tap(
        &mut self,
        now_ms: u64,
        session_established: bool,
        touch_blocks: bool,
    ) -> Decision {
        if !session_established {
            self.counters.dropped_no_session = self.counters.dropped_no_session.saturating_add(1);
            return Decision::DropNoSession;
        }
        if touch_blocks {
            self.counters.dropped_touch = self.counters.dropped_touch.saturating_add(1);
            return Decision::DropTouch;
        }
        if self
            .last_sent_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < MIN_TAP_INTERVAL_MS)
        {
            self.counters.dropped_too_soon = self.counters.dropped_too_soon.saturating_add(1);
            return Decision::DropTooSoon;
        }
        if self.pending_since_ms.is_some() {
            // 保留中の1件を残す。2件目は、先の1件の送出から最小間隔未満になるので、最小間隔の判定と同じ扱いにする。
            self.counters.dropped_too_soon = self.counters.dropped_too_soon.saturating_add(1);
            return Decision::DropTooSoon;
        }
        if self.confirm_ms > 0 {
            self.pending_since_ms = Some(now_ms);
            return Decision::Hold;
        }
        self.last_sent_ms = Some(now_ms);
        self.counters.sent = self.counters.sent.saturating_add(1);
        Decision::Send
    }

    /// tapを読む周期ごとに、**[`Self::on_tick`]と[`Self::on_tap`]の両方より前に**呼ぶ。`session_epoch`は、Piとのsessionが
    /// 確立した（`Established`へ入った）回数である。前に見た値と違えば、sessionが入れ替わったので、保留中のtapを捨てて
    /// `dropped_no_session`に数え、`true`を返す。新しいPi `sid`の`hello`による遷移のように、ESP32の`sid`が変わらず、tapを読む周期の間に入れ替わりきる場合も捨てられる。
    /// 新しい世代で見つけたtapを、前の世代の保留と取り違えないよう、`on_tap`より前に呼ぶ。
    /// 比べるのは「等しいか」だけで、一周して同じ値に戻るのは現実の時間では起きない。
    pub fn on_session(&mut self, session_epoch: u32) -> bool {
        if session_epoch == self.session_epoch {
            return false;
        }
        self.session_epoch = session_epoch;
        if self.pending_since_ms.take().is_none() {
            return false;
        }
        self.counters.dropped_no_session = self.counters.dropped_no_session.saturating_add(1);
        true
    }

    /// tap検出が止まる（`on_tick`が二度と呼ばれない）ときに呼ぶ。保留中のtapがあれば捨てて`dropped_detection_stopped`に数え、`true`を返す。
    pub fn abandon_pending(&mut self) -> bool {
        if self.pending_since_ms.take().is_none() {
            return false;
        }
        self.counters.dropped_detection_stopped =
            self.counters.dropped_detection_stopped.saturating_add(1);
        true
    }

    /// 保留中のtapがあるときに、tapを読む周期ごとに呼ぶ（tapのbitの有無に関わらず。[`Self::on_tap`]より先に呼ぶ）。
    /// 保留が終わった（送る、または捨てる）ときだけ`Some`を返す。まだ待つ間と、保留が無いときは`None`。
    /// 順序：sessionが無ければ`DropNoSession`、接触中か離れた直後なら`DropTouch`、保留から[`Self::with_confirmation`]の時間以上
    /// 経っていれば`Send`。**時間の境界は「以上」**（[`TAP_CONFIRM_MS`]未満では送らず、ちょうどで送る）。
    pub fn on_tick(
        &mut self,
        now_ms: u64,
        session_established: bool,
        touch_blocks: bool,
    ) -> Option<Decision> {
        let since = self.pending_since_ms?;
        if !session_established {
            self.pending_since_ms = None;
            self.counters.dropped_no_session = self.counters.dropped_no_session.saturating_add(1);
            return Some(Decision::DropNoSession);
        }
        if touch_blocks {
            self.pending_since_ms = None;
            self.counters.dropped_touch = self.counters.dropped_touch.saturating_add(1);
            return Some(Decision::DropTouch);
        }
        // 時計が戻ったとき（`now_ms < since`）は、経過0として保留を続ける。
        if now_ms.saturating_sub(since) < self.confirm_ms {
            return None;
        }
        self.pending_since_ms = None;
        self.last_sent_ms = Some(now_ms);
        self.counters.sent = self.counters.sent.saturating_add(1);
        Some(Decision::Send)
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
        assert_eq!(f.on_tap(100, false, false), Decision::DropNoSession);
        assert_eq!(f.on_tap(5_000, false, false), Decision::DropNoSession);
        assert_eq!(f.counters().dropped_no_session, 2);
        assert_eq!(f.counters().sent, 0);
        // 確立後の最初のtapは、捨てた時刻に関係なく送る。
        assert_eq!(f.on_tap(5_001, true, false), Decision::Send);
    }

    #[test]
    fn a_tap_within_the_minimum_interval_is_dropped_and_counted() {
        let mut f = TapFilter::new();
        assert_eq!(f.on_tap(1_000, true, false), Decision::Send);
        assert_eq!(
            f.on_tap(1_000 + MIN_TAP_INTERVAL_MS - 1, true, false),
            Decision::DropTooSoon
        );
        assert_eq!(f.counters().dropped_too_soon, 1);
        // 捨てたtapは「送った時刻」を更新しない。間隔は前の送出から数える。
        assert_eq!(
            f.on_tap(1_000 + MIN_TAP_INTERVAL_MS, true, false),
            Decision::Send
        );
        assert_eq!(f.counters().sent, 2);
    }

    #[test]
    fn a_tap_while_touching_is_dropped_counted_and_does_not_start_the_interval() {
        let mut f = TapFilter::new();
        assert_eq!(f.on_tap(1_000, true, true), Decision::DropTouch);
        assert_eq!(f.counters().dropped_touch, 1);
        assert_eq!(f.counters().sent, 0);
        // 捨てたtapは「送った時刻」を更新しない。すぐ後のtapは送る。
        assert_eq!(f.on_tap(1_001, true, false), Decision::Send);
        // sessionが無ければ、タッチより先にそちらで数える。
        assert_eq!(f.on_tap(2_000, false, true), Decision::DropNoSession);
        assert_eq!(f.counters().dropped_touch, 1);
    }

    #[test]
    fn touch_blocks_tap_while_touching_and_for_the_holdoff_after_release() {
        assert!(touch_blocks_tap(true, None, 100, 300));
        assert!(touch_blocks_tap(true, Some(0), 10_000, 300));
        assert!(!touch_blocks_tap(false, None, 100, 300));
        assert!(touch_blocks_tap(false, Some(1_000), 1_299, 300));
        assert!(!touch_blocks_tap(false, Some(1_000), 1_300, 300));
        // 時計が戻っても桁あふれしない（離れた直後として扱う）。
        assert!(touch_blocks_tap(false, Some(5_000), 10, 300));
    }

    #[test]
    fn an_uptime_that_goes_backwards_does_not_underflow() {
        let mut f = TapFilter::new();
        assert_eq!(f.on_tap(10_000, true, false), Decision::Send);
        assert_eq!(f.on_tap(5, true, false), Decision::DropTooSoon);
    }

    const C: u64 = TAP_CONFIRM_MS;

    #[test]
    fn a_held_tap_is_sent_at_the_confirmation_time_and_not_before() {
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        assert_eq!(f.counters().sent, 0);
        assert_eq!(f.on_tick(1_000 + C - 1, true, false), None);
        assert_eq!(f.on_tick(1_000 + C, true, false), Some(Decision::Send));
        assert_eq!(f.counters().sent, 1);
        // 保留は消えている。
        assert_eq!(f.on_tick(1_000 + C + 1, true, false), None);
    }

    #[test]
    fn a_touch_that_starts_while_held_drops_the_tap() {
        // 待ちの途中。
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        assert_eq!(f.on_tick(1_100, true, false), None);
        assert_eq!(f.on_tick(1_150, true, true), Some(Decision::DropTouch));
        assert_eq!(f.counters().dropped_touch, 1);
        assert_eq!(f.counters().sent, 0);
        assert_eq!(f.on_tick(1_000 + C, true, false), None);
        // 送った時刻は更新されない。すぐ後のtapは保留できる。
        assert_eq!(f.on_tap(1_200, true, false), Decision::Hold);
    }

    #[test]
    fn a_touch_that_is_already_blocking_at_the_confirmation_time_drops_the_tap() {
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        assert_eq!(f.on_tick(1_000 + C, true, true), Some(Decision::DropTouch));
        assert_eq!(f.counters().sent, 0);
    }

    #[test]
    fn a_touch_that_starts_after_the_tap_was_sent_does_not_unsend_it() {
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        assert_eq!(f.on_tick(1_000 + C, true, false), Some(Decision::Send));
        assert_eq!(f.on_tick(1_000 + C + 20, true, true), None);
        assert_eq!(f.counters().dropped_touch, 0);
    }

    #[test]
    fn a_tap_that_is_already_blocked_is_not_held() {
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(1_000, true, true), Decision::DropTouch);
        assert_eq!(f.on_tick(1_000 + C, true, false), None);
        assert_eq!(f.counters().dropped_touch, 1);
    }

    #[test]
    fn a_held_tap_is_dropped_when_the_session_ends() {
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        assert_eq!(
            f.on_tick(1_100, false, false),
            Some(Decision::DropNoSession)
        );
        assert_eq!(f.counters().dropped_no_session, 1);
        assert_eq!(f.on_tick(1_000 + C, true, false), None);
    }

    #[test]
    fn abandoning_a_held_tap_counts_it_once() {
        let mut f = TapFilter::with_confirmation(C);
        assert!(!f.abandon_pending());
        assert_eq!(f.counters().dropped_detection_stopped, 0);
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        assert!(f.abandon_pending());
        assert!(!f.abandon_pending());
        assert_eq!(f.counters().dropped_detection_stopped, 1);
        assert_eq!(f.counters().sent, 0);
        // 保留は消えている。
        assert_eq!(f.on_tick(1_000 + C, true, false), None);
    }

    #[test]
    fn a_tap_without_a_session_is_never_held() {
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(1_000, false, false), Decision::DropNoSession);
        assert_eq!(f.on_tick(1_000 + C, true, false), None);
        assert_eq!(f.counters().sent, 0);
    }

    #[test]
    fn a_second_tap_while_held_is_dropped_too_soon_and_the_first_is_kept() {
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        assert_eq!(f.on_tap(1_100, true, false), Decision::DropTooSoon);
        assert_eq!(f.on_tap(1_000 + C - 1, true, false), Decision::DropTooSoon);
        assert_eq!(f.counters().dropped_too_soon, 2);
        assert_eq!(f.on_tick(1_000 + C, true, false), Some(Decision::Send));
        assert_eq!(f.counters().sent, 1);
    }

    #[test]
    fn the_minimum_interval_counts_from_the_sent_time_not_the_detected_time() {
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        let sent_at = 1_000 + C;
        assert_eq!(f.on_tick(sent_at, true, false), Some(Decision::Send));
        // 送った時刻から最小間隔未満は、保留も始めない。
        assert_eq!(
            f.on_tap(sent_at + MIN_TAP_INTERVAL_MS - 1, true, false),
            Decision::DropTooSoon
        );
        assert_eq!(
            f.on_tap(sent_at + MIN_TAP_INTERVAL_MS, true, false),
            Decision::Hold
        );
        assert_eq!(f.counters().dropped_too_soon, 1);
    }

    #[test]
    fn a_filter_without_confirmation_sends_at_once_and_has_nothing_to_tick() {
        let mut f = TapFilter::new();
        assert_eq!(f.on_tap(1_000, true, false), Decision::Send);
        assert_eq!(f.on_tick(1_000 + C, true, false), None);
    }

    #[test]
    fn an_uptime_that_goes_backwards_keeps_the_hold() {
        let mut f = TapFilter::with_confirmation(C);
        assert_eq!(f.on_tap(10_000, true, false), Decision::Hold);
        assert_eq!(f.on_tick(5, true, false), None);
        assert_eq!(f.on_tick(10_000 + C, true, false), Some(Decision::Send));
    }

    #[test]
    fn the_holdoff_after_release_boundary_is_2999_and_3000_ms() {
        // 製品buildの値（`firmware/esp32/src/touch.rs`の`TAP_HOLDOFF_AFTER_RELEASE_MS`）と同じ3000 msで境界を確かめる。
        assert!(touch_blocks_tap(false, Some(2_000), 4_999, 3_000));
        assert!(!touch_blocks_tap(false, Some(2_000), 5_000, 3_000));
    }

    #[test]
    fn a_touch_released_600_820_1020_and_2180_ms_ago_still_blocks_with_the_3000_ms_holdoff() {
        // `EXP-044`の旧buildの再集計で、離れてから600・820 ms後と、`Happy`の直後の1020・2180 ms後にtapが出ていた。
        for after_release_ms in [600, 820, 1_020, 2_180] {
            assert!(touch_blocks_tap(
                false,
                Some(5_000),
                5_000 + after_release_ms,
                3_000
            ));
        }
    }

    #[test]
    fn a_held_tap_is_dropped_when_the_session_epoch_changes_even_if_it_is_established_again() {
        let mut f = TapFilter::with_confirmation(C);
        assert!(!f.on_session(1));
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        // 入れ替わった。周期の間に確立し直しているので、`session_established`は`true`のままである。
        assert!(f.on_session(2));
        assert_eq!(f.counters().dropped_no_session, 1);
        assert_eq!(f.on_tick(1_000 + C, true, false), None);
        assert_eq!(f.counters().sent, 0);
    }

    #[test]
    fn a_session_change_without_a_held_tap_counts_nothing() {
        let mut f = TapFilter::with_confirmation(C);
        assert!(!f.on_session(1));
        assert!(!f.on_session(2));
        assert_eq!(f.counters().dropped_no_session, 0);
    }

    #[test]
    fn the_same_epoch_keeps_the_hold_and_it_is_sent_when_due() {
        let mut f = TapFilter::with_confirmation(C);
        assert!(!f.on_session(3));
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        assert!(!f.on_session(3));
        assert_eq!(f.on_tick(1_000 + C, true, false), Some(Decision::Send));
    }

    #[test]
    fn a_tap_found_in_the_new_epoch_is_held_afresh_when_on_session_runs_first() {
        let mut f = TapFilter::with_confirmation(C);
        assert!(!f.on_session(1));
        assert_eq!(f.on_tap(1_000, true, false), Decision::Hold);
        // 次の周期：入れ替わりと、新しい世代での新しいtapが同時に見つかる。呼ぶ順は on_session → on_tick → on_tap。
        let now = 1_100;
        assert!(f.on_session(2));
        assert_eq!(f.on_tick(now, true, false), None);
        assert_eq!(f.on_tap(now, true, false), Decision::Hold);
        // 新しい保留は、古い保留の時刻ではなく、新しい tap を見つけた時刻から数える。
        assert_eq!(f.on_tick(now + C - 1, true, false), None);
        assert_eq!(f.on_tick(now + C, true, false), Some(Decision::Send));
        assert_eq!(f.counters().dropped_no_session, 1);
        assert_eq!(f.counters().sent, 1);
    }
}
