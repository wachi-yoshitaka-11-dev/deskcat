//! Health snapshot。
//!
//! 起動からの uptime、heartbeat の連番、および `crates/deskcat-protocol` の
//! [`ProtocolCounters`] を保持する。
//!
//! **新しい counter 型を作らない。**Protocol counter は同 crate の
//! [`ProtocolCounters`] をそのまま使う。各 field の意味の正本は Protocol §4.6 の
//! counter 対応表であり、ここへ再掲しない。
//!
//! **Protocol session は確立しない。**この firmware は serial link（#11）も
//! session state（#12）も持たないため、[`ProtocolCounters`] は**すべて 0 のままである**。
//! ここで示すのは「counter schema を `status` へ載せられる」ことであって、
//! 「counter が動いている」ことではない。

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
    /// 起動時の`boot` message（`crate::protocol`）のserializeに失敗した回数。
    /// **Protocol counterではない。**[`Self::snapshot_errors`]と原因が違うため
    /// 混ぜずに分けて数える。
    boot_serialize_errors: u32,
    /// Protocol counter。**すべて 0 のままである**（module doc 参照）。
    counters: ProtocolCounters,
    /// Machine-readable な reset reason。
    reset_reason: &'static str,
}

impl Health {
    /// 起動時点を起点として作る。
    pub fn new(reset_reason: &'static str) -> Self {
        Self {
            boot: Instant::now(),
            heartbeat_seq: 0,
            overrun_ticks: 0,
            snapshot_errors: 0,
            boot_serialize_errors: 0,
            counters: ProtocolCounters::default(),
            reset_reason,
        }
    }

    /// 起動からの経過時間（milliseconds）。
    ///
    /// 型は `u64` である。Protocol §3 が `ts_ms` に `u64` を採ったのは
    /// 「`u32`は約49.7日でwrapし、長時間動作で`ts_ms`の単調性が崩れる」ためであり、
    /// **`u32` で持たない。**`Envelope::ts_ms` へそのまま載せられる型に揃えてある
    /// （載せるのは serial link と session が入ってからである。#11／#12）。
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

    /// `boot` messageのserialize失敗を計上する。
    pub fn record_boot_serialize_error(&mut self) {
        self.boot_serialize_errors = self.boot_serialize_errors.saturating_add(1);
    }

    /// 期限超過の回数。
    pub fn overrun_ticks(&self) -> u32 {
        self.overrun_ticks
    }

    /// Snapshot の serialize 失敗の回数。
    pub fn snapshot_errors(&self) -> u32 {
        self.snapshot_errors
    }

    /// `boot` messageのserialize失敗の回数。
    pub fn boot_serialize_errors(&self) -> u32 {
        self.boot_serialize_errors
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
