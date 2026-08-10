//! Protocolの上限値。
//!
//! ここに置く値は、`docs/protocol/esp32-pi-protocol.md`が`Candidate`または`TBD`としている
//! 段階の暫定値である。実測で確定するまで「検証済み」として扱わない。
//! string上限は勘で置かず、[`MAX_LINE_BYTES`]から導出できることを`tests/limits.rs`が検査する。

/// 実装が受理するwire protocolのmajor version（`v`）。
///
/// `docs/protocol/esp32-pi-protocol.md` §11。schemaがDraftである間、この値が一致しても
/// wire互換を意味しない。互換の根拠はconformance fixtureの一致である。
pub const PROTOCOL_VERSION: u16 = 1;

/// 改行を含む、encode済み1 lineの最大byte数。
///
/// §2の候補値であり、`PROTO-TBD-002`で確定する。worst-caseのpayloadとmemory testを
/// 経るまで確定値として扱わない。
pub const MAX_LINE_BYTES: usize = 1024;

/// `boot.firmware`と`status.firmware`のbyte上限。
pub const MAX_FIRMWARE_BYTES: usize = 64;

/// `boot.board`のbyte上限。
pub const MAX_BOARD_BYTES: usize = 32;

/// `boot.reset_reason`と`status.reset_reason`のbyte上限。
pub const MAX_RESET_REASON_BYTES: usize = 32;

/// `hello.host`のbyte上限。
pub const MAX_HOST_BYTES: usize = 64;

/// `hello.version`のbyte上限。
pub const MAX_VERSION_BYTES: usize = 64;

/// `ack.detail`のbyte上限。
///
/// §6は「短く上限を設けた`detail`」とだけ定めており、値は`PROTO-TBD-006`で確定する。
pub const MAX_DETAIL_BYTES: usize = 128;

/// `status`が持つstate名とexpression名のbyte上限。
///
/// 列挙値そのものは`PROTO-TBD-006`（最終status field）と`PROTO-TBD-008`（motion名）で
/// 確定するため、ここでは長さだけを縛る。
pub const MAX_STATE_NAME_BYTES: usize = 32;
