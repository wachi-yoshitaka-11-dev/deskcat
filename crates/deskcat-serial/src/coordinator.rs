//! [`Session`]（transportとPi自身の送信）と[`PeerSession`]（ESP32側の状態）を
//! またいで、送信を伴う判断を行う。
//!
//! ここでやるのは2つだけである。
//!
//! - `boot`を1件処理してACKを送り返す（outcomeによらず常に。§4.1）。ACKの
//!   送信自体が失敗しても、この関数は再試行しない（回復はESP32側の`boot`再送に
//!   頼る）。確立した場合は続けて現在状態を要求し（§10.1 step1〜4）、
//!   こちらの送信が失敗した場合は、送れる見込みがある間だけ次回以降のtickで再試行する
//!   （`SendError`ごとの扱いは`try_send_get_status`が持つ）
//! - 応答待ちの要求がACK timeoutを超えたら、同じ`id`で再送する（§9）
//!
//! **desired stateとの比較（§10.1 step6）や、状態設定commandの送出（step7）は
//! ここに含まない。**「望ましい状態」はdomain層の概念であり、`lib.rs`の
//! module docが定める「domain動作はこのcrateに入らない」という範囲の外にある。

use deskcat_protocol::{Boot, Message};

use crate::peer::{BootHandled, BootOutcome, OutstandingAction, OutstandingKind, PeerSession};
use crate::session::{ConnectionState, SendError, Session};

/// `boot`を1件処理し、ACKを送り返す。新しいESP32 sessionを確立した場合は、
/// 続けて`get_status`を送って現在状態を要求する（§10.1 step1〜4）。
///
/// `sid`／`id`／`boot`は、受信した`boot` frameからそのまま渡す。`now_ms`は
/// 送信に使う時刻（送信側のuptime。wall-clock timeではない、§3）である。
///
/// **常に`BootHandled`を返す。**[`PeerSession::handle_boot`]自体は失敗せず、
/// session状態はその呼び出し時点で既に確定している。続くACK／`get_status`の
/// 送信が失敗しても巻き戻らないため、`Result`にして呼び出し側から`outcome`を
/// 隠さない。送信の失敗は握りつぶさずlogへ残す（ACKと再試行する`get_status`は`log::warn!`、
/// 再試行しない`get_status`は`log::error!`）。`SessionCounters`へ
/// 計上されるかどうかは`SendError`のvariantによる
/// （`session.rs`の`encode_and_enqueue`参照）。
pub fn handle_boot(
    session: &mut Session,
    peer: &mut PeerSession,
    sid: u32,
    id: u32,
    boot: Boot,
    now_ms: u64,
) -> BootHandled {
    let handled = peer.handle_boot(sid, id, boot);

    if let Err(err) = session.send(handled.reply.clone(), now_ms) {
        log::warn!("bootへのACK送信に失敗した: {err}");
    }

    if matches!(handled.outcome, BootOutcome::Established { .. }) {
        try_send_get_status(session, peer, now_ms);
    }

    handled
}

/// `get_status`の送信が`err`で失敗したあと、予約を残すか。
///
/// **予約を残すか消すかは、この関数だけが決める。**理由は[`try_send_get_status`]の表が持つ。
const fn keeps_status_sync(err: &SendError) -> bool {
    matches!(
        err,
        SendError::Dropped | SendError::Stopped(_) | SendError::IdSpaceExhausted
    )
}

/// `get_status`の送信を1回試みる。成功すれば追跡を始める。失敗した場合は
/// [`keeps_status_sync`]で分け、予約を残すなら
/// [`PeerSession::mark_status_sync_pending`]で「まだ送れていない」と記録する。
///
/// [`handle_boot`]（初回の試行）と[`retry_due_requests`]（次tickでの再試行）の
/// 両方から呼ぶ、共通の一手である。
///
/// | `SendError` | 予約 | 理由 |
/// |---|---|---|
/// | `Dropped` | 残す | 送信queueが満杯だっただけであり、書き出せば空く |
/// | `Stopped` | 残す（以前と同じ） | この`Session`は停止から戻らない。**停止した`Session`へは[`retry_due_requests`]が再試行しない** |
/// | `IdSpaceExhausted` | 残す（以前と同じ） | `id`を払い出せず、この`Session`も停止する。扱いは`Stopped`と同じ |
/// | `Encode` | 消す | 同じ`get_status`をencodeし直しても、`Session`を替えても結果は変わらない |
/// | `EmptyPayload` | 消す | 内部経路では起こらない（`session.rs`の`encode_and_enqueue`）。起きた場合も入力が同じなら同じ結果になる |
///
/// `Dropped`以外は`log::error!`を出す（この呼び出し1回につき1行）。予約を消す場合は
/// [`PeerSession::clear_status_sync_pending`]を呼ぶ。**以前は停止した`Session`へも
/// 毎tick送り直し、`log::warn!`を出し続けた**（#472）。
/// `Dropped`で再試行する回数に上限は無い（queueが空くまで待つ）。
///
/// **`Stopped`／`IdSpaceExhausted`で予約を残すことと、session reset（§3.1）の後の扱いは
/// 変えていない。**変えたのは、停止した同じ`Session`へ送り直さないことだけである。
/// 残した予約を、別の`Session`を渡した[`retry_due_requests`]が送りうるのは以前と同じである。
/// `Encode`／`EmptyPayload`では予約を消すため、以前と違い、reset後の`Session`でも送らない。
/// reset後に`get_status`を送る順序（§10.2）と、`PeerSession`に残る旧sessionの要求の扱いは、
/// このcrateの呼び出し側が決める。
fn try_send_get_status(session: &mut Session, peer: &mut PeerSession, now_ms: u64) {
    match session.send(Message::GetStatus, now_ms) {
        Ok(status_id) => {
            // `Message::GetStatus`は必ず追跡対象に分類される
            // （`OutstandingKind::classify`参照）ため、`None`は来ない。
            // `note_sent`自身がstatus_sync_pendingを落とす（peer.rsのdoc参照）。
            let _ = peer.note_sent(status_id, Message::GetStatus, now_ms);
        }
        Err(err) if keeps_status_sync(&err) => {
            if matches!(err, SendError::Dropped) {
                log::warn!("get_statusの送信に失敗した。次のtickで再試行する: {err}");
            } else {
                log::error!(
                    "get_statusの送信に失敗した。このsessionは停止しているため、このsessionでは再試行しない: {err}"
                );
            }
            peer.mark_status_sync_pending();
        }
        Err(err) => {
            log::error!(
                "get_statusの送信に失敗した。送り直しても同じ結果になるため再試行しない: {err}"
            );
            peer.clear_status_sync_pending();
        }
    }
}

/// 応答待ちの要求のうちACK timeoutを超えたものを処理した結果。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RetryOutcome {
    /// 同じ`id`で再送した。
    Resent(u32, OutstandingKind),
    /// 再送しようとしたが、送信自体が失敗した。**どの条件で`SendError`のどの
    /// variantになるかは、ここへ再掲しない。**[`Session::resend`]の`# Errors`節が
    /// 正本であり、ここに複製すると片方だけ直る欠陥を作る（実際に一度作った）。
    ResendFailed(u32, OutstandingKind, SendError),
    /// 再送予算を使い切り、要求を取り下げた。
    ///
    /// **これは「wireへ実際に出た回数」の話ではない。**[`PeerSession::poll_outstanding`]は
    /// 再送を試みると決めた時点で`retries`を増やし、その後の送信が失敗しても
    /// 戻さない（[`Session::begin_reconnect`]が接続の成否を問わず試行を数えるのと
    /// 同じ形）。`GaveUp`単体では「一度もwireへ出ていない」と断定できない。
    /// それを見分けたい呼び出し側は、同じ`id`への過去の`RetryOutcome`を追う。
    ///
    /// `log::warn!`へも残す。返り値の`Vec`を呼び出し側が読まなくても、logには残る。
    GaveUp(u32, OutstandingKind),
}

/// 応答待ちの要求のうちACK timeoutを超えたものを、同じ`id`で再送する。
/// あわせて、`boot`確立直後の`get_status`送信が失敗していれば再試行する
/// （[`PeerSession::status_sync_pending`]）。
///
/// 呼び出し側が一定間隔で呼ぶ。timeoutの判定と取り下げは
/// [`PeerSession::poll_outstanding`]が行い、この関数はその結果を受けて
/// 実際に送信するだけである。再送・ACK timeoutの方針は
/// `session.config().retry()`から読む（[`Session::begin_reconnect`]が
/// `self.config.reconnect()`を内部で読むのと同じ形）。呼び出し側が別の方針を
/// ここへ渡して食い違わせる経路は作らない。
///
/// 送信失敗（[`RetryOutcome::ResendFailed`]）と取り下げ（[`RetryOutcome::GaveUp`]）は
/// どちらも`log::warn!`へ残す。握りつぶさない。
#[must_use = "再送・取り下げの結果を捨てると、失敗をlog以外では追えなくなる"]
pub fn retry_due_requests(
    session: &mut Session,
    peer: &mut PeerSession,
    now_ms: u64,
) -> Vec<RetryOutcome> {
    // **停止した`Session`へは送り直さない。**停止から戻らないため、毎tick失敗して
    // logを出し続けるだけになる（#472）。予約の扱いは`try_send_get_status`の表が持つ。
    if peer.status_sync_pending() && !matches!(session.state(), ConnectionState::Stopped(_)) {
        try_send_get_status(session, peer, now_ms);
    }

    // `resend`が`&mut session`を要するため、`config()`の借用をここで終える
    // （`RetryPolicy`は小さい値であり、cloneのcostは無視できる）。
    let policy = session.config().retry().clone();

    peer.poll_outstanding(now_ms, &policy)
        .into_iter()
        .map(|(id, action)| match action {
            OutstandingAction::Retry(kind, message) => match session.resend(id, message, now_ms) {
                Ok(()) => RetryOutcome::Resent(id, kind),
                Err(err) => {
                    log::warn!("id {id}の再送に失敗した: {err}");
                    RetryOutcome::ResendFailed(id, kind, err)
                }
            },
            OutstandingAction::GaveUp(kind) => {
                log::warn!("id {id}の応答待ちを取り下げた（再送予算を使い切った）");
                RetryOutcome::GaveUp(id, kind)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use deskcat_protocol::error::{DecodeError, ErrorCode};

    use super::keeps_status_sync;
    use crate::session::{SendError, StopReason};

    /// `SendError`の5種すべてについて、予約を残すかを固定する（#472）。
    ///
    /// **予約を残すか消すかは`keeps_status_sync`だけが決める**（`try_send_get_status`は
    /// その戻り値でmarkかclearを選ぶ）。`Encode`／`EmptyPayload`は`get_status`の送信を通しては
    /// 起こせないため、分類そのものを見る。
    #[test]
    fn every_send_error_has_a_decided_status_sync_disposition() {
        assert!(keeps_status_sync(&SendError::Dropped));
        assert!(keeps_status_sync(&SendError::Stopped(StopReason::Fatal)));
        assert!(keeps_status_sync(&SendError::IdSpaceExhausted));
        assert!(!keeps_status_sync(&SendError::Encode(DecodeError::new(
            ErrorCode::InvalidEnvelope,
            "test",
        ))));
        assert!(!keeps_status_sync(&SendError::EmptyPayload));
    }
}
