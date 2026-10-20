//! #15 の試験 build 専用の ADXL345 連続測定。bus はこの main-loop task が所有する。
//!
//! 設定値・復号・統計は `deskcat_accel` が持つ（host test 済み。ここへ再掲しない）。
//! `POLL_INTERVAL_MS` ごとに `FIFO_STATUS` を読み、溜まった件数だけ（上限 [`MAX_READS_PER_POLL`]）
//! 6 byte を読み出す。FIFO は Stream mode（最後の 32 件）なので、main loop の周期が
//! 揺れても 320 ms（100 Hz で 32 件）までは取りこぼさない。`INT_SOURCE` の Overrun を
//! poll ごとに、**FIFO を読む前に**読み、立った回数を数える（立てば FIFO が満杯になり古い件が捨てられている）。
//!
//! # 出力
//!
//! すべて UART0 の log。`accel_raw` は設定後の最初の [`RAW_LOG_SAMPLES`] 件（warm-up を除く）の
//! 生値（LSB）。`accel_window` は [`WINDOW_SAMPLES`] 件ごとの平均・標準偏差（mLSB は LSB の 1/1000）と
//! 最小・最大（LSB）。[`WINDOWS`] 窓で終わる。**この file の現在の手順は、実機で動かした記録が無い。**
//! 2026-10-17 に実機へ書き込んだ版（`docs/hardware/experiment-log.md` の `EXP-033`）は、`INT_SOURCE` の Overrun を
//! FIFO の読み出しの**後**に読み、連続失敗の数え方も今と違う。`mean_mg_typ` も、平均を LSB へ切り捨ててから換算していた（今は mLSB から 1 回の丸めで換算する）。
//!
//! # 確かめていないこと
//!
//! - host test は `deskcat_accel` の復号と統計だけである。この file の現在の I2C 手順は ESP32 build と Clippy までで、
//!   実機では動かしていない（`EXP-033` の版との違いは上のとおり）。overrun を起こす条件や I2C のエラー時の挙動も確かめていない。
//! - warm-up の件数 [`WARMUP_SAMPLES`] は、turn-on 時間（100 Hz で約 11.1 ms、Rev. E Table 1 注）より
//!   十分長い一般値であり、安全要件の 5 項目に効かない。
//! - 窓の長さと窓数は、noise を見るための暫定の一般値。

use deskcat_accel::{
    decode_sample, fifo_entries, is_overrun, mean_magnitude_mlsb, mlsb_to_mg, SampleFault, Stats,
};
use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::sys::{EspError, ESP_ERR_TIMEOUT};

use crate::accel::Adxl345;

/// `FIFO_STATUS` を読む間隔（ms）。FIFO の保持時間（32 件÷100 Hz＝320 ms）より十分短い。
pub const POLL_INTERVAL_MS: u64 = 20;
/// 1 回の poll で読む最大件数。main loop を I2C で塞ぎすぎない上限（1 件は 100 kHz で約 1 ms）。
pub const MAX_READS_PER_POLL: u8 = 8;
/// 設定後に捨てる件数（100 Hz で 100 ms）。
pub const WARMUP_SAMPLES: u32 = 10;
/// 生値を 1 件ずつ log に出す件数。
pub const RAW_LOG_SAMPLES: u32 = 100;
/// 1 窓の件数（100 Hz で約 5 秒）。
pub const WINDOW_SAMPLES: u32 = 500;
/// 窓の数。
pub const WINDOWS: u32 = 6;
/// 続けて失敗したら止める回数。
pub const MAX_CONSECUTIVE_ERRORS: u32 = 5;

#[derive(Default)]
struct Counters {
    io_timeout: u32,
    io_other: u32,
    out_of_range: u32,
    overrun_polls: u32,
    polls: u32,
    samples: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Configure,
    Collect,
    Done,
}

pub struct AccelSampler {
    driver: I2cDriver<'static>,
    sensor: Adxl345,
    state: State,
    next_poll_ms: u64,
    consecutive_errors: u32,
    warmup_left: u32,
    stats: Stats,
    window: u32,
    counters: Counters,
}

impl AccelSampler {
    /// Device ID の一致は呼び出し側（`main.rs`）が確かめてから作る。**まだ何も書かない。**
    pub fn new(driver: I2cDriver<'static>, sensor: Adxl345, now_ms: u64) -> Self {
        Self {
            driver,
            sensor,
            state: State::Configure,
            next_poll_ms: now_ms,
            consecutive_errors: 0,
            warmup_left: WARMUP_SAMPLES,
            stats: Stats::new(),
            window: 0,
            counters: Counters::default(),
        }
    }

    /// 次に動く時刻（uptime の ms）。終わっていれば `None`。
    pub fn next_deadline_ms(&self) -> Option<u64> {
        (self.state != State::Done).then_some(self.next_poll_ms)
    }

    /// true なら試行は終了。失敗後に新たな書き込みをしない。
    pub fn poll(&mut self, now_ms: u64) -> bool {
        if self.state == State::Done || now_ms < self.next_poll_ms {
            return self.state == State::Done;
        }
        self.next_poll_ms = now_ms + POLL_INTERVAL_MS;
        match self.state {
            State::Configure => match self.sensor.configure_sampling(&mut self.driver) {
                Ok(()) => {
                    self.state = State::Collect;
                    log::info!(
                        "accel_sample_configured data_format=0x{:02x} bw_rate=0x{:02x} fifo_ctl=0x{:02x} poll_interval_ms={POLL_INTERVAL_MS} warmup={WARMUP_SAMPLES} window={WINDOW_SAMPLES} windows={WINDOWS}",
                        deskcat_accel::DATA_FORMAT_FULL_RES_2G,
                        deskcat_accel::BW_RATE_100_HZ,
                        deskcat_accel::FIFO_CTL_STREAM
                    );
                }
                Err(err) => self.io_fault("configure", &err),
            },
            State::Collect => self.collect(),
            State::Done => {}
        }
        self.state == State::Done
    }

    fn collect(&mut self) {
        self.counters.polls = self.counters.polls.saturating_add(1);
        let entries = match self.sensor.read_fifo_status(&mut self.driver) {
            Ok(status) => fifo_entries(status),
            Err(err) => return self.io_fault("fifo_status", &err),
        };
        // Overrun は FIFO 内容を読むと消える。**読み出しの前に**確かめ、立っていれば取りこぼしを数える。
        match self.sensor.read_int_source(&mut self.driver) {
            Ok(src) if is_overrun(src) => {
                self.counters.overrun_polls = self.counters.overrun_polls.saturating_add(1);
                log::warn!(
                    "accel_overrun int_source=0x{src:02x} polls_with_overrun={}",
                    self.counters.overrun_polls
                );
            }
            Ok(_) => {}
            Err(err) => return self.io_fault("int_source", &err),
        }
        for _ in 0..entries.min(MAX_READS_PER_POLL) {
            let bytes = match self.sensor.read_sample_bytes(&mut self.driver) {
                Ok(b) => b,
                Err(err) => return self.io_fault("data", &err),
            };
            match decode_sample(&bytes) {
                Ok(sample) => self.accept(sample),
                Err(SampleFault::OutOfRange) => {
                    self.counters.out_of_range = self.counters.out_of_range.saturating_add(1);
                    log::error!(
                        "accel_fault kind=out_of_range raw={bytes:02x?} count={}",
                        self.counters.out_of_range
                    );
                }
            }
            if self.state == State::Done {
                return;
            }
        }
        // 1 周回の I/O（FIFO_STATUS・INT_SOURCE・各件の読み出し）が全部成功したときだけ数え直す。
        // 途中の段が成功しても戻さない（戻すと、後ろの段の連続失敗で停止しない）。
        self.consecutive_errors = 0;
    }

    fn accept(&mut self, sample: deskcat_accel::Sample) {
        if self.warmup_left > 0 {
            self.warmup_left -= 1;
            return;
        }
        self.counters.samples = self.counters.samples.saturating_add(1);
        if self.counters.samples <= RAW_LOG_SAMPLES {
            log::info!(
                "accel_raw n={} x={} y={} z={}",
                self.counters.samples,
                sample.x,
                sample.y,
                sample.z
            );
        }
        self.stats.push(sample);
        if self.stats.count() >= WINDOW_SAMPLES {
            self.finish_window();
        }
    }

    fn finish_window(&mut self) {
        self.window += 1;
        let (Some(x), Some(y), Some(z)) =
            (self.stats.axis(0), self.stats.axis(1), self.stats.axis(2))
        else {
            self.state = State::Done;
            return;
        };
        let magnitude = mean_magnitude_mlsb(&self.stats).unwrap_or(0);
        log::info!(
            "accel_window w={} n={} mean_mlsb=[{},{},{}] stddev_mlsb=[{},{},{}] min=[{},{},{}] max=[{},{},{}] mean_mg_typ=[{},{},{}] magnitude_mlsb={magnitude} polls={} overrun_polls={} out_of_range={}",
            self.window,
            self.stats.count(),
            x.mean_mlsb, y.mean_mlsb, z.mean_mlsb,
            x.stddev_mlsb, y.stddev_mlsb, z.stddev_mlsb,
            x.min, y.min, z.min,
            x.max, y.max, z.max,
            mlsb_to_mg(x.mean_mlsb),
            mlsb_to_mg(y.mean_mlsb),
            mlsb_to_mg(z.mean_mlsb),
            self.counters.polls,
            self.counters.overrun_polls,
            self.counters.out_of_range
        );
        self.stats = Stats::new();
        if self.window >= WINDOWS {
            log::info!("accel_sample_done windows={}", self.window);
            self.state = State::Done;
        }
    }

    fn io_fault(&mut self, stage: &'static str, err: &EspError) {
        if err.code() == ESP_ERR_TIMEOUT {
            self.counters.io_timeout = self.counters.io_timeout.saturating_add(1);
            log::error!(
                "accel_fault kind=i2c_timeout stage={stage} error={err} count={}",
                self.counters.io_timeout
            );
        } else {
            self.counters.io_other = self.counters.io_other.saturating_add(1);
            log::error!(
                "accel_fault kind=i2c_other stage={stage} error={err} count={}",
                self.counters.io_other
            );
        }
        self.consecutive_errors += 1;
        // 設定の失敗は 1 回で止める（書き込みを繰り返さない）。読み出しは続けて失敗したら止める。
        if stage == "configure" || self.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
            log::error!("accel_sample_stopped stage={stage}");
            self.state = State::Done;
        }
    }
}
