//! 製品buildの`ENV-01`（BME280）の周期測定と、`environment` eventの材料づくり（[#487](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487)）。
//!
//! 製品build（featureなし）だけがこのmoduleを持つ。`crate::tap::TapDetector`と同じI2C bus（`crate::i2c_bus`）を、
//! main loopの同じtaskから借りる。**ISRからは呼ばない。**
//!
//! # 設定と周期
//!
//! 設定は、Bosch BME280 Data Sheet BST-BME280-DS001-24 Revision 1.24 §3.5.1 Table 7（weather monitoring）の
//! 推奨設定である。**forced mode、1 sample／分、気圧・温度・湿度とも×1、IIR filter off。**
//! `ctrl_hum`と`ctrl_meas`の値は`crate::env`が持つ（`CTRL_HUM`＝`0x01`、`CTRL_MEAS_FORCED`＝`0x25`。
//! §5.4.3 Table 20、§5.4.5 Tables 23–25。`config` registerは書かないので、IIR filterはreset値のoffである）。
//! 周期は[`PERIOD_MS`]で、**測定の開始から次の開始まで**である。測定を始めるたびに`ctrl_hum`、`ctrl_meas`の順に書く
//! （forced測定のたびに`ctrl_meas`のmodeを書き直す必要があり、`ctrl_hum`は`ctrl_meas`への書き込みの後に有効になる。§5.4.3）。
//!
//! # 変換を待つ間、main loopを止めない
//!
//! 測定を始めたら、すぐにmain loopへ戻る。読みに行くのは、変換の時間が過ぎた後の周回である。
//! 変換の時間は§9.1の式から取る。最大は`1.25 + 2.3·T + (2.3·P + 0.575) + (2.3·H + 0.575)` ms（T・P・Hはoversampling倍率）で、
//! ×1／×1／×1は9.3 msである（`crate::env_sample`のdocが同じ式を引く）。[`CONVERSION_MAX_MS`]はそれを切り上げた値である。
//! 読みに行って`measuring`が立っていたら、[`POLL_INTERVAL_MS`]後に読み直し、[`DEADLINE_MS`]を過ぎても終わらなければ
//! `not_ready`として数え、その周期の測定を捨てる。**1周回で行うI2C transactionは3回以下である**（下の「1周回の仕事」）。
//! 実際に待つのは1 tick（`CONFIG_FREERTOS_HZ=100`で10 ms）単位である。
//!
//! # 1周回の仕事（bus時間の上界）
//!
//! | 段 | transaction |
//! |---|---|
//! | Chip IDの確認（成功するまで毎周期。一致した後は読まない） | 1 |
//! | 補正係数（成功するまで毎周期。成功した後は読まない） | 3（status、2領域） |
//! | 測定の開始 | 2（`ctrl_hum`、`ctrl_meas`） |
//! | 読み出し | 2（status、raw burst） |
//!
//! 1 transactionのtimeoutは`config::I2C_TRANSACTION_TIMEOUT_MS`である。**失敗したときの1周回は、transaction数×この上限
//! （とbus clear）だけ延びうる。**tapの読みは、その分だけ遅れうるが、tap bitは読むまで残るので検出は落ちない（`crate::i2c_bus`）。
//!
//! # 送ってよい値（`environment` event）
//!
//! 測定が1回終わるたびに、範囲内の量だけを[`Environment`]へ詰める（`deskcat_env::Reading::publishable`）。**範囲外の量は省き、
//! `out_of_range`へ数える。**送れる量が1つも無ければ、eventを作らない。**確立前に作ったeventは最新の1件だけ持ち**
//! （古い1件は`superseded`へ数えて捨てる）、sessionが確立した後の最初の周回で送る。**送った後は持たない**（同じ値を送り直さない）。
//! **欠測の周期（失敗、範囲外のみ、送れる量が無い周期）は、持っている未送出の1件も捨てる**（`stale_dropped`）。値は測定時刻を持たないため、
//! 前の周期の値を、欠測の後の値として送らないためである。
//! 送る条件・周期・欠測・範囲は`docs/protocol/esp32-pi-protocol.md`の§4.5が正本である（**ここへ再掲しない**）。
//!
//! # 失敗の扱い
//!
//! どの失敗もpanicせず、その周期の測定を捨てて、次の周期にやり直す。種別ごとのcounterをlogへ出す
//! （`env_fault kind=... stage=... count=N`。`crate::env_sample`と同じ分類）。chip IDが`0x60`以外なら、別のdeviceが
//! いるとみなして**何も書かず**、二度と動かさない（`identity`）。
//!
//! # 確かめていないこと
//!
//! **このmoduleの振る舞いは、host testでも実機でも確かめていない。**`firmware/esp32`はhostのworkspaceから除外され、
//! `esp_idf_svc`へ依存するため、host testでは動かせない。範囲の判定は`deskcat_env`のhost testが固定している。
//! 実機の確認（`ENV-01`が周期どおりに測れること、`environment`がPiへ届くこと、tapの検出が遅れないこと）は、
//! 実機の統合が持つ。**周期と変換待ちの実測は無い。**

use deskcat_env::{
    check_calibration_ready, check_identity, check_measurement_ready, classify_io_error,
    compensate, decode_calibration, decode_raw, Calibration, IoFault, RegisterFault,
};
use deskcat_protocol::{Environment, FiniteF32};
use esp_idf_svc::sys::{EspError, ESP_ERR_TIMEOUT};

use crate::env::Bme280;
use crate::i2c_bus::SharedI2c;

/// 測定の開始から次の開始までの間隔（ms）。Bosch Table 7の1 sample／分。
pub const PERIOD_MS: u64 = 60_000;
/// 変換を始めてから、`measuring`を読みに行くまでの時間（ms）。§9.1の最大9.3 ms（×1／×1／×1）の切り上げ。
const CONVERSION_MAX_MS: u64 = 10;
/// `measuring`がまだ立っていたとき、読み直すまでの間隔（ms）。
const POLL_INTERVAL_MS: u64 = 10;
/// 変換の開始から、`not_ready`と数えるまでの時間（ms）。最大の約3倍（`crate::env_sample`の×1と同じ）。
const DEADLINE_MS: u64 = 30;

#[derive(Clone, Copy)]
enum State {
    /// 次の測定の開始時刻（uptimeのms）まで待つ。
    Idle { next_start_ms: u64 },
    /// Chip IDを読む。
    CheckId { cycle_ms: u64 },
    /// 補正係数を読む。
    ReadCalibration { cycle_ms: u64 },
    /// `ctrl_hum`、`ctrl_meas`の順に書き、forced測定を始める。
    Start { cycle_ms: u64 },
    /// 変換を待つ。
    Wait {
        cycle_ms: u64,
        next_poll_ms: u64,
        deadline_ms: u64,
    },
    /// Chip IDが違う。二度と動かさない。
    Disabled,
}

/// protocol counterではない。faultのlogごとに更新後の件数を出す。
#[derive(Default)]
struct Counters {
    identity: u32,
    io_timeout: u32,
    io_other: u32,
    not_ready: u32,
    invalid_data: u32,
    compensation: u32,
    out_of_range: u32,
    empty: u32,
    superseded: u32,
    stale_dropped: u32,
    samples: u32,
    events: u32,
}

pub struct EnvReporter {
    bus: SharedI2c,
    sensor: Bme280,
    state: State,
    id_ok: bool,
    calibration: Option<Calibration>,
    pending: Option<Environment>,
    counters: Counters,
}

impl EnvReporter {
    /// **まだ通信しない。**
    pub fn new(bus: SharedI2c, address: u8, now_ms: u64) -> Self {
        Self {
            bus,
            sensor: Bme280::new(address),
            state: State::Idle {
                next_start_ms: now_ms,
            },
            id_ok: false,
            calibration: None,
            pending: None,
            counters: Counters::default(),
        }
    }

    /// 次にpollする時刻（uptimeのms）。測定の段は「すぐ」（0）を返し、main loopを眠らせない。
    pub fn next_deadline_ms(&self) -> Option<u64> {
        match self.state {
            State::Idle { next_start_ms } => Some(next_start_ms),
            State::Wait { next_poll_ms, .. } => Some(next_poll_ms),
            State::CheckId { .. } | State::ReadCalibration { .. } | State::Start { .. } => Some(0),
            State::Disabled => None,
        }
    }

    /// 送るべき`environment`があれば取り出す。**取り出したら持たない。**呼び出し側は、sessionが確立している間だけ呼ぶ。
    pub fn take_event(&mut self) -> Option<Environment> {
        let event = self.pending.take();
        if event.is_some() {
            self.counters.events = self.counters.events.saturating_add(1);
        }
        event
    }

    /// 時刻に達していれば、測定を1段進める。**1回の呼び出しで、I2C transactionは3回以下である。**
    pub fn poll(&mut self, now_ms: u64) {
        match self.state {
            State::Idle { next_start_ms } => {
                if now_ms >= next_start_ms {
                    self.state = if !self.id_ok {
                        State::CheckId { cycle_ms: now_ms }
                    } else if self.calibration.is_none() {
                        State::ReadCalibration { cycle_ms: now_ms }
                    } else {
                        State::Start { cycle_ms: now_ms }
                    };
                }
            }
            State::CheckId { cycle_ms } => self.check_id(cycle_ms),
            State::ReadCalibration { cycle_ms } => self.read_calibration(cycle_ms),
            State::Start { cycle_ms } => self.start(cycle_ms, now_ms),
            State::Wait {
                cycle_ms,
                next_poll_ms,
                deadline_ms,
            } => {
                if now_ms >= next_poll_ms {
                    self.read_sample(cycle_ms, now_ms, deadline_ms);
                }
            }
            State::Disabled => {}
        }
    }

    /// 欠測の周期を終える。**持っている未送出の1件も捨てる**（前の周期の値を、欠測の後の周期の値として送らない。
    /// 測定時刻を持たないので、古さを後から確かめられない）。捨てたら`stale_dropped`へ数える。
    fn fail_cycle(&mut self, cycle_ms: u64) {
        if self.pending.take().is_some() {
            self.counters.stale_dropped = self.counters.stale_dropped.saturating_add(1);
            log::warn!(
                "env_event_stale_dropped count={}",
                self.counters.stale_dropped
            );
        }
        self.finish_cycle(cycle_ms);
    }

    /// 次の周期へ戻す。開始時刻は、今の周期の開始から`PERIOD_MS`後である。
    fn finish_cycle(&mut self, cycle_ms: u64) {
        self.state = State::Idle {
            next_start_ms: cycle_ms.saturating_add(PERIOD_MS),
        };
    }

    fn check_id(&mut self, cycle_ms: u64) {
        let read = self.sensor.read_chip_id(&mut self.bus.borrow_mut());
        match read {
            Ok(id) if check_identity(id).is_ok() => {
                self.id_ok = true;
                // 続けて補正係数を読む（次の周回）。
                self.state = State::ReadCalibration { cycle_ms };
            }
            Ok(id) => {
                self.counters.identity = self.counters.identity.saturating_add(1);
                log::error!(
                    "env_fault kind=chip_id_mismatch raw=0x{id:02x} count={}",
                    self.counters.identity
                );
                self.state = State::Disabled;
            }
            Err(err) => self.io_fault("chip_id", err, cycle_ms),
        }
    }

    fn read_calibration(&mut self, cycle_ms: u64) {
        let status = self.sensor.read_status(&mut self.bus.borrow_mut());
        match status {
            Ok(status) if check_calibration_ready(status).is_ok() => {}
            Ok(_) => return self.not_ready("calibration_not_ready", cycle_ms),
            Err(err) => return self.io_fault("status_calibration", err, cycle_ms),
        }
        let read = self.sensor.read_calibration(&mut self.bus.borrow_mut());
        match read {
            Ok((tp, h)) => {
                let c = decode_calibration(&tp, &h);
                if c.dig_p1 == 0 {
                    self.invalid_data("calibration_p1_zero", cycle_ms);
                } else {
                    self.calibration = Some(c);
                    log::info!(
                        "env_calibration_read dig_p1={} dig_h4={} dig_h5={}",
                        c.dig_p1,
                        c.dig_h4,
                        c.dig_h5
                    );
                    self.state = State::Start { cycle_ms };
                }
            }
            Err(err) => self.io_fault("calibration", err, cycle_ms),
        }
    }

    fn start(&mut self, cycle_ms: u64, now_ms: u64) {
        let humidity = self.sensor.configure_humidity(&mut self.bus.borrow_mut());
        if let Err(err) = humidity {
            return self.io_fault("ctrl_hum_write", err, cycle_ms);
        }
        let forced = self.sensor.start_forced(&mut self.bus.borrow_mut());
        match forced {
            Ok(()) => {
                // 周期は、測定の開始（この周回の時刻）から数える。
                let started_ms = now_ms.max(cycle_ms);
                self.state = State::Wait {
                    cycle_ms: started_ms,
                    next_poll_ms: started_ms + CONVERSION_MAX_MS,
                    deadline_ms: started_ms + DEADLINE_MS,
                };
            }
            Err(err) => self.io_fault("ctrl_meas_write", err, cycle_ms),
        }
    }

    fn read_sample(&mut self, cycle_ms: u64, now_ms: u64, deadline_ms: u64) {
        let status = self.sensor.read_status(&mut self.bus.borrow_mut());
        match status {
            Ok(status) if check_measurement_ready(status).is_ok() => {}
            Ok(_) if now_ms > deadline_ms => {
                return self.not_ready("measurement_timeout", cycle_ms)
            }
            Ok(_) => {
                // 変換中。main loopを止めず、次の締切で読み直す。
                self.state = State::Wait {
                    cycle_ms,
                    next_poll_ms: now_ms + POLL_INTERVAL_MS,
                    deadline_ms,
                };
                return;
            }
            Err(err) => return self.io_fault("status_measurement", err, cycle_ms),
        }
        let read = self.sensor.read_raw(&mut self.bus.borrow_mut());
        let bytes = match read {
            Ok(bytes) => bytes,
            Err(err) => return self.io_fault("raw", err, cycle_ms),
        };
        let raw = match decode_raw(&bytes) {
            Ok(raw) => raw,
            Err(RegisterFault::SkippedChannel) => {
                return self.invalid_data("skipped_channel", cycle_ms)
            }
            Err(_) => return self.invalid_data("raw_register_fault", cycle_ms),
        };
        // 補正係数を読んだ後にしかここへ来ない。状態の破綻も、faultとして数える。
        let Some(calibration) = self.calibration.as_ref() else {
            return self.invalid_data("calibration_missing", cycle_ms);
        };
        let reading = match compensate(raw, calibration) {
            Ok(reading) => reading,
            Err(err) => {
                self.counters.compensation = self.counters.compensation.saturating_add(1);
                log::error!(
                    "env_fault kind=compensation detail={err:?} count={}",
                    self.counters.compensation
                );
                return self.fail_cycle(cycle_ms);
            }
        };
        self.counters.samples = self.counters.samples.saturating_add(1);
        let publishable = reading.publishable();
        if publishable.out_of_range > 0 {
            self.counters.out_of_range = self
                .counters
                .out_of_range
                .saturating_add(u32::from(publishable.out_of_range));
            log::warn!(
                "env_out_of_range quantities={} count={}",
                publishable.out_of_range,
                self.counters.out_of_range
            );
        }
        if publishable.is_empty() {
            self.counters.empty = self.counters.empty.saturating_add(1);
            log::warn!("env_no_publishable_quantity count={}", self.counters.empty);
            // 送れる量が無い周期も欠測である。持っている1件を送り続けない。
            return self.fail_cycle(cycle_ms);
        } else {
            // 値が有限であることは`publishable`が範囲で保証する。`None`になる経路は無いが、省いても安全側に倒れる。
            let event = Environment {
                temperature_c: publishable.temperature_c.and_then(FiniteF32::new),
                humidity_pct: publishable.humidity_pct.and_then(FiniteF32::new),
                pressure_hpa: publishable.pressure_hpa.and_then(FiniteF32::new),
            };
            if self.pending.replace(event).is_some() {
                self.counters.superseded = self.counters.superseded.saturating_add(1);
                log::warn!(
                    "env_event_superseded count={} (session not established)",
                    self.counters.superseded
                );
            }
            log::info!(
                "env_reading temperature_centi_c={} pressure_q24_8_pa={} humidity_q22_10_percent={} samples={}",
                reading.temperature_centi_c,
                reading.pressure_q24_8_pa,
                reading.humidity_q22_10_percent,
                self.counters.samples
            );
        }
        self.finish_cycle(cycle_ms);
    }

    fn io_fault(&mut self, stage: &'static str, err: EspError, cycle_ms: u64) {
        if classify_io_error(err.code(), ESP_ERR_TIMEOUT) == IoFault::Timeout {
            self.counters.io_timeout = self.counters.io_timeout.saturating_add(1);
            log::error!(
                "env_fault kind=i2c_timeout stage={stage} error={err} count={}",
                self.counters.io_timeout
            );
        } else {
            // ESP-IDFの一般的なI2C errorだけからは、配線断とNACKを区別できない。
            self.counters.io_other = self.counters.io_other.saturating_add(1);
            log::error!(
                "env_fault kind=i2c_other stage={stage} error={err} count={}",
                self.counters.io_other
            );
        }
        self.fail_cycle(cycle_ms);
    }

    fn not_ready(&mut self, kind: &'static str, cycle_ms: u64) {
        self.counters.not_ready = self.counters.not_ready.saturating_add(1);
        log::error!("env_fault kind={kind} count={}", self.counters.not_ready);
        self.fail_cycle(cycle_ms);
    }

    fn invalid_data(&mut self, kind: &'static str, cycle_ms: u64) {
        self.counters.invalid_data = self.counters.invalid_data.saturating_add(1);
        log::error!("env_fault kind={kind} count={}", self.counters.invalid_data);
        self.fail_cycle(cycle_ms);
    }
}
