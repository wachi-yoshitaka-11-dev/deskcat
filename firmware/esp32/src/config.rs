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

/// I2Cの1 transactionのtimeout（milliseconds）。`crate::accel`・`crate::env`が
/// tick数へ換算して`I2cDriver::write_read`へ渡す。
///
/// **一般値である。一次資料に基づく値ではない。**I2C読み出しのtimeout値は
/// [Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)の
/// 安全要件5項目のいずれにも効かないため、同policyの`5項目以外の扱い`により一般値で
/// 開始してよい（[AGENTS.md](../../../AGENTS.md)「推測禁止」の後段。**`TBD`にしない。**）。
/// **どの値が5項目に効くかの判定は同policyが正本であり、ここへ5項目を再掲しない。**
///
/// # 導出
///
/// **「安全な秒数」ではなく「正常なら確実に終わっている時間」として決める。**
///
/// - 1 byte registerの読み出し（`write_read`）は`START`＋`addr+W`＋`reg`＋
///   `repeated START`＋`addr+R`＋`data`＋`STOP`であり、ACK／NACKを含めて約36 bit時間
///   である。bus speedは`main.rs`の`I2C_BAUDRATE_HZ`＝100 kHz（Standard-mode）なので
///   1 bitは10 µs、合計は約0.36 msである。START／repeated START／STOPの分を足しても
///   1 msに満たない。
/// - FreeRTOSのtickは10 msである。`sdkconfig.defaults`が`CONFIG_FREERTOS_HZ`を
///   設定しておらず、ESP-IDF v5.5.3の`components/freertos/Kconfig`の既定値100が効く
///   （build生成物の`sdkconfig`で`CONFIG_FREERTOS_HZ=100`を確認した）。
///   `write_read`のtimeoutはtick単位であるため、10 msが分解能の下限である。
/// - この値は10 tickであり、上の所要時間のおよそ250倍の余裕がある。**正常な読み出しが
///   この値で打ち切られることはない。**
/// - 上限側は、人がlogを読んで状況を判断できる長さに収めるために置く。sensorは2つ
///   （`ACCEL-01`／`ENV-01`）あるため、両方が固着した場合の「待ち」はこの値の2回分で
///   ある。**bring-upからmain loopまでの実際の遅れはそれより長い。**timeout検出後の
///   bus clear待ちが1回ごとに加わるためであり、その分は`main.rs`の`run_i2c_bringup`の
///   doc commentが持つ。**ここへ再掲しない。**いずれにせよ秒のorderには達しない。
///
/// 上限が何によって効くか（ESP-IDFのどの機構が`Err`を返すか）と、
/// `esp_idf_svc::hal::i2c::config::Config`の`timeout`フィールドを設定しない理由は、
/// `main.rs`の`run_i2c_bringup`のdoc commentが持つ。**ここへ再掲しない。**
pub const I2C_TRANSACTION_TIMEOUT_MS: u64 = 100;

/// `DISP-01`のbring-upで、単色fillの各色と向きのpatternを表示したまま保つ時間（milliseconds）。
///
/// [Issue #13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)の条件2〜4
/// （単色fill、color order、四隅とorientation）を写真で残すための待ちである。以前は各色を
/// 描いた直後に次の色を描いていた。patternも描いた直後に`run_display_bringup`が戻り、
/// driverのDropでbacklightが消える（`docs/hardware/experiment-log.md`の`EXP-017`。sourceの
/// 読解による説明であり実測ではない）。どちらも、人が撮る時間が無かった。
///
/// **一般値である。一次資料に基づく値ではない。**表示を保つ時間は
/// [Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)の安全要件5項目の
/// いずれにも効かない。外した場合の帰結は「撮り直す」に留まる。**どの値が5項目に効くかの判定は
/// 同policyが正本であり、ここへ5項目を再掲しない。**この待ちはbacklightの点灯時間を延ばすが、
/// 流れる電流は変えない。`#461`の余裕解析はbacklightが点灯した状態を含む
/// （`main.rs`のmodule doc「`DISP-01`のbring-upを有効にする手順」節）。
///
/// 5色とpatternの合計でこの値の6倍だけ、`run_i2c_bringup`のlog（`accel_device_id`・
/// `env_chip_id`）が遅れて出る。**#487からは、それに描画の時間と、描画の段ごとの待ち（main loopが
/// 1段ごとに最大1 tick＝0〜10 ms待つ。描く段は約100。`crate::display_test`）が加わる**（計算であり、
/// 実測ではない。待ちは上限の見積もりである）。
#[cfg(feature = "bringup-display-13")]
pub const DISPLAY_HOLD_MS: u32 = 3_000;

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

/// LEDCへの出力（`set_duty`）と`Limiter`の1 stepを、N tick（制御周期）に1回にするときのN。
/// [ADR-0027](../../../docs/decisions/0027-servo-output-decimation-for-ledc-update-latency.md)、Issue #708。
///
/// **回ごとの実行承認の値ではなく、設計の定数である。**ESP32のLEDCは`ledc_set_duty_and_update`が
/// 前の更新の効くまで（約40.1〜40.2 ms。2回目のPWMの周期の頭）critical sectionの中でbusy-waitする
/// （ESP-IDF v5.5.3 `ledc.c:1596-1611`）。呼ぶ間隔が約40.2 ms以上ならこの待ちがほぼ起きない。
/// 制御周期20 msなら2では足りない周がありうるので、3から始める
/// （`SERVO_OUTPUT_DECIMATION_TICKS × 制御周期 ≥ PWMの周期 × 2`は[`decimation_meets_pwm_floor`]が
/// compile時に検査する。**これは必要条件であり、十分条件ではない**）。
/// **2へ変えるときはcodeの変更（Pull Request）で行い、`servo_calib_18_phase_trace`の出力したtickの時間で判断する。**
#[allow(dead_code)]
pub const SERVO_OUTPUT_DECIMATION_TICKS: u32 = 3;

/// 間引きの間隔（制御周期の数）が、LEDCの更新の遅れの**必要条件**を満たすか。
/// `n × 制御周期(ms) × PWM周波数(Hz) ≥ 2000`、つまり`n × 制御周期 ≥ PWMの周期 × 2`。
///
/// **十分条件ではない。**実際の更新の遅れ（約40.1〜40.2 ms）を超えるかは、PWMの周期がLEDCの
/// timer分周の丸めで決まる（約20.06〜20.12 ms）ことと、「2回目の周期の頭」で落ちる理由がTRMで
/// 未確認なことから、compile時には検査できない。その確認は、出力したtickの時間のlogで行う。
#[allow(dead_code)]
pub const fn decimation_meets_pwm_floor(
    n: u32,
    control_period_ms: u32,
    pwm_frequency_hz: u32,
) -> bool {
    (n as u64) * (control_period_ms as u64) * (pwm_frequency_hz as u64) >= 2_000
}

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

/// `bench-servo-test-17` featureで、人間がUART0のconsoleへ確認の語を入力してから
/// servoを動かすまで待つ時間（ミリ秒）。人間が手を離して見守る運用上の猶予であり、安全要件には効かない。
/// 起動だけではservoを動かさない（`main.rs`の`wait_for_bench_confirmation`）。
/// 詳細は[servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
/// `決定事項`と`起動とarm delay`stepを参照。
#[allow(dead_code)]
pub const SERVO_BENCH_TEST_ARM_DELAY_MS: u32 = 10_000;

/// `bench-servo-test-17` featureで、1回の指令の後にservoへ信号を出し続ける時間
/// （ミリ秒）。安全な保持時間の主張ではなく露出時間の最小化。
#[allow(dead_code)]
pub const SERVO_BENCH_TEST_EXPOSURE_MS: u32 = 300;

// `bench-servo-calib-18`（#18の校正build）のbuild定数。仕様は
// [servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の`#18の校正（案）`の
// `校正buildの仕様（案）`節の表（定数の意味、決め方、出所はそこが正本。**ここへ写さない**）。
//
// **どの定数にも値を置かない（`None`）。**「運用上の値」を含め、既定値を置かない。**回ごとの実行承認が
// 決めた値を、その回の版として`Some(..)`で書く**（`承認の状態`の項目6の(c)）。**値が1つでも`None`の間、
// このfeature付きのbuildは、`servo_calib`の定数の評価（`calib_18_required`の`panic!`）で
// compileが止まる。**型検査やhost testのための値を、この形で置かない。
//
// 単位は名前のとおり（`_DEG`は角度規約の度、`_MS`はミリ秒、`_S`は秒、`_DEG_S`は度毎秒、`_DEG_S2`は度毎秒毎秒）。

/// limiterの承認値の位置範囲の下端。hard boundも同じ値にする。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_APPROVED_MIN_DEG: Option<f32> = None;
/// limiterの承認値の位置範囲の上端。hard boundも同じ値にする。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_APPROVED_MAX_DEG: Option<f32> = None;
/// `LimitedServo::new`へ渡す起動時の位置。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_START_DEG: Option<f32> = None;
/// `ServoLimits::new`へ渡すneutral。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_NEUTRAL_DEG: Option<f32> = None;
/// 単一commandの最大変化量（`Cap`の承認値。hard boundも同じ値にする）。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_MAX_STEP_DEG: Option<f32> = None;
/// limiterの速度上限（`Cap`の承認値。hard boundも同じ値にする）。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_MAX_VELOCITY_DEG_S: Option<f32> = None;
/// 「目標へ着いた」と判定する、位置と目標の差。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_REACHED_TOLERANCE_DEG: Option<f32> = None;
/// limiterの加速度上限（`Cap`の承認値。hard boundも同じ値にする）。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_MAX_ACCELERATION_DEG_S2: Option<f32> = None;
/// `ControlPeriod`の公称。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_CONTROL_PERIOD_NOMINAL_MS: Option<u32> = None;
/// `ControlPeriod`の許容幅。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_CONTROL_PERIOD_TOLERANCE_MS: Option<u32> = None;
/// 1 commandの露出時間の上限（移動時間を含む）。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_EXPOSURE_MS: Option<u32> = None;
/// 1回の起動で受理するmotion commandの数。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_MAX_COMMANDS: Option<u32> = None;
/// 前のcommandが終わってから、次を受けるまでの最短間隔。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_MIN_COMMAND_INTERVAL_MS: Option<u32> = None;
/// `GO18`から回を打ち切るまでの時間。
#[cfg(feature = "bench-servo-calib-18")]
pub const SERVO_CALIB_18_MAX_SESSION_S: Option<u32> = None;

/// `SERVO_CALIB_18_*`の値を取り出す。**`None`（未設定）のとき、定数の評価が失敗し、compileが止まる。**
/// 失敗の表示は、評価していた定数の名前（`crate::servo_calib`の同名の定数）を指す。
#[cfg(feature = "bench-servo-calib-18")]
pub const fn calib_18_required<T: Copy>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!(
            "SERVO_CALIB_18_*が未設定である。回ごとの実行承認が決めた値を、config.rsへ書いてからbuildする"
        ),
    }
}

/// Pi link（`PI-UART-TX`／`PI-UART-RX`）のbaud（Hz）。
/// **確定値ではない。**`docs/protocol/esp32-pi-protocol.md`§2の`Baud`行が`Candidate`
/// （両端で検証する）とする115200 bpsを使う（`PROTO-TBD-001`、最終baudは未確定）。
/// 直列4.7 kΩを入れたままこのbaudで通信できるかは、初回の接続で確かめる
/// （`docs/hardware/gpio-assignment.md`の`信号線をつないでよい条件`4）。
///
/// **`#487`より前は、UART0のconsole（`CONFIG_ESP_CONSOLE_UART_BAUDRATE`）と揃える
/// ことを根拠にしていた。**Pi linkがUART0から外れたため、その根拠は無くなった
/// （UART0のconsoleとは別のUARTであり、揃える必要が無い）。
#[cfg(not(feature = "bench-servo-test-17"))]
pub const PI_PROTOCOL_UART_BAUDRATE_HZ: u32 = 115_200;

/// Pi link UARTの受信ring buffer容量（byte）。`UartDriver`（interrupt駆動）が
/// hardware FIFOから継続的に吸い上げる先であり、hardware FIFO自体
/// （`SOC_UART_FIFO_LEN`＝128 byte、ESP32の`soc_caps.h`）より大きくなければ
/// `uart_driver_install`が`ESP_FAIL`を返す（ESP-IDF v5.5.3の
/// `esp_driver_uart/src/uart.c`の`rx_buffer_size > UART_HW_FIFO_LEN`検査）。
///
/// # 容量の根拠
///
/// `main()`のloopは、次の締切（heartbeat／health snapshot／`boot`再送）までの
/// 残り時間を`UartDriver::read`のtimeoutへ渡す（`sleep_ms_until`の代わり）。
/// interrupt駆動のring bufferはこの待ちの間もhardware FIFOから継続的に吸い上げる
/// ため、**待ち時間の長さ（heartbeatの`1_000` msなど）そのものはring buffer容量に
/// 効かない。**効くのは、1回の`read`呼び出しから次の呼び出しまでの間にどれだけ
/// 溜まりうるかであり、`read`はdataが来ればtimeoutを待たずに戻る
/// （esp-idf-hal 0.46.2の`UartRxDriver::read`（`uart.rs`1179〜1246行）が、
/// まずnon-blockingで試し、無ければ**1 byteだけ**を実際のtimeoutでblocking
/// 読みし、来たら残りをnon-blockingで拾う、という2段構えの実装になっている
/// ため。「1 byteだけ」の要求に対して、ESP-IDF v5.5.3の`uart.c`の
/// `uart_read_bytes`（1662〜1701行）内部の`xRingbufferReceiveUpTo`は、
/// その1 byteが来た時点で満たされ即座に戻る。同じ`uart_read_bytes`を
/// buffer全長で呼んだ場合は、要求量を満たすかtimeoutまで戻らない
/// （hal側がこの2段構えを採る理由）ため、通常は小さい。**ただし`crate::pi_link::PiLink::on_bytes`の
/// 処理中（応答の書き込みと、`reselect_sid`のNVS操作を含む）はこの`read`を呼ばないため、
/// その間はring bufferが貯まり続ける。**このNVS操作の所要時間は未確認
/// （下記）。
///
/// **`#487`から、`read`と`read`の間にUART0へのdebug log出力も入る。**Pi linkを持つbuildが
/// loggingを止めなくなったためである（`crate::console`参照）。`bringup-display-13`付きbuildでは、
/// LCDの描画の1段（`crate::display_test`）と、LCDのbring-upの後に1回だけ走るI2Cのbring-up（`main.rs`の`i2c_pending`）も入る。heartbeat、health snapshot
/// （JSON 1行）、`boot_tx`等のlogを書いている間も、この`read`は呼ばれない。**log出力の
/// 所要時間は測っていない。**consoleがbyteを送り終えるまで戻らない場合、所要時間は
/// logのbyte数に比例する。UART0のbaudは生成された`sdkconfig`の`CONFIG_ESP_CONSOLE_UART_BAUDRATE`
/// で`115200`と確かめた（`#487`、ESP-IDF v5.5.3、debug profileのbuild出力）。Pi linkも115200 bpsであるため、その間にPi linkへ
/// 届きうるbyte数はlogのbyte数と同程度になる（計算であり、実測ではない）。health snapshotの
/// 1行の長さも測っていない。**この見込みが512 byteに収まるかは確かめていない。**
/// `#487`から、溢れはUART driverのevent（`UART_BUFFER_FULL`／`UART_FIFO_OVF`）として数え、
/// 読みに行く間隔の最大値と一緒にhealth snapshotの行へ出す（`main.rs`の`drain_uart_events`、
/// `crate::health::UartObservations`）。**eventが数えられなかった場合もありうる**（同関数のdoc）。
///
/// **この値は理論値ではなく安全側の見込みである。**`boot`のACK（§6の例で約115 byte）に
/// 続けて`get_status`等の別messageが即座に届く場合（`coordinator::handle_boot`が
/// ACK後に同期送信する。`crates/deskcat-serial/src/coordinator.rs`参照）を想定し、
/// hardware FIFO（128 byte）の4倍を確保して複数行分の余裕を見た。この余裕（4倍）は、
/// main loopが次に`read`を呼ぶまで、受信bufferだけで溢れないための根拠である。
/// **このring bufferが実機で溢れないことはbuildでは示せない。**実機確認の項目とする
/// （`console.rs`のmodule doc参照）。**`generate_sid`が行うNVSへの書き込み・
/// 消去（flash操作）の間、UART受信interruptが遅延・停止しうるかどうかは
/// 確認していない。**`generate_sid`は起動直後の初回呼び出し（`main.rs`の
/// `BootSession::start`の引数評価）でも、`reselect_sid`からの再呼び出しでも
/// 同じ経路を通る。前者は`UartDriver`のinstall後に評価されるため、受信
/// interruptが有効な状態で発生する。書き込みは通常`set_u32`1回分だが、
/// `EspDefaultNvsPartition::take()`が`ESP_ERR_NVS_NO_FREE_PAGES`／
/// `ESP_ERR_NVS_NEW_VERSION_FOUND`を検知した場合は`nvs_flash_erase()`で
/// partition全体を消去する経路もある（`generate_sid`のdoc「衝突許容確率」
/// 節参照。この経路の実在はesp-idf-svcのsourceで確認済み）。**この書き込み・
/// 消去の間、UART受信interruptが遅延・停止しうるかどうかは一次資料で
/// 確認しておらず、この版ではその影響を測っていない。**
#[cfg(not(feature = "bench-servo-test-17"))]
pub const PI_PROTOCOL_UART_RX_BUFFER_BYTES: usize = 512;

/// Pi link UARTの送信ring buffer容量（byte）。受信側ほど余裕を必要と
/// しない。`UartDriver::write`が呼ぶ`uart_write_bytes`→`uart_tx_all`は
/// `portMAX_DELAY`でblockし、渡した全byteをtx ring bufferへ積み終えるまで
/// 戻らない（wireへ送り終えるまでではない。`crate::boot_session`の`send_boot`の
/// comment参照）ため、tx ring bufferが溢れて送信側がdataを失うことは無い。`uart_driver_install`は`tx_fifo_size`にも
/// `> UART_HW_FIFO_LEN`（または`0`）を要求するため、受信側と同じ値にしておく。
#[cfg(not(feature = "bench-servo-test-17"))]
pub const PI_PROTOCOL_UART_TX_BUFFER_BYTES: usize = 512;

/// `main()`のloopで1回の`UartDriver::read`に渡すstack buffer長（byte）。
/// Ring buffer容量（[`PI_PROTOCOL_UART_RX_BUFFER_BYTES`]）より小さくてよい
/// （`read`は複数回に分けて呼ばれ、`crate::pi_link::PiLink`の`LineReceiver`が
/// 受信済みbyteを跨いで行を組み立てる）。stack上に置くため小さく抑えた
/// （ring buffer容量の半分）。**`main()`の他のlocal変数と合わせた合計stack使用量は
/// 測っていない。**task stack sizeを圧迫しないという主張はしない。
#[cfg(not(feature = "bench-servo-test-17"))]
pub const PI_PROTOCOL_UART_READ_CHUNK_BYTES: usize = 256;

/// Pi link UARTのevent queueの長さ（件）。受信の異常（ring bufferの満杯、FIFOの溢れ）を
/// 数えるために使う（`main.rs`の`drain_uart_events`）。**導出した値ではない。**
#[cfg(not(feature = "bench-servo-test-17"))]
pub const PI_PROTOCOL_UART_EVENT_QUEUE_LEN: usize = 32;

/// `LED-COMM`（白、[Issue #514](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514)）の
/// GPIO番号。出所は[gpio-assignment.md](../../../docs/hardware/gpio-assignment.md)の`信号inventory`。
/// `SERVO_PWM_GPIO`と同じく確認用途のみ（[`crate::led::Leds::new`]の`debug_assert_eq!`。debug buildだけ）。
#[cfg(any(feature = "bringup-led-514", deskcat_product_led))]
pub const LED_COMM_GPIO: u8 = 2;

/// `LED-REACT`（赤、#514）のGPIO番号。出所と用途は[`LED_COMM_GPIO`]と同じ。
#[cfg(any(feature = "bringup-led-514", deskcat_product_led))]
pub const LED_REACT_GPIO: u8 = 5;

/// `LED-COMM`の点滅周期（milliseconds）。半分ずつ点灯と消灯にする。
///
/// **一般値である。**値は[led-514-demo.md](../../../docs/hardware/led-514-demo.md)の
/// `役割と表示`が選んだ1秒周期であり、一次資料に基づかない。表示の周期は流れる電流を変えず、
/// 外した場合の帰結は見え方が変わるだけなので、
/// [Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)の安全要件5項目の
/// いずれにも効かない（**どの値が5項目に効くかの判定は同policyが正本であり、ここへ再掲しない**）。
/// **2以上の偶数**であること（[`crate::led`]の割り算の前提。`led.rs`のconst assertが検査する）。
#[cfg(any(feature = "bringup-led-514", deskcat_product_led))]
pub const LED_BLINK_PERIOD_MS: u32 = 1_000;

/// 起動時の点灯試験で、赤だけ・白だけ・両方の各区間を保つ時間（milliseconds）。
///
/// 人間が極性と明るさを見て、点灯中の電流制限抵抗`R`の両端電圧をテスターで読むための待ちである
/// （[led-514-demo.md](../../../docs/hardware/led-514-demo.md)の`段階的な確認`）。
/// **一般値である。**外した場合の帰結は「試験をやり直す」に留まり、安全要件5項目に効かない。
/// 3区間の合計（この値の3倍）だけ、白の点滅が始まるのが遅れる。
/// 0にしない（`led.rs`のconst assertが検査する）。点灯試験を行うのは`bringup-led-514`付きbuildだけである
/// （製品buildはこの値を使わない）。
#[cfg(any(feature = "bringup-led-514", deskcat_product_led))]
pub const LED_LAMP_TEST_HOLD_MS: u32 = 10_000;

/// `happy`の受理で`LED-REACT`（赤）を点ける時間（milliseconds）。
///
/// **一般値である。**値は[led-514-demo.md](../../../docs/hardware/led-514-demo.md)の`役割と表示`が選んだ
/// 1秒であり、一次資料に基づかない。表示の長さは流れる電流を変えず（電流は直列抵抗`R`が決める）、
/// 外した場合の帰結は見え方が変わるだけなので、
/// [Hardware Safety Policy](../../../docs/governance/hardware-safety-policy.md)の安全要件5項目の
/// いずれにも効かない（**どの値が5項目に効くかの判定は同policyが正本であり、ここへ再掲しない**）。
#[cfg(any(feature = "bringup-led-514", deskcat_product_led))]
pub const LED_REACT_PULSE_MS: u32 = 1_000;
