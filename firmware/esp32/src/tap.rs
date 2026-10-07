//! ADXL345のtap検出を、main loopの中で周期的に読む（[#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)、F1）。
//!
//! `tap-21` feature付きbuildだけがこのmoduleを使う。起動時のI2C bring-upが返す`I2cDriver`を持ち続け、
//! [`TapDetector::configure`]でtap検出を設定し、[`TapDetector::poll`]が[`POLL_INTERVAL_MS`]ごとに
//! `INT_SOURCE`をI2Cで読む（**割り込みのINT線は使わない。配線は要らない**）。tapを見つけたら
//! `deskcat_tap::TapFilter`が送ってよいかを決める（bootのACK後だけ、前の`tapped`から500 ms以上）。
//! 設定値・出所・暫定であることは`deskcat_tap`のcrate docが持つ（**ここへ再掲しない**）。
//!
//! # 失敗の扱い
//!
//! I2Cの読み出しが失敗しても止まらない（1 transactionは有限時間で返る）。[`MAX_CONSECUTIVE_ERRORS`]回続けて失敗したら、
//! 検出をやめる（bus固着のとき、main loopを毎周回I2C timeoutで塞がないため）。やめたことはlogへ出す。
//!
//! # 確かめていないこと
//!
//! **このmoduleの振る舞いは、host testでも実機でも確かめていない。**`firmware/esp32`はhostのworkspaceから除外され、
//! `esp_idf_svc`へ依存するため、host testでは動かせない。判定は`deskcat_tap`のhost testが固定している。
//! 実機の確認（撫でて`tapped`がPiへ届くこと、閾値の適否、誤検出）は#21の実機統合が持つ。

// 既定buildは`main()`から使わないためdead_codeになる（`tap-21` featureが無いと呼ばない）。moduleごと`#[cfg]`で
// 落とさないのは、既定buildでもcross-compileを確認し続けるためである（`crate::display`と同じ）。
#![allow(dead_code)]

use deskcat_tap::{Decision, TapCounters, TapFilter};
use esp_idf_svc::hal::i2c::I2cDriver;

use crate::accel::Adxl345;

/// `INT_SOURCE`を読む間隔（ms）。**暫定の一般値である**（安全要件の5項目に効かない）。データレート（reset値100 Hz＝10 ms）の
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
                self.enabled = true;
                self.next_poll_ms = now_ms + POLL_INTERVAL_MS;
                log::info!(
                    "tap_configured thresh_tap=0x{:02x} dur=0x{:02x} tap_axes=0x{:02x} poll_interval_ms={POLL_INTERVAL_MS}",
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

    /// これまでのtapの扱いの内訳。
    pub fn counters(&self) -> TapCounters {
        self.filter.counters()
    }
}
