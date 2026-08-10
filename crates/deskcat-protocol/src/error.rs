//! Error code（§7）とdecode失敗の分類。

use core::fmt;

use serde::{Deserialize, Serialize};

/// `docs/protocol/esp32-pi-protocol.md` §7が定めるmachine-readableなerror code。
///
/// 新しいcodeの追加で下流の`match`が壊れないよう`#[non_exhaustive]`とする。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorCode {
    /// Protocol major versionに未対応。
    UnsupportedVersion,
    /// Message typeが未知。
    UnknownType,
    /// 必須fieldまたはtop-level typeが不正。
    InvalidEnvelope,
    /// Type固有payloadが不正。
    InvalidPayload,
    /// 上限のある値が許容範囲外。
    OutOfRange,
    /// 受信lineが設定した最大長を超過。
    LineTooLong,
    /// 上限のあるresourceがcommandを受け入れられない。
    Busy,
    /// 必要なhardwareの準備が未完了。
    HardwareUnavailable,
    /// 保持履歴から失われたduplicateを安全に再実行できない。
    DuplicateExpired,
    /// 単位時間あたりの受理上限、session遷移budget／cooldownを超過。
    RateLimited,
    /// 現在のsessionとして承認されていない`sid`のmessageを受信した。
    StaleSession,
}

impl ErrorCode {
    /// wire上の文字列表現を返す。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "unsupported_version",
            Self::UnknownType => "unknown_type",
            Self::InvalidEnvelope => "invalid_envelope",
            Self::InvalidPayload => "invalid_payload",
            Self::OutOfRange => "out_of_range",
            Self::LineTooLong => "line_too_long",
            Self::Busy => "busy",
            Self::HardwareUnavailable => "hardware_unavailable",
            Self::DuplicateExpired => "duplicate_expired",
            Self::RateLimited => "rate_limited",
            Self::StaleSession => "stale_session",
        }
    }

    /// このcodeを計上する`status.payload.protocol`のfield名を返す。
    ///
    /// 対応するcounterが定義されていないcodeでは`None`を返す。§4.6のcounter対応表に従う。
    /// `invalid_envelope`が`parse_errors`へ向くのは、§4.6が「UTF-8／JSON／envelope不正」を
    /// まとめて`parse_errors`としているためである。
    #[must_use]
    pub const fn counter_field(self) -> Option<&'static str> {
        match self {
            Self::InvalidEnvelope => Some("parse_errors"),
            Self::InvalidPayload => Some("invalid_payloads"),
            Self::UnsupportedVersion => Some("unsupported_versions"),
            Self::LineTooLong => Some("oversize_lines"),
            Self::UnknownType => Some("unknown_types"),
            Self::RateLimited => Some("rate_limited"),
            Self::Busy => Some("busy"),
            Self::OutOfRange => Some("out_of_range"),
            Self::StaleSession => Some("stale_sessions"),
            Self::HardwareUnavailable | Self::DuplicateExpired => None,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 1 lineのdecodeに失敗した理由。
///
/// `code`は相手へ返す分類、`detail`は診断用の説明である。`detail`をwireへそのまま
/// 載せない。§6の`ack.detail`は別に上限を持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    code: ErrorCode,
    detail: String,
}

impl DecodeError {
    /// 分類と説明からerrorを作る。
    #[must_use]
    pub fn new(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    /// 相手へ返すerror codeを返す。
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    /// 診断用の説明を返す。
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// このerrorを計上するcounterのfield名を返す。
    #[must_use]
    pub const fn counter_field(&self) -> Option<&'static str> {
        self.code.counter_field()
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl core::error::Error for DecodeError {}
