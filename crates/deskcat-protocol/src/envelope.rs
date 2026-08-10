//! Envelope（§3）と、envelopeとmessageを組にしたframe。

use crate::message::Message;

/// すべてのmessageが持つenvelope field（§3）。
///
/// `type`と`payload`は[`Message`]が保持するため、ここには持たない。
///
/// integer widthは`PROTO-TBD-003`として保留されていたものを、conformance fixtureと
/// あわせて次のとおり確定する。宣言した幅に収まらない値はenvelopeとして復元できないため
/// [`crate::ErrorCode::InvalidEnvelope`]で拒否する。
///
/// | Field | 型 | 根拠 |
/// |---|---|---|
/// | `v` | `u16` | major versionは増加が遅く、幅を広げる意味がない |
/// | `sid` | `u32` | session ID |
/// | `id` | `u32` | session内の単調増加ID |
/// | `ts_ms` | `u64` | uptime ms。`u32`は約49.7日でwrapし、長時間動作で単調性が崩れる |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Envelope {
    /// Protocol major version。
    pub v: u16,
    /// 送信側のsession ID。起動ごとに新しい値を選ぶ。
    pub sid: u32,
    /// 同一session内で単調増加するmessage ID。
    pub id: u32,
    /// 送信側のuptime（milliseconds）。wall-clock timeではない。
    pub ts_ms: u64,
}

impl Envelope {
    /// message同一性を表す`(sid, id)`の組を返す。
    ///
    /// §9が「duplicate判定は必ず`(sid, id)`の組で行う。`id`だけで判定してはならない」と
    /// 定めているため、`id`単独を取り出すaccessorは用意しない。
    #[must_use]
    pub const fn identity(&self) -> (u32, u32) {
        (self.sid, self.id)
    }
}

/// 1 lineに対応する、envelopeとmessageの組。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Envelope field。
    pub envelope: Envelope,
    /// Type固有のmessage。
    pub message: Message,
}

impl Frame {
    /// envelopeとmessageからframeを作る。
    #[must_use]
    pub const fn new(envelope: Envelope, message: Message) -> Self {
        Self { envelope, message }
    }
}
