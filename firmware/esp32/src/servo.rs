//! `SERVO-01`（SG90首振りサーボ）のPWM driver。
//!
//! [Issue #17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、
//! [Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)。
//! 首振り、fail-safe、拘束検知は別途進める。
//!
//! # 角度を出せるのは、limiterを所有する[`LimitedServo`]だけである
//!
//! **module の外から角度を指定してpulseを出す入口は[`LimitedServo::tick`]だけである。**
//! [`LimitedServo`]は[`Sg90`]と`deskcat_servo::Limiter`を**1つずつ所有**し、外へ出す操作は
//! [`LimitedServo::admit`]、[`LimitedServo::tick`]、[`LimitedServo::stop`]、
//! [`LimitedServo::counters`]、再開の[`LimitedServo::resume`]だけである（#19の受け入れ条件2・3、
//! `AGENTS.md`「サーボ安全制限をデバッグ経路からも迂回させない」）。
//!
//! - **`Setpoint`も`AdmittedTarget`も外から受け取らない。**`admit`は受理したtargetを内部に持ち、
//!   `tick`はそのtargetへ向けて**自分の**`Limiter::step`を呼び、結果をその場で出力する。
//!   別に作った`Limiter`の結果や、保存しておいた古い`Setpoint`を流し込む経路が無い。
//!   `Setpoint`は`Copy`であり、出力を受け取る入口があると、古い値を後で出し直して
//!   速度と加速度の制限を通らずに位置を跳ばせてしまうためである
//! - **制御周期の間隔を呼び出し側から受け取らない。**`tick`は前回の`tick`からの経過時間を
//!   単調時計（`std::time::Instant`）で**自分で測り**、それを`Limiter::step`へ渡す。
//!   間隔を申告させると、公称値を申告したまま短い間隔で呼ぶだけで、実時間の速度と加速度が
//!   `Limiter`の制限を超えるためである
//! - **servo 1つにつき`Limiter`は1つである。**[`LimitedServo::new`]は[`Sg90`]を値で受け取り、
//!   [`Sg90::new`]はLEDC timer・channel・pin（`Peripherals`から1回だけ取れる）を消費する。
//!   止めた後は[`Stopped`]になり、[`Stopped`]から[`Sg90`]を取り出す手段は無い
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
//! # 間隔を測れなかったとき、出力に失敗したときはlatchする
//!
//! 次のどちらかが起きたら、**以後の`tick`は出力も`Limiter`の更新もせず[`TickError::Faulted`]を
//! 返す。**解除の経路は、[`LimitedServo::stop`]の後に[`LimitedServo::resume`]で作り直すことだけである。
//!
//! - **測った間隔が`Limiter`の制御周期（`ControlPeriod`）の外にある**（[`TickError::Rejected`]）。
//!   遅れの後にそのまま再開すると、その間止まっていた実機へ、`Limiter`に残った速度を
//!   次の周期に指令することになり、`最大加速度`を超えうる。**jitterが許容幅を超えると、
//!   servoはその位置で止まる（最後のdutyを保つ）。**安全側に倒れる既知の性質である
//! - **`Limiter::step`の後にPWMの出力が失敗した**（[`TickError::Output`]）。`Limiter`だけが1 step進み、
//!   実際の出力は前の値のまま残るため、次の`tick`は1周期で2 step分の変化を指令し、
//!   `最大速度`（最大で2倍）と`最大加速度`を超えうる
//!
//! これは`servo-safety-limits.md`の`起動時とdriver故障時の動作`（`HW-TBD-019`、PWM driverの
//! 実行中の故障を検知したときの動作は未確定）の中で、「未検証の動作出力を行わない」側の
//! 既定として置いたものである。**値も復帰の条件も決めていない。**
//!
//! **latchは「それ以上動かさない」までであり、「出力を止める」ではない。**LEDCは、最後に
//! 設定したdutyを出し続けうる。**[`TickError::Rejected`]、[`TickError::Output`]、
//! [`TickError::Faulted`]を受けた呼び出し側は、[`LimitedServo::stop`]を呼ぶ。**
//! その配線（`main.rs`）はまだ無い。
//!
//! # 止めた後の再開は、最後に出力した位置からだけ
//!
//! [`LimitedServo::stop`]は自分を消費して[`Stopped`]を返す。[`Stopped`]は[`Sg90`]と
//! **最後に出力した位置**をprivateに持つ。再開の[`LimitedServo::resume`]は、渡された`Limiter`の
//! 位置が最後に出力した位置と一致しなければ拒否する。呼び出し側は
//! [`Stopped::last_output_position`]を初期位置として新しい`Limiter`（速度0）を作る。
//!
//! - **止める直前の速度を持ち越さない。**持ち越すと、duty 0で止まっていたservoへ、再開の
//!   最初の周期にその速度を指令することになり、`最大加速度`を超えうる
//! - **呼び出し側が選んだ位置から再開させない。**選べると、その位置へ`Limiter`の速度・加速度の
//!   制限を受けずに跳べてしまう
//! - 一度も出力していない（`last_output_position`が`None`）場合は、位置を問わない。
//!   その場合は下の起動時と同じ扱いになる
//!
//! # 既知の制限: 起動時の初期位置は実際の位置と一致する保証が無い
//!
//! [`LimitedServo::new`]へ渡す`Limiter`の初期位置は、呼び出し側が渡す値である。
//! **servoの実際の位置をfirmwareは測れない。**両者が食い違っていると、最初の`admit`の後の`tick`で、
//! servoは実際の位置からその初期位置付近へ**`Limiter`の速度・加速度の制限を受けずに**動きうる
//! （servo内部の制御が追従するため）。起動時の位置の扱いは`HW-TBD-019`／`HW-TBD-020`の範囲であり、
//! **このmoduleは値も扱いも決めない。**
//!
//! 止めた後も、duty 0の間にservoが押されて動けば、最後に出力した位置と実際の位置は食い違う。
//! これも測れないことによる制限であり、呼び出し側が選べる経路ではない。
//!
//! # 出力を止めるものと、止めないもの
//!
//! - **[`LimitedServo::stop`]:** dutyを0へ戻す（[`Sg90::stop`]）。**失敗したら、人が外部電源を切る。**
//! - **drop（panicを除く）:** [`Sg90`]が持つ`esp_idf_svc::hal::ledc::LedcDriver`の`Drop`が
//!   `ledc_stop`をidle level `0`で呼び、GPIO27をlowに固定する（`esp-idf-hal` 0.46.2の
//!   `src/ledc.rs`）。**ただしその`Drop`は`ledc_stop`の失敗を`unwrap`するため、失敗すると
//!   panicになり、下のpanicの経路に入る。**
//! - **panic、およびCPUのsoftware reset（watchdogによるresetを含む）では止まらない。**
//!   firmwareは`panic = "abort"`でbuildする（`firmware/esp32/.cargo/config.toml`の`build-std`と、
//!   target `xtensa-esp32-espidf`の`panic-strategy`）ため`Drop`は走らない。ESP-IDF v5.5.3の
//!   panic handlerは、buildが生成する`sdkconfig`の`CONFIG_ESP_SYSTEM_PANIC_PRINT_REBOOT=y`
//!   （既定値。`sdkconfig.defaults`では設定していない）により`esp_restart_noos`を呼ぶ。
//!   これがresetするperipheralにLEDCは含まれない（`components/esp_system/port/soc/esp32/system_internal.c`の
//!   `esp_system_reset_modules_on_exit`。`esp_system.h`の`esp_restart`のdocも
//!   「Peripherals (except for Wi-Fi, BT, UART0, SPI1, and legacy timers) are not reset」と書く）。
//!   reset理由がCPUのsoftware resetのとき、起動処理はそれまで有効だったperipheralのclockを
//!   止めない（`components/esp_system/port/soc/esp32/clk.c`）。**したがってLEDCは最後のdutyを
//!   出し続けうる。**既定buildはGPIO27を初期化しないため、電源を切るまで続きうる。
//!   **止めるのは、起動の早い段階で[`Sg90::new`]を作り直すこと**（`LedcDriver::new`がchannelを
//!   duty 0で設定し直す）**と、人が外部電源を切ることである。**起動時の配線（`main.rs`）はまだ無い
//!   （`HW-TBD-019`の`Watchdog、panic、brownout reset後の動作`）。**いずれも実機では確かめていない。**
//!   source を読んだ結果である
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

use deskcat_servo::{AdmittedTarget, Limiter, LimiterCounters, MotionRequest, Rejection, Setpoint};
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
/// **値を持たない。**`Limiter`（可動域、速度、加速度、`単一commandの最大変化量`、
/// 制御周期、受理するmotion名、初期位置）は呼び出し側が作って渡す。
pub struct LimitedServo<'d> {
    servo: Sg90<'d>,
    limiter: Limiter,
    target: Option<AdmittedTarget>,
    /// 前回の`tick`の時刻。`admit`の後の最初の`tick`で基準として取る。
    last_tick: Option<Instant>,
    /// 最後に出力できた位置（角度規約の度）。一度も出力していなければ`None`。
    last_output: Option<f32>,
    /// latchしているか（module docの「間隔を測れなかったとき、出力に失敗したときはlatchする」）。
    faulted: bool,
}

/// [`LimitedServo::stop`]の後の状態。[`Sg90`]と最後に出力した位置を持つ。
///
/// **[`Sg90`]を取り出す手段は無い。**再開は[`LimitedServo::resume`]だけである
/// （module docの「止めた後の再開は、最後に出力した位置からだけ」）。
pub struct Stopped<'d> {
    servo: Sg90<'d>,
    last_output: Option<f32>,
}

impl Stopped<'_> {
    /// 最後に出力した位置（角度規約の度）。再開の`Limiter`はこれを初期位置として作る。
    /// 一度も出力していなければ`None`。
    #[must_use]
    pub fn last_output_position(&self) -> Option<f32> {
        self.last_output
    }
}

/// [`LimitedServo::new`]と[`LimitedServo::resume`]が拒否した理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitedServoError {
    /// `Limiter`の承認値の位置範囲が、角度規約（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜
    /// `_MAX_DEG`）の外へはみ出している。
    ///
    /// はみ出した分はpulse幅の変換で黙って飽和し、`Limiter`が認めた位置と
    /// 実際に出す位置が食い違う。**黙って受けずに構築を拒否する。**
    ApprovedRangeOutsideAngleConvention,
    /// 再開の`Limiter`の位置が、最後に出力した位置と一致しない（[`LimitedServo::resume`]）。
    PositionDiffersFromLastOutput,
}

impl fmt::Display for LimitedServoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApprovedRangeOutsideAngleConvention => {
                f.write_str("approved position range lies outside the servo angle convention")
            }
            Self::PositionDiffersFromLastOutput => {
                f.write_str("limiter position differs from the last output position")
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

/// `limiter`の承認値の位置範囲が角度規約の内側にあるか。
fn within_angle_convention(limiter: &Limiter) -> bool {
    let range = limiter.limits().position();
    range.approved_min() >= crate::config::SERVO_ANGLE_CONVENTION_MIN_DEG
        && range.approved_max() <= crate::config::SERVO_ANGLE_CONVENTION_MAX_DEG
}

impl<'d> LimitedServo<'d> {
    /// 起動時に[`Sg90`]と`Limiter`を所有する。**この時点では何も出力しない。**
    ///
    /// `limiter`の初期位置は実際の位置と一致する保証が無い（module docの「既知の制限」）。
    ///
    /// # Errors
    ///
    /// `limiter`の承認値の位置範囲が角度規約の外へはみ出している場合に拒否し、
    /// [`Sg90`]を返す（[`LimitedServoError::ApprovedRangeOutsideAngleConvention`]）。
    pub fn new(servo: Sg90<'d>, limiter: Limiter) -> Result<Self, (Sg90<'d>, LimitedServoError)> {
        if !within_angle_convention(&limiter) {
            return Err((
                servo,
                LimitedServoError::ApprovedRangeOutsideAngleConvention,
            ));
        }
        Ok(Self {
            servo,
            limiter,
            target: None,
            last_tick: None,
            last_output: None,
            faulted: false,
        })
    }

    /// [`LimitedServo::stop`]の後に、新しい`Limiter`で作り直す。**この時点では何も出力しない。**
    ///
    /// `limiter`は[`Stopped::last_output_position`]を初期位置として作る（速度0）。
    ///
    /// # Errors
    ///
    /// 承認値の位置範囲が角度規約の外へはみ出している場合、または`limiter`の位置が
    /// 最後に出力した位置と一致しない場合に拒否し、[`Stopped`]を返す。
    /// 位置は**bit単位で一致**を求める（近似で比べると、差の分だけ制限なしに跳べる）。
    pub fn resume(
        stopped: Stopped<'d>,
        limiter: Limiter,
    ) -> Result<Self, (Stopped<'d>, LimitedServoError)> {
        if !within_angle_convention(&limiter) {
            return Err((
                stopped,
                LimitedServoError::ApprovedRangeOutsideAngleConvention,
            ));
        }
        if let Some(last) = stopped.last_output {
            if limiter.position().to_bits() != last.to_bits() {
                return Err((stopped, LimitedServoError::PositionDiffersFromLastOutput));
            }
        }
        Ok(Self {
            servo: stopped.servo,
            limiter,
            target: None,
            last_tick: None,
            last_output: stopped.last_output,
            faulted: false,
        })
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
    /// その時間だけ進め、結果を出力する。
    ///
    /// **何も出力せず`Ok(None)`を返す場合が2つある。**targetが無い（まだ`admit`していない）とき、
    /// と、`admit`の後の最初の`tick`（時刻の基準を取るだけ）である。起動時の出力は
    /// `HW-TBD-019`で未確定であり、`servo-safety-limits.md`の`起動時とdriver故障時の動作`が
    /// 「承認されるまで、安全状態は『未検証の動作出力を行わない』とする」としているためである。
    ///
    /// 返す`Setpoint`は**出力した値の報告**（log用）である。これを受け取る入口は無い。
    ///
    /// # Errors
    ///
    /// 測った間隔を`Limiter::step`がrejectした場合は[`TickError::Rejected`]（何も出力しない）、
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
        self.last_output = Some(setpoint.position());
        Ok(Some(setpoint))
    }

    /// dutyを0へ戻し（[`Sg90::stop`]）、**自分を消費して**[`Stopped`]を返す。
    ///
    /// `Limiter`とtargetはここで捨てる。再開は[`LimitedServo::resume`]だけである
    /// （module docの「止めた後の再開は、最後に出力した位置からだけ」）。
    ///
    /// 返す`Result`は[`Sg90::stop`]の結果である。**失敗を握りつぶさない。**
    /// **失敗したら、人が外部電源を切る**（module docの「出力を止めるものと、止めないもの」。
    /// dropに任せると、`LedcDriver`の`Drop`の`unwrap`がpanicになり、LEDCが止まらない経路に入る）。
    #[must_use = "dutyを0へ戻せたかをResultで確かめること"]
    pub fn stop(self) -> (Stopped<'d>, Result<(), EspError>) {
        let Self {
            mut servo,
            last_output,
            ..
        } = self;
        let result = servo.stop();
        (Stopped { servo, last_output }, result)
    }

    /// 自分の`Limiter`の累計counter（#19の受け入れ条件5）。
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
