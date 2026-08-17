//! Host側のserial session。
//!
//! wire仕様の正本は`docs/protocol/esp32-pi-protocol.md`である。message型、検証、
//! 上限付きline受信は[`deskcat_protocol`]が持ち、**このcrateは再実装しない。**
//!
//! # 範囲
//!
//! 含むもの:
//!
//! - port名とbaudをsecretではなく設定として扱う型（[`SerialConfig`]）
//! - byte列を運ぶ層の境界（[`Transport`]）とI/O errorの分類（[`IoDisposition`]）
//! - 上限のある送信queue（[`Outbox`]）。溢れは明示的にdropしてcounterへ計上する
//! - 送信側の`id`採番（[`IdAllocator`]）。仕様§3の`PROTO-TBD-003`をそのまま実装する
//! - 接続stateとcounter（[`Session`]）。partial I/O、切断の観測、
//!   再接続の上限とrate limitを含む
//!
//! 含まないもの:
//!
//! - **実serial deviceのopen。**VM上では実portを検証できず、device名も未確認である。
//!   `serialport`のようなcrateの選定は依存追加の規則に従って別途行う。
//!   **検証できない依存を先に足さない。**[`Transport`]を実装する形で後から載せる
//! - **domain動作。**感情、性格、行動判断、独り言はこのcrateに入らない
//! - **session遷移の確定、duplicate履歴、受理budget、遷移cooldown。**
//!   受信側のstateを要するため[Issue #12]の範囲である
//!
//! # 暫定値について
//!
//! 再接続の回数上限とbackoff、送信queueの容量は**設定parameterとして受け取る。**
//! [`ReconnectPolicy::provisional`]と[`SerialConfig::DEFAULT_OUTBOX_CAPACITY`]が
//! 返す値は**暫定であり、確定値ではない。**正本は`PROTO-TBD-012`と`PROTO-TBD-017`で、
//! いずれも負荷試験・reconnect試験待ちである。
//!
//! # 例
//!
//! ```
//! use deskcat_protocol::{Hello, HelloReason, Message};
//! use deskcat_serial::{ConnectionState, SerialConfig, Session};
//!
//! // port名とbaudは呼び出し側が渡す。既定値を持たせない。
//! let config = SerialConfig::new("/dev/example", 115_200);
//! let mut session = Session::new(config, 90_312);
//!
//! assert_eq!(session.state(), ConnectionState::Disconnected);
//! session.note_connected();
//!
//! let hello = Message::Hello(Hello {
//!     host: "deskcatd".to_owned(),
//!     version: "0.1.0".to_owned(),
//!     reason: HelloReason::Startup,
//! });
//! let id = session.send(hello, 40).expect("queueへ入る");
//! assert_eq!(id, 1);
//! ```
//!
//! [Issue #12]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12

pub mod config;
pub mod ids;
pub mod outbox;
pub mod session;
pub mod transport;

pub use config::{ReconnectPolicy, SerialConfig};
pub use ids::{IdAllocator, IdSpaceExhausted};
pub use outbox::{Enqueued, Outbox};
pub use session::{ConnectionState, Pump, SendError, Session, SessionCounters, StopReason};
pub use transport::{IoDisposition, Transport};
