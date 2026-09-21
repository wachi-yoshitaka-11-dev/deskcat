//! [`Session`]（transportとPi自身の送信）と[`PeerSession`]（ESP32側の状態）を
//! またいで、送信を伴う判断を行う。
//!
//! ここでやるのは2つだけである。
//!
//! - `boot`を1件処理してACKを送り返す（outcomeによらず常に。§4.1）。ACKの
//!   送信自体が失敗しても、この関数は再試行しない（回復はESP32側の`boot`再送に
//!   頼る）。確立した場合は続けて現在状態を要求し（§10.1 step1〜4）、
//!   こちらの送信が失敗した場合だけ、送れるまで次回以降のtickで再試行する
//! - 応答待ちの要求がACK timeoutを超えたら、同じ`id`で再送する（§9）
//!
//! **desired stateとの比較（§10.1 step6）や、状態設定commandの送出（step7）は
//! ここに含まない。**「望ましい状態」はdomain層の概念であり、`lib.rs`の
//! module docが定める「domain動作はこのcrateに入らない」という範囲の外にある。

use deskcat_protocol::{Boot, Message};

use crate::peer::{BootHandled, BootOutcome, OutstandingAction, OutstandingKind, PeerSession};
use crate::session::{SendError, Session};

/// `boot`を1件処理し、ACKを送り返す。新しいESP32 sessionを確立した場合は、
/// 続けて`get_status`を送って現在状態を要求する（§10.1 step1〜4）。
///
/// `sid`／`id`／`boot`は、受信した`boot` frameからそのまま渡す。`now_ms`は
/// 送信に使う時刻（送信側のuptime。wall-clock timeではない、§3）である。
///
/// **常に`BootHandled`を返す。**[`PeerSession::handle_boot`]自体は失敗せず、
/// session状態はその呼び出し時点で既に確定している。続くACK／`get_status`の
/// 送信が失敗しても巻き戻らないため、`Result`にして呼び出し側から`outcome`を
/// 隠さない。送信の失敗は握りつぶさず`log::warn!`へ残す。`SessionCounters`へ
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

/// `get_status`の送信を1回試みる。成功すれば追跡を始め、失敗すれば
/// [`PeerSession::mark_status_sync_pending`]で「まだ送れていない」と記録する。
///
/// [`handle_boot`]（初回の試行）と[`retry_due_requests`]（次tickでの再試行）の
/// 両方から呼ぶ、共通の一手である。
///
/// **この再試行に回数の上限は無い。**`session`が恒久的に停止している場合
/// （`StopReason`参照）も呼び出し側が呼び続ける限り毎tick`log::warn!`が出る。
/// このcrateはtimeout／停止状態を見て自動で呼び出しを止める機構を持たない。
/// 呼び出し側のloopが止める設計であることに依存している。
fn try_send_get_status(session: &mut Session, peer: &mut PeerSession, now_ms: u64) {
    match session.send(Message::GetStatus, now_ms) {
        Ok(status_id) => {
            // `Message::GetStatus`は必ず追跡対象に分類される
            // （`OutstandingKind::classify`参照）ため、`None`は来ない。
            // `note_sent`自身がstatus_sync_pendingを落とす（peer.rsのdoc参照）。
            let _ = peer.note_sent(status_id, Message::GetStatus, now_ms);
        }
        Err(err) => {
            log::warn!("get_statusの送信に失敗した。次のtickで再試行する: {err}");
            peer.mark_status_sync_pending();
        }
    }
}

/// 応答待ちの要求のうちACK timeoutを超えたものを処理した結果。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RetryOutcome {
    /// 同じ`id`で再送した。
    Resent(u32, OutstandingKind),
    /// 再送しようとしたが、送信自体が失敗した。どの`SendError`になりうるかは
    /// [`Session::resend`]のdocが持つ（`stopped`と`link_connected`の両方を
    /// 満たしたときだけ`SendError::Stopped`を返す。それ以外の失敗要因は
    /// `resend`の`# Errors`節を参照）。
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
    if peer.status_sync_pending() {
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
