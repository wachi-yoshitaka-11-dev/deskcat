//! ESP32 peer sessionの状態（`boot`のsession遷移、duplicate履歴、`stale_session`判定）。
//!
//! [`crate::session::Session`]はbyte<->frameの往復とPi自身の送信sessionを持つが、
//! **相手（ESP32）のsession遷移、duplicate履歴、`stale_session`判定は持たない**
//! （`session.rs`のmodule doc参照）。この型がそれを持つ。
//!
//! 仕様の正本は`docs/protocol/esp32-pi-protocol.md`の§3.1・§4.1・§5.1・§6・§8。
//! 単位時間あたりの受理上限、session遷移budget、cooldown（`PROTO-TBD-012`）、
//! retired session保持件数と`hello`／`boot`の最大retry回数（`PROTO-TBD-011`、
//! `PROTO-TBD-017`）は未確定であり、**ここでは実装しない。**この型が扱うのは、
//! それらのbudgetに依存しない部分——現在session／retired sessionの判定、`boot`の
//! duplicate履歴、Pi自身が送った要求への応答の相関——だけである。
//!
//! `PeerSession`はbyte列やtransportを持たない。呼び出し側が
//! [`crate::session::Session::pump_read`]で復元した[`Frame`]をここへ渡し、
//! 返ってきた返信を[`crate::session::Session::send`]で送る。

use std::collections::{HashMap, VecDeque};

use deskcat_protocol::{Ack, AckStatus, Boot, ErrorCode, Frame, Message, Status};

/// retired session集合の上限。**暫定値であり、確定値ではない。**
///
/// 正本は`PROTO-TBD-011`（保持件数は「保持期間内に起こりうる最大の session 遷移回数」
/// から導出する、と仕様§3.1が定める）。この値は一次資料を持たず、実装を進めるための
/// 仮の上限である。確定したら置き換える。
const RETIRED_CAPACITY: usize = 4;

/// `boot`のduplicate履歴の上限。**暫定値であり、確定値ではない。**
///
/// 正本は`PROTO-TBD-011`・`PROTO-TBD-017`。[`RETIRED_CAPACITY`]と同じく実装都合の
/// 仮の値である。
const BOOT_HISTORY_CAPACITY: usize = 8;

/// Piが送った、応答待ちの要求。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutstandingKind {
    /// `ping`（§5.7）。
    Ping,
    /// `get_status`（§5.6）。
    GetStatus,
}

/// `boot`を拒否した、またはPiの要求と相関しないmessageを受けた理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PeerRejection {
    /// 現在承認していない`sid`（retiredまたは未知）からのmessage（§7、§5.1優先順位1・4）。
    StaleSession,
    /// 保持履歴から失われたduplicateを安全に再実行できない（§7、`PROTO-TBD-005`）。
    ///
    /// 判定は近似である。**`(sid, id)`が現在sessionに属し、これまで処理した最大`id`
    /// 以下であるにもかかわらず履歴に残っていない場合**に、履歴からevictされたと
    /// みなす。保持件数・保持期間そのものは`PROTO-TBD-011`が未確定であり、
    /// この型は固定容量[`BOOT_HISTORY_CAPACITY`]で近似するだけである。
    DuplicateExpired,
    /// 現在の`sid`に、未処理の新しい`id`を付けた`boot`（§5.1「現在sessionで未処理の
    /// session確立message」）。ESP32 processの再起動には新しい`sid`が必須であり、
    /// 同じ`sid`のまま新しい`id`を名乗ることは想定されない。
    InvalidPayload,
    /// Piが送っていない要求への相関ACK、または`reply_sid`／envelopeの`sid`が
    /// 現在のsessionと一致しないACK（§6）。
    UnmatchedAck,
}

impl PeerRejection {
    /// 相手へ返す、または計上する[`ErrorCode`]。
    #[must_use]
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::StaleSession => ErrorCode::StaleSession,
            Self::DuplicateExpired => ErrorCode::DuplicateExpired,
            Self::InvalidPayload | Self::UnmatchedAck => ErrorCode::InvalidPayload,
        }
    }
}

/// `boot`を処理した結果。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BootOutcome {
    /// 新しいESP32 sessionを確立した。旧sessionのduplicate履歴を破棄した。
    Established {
        /// 新しいESP32 `sid`。
        sid: u32,
        /// 確立に使った`boot`のpayload。
        boot: Boot,
    },
    /// 現在sessionの`boot`再送（duplicate）。session状態は変えていない。
    Replayed,
    /// 拒否した。session状態は変えていない。
    Rejected(PeerRejection),
}

/// `boot`を処理した結果と、相手へ返すACK。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootHandled {
    /// 何が起きたか。
    pub outcome: BootOutcome,
    /// 相手へ返すACK。`boot`は`(sid, id)`を送信側が生成するため、この型が
    /// 受け取る時点で常に復元できており、応答を構成できないケースは無い（§4.1）。
    pub reply: Message,
}

/// Piが送った要求への相関を取れた`ack`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelatedAck {
    /// 対応する要求の種類。
    pub request: OutstandingKind,
    /// 届いたACK。
    pub ack: Ack,
}

/// `boot`のduplicate履歴。
#[derive(Debug, Default)]
struct BootHistory {
    /// `id -> 返したACK`。挿入順を`order`で追い、境界を超えたら最古を捨てる。
    entries: HashMap<u32, Ack>,
    order: VecDeque<u32>,
    /// これまでに処理した最大`id`。履歴から消えていても「処理済みだった」と
    /// 判定するために使う（`duplicate_expired`の根拠）。
    highest_processed: Option<u32>,
}

impl BootHistory {
    fn get(&self, id: u32) -> Option<&Ack> {
        self.entries.get(&id)
    }

    fn was_processed_but_evicted(&self, id: u32) -> bool {
        !self.entries.contains_key(&id) && self.highest_processed.is_some_and(|h| id <= h)
    }

    fn insert(&mut self, id: u32, ack: Ack) {
        if !self.entries.contains_key(&id) {
            self.order.push_back(id);
            while self.order.len() > BOOT_HISTORY_CAPACITY {
                if let Some(oldest) = self.order.pop_front() {
                    self.entries.remove(&oldest);
                }
            }
        }
        self.entries.insert(id, ack);
        self.highest_processed = Some(self.highest_processed.map_or(id, |h| h.max(id)));
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.highest_processed = None;
    }
}

/// ESP32 peer sessionの状態。
#[derive(Debug)]
pub struct PeerSession {
    /// 現在承認しているESP32 `sid`。最初の`boot`を受けるまでは`None`。
    esp32_sid: Option<u32>,
    /// 直前まで有効だった`sid`の上限付き集合（§3.1、§5.1）。
    retired: VecDeque<u32>,
    boot_history: BootHistory,
    /// Piが送った、応答待ちの要求。`session.send`で割り当てた`id`をkeyにする。
    outstanding: HashMap<u32, OutstandingKind>,
}

impl Default for PeerSession {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerSession {
    /// 未確立のsessionを作る。
    #[must_use]
    pub fn new() -> Self {
        Self {
            esp32_sid: None,
            retired: VecDeque::with_capacity(RETIRED_CAPACITY),
            boot_history: BootHistory::default(),
            outstanding: HashMap::new(),
        }
    }

    /// 現在承認しているESP32 `sid`。未確立なら`None`。
    #[must_use]
    pub const fn esp32_sid(&self) -> Option<u32> {
        self.esp32_sid
    }

    /// Piが送った要求を記録する。`session.send`が返した`id`をそのまま渡す。
    ///
    /// 呼び出し側が`session.send(Message::Ping, ts)`などで送信した直後に呼ぶ。
    /// 記録していない`id`へのACKは[`PeerRejection::UnmatchedAck`]になる。
    pub fn note_sent(&mut self, id: u32, kind: OutstandingKind) {
        self.outstanding.insert(id, kind);
    }

    /// 応答待ちの要求を1件取り下げる。
    ///
    /// ACKが返らないまま放置すると`outstanding`が際限なく増える。**timeoutの判定
    /// そのものはこの型が持たない**（間隔は`PROTO-TBD-017`等が未確定であり、
    /// この型は値を推測しない）。呼び出し側がtimeoutを判定した`id`をここへ渡して
    /// 取り下げる。記録していない`id`なら`None`を返す。
    pub fn forget_sent(&mut self, id: u32) -> Option<OutstandingKind> {
        self.outstanding.remove(&id)
    }

    /// 応答待ちの要求の件数。**個々の`id`は特定しない。**呼び出し側が、放置された
    /// 要求が際限なく積み上がっていないかを観測するために使う。特定の`id`を
    /// timeout扱いにするかどうかの判断は、呼び出し側が別途持つ情報（送信時刻など）に基づく。
    #[must_use]
    pub fn outstanding_len(&self) -> usize {
        self.outstanding.len()
    }

    fn is_retired(&self, sid: u32) -> bool {
        self.retired.contains(&sid)
    }

    fn retire_current(&mut self) {
        if let Some(old) = self.esp32_sid
            && !self.retired.contains(&old)
        {
            self.retired.push_back(old);
            while self.retired.len() > RETIRED_CAPACITY {
                self.retired.pop_front();
            }
        }
    }

    /// 拒否ACKを組み立てる。`(sid, id)`は送信側が生成した`boot`から復元できている
    /// ため、常に構成できる（§4.1）。
    fn rejection_ack(reply_sid: u32, reply_to: u32, rejection: PeerRejection) -> Message {
        Message::Ack(Ack {
            reply_sid,
            reply_to,
            status: AckStatus::Rejected,
            code: Some(rejection.code()),
            detail: None,
        })
    }

    /// `boot`を処理する（§4.1、§5.1、§8手順8〜10）。
    ///
    /// 遷移候補のsession遷移budget・cooldown（§5.1、`PROTO-TBD-012`）は判定しない。
    /// 呼び出し側がその上限を別途課す場合は、この呼び出し自体を抑制すること
    /// （session状態はこの呼び出しで変わるため、抑制せずに呼ぶと無条件で遷移する）。
    pub fn handle_boot(&mut self, sid: u32, id: u32, boot: Boot) -> BootHandled {
        if self.is_retired(sid) {
            let rejection = PeerRejection::StaleSession;
            return BootHandled {
                reply: Self::rejection_ack(sid, id, rejection),
                outcome: BootOutcome::Rejected(rejection),
            };
        }

        if self.esp32_sid == Some(sid) {
            if let Some(ack) = self.boot_history.get(id) {
                return BootHandled {
                    reply: Message::Ack(ack.clone()),
                    outcome: BootOutcome::Replayed,
                };
            }
            let rejection = if self.boot_history.was_processed_but_evicted(id) {
                PeerRejection::DuplicateExpired
            } else {
                // 現在のsidで未処理の新しいid。ESP32の再起動には新しいsidが必須であり、
                // 同じsidのまま新しいidを名乗ることは想定されない（§5.1）。
                PeerRejection::InvalidPayload
            };
            return BootHandled {
                reply: Self::rejection_ack(sid, id, rejection),
                outcome: BootOutcome::Rejected(rejection),
            };
        }

        // 未知のsid: hello／bootだけが遷移候補になれる（§5.1優先順位3）。
        self.retire_current();
        self.boot_history.clear();
        self.esp32_sid = Some(sid);
        // 新sessionでは、Piが持っていたESP32宛ての未完了要求はtimeout扱いにする
        // （§6「session切り替え後は旧sessionの未ACK commandをtimeout扱いにする」）。
        self.outstanding.clear();

        let ack = Ack {
            reply_sid: sid,
            reply_to: id,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        };
        self.boot_history.insert(id, ack.clone());
        BootHandled {
            reply: Message::Ack(ack),
            outcome: BootOutcome::Established { sid, boot },
        }
    }

    /// `ack`を、Piが送った要求と相関させる（§6）。
    ///
    /// 検査する条件は§6のとおり: envelopeの`sid`が現在のESP32 session、
    /// `reply_sid`がPi自身の現在session、`reply_to`が未完了の要求である。
    ///
    /// # Errors
    ///
    /// envelopeの`sid`が現在のESP32 sessionと異なる場合、`reply_sid`がPi自身の
    /// 現在sessionと異なる場合、または`reply_to`が記録した未完了の要求でない場合に
    /// [`PeerRejection`]を返す。
    pub fn correlate_ack(
        &mut self,
        envelope_sid: u32,
        our_sid: u32,
        ack: Ack,
    ) -> Result<CorrelatedAck, PeerRejection> {
        if self.esp32_sid != Some(envelope_sid) {
            return Err(if self.is_retired(envelope_sid) {
                PeerRejection::StaleSession
            } else {
                PeerRejection::UnmatchedAck
            });
        }
        if ack.reply_sid != our_sid {
            return Err(PeerRejection::UnmatchedAck);
        }
        let Some(request) = self.outstanding.remove(&ack.reply_to) else {
            return Err(PeerRejection::UnmatchedAck);
        };
        Ok(CorrelatedAck { request, ack })
    }

    /// `status`を検査する（§5.1優先順位・§8）。ESP32からのeventであり、
    /// Piは応答を返さない。
    ///
    /// # Errors
    ///
    /// envelopeの`sid`が現在のESP32 sessionと異なる場合は[`PeerRejection::StaleSession`]を返す。
    pub fn accept_status(
        &self,
        envelope_sid: u32,
        status: Status,
    ) -> Result<Status, PeerRejection> {
        if self.esp32_sid == Some(envelope_sid) {
            Ok(status)
        } else {
            Err(PeerRejection::StaleSession)
        }
    }

    /// 復元した[`Frame`]の送信元`sid`が、現在のESP32 sessionと一致するか。
    ///
    /// `boot`／`ack`／`status`以外の、このcrateがまだ扱わないtypeを受けたときに
    /// 呼び出し側が使う。一致しなければ`stale_session`として計数してよい
    /// （§5.1優先順位4: `hello`／`boot`以外の未知`sid`は`stale_session`）。
    #[must_use]
    pub fn is_current_session(&self, frame: &Frame) -> bool {
        self.esp32_sid == Some(frame.envelope.sid)
    }
}

#[cfg(test)]
mod tests {
    use deskcat_protocol::{DisplayStatus, ProtocolCounters, SensorStatus, ServoStatus};

    use super::{
        Ack, AckStatus, BOOT_HISTORY_CAPACITY, Boot, BootHistory, BootOutcome, ErrorCode,
        OutstandingKind, PeerRejection, PeerSession, Status,
    };

    fn boot() -> Boot {
        Boot {
            firmware: "0.1.0".to_owned(),
            board: "esp32".to_owned(),
            reset_reason: "power_on".to_owned(),
        }
    }

    /// `BootHistory`単体: 容量を超えると最古のentryがevictされ、
    /// evictされたidは「処理済みだが失われた」と判定される。
    ///
    /// `duplicate_expired`はprotocolが定める動作である。`docs/protocol/esp32-pi-protocol.md`は
    /// duplicate履歴のoverflow時に最古のentryをevictし、evict済みへの再送を
    /// `duplicate_expired`で拒否すると定めており、この型はその動作を実装する。
    ///
    /// **これは弱点として書いておく。**公開APIの`PeerSession::handle_boot`では、正規の
    /// `boot`は1 sessionにつき1つの`id`しか処理しない（同じ`sid`で未処理の新しい`id`は
    /// `invalid_payload`で拒否するため）。したがって**この容量超過は、正規のtrafficだけでは
    /// `boot`単独では到達しない。**公開APIを通した`tests/simulator.rs`の受け入れ条件testでは
    /// 検証できないため、ここで内部構造を直接叩いて境界そのものを検査する。
    #[test]
    fn boot_history_reports_duplicate_expired_for_evicted_entries() {
        let mut history = BootHistory::default();
        let ack_for = |id: u32| Ack {
            reply_sid: 1,
            reply_to: id,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        };

        for id in 1..=u32::try_from(BOOT_HISTORY_CAPACITY).expect("小さい定数である") {
            history.insert(id, ack_for(id));
        }
        assert_eq!(history.get(1), Some(&ack_for(1)), "容量内はまだ残っている");

        // 容量を1つ超えて挿入すると、最古（id=1）がevictされる。
        let next = u32::try_from(BOOT_HISTORY_CAPACITY).expect("小さい定数である") + 1;
        history.insert(next, ack_for(next));

        assert_eq!(history.get(1), None, "最古のentryがevictされている");
        assert!(
            history.was_processed_but_evicted(1),
            "処理済みだが履歴からは失われている"
        );
        assert!(
            !history.was_processed_but_evicted(next + 1),
            "一度も処理していないidはduplicate_expiredではない"
        );
    }

    #[test]
    fn an_unknown_sid_establishes_a_new_session_and_returns_an_ok_ack() {
        let mut peer = PeerSession::new();
        assert_eq!(peer.esp32_sid(), None);

        let handled = peer.handle_boot(41_207, 1, boot());

        assert!(matches!(
            handled.outcome,
            BootOutcome::Established { sid: 41_207, .. }
        ));
        assert_eq!(peer.esp32_sid(), Some(41_207));
        match handled.reply {
            super::Message::Ack(ack) => {
                assert_eq!(ack.reply_sid, 41_207);
                assert_eq!(ack.reply_to, 1);
                assert_eq!(ack.status, AckStatus::Ok);
            }
            other => panic!("Ackを期待した: {other:?}"),
        }
    }

    #[test]
    fn a_retried_boot_with_the_same_sid_and_id_replays_the_stored_ack() {
        let mut peer = PeerSession::new();
        let first = peer.handle_boot(41_207, 1, boot());
        let retry = peer.handle_boot(41_207, 1, boot());

        assert_eq!(retry.outcome, BootOutcome::Replayed);
        assert_eq!(retry.reply, first.reply, "同じACKを再送する");
    }

    #[test]
    fn a_boot_from_a_retired_sid_is_rejected_as_stale_session() {
        let mut peer = PeerSession::new();
        let _ = peer.handle_boot(41_207, 1, boot());
        // 別のsidへ遷移させ、41_207をretiredへ移す。
        let _ = peer.handle_boot(90_000, 1, boot());

        let rejected = peer.handle_boot(41_207, 2, boot());
        assert_eq!(
            rejected.outcome,
            BootOutcome::Rejected(PeerRejection::StaleSession)
        );
        match rejected.reply {
            super::Message::Ack(ack) => {
                assert_eq!(ack.status, AckStatus::Rejected);
                assert_eq!(ack.code, Some(ErrorCode::StaleSession));
            }
            other => panic!("Ackを期待した: {other:?}"),
        }
    }

    #[test]
    fn a_new_id_under_the_current_sid_is_rejected_as_invalid_payload() {
        let mut peer = PeerSession::new();
        let _ = peer.handle_boot(41_207, 1, boot());

        let rejected = peer.handle_boot(41_207, 2, boot());
        assert_eq!(
            rejected.outcome,
            BootOutcome::Rejected(PeerRejection::InvalidPayload)
        );
        assert_eq!(
            peer.esp32_sid(),
            Some(41_207),
            "拒否はsession stateを変更しない"
        );
    }

    #[test]
    fn correlate_ack_matches_a_pending_request() {
        let mut peer = PeerSession::new();
        let _ = peer.handle_boot(41_207, 1, boot());
        peer.note_sent(7, OutstandingKind::Ping);

        let ack = Ack {
            reply_sid: 90_312,
            reply_to: 7,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        };
        let correlated = peer
            .correlate_ack(41_207, 90_312, ack)
            .expect("相関が取れる");
        assert_eq!(correlated.request, OutstandingKind::Ping);

        // 消費済み。同じreply_toを二度使えない。
        let ack_again = Ack {
            reply_sid: 90_312,
            reply_to: 7,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        };
        assert_eq!(
            peer.correlate_ack(41_207, 90_312, ack_again),
            Err(PeerRejection::UnmatchedAck)
        );
    }

    #[test]
    fn correlate_ack_rejects_a_sid_that_is_not_the_current_esp32_session() {
        let mut peer = PeerSession::new();
        let _ = peer.handle_boot(41_207, 1, boot());
        peer.note_sent(7, OutstandingKind::GetStatus);

        let ack = Ack {
            reply_sid: 90_312,
            reply_to: 7,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        };
        assert_eq!(
            peer.correlate_ack(99_999, 90_312, ack),
            Err(PeerRejection::UnmatchedAck),
            "現session以外からの未知sidはUnmatchedAckとして計数する"
        );
    }

    #[test]
    fn correlate_ack_rejects_a_mismatched_reply_sid() {
        let mut peer = PeerSession::new();
        let _ = peer.handle_boot(41_207, 1, boot());
        peer.note_sent(7, OutstandingKind::Ping);

        let ack = Ack {
            reply_sid: 12_345, // Piの現在のsidと一致しない
            reply_to: 7,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        };
        assert_eq!(
            peer.correlate_ack(41_207, 90_312, ack),
            Err(PeerRejection::UnmatchedAck)
        );
    }

    #[test]
    fn accept_status_requires_the_current_esp32_session() {
        let mut peer = PeerSession::new();
        let _ = peer.handle_boot(41_207, 1, boot());

        let status = sample_status();
        assert!(peer.accept_status(41_207, status.clone()).is_ok());
        assert_eq!(
            peer.accept_status(99_999, status),
            Err(PeerRejection::StaleSession)
        );
    }

    fn sample_status() -> Status {
        Status {
            firmware: "0.1.0".to_owned(),
            reset_reason: "power_on".to_owned(),
            display: DisplayStatus {
                state: "ready".to_owned(),
                expression: "neutral".to_owned(),
            },
            servo: ServoStatus {
                state: "disabled".to_owned(),
            },
            sensors: SensorStatus {
                touch: "unknown".to_owned(),
                acceleration: "unknown".to_owned(),
                environment: "unknown".to_owned(),
            },
            protocol: ProtocolCounters::default(),
        }
    }
}
