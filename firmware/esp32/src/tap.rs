//! ADXL345のtap検出を、main loopの中で周期的に読む（[#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)、F1）。
//!
//! `tap-21` feature付きbuildだけがこのmoduleを使う。起動時のI2C bring-upが返す`I2cDriver`を持ち続け、
//! [`TapDetector::configure`]でtap検出を設定し、[`TapDetector::poll`]が[`POLL_INTERVAL_MS`]ごとに
//! `INT_SOURCE`をI2Cで読む（**割り込みのINT線は使わない。配線は要らない**）。tapを見つけたら
//! `deskcat_tap::TapFilter`が送ってよいかを決める（bootのACK後だけ、前の`tapped`から最小間隔以上）。
//! 設定値・出所・暫定であることは`deskcat_tap`のcrate docが持つ（**ここへ再掲しない**）。
//!
//! # I2C driverの所有
//!
//! `tap-21`付きbuildでは、`TapDetector`が起動時のI2C bring-upの`I2cDriver`を唯一の所有者として持ち続ける。そのため、
//! bring-upの後に`ENV-01`（BME280）をI2Cで読む経路は、このbuildには無い（今は誰も読まない）。
//!
//! # 失敗の扱い
//!
//! I2Cの読み出しが失敗しても止まらない（1 transactionは有限時間で返る。上限は`config::I2C_TRANSACTION_TIMEOUT_MS`の100 ms。
//! 設定の書き込みは6 transactionと捨て読み1 transactionなので最大700 ms、読み出しの失敗は1周回あたり最大100 ms）。
//! **設定（`configure`）が走る場所はbuildで違う。**`bringup-display-13`を持たないbuildは、起動時のI2C bring-upの直後、
//! main loopの前で走る（`BootSession::start`の前でもあるため、bus固着のときは最大700 msが最初の`boot`の送出を遅らせる）。
//! `bringup-display-13`付きbuildは、LCDのbring-upが済んだ後にmain loopの1周回の中で走る（その1周回が最大700 ms延びる）。
//! 読み出しは、いずれのbuildでもmain loopの1周回に乗る。[`MAX_CONSECUTIVE_ERRORS`]回続けて失敗したら、
//! 検出をやめる（bus固着のとき、main loopを毎周回I2C timeoutで塞がないため）。やめたことはlogへ出す。
//!
//! # 確かめていないこと
//!
//! **このmoduleの振る舞いは、host testでも実機でも確かめていない。**`firmware/esp32`はhostのworkspaceから除外され、
//! `esp_idf_svc`へ依存するため、host testでは動かせない。判定は`deskcat_tap`のhost testが固定している。
//! 実機の確認（指で軽く叩いて`tapped`がPiへ届くこと、閾値の適否、誤検出）は#21の実機統合が持つ。`configure`は、測定modeへ入れた直後に`INT_SOURCE`を
//! 1回読んで捨てる（過渡でtapのbitが立つかは確かめていない。立っても誤った`tapped`を送らないための備えである）。

// 既定buildは`main()`から使わないためdead_codeになる（`tap-21` featureが無いと呼ばない）。moduleごと`#[cfg]`で
// 落とさないのは、既定buildでもcross-compileを確認し続けるためである（`crate::display`と同じ）。
#![allow(dead_code)]

use deskcat_tap::{Decision, TapFilter};
use esp_idf_svc::hal::i2c::I2cDriver;

use crate::accel::Adxl345;

/// `INT_SOURCE`を読む間隔（ms）。**暫定の一般値である**（安全要件の5項目に効かない）。データレート（`deskcat_tap::BW_RATE_100_HZ`＝100 Hz＝10 ms）の
/// 2倍にした。tapのbitは読むまで残るため、読み損ねは起きない。
pub const POLL_INTERVAL_MS: u64 = 20;
/// 続けて失敗したらtap検出をやめる回数。暫定の一般値である。
pub const MAX_CONSECUTIVE_ERRORS: u32 = 5;

/// tap検出の状態。
pub struct TapDetector {
    accel: Adxl345,
    i2c: I2cDriver<'static>,
    filter: TapFilter,
    next_poll_ms: u64,
    consecutive_errors: u32,
    enabled: bool,
}

impl TapDetector {
    /// 起動時のI2C bring-upが返した`I2cDriver`を受け取る。**まだ設定は書かない。**
    pub fn new(accel: Adxl345, i2c: I2cDriver<'static>) -> Self {
        Self {
            accel,
            i2c,
            filter: TapFilter::new(),
            next_poll_ms: 0,
            consecutive_errors: 0,
            enabled: false,
        }
    }

    /// tap検出の設定を書く。失敗したら検出を始めない（logへ出す）。
    pub fn configure(&mut self, now_ms: u64) {
        match self.accel.configure_tap(&mut self.i2c) {
            Ok(()) => {
                // 測定modeへ入れた直後にtapのbitが過渡で立っていても、誤った`tapped`を送らないよう、
                // `INT_SOURCE`を1回読んで捨てる（読むとtapのbitが消える。失敗しても検出は始める）。
                match self.accel.read_int_source(&mut self.i2c) {
                    Ok(discarded) => {
                        log::info!("tap_configure_discard int_source=0x{discarded:02x}")
                    }
                    Err(err) => log::warn!("tap_configure_discard_failed error={err}"),
                }
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

    /// 時刻に達していれば`INT_SOURCE`を1回読む。**`tapped`を送るべきなら`true`を返す。**
    pub fn poll(&mut self, now_ms: u64, session_established: bool) -> bool {
        if !self.enabled || now_ms < self.next_poll_ms {
            return false;
        }
        // 次の期限は`now_ms`から積む（前の期限からではない）。main loopの1周回が延びた分だけ間隔が後ろへずれるが、
        // tapのbitは読むまで残るため、検出漏れにはならない。
        self.next_poll_ms = now_ms + POLL_INTERVAL_MS;
        match self.accel.read_int_source(&mut self.i2c) {
            Ok(int_source) => {
                self.consecutive_errors = 0;
                if !deskcat_tap::is_single_tap(int_source) {
                    return false;
                }
                let decision = self.filter.on_tap(now_ms, session_established);
                let c = self.filter.counters();
                log::info!(
                    "tap_detected decision={decision:?} int_source=0x{int_source:02x} sent={} dropped_no_session={} dropped_too_soon={}",
                    c.sent,
                    c.dropped_no_session,
                    c.dropped_too_soon
                );
                decision == Decision::Send
            }
            Err(err) => {
                self.consecutive_errors += 1;
                log::error!(
                    "tap_read_failed error={err} consecutive={}",
                    self.consecutive_errors
                );
                if self.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    self.enabled = false;
                    log::error!(
                        "tap_disabled consecutive_errors={}",
                        self.consecutive_errors
                    );
                }
                false
            }
        }
    }
}
