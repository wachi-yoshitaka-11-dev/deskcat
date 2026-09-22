//! `SERVO-01`（SG90首振りサーボ）のPWM driver。
//!
//! [Issue #17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)。
//! この段階では**「指定した角度へ1回動かす」だけを扱う**。首振り、fail-safe、
//! 拘束検知は別途進める。
//!
//! **既定build（`bench-servo-test-17` featureなし）は`main()`からこのmoduleを
//! 呼ばない**（gate状態は[TBD台帳](../../../docs/hardware/tbd-register.md)、理由は
//! [servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
//! `サーボ出力を有効化してよい条件`が正本）。`bench-servo-test-17` feature付きbuild
//! だけが[`run_servo_bench_test`](../../../firmware/esp32/src/main.rs)経由で呼ぶ
//! （承認の状態は同文書の`承認の状態`節が正本）。
//!
//! pin割り当ては[gpio-assignment.md](../../../docs/hardware/gpio-assignment.md)の
//! `信号inventory`が正本（`SERVO-PWM`＝GPIO27、[`crate::config::SERVO_PWM_GPIO`]）。
//! 電源は外部5 V系（`PSU-SERVO-01`）から取る（ESP32の電源pinからは給電しない）。
//! このmoduleは信号線だけを扱い、電源配線は対象外。
//!
//! PWM周期・pulse幅・角度規約は一次資料に基づかない一般値であり、確定は
//! `HW-TBD-026`／`010`の範囲。詳細は
//! [servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の残余riskを参照。

// 既定buildは`main()`からこのmoduleを呼ばないためdead_codeになる（module doc参照）。
// `bench-servo-test-17` feature付きbuildでは呼ばれるため無害。
#![allow(dead_code)]

use esp_idf_svc::hal::gpio::OutputPin;
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys::EspError;

/// `SERVO-01`（SG90）のPWM driver。hardware LEDCでpulseを生成する（module doc参照）。
/// `Sg90::new`でGPIO27の駆動が始まるが初期dutyは0%であり、`move_to_angle_once`を
/// 呼ぶまで有効なservo pulseは出ない。
pub struct Sg90<'d> {
    driver: LedcDriver<'d>,
}

impl<'d> Sg90<'d> {
    /// LEDC timer・channel・pinを受け取ってdriverを作る。周期は
    /// [`crate::config::SERVO_PWM_FREQUENCY_HZ`]を使う。
    pub fn new<T, C>(timer: T, channel: C, pin: impl OutputPin + 'd) -> Result<Self, EspError>
    where
        T: LedcTimer + 'd,
        C: LedcChannel<SpeedMode = T::SpeedMode> + 'd,
    {
        // esp-idf-halはpinをtype levelで選ぶため、config::SERVO_PWM_GPIOはpin選択には
        // 使えない（呼び出し側が手書きする）。一致を実行時に確認する。
        debug_assert_eq!(
            pin.pin(),
            crate::config::SERVO_PWM_GPIO,
            "呼び出し側が渡したpinがconfig::SERVO_PWM_GPIOと一致しない"
        );

        // 既定resolution（Bits8、max_duty=256）は20 msの周期に対し1 stepが約78 µsと粗く、
        // 1000〜2000 µsのpulse幅範囲を約13 stepでしか刻めない。Bits16（約0.31 µs/step）へ
        // 上げる。検討経緯はPR本文参照。
        let timer_driver = LedcTimerDriver::new(
            timer,
            &TimerConfig::new()
                .frequency(crate::config::SERVO_PWM_FREQUENCY_HZ.Hz())
                .resolution(esp_idf_svc::hal::ledc::config::Resolution::Bits16),
        )?;
        let driver = LedcDriver::new(channel, timer_driver, pin)?;
        Ok(Self { driver })
    }

    /// 指定した角度（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜`_MAX_DEG`）へ**1回だけ**動かす
    /// （連続動作は行わない）。中央からの偏角を
    /// [`crate::config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG`]でclampする
    /// （暫定値。同定数のdoc参照）。
    pub fn move_to_angle_once(&mut self, angle_deg: f32) -> Result<(), EspError> {
        let neutral = crate::config::SERVO_ANGLE_CONVENTION_NEUTRAL_DEG;
        let max_deviation = crate::config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG;
        let clamped_angle_deg = angle_deg.clamp(neutral - max_deviation, neutral + max_deviation);

        let pulse_width_us = pulse_width_us_for_angle(clamped_angle_deg);
        let duty = self.duty_for_pulse_width_us(pulse_width_us);
        self.driver.set_duty(duty)
    }

    /// dutyを0へ戻し、pulseの生成を止める。**信号を止めてもservo側が駆動を止める
    /// 保証は無い**（詳細は
    /// [servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
    /// 残余risk参照）。拘束を確実に止める手段は人間による外部電源の遮断である。
    ///
    /// `Self`が後でdropされると、`esp-idf-hal`の`LedcDriver::drop`が`ledc_stop`を
    /// idle level `0`で呼び、GPIO27をlowへ固定する（high-Zには戻らない）。
    pub fn stop(&mut self) -> Result<(), EspError> {
        self.driver.set_duty(0)
    }

    /// pulse幅（マイクロ秒）をLEDCのduty値（`set_duty`が受ける単位）へ変換する。
    ///
    /// `duty = pulse_width_us / period_us * max_duty`。`period_us`は
    /// [`crate::config::SERVO_PWM_FREQUENCY_HZ`]から求める。
    fn duty_for_pulse_width_us(&self, pulse_width_us: u32) -> u32 {
        let period_us = 1_000_000 / crate::config::SERVO_PWM_FREQUENCY_HZ;
        let max_duty = self.driver.get_max_duty();
        ((u64::from(pulse_width_us) * u64::from(max_duty)) / u64::from(period_us)) as u32
    }
}

/// 角度（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜`_MAX_DEG`）をpulse幅（マイクロ秒）へ
/// 線形変換する（module doc参照）。
fn pulse_width_us_for_angle(angle_deg: f32) -> u32 {
    let min_deg = crate::config::SERVO_ANGLE_CONVENTION_MIN_DEG;
    let max_deg = crate::config::SERVO_ANGLE_CONVENTION_MAX_DEG;
    let min_us = crate::config::SERVO_PULSE_WIDTH_MIN_US as f32;
    let max_us = crate::config::SERVO_PULSE_WIDTH_MAX_US as f32;

    let angle_deg = angle_deg.clamp(min_deg, max_deg);
    let ratio = (angle_deg - min_deg) / (max_deg - min_deg);
    (min_us + ratio * (max_us - min_us)).round() as u32
}
