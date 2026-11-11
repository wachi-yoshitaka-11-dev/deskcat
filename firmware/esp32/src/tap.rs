//! ADXL345のtap検出を、main loopの中で周期的に読む（[#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)、F1）。
//!
//! 製品buildと`tap-21`試験buildがこのmoduleを使う。起動時のI2C bring-upが返す`I2cDriver`を持ち続け、
//! [`TapDetector::configure`]でtap検出を設定し、[`TapDetector::poll`]が[`POLL_INTERVAL_MS`]ごとに
//! `INT_SOURCE`をI2Cで読む（**割り込みのINT線は使わない。配線は要らない**）。tapを見つけたら
//! `deskcat_tap::TapFilter`が送ってよいかを決める（bootのACK後だけ、前の`tapped`から最小間隔以上、製品buildはタッチの接触中と離れた直後でない）。
//! **製品buildは、tapを見つけてもすぐには送らず、`deskcat_tap::TAP_CONFIRM_MS`保留する**（待つ間に接触が始まれば捨てる）。保留は1件だけで、
//! 待つ間の2件目は最小間隔未満として捨てる（先の1件を残す）。保留中にPiとのsessionが入れ替わったら（`BootSession::session_epoch`が変わったら）、保留中の1件を捨てる。`tap-21`試験buildはタッチを読まないので保留せず、見つけたらすぐ送る。
//! 設定値・出所・暫定であることは`deskcat_tap`のcrate docが持つ（**ここへ再掲しない**）。
//!
//! # I2C driverの所有
//!
//! ADXL345のDevice IDが`0xE5`のとき（製品buildは`main.rs`の`start_product_sensors`、`tap-21`付き試験buildは`start_tap`が確かめる）、
//! `TapDetector`が起動時のI2C bring-upの`I2cDriver`を持つ。**一致しなければ何も書かず、`TapDetector`を作らない。**
//! `tap-21`付き試験buildは、このとき`I2cDriver`も手放す。
//! **製品buildは、同じ`I2cDriver`を`crate::env_report::EnvReporter`と共有する**（`crate::i2c_bus`。#487）。
//! **製品buildは、Device IDが一致しなくても`I2cDriver`を手放さない**（`EnvReporter`が持ち続ける）。
//! 共有しても、tapのbitは読むまで残るので、envのtransactionが挟まってもtapの検出は落ちない（読みが遅れるだけである）。
//! `tap-21`付き試験buildは、envと共有しない（今は誰も`ENV-01`を読まない）。
//!
//! # 失敗の扱い
//!
//! I2Cの読み出しが失敗しても止まらない（1 transactionは有限時間で返る。上限は`config::I2C_TRANSACTION_TIMEOUT_MS`の100 ms。
//! 設定（`configure`）の滞留の**上界**は、全部成功する経路で700 ms（6書き込み＋捨て読み1。各transactionのtimeout×7の上界で、
//! 成功したtransactionはその前に終わる）、書き込みが途中で失敗する経路で600 ms（最初の失敗で止まる）、読み出しの失敗は1周回あたり
//! 最大100 ms。**揺れの設定（`deskcat_tap::shake::SHAKE`）に値が入ると、書き込みは8件になり、上の2つの上界はそれぞれ900 ms・800 msである**）。
//! **設定（`configure`）は、Device IDが一致したときだけ走る**（一致しなければ`start_product_sensors`／`start_tap`が`tap_not_started`をlogへ出す）。
//! **設定（`configure`）が走る場所はbuildで違う。**`bringup-display-13`を持たないbuildは、起動時のI2C bring-upの直後、
//! main loopの前で走る（`BootSession::start`の前でもあるため、上の時間が最初の`boot`の送出を遅らせうる）。
//! `bringup-display-13`付きbuildは、LCDのbring-upが済んだ後にmain loopの1周回の中で走る（その1周回が、上の時間だけ延びうる）。
//! 読み出しは、いずれのbuildでもmain loopの1周回に乗る。[`MAX_CONSECUTIVE_ERRORS`]回続けて失敗したら、
//! 検出をやめる（bus固着のとき、main loopを毎周回I2C timeoutで塞がないため）。やめたことはlogへ出す。
//!
//! # 確かめていないこと
//!
//! **このmoduleの振る舞いは、host testでも実機でも確かめていない。**`firmware/esp32`はhostのworkspaceから除外され、
//! `esp_idf_svc`へ依存するため、host testでは動かせない。判定は`deskcat_tap`のhost testが固定している。
//! 実機の確認（指で軽く叩いて`tapped`がPiへ届くこと、閾値の適否、誤検出）は#21の実機統合が持つ。`configure`は、測定modeへ入れた直後に`INT_SOURCE`を
//! 1回読んで捨てる（過渡でtapのbitが立つかは確かめていない。立っても誤った`tapped`を送らないための備えである）。

// tapを持たない試験buildではdead_codeになる。moduleごと`#[cfg]`で落とさず、
// 各試験buildでもcross-compileを確認する。
#![allow(dead_code)]

use crate::accel::Adxl345;
use crate::i2c_bus::SharedI2c;
use deskcat_tap::shake::{ShakeConfig, ShakeDecision, ShakeFilter};
use deskcat_tap::{Decision, TapFilter};

/// `INT_SOURCE`を読む間隔（ms）。**暫定の一般値である**（安全要件の5項目に効かない）。データレート（`deskcat_tap::BW_RATE_100_HZ`＝100 Hz＝10 ms）の
/// 2倍にした。tapのbitは読むまで残るため、読み損ねは起きない。
pub const POLL_INTERVAL_MS: u64 = 20;
/// 続けて失敗したらtap検出をやめる回数。暫定の一般値である。
pub const MAX_CONSECUTIVE_ERRORS: u32 = 5;

/// 1回の[`TapDetector::poll`]で、送るべきevent。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PollEvents {
    /// `tapped`を送る。
    pub tapped: bool,
    /// `shaken`を送る（揺れの検出が有効なときだけ`true`になる。`deskcat_tap::shake`）。
    pub shaken: bool,
}

/// tap検出の状態。
pub struct TapDetector {
    accel: Adxl345,
    i2c: SharedI2c,
    filter: TapFilter,
    /// 揺れの検出。`deskcat_tap::shake::SHAKE`が`None`（`TBD`）の間は`None`で、Activityを設定せず、`shaken`も送らない。
    shake_config: Option<ShakeConfig>,
    shake: Option<ShakeFilter>,
    next_poll_ms: u64,
    consecutive_errors: u32,
    enabled: bool,
}

impl TapDetector {
    /// 起動時のI2C bring-upが返した`I2cDriver`を、共有できる形（`crate::i2c_bus::SharedI2c`）で受け取る。**まだ設定は書かない。**
    pub fn new(accel: Adxl345, i2c: SharedI2c) -> Self {
        Self {
            accel,
            i2c,
            // 製品buildだけが保留する。`tap-21`試験buildはタッチを読まないので、保留しない。
            #[cfg(deskcat_product_mode)]
            filter: TapFilter::with_confirmation(deskcat_tap::TAP_CONFIRM_MS),
            #[cfg(not(deskcat_product_mode))]
            filter: TapFilter::new(),
            next_poll_ms: 0,
            shake_config: deskcat_tap::shake::active(deskcat_tap::shake::SHAKE),
            shake: None,
            consecutive_errors: 0,
            enabled: false,
        }
    }

    /// tap検出の設定を書く。失敗したら検出を始めない（logへ出す）。
    pub fn configure(&mut self, now_ms: u64) {
        // **借用（`RefCell`）は`let`の文で終える。**`match`の被判定式に置くと、armの中まで借用が続き、armの中の2回目の借用が`panic`する。
        let configured = self
            .accel
            .configure_tap(&mut self.i2c.borrow_mut(), self.shake_config.as_ref());
        match configured {
            Ok(()) => {
                // 測定modeへ入れた直後にtapのbitが過渡で立っていても、誤った`tapped`を送らないよう、
                // `INT_SOURCE`を1回読んで捨てる（読むとtapのbitが消える。失敗しても検出は始める）。
                let discard = self.accel.read_int_source(&mut self.i2c.borrow_mut());
                match discard {
                    Ok(discarded) => {
                        log::info!("tap_configure_discard int_source=0x{discarded:02x}")
                    }
                    Err(err) => log::warn!("tap_configure_discard_failed error={err}"),
                }
                self.shake = self.shake_config.map(ShakeFilter::new);
                log::info!("shake_configured enabled={}", self.shake.is_some());
                self.enabled = true;
                self.next_poll_ms = now_ms + POLL_INTERVAL_MS;
                log::info!(
                    "tap_configured bw_rate=0x{:02x} thresh_tap=0x{:02x} dur=0x{:02x} tap_axes=0x{:02x} poll_interval_ms={POLL_INTERVAL_MS}",
                    deskcat_tap::BW_RATE_100_HZ,
                    deskcat_tap::THRESH_TAP,
                    deskcat_tap::DUR,
                    deskcat_tap::TAP_AXES
                );
            }
            Err(err) => log::error!("tap_configure_failed error={err}"),
        }
    }

    /// 次に読む時刻（uptimeのms）。検出していなければ`None`。
    pub fn next_deadline_ms(&self) -> Option<u64> {
        self.enabled.then_some(self.next_poll_ms)
    }

    /// 時刻に達していれば、保留中のtapを決め（`TapFilter::on_tick`）、`INT_SOURCE`を1回読む。**送るべきevent（`tapped`と`shaken`）を[`PollEvents`]で返す。**
    /// `touch_blocks`は、タッチの接触中か離れた直後か（`deskcat_tap::touch_blocks_tap`。タッチを読まないbuildは`false`）。
    /// 接触中か直後のtapは捨てて`dropped_touch`に数える。保留する構成では、見つけた周期ではなく、保留が明けた周期に`tapped`を`true`にする。
    /// 1周期に`tapped`が`true`なのは高々1回である（保留が明けた周期に新しく見つけたtapは、最小間隔未満として捨てる）。
    /// `session_epoch`は`BootSession::session_epoch`（sessionが入れ替わったら、保留中の1件を捨てる。`TapFilter::on_session`）。
    pub fn poll(
        &mut self,
        now_ms: u64,
        session_established: bool,
        session_epoch: u32,
        touch_blocks: bool,
    ) -> PollEvents {
        let mut events = PollEvents::default();
        if !self.enabled || now_ms < self.next_poll_ms {
            return events;
        }
        // 次の期限は`now_ms`から積む（前の期限からではない）。main loopの1周回が延びた分だけ間隔が後ろへずれるが、
        // tapのbitは読むまで残るため、検出漏れにはならない。
        self.next_poll_ms = now_ms + POLL_INTERVAL_MS;
        // sessionの入れ替わりを、`on_tick`と`on_tap`の両方より先に反映する（新しい世代のtapを、前の世代の保留と取り違えない）。
        if self.filter.on_session(session_epoch) {
            self.log_decision("tap_confirmed", Decision::DropNoSession, None);
        }
        // 保留中のtapを先に決める（読み出しの失敗に関わらず）。
        let confirmed = self
            .filter
            .on_tick(now_ms, session_established, touch_blocks);
        if let Some(decision) = confirmed {
            self.log_decision("tap_confirmed", decision, None);
        }
        events.tapped = confirmed == Some(Decision::Send);
        let int_source = self.accel.read_int_source(&mut self.i2c.borrow_mut());
        match int_source {
            Ok(int_source) => {
                self.consecutive_errors = 0;
                // 揺れ（Activity）は、tapと同じ`INT_SOURCE`の読みから取る。tapの判定より先に見ても、tapの扱いは変わらない。
                if deskcat_tap::shake::is_activity(int_source) {
                    if let Some(shake) = self.shake.as_mut() {
                        let decision = shake.on_activity(now_ms, session_established);
                        events.shaken = decision == ShakeDecision::Send;
                        if decision != ShakeDecision::Counting {
                            let c = shake.counters();
                            log::info!(
                                "shake_detected decision={decision:?} int_source=0x{int_source:02x} activity_hits={} sent={} dropped_no_session={} dropped_too_soon={}",
                                c.activity_hits,
                                c.sent,
                                c.dropped_no_session,
                                c.dropped_too_soon
                            );
                        }
                    }
                }
                if !deskcat_tap::is_single_tap(int_source) {
                    return events;
                }
                let decision = self
                    .filter
                    .on_tap(now_ms, session_established, touch_blocks);
                self.log_decision("tap_detected", decision, Some(int_source));
                events.tapped |= decision == Decision::Send;
                events
            }
            Err(err) => {
                self.consecutive_errors += 1;
                log::error!(
                    "tap_read_failed error={err} consecutive={}",
                    self.consecutive_errors
                );
                if self.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    self.enabled = false;
                    // 保留中の1件は、検出が止まると決められず、送られない（`on_tick`が呼ばれなくなる）。数えて log に残す。
                    let pending_tap_lost = self.filter.abandon_pending();
                    log::error!(
                        "tap_disabled consecutive_errors={} pending_tap_lost={pending_tap_lost} dropped_detection_stopped={}",
                        self.consecutive_errors,
                        self.filter.counters().dropped_detection_stopped
                    );
                }
                events
            }
        }
    }

    fn log_decision(&self, event: &str, decision: Decision, int_source: Option<u8>) {
        let c = self.filter.counters();
        let int_source = int_source.map_or(String::new(), |v| format!(" int_source=0x{v:02x}"));
        log::info!(
            "{event} decision={decision:?}{int_source} sent={} dropped_no_session={} dropped_touch={} dropped_too_soon={} dropped_detection_stopped={}",
            c.sent,
            c.dropped_no_session,
            c.dropped_touch,
            c.dropped_too_soon,
            c.dropped_detection_stopped
        );
    }
}
