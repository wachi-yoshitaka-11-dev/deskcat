//! #18の校正build（`bench-servo-calib-18`）に足す、加速度（`ACCEL-01`、ADXL345）のraw sampleのlog
//! （`accel-log-18` feature。[Issue #15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)の
//! 「サーボの振動のsample」と、[Issue #22](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/22)の
//! 「サーボ自身の振動でeventを繰り返しtriggerしない」の材料を、#18の段2の窓で同時に取るためのもの）。
//!
//! **servoの制御・`Limiter`・露出・停止の動作と順序は変えない。**`crate::servo_calib`が足すのは、
//! 回が始まる前の[`start`]の呼び出しと、回が終わった後の[`AccelLog::stop`]の呼び出しと、
//! 1 commandごとの時刻合わせの1行（`servo_calib_18_motion_clock`。stopの後に出す）だけである。
//!
//! # 制御周期を乱さないための構成
//!
//! I2Cの読み出し（1件が100 kHzで約1 ms）をservoのloopへ入れず、**別threadで行う。**threadは、main taskと**別のcore**に固定し、
//! 優先度をmain task（ESP-IDFの既定で1）と同じ値（1）にする。起こせなかった場合（設定の失敗、thread
//! の失敗）は、**加速度なしでservoだけを続ける**（`accel_log_18_not_started`を出す）。優先度とcoreを決められないままthreadを
//! 起こさない。**実機での影響は確かめていない。**確かめる材料は、同じ値の校正buildを加速度なしで動かした回の`servo_calib_18_motion_done`の
//! `tick_min_ms`／`tick_max_ms`と、`servo_calib_18_session_end`の`interval_rejections`（各tickの間隔、または`Limiter`へ渡す窓の間隔が許容幅の外でrejectされてlatchした回数。latchで回が終わるので、
//! 1回の起動で最大1）を、この build の回と並べることである。**校正buildにはheartbeat loopもhealth snapshotも無く、`overrun_ticks`は出ない。**上の2つがその代わりである。
//!
//! I2Cのbus（`GPIO25`＝SDA、`GPIO26`＝SCL）はこのthreadだけが持つ（校正buildはI2Cを他に使わない）。
//! ISRからは呼ばない（I2Cはblockする。ここはthreadで、ISRではない）。
//!
//! # 出力
//!
//! すべてUART0のlog。**servoの動作中にmain taskはlogを出さない**（`servo_calib`のmodule doc。dutyを先に0にするため）が、
//! このthreadは動作中にも出す。別threadなので、main taskの周期は止めない（上のとおり、実機では確かめていない）。
//!
//! - `accel_log_18_configured`：設定を書いた後に1回。
//! - `accel_log_18_raw t_us=<T> n=<N> k=<K> xyz=x,y,z;x,y,z;...`：1回のpollで読んだ**K件**のraw値（LSB。±2 g・full resolution。
//!   warm-upの件を除く）。`n`はその行の最初の件の通し番号（1始まり）。`t_us`は、そのpollがFIFOを読み終えた時刻で、
//!   `servo_calib::run`の`started`（`GO18`を受けた後に取る`Instant`）からの経過（µs）である。**件ごとの時刻ではない。**
//!   出力data rateは100 Hz（公称）なので、件の間隔は約10 msで、行の最後の件が`t_us`の直前に近い。件ごとのsample時刻は持たない。
//! - `accel_log_18_overrun`：`INT_SOURCE`のOverrunが立っていたpoll（FIFOが満杯になり、古い件が捨てられた）。
//! - `accel_log_18_fault`：I2Cの失敗と、復号できなかった件。
//! - `accel_log_18_summary`：threadが終わるとき。poll数、件数、overrunしたpollの数、I2Cの失敗の数、1回のpollの時間の最大（µs）。
//!
//! 時刻合わせ：servoの1 commandの時刻（`servo_calib_18_tick_trace`の`at_us`など）は`motion_start`からの経過である。
//! `servo_calib_18_motion_clock n=<n> start_us=<S>`の`S`は、`motion_start`の`started`からの経過（µs）で、`t_us`と同じ基準である。
//! **`S`と`t_us`の差は、`Instant::now()`を取る場所が違うために生じる誤差を含む（大きさは測っていない）。**
//!
//! # 書き込み
//!
//! `deskcat_accel::SETUP`の順に書く（`accel-sample-15`と同じ。`INT_ENABLE`を0にするので、tapの機能も止まる）。**Device IDが`0xE5`のときだけ書く。**
//! 一致しなければ何も書かず、threadを終える。`POWER_CTL`をstandbyへ戻す書き込みは足さない（電源が落ちるまで測定が続く）。
//!
//! # 確かめていないこと
//!
//! - **実機では動かしていない。**host testは無い（firmware crateはhostで動かせない）。復号・設定値は`deskcat_accel`のhost testが持つ。
//! - 別threadの優先度・core固定が、servoの周期（`tick_min_ms`／`tick_max_ms`）に影響しないこと。
//! - 1行のlogがUART0（115200 baud想定）へ出る時間。poll 1回の読み出しが最大8件のとき、1行が長くなり、連続して出すと、
//!   他のthreadのlogを遅らせうる（main taskは動作中にlogを出さない）。
//! - warm-upの件数は`accel_sample`と同じ一般値であり、安全要件の5項目に効かない。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use deskcat_accel::{decode_sample, fifo_entries, is_overrun, SampleFault};
use esp_idf_svc::hal::cpu::Core;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{InputPin, OutputPin};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver, I2C0};
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::{EspError, ESP_ERR_TIMEOUT};

use crate::accel::{Adxl345, EXPECTED_DEVICE_ID};

/// `FIFO_STATUS`を読む間隔（ms）。FIFOの保持時間（32件÷100 Hz＝320 ms）より十分短い。`accel_sample`と同じ。
const POLL_INTERVAL_MS: u32 = 20;
/// 1回のpollで読む最大件数。`accel_sample`と同じ。
const MAX_READS_PER_POLL: u8 = 8;
/// 設定後に捨てる件数（100 Hzで100 ms）。`accel_sample`と同じ。
const WARMUP_SAMPLES: u32 = 10;
/// 続けて失敗したら止める回数。`accel_sample`と同じ。
const MAX_CONSECUTIVE_ERRORS: u32 = 5;
/// threadのstack（byte）。`format!`とlogに足りる一般値で、安全要件の5項目に効かない。
const THREAD_STACK_BYTES: usize = 6144;
/// threadの優先度。main taskの既定（ESP-IDFの`ESP_TASK_MAIN_PRIO`＝1）と同じ値にする。**main taskの値そのものは確かめていない**
/// （取るには`unsafe`のFFIが要る。`servo_calib::log_main_thread_info`のdoc）。
const THREAD_PRIORITY: u8 = 1;

/// 起こしたthreadの手綱。[`AccelLog::stop`]でthreadへ終わりを伝える。
pub struct AccelLog {
    stop: Arc<AtomicBool>,
}

impl AccelLog {
    /// threadへ終わりを伝える（次のpollで終わる）。待たない。
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// I2Cのdriverを作り、別threadでsampleのlogを始める。**失敗しても`None`を返すだけで、servoを止めない**
/// （`accel_log_18_not_started`を出す）。
///
/// `deadline`は`started`からの上限で、これを超えたらthreadは終わる（`MAX_SESSION_S`と同じ長さを渡す）。
pub fn start(
    i2c0: I2C0<'static>,
    sda: impl InputPin + OutputPin + Send + 'static,
    scl: impl InputPin + OutputPin + Send + 'static,
    started: Instant,
    deadline: Duration,
) -> Option<AccelLog> {
    // ESP32内蔵のweak pull-upは有効にしない（`run_i2c_bringup`と同じ。実効pull-upは外部pull-upだけを前提にしている）。
    let config = I2cConfig::new()
        .baudrate(Hertz(crate::I2C_BAUDRATE_HZ))
        .sda_enable_pullup(false)
        .scl_enable_pullup(false);
    let driver = match I2cDriver::new(i2c0, sda, scl, &config) {
        Ok(driver) => driver,
        Err(err) => {
            log::error!("accel_log_18_not_started reason=i2c_driver_new_failed error={err}");
            return None;
        }
    };

    // main taskと別のcoreへ固定する。決められなければ、thread自体を起こさない。
    let other_core = match esp_idf_svc::hal::cpu::core() {
        Core::Core0 => Core::Core1,
        Core::Core1 => Core::Core0,
    };
    let spawn_conf = ThreadSpawnConfiguration {
        name: Some(c"accel_log_18"),
        stack_size: THREAD_STACK_BYTES,
        priority: THREAD_PRIORITY,
        pin_to_core: Some(other_core),
        ..ThreadSpawnConfiguration::default()
    };
    if let Err(err) = spawn_conf.set() {
        log::error!("accel_log_18_not_started reason=thread_config_failed error={err}");
        return None;
    }
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let spawned = std::thread::Builder::new()
        .name("accel_log_18".into())
        .stack_size(THREAD_STACK_BYTES)
        .spawn(move || run(driver, started, deadline, &thread_stop));
    // 後に作るthreadへ、この設定を引き継がない。
    if let Err(err) = ThreadSpawnConfiguration::default().set() {
        log::warn!("accel_log_18_spawn_config_restore_failed error={err}");
    }
    match spawned {
        Ok(_) => Some(AccelLog { stop }),
        Err(err) => {
            log::error!("accel_log_18_not_started reason=thread_spawn_failed error={err}");
            None
        }
    }
}

#[derive(Default)]
struct Counters {
    polls: u32,
    samples: u32,
    overrun_polls: u32,
    out_of_range: u32,
    io_timeout: u32,
    io_other: u32,
    /// 1回のpollの時間の最大（µs）。
    poll_max_us: u64,
}

/// 1回のpollの結果。
enum Poll {
    Ok,
    /// 停止する（連続失敗）。
    Stop,
}

fn run(mut driver: I2cDriver<'static>, started: Instant, deadline: Duration, stop: &AtomicBool) {
    log::info!(
        "accel_log_18_thread_info name={:?} core={:?} priority={THREAD_PRIORITY}",
        std::thread::current().name(),
        esp_idf_svc::hal::cpu::core(),
    );
    let sensor = Adxl345::new(crate::ACCEL_I2C_ADDRESS);
    match sensor.read_device_id(&mut driver) {
        Ok(EXPECTED_DEVICE_ID) => {}
        Ok(other) => {
            log::error!(
                "accel_log_18_not_started reason=device_id_mismatch accel_id=0x{other:02x} expected=0x{EXPECTED_DEVICE_ID:02x}"
            );
            return;
        }
        Err(err) => {
            log::error!("accel_log_18_not_started reason=device_id_read_failed error={err}");
            return;
        }
    }
    // 設定の失敗は1回で止める（書き込みを繰り返さない）。
    if let Err(err) = sensor.configure_sampling(&mut driver) {
        log::error!("accel_log_18_not_started reason=configure_failed error={err}");
        return;
    }
    log::info!(
        "accel_log_18_configured data_format=0x{:02x} bw_rate=0x{:02x} fifo_ctl=0x{:02x} poll_interval_ms={POLL_INTERVAL_MS} warmup={WARMUP_SAMPLES}",
        deskcat_accel::DATA_FORMAT_FULL_RES_2G,
        deskcat_accel::BW_RATE_100_HZ,
        deskcat_accel::FIFO_CTL_STREAM
    );

    let mut counters = Counters::default();
    let mut warmup_left = WARMUP_SAMPLES;
    let mut consecutive_errors = 0_u32;
    while !stop.load(Ordering::Relaxed) && started.elapsed() < deadline {
        FreeRtos::delay_ms(POLL_INTERVAL_MS);
        let poll_start = Instant::now();
        let result = poll(
            &sensor,
            &mut driver,
            started,
            &mut counters,
            &mut warmup_left,
            &mut consecutive_errors,
        );
        let poll_us = u64::try_from(poll_start.elapsed().as_micros()).unwrap_or(u64::MAX);
        counters.poll_max_us = counters.poll_max_us.max(poll_us);
        if matches!(result, Poll::Stop) {
            log::error!("accel_log_18_stopped reason=consecutive_errors");
            break;
        }
    }
    log::info!(
        "accel_log_18_summary polls={} samples={} overrun_polls={} out_of_range={} io_timeout={} io_other={} poll_max_us={}",
        counters.polls,
        counters.samples,
        counters.overrun_polls,
        counters.out_of_range,
        counters.io_timeout,
        counters.io_other,
        counters.poll_max_us,
    );
}

fn poll(
    sensor: &Adxl345,
    driver: &mut I2cDriver<'static>,
    started: Instant,
    counters: &mut Counters,
    warmup_left: &mut u32,
    consecutive_errors: &mut u32,
) -> Poll {
    counters.polls = counters.polls.saturating_add(1);
    let entries = match sensor.read_fifo_status(driver) {
        Ok(status) => fifo_entries(status),
        Err(err) => return io_fault("fifo_status", &err, counters, consecutive_errors),
    };
    // Overrunは、FIFOの内容を読むと消える。**読み出しの前に**確かめる（`accel_sample`と同じ）。
    match sensor.read_int_source(driver) {
        Ok(src) if is_overrun(src) => {
            counters.overrun_polls = counters.overrun_polls.saturating_add(1);
            log::warn!(
                "accel_log_18_overrun t_us={} int_source=0x{src:02x} polls_with_overrun={}",
                elapsed_us(started),
                counters.overrun_polls
            );
        }
        Ok(_) => {}
        Err(err) => return io_fault("int_source", &err, counters, consecutive_errors),
    }
    let first_n = counters.samples.saturating_add(1);
    let mut line = String::new();
    let mut kept = 0_u32;
    for _ in 0..entries.min(MAX_READS_PER_POLL) {
        let bytes = match sensor.read_sample_bytes(driver) {
            Ok(bytes) => bytes,
            Err(err) => {
                flush(&line, started, first_n, kept);
                return io_fault("data", &err, counters, consecutive_errors);
            }
        };
        match decode_sample(&bytes) {
            Ok(sample) => {
                if *warmup_left > 0 {
                    *warmup_left -= 1;
                } else {
                    counters.samples = counters.samples.saturating_add(1);
                    if !line.is_empty() {
                        line.push(';');
                    }
                    line.push_str(&format!("{},{},{}", sample.x, sample.y, sample.z));
                    kept += 1;
                }
            }
            Err(SampleFault::OutOfRange) => {
                counters.out_of_range = counters.out_of_range.saturating_add(1);
                log::error!(
                    "accel_log_18_fault kind=out_of_range raw={bytes:02x?} count={}",
                    counters.out_of_range
                );
            }
        }
    }
    flush(&line, started, first_n, kept);
    // 1周回のI/O（FIFO_STATUS・INT_SOURCE・各件の読み出し）が全部成功したときだけ数え直す（`accel_sample`と同じ）。
    *consecutive_errors = 0;
    Poll::Ok
}

fn elapsed_us(started: Instant) -> u128 {
    started.elapsed().as_micros()
}

/// 読んだ件を1行で出す。件が無ければ出さない。
fn flush(line: &str, started: Instant, first_n: u32, kept: u32) {
    if kept > 0 {
        log::info!(
            "accel_log_18_raw t_us={} n={first_n} k={kept} xyz={line}",
            elapsed_us(started)
        );
    }
}

fn io_fault(
    stage: &'static str,
    err: &EspError,
    counters: &mut Counters,
    consecutive_errors: &mut u32,
) -> Poll {
    if err.code() == ESP_ERR_TIMEOUT {
        counters.io_timeout = counters.io_timeout.saturating_add(1);
        log::error!(
            "accel_log_18_fault kind=i2c_timeout stage={stage} error={err} count={}",
            counters.io_timeout
        );
    } else {
        counters.io_other = counters.io_other.saturating_add(1);
        log::error!(
            "accel_log_18_fault kind=i2c_other stage={stage} error={err} count={}",
            counters.io_other
        );
    }
    *consecutive_errors += 1;
    if *consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
        Poll::Stop
    } else {
        Poll::Ok
    }
}
