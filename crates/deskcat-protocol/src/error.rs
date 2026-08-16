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

    /// 全variantの一覧。
    ///
    /// `#[non_exhaustive]`は**同一crate内の`match`には効かない**が、crate外には効く。
    /// したがって`tests/`のようなdownstream crateで`match`を書いても、wildcard armが
    /// 必要になりvariant追加時のcompile errorが起きない。網羅性を要求するtestは、
    /// 自前の`match`ではなくこの配列を走査する。
    ///
    /// **この配列自体の網羅性は、下の`const _`がcompilerに守らせる。**
    pub const ALL: [Self; 11] = [
        Self::UnsupportedVersion,
        Self::UnknownType,
        Self::InvalidEnvelope,
        Self::InvalidPayload,
        Self::OutOfRange,
        Self::LineTooLong,
        Self::Busy,
        Self::HardwareUnavailable,
        Self::DuplicateExpired,
        Self::RateLimited,
        Self::StaleSession,
    ];

    /// このcodeを計上する`status.payload.protocol`のfield名を返す。
    ///
    /// §4.6のcounter対応表に従う。**現在は全codeがcounterを持つ**が、
    /// 戻り値は`Option`のままにする。将来「counterではなく別の方法で観測する」codeを
    /// 足す余地を残すためであり、そのときは§4.6へ観測方法を明記する。
    ///
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
            Self::HardwareUnavailable => Some("hardware_unavailable"),
            Self::DuplicateExpired => Some("duplicate_expired"),
        }
    }
}

/// [`ErrorCode::ALL`]が全variantを1回ずつ含むことを、compile時に検査する。
///
/// `index_of`の`match`は**このcrate内**にあるため網羅性checkが効く。variantを足すと
/// ここが非網羅になりcompileできない。足したvariantを`ALL`へ入れ忘れれば、
/// 配列長か`assert!`が合わずcompileできない。**testでは無くcompileで止める。**
const _: () = {
    const fn index_of(code: ErrorCode) -> usize {
        match code {
            ErrorCode::UnsupportedVersion => 0,
            ErrorCode::UnknownType => 1,
            ErrorCode::InvalidEnvelope => 2,
            ErrorCode::InvalidPayload => 3,
            ErrorCode::OutOfRange => 4,
            ErrorCode::LineTooLong => 5,
            ErrorCode::Busy => 6,
            ErrorCode::HardwareUnavailable => 7,
            ErrorCode::DuplicateExpired => 8,
            ErrorCode::RateLimited => 9,
            ErrorCode::StaleSession => 10,
        }
    }

    let mut i = 0;
    while i < ErrorCode::ALL.len() {
        assert!(index_of(ErrorCode::ALL[i]) == i, "ErrorCode::ALL is stale");
        i += 1;
    }
};

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
