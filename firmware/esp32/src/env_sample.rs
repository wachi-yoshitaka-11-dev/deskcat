//! #16 の試験 build 専用の一回測定。bus はこの main-loop task が所有し、
//! `env::Bme280` は各 transaction の間だけ借りる。
//!
//! Bosch BME280 Data Sheet BST-BME280-DS001-24 Rev. 1.24 §4.1, §4.2.2,
//! §5.4.3–5.4.9, §9.1 に従う。現時点の証拠は VM の ESP32 build・Clippy と
//! host test まで。raw／変換値と reference の実機比較は未実施。
//!
//! 各 poll は有限 timeout の I2C transaction だけを行い、変換時間は待たずに
//! main loop へ戻す。log は `crate::console` により UART0 へ出す。

use deskcat_env::{
    check_calibration_ready, check_identity, check_measurement_ready, classify_io_error,
    compensate, decode_calibration, decode_raw, Calibration, IoFault, RegisterFault,
};
use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::sys::{EspError, ESP_ERR_TIMEOUT};

use crate::env::Bme280;

/// Bosch §9.1 の ×1/×1/×1 測定時間は typ 8 ms、max 9.3 ms。
/// typ の時点から status を確かめ始め、[`DEADLINE_MS`] まで待つ。
const READY_AFTER_MS: u64 = 8;
/// 状態異常の診断用上限。測定最大時間と loop の遅延を含む余裕を設ける。
const DEADLINE_MS: u64 = 30;

#[derive(Clone, Copy)]
enum State {
    WaitCalibration { deadline_ms: u64 },
    ReadCalibration,
    ConfigureHumidity,
    StartForced,
    WaitConversion { ready_ms: u64, deadline_ms: u64 },
    ReadSample,
    Done,
}

/// protocol counter ではない。UART0 の fault 行ごとに更新後の件数を出す。
#[derive(Default)]
struct Counters {
    identity: u32,
    io_timeout: u32,
    io_other: u32,
    not_ready: u32,
    invalid_data: u32,
    compensation: u32,
    samples: u32,
}

pub struct EnvSampler {
    driver: I2cDriver<'static>,
    sensor: Bme280,
    state: State,
    calibration: Option<Calibration>,
    counters: Counters,
}

/// bus 初期化は sampler を作る前に 1 回だけ試すので、失敗件数は 1 として記録する。
pub fn report_bus_init_fault(err: &EspError) {
    let kind = if classify_io_error(err.code(), ESP_ERR_TIMEOUT) == IoFault::Timeout {
        "i2c_timeout"
    } else {
        "i2c_other"
    };
    log::error!("env_fault kind={kind} stage=driver_new error={err} count=1");
}

impl EnvSampler {
    pub fn new(
        driver: I2cDriver<'static>,
        chip_id: Result<u8, EspError>,
        now_ms: u64,
        address: u8,
    ) -> Self {
        let mut sampler = Self {
            driver,
            sensor: Bme280::new(address),
            state: State::WaitCalibration {
                deadline_ms: now_ms + DEADLINE_MS,
            },
            calibration: None,
            counters: Counters::default(),
        };
        match chip_id {
            Ok(id) if check_identity(id).is_ok() => {}
            Ok(id) => {
                sampler.counters.identity += 1;
                log::error!(
                    "env_fault kind=chip_id_mismatch raw=0x{id:02x} count={}",
                    sampler.counters.identity
                );
                sampler.state = State::Done;
            }
            Err(err) => sampler.io_fault("chip_id", err),
        }
        sampler
    }

    /// true なら試行は終了。失敗後に新たな書き込みをしない。
    pub fn poll(&mut self, now_ms: u64) -> bool {
        match self.state {
            State::WaitCalibration { deadline_ms } => {
                match self.sensor.read_status(&mut self.driver) {
                    Ok(status) if check_calibration_ready(status).is_ok() => {
                        self.state = State::ReadCalibration;
                    }
                    Ok(_) if now_ms > deadline_ms => self.not_ready("calibration_timeout"),
                    Ok(_) => {} // 次の loop で確認する。
                    Err(err) => self.io_fault("status_calibration", err),
                }
            }
            State::ReadCalibration => match self.sensor.read_calibration(&mut self.driver) {
                Ok((tp, h)) => {
                    let c = decode_calibration(&tp, &h);
                    if c.dig_p1 == 0 {
                        self.invalid_data("calibration_p1_zero");
                    } else {
                        self.calibration = Some(c);
                        self.state = State::ConfigureHumidity;
                        log::info!(
                            "env_calibration_read dig_p1={} dig_h4={} dig_h5={}",
                            c.dig_p1,
                            c.dig_h4,
                            c.dig_h5
                        );
                    }
                }
                Err(err) => self.io_fault("calibration", err),
            },
            State::ConfigureHumidity => match self.sensor.configure_humidity(&mut self.driver) {
                Ok(()) => self.state = State::StartForced,
                Err(err) => self.io_fault("ctrl_hum_write", err),
            },
            State::StartForced => match self.sensor.start_forced(&mut self.driver) {
                Ok(()) => {
                    let started_ms = now_ms;
                    self.state = State::WaitConversion {
                        ready_ms: started_ms + READY_AFTER_MS,
                        deadline_ms: started_ms + DEADLINE_MS,
                    };
                }
                Err(err) => self.io_fault("ctrl_meas_write", err),
            },
            State::WaitConversion {
                ready_ms,
                deadline_ms,
            } => {
                if now_ms >= ready_ms {
                    match self.sensor.read_status(&mut self.driver) {
                        Ok(status) if check_measurement_ready(status).is_ok() => {
                            self.state = State::ReadSample;
                        }
                        Ok(_) if now_ms > deadline_ms => self.not_ready("measurement_timeout"),
                        Ok(_) => {} // 変換中。main loop を止めず再確認する。
                        Err(err) => self.io_fault("status_measurement", err),
                    }
                }
            }
            State::ReadSample => match self.sensor.read_raw(&mut self.driver) {
                Ok(bytes) => match decode_raw(&bytes) {
                    Ok(raw) => {
                        log::info!(
                            "env_raw temperature={} pressure={} humidity={}",
                            raw.temperature,
                            raw.pressure,
                            raw.humidity
                        );
                        // ReadCalibration からしかここへ進まない。状態の破綻も fault として記録する。
                        let Some(calibration) = self.calibration.as_ref() else {
                            self.invalid_data("calibration_missing");
                            return true;
                        };
                        match compensate(raw, calibration) {
                            Ok(reading) => {
                                self.counters.samples = self.counters.samples.saturating_add(1);
                                log::info!("env_reading temperature_centi_c={} pressure_q24_8_pa={} humidity_q22_10_percent={} samples={}", reading.temperature_centi_c, reading.pressure_q24_8_pa, reading.humidity_q22_10_percent, self.counters.samples);
                                self.state = State::Done;
                            }
                            Err(err) => {
                                self.counters.compensation =
                                    self.counters.compensation.saturating_add(1);
                                log::error!(
                                    "env_fault kind=compensation detail={err:?} count={}",
                                    self.counters.compensation
                                );
                                self.state = State::Done;
                            }
                        }
                    }
                    Err(RegisterFault::SkippedChannel) => self.invalid_data("skipped_channel"),
                    Err(_) => self.invalid_data("raw_register_fault"),
                },
                Err(err) => self.io_fault("raw", err),
            },
            State::Done => return true,
        }
        matches!(self.state, State::Done)
    }

    fn io_fault(&mut self, stage: &'static str, err: EspError) {
        if classify_io_error(err.code(), ESP_ERR_TIMEOUT) == IoFault::Timeout {
            self.counters.io_timeout = self.counters.io_timeout.saturating_add(1);
            log::error!(
                "env_fault kind=i2c_timeout stage={stage} error={err} count={}",
                self.counters.io_timeout
            );
        } else {
            // ESP-IDF の一般 I2C error だけから配線断と NACK は区別できない。
            self.counters.io_other = self.counters.io_other.saturating_add(1);
            log::error!(
                "env_fault kind=i2c_other stage={stage} error={err} count={}",
                self.counters.io_other
            );
        }
        self.state = State::Done;
    }

    fn not_ready(&mut self, kind: &'static str) {
        self.counters.not_ready = self.counters.not_ready.saturating_add(1);
        log::error!("env_fault kind={kind} count={}", self.counters.not_ready);
        self.state = State::Done;
    }

    fn invalid_data(&mut self, kind: &'static str) {
        self.counters.invalid_data = self.counters.invalid_data.saturating_add(1);
        log::error!("env_fault kind={kind} count={}", self.counters.invalid_data);
        self.state = State::Done;
    }
}
