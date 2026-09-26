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
// `pi-protocol-mode`では`crate::accel`／`crate::env`をcompileしないため未到達になる。
#[allow(dead_code)]
pub const I2C_TRANSACTION_TIMEOUT_MS: u64 = 100;

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

/// `pi-protocol-mode`のUART0 baud（Hz）。**一般値ではなく既定buildのconsoleと
/// 揃えた値である。**`CONFIG_ESP_CONSOLE_UART_BAUDRATE`は、既定buildで
/// `cargo build`が生成する`target/xtensa-esp32-espidf/debug/build/esp-idf-sys-*/out/sdkconfig`
/// （`#446` PR B時点、ESP-IDF v5.5.3）で`115200`と確認した。一致させないと、
/// 既定buildと`pi-protocol-mode`で通信速度が変わってしまう。`PROTO-TBD-001`
/// （最終baud）は未確定のままであり、この値もその暫定値の一つである。この定数は
/// `sdkconfig`の値から自動で導出していない（手で揃え続ける前提であり、
/// `sdkconfig`側が変わっても黙ってずれる）。
#[cfg(feature = "pi-protocol-mode")]
pub const PI_PROTOCOL_UART_BAUDRATE_HZ: u32 = 115_200;

/// `pi-protocol-mode`のUART0受信ring buffer容量（byte）。`UartDriver`（interrupt駆動）が
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
/// （hal側がこの2段構えを採る理由）ため、通常は小さい。**ただし`on_bytes`の
/// 処理中（`reselect_sid`のNVS操作を含む）はこの`read`を呼ばないため、
/// その間はring bufferが貯まり続ける。**このNVS操作の所要時間は未確認
/// （下記）。
///
/// **この値は理論値ではなく安全側の見込みである。**`boot`のACK（§6の例で約115 byte）に
/// 続けて`get_status`等の別messageが即座に届く場合（`coordinator::handle_boot`が
/// ACK後に同期送信する。`crates/deskcat-serial/src/coordinator.rs`参照）を想定し、
/// hardware FIFO（128 byte）の4倍を確保して複数行分の余裕を見た。**`BootSession`は
/// `boot`のACK以外を読み捨てる（`Established`後は`on_bytes`の冒頭で即return する。
/// `crate::boot_session::BootSession::on_bytes`参照）ため、Piが送るこれらの
/// messageは処理されずに読み捨てられるだけであり、この余裕（4倍）は「処理する
/// ために必要」ではなく「読み捨てる前に受信bufferだけで溢れないため」の
/// 根拠である。
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
#[cfg(feature = "pi-protocol-mode")]
pub const PI_PROTOCOL_UART_RX_BUFFER_BYTES: usize = 512;

/// `pi-protocol-mode`のUART0送信ring buffer容量（byte）。受信側ほど余裕を必要と
/// しない。`UartDriver::write`が呼ぶ`uart_write_bytes`→`uart_tx_all`は
/// `portMAX_DELAY`でblockし、渡した全byteをtx ring bufferへ積み終えるまで
/// 戻らない（wireへ送り終えるまでではない。`crate::boot_session`の`send_boot`の
/// comment参照）ため、tx ring bufferが溢れて送信側がdataを失うことは無い。`uart_driver_install`は`tx_fifo_size`にも
/// `> UART_HW_FIFO_LEN`（または`0`）を要求するため、受信側と同じ値にしておく。
#[cfg(feature = "pi-protocol-mode")]
pub const PI_PROTOCOL_UART_TX_BUFFER_BYTES: usize = 512;

/// `main()`のloopで1回の`UartDriver::read`に渡すstack buffer長（byte）。
/// Ring buffer容量（[`PI_PROTOCOL_UART_RX_BUFFER_BYTES`]）より小さくてよい
/// （`read`は複数回に分けて呼ばれ、`crate::boot_session::BootSession`が
/// 受信済みbyteを跨いで行を組み立てる）。stack上に置くため小さく抑えた
/// （ring buffer容量の半分）。**`main()`の他のlocal変数と合わせた合計stack使用量は
/// 測っていない。**task stack sizeを圧迫しないという主張はしない。
#[cfg(feature = "pi-protocol-mode")]
pub const PI_PROTOCOL_UART_READ_CHUNK_BYTES: usize = 256;
