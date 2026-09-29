//! `SERVO-01`（SG90首振りサーボ）のPWM driver。
//!
//! [Issue #17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、
//! [Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)。
//! 首振り、fail-safe、拘束検知は別途進める。
//!
//! # 角度を出せるのは、limiterを所有する[`LimitedServo`]だけである
//!
//! **この型が保証するのは、command の水準までである。**すなわち、compileできるbuildで、
//! このmoduleを通ってLEDCへ設定する位置の列が、自分で測った間隔で`Limiter`の制限
//! （位置、速度、加速度、`単一commandの最大変化量`）を満たすことまでである。**実機がその列を
//! どう実現するか（PWMのframe、servo内部の追従、非常停止、resetの後）は保証しない。**
//! それは下の「実機での実現は保証しない」に列挙し、段3の監視下の試験（#19の受け入れ条件7）で確かめる。
//!
//! **compileできるbuildで、このmoduleの外から角度を指定してpulseを出す入口は
//! [`LimitedServo::tick`]だけである。**[`LimitedServo`]は[`Sg90`]と`deskcat_servo::Limiter`を
//! **1つずつ所有**し、外へ出す操作は[`LimitedServo::new`]、[`LimitedServo::resume`]、
//! [`LimitedServo::admit`]、[`LimitedServo::tick`]、[`LimitedServo::stop`]、
//! [`LimitedServo::counters`]だけである（#19の受け入れ条件2・3、`AGENTS.md`
//! 「サーボ安全制限をデバッグ経路からも迂回させない」）。
//!
//! - **`Limiter`を外から受け取らない。**[`LimitedServo::new`]と[`LimitedServo::resume`]は、
//!   制限・制御周期・受理するmotion名を受け取り、**この型の中で速度0の`Limiter`を作る。**
//!   呼び出し側が進めておいた`Limiter`（位置を合わせたまま速度を持たせたもの等）を渡す経路が無い
//! - **`Setpoint`も`AdmittedTarget`も外から受け取らない。**`admit`は受理したtargetを内部に持ち、
//!   `tick`はそのtargetへ向けて**自分の**`Limiter::step`を呼び、結果を出力する。
//!   `Setpoint`は`Copy`であり、出力を受け取る入口があると、古い値を後で出し直して
//!   速度と加速度の制限を通らずに位置を跳ばせてしまうためである
//! - **制御周期の間隔を呼び出し側から受け取らない。**`tick`は前回の`tick`からの経過時間を
//!   単調時計（`std::time::Instant`）で**自分で測り**、それを`Limiter::step`へ渡す。
//!   間隔を申告させると、公称値を申告したまま短い間隔で呼ぶだけで、設定する位置の列が
//!   実時間で`Limiter`の制限を満たさなくなるためである
//! - **servo 1つにつき`Limiter`は1つである。**[`LimitedServo::new`]は[`Sg90`]を値で受け取り、
//!   [`Sg90::new`]はLEDC timer・channel・pinを消費する。これらが`Peripherals`から1回だけ取れるのは、
//!   `unsafe`の`steal`／`reborrow`を使わない場合であり、firmwareは`unsafe_code = "forbid"`で
//!   それを禁じている（`firmware/esp32/Cargo.toml`）。止めた後は[`Stopped`]になり、
//!   [`Stopped`]から[`Sg90`]を取り出す手段は無い
//! - [`Sg90`]が外へ出すのは[`Sg90::new`]と[`Sg90::stop`]（dutyを0へ戻す。角度を指定しない）
//!   だけであり、角度を出す関数はこのmoduleのprivateである
//! - Pi由来でもdebug由来でも、`MotionRequest::source`で判定を分けない（`Limiter::admit`のdoc）
//!
//! **型が塞ぐのは、このmoduleを通る経路までである。**LEDC timer・channel・pinを[`Sg90::new`]へ
//! 渡さずに、`esp_idf_svc`の`LedcDriver`を直接作る経路は型では止められない。
//! `SERVO-PWM`へ触る経路をこのmoduleの外に作らないことは、呼び出し側（`main.rs`）の規則である。
//!
//! **例外は`bench-servo-test-17`の`Sg90::move_to_angle_once`である。**limiterを通らない。
//! 同featureのbuildでだけcompileされ、そのbuildは#474の`compile_error!`で止まる
//! （下記）。**compileできるbuildからは呼べない。**bench経路をlimiterへ通す書き換えは、
//! 測定用のbuild（`servo-safety-limits.md`の`承認の状態`の項目6(b)）で行う。
//!
//! # 制御周期の外の間隔、出力の失敗ではlatchする
//!
//! 次のどちらかが起きたら、**以後の`tick`は何も設定せず、`Limiter`も進めず、[`TickError::Faulted`]を
//! 返す。**解除の経路は、[`LimitedServo::stop`]の後に[`LimitedServo::resume`]で作り直すことだけである。
//!
//! - **測った間隔が制御周期（`ControlPeriod`）の外にある**（[`TickError::Rejected`]）。
//!   長すぎる場合（遅れ）にそのまま続けると、`Limiter`に残った速度で次の位置を設定することになり、
//!   その間の位置の列が`Limiter`の制限を満たさなくなる。短すぎる場合は、`Limiter`がその間隔では
//!   `最大加速度`を保証できない（`deskcat_servo::Limiter::step`のdocの「`dt_s`は
//!   `ControlPeriod`の範囲内でなければならない」）。**jitterが許容幅を超えると、そこで止まる。**
//!   安全側に倒れる既知の性質である
//! - **`Limiter::step`の後にPWMの出力が失敗した**（[`TickError::Output`]）。`Limiter`だけが1 step進み、
//!   LEDCに設定した位置は前の値のまま残る（esp-idf-hal 0.46.2の`LedcDriver::set_duty`が呼ぶ
//!   ESP-IDFの`ledc_set_duty_and_update`は、失敗をregisterへ書く前の検査で返す。source を読んだ結果）。
//!   そのまま続けると、次の`tick`で2 step分の変化を設定することになる
//!
//! これは`servo-safety-limits.md`の`起動時とdriver故障時の動作`（`HW-TBD-019`、PWM driverの
//! 実行中の故障を検知したときの動作は未確定）の中で、「未検証の動作出力を行わない」側の
//! 既定として置いたものである。**値も復帰の条件も決めていない。**
//!
//! **latchは「それ以上動かさない」までであり、「出力を止める」ではない。**LEDCは最後に設定した
//! dutyを出し続けうる。**[`TickError::Rejected`]、[`TickError::Output`]、[`TickError::Faulted`]を
//! 受けた呼び出し側は、[`LimitedServo::stop`]を呼ぶ。**その配線（`main.rs`）はまだ無い。
//!
//! # 間隔の基準と、tickを呼び続ける前提
//!
//! **間隔の基準を取るのは、[`LimitedServo::new`]または[`LimitedServo::resume`]の後で、targetが
//! あるときの最初の`tick`だけである。**その`tick`は何も設定しない。2回目以降の`admit`では基準を
//! 取り直さない。**したがって、一度動き始めたら、targetに着いた後も制御周期で`tick`を呼び続ける。**
//! 呼ぶのを止めてから再開すると、測った間隔が制御周期の外になり、latchする。
//!
//! # 止めた後の再開
//!
//! [`LimitedServo::stop`]は自分を消費して[`Stopped`]を返す。[`Stopped`]は[`Sg90`]と**再開の位置**
//! （最後にLEDCへ設定できた位置。一度も設定していなければ起動時の初期位置）をprivateに持つ。
//! [`LimitedServo::resume`]は**位置を受け取らず**、その位置から速度0の`Limiter`を作る。
//!
//! - **止める直前の速度を持ち越さない。**持ち越すと、再開の最初の位置が、止まっていた間を
//!   無視した速度で設定される
//! - **呼び出し側が選んだ位置から再開させない。**選べると、その位置へ`Limiter`の速度・加速度の
//!   制限を受けずに設定できてしまう
//!
//! # 既知の制限: 起動時の初期位置
//!
//! [`LimitedServo::new`]の`initial_position`は、呼び出し側が渡す値である。**呼び出し側が位置を
//! 選べる入口は、起動時のこの1つだけである。**`Limiter`の制限はこの位置から始まる列に効き、
//! この位置と実機の位置の食い違いには効かない（起動時の位置の扱いは`HW-TBD-019`／`HW-TBD-020`）。
//! **このmoduleは値も扱いも決めない。**
//!
//! # 実機での実現は保証しない
//!
//! 次は型でも、ESP-IDFのsourceを読むことでも確かめきれない。**量を主張しない。**
//! 段3の監視下の試験（#19の受け入れ条件7）の項目として扱う（#19のコメントに記録する）。
//!
//! - **PWMのframeで効くこと。**新しいdutyは次のPWM cycleから効く（ESP-IDF v5.5.3の
//!   `components/esp_driver_ledc/include/driver/ledc.h`、`ledc_update_duty`のnote）。servoも
//!   frameごとにしかpulseを読まない。制御周期とPWMの周期の関係によって、実機が受け取る位置の
//!   列は、LEDCへ設定した列と時間の刻みが違う。**制御周期は段3で決める**（このmoduleは値を持たない）
//! - **servoが受け取った位置へどう追従するか**（速さ、遅れ、行き過ぎ）は確かめていない
//! - **[`LimitedServo::stop`]は、軌道の制限を受けない非常停止である。**dutyを0へ戻すだけで、
//!   減速しない。**信号を止めてもservo側が駆動を止める保証は無い**（[`Sg90::stop`]のdoc）
//! - **再開の位置は、最後にLEDCへ設定した値であり、実機に届いたとは限らない。**次のframeより前に
//!   `stop`のduty 0が上書きした場合や、duty 0の間にservoが押されて動いた場合は、実機の位置と食い違う
//! - **reset、panic、drop、`core::mem::forget`の後の出力は保証しない。**範囲外のpulseになりうる。
//!   **止めるのは、人が外部電源を切ることである。**firmwareは`panic = "abort"`でbuildするため、
//!   panicでは`Drop`が走らない。ESP-IDF v5.5.3のpanic handler（生成される`sdkconfig`の
//!   `CONFIG_ESP_SYSTEM_PANIC_PRINT_REBOOT=y`）と`esp_restart`が通る`esp_restart_noos`は、
//!   resetするperipheralにLEDCを含まず（`components/esp_system/port/soc/esp32/system_internal.c`、
//!   `esp_system.h`の`esp_restart`のdoc）、reset前にCPUとAPBのclockを切り替える（同fileの
//!   `rtc_clk_cpu_set_to_default_config`）。LEDCの`Drop`は`ledc_stop`を呼ぶが、その失敗を`unwrap`する。
//!   **経路ごとの分析と、起動時にservoのLEDCを最初に止める要件は、#19のコメントに置く。**
//!
//! **`Setpoint`の位置の単位は、下の角度規約の度である**（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜
//! `_MAX_DEG`）。`Limiter`へ渡す可動域・速度・加速度もこの単位で与える。**値はこのmoduleが
//! 持たない。**どれも`HW-TBD-010`／`011`／`020`で未確定であり、呼び出し側が渡す。
//! 度からpulse幅への変換は、下の一般値による線形変換のままであり、**calibration済みの
//! 変換ではない**（`HW-TBD-026`／`010`）。
//!
//! **既定build（`bench-servo-test-17` featureなし）は`main()`からこのmoduleを
//! 呼ばない**（gate状態は[TBD台帳](../../../docs/hardware/tbd-register.md)、理由は
//! [servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
//! `サーボ出力を有効化してよい条件`が正本）。`bench-servo-test-17` feature付きbuild
//! だけが[`run_servo_bench_test`](../../../firmware/esp32/src/main.rs)経由で呼ぶ
//! （承認の状態は同文書の`承認の状態`節が正本）。**#474で、このfeature付きbuildは
//! `main.rs`の`compile_error!`でcompileが止まる。**
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
// `bench-servo-test-17` feature付きbuildでは呼ばれるため無害（#474で、そのbuildはcompileが止まる）。
#![allow(dead_code)]

use core::fmt;
use std::time::Instant;

use deskcat_servo::{
    AdmittedTarget, ControlPeriod, Limiter, LimiterCounters, LimitsError, MotionCatalog,
    MotionRequest, Rejection, ServoLimits, Setpoint,
};
use esp_idf_svc::hal::gpio::OutputPin;
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys::EspError;

/// `SERVO-01`（SG90）のPWM driver。hardware LEDCでpulseを生成する（module doc参照）。
/// `Sg90::new`でGPIO27の駆動が始まるが初期dutyは0%であり、[`LimitedServo::tick`]が
/// 出力するまで有効なservo pulseは出ない（`bench-servo-test-17`付きbuildでは
/// `move_to_angle_once`も同じ）。**角度を出す関数はこのmoduleのprivateである**（module doc参照）。
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

    /// 角度規約の度でpulseを出す。**privateであり、[`LimitedServo::tick`]だけが呼ぶ**
    /// （module doc参照）。
    ///
    /// 一般値の線形変換でpulse幅にする。変換は角度規約の範囲
    /// （`SERVO_ANGLE_CONVENTION_MIN_DEG`〜`_MAX_DEG`）へclampするが、
    /// **これは安全制限ではない。**可動域を強制するのは`Limiter`であり、
    /// [`LimitedServo::new`]が承認値の位置範囲をこの規約の内側に限る。
    fn output_angle(&mut self, angle_deg: f32) -> Result<(), EspError> {
        let pulse_width_us = pulse_width_us_for_angle(angle_deg);
        let duty = self.duty_for_pulse_width_us(pulse_width_us);
        self.driver.set_duty(duty)
    }

    /// 指定した角度（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜`_MAX_DEG`）へ**1回だけ**動かす
    /// （連続動作は行わない）。中央からの偏角を
    /// [`crate::config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG`]でclampする
    /// （暫定値。同定数のdoc参照）。
    ///
    /// **limiterを通らない。**そのため`bench-servo-test-17`付きbuildでだけcompileする
    /// （module doc参照）。
    #[cfg(feature = "bench-servo-test-17")]
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

/// [`Sg90`]と`deskcat_servo::Limiter`を1つずつ所有し、limiterを通った出力だけを出す
/// （module doc参照）。
///
/// **値を持たない。**制限（`ServoLimits`）、制御周期（`ControlPeriod`）、受理するmotion名
/// （`MotionCatalog`）、起動時の初期位置は呼び出し側が渡す。**`Limiter`そのものは受け取らず、
/// この型の中で作る**（速度0から始まる`Limiter`だけを持つため）。
pub struct LimitedServo<'d> {
    servo: Sg90<'d>,
    limiter: Limiter,
    target: Option<AdmittedTarget>,
    /// 前回の`tick`の時刻。`new`／`resume`の後で、targetがあるときの最初の`tick`で基準として取る
    /// （module docの「間隔の基準と、tickを呼び続ける前提」）。
    last_tick: Option<Instant>,
    /// 再開の位置（角度規約の度）。最後に出力できた位置、一度も出力していなければ起動時の初期位置。
    resume_position: f32,
    /// latchしているか（module docの「制御周期の外の間隔、出力の失敗ではlatchする」）。
    faulted: bool,
}

/// [`LimitedServo::stop`]の後の状態。[`Sg90`]と再開の位置を持つ。
///
/// **[`Sg90`]を取り出す手段は無い。**再開は[`LimitedServo::resume`]だけであり、
/// 再開の位置を呼び出し側から受け取らない（module docの「止めた後の再開」）。
pub struct Stopped<'d> {
    servo: Sg90<'d>,
    resume_position: f32,
}

impl Stopped<'_> {
    /// 再開の位置（角度規約の度。log用）。最後に出力できた位置、一度も出力していなければ
    /// 起動時の初期位置である。**これを渡す入口は無い。**
    #[must_use]
    pub fn resume_position(&self) -> f32 {
        self.resume_position
    }
}

/// [`LimitedServo::new`]と[`LimitedServo::resume`]が拒否した理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitedServoError {
    /// `Limiter::new`が拒否した（初期位置が有限でない、または承認値の位置範囲の外）。
    Limits(LimitsError),
    /// 承認値の位置範囲が、角度規約（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜`_MAX_DEG`）の外へ
    /// はみ出している。
    ///
    /// はみ出した分はpulse幅の変換で黙って飽和し、`Limiter`が認めた位置と
    /// 実際に出す位置が食い違う。**黙って受けずに構築を拒否する。**
    ApprovedRangeOutsideAngleConvention,
}

impl fmt::Display for LimitedServoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limits(err) => write!(f, "limiter rejected the configuration: {err}"),
            Self::ApprovedRangeOutsideAngleConvention => {
                f.write_str("approved position range lies outside the servo angle convention")
            }
        }
    }
}

/// [`LimitedServo::tick`]の失敗。**どれを受けても、呼び出し側は[`LimitedServo::stop`]を呼ぶ。**
#[derive(Debug)]
pub enum TickError {
    /// 測った間隔を`Limiter::step`がrejectした（制御周期の範囲外、または0）。
    /// **何も出力せず、`Limiter`も進めていない。**ここでlatchする。
    Rejected(Rejection),
    /// `Limiter::step`は通ったが、PWMの出力に失敗した。
    ///
    /// **`Limiter`の位置と速度はこのstep分だけ進んでいる。**戻さずにここでlatchする。
    Output(EspError),
    /// 以前の`tick`でlatchしている。**何も出力せず、`Limiter`も進めていない。**
    Faulted,
}

impl fmt::Display for TickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(rejection) => {
                write!(f, "limiter rejected the measured interval: {rejection}")
            }
            Self::Output(err) => write!(f, "servo output failed: {err}"),
            Self::Faulted => f.write_str("servo output is latched after an earlier failure"),
        }
    }
}

/// 制限がservoの角度規約に収まるかを確かめ、速度0の`Limiter`を作る。
fn build_limiter(
    limits: ServoLimits,
    period: ControlPeriod,
    catalog: MotionCatalog,
    start_position: f32,
) -> Result<Limiter, LimitedServoError> {
    let range = limits.position();
    if range.approved_min() < crate::config::SERVO_ANGLE_CONVENTION_MIN_DEG
        || range.approved_max() > crate::config::SERVO_ANGLE_CONVENTION_MAX_DEG
    {
        return Err(LimitedServoError::ApprovedRangeOutsideAngleConvention);
    }
    Limiter::new(limits, period, catalog, start_position).map_err(LimitedServoError::Limits)
}

impl<'d> LimitedServo<'d> {
    /// 起動時に[`Sg90`]を所有し、速度0の`Limiter`を作る。**この時点では何も出力しない。**
    ///
    /// `initial_position`は実機の位置と一致する保証が無い（module docの「既知の制限: 起動時の初期位置」）。
    ///
    /// # Errors
    ///
    /// [`LimitedServoError`]のいずれかに当たる場合に拒否し、[`Sg90`]を返す。
    pub fn new(
        servo: Sg90<'d>,
        limits: ServoLimits,
        period: ControlPeriod,
        catalog: MotionCatalog,
        initial_position: f32,
    ) -> Result<Self, (Sg90<'d>, LimitedServoError)> {
        match build_limiter(limits, period, catalog, initial_position) {
            Ok(limiter) => Ok(Self {
                servo,
                limiter,
                target: None,
                last_tick: None,
                resume_position: initial_position,
                faulted: false,
            }),
            Err(err) => Err((servo, err)),
        }
    }

    /// [`LimitedServo::stop`]の後に、[`Stopped`]が持つ再開の位置から、速度0の`Limiter`で
    /// 作り直す。**位置は受け取らない。この時点では何も出力しない。**
    ///
    /// # Errors
    ///
    /// [`LimitedServoError`]のいずれかに当たる場合（再開の位置が新しい制限の承認値の範囲外に
    /// ある場合を含む）に拒否し、[`Stopped`]を返す。
    pub fn resume(
        stopped: Stopped<'d>,
        limits: ServoLimits,
        period: ControlPeriod,
        catalog: MotionCatalog,
    ) -> Result<Self, (Stopped<'d>, LimitedServoError)> {
        match build_limiter(limits, period, catalog, stopped.resume_position) {
            Ok(limiter) => Ok(Self {
                servo: stopped.servo,
                limiter,
                target: None,
                last_tick: None,
                resume_position: stopped.resume_position,
                faulted: false,
            }),
            Err(err) => Err((stopped, err)),
        }
    }

    /// motion commandを自分の`Limiter::admit`へ通し、受理したtargetを内部に持つ。
    ///
    /// **Pi由来でもdebug由来でも同じ判定を通る。**直前のtargetは置き換える。
    /// **このcallは出力しない。**出力は[`LimitedServo::tick`]で行う。
    ///
    /// # Errors
    ///
    /// `Limiter::admit`のrejectをそのまま返す。rejectしたときは直前のtargetを保つ。
    pub fn admit(&mut self, request: &MotionRequest<'_>) -> Result<(), Rejection> {
        let admitted = self.limiter.admit(request)?;
        self.target = Some(admitted);
        Ok(())
    }

    /// 前回の`tick`からの経過時間を自分で測り、内部のtargetへ向けて自分の`Limiter::step`を
    /// その時間だけ進め、結果をLEDCへ設定する（実機でいつ・どう効くかはmodule docの
    /// 「実機での実現は保証しない」）。
    ///
    /// **何も設定せず`Ok(None)`を返す場合が2つある。**targetが無い（まだ`admit`していない）とき、
    /// と、`new`／`resume`の後でtargetがあるときの最初の`tick`（時刻の基準を取るだけ。
    /// module docの「間隔の基準と、tickを呼び続ける前提」）である。起動時の出力は
    /// `HW-TBD-019`で未確定であり、`servo-safety-limits.md`の`起動時とdriver故障時の動作`が
    /// 「承認されるまで、安全状態は『未検証の動作出力を行わない』とする」としているためである。
    ///
    /// 返す`Setpoint`は**出力した値の報告**（log用）である。これを受け取る入口は無い。
    ///
    /// # Errors
    ///
    /// 測った間隔を`Limiter::step`がrejectした場合は[`TickError::Rejected`]（何も設定しない）、
    /// PWMの出力に失敗した場合は[`TickError::Output`]を返し、どちらもここでlatchする。
    /// latchの後は[`TickError::Faulted`]を返す。
    pub fn tick(&mut self) -> Result<Option<Setpoint>, TickError> {
        if self.faulted {
            return Err(TickError::Faulted);
        }
        let Some(target) = self.target else {
            return Ok(None);
        };
        let now = Instant::now();
        let Some(previous) = self.last_tick.replace(now) else {
            return Ok(None);
        };
        let dt_s = now.duration_since(previous).as_secs_f32();
        let setpoint = match self.limiter.step(target, dt_s) {
            Ok(setpoint) => setpoint,
            Err(rejection) => {
                self.faulted = true;
                return Err(TickError::Rejected(rejection));
            }
        };
        if let Err(err) = self.servo.output_angle(setpoint.position()) {
            self.faulted = true;
            return Err(TickError::Output(err));
        }
        self.resume_position = setpoint.position();
        Ok(Some(setpoint))
    }

    /// dutyを0へ戻し（[`Sg90::stop`]）、**自分を消費して**[`Stopped`]を返す。
    ///
    /// `Limiter`、target、counterはここで捨てる。再開は[`LimitedServo::resume`]だけである
    /// （module docの「止めた後の再開」）。
    ///
    /// **軌道の制限を受けない非常停止である**（減速しない。module docの「実機での実現は保証しない」）。
    ///
    /// 返す`Result`は[`Sg90::stop`]の結果である。**失敗を握りつぶさない。失敗したら、人が外部電源を
    /// 切る。**dropやpanicの後の出力は保証しない（同じ節）。
    #[must_use = "dutyを0へ戻せたかをResultで確かめること"]
    pub fn stop(self) -> (Stopped<'d>, Result<(), EspError>) {
        let Self {
            mut servo,
            resume_position,
            ..
        } = self;
        let result = servo.stop();
        (
            Stopped {
                servo,
                resume_position,
            },
            result,
        )
    }

    /// 自分の`Limiter`の累計counter（#19の受け入れ条件5）。
    ///
    /// **[`LimitedServo::stop`]で`Limiter`ごと捨てるため、`stop`と`resume`を越えて残らない。**
    /// latchの原因のcounterも含め、必要なら`stop`の前に読む。
    #[must_use]
    pub fn counters(&self) -> LimiterCounters {
        self.limiter.counters()
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
