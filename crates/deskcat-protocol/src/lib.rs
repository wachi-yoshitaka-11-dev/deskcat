//! DeskCatのESP32–Raspberry Pi wire protocolの型、検証、共有conformance fixture。
//!
//! wire仕様の正本は`docs/protocol/esp32-pi-protocol.md`である。このcrateは仕様を
//! 置き換えない。仕様がDraftである間、`v: 1`が一致してもwire互換は保証されず、
//! 互換の根拠は共有fixtureの一致である（§11、§12.1）。
//!
//! # 範囲
//!
//! 含むもの:
//!
//! - envelope（§3）とIssue #4が承認した最小message type（`boot`、`hello`、`ping`、
//!   `get_status`、`status`、`ack`）
//! - error code（§7）と、それを計上するcounterへの対応付け
//! - 1 lineのdecodeとencode、および検証順序
//! - 共有conformance fixture（`tests/fixtures/`）
//!
//! 含まないもの:
//!
//! - serial I/O、byte受信、分割lineの結合、invalid UTF-8の分類（Issue #10、#11）
//! - session state、duplicate履歴、受理budget、遷移cooldown（Issue #12）
//! - hardwareに依存する値と処理
//!
//! # 例
//!
//! ```
//! use deskcat_protocol::{Message, decode_line};
//!
//! let line = r#"{"v":1,"sid":90312,"id":907,"ts_ms":54000,"type":"ping","payload":{}}"#;
//! let frame = decode_line(line).expect("valid line");
//!
//! assert_eq!(frame.envelope.identity(), (90312, 907));
//! assert!(matches!(frame.message, Message::Ping));
//! ```
//!
//! 分類済みerrorで失敗する例:
//!
//! ```
//! use deskcat_protocol::{ErrorCode, decode_line};
//!
//! let line = r#"{"v":2,"sid":1,"id":1,"ts_ms":0,"type":"nope","payload":{}}"#;
//! let err = decode_line(line).expect_err("unsupported version");
//!
//! // `v`の判定はtypeの解決より先である（§5.1の手順2と3）。
//! assert_eq!(err.code(), ErrorCode::UnsupportedVersion);
//! assert_eq!(err.counter_field(), Some("unsupported_versions"));
//! ```

pub mod decode;
pub mod envelope;
pub mod error;
pub mod limits;
pub mod message;

pub use decode::{decode_line, encode_line};
pub use envelope::{Envelope, Frame};
pub use error::{DecodeError, ErrorCode};
pub use message::{
    Ack, AckStatus, Boot, DisplayStatus, Hello, HelloReason, Message, ProtocolCounters,
    SensorStatus, ServoStatus, Status,
};
