//! 「ESP32に出させたい表情」と「ESP32が出していると分かっている表情」の差を埋める。
//!
//! **I/Oを持たない。**送るかどうかの判断だけを持ち、送信は[`crate::daemon`]が行う。
//!
//! # 規則
//!
//! - `set_expression`は**同時に1件しか応答待ちにしない**。応答待ちの間に表情が変わったら、
//!   欲しい表情だけを更新し、ACKかACK待ちの取り下げの後に最新の表情を新しい`id`で送る。
//!   **古い要求の追跡を外さない**（外すと、そのACKが`unmatched_acks`に入り、§6の本物の
//!   不一致と区別できなくなる）。
//! - 送ってよいのは、`get_status`への`status`を受けて**現在の表示が分かった後**だけである
//!   （§10.1手順6〜7）。それまでは欲しい表情の更新だけを行う。
//! - 内容を拒否された表情は、欲しい表情が変わるまで送り直さない（拒否のACKに毎tick送り返さない）。
//!   **`stale_session`の拒否は内容の拒否ではない**（sessionが未確立なだけである）。再開の後に送り直す。
//! - 表情の見た目の送り直し（古いrelative motionの再実行）はしない（§10.1手順8）。

use deskcat_domain::Expression;
use deskcat_protocol::ExpressionName;

/// 一時的な拒否（`busy`、`rate_limited`、`duplicate_expired`）の後に、同じ表情を送り直してよい連続の回数。
/// **暫定値であり、根拠は無い。**「有限である」という性質だけを満たす。超えたら、内容の拒否と同じに扱う。
pub const TRANSIENT_RETRY_LIMIT: u32 = 3;

/// ACKが無いまま取り下げてから、同じ表情を新しい`id`で送り直してよい連続の回数。**暫定値。**
///
/// 根拠は無い。「有限である」という性質だけを満たす。超えたら、欲しい表情が変わるまで送らない。
/// 現行のfirmwareは`set_expression`にACKを返さない（`firmware/esp32/src/pi_link.rs`。処理を足すのは#21）ので、
/// 上限が無いと、新しい`id`で送り続ける。
pub const SILENT_RETRY_LIMIT: u32 = 3;

/// 応答待ちの`set_expression`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InFlight {
    id: u32,
    expression: Expression,
}

/// 表情の同期の状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionSync {
    desired: Expression,
    /// ESP32が出していると分かっている表情。分からなければ`None`。
    confirmed: Option<Expression>,
    in_flight: Option<InFlight>,
    /// 拒否された表情。欲しい表情がこれと違う値になるまで、同じ表情を送らない。
    rejected: Option<Expression>,
    /// `status`で現在の表示が分かっており、送ってよいか。
    ready: bool,
    /// 一時的な拒否の後、この時刻まで送らない（cooldown）。
    retry_at: Option<u64>,
    /// 一時的な拒否の連続の回数。
    transient: u32,
    /// ACKが無いまま取り下げた連続の回数。
    silent: u32,
}

impl ExpressionSync {
    /// `Neutral`を欲しい表情として始める。`ready`になるまで何も送らない。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            desired: Expression::Neutral,
            confirmed: None,
            in_flight: None,
            rejected: None,
            ready: false,
            retry_at: None,
            transient: 0,
            silent: 0,
        }
    }

    /// 欲しい表情。
    #[must_use]
    pub const fn desired(&self) -> Expression {
        self.desired
    }

    /// 欲しい表情を更新する。
    pub fn set_desired(&mut self, expression: Expression) {
        if self.desired != expression {
            self.retry_at = None;
            self.transient = 0;
            self.silent = 0;
        }
        self.desired = expression;
        if self.rejected != Some(expression) {
            self.rejected = None;
        }
    }

    /// 次に送る表情。送らないなら`None`。
    #[must_use]
    pub fn next_to_send(&self, now_ms: u64) -> Option<Expression> {
        let send = self.ready
            && self.in_flight.is_none()
            && self.retry_at.is_none_or(|at| now_ms >= at)
            && self.confirmed != Some(self.desired)
            && self.rejected != Some(self.desired);
        send.then_some(self.desired)
    }

    /// `expression`を`id`で送った。
    pub fn sent(&mut self, id: u32, expression: Expression) {
        self.in_flight = Some(InFlight { id, expression });
    }

    /// `id`のACKが`ok`だった。応答待ちの`id`と違えば何もしない。
    pub fn acked_ok(&mut self, id: u32) {
        if let Some(flight) = self.take_in_flight(id) {
            self.confirmed = Some(flight.expression);
            self.retry_at = None;
            self.transient = 0;
            self.silent = 0;
        }
    }

    /// `id`のACKが拒否だった。**同じ表情を送り直さない。**表示の状態も分からなくなる。
    pub fn acked_rejected(&mut self, id: u32) {
        if let Some(flight) = self.take_in_flight(id) {
            self.silent = 0;
            self.rejected = Some(flight.expression);
            self.confirmed = None;
        }
    }

    /// `id`のACKが`stale_session`の拒否だった。**表情の内容は拒否されていない**（sessionが未確立）。
    /// 表示の状態は分からなくなるが、同じ表情を送り直してよい（再開の後に）。
    ///
    /// **再開の判断を見送る場合があるので、ここで送るのを止める。**止めないと、再開を待つ間、
    /// 同じ表情を毎周送り、`stale_session`の往復を繰り返す。
    pub fn acked_stale(&mut self, id: u32) {
        if self.take_in_flight(id).is_some() {
            self.silent = 0;
            self.confirmed = None;
            self.ready = false;
        }
    }

    /// `id`のACKが一時的な拒否（`busy`、`rate_limited`、`duplicate_expired`）だった。
    /// `retry_at`まで待ってから、同じ表情を新しい`id`で送り直す。連続が
    /// [`TRANSIENT_RETRY_LIMIT`]を超えたら、内容の拒否と同じに扱う。
    pub fn acked_retry_later(&mut self, id: u32, retry_at: u64) {
        if let Some(flight) = self.take_in_flight(id) {
            self.silent = 0;
            self.confirmed = None;
            self.transient += 1;
            if self.transient > TRANSIENT_RETRY_LIMIT {
                self.rejected = Some(flight.expression);
                self.retry_at = None;
                self.transient = 0;
            } else {
                self.retry_at = Some(retry_at);
            }
        }
    }

    /// `id`のACK待ちを取り下げた。表示の状態は分からない。送り直す（新しい`id`で）。
    ///
    /// 連続が[`SILENT_RETRY_LIMIT`]を超えたら、内容の拒否と同じく、欲しい表情が変わるまで送らない。
    /// 送るのを止めたときに`true`を返す（呼び出し側が報告する）。
    pub fn gave_up(&mut self, id: u32) -> bool {
        let Some(flight) = self.take_in_flight(id) else {
            return false;
        };
        self.confirmed = None;
        self.silent += 1;
        if self.silent > SILENT_RETRY_LIMIT {
            self.rejected = Some(flight.expression);
            self.silent = 0;
            return true;
        }
        false
    }

    /// 欲しい表情をmessageにできなかった（送り直しても同じ結果になる）。
    pub fn input_fault(&mut self) {
        self.rejected = Some(self.desired);
    }

    /// `status`が現在の表示を報せた。分からない名前なら`None`。
    ///
    /// 応答した`get_status`（`solicited`）なら、以後送ってよい。
    pub fn status_reported(&mut self, reported: Option<Expression>, solicited: bool) {
        self.confirmed = reported;
        if solicited {
            self.ready = true;
        }
    }

    /// 現在の表示が分からなくなった（session確立のやり直し）。再び`status`を受けるまで送らない。
    pub fn not_ready(&mut self) {
        self.ready = false;
        self.confirmed = None;
    }

    /// 送ってよいか。
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.ready
    }

    fn take_in_flight(&mut self, id: u32) -> Option<InFlight> {
        if self.in_flight.is_some_and(|f| f.id == id) {
            self.in_flight.take()
        } else {
            None
        }
    }
}

impl Default for ExpressionSync {
    fn default() -> Self {
        Self::new()
    }
}

/// 表情をwireの名前にする。
#[must_use]
pub const fn to_wire(expression: Expression) -> ExpressionName {
    match expression {
        Expression::Neutral => ExpressionName::Neutral,
        Expression::Happy => ExpressionName::Happy,
        Expression::Surprised => ExpressionName::Surprised,
    }
}

/// `status.display.expression`の名前を表情にする。**知らない名前は`None`**（推測しない）。
///
/// 名前は§5.2の初期のexpression名であり、`ExpressionName`のserdeの出力と一致する
/// （testで確かめている）。
#[must_use]
pub fn from_status_name(name: &str) -> Option<Expression> {
    match name {
        "neutral" => Some(Expression::Neutral),
        "happy" => Some(Expression::Happy),
        "surprised" => Some(Expression::Surprised),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> ExpressionSync {
        let mut sync = ExpressionSync::new();
        sync.status_reported(Some(Expression::Neutral), true);
        sync
    }

    #[test]
    fn nothing_is_sent_before_the_display_is_known() {
        let mut sync = ExpressionSync::new();
        sync.set_desired(Expression::Happy);
        assert_eq!(sync.next_to_send(0), None);
    }

    #[test]
    fn an_unsolicited_status_does_not_make_it_ready() {
        let mut sync = ExpressionSync::new();
        sync.set_desired(Expression::Happy);
        sync.status_reported(Some(Expression::Neutral), false);
        assert_eq!(sync.next_to_send(0), None);
    }

    #[test]
    fn a_difference_is_sent_once_and_only_one_is_in_flight() {
        let mut sync = ready();
        sync.set_desired(Expression::Happy);
        assert_eq!(sync.next_to_send(0), Some(Expression::Happy));
        sync.sent(1, Expression::Happy);
        sync.set_desired(Expression::Surprised);
        assert_eq!(sync.next_to_send(0), None, "応答待ちの間は送らない");
        sync.acked_ok(1);
        assert_eq!(sync.next_to_send(0), Some(Expression::Surprised));
    }

    #[test]
    fn an_ack_for_another_id_changes_nothing() {
        let mut sync = ready();
        sync.set_desired(Expression::Happy);
        sync.sent(2, Expression::Happy);
        sync.acked_ok(1);
        assert_eq!(sync.next_to_send(0), None);
    }

    #[test]
    fn nothing_is_sent_when_the_display_already_matches() {
        let mut sync = ready();
        sync.set_desired(Expression::Neutral);
        assert_eq!(sync.next_to_send(0), None);
    }

    #[test]
    fn a_rejected_expression_is_not_resent_until_the_desire_changes() {
        let mut sync = ready();
        sync.set_desired(Expression::Happy);
        sync.sent(1, Expression::Happy);
        sync.acked_rejected(1);
        assert_eq!(sync.next_to_send(0), None);
        sync.set_desired(Expression::Happy);
        assert_eq!(sync.next_to_send(0), None, "同じ値の更新では解けない");
        sync.set_desired(Expression::Neutral);
        sync.set_desired(Expression::Happy);
        assert_eq!(sync.next_to_send(0), Some(Expression::Happy));
    }

    #[test]
    fn a_stale_session_rejection_does_not_block_the_same_expression() {
        let mut sync = ready();
        sync.set_desired(Expression::Happy);
        sync.sent(1, Expression::Happy);
        sync.acked_stale(1);
        sync.not_ready();
        assert_eq!(sync.next_to_send(0), None, "再開の間は送らない");
        sync.status_reported(Some(Expression::Neutral), true);
        assert_eq!(sync.next_to_send(0), Some(Expression::Happy));
    }

    #[test]
    fn silence_stops_sending_after_the_limit_until_the_desire_changes() {
        let mut sync = ready();
        sync.set_desired(Expression::Happy);
        for round in 0..=SILENT_RETRY_LIMIT {
            assert_eq!(sync.next_to_send(0), Some(Expression::Happy), "{round}");
            sync.sent(round, Expression::Happy);
            let stopped = sync.gave_up(round);
            assert_eq!(stopped, round == SILENT_RETRY_LIMIT, "{round}");
        }
        assert_eq!(sync.next_to_send(0), None);
        sync.set_desired(Expression::Neutral);
        sync.set_desired(Expression::Happy);
        assert_eq!(sync.next_to_send(0), Some(Expression::Happy));
    }

    #[test]
    fn an_ack_resets_the_silence() {
        let mut sync = ready();
        sync.set_desired(Expression::Happy);
        for round in 0..SILENT_RETRY_LIMIT {
            sync.sent(round, Expression::Happy);
            assert!(!sync.gave_up(round));
        }
        sync.sent(100, Expression::Happy);
        sync.acked_retry_later(100, 0);
        for round in 0..SILENT_RETRY_LIMIT {
            sync.sent(200 + round, Expression::Happy);
            assert!(!sync.gave_up(200 + round), "ACKで数え直す");
        }
    }

    #[test]
    fn giving_up_resends_with_a_new_id() {
        let mut sync = ready();
        sync.set_desired(Expression::Happy);
        sync.sent(1, Expression::Happy);
        sync.gave_up(1);
        assert_eq!(sync.next_to_send(0), Some(Expression::Happy));
    }

    #[test]
    fn an_input_fault_is_not_retried() {
        let mut sync = ready();
        sync.set_desired(Expression::Happy);
        sync.input_fault();
        assert_eq!(sync.next_to_send(0), None);
    }

    #[test]
    fn not_ready_stops_sending_until_the_next_solicited_status() {
        let mut sync = ready();
        sync.not_ready();
        sync.set_desired(Expression::Happy);
        assert_eq!(sync.next_to_send(0), None);
        sync.status_reported(Some(Expression::Surprised), true);
        assert_eq!(sync.next_to_send(0), Some(Expression::Happy));
    }

    #[test]
    fn an_unknown_reported_name_is_not_assumed_to_match() {
        let mut sync = ready();
        sync.status_reported(from_status_name("sleepy"), true);
        assert_eq!(sync.next_to_send(0), Some(Expression::Neutral));
    }

    #[test]
    fn status_names_agree_with_the_wire_names() {
        for expression in [
            Expression::Neutral,
            Expression::Happy,
            Expression::Surprised,
        ] {
            let wire = serde_json::to_value(to_wire(expression)).expect("serializeできる");
            let name = wire.as_str().expect("文字列である");
            assert_eq!(from_status_name(name), Some(expression));
        }
    }
}
