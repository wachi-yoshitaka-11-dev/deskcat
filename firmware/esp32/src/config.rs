//! Firmware の設定値。
//!
//! 周期などの調整可能な値をここへ集める。**呼び出し側へ数値を直接書かない。**
//! 散在させると、根拠を確認する場所と変更する場所が分かれる。

/// Board-configuration ID。
///
/// `crates/deskcat-protocol` の fixture が `"esp32"` を使っており、それへ揃える。
/// 値の意味は同 crate の `Boot` message の `board` field である。
pub const BOARD: &str = "esp32";

/// Heartbeat の周期（milliseconds）。
///
/// **暫定値である。**Protocol §5.7 は `ping` について「**Heartbeatの最終用途と
/// intervalは`TBD`とする。**」としており、この値に一次資料の根拠は無い。
/// serial link（#11）と session（#12）が入って用途が決まった時点で置き換える。
///
/// この定数が heartbeat の rate limit そのものである。**長期の出力 rate は
/// `1 / この周期` を超えない。**保証の正確な範囲（slot ごとに 1 回 ＝ burst 1。
/// 連続する 2 回の間隔が必ずこの周期以上、とまでは保証しない）は
/// `main.rs` の `next_deadline` の doc comment にある。**ここへ再掲しない。**
pub const HEARTBEAT_PERIOD_MS: u32 = 1_000;

/// Health snapshot の周期（milliseconds）。
///
/// **暫定値である。**根拠は [`HEARTBEAT_PERIOD_MS`] と同じく無い。
///
/// Heartbeat より粗くしてある。Heartbeat は「task が進んでいる」ことだけを示す
/// 安価な行であり、snapshot は counter を含む重い行である。Protocol §4.6 が
/// `status` を「必要に応じてrate limit付きの定期health messageとして送信する」と
/// している対応物であり、同じ頻度で出す必要が無い。
pub const HEALTH_SNAPSHOT_PERIOD_MS: u32 = 10_000;

/// `SERVO-PWM`（SG90への制御信号）のGPIO番号。出所は
/// [gpio-assignment.md](../../../docs/hardware/gpio-assignment.md)の`信号inventory`。
/// `esp-idf-hal`はpinをtype levelで選ぶためpin選択には使えず、確認用途のみ
/// （呼び出し側との一致を[`crate::servo::Sg90::new`]の`debug_assert_eq!`で確認）。
/// 既定buildでは未到達のため`#[allow(dead_code)]`（本fileの他のservo定数も同じ理由）。
#[allow(dead_code)]
pub const SERVO_PWM_GPIO: u8 = 27;

/// Servo制御PWMの周期（Hz）。一般値であり一次資料の確定値ではない
/// （根拠は[servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
/// `サーボ識別情報`表`PWM周期／rate`行、`HW-TBD-026`）。
#[allow(dead_code)]
pub const SERVO_PWM_FREQUENCY_HZ: u32 = 50;

/// パルス幅の下限（マイクロ秒、約1 ms）。一般値。根拠は`SERVO_PWM_FREQUENCY_HZ`と同じ。
#[allow(dead_code)]
pub const SERVO_PULSE_WIDTH_MIN_US: u32 = 1_000;

/// パルス幅の中央（マイクロ秒、約1.5 ms）。一般値。現時点では未参照
/// （`pulse_width_us_for_angle`は角度からの線形変換のみを使う）。
#[allow(dead_code)]
pub const SERVO_PULSE_WIDTH_NEUTRAL_US: u32 = 1_500;

/// パルス幅の上限（マイクロ秒、約2 ms）。一般値。根拠は`SERVO_PWM_FREQUENCY_HZ`と同じ。
#[allow(dead_code)]
pub const SERVO_PULSE_WIDTH_MAX_US: u32 = 2_000;

/// `angle_deg`から制御pulse幅への変換に用いる角度規約（`0`〜`180`度が
/// [`SERVO_PULSE_WIDTH_MIN_US`]〜[`SERVO_PULSE_WIDTH_MAX_US`]へ線形対応、`90`度が中央）。
/// この個体の機械的可動域（`HW-TBD-010`）ではない。
#[allow(dead_code)]
pub const SERVO_ANGLE_CONVENTION_MIN_DEG: f32 = 0.0;
#[allow(dead_code)]
pub const SERVO_ANGLE_CONVENTION_NEUTRAL_DEG: f32 = 90.0;
#[allow(dead_code)]
pub const SERVO_ANGLE_CONVENTION_MAX_DEG: f32 = 180.0;

/// 初回動作で許容する、`SERVO_ANGLE_CONVENTION_NEUTRAL_DEG`からの最大偏角（度）。
/// 一次資料にも実測にも基づかない暫定値であり、根拠は無い（`HW-TBD-010`確定まで）。
#[allow(dead_code)]
pub const SERVO_FIRST_MOTION_MAX_DEVIATION_DEG: f32 = 15.0;

/// `bench-servo-test-17` featureで、firmware起動からservoを動かすまで待つ時間
/// （ミリ秒）。人間がservo電源を投入する運用上の猶予であり、安全要件には効かない。
/// 詳細は[servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
/// `決定事項`と`起動とarm delay`stepを参照。
#[allow(dead_code)]
pub const SERVO_BENCH_TEST_ARM_DELAY_MS: u32 = 10_000;

/// `bench-servo-test-17` featureで、1回の指令の後にservoへ信号を出し続ける時間
/// （ミリ秒）。安全な保持時間の主張ではなく露出時間の最小化。
#[allow(dead_code)]
pub const SERVO_BENCH_TEST_EXPOSURE_MS: u32 = 300;
