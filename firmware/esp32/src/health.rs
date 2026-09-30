//! Health snapshot。
//!
//! 起動からの uptime、heartbeat の連番、および `crates/deskcat-protocol` の
//! [`ProtocolCounters`] を保持する。
//!
//! **新しい counter 型を作らない。**Protocol counter は同 crate の
//! [`ProtocolCounters`] をそのまま使う。各 field の意味の正本は Protocol §4.6 の
//! counter 対応表であり、ここへ再掲しない。
//!
//! **この`Health`型はsession state（#12）を持たない。**Pi linkの`boot`の受理確認・
//! 再送は`crate::boot_session`が、`hello`等の受理は`crate::protocol::PiSession`が持つ。
//! どの構成でも[`ProtocolCounters`]を増やす経路は無いため、**すべて 0 のままである**。
//! ここで示すのは「counter schema を `status` へ載せられる」ことであって、
//! 「counter が動いている」ことではない。
//!
//! # Pi linkのUARTの観測（[#487]）
//!
//! [`UartObservations`]は、Pi linkのUART（UART1）の受信で起きた異常のeventの数と、
//! main loopが受信を読みに行く間隔の最大値を持つ。**Protocol counterではない**
//! （§4.6の`protocol`へ載せず、health snapshotのlogの行にだけ出す）。受信のring buffer
//! （`config::PI_PROTOCOL_UART_RX_BUFFER_BYTES`）が溢れたかどうかは、以前はどこからも
//! 観測できなかった（`crate::console`のmodule doc (2)）。数え方と限界は`main.rs`の
//! `drain_uart_events`のdocが持つ。
//!
//! [#487]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487

use std::time::Instant;

use deskcat_protocol::{DisplayStatus, ProtocolCounters, SensorStatus, ServoStatus, Status};

/// 未初期化の subsystem に使う状態名。
///
/// **未初期化のものへ状態名を宣言しない。**Protocol §4.6 の例が使う `ready`／
/// `neutral`／`disabled` は初期化を経た状態を指す。この firmware は
/// `Peripherals::take()` を呼ばず、display も servo も sensor も存在しないため、
/// それらの語を使うと実態より強い主張になる。
///
/// `deskcat_protocol::limits::MAX_STATE_NAME_BYTES`（32）に収まる。
const UNKNOWN_STATE: &str = "unknown";

/// Firmware task の health。
pub struct Health {
    /// 起動時点。uptime の起点。
    boot: Instant,
    /// Heartbeat の連番。起動ごとに 0 から始まる。
    heartbeat_seq: u64,
    /// 期限を 1 周期以上過ぎた回数。
    ///
    /// **Protocol counter ではない。**[`ProtocolCounters::rate_limited`] を流用しない。
    /// §4.6 は `rate_limited` を「受理上限／session遷移budget・cooldown／servoの
    /// 受理command数超過の合算」と定めており、heartbeat の遅延はそのいずれでもない。
    overrun_ticks: u32,
    /// Snapshot の serialize に失敗した回数。**Protocol counter ではない。**
    snapshot_errors: u32,
    /// Protocol counter。**すべて 0 のままである**（module doc 参照）。
    counters: ProtocolCounters,
    /// Machine-readable な reset reason。
    reset_reason: &'static str,
    /// Pi linkのUARTの観測（module doc「Pi linkのUARTの観測」）。
    uart: UartObservations,
}

/// Pi linkのUARTの受信で観測したeventの数と、受信を読みに行く間隔の最大値。
///
/// **Protocol counterではない**（`Health`のmodule doc参照）。
#[derive(Debug, Clone, Copy, Default)]
pub struct UartObservations {
    /// ESP-IDFのUART driverが報告した、受信のring bufferの満杯（`UART_BUFFER_FULL`）の回数。
    pub rx_buffer_full: u32,
    /// hardware FIFOの溢れ（`UART_FIFO_OVF`）の回数。
    pub rx_fifo_overflow: u32,
    /// frame error（`UART_FRAME_ERR`）の回数。baudの不一致や、信号の乱れで起きうる。
    pub frame_error: u32,
    /// parity error（`UART_PARITY_ERR`）の回数。8N1ではparityを使わないため、通常は0である。
    pub parity_error: u32,
    /// 直前のhealth snapshotから今までの、受信を読みに行く間隔の最大値（ms）。
    /// 間隔は、`UartDriver::read`から戻ってから次の`read`を始めるまでの時間であり、`read`の中で
    /// 待っている時間を含まない（その間はdriverが受信を読み進める）。この間にring bufferへ貯まる
    /// byte数が、溢れの余裕を決める（`config::PI_PROTOCOL_UART_RX_BUFFER_BYTES`のdoc）。
    /// health snapshotを出すたびに0へ戻す。
    pub max_read_gap_ms: u64,
}

impl Health {
    /// 起動時点を起点として作る。
    pub fn new(reset_reason: &'static str) -> Self {
        Self {
            boot: Instant::now(),
            heartbeat_seq: 0,
            overrun_ticks: 0,
            snapshot_errors: 0,
            counters: ProtocolCounters::default(),
            reset_reason,
            uart: UartObservations::default(),
        }
    }

    /// Pi linkのUARTの観測を書き換えるために借りる。
    pub fn uart_mut(&mut self) -> &mut UartObservations {
        &mut self.uart
    }

    /// Pi linkのUARTの観測を返す。
    pub fn uart(&self) -> UartObservations {
        self.uart
    }

    /// 起動からの経過時間（milliseconds）。
    ///
    /// 型は `u64` である。Protocol §3 が `ts_ms` に `u64` を採ったのは
    /// 「`u32`は約49.7日でwrapし、長時間動作で`ts_ms`の単調性が崩れる」ためであり、
    /// **`u32` で持たない。**`Envelope::ts_ms` へそのまま載せられる型に揃えてある。
    /// Pi linkの`crate::boot_session`（`#446` PR B）は、この値を`Envelope::ts_ms`へ載せている。
    ///
    /// [`Instant`] は単調性が型の契約であるため、この値も単調非減少である。
    /// `Duration::as_millis()` は `u128` を返すので飽和させるが、飽和しても
    /// 単調性は崩れない。到達には `u64::MAX` ミリ秒（約 5.8 億年）を要する。
    pub fn uptime_ms(&self) -> u64 {
        let elapsed = self.boot.elapsed().as_millis();
        u64::try_from(elapsed).unwrap_or(u64::MAX)
    }

    /// Heartbeat の連番を 1 つ進めて返す。
    pub fn next_heartbeat_seq(&mut self) -> u64 {
        self.heartbeat_seq = self.heartbeat_seq.saturating_add(1);
        self.heartbeat_seq
    }

    /// 期限超過を計上する。
    pub fn record_overrun(&mut self) {
        self.overrun_ticks = self.overrun_ticks.saturating_add(1);
    }

    /// Snapshot の serialize 失敗を計上する。
    pub fn record_snapshot_error(&mut self) {
        self.snapshot_errors = self.snapshot_errors.saturating_add(1);
    }

    /// 期限超過の回数。
    pub fn overrun_ticks(&self) -> u32 {
        self.overrun_ticks
    }

    /// Snapshot の serialize 失敗の回数。
    pub fn snapshot_errors(&self) -> u32 {
        self.snapshot_errors
    }

    /// Protocol の [`Status`] を組み立てる。
    ///
    /// **Counter schema を `status` へ使えることを、型で示すのがこの関数である。**
    /// [`ProtocolCounters`] をそのまま `Status::protocol` へ載せる。
    ///
    /// `firmware` は §4.6 の例に合わせて package version だけを入れる
    /// （`deskcat_protocol::limits::MAX_FIRMWARE_BYTES` = 64 に収まる）。
    /// 未初期化の subsystem は [`UNKNOWN_STATE`] で埋める。
    pub fn to_status(&self) -> Status {
        Status {
            firmware: env!("CARGO_PKG_VERSION").to_owned(),
            reset_reason: self.reset_reason.to_owned(),
            display: DisplayStatus {
                state: UNKNOWN_STATE.to_owned(),
                expression: UNKNOWN_STATE.to_owned(),
            },
            servo: ServoStatus {
                state: UNKNOWN_STATE.to_owned(),
            },
            sensors: SensorStatus {
                touch: UNKNOWN_STATE.to_owned(),
                acceleration: UNKNOWN_STATE.to_owned(),
                environment: UNKNOWN_STATE.to_owned(),
            },
            protocol: self.counters,
        }
    }
}
