//! #18の校正build（`bench-servo-calib-18` feature付きのbuildだけが持つ）。
//!
//! 仕様は[servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の`#18の校正（案）`の
//! `校正buildの仕様（案）`節だけである（**この文書へ値も規則も写さない**）。この文書が書くのは、
//! 仕様の各項目を実装のどこが持つかと、仕様が決めていないために実装が選んだことだけである。
//!
//! **実機では動かしていない。**host testも無い（firmware crateはhostで動かせない）。
//! 回ごとの実行承認（`承認の状態`の項目6の(c)）と、手順・停止基準の文書が揃うまで、このbuildで駆動しない。
//!
//! # 持たないもの
//!
//! Pi link（`crate::boot_session`、UART1）、heartbeat、health snapshot、I2C、LCD、NVSを持たない。
//! `main.rs`の`servo_calib`用の`main`は、識別のlogを出してから[`run`]を呼ぶだけである。
//! **仕様が書いていないheartbeatとI2Cを足さなかった**（`bench-servo-test-17`はI2CのID読み出しとheartbeatを持つ）。
//!
//! # servoを駆動する型
//!
//! [`crate::servo::LimitedServo`]（`Limiter`を通る）だけで出力する。`Sg90::move_to_angle_once`は
//! `bench-servo-test-17`でだけcompileされ、このbuildには無い。
//!
//! # 値
//!
//! 値は`crate::config`の`SERVO_CALIB_18_*`が持つ。**どれも`None`であり、1つでも`None`のbuildは
//! 下の定数の評価でcompileが止まる**（`config::calib_18_required`）。回ごとの実行承認が決めた値を
//! `Some(..)`で書いたものが、その回のfirmwareの版になる。
//!
//! # 仕様が決めておらず、実装が選んだこと
//!
//! - **limiterの`Cap`のhard boundは承認値と同じ値にする**（位置範囲と同じ。`Limiter`は`Cap::hard`を読まない）。
//! - **`LimitedServo`は、次のcommandを待つ間も持つ**（`new`または`resume`で作ったもの。`admit`は出力せず、
//!   拒否してもdutyに触れない）。**`stop`（dutyを0にする）は、動作の終わりと回の終わりだけで呼ぶ**。動作の終わりの
//!   `stop`の後は、すぐ`resume`で作り直して次のcommandを待つ（tickの履歴を持ち越すと、間隔が許容幅の外になりlatchするため）。
//! - **`position_min_deg`／`position_max_deg`は、そのcommandの開始位置を含む。**
//! - **logの名前を足した**: `servo_calib_18_command_refused`（拒否の記録。`reason=`は仕様の名前）、
//!   `servo_calib_18_limiter_counters`（動作中のcommandでは、counterの値を`stop`の前に取り、logは`stop`の後に出す。dutyを先に0にするため。仕様は`stop`の直前に出すと書く）、`servo_calib_18_refusals_dropped`（動作中に拒否した行のlogが溜める上限を超えた件数）、`servo_calib_18_motion_interrupted`（途中で終えたcommandの`motion_done`と同じ項目。`reason=`は`session_end`と同じ）。`reason=`の値で仕様が列挙していない
//!   `session_end`のもの（`stop`、`end`、`max_commands`、`max_session`、`fault`、`stop_failed`、
//!   `console_failed`、`limits_rejected`）は実装が付けた。
//! - **tickの間隔は、`delay_ms(公称周期)`で刻む。**`FreeRtos::delay_ms`はtick単位（`CONFIG_FREERTOS_HZ`。
//!   生成された`sdkconfig`で100なら10 ms）で待つ。最初のtickの前に1 tick待って、tickの境に揃えてから刻む。
//!   **実機での間隔は確かめていない**（commandごとに`tick_min_ms`／`tick_max_ms`をlogに出す。`CONTROL_PERIOD_TOLERANCE_MS`を
//!   決めるための値である）。
//! - **露出の期限（`EXPOSURE_MS`）は、周の始めで判定する。厳密な上限ではない。**期限の前の最後の周で、
//!   通常の待ち（`delay_ms(公称周期)`。tick単位へ切り上げる）、consoleの処理、判定の後のtick1回（limiterの計算と
//!   PWMの設定）の分だけ、期限を超えうる。残りが公称周期以下になった周は、tickの整数倍へ切り下げた時間だけ待って止める。
//! - **`MAX_ACCELERATION_DEG_S2`と速度上限・制御周期の組み合わせが`acceleration_bound_conceded`を立てないことは、
//!   この実装では確かめていない。**`Limiter::step`のdocは、許容幅の内側で立たないとする（`deskcat_servo`の
//!   `tests/trajectory.rs`の組み合わせで1度も立たなかった）が、この校正buildの値で確かめた試験は無い。
//!   立てば`TickError::AccelerationConceded`でlatchし、回を終える（出力しない）。

use std::time::{Duration, Instant};

use deskcat_servo::{
    parse_debug_line, Cap, CommandSource, ControlPeriod, LimiterCounters, LimitsError,
    MotionCatalog, MotionRequest, PositionRange, Rejection, ServoLimits,
};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::sys::EspError;

use crate::config::{self, calib_18_required};
use crate::debug_motion::{ConsoleEvent, DebugConsole};
use crate::servo::{exact_pulse_width_us_for_angle, LimitedServo, Sg90, Stopped};

const APPROVED_MIN_DEG: f32 = calib_18_required(config::SERVO_CALIB_18_APPROVED_MIN_DEG);
const APPROVED_MAX_DEG: f32 = calib_18_required(config::SERVO_CALIB_18_APPROVED_MAX_DEG);
const START_DEG: f32 = calib_18_required(config::SERVO_CALIB_18_START_DEG);
const NEUTRAL_DEG: f32 = calib_18_required(config::SERVO_CALIB_18_NEUTRAL_DEG);
const MAX_STEP_DEG: f32 = calib_18_required(config::SERVO_CALIB_18_MAX_STEP_DEG);
const MAX_VELOCITY_DEG_S: f32 = calib_18_required(config::SERVO_CALIB_18_MAX_VELOCITY_DEG_S);
const REACHED_TOLERANCE_DEG: f32 = calib_18_required(config::SERVO_CALIB_18_REACHED_TOLERANCE_DEG);
const MAX_ACCELERATION_DEG_S2: f32 =
    calib_18_required(config::SERVO_CALIB_18_MAX_ACCELERATION_DEG_S2);
const CONTROL_PERIOD_NOMINAL_MS: u32 =
    calib_18_required(config::SERVO_CALIB_18_CONTROL_PERIOD_NOMINAL_MS);
const CONTROL_PERIOD_TOLERANCE_MS: u32 =
    calib_18_required(config::SERVO_CALIB_18_CONTROL_PERIOD_TOLERANCE_MS);
const EXPOSURE_MS: u32 = calib_18_required(config::SERVO_CALIB_18_EXPOSURE_MS);
const MAX_COMMANDS: u32 = calib_18_required(config::SERVO_CALIB_18_MAX_COMMANDS);
const MIN_COMMAND_INTERVAL_MS: u32 =
    calib_18_required(config::SERVO_CALIB_18_MIN_COMMAND_INTERVAL_MS);
const MAX_SESSION_S: u32 = calib_18_required(config::SERVO_CALIB_18_MAX_SESSION_S);

/// 人間がUART0のconsoleへ入力して、servoを動かしてよいと確かめる語（仕様の起動の1）。
const CONFIRM_TOKEN: &str = "GO18";
/// [`CONFIRM_TOKEN`]の確認として受ける行の長さの上限（byte）。超える行は`too_long`で捨てる。
const CONFIRM_MAX_LINE_BYTES: usize = 16;
/// catalogの唯一のmotion名。
const MOTION_NAME: &str = "calib";
/// consoleを待つ間（動いていないとき）の周期（ms）。人が打つ行を読むためだけの値で、
/// 安全要件の5項目に効かない。
const IDLE_POLL_MS: u32 = 20;
/// 「目標へ着いた」と判定するのに要る、続けて収まるtickの数（仕様の動作の6）。
const REACHED_TICKS: u32 = 2;

/// 回の終わりの後、再起動まで持ち続けるもの。dropするとLEDCがGPIO27をlowへ固定する
/// （`Sg90::stop`のdoc）ので、手放さない。
enum Held {
    /// `LimitedServo::new`が拒否した。
    Servo(Sg90<'static>),
    /// `stop`の後。
    Stopped(Stopped<'static>),
}

/// 受けた行の種類（仕様の動作の4）。大文字小文字を区別する。
#[derive(Debug, PartialEq)]
enum Command<'a> {
    Stop,
    End,
    Motion { name: &'a str, target: f32 },
    Unknown,
}

fn classify(line: &str) -> Command<'_> {
    match line.trim() {
        "STOP" => Command::Stop,
        "END" => Command::End,
        other => match parse_debug_line(other) {
            Ok(motion) => Command::Motion {
                name: motion.name,
                target: motion.target,
            },
            Err(_) => Command::Unknown,
        },
    }
}

/// 拒否の理由。名前は`deskcat_servo::Rejection`の変数名をそのまま出し、校正buildが足す3つを足す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    Limiter(Rejection),
    UnknownCommand,
    RateLimited,
    ExceedsMaxStep,
}

impl Refusal {
    fn name(self) -> String {
        match self {
            Self::Limiter(rejection) => format!("{rejection:?}"),
            Self::UnknownCommand => "UnknownCommand".to_owned(),
            Self::RateLimited => "RateLimited".to_owned(),
            Self::ExceedsMaxStep => "ExceedsMaxStep".to_owned(),
        }
    }
}

/// `MOTION calib <deg>`を、`Limiter`へ渡す前に判定する（仕様の動作の5の(0)〜(iii)。(i)の`Busy`は
/// 動いている間の判定であり、[`run_motion`]が持つ）。
///
/// `since_last_end`は前のcommandの終了からの経過（最初のcommandは`None`）、`position`は直前に出した位置
/// （回の最初は`START_DEG`）。
fn precheck(
    name: &str,
    target: f32,
    since_last_end: Option<Duration>,
    position: f32,
) -> Result<(), Refusal> {
    if name != MOTION_NAME {
        return Err(Refusal::Limiter(Rejection::UnknownMotion));
    }
    if since_last_end
        .is_some_and(|elapsed| elapsed < Duration::from_millis(MIN_COMMAND_INTERVAL_MS.into()))
    {
        return Err(Refusal::RateLimited);
    }
    // NaNは比べて偽になり、(iv)の`LimitedServo::admit`（`NonFiniteTarget`）へ進む。
    if (target - position).abs() > MAX_STEP_DEG {
        return Err(Refusal::ExceedsMaxStep);
    }
    Ok(())
}

fn build_limits() -> Result<(ServoLimits, ControlPeriod), LimitsError> {
    // hard boundは承認値と同じ値にする（module doc）。
    let position = PositionRange::new(
        APPROVED_MIN_DEG,
        APPROVED_MIN_DEG,
        APPROVED_MAX_DEG,
        APPROVED_MAX_DEG,
    )?;
    let limits = ServoLimits::new(
        position,
        NEUTRAL_DEG,
        Cap::new(MAX_VELOCITY_DEG_S, MAX_VELOCITY_DEG_S, "max_velocity")?,
        Cap::new(
            MAX_ACCELERATION_DEG_S2,
            MAX_ACCELERATION_DEG_S2,
            "max_acceleration",
        )?,
        Cap::new(MAX_STEP_DEG, MAX_STEP_DEG, "max_step")?,
    )?;
    let period = ControlPeriod::new(
        CONTROL_PERIOD_NOMINAL_MS as f32 / 1000.0,
        CONTROL_PERIOD_TOLERANCE_MS as f32 / 1000.0,
    )?;
    Ok((limits, period))
}

/// 回の途中で変わる値。
struct State {
    /// 受理したcommandの数。
    accepted: u32,
    /// 直前に出した位置（角度規約の度）。回の最初は`START_DEG`。
    position: f32,
    /// 前のcommandが終わった時刻。
    last_command_end: Option<Instant>,
}

/// 校正buildの入口。**戻らない。**
pub fn run() -> ! {
    // **ここまでGPIO27を設定しない**（`GO18`の入力を受けるまで、`Sg90::new`を呼ばない）。
    let peripherals = Peripherals::take().expect("Peripherals::take must succeed exactly once");
    let console = DebugConsole::start();

    match wait_for_confirmation(&console) {
        Ok(()) => {}
        Err(err) => {
            // 動かさずに終える。再起動まで何も受けない（consoleも読まない）。
            log::error!("servo_calib_18_confirmation_failed error={err}");
            park();
        }
    }

    let started = Instant::now();
    log::info!(
        "servo_calib_18_confirmed approved_min_deg={APPROVED_MIN_DEG} approved_max_deg={APPROVED_MAX_DEG} \
         start_deg={START_DEG} neutral_deg={NEUTRAL_DEG} max_step_deg={MAX_STEP_DEG} \
         max_velocity_deg_s={MAX_VELOCITY_DEG_S} reached_tolerance_deg={REACHED_TOLERANCE_DEG} \
         max_acceleration_deg_s2={MAX_ACCELERATION_DEG_S2} \
         control_period_nominal_ms={CONTROL_PERIOD_NOMINAL_MS} \
         control_period_tolerance_ms={CONTROL_PERIOD_TOLERANCE_MS} exposure_ms={EXPOSURE_MS} \
         max_commands={MAX_COMMANDS} min_command_interval_ms={MIN_COMMAND_INTERVAL_MS} \
         max_session_s={MAX_SESSION_S}"
    );

    let servo = match Sg90::new(
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
        peripherals.pins.gpio27,
    ) {
        Ok(servo) => servo,
        Err(err) => {
            log::error!("servo_calib_18_limits_rejected error=Sg90::new failed: {err}");
            ended(&console, None);
        }
    };

    let held = session(servo, &console, started);
    ended(&console, Some(held))
}

/// 何もせず待ち続ける。
fn park() -> ! {
    loop {
        FreeRtos::delay_ms(1000);
    }
}

/// 回の終わりの後。届いた行は、logに出して捨てる（動かさない）。
fn ended(console: &DebugConsole, held: Option<Held>) -> ! {
    // 手放さない（`Held`のdoc）。
    let _held = held;
    loop {
        match console.try_recv() {
            Some(ConsoleEvent::Line(line)) => {
                log::info!("servo_calib_18_ignored_after_end len={}", line.len());
            }
            Some(ConsoleEvent::TooLong(len)) => {
                log::info!("servo_calib_18_ignored_after_end len={len}");
            }
            Some(ConsoleEvent::Empty) => {
                log::info!("servo_calib_18_ignored_after_end len=0");
            }
            Some(ConsoleEvent::NotUtf8(len)) => {
                log::info!("servo_calib_18_ignored_after_end len={len}");
            }
            Some(ConsoleEvent::ReadFailed(err)) => {
                log::error!("servo_calib_18_console_failed error={err}");
            }
            None => FreeRtos::delay_ms(IDLE_POLL_MS),
        }
    }
}

/// `GO18`（完全一致）を待つ。期限は置かない。
fn wait_for_confirmation(console: &DebugConsole) -> Result<(), String> {
    log::warn!(
        "servo_calib_18_waiting_for_confirmation token={CONFIRM_TOKEN}. \
         servo電源を入れ、立ち会いを確かめてから、このconsoleへ{CONFIRM_TOKEN}と入力して改行する"
    );
    if !console.is_running() {
        return Err("console read thread could not be started".to_owned());
    }
    loop {
        match console.try_recv() {
            None => FreeRtos::delay_ms(IDLE_POLL_MS),
            Some(ConsoleEvent::Line(line)) => {
                if line == CONFIRM_TOKEN {
                    return Ok(());
                }
                let reason = if line.len() > CONFIRM_MAX_LINE_BYTES {
                    "too_long"
                } else {
                    "mismatch"
                };
                log::warn!("servo_calib_18_confirmation_rejected reason={reason}");
            }
            Some(ConsoleEvent::TooLong(_)) => {
                log::warn!("servo_calib_18_confirmation_rejected reason=too_long");
            }
            Some(ConsoleEvent::Empty) => {
                log::warn!("servo_calib_18_confirmation_rejected reason=mismatch");
            }
            Some(ConsoleEvent::NotUtf8(len)) => {
                let reason = if len > CONFIRM_MAX_LINE_BYTES {
                    "too_long"
                } else {
                    "mismatch"
                };
                log::warn!("servo_calib_18_confirmation_rejected reason={reason}");
            }
            Some(ConsoleEvent::ReadFailed(err)) => return Err(err),
        }
    }
}

/// 回を進める。回が終わったら、持ち続けるものを返す。
fn session(servo: Sg90<'static>, console: &DebugConsole, started: Instant) -> Held {
    let (limits, period) = match build_limits() {
        Ok(built) => built,
        Err(err) => {
            log::error!("servo_calib_18_limits_rejected error={err}");
            return Held::Servo(servo);
        }
    };
    let mut ready = match LimitedServo::new(
        servo,
        limits,
        period,
        MotionCatalog::new([MOTION_NAME]),
        START_DEG,
    ) {
        Ok(ready) => ready,
        Err((servo, err)) => {
            log::error!("servo_calib_18_limits_rejected error={err}");
            return Held::Servo(servo);
        }
    };
    let mut state = State {
        accepted: 0,
        position: START_DEG,
        last_command_end: None,
    };
    let max_session = Duration::from_secs(MAX_SESSION_S.into());

    // 受理する数が0なら、1件も受けずに終える。
    if MAX_COMMANDS == 0 {
        return finish(ready, "max_commands", None, &state);
    }

    loop {
        if started.elapsed() >= max_session {
            return finish(ready, "max_session", None, &state);
        }
        let Some(event) = console.try_recv() else {
            FreeRtos::delay_ms(IDLE_POLL_MS);
            continue;
        };
        let line = match event {
            ConsoleEvent::Line(line) => line,
            ConsoleEvent::TooLong(_) => {
                refuse(Refusal::UnknownCommand, "too_long");
                continue;
            }
            ConsoleEvent::Empty => {
                refuse(Refusal::UnknownCommand, "empty");
                continue;
            }
            ConsoleEvent::NotUtf8(_) => {
                refuse(Refusal::UnknownCommand, "not_utf8");
                continue;
            }
            ConsoleEvent::ReadFailed(err) => {
                log::error!("servo_calib_18_console_failed error={err}");
                return finish(ready, "console_failed", Some(err), &state);
            }
        };
        match classify(&line) {
            Command::Stop => return finish(ready, "stop", None, &state),
            Command::End => return finish(ready, "end", None, &state),
            Command::Unknown => refuse(Refusal::UnknownCommand, &format!("{line:?}")),
            Command::Motion { name, target } => {
                let since_last_end = state.last_command_end.map(|end| end.elapsed());
                if let Err(refusal) = precheck(name, target, since_last_end, state.position) {
                    refuse(refusal, &format!("{line:?}"));
                    continue;
                }
                let request = MotionRequest {
                    name,
                    target,
                    source: CommandSource::Debug,
                };
                if let Err(rejection) = ready.admit(&request) {
                    refuse(Refusal::Limiter(rejection), &format!("{line:?}"));
                    continue;
                }
                state.accepted += 1;
                log::info!(
                    "servo_calib_18_command_accepted n={} target_deg={target}",
                    state.accepted
                );
                match run_motion(console, ready, target, &mut state, started) {
                    Motion::Next(next) => ready = next,
                    Motion::Ended(held) => return held,
                }
            }
        }
    }
}

/// 動作中に拒否した行のlogを、`stop`の後まで溜める上限（件）。上限のないallocationを避けるための値で、
/// 安全要件の5項目に効かない。超えた分は件数だけ数える。
const DEFERRED_MAX: usize = 16;

/// 動作中に拒否した行（`stop`の後にlogへ出す）。
#[derive(Default)]
struct Deferred {
    kept: Vec<(Refusal, String)>,
    /// 上限を超えて溜めなかった件数。
    dropped: u32,
}

fn defer(deferred: &mut Deferred, refusal: Refusal, detail: impl FnOnce() -> String) {
    if deferred.kept.len() < DEFERRED_MAX {
        deferred.kept.push((refusal, detail()));
    } else {
        deferred.dropped = deferred.dropped.saturating_add(1);
    }
}

/// `servo_calib_18_limiter_counters`を出す。
fn log_limiter_counters(n: u32, counters: LimiterCounters) {
    log::info!(
        "servo_calib_18_limiter_counters n={n} clamped_position={} clamped_step={} clamped_velocity={} \
         clamped_braking={} clamped_acceleration={} acceleration_bound_conceded={} \
         rejected_invalid_payload={} rejected_out_of_range={}",
        counters.clamped_position,
        counters.clamped_step,
        counters.clamped_velocity,
        counters.clamped_braking,
        counters.clamped_acceleration,
        counters.acceleration_bound_conceded,
        counters.rejected_invalid_payload,
        counters.rejected_out_of_range,
    );
}

fn refuse(refusal: Refusal, detail: &str) {
    log::warn!(
        "servo_calib_18_command_refused reason={} line={detail}",
        refusal.name()
    );
}

/// 1 commandの動作の結果。
enum Motion {
    /// 次のcommandを待てる（`stop`の後、`resume`で作り直した）。
    Next(LimitedServo<'static>),
    /// 回が終わった。
    Ended(Held),
}

/// 受理した目標へ動かす（仕様の動作の3(b)・6・8）。
fn run_motion(
    console: &DebugConsole,
    mut servo: LimitedServo<'static>,
    target: f32,
    state: &mut State,
    session_started: Instant,
) -> Motion {
    let period = Duration::from_millis(CONTROL_PERIOD_NOMINAL_MS.into());
    let exposure = Duration::from_millis(EXPOSURE_MS.into());
    let max_session = Duration::from_secs(MAX_SESSION_S.into());
    let n = state.accepted;

    let motion_start = Instant::now();
    let mut position = state.position;
    let mut position_min = position;
    let mut position_max = position;
    let mut last_tick_at: Option<Instant> = None;
    let mut tick_min: Option<Duration> = None;
    let mut tick_max: Option<Duration> = None;
    let mut reached_ticks = 0_u32;
    // 回を途中で終える理由と、その詳細。
    let mut interrupted: Option<(&'static str, Option<String>)> = None;
    let mut done_reason: Option<&'static str> = None;
    // 動作中に受けて拒否した行。`stop`の後にlogへ出す。
    let mut deferred = Deferred::default();

    // tickの境へ揃えてから最初のtick（基準を取るだけで、出力しない）を呼ぶ（module doc）。
    FreeRtos::delay_ms(1);

    while interrupted.is_none() && done_reason.is_none() {
        // (a) console。動いている間に受けるのは`STOP`だけで、それ以外は拒否する。拒否のlogは、`stop`の後に出す
        // （logの時間だけdutyが残らないように）。
        while let Some(event) = console.try_recv() {
            match event {
                ConsoleEvent::Line(line) => match classify(&line) {
                    Command::Stop => {
                        interrupted = Some(("stop", None));
                        break;
                    }
                    Command::End => defer(&mut deferred, Refusal::Limiter(Rejection::Busy), || {
                        format!("{line:?}")
                    }),
                    Command::Motion { name, .. } => {
                        let refusal = if name == MOTION_NAME {
                            Refusal::Limiter(Rejection::Busy)
                        } else {
                            Refusal::Limiter(Rejection::UnknownMotion)
                        };
                        defer(&mut deferred, refusal, || format!("{line:?}"));
                    }
                    Command::Unknown => defer(&mut deferred, Refusal::UnknownCommand, || {
                        format!("{line:?}")
                    }),
                },
                ConsoleEvent::TooLong(_) => defer(&mut deferred, Refusal::UnknownCommand, || {
                    "too_long".to_owned()
                }),
                ConsoleEvent::Empty => defer(&mut deferred, Refusal::UnknownCommand, || {
                    "empty".to_owned()
                }),
                ConsoleEvent::NotUtf8(_) => defer(&mut deferred, Refusal::UnknownCommand, || {
                    "not_utf8".to_owned()
                }),
                ConsoleEvent::ReadFailed(err) => {
                    interrupted = Some(("console_failed", Some(err)));
                    break;
                }
            }
        }
        if interrupted.is_none() && session_started.elapsed() >= max_session {
            interrupted = Some(("max_session", None));
        }
        if interrupted.is_some() {
            break;
        }

        // 期限は、周の始めで判定する。達していれば、tickせず（新しい位置を出さず）に終える。前の周の待ち
        // （tick単位へ切り上げる）とconsoleの処理で、期限を過ぎていることがある。判定の後のtick1回
        // （limiterの計算とPWMの設定）の間も、期限を超えうる。
        if motion_start.elapsed() >= exposure {
            done_reason = Some("exposure_elapsed");
            break;
        }

        // (b) tick。
        let now = Instant::now();
        if let Some(previous) = last_tick_at.replace(now) {
            let interval = now.duration_since(previous);
            tick_min = Some(tick_min.map_or(interval, |min| min.min(interval)));
            tick_max = Some(tick_max.map_or(interval, |max| max.max(interval)));
        }
        match servo.tick() {
            Ok(None) => {}
            Ok(Some(setpoint)) => {
                position = setpoint.position();
                position_min = position_min.min(position);
                position_max = position_max.max(position);
                if (position - target).abs() <= REACHED_TOLERANCE_DEG {
                    reached_ticks += 1;
                } else {
                    reached_ticks = 0;
                }
            }
            Err(err) => {
                interrupted = Some(("fault", Some(err.to_string())));
                break;
            }
        }
        if reached_ticks >= REACHED_TICKS {
            done_reason = Some("reached");
        } else {
            // 次のtickの前に露出が尽きるなら、tickせず、残りだけ待って止める
            // （`EXPOSURE_MS`は厳密な上限ではない。期限は周の始めで判定するので、通常の待ち・consoleの処理・
            // tick1回の分だけ超えうる）。
            // `delay_ms`はtick単位へ切り上げるので、待ちはtickの整数倍へ切り下げる。残りがtick1つに
            // 満たなければ待たずに止める（上限より早く止める側へ倒す）。
            let remaining = exposure.saturating_sub(motion_start.elapsed());
            if remaining <= period {
                // tick数で数える（`CONFIG_FREERTOS_HZ`が1000を割り切れなくても、残りを超えて待たない）。
                // `delay_ms`はmsをtick数へ切り上げるので、切り下げたtick数に対応するmsを切り上げて渡す。
                let hz = u64::from(esp_idf_svc::sys::CONFIG_FREERTOS_HZ);
                let wait_ticks =
                    u64::try_from(remaining.as_millis()).unwrap_or(u64::MAX) * hz / 1000;
                if wait_ticks > 0 {
                    let wait_ms = (wait_ticks * 1000).div_ceil(hz);
                    FreeRtos::delay_ms(u32::try_from(wait_ms).unwrap_or(u32::MAX));
                }
                done_reason = Some("exposure_elapsed");
            } else {
                FreeRtos::delay_ms(CONTROL_PERIOD_NOMINAL_MS);
            }
        }
    }

    // counterの値は`stop`の前に取る（`stop`で`Limiter`ごと捨てる）。logは`stop`の後に出す（dutyを先に0にする）。
    let counters = servo.counters();
    let (stopped, stop_result) = servo.stop();
    // 動いていた時間は、`stop`の直後に取る（後のlogの時間を含めない）。
    let motion_elapsed = motion_start.elapsed();
    state.position = stopped.resume_position();
    log_limiter_counters(n, counters);
    for (refusal, detail) in &deferred.kept {
        refuse(*refusal, detail);
    }
    if deferred.dropped > 0 {
        log::warn!("servo_calib_18_refusals_dropped count={}", deferred.dropped);
    }

    let fmt_ms = |interval: Option<Duration>| {
        interval.map_or("none".to_owned(), |d| {
            format!("{:.2}", d.as_secs_f32() * 1000.0)
        })
    };
    // 途中で終えたcommandも、間隔と位置を残す（`tick`の間隔のrejectでlatchした回の間隔を、logで読むため）。
    let measures = format!(
        "position_deg={position} position_min_deg={position_min} position_max_deg={position_max} \
         pulse_us={:.1} elapsed_ms={} tick_min_ms={} tick_max_ms={}",
        exact_pulse_width_us_for_angle(position),
        motion_elapsed.as_millis(),
        fmt_ms(tick_min),
        fmt_ms(tick_max),
    );
    if let Some((reason, error)) = interrupted {
        if let ("console_failed", Some(err)) = (reason, &error) {
            log::error!("servo_calib_18_console_failed error={err}");
        }
        log::info!("servo_calib_18_motion_interrupted n={n} reason={reason} {measures}");
        return Motion::Ended(end_session(stopped, stop_result, reason, error));
    }
    let Some(reason) = done_reason else {
        unreachable!("the motion loop ends only with a reason");
    };
    log::info!("servo_calib_18_motion_done n={n} reason={reason} {measures}");
    state.last_command_end = Some(Instant::now());

    if stop_result.is_err() {
        return Motion::Ended(end_session(stopped, stop_result, "stop_failed", None));
    }
    if n >= MAX_COMMANDS {
        return Motion::Ended(end_session(stopped, stop_result, "max_commands", None));
    }
    let (limits, control_period) = match build_limits() {
        Ok(built) => built,
        Err(err) => {
            log::error!("servo_calib_18_limits_rejected error={err}");
            return Motion::Ended(end_session(
                stopped,
                stop_result,
                "limits_rejected",
                Some(err.to_string()),
            ));
        }
    };
    match LimitedServo::resume(
        stopped,
        limits,
        control_period,
        MotionCatalog::new([MOTION_NAME]),
    ) {
        Ok(next) => Motion::Next(next),
        Err((stopped, err)) => {
            log::error!("servo_calib_18_limits_rejected error={err}");
            Motion::Ended(end_session(
                stopped,
                stop_result,
                "limits_rejected",
                Some(err.to_string()),
            ))
        }
    }
}

/// 動いていないときに回を終える。dutyを0にする。
fn finish(
    servo: LimitedServo<'static>,
    reason: &'static str,
    error: Option<String>,
    state: &State,
) -> Held {
    log_limiter_counters(state.accepted, servo.counters());
    let (stopped, stop_result) = servo.stop();
    end_session(stopped, stop_result, reason, error)
}

/// `servo_calib_18_session_end`を出す。`stop`が失敗していれば、`reason`を`stop_failed`にしてerrorで出す
/// （人が外部電源を切る）。
fn end_session(
    stopped: Stopped<'static>,
    stop_result: Result<(), EspError>,
    reason: &'static str,
    error: Option<String>,
) -> Held {
    let faults = stopped.fault_counters();
    let position = stopped.resume_position();
    let counters = format!(
        "interval_rejections={} output_failures={} acceleration_conceded={} faulted_ticks={}",
        faults.interval_rejections,
        faults.output_failures,
        faults.acceleration_conceded,
        faults.faulted_ticks,
    );
    let detail = error.map_or(String::new(), |error| format!(" error={error}"));
    match stop_result {
        Ok(()) => log::info!(
            "servo_calib_18_session_end reason={reason} position_deg={position} {counters}{detail}"
        ),
        Err(stop_error) => log::error!(
            "servo_calib_18_session_end reason=stop_failed first_reason={reason} position_deg={position} \
             {counters}{detail} stop_error={stop_error}. 人が外部電源を切る"
        ),
    }
    Held::Stopped(stopped)
}
