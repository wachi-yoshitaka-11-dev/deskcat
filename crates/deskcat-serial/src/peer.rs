//! ESP32 peer sessionの状態（`boot`のsession遷移、duplicate履歴、`stale_session`判定）。
//!
//! [`crate::session::Session`]はbyte<->frameの往復とPi自身の送信sessionを持つが、
//! **相手（ESP32）のsession遷移、duplicate履歴、`stale_session`判定は持たない**
//! （`session.rs`のmodule doc参照）。この型がそれを持つ。
//!
//! 仕様の正本は`docs/protocol/esp32-pi-protocol.md`の§3.1・§4.1・§4.6・§5.1・§5.6・§6・§8・§10.1。
//! 単位時間あたりの受理上限、session遷移budget、cooldown（`PROTO-TBD-012`）、
//! retired session保持件数と`hello`／`boot`の最大retry回数（`PROTO-TBD-011`、
//! `PROTO-TBD-017`）は未確定であり、**ここでは実装しない。**この型が扱うのは、
//! それらのbudgetに依存しない部分——現在session／retired sessionの判定、`boot`の
//! duplicate履歴、Pi自身が送った要求への応答の相関——だけである。
//! duplicate履歴の保持件数と保持期間（`PROTO-TBD-005`）も未確定であり、
//! [`PeerSession::new`]が[`DuplicatePolicy`]として呼び出し側から受け取る。
//!
//! `PeerSession`はbyte列やtransportを持たない。呼び出し側が
//! [`crate::session::Session::pump_read`]で復元した[`Frame`]をここへ渡し、
//! 返ってきた返信を[`crate::session::Session::send`]で送る。

use core::time::Duration;
use std::collections::{HashMap, VecDeque};

use deskcat_protocol::{Ack, AckStatus, Boot, ErrorCode, Frame, Message, Status};

use crate::config::{DuplicatePolicy, RetryPolicy};
use crate::duplicate::{DuplicateHistory, Lookup};

/// retired session集合の上限。**暫定値であり、確定値ではない。**
///
/// 正本は`PROTO-TBD-011`。保持件数の下限は
/// `N_transition × ceil(T_retention / T_window)`件であり、`N_transition`と`T_window`は
/// `PROTO-TBD-012`、`T_retention`は`PROTO-TBD-011`で、**どれも未確定である。**
/// **したがってこの値は式の解ではない。**一次資料を持たない仮の上限である。
///
/// **上限に達しても、retired sessionを追い出さない。**仕様§3.1が定める。
///
/// > **保持件数の上限によって、保持期間の満了前にretired sessionを追い出してはならない。**
/// > 追い出すと、その`sid`は「未知」に戻る。遅れて届いた`hello`／`boot`が遷移候補として
/// > 受理され、現在のduplicate履歴を破棄して実行中motionを停止する。
/// > `stale_session`で拒否されるはずの経路が、逆に副作用を起こす経路になる。
///
/// **代わりに、追い出しを要する遷移そのものを拒否する**（[`PeerRejection::RetentionFull`]）。
/// 値が仮であることと、追い出さないことは別の話である。**仮の値のまま追い出すと、
/// 上の副作用が仮の件数で起きる。**確定したら値だけを置き換える。
const RETIRED_CAPACITY: usize = 4;

/// Piが送った、応答待ちの要求。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutstandingKind {
    /// `ping`（§5.7）。
    Ping,
    /// `get_status`（§5.6）。
    GetStatus,
}

impl OutstandingKind {
    /// `message`が追跡対象なら、その種別を返す。
    ///
    /// **呼び出し側が`kind`を別途指定する経路を作らない。**`kind`と`message`を
    /// 別々の引数で受け取ると、両者が食い違う値を渡せてしまう
    /// （例えば`kind: Ping`なのに`message: Message::Hello(..)`）。この場合
    /// [`CorrelatedAck::request`]や[`OutstandingAction::Retry`]の種別が、
    /// 実際にwireへ送った`message`と食い違ったまま報告される。`message`から
    /// `kind`を導くことで、この食い違いをそもそも作れないようにする。
    fn classify(message: &Message) -> Option<Self> {
        match message {
            Message::Ping => Some(Self::Ping),
            Message::GetStatus => Some(Self::GetStatus),
            _ => None,
        }
    }
}

/// [`PeerSession::poll_outstanding`]が応答待ちの要求へ下す判断。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutstandingAction {
    /// ACK timeoutを超えたが、再送予算がまだ残っている。同じ`id`で再送してよい。
    /// [`Message`]は送信時にそのまま保持しておいたものであり、**種別から
    /// 組み直したものではない。**
    Retry(OutstandingKind, Message),
    /// 再送予算を使い切った。要求を取り下げた。
    GaveUp(OutstandingKind),
}

/// Piが送った、応答待ちの要求。送信したmessage自体、送信時刻、これまでの
/// 再送回数を持つ。
///
/// **種別（[`OutstandingKind`]）だけでなく、送信した[`Message`]そのものを保持する。**
/// `ping`／`get_status`はpayloadを持たないため、種別だけからでも`Message::Ping`
/// ／`Message::GetStatus`を組み直せてしまう。だが将来`OutstandingKind`へ
/// payloadを持つ種別（例えばmotion command）が加わったとき、種別からの復元は
/// 「元と違うpayloadで静かに再送する」という、気づきにくい壊れ方をする。
/// 実際に送ったmessageを保持しておけば、この種のkindが増えても再送は
/// 常に元のmessageのままである。
#[derive(Debug, Clone)]
struct OutstandingEntry {
    kind: OutstandingKind,
    message: Message,
    /// 最後に送信した（または再送した）時刻。
    sent_at_ms: u64,
    /// これまでの再送回数。最初の送信では0。
    retries: u32,
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
    /// 以下であるにもかかわらず履歴に残っていない、または保持期間を過ぎている場合**に、
    /// 履歴から失われたとみなす（`crate::duplicate`のmodule doc参照）。保持件数と保持期間は
    /// `PROTO-TBD-005`が未確定であり、呼び出し側が渡した[`DuplicatePolicy`]に従う。
    DuplicateExpired,
    /// 現在の`sid`に、未処理の新しい`id`を付けた`boot`（§5.1「現在sessionで未処理の
    /// session確立message」）。ESP32 processの再起動には新しい`sid`が必須であり、
    /// 同じ`sid`のまま新しい`id`を名乗ることは想定されない。
    InvalidPayload,
    /// Piが送っていない要求への相関ACK、または`reply_sid`／envelopeの`sid`が
    /// 現在のsessionと一致しないACK（§6）。
    UnmatchedAck,
    /// retired session保持が満杯で、遷移すると保持期間の満了前に追い出すことになる
    /// （§3.1、`PROTO-TBD-011`）。
    ///
    /// **既存の4つに収まらないため足した。**`StaleSession`は現在承認していない`sid`から
    /// のmessage、`DuplicateExpired`は履歴から失われたduplicate、`InvalidPayload`と
    /// `UnmatchedAck`は形の話であり、**いずれも「遷移そのものを受けられない」ではない。**
    ///
    /// codeは`rate_limited`である。仕様§3.1が「遷移速度の上限も併せて定め、上限を
    /// 超える遷移は`rate_limited`で拒否する」と定めており、**保持が満杯になるのは
    /// 保持期間内の遷移が想定を超えたときである。**
    RetentionFull,
}

impl PeerRejection {
    /// 相手へ返す、または計上する[`ErrorCode`]。
    #[must_use]
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::StaleSession => ErrorCode::StaleSession,
            Self::DuplicateExpired => ErrorCode::DuplicateExpired,
            Self::InvalidPayload | Self::UnmatchedAck => ErrorCode::InvalidPayload,
            Self::RetentionFull => ErrorCode::RateLimited,
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

/// [`PeerSession::accept_status`]が受理した`status`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedStatus {
    /// 受理した`status`。
    pub status: Status,
    /// Piが送った`get_status`への応答として待っていたものか（§5.6）。
    ///
    /// `false`は、`get_status`の`ok` ACKを受けていない時点で届いた`status`である。
    /// §4.6は`status`を「必要に応じてrate limit付きの定期health messageとして」送ることも
    /// 認めているため、**`false`を異常として拒否しない。**区別を呼び出し側へ渡すだけである。
    pub solicited: bool,
}

/// Pi側で観測するcounter（§8「Piも`parse_errors`、`unknown_types`、`suppressed_responses`を
/// local metricとlogへ保持する」）。
///
/// `parse_errors`に当たるものは、lineを復元する層の[`crate::SessionCounters::rejected_in`]が
/// 持つ。ここはframeを復元できた後の拒否と遷移だけを数える。`suppressed_responses`は、
/// Piが`boot`への応答を抑制・保留する経路（`PROTO-TBD-012`のbudget）をまだ持たないため無い。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PeerCounters {
    /// 確定したESP32 session遷移の回数（§4.6の`session_switches`に当たる）。
    pub session_switches: u64,
    /// 保持した`ack`をreplayした`boot`の件数。
    pub duplicate_replays: u64,
    /// `stale_session`で拒否・遮蔽した件数。
    pub stale_sessions: u64,
    /// `duplicate_expired`で拒否した件数。
    pub duplicate_expired: u64,
    /// `invalid_payload`で拒否した`boot`の件数（現在の`sid`で新しい`id`）。
    pub invalid_payloads: u64,
    /// retired session保持が満杯で遷移を拒否した件数（codeは`rate_limited`）。
    pub rate_limited: u64,
    /// Piの要求と相関しなかったACKの件数（§6）。
    ///
    /// **Piが送った`hello`へのACKもここに入る。**[`PeerSession::note_sent`]は`ping`／`get_status`
    /// だけを追跡するためである（`crate::coordinator::handle_frame`のdoc参照）。
    pub unmatched_acks: u64,
    /// ESP32→Piで定義されていないtypeを無視した件数（§3、§8）。
    pub unknown_types: u64,
}

/// ESP32 peer sessionの状態。
#[derive(Debug)]
pub struct PeerSession {
    /// 現在承認しているESP32 `sid`。最初の`boot`を受けるまでは`None`。
    esp32_sid: Option<u32>,
    /// 直前まで有効だった`sid`の上限付き集合（§3.1、§5.1）。
    retired: VecDeque<u32>,
    /// 現在sessionの`boot`のduplicate履歴（§9）。session遷移を確定したときだけ破棄する。
    boot_history: DuplicateHistory<Ack>,
    /// Piが送った、応答待ちの要求。`session.send`で割り当てた`id`をkeyにする。
    ///
    /// **`retired`／`boot_history`と違い、容量の上限を持たない。**ACKが返る、
    /// [`Self::poll_outstanding`]が取り下げる、[`Self::forget_sent`]を呼ぶ、
    /// [`Self::handle_boot`]がsession遷移を確定して丸ごと`clear`する、の
    /// いずれかで抜けるまで残る。上限が無い代わり、呼び出し側が
    /// [`Self::poll_outstanding`]を一定間隔で呼び続けることに頼っている。
    outstanding: HashMap<u32, OutstandingEntry>,
    /// `boot`確立後の`get_status`（§10.1 step4）を、まだenqueueできていないか。
    ///
    /// `Established`直後の送信が失敗し、予約を残す`SendError`だった場合にtrueになる
    /// （どれが当たるかは`coordinator.rs`の`keeps_status_sync`だけが決める。理由は`try_send_get_status`の表）。
    /// この時点では`note_sent`が呼ばれていないため`outstanding`には何も残らず、
    /// [`Self::poll_outstanding`]の対象にもならない——このflagが唯一の記録である。
    /// `note_sent`が`get_status`をenqueueした時点で自動的に落ちる
    /// （[`Self::note_sent`]のdoc参照）。以後のACK到達までの再送は
    /// `outstanding`／[`Self::poll_outstanding`]が引き継ぐ。このflagは
    /// enqueueできたかどうかだけを表し、ACKが来たかどうかは表さない。
    status_sync_pending: bool,
    /// `get_status`の`ok` ACKを受け、まだ`status`を受けていないか（§5.6）。
    /// 次に届いた現在sessionの`status`を応答として扱うために使う（[`AcceptedStatus`]）。
    status_awaited: bool,
    counters: PeerCounters,
}

impl PeerSession {
    /// 未確立のsessionを作る。
    ///
    /// `duplicate_policy`は`boot`のduplicate履歴の保持件数と保持期間である。
    /// **このcrateは値を持たない**（[`DuplicatePolicy`]のdoc参照）。
    #[must_use]
    pub fn new(duplicate_policy: DuplicatePolicy) -> Self {
        Self {
            esp32_sid: None,
            retired: VecDeque::with_capacity(RETIRED_CAPACITY),
            boot_history: DuplicateHistory::new(duplicate_policy),
            outstanding: HashMap::new(),
            status_sync_pending: false,
            status_awaited: false,
            counters: PeerCounters::default(),
        }
    }

    /// これまでのcounter。
    #[must_use]
    pub const fn counters(&self) -> PeerCounters {
        self.counters
    }

    /// `get_status`の`ok` ACKを受け、`status`をまだ受けていないか（§5.6）。
    #[must_use]
    pub const fn status_awaited(&self) -> bool {
        self.status_awaited
    }

    /// 拒否を[`PeerCounters`]へ計上する。
    fn count_rejection(&mut self, rejection: PeerRejection) {
        let counter = match rejection {
            PeerRejection::StaleSession => &mut self.counters.stale_sessions,
            PeerRejection::DuplicateExpired => &mut self.counters.duplicate_expired,
            PeerRejection::InvalidPayload => &mut self.counters.invalid_payloads,
            PeerRejection::UnmatchedAck => &mut self.counters.unmatched_acks,
            PeerRejection::RetentionFull => &mut self.counters.rate_limited,
        };
        *counter = counter.saturating_add(1);
    }

    /// ESP32→Piで定義されていないtypeを受けたと計上する（§3、§8）。
    ///
    /// `hello`・`ping`・`get_status`はPi→ESP32のmessageであり、Piが受ける方向では
    /// 定義されていない。§8は、Piが受けた`hello`を`unknown_type`として扱い、応答せず、
    /// duplicate照会もsession遷移も行わないと定める。**`sid`の判定より先に扱う**（§3）ため、
    /// `stale_sessions`には数えない。
    pub fn note_undefined_type(&mut self) {
        self.counters.unknown_types = self.counters.unknown_types.saturating_add(1);
    }

    /// 現在承認しているESP32 `sid`。未確立なら`None`。
    #[must_use]
    pub const fn esp32_sid(&self) -> Option<u32> {
        self.esp32_sid
    }

    /// Piが送った要求を記録する。`session.send`が返した`id`と、実際に送った
    /// `message`をそのまま渡す。記録していない`id`へのACKは
    /// [`PeerRejection::UnmatchedAck`]になる。
    ///
    /// **`kind`は`message`から導く**（呼び出し側が別々に指定する経路は無い。
    /// `OutstandingKind::classify`参照）。`message`が`ping`／`get_status`の
    /// どちらでもない場合は追跡せず`None`を返す。
    ///
    /// **`message`が`get_status`なら、[`Self::status_sync_pending`]も
    /// 合わせて`false`にする。**呼び出し側へ2手を強いると対が崩れうるため、
    /// 1手にまとめている。
    #[must_use = "Noneは追跡されなかったことを意味する。無視すると、応答が来ても相関しない"]
    pub fn note_sent(
        &mut self,
        id: u32,
        message: Message,
        sent_at_ms: u64,
    ) -> Option<OutstandingKind> {
        let kind = OutstandingKind::classify(&message)?;
        self.outstanding.insert(
            id,
            OutstandingEntry {
                kind,
                message,
                sent_at_ms,
                retries: 0,
            },
        );
        if matches!(kind, OutstandingKind::GetStatus) {
            self.status_sync_pending = false;
        }
        Some(kind)
    }

    /// `boot`確立後の`get_status`（§10.1 step4）を、まだ送れていないと記録する。
    ///
    /// 呼び出し側が送信を試みて失敗した直後に呼ぶ。[`Self::status_sync_pending`]が
    /// `true`を返すようになり、次に送信を試みる契機になる。
    pub fn mark_status_sync_pending(&mut self) {
        self.status_sync_pending = true;
    }

    /// `boot`確立後の`get_status`を、送り直しても送れないと確定したと記録する。
    ///
    /// 送り直しても、`Session`を替えても結果が変わらない失敗の直後に呼ぶ
    /// （どの`SendError`が当たるかは`coordinator.rs`の`keeps_status_sync`だけが決める）。
    /// [`Self::status_sync_pending`]が`false`へ戻り、再試行の契機が消える。
    /// **送れた場合に落とすのは[`Self::note_sent`]であり、この関数ではない。**
    pub fn clear_status_sync_pending(&mut self) {
        self.status_sync_pending = false;
    }

    /// `boot`確立後の`get_status`を、まだ送れていないか。
    ///
    /// `true`なら、呼び出し側は`get_status`を送り直し、成功したら
    /// [`Self::note_sent`]を呼ぶ。`note_sent`が`message`を`get_status`として
    /// 分類した時点でこのflagを自動的に落とすため、呼び出し側が別途
    /// 落とす必要はない。
    #[must_use]
    pub const fn status_sync_pending(&self) -> bool {
        self.status_sync_pending
    }

    /// 応答待ちの要求を1件取り下げる。記録していない`id`なら`None`を返す。
    ///
    /// **ACK timeoutに基づく自動的な取り下げは[`Self::poll_outstanding`]が行う。**
    /// こちらは呼び出し側が別の理由で明示的に取り下げるためにある。
    /// session切り替えの取り下げはこれを使わない（[`Self::handle_boot`]が
    /// 遷移確定時に`outstanding`を丸ごと`clear`する）。
    pub fn forget_sent(&mut self, id: u32) -> Option<OutstandingKind> {
        self.outstanding.remove(&id).map(|entry| entry.kind)
    }

    /// ACK timeoutを超えた応答待ちの要求を判定する。
    ///
    /// timeoutに達していない要求は対象にしない。timeoutに達した要求のうち、
    /// 再送予算（[`RetryPolicy::max_retries`]）がまだ残っているものは`retries`を
    /// 1増やし送信時刻を`now_ms`へ更新したうえで保持し、
    /// [`OutstandingAction::Retry`]として返す。予算を使い切ったものは取り下げ、
    /// [`OutstandingAction::GaveUp`]として返す。**再送そのものは行わない**
    /// （送信はtransportを持つ[`crate::Session`]の役割。module doc参照）。
    ///
    /// `now_ms`は[`Self::note_sent`]へ渡した`sent_at_ms`と同じ時計（送信側の
    /// uptime。§3）で、単調に増加する値を渡す（巻き戻るとそのentryがtimeoutを
    /// 検出できなくなる）。
    ///
    /// `policy`を`Self`が保持せず引数で受け取るのは、この型が`SerialConfig`を
    /// 持たないためである（`PeerSession`はbyte列やtransportを持たない。
    /// module doc参照）。呼び出しをまたいだ一貫性は呼び出し側の責務であり、
    /// [`crate::retry_due_requests`]が毎回`session.config().retry()`から
    /// 読むことでそれを保っている。
    pub fn poll_outstanding(
        &mut self,
        now_ms: u64,
        policy: &RetryPolicy,
    ) -> Vec<(u32, OutstandingAction)> {
        let mut due = Vec::new();
        self.outstanding.retain(|&id, entry| {
            let elapsed = Duration::from_millis(now_ms.saturating_sub(entry.sent_at_ms));
            if elapsed < policy.ack_timeout() {
                return true;
            }
            if entry.retries >= policy.max_retries() {
                due.push((id, OutstandingAction::GaveUp(entry.kind)));
                false
            } else {
                entry.retries += 1;
                entry.sent_at_ms = now_ms;
                due.push((
                    id,
                    OutstandingAction::Retry(entry.kind, entry.message.clone()),
                ));
                true
            }
        });
        due
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

    /// 現在sessionをretireすると、保持期間の満了前に追い出すことになるかを返す。
    ///
    /// **既にretiredにある`sid`は件数を増やさない**ため、追い出しは起きない。
    fn retiring_would_evict(&self) -> bool {
        match self.esp32_sid {
            Some(old) if !self.retired.contains(&old) => self.retired.len() >= RETIRED_CAPACITY,
            _ => false,
        }
    }

    /// 現在sessionをretiredへ移す。
    ///
    /// **追い出さない**（§3.1）。追い出しを要する場合は、呼び出す前に
    /// [`Self::retiring_would_evict`]で遷移そのものを拒否している。
    fn retire_current(&mut self) {
        if let Some(old) = self.esp32_sid
            && !self.retired.contains(&old)
        {
            debug_assert!(
                self.retired.len() < RETIRED_CAPACITY,
                "retiring would evict; the transition must be rejected first"
            );
            self.retired.push_back(old);
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

    /// `boot`を拒否し、計上する。**session状態は変えない。**
    fn reject_boot(&mut self, sid: u32, id: u32, rejection: PeerRejection) -> BootHandled {
        self.count_rejection(rejection);
        BootHandled {
            reply: Self::rejection_ack(sid, id, rejection),
            outcome: BootOutcome::Rejected(rejection),
        }
    }

    /// `boot`を処理する（§4.1、§5.1、§8手順8〜10）。
    ///
    /// 遷移候補のsession遷移budget・cooldown（§5.1、`PROTO-TBD-012`）は判定しない。
    /// 呼び出し側がその上限を別途課す場合は、この呼び出し自体を抑制すること
    /// （session状態はこの呼び出しで変わるため、抑制せずに呼ぶと無条件で遷移する）。
    ///
    /// `now_ms`はduplicate履歴の保持期間を測る時計（Pi自身のuptime）の値である。
    /// 単調に増加する値を渡す。
    pub fn handle_boot(&mut self, sid: u32, id: u32, boot: Boot, now_ms: u64) -> BootHandled {
        if self.is_retired(sid) {
            return self.reject_boot(sid, id, PeerRejection::StaleSession);
        }

        if self.esp32_sid == Some(sid) {
            let rejection = match self.boot_history.lookup(id, now_ms) {
                Lookup::Replay(ack) => {
                    let reply = Message::Ack(ack.clone());
                    self.counters.duplicate_replays =
                        self.counters.duplicate_replays.saturating_add(1);
                    return BootHandled {
                        reply,
                        outcome: BootOutcome::Replayed,
                    };
                }
                Lookup::Expired => PeerRejection::DuplicateExpired,
                // 現在のsidで未処理の新しいid。ESP32の再起動には新しいsidが必須であり、
                // 同じsidのまま新しいidを名乗ることは想定されない（§5.1）。
                Lookup::New => PeerRejection::InvalidPayload,
            };
            return self.reject_boot(sid, id, rejection);
        }

        // 未知のsid: hello／bootだけが遷移候補になれる（§5.1優先順位3）。

        // **保持が満杯なら、遷移そのものを拒否する。**旧sidを追い出すと「未知」に戻り、
        // 遅れて届いたhello／bootが遷移候補として受理されて実行中motionを停止する
        // （§3.1）。**session状態は変えない。**
        if self.retiring_would_evict() {
            return self.reject_boot(sid, id, PeerRejection::RetentionFull);
        }

        // **ここからが旧sessionの追跡のreset（§10.1 step2）である。**遷移を確定した
        // ときだけ行い、拒否の経路（上）では何も変えない。
        self.retire_current();
        self.boot_history.clear();
        self.esp32_sid = Some(sid);
        // 新sessionでは、Piが持っていたESP32宛ての未完了要求はtimeout扱いにする
        // （§6「session切り替え後は旧sessionの未ACK commandをtimeout扱いにする」）。
        self.outstanding.clear();
        // 旧sessionの`get_status`に対する`status`は、もう届いても応答として扱わない。
        self.status_awaited = false;
        self.counters.session_switches = self.counters.session_switches.saturating_add(1);

        let ack = Ack {
            reply_sid: sid,
            reply_to: id,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        };
        self.boot_history.record(id, ack.clone(), now_ms);
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
    /// 相関が取れたACKは、`status`が`ok`でも`rejected`でも要求を完了させる
    /// （`rejected`は終端の応答であり、再送の対象ではない）。`get_status`への`ok`なら、
    /// 続けて届く`status`を応答として待つ（[`Self::status_awaited`]、§5.6）。
    ///
    /// 同じ`reply_to`への2件目のACK（Piが再送し、ESP32が保持ACKを返した場合など）は、
    /// 1件目で要求が完了しているため[`PeerRejection::UnmatchedAck`]になる。
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
        let result = self.correlate(envelope_sid, our_sid, ack);
        if let Err(rejection) = result {
            self.count_rejection(rejection);
        }
        result
    }

    fn correlate(
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
        let Some(entry) = self.outstanding.remove(&ack.reply_to) else {
            return Err(PeerRejection::UnmatchedAck);
        };
        if matches!(entry.kind, OutstandingKind::GetStatus) && ack.status == AckStatus::Ok {
            self.status_awaited = true;
        }
        Ok(CorrelatedAck {
            request: entry.kind,
            ack,
        })
    }

    /// `status`を検査する（§5.1優先順位・§8）。ESP32からのeventであり、
    /// Piは応答を返さない。
    ///
    /// 受理した`status`が`get_status`への応答として待っていたものかを
    /// [`AcceptedStatus::solicited`]で返し、待ちを解く（§5.6）。
    ///
    /// # Errors
    ///
    /// envelopeの`sid`が現在のESP32 sessionと異なる場合は[`PeerRejection::StaleSession`]を返す。
    /// この場合、`get_status`への応答の待ちは解かない。
    pub fn accept_status(
        &mut self,
        envelope_sid: u32,
        status: Status,
    ) -> Result<AcceptedStatus, PeerRejection> {
        if self.esp32_sid == Some(envelope_sid) {
            let solicited = core::mem::take(&mut self.status_awaited);
            Ok(AcceptedStatus { status, solicited })
        } else {
            self.count_rejection(PeerRejection::StaleSession);
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

    use core::time::Duration;

    use super::{
        Ack, AckStatus, Boot, BootOutcome, DuplicatePolicy, ErrorCode, OutstandingAction,
        OutstandingKind, PeerRejection, PeerSession, RETIRED_CAPACITY, RetryPolicy, Status,
    };

    fn boot() -> Boot {
        Boot {
            firmware: "0.1.0".to_owned(),
            board: "esp32".to_owned(),
            reset_reason: "power_on".to_owned(),
        }
    }

    /// test用のduplicate履歴の方針。**値はtestの都合であり、`PROTO-TBD-005`の候補ではない。**
    const TEST_RETENTION_MS: u64 = 1_000;

    fn policy() -> DuplicatePolicy {
        DuplicatePolicy::new(8, Duration::from_millis(TEST_RETENTION_MS)).expect("0ではない")
    }

    /// `boot`の保持ACKが保持期間を過ぎたら、再送は`duplicate_expired`で拒否する。
    /// **session遷移として受け直さない**（`PROTO-TBD-005`「evictしたentryへの再送は
    /// 新規commandとして実行しない」）。
    #[test]
    fn a_boot_retried_after_the_retention_period_is_rejected_as_duplicate_expired() {
        let mut peer = PeerSession::new(policy());
        let _ = peer.handle_boot(41_207, 1, boot(), 0);

        let late = peer.handle_boot(41_207, 1, boot(), TEST_RETENTION_MS);
        assert_eq!(
            late.outcome,
            BootOutcome::Rejected(PeerRejection::DuplicateExpired)
        );
        match late.reply {
            super::Message::Ack(ack) => assert_eq!(ack.code, Some(ErrorCode::DuplicateExpired)),
            other => panic!("Ackを期待した: {other:?}"),
        }
        assert_eq!(peer.esp32_sid(), Some(41_207), "sessionは変わらない");
        assert_eq!(peer.counters().session_switches, 1, "遷移は最初の1回だけ");
        assert_eq!(peer.counters().duplicate_expired, 1);
    }

    #[test]
    fn an_unknown_sid_establishes_a_new_session_and_returns_an_ok_ack() {
        let mut peer = PeerSession::new(policy());
        assert_eq!(peer.esp32_sid(), None);

        let handled = peer.handle_boot(41_207, 1, boot(), 0);

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
        let mut peer = PeerSession::new(policy());
        let first = peer.handle_boot(41_207, 1, boot(), 0);
        let retry = peer.handle_boot(41_207, 1, boot(), 0);

        assert_eq!(retry.outcome, BootOutcome::Replayed);
        assert_eq!(retry.reply, first.reply, "同じACKを再送する");
    }

    /// [`RETIRED_CAPACITY`]を`sid`の型で扱う。**`as`で切り捨てない。**
    fn retired_capacity() -> u32 {
        u32::try_from(RETIRED_CAPACITY).expect("RETIRED_CAPACITY fits in u32")
    }

    #[test]
    fn a_transition_that_would_evict_a_retired_sid_is_rejected() {
        // 保持が満杯になるまで遷移させる。**追い出しは起きない。**
        let mut peer = PeerSession::new(policy());
        let _ = peer.handle_boot(1, 1, boot(), 0);
        for sid in 2..=(retired_capacity() + 1) {
            let _ = peer.handle_boot(sid, 1, boot(), 0);
        }
        // ここで retired は満杯であり、次の遷移は最古を追い出すことになる。
        let rejected = peer.handle_boot(9_000, 1, boot(), 0);
        assert_eq!(
            rejected.outcome,
            BootOutcome::Rejected(PeerRejection::RetentionFull)
        );
        match rejected.reply {
            super::Message::Ack(ack) => {
                assert_eq!(ack.status, AckStatus::Rejected);
                assert_eq!(ack.code, Some(ErrorCode::RateLimited));
            }
            other => panic!("Ackを期待した: {other:?}"),
        }
    }

    #[test]
    fn a_rejected_transition_does_not_change_session_state() {
        // **拒否は副作用を持たない。**最古のsidは「未知」へ戻らない。
        let mut peer = PeerSession::new(policy());
        let _ = peer.handle_boot(1, 1, boot(), 0);
        for sid in 2..=(retired_capacity() + 1) {
            let _ = peer.handle_boot(sid, 1, boot(), 0);
        }
        let current = retired_capacity() + 1;
        let _ = peer.handle_boot(9_000, 1, boot(), 0);

        // 現在sessionは変わっていない。
        assert_eq!(peer.esp32_sid, Some(current));
        // 最古のsidはretiredのままであり、stale_sessionで拒否される。
        // **追い出していれば、ここは遷移候補として受理されてしまう。**
        let stale = peer.handle_boot(1, 2, boot(), 0);
        assert_eq!(
            stale.outcome,
            BootOutcome::Rejected(PeerRejection::StaleSession)
        );
    }

    #[test]
    fn a_boot_from_a_retired_sid_is_rejected_as_stale_session() {
        let mut peer = PeerSession::new(policy());
        let _ = peer.handle_boot(41_207, 1, boot(), 0);
        // 別のsidへ遷移させ、41_207をretiredへ移す。
        let _ = peer.handle_boot(90_000, 1, boot(), 0);

        let rejected = peer.handle_boot(41_207, 2, boot(), 0);
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
        let mut peer = PeerSession::new(policy());
        let _ = peer.handle_boot(41_207, 1, boot(), 0);

        let rejected = peer.handle_boot(41_207, 2, boot(), 0);
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
        let mut peer = PeerSession::new(policy());
        let _ = peer.handle_boot(41_207, 1, boot(), 0);
        let _ = peer.note_sent(7, super::Message::Ping, 100);

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
        let mut peer = PeerSession::new(policy());
        let _ = peer.handle_boot(41_207, 1, boot(), 0);
        let _ = peer.note_sent(7, super::Message::GetStatus, 100);

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
        let mut peer = PeerSession::new(policy());
        let _ = peer.handle_boot(41_207, 1, boot(), 0);
        let _ = peer.note_sent(7, super::Message::Ping, 100);

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
        let mut peer = PeerSession::new(policy());
        let _ = peer.handle_boot(41_207, 1, boot(), 0);

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

    fn retry_policy(ack_timeout_ms: u64, max_retries: u32) -> RetryPolicy {
        RetryPolicy::new(Duration::from_millis(ack_timeout_ms), max_retries)
            .expect("ack_timeoutは0ではない")
    }

    /// timeoutに達していない要求は対象にしない。
    #[test]
    fn poll_outstanding_ignores_requests_that_have_not_timed_out_yet() {
        let mut peer = PeerSession::new(policy());
        let _ = peer.note_sent(1, super::Message::Ping, 1_000);

        let due = peer.poll_outstanding(1_100, &retry_policy(500, 3));
        assert!(due.is_empty(), "500ms timeoutに対し100msしか経っていない");
        assert_eq!(peer.outstanding_len(), 1, "取り下げていない");
    }

    /// timeoutを超え、再送予算が残っていれば`Retry`を返し、送信時刻を更新する。
    #[test]
    fn poll_outstanding_retries_a_timed_out_request_while_budget_remains() {
        let mut peer = PeerSession::new(policy());
        let _ = peer.note_sent(1, super::Message::Ping, 1_000);

        let due = peer.poll_outstanding(1_600, &retry_policy(500, 3));
        assert_eq!(
            due,
            vec![(
                1,
                OutstandingAction::Retry(OutstandingKind::Ping, super::Message::Ping)
            )]
        );
        assert_eq!(peer.outstanding_len(), 1, "予算が残る限り保持する");

        // 送信時刻が更新されているため、直後にもう一度呼んでもすぐには再度timeoutしない。
        let immediate = peer.poll_outstanding(1_650, &retry_policy(500, 3));
        assert!(immediate.is_empty(), "送信時刻を更新したはずである");
    }

    /// 再送予算を使い切ったら`GaveUp`を返し、要求を取り下げる。
    #[test]
    fn poll_outstanding_gives_up_after_exhausting_the_retry_budget() {
        let mut peer = PeerSession::new(policy());
        let _ = peer.note_sent(1, super::Message::GetStatus, 0);
        let policy = retry_policy(100, 2);

        // 1回目・2回目はretry予算が残っている。
        for round in 1_u64..=2 {
            let now = round * 100;
            let due = peer.poll_outstanding(now, &policy);
            assert_eq!(
                due,
                vec![(
                    1,
                    OutstandingAction::Retry(OutstandingKind::GetStatus, super::Message::GetStatus)
                )],
                "round {round}"
            );
        }

        // 3回目でmax_retries(2)を使い切り、取り下げる。
        let due = peer.poll_outstanding(300, &policy);
        assert_eq!(
            due,
            vec![(1, OutstandingAction::GaveUp(OutstandingKind::GetStatus))]
        );
        assert_eq!(peer.outstanding_len(), 0, "取り下げ後は残らない");
    }

    /// `note_sent`は`kind`を`message`から導く。呼び出し側が`kind`を別途渡す
    /// 経路が無いため、`message`とは食い違う`kind`を記録させられない。
    #[test]
    fn note_sent_classifies_the_kind_from_the_message() {
        let mut peer = PeerSession::new(policy());
        assert_eq!(
            peer.note_sent(1, super::Message::Ping, 0),
            Some(OutstandingKind::Ping)
        );
        assert_eq!(
            peer.note_sent(2, super::Message::GetStatus, 0),
            Some(OutstandingKind::GetStatus)
        );
        assert_eq!(peer.outstanding_len(), 2);
    }

    /// `note_sent`が`get_status`を追跡したら、`status_sync_pending`が
    /// 呼び出し側の追加操作なしに自動的に落ちる。
    ///
    /// `status_sync_pending`（flagが立っているか）と`outstanding`（実際に
    /// 追跡中の`get_status`があるか）は、同じ「まだ送れていない」という事実を
    /// 別々の場所に持つ。呼び出し側に「`note_sent`を呼んだら
    /// `clear_status_sync_pending`も呼ぶ」という2手を強いると、その対が崩れる
    /// 呼び出し経路（`note_sent`だけ呼んでflagを消し忘れる）を作れてしまう。
    /// `note_sent`自身が両方を1手で更新することで、この食い違いを構造的に
    /// 無くす。
    #[test]
    fn note_sent_clears_status_sync_pending_when_it_tracks_a_get_status() {
        let mut peer = PeerSession::new(policy());
        peer.mark_status_sync_pending();
        assert!(peer.status_sync_pending(), "前提: flagが立っている");

        // try_send_get_statusを経由せず、note_sentを直接呼ぶ
        // （将来別の呼び出し経路が増えても、この不変条件が保たれることを確認する）。
        assert_eq!(
            peer.note_sent(1, super::Message::GetStatus, 0),
            Some(OutstandingKind::GetStatus)
        );

        assert!(
            !peer.status_sync_pending(),
            "note_sentがget_statusを追跡した時点でflagは落ちる"
        );
    }

    /// 追跡対象外の`message`（`ping`／`get_status`以外）は記録しない。
    ///
    /// `kind`を`message`から導くようにしたのは、呼び出し側が`kind`と`message`を
    /// 食い違う組み合わせで渡せてしまう経路を無くすためである
    /// （[`OutstandingKind::classify`]のdoc参照）。分類できない`message`を
    /// 黙って追跡してしまうと、同じ食い違いが別の形で戻ってくる。
    #[test]
    fn note_sent_does_not_track_a_message_it_cannot_classify() {
        let mut peer = PeerSession::new(policy());
        let hello = super::Message::Hello(deskcat_protocol::Hello {
            host: "deskcatd".to_owned(),
            version: "0.1.0".to_owned(),
            reason: deskcat_protocol::HelloReason::Resync,
        });

        assert_eq!(peer.note_sent(1, hello, 0), None);
        assert_eq!(peer.outstanding_len(), 0, "追跡していない");
    }

    /// `poll_outstanding`が再送用に返すのは`note_sent`へ渡した[`super::Message`]
    /// そのものであって、種別から組み直したものではない。
    #[test]
    fn poll_outstanding_retries_the_exact_message_that_was_sent() {
        let mut peer = PeerSession::new(policy());
        let _ = peer.note_sent(1, super::Message::GetStatus, 0);

        let due = peer.poll_outstanding(1_000, &retry_policy(500, 1));
        assert_eq!(
            due,
            vec![(
                1,
                OutstandingAction::Retry(OutstandingKind::GetStatus, super::Message::GetStatus)
            )]
        );
    }
}
