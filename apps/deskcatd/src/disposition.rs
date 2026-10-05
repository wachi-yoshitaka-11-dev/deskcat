//! Piが送った要求への結果を、その後の振る舞いへ写す**1つの分類**。
//!
//! 表（要求の種類 × 返りうる結果 × 振る舞い × 根拠の節）は[`crate::daemon`]のmodule docが持つ。
//! **この関数は、その表のセルを写したものである。**表と関数を別々に直さない。
//! 全`ErrorCode`と全要求の組み合わせが決まった振る舞いを持つことは、testが
//! `ErrorCode::ALL`を走査して固定している（`tests/disposition.rs`）。

use deskcat_protocol::ErrorCode;

/// Piが送る要求の種類。`ping`は送らない（`PROTO-TBD-010`）ため無い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Request {
    /// `startup`の`hello`（`port_reopen`と`resync`の`hello`は送らない）。
    Hello,
    /// `get_status`（§5.6）。
    GetStatus,
    /// `set_expression`（§5.2）。
    SetExpression,
}

/// 要求への結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Answer {
    /// `status: ok`のACK。
    Ok,
    /// `status: rejected`のACK。`code`は仕様上必須だが、欠けていても扱いを決める。
    Rejected(Option<ErrorCode>),
    /// ACK timeoutと再送の予算を使い切っても、ACKが無い。
    NoResponse,
}

/// 結果を受けた後の振る舞い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Disposition {
    /// 受理された。要求ごとの次の手へ進む。
    Proceed,
    /// 同じ`sid`の`startup`の`hello`を、新しい`id`で送り直す（再開。§3.1、§10.2）。
    Resume,
    /// `sid`の衝突。processを終了して選び直す（§3.1）。
    RestartCollision,
    /// `hello`を、同じ`(sid, id)`で再送する（有限のbudget内。§5.1）。`rate_limited`の拒否の後は
    /// cooldownを待ち、ACKが無い場合はACK timeoutを待つ。
    RetryHelloSameId,
    /// `get_status`を、cooldownの後に新しい`id`で送り直す（§5.6）。待てば受け付けられる拒否の連続には
    /// 上限を置かない。無応答の連続だけは、`daemon`が数えて上限を置く。
    AskAgain,
    /// `set_expression`を、cooldownの後に新しい`id`で送り直す（§9）。一時的な拒否の連続は有限の回数で
    /// 止める（`ExpressionSync`）。無応答の連続も、`SILENT_RETRY_LIMIT`を超えたら、欲しい表情が変わるまで
    /// 送らない。
    RetryLater,
    /// 表情の内容が受け付けられない。欲しい表情が変わるまで同じ表情を送らない。
    StopSending,
    /// processを終了する（終了コード1）。`hello`と`get_status`の決まった拒否。
    ExitFatal,
}

/// `request`への`answer`の振る舞い。
#[must_use]
pub const fn disposition(request: Request, answer: Answer) -> Disposition {
    match (request, answer) {
        // ACK timeoutまでACKが無い。同じ`(sid, id)`で再送する（§5.1。有限のbudget内）。
        // `ACK`の喪失で`sid`を選び直さず、新しい`id`の`startup`も送らない（既に処理済みなら
        // 「`startup`なのに`sid`が現在のPi sessionと同じ」になる）。
        (Request::Hello, Answer::NoResponse) => Disposition::RetryHelloSameId,
        (_, Answer::Ok) => Disposition::Proceed,

        (Request::Hello, Answer::Rejected(code)) => hello_rejected(code),
        (
            Request::GetStatus | Request::SetExpression,
            Answer::Rejected(Some(ErrorCode::StaleSession)),
        ) => Disposition::Resume,
        // `get_status`は状態を読むだけで、新しい`id`で送り直しても二重実行の問題は起きない（§5.6）。
        // 無応答は、新しい`id`で送り直す（連続は`daemon`が数えて上限を置く）。
        (Request::GetStatus, Answer::NoResponse) => Disposition::AskAgain,
        // 拒否は`set_expression`と同じ規則である。待てば受け付けられる拒否だけを送り直し、
        // 決まった拒否（再送でも結果が変わらない）は、繰り返さずに終了する。§5.6が許すのは、
        // 実stateが要るときに新しい`id`で送ることであり、決まった拒否を無期限に繰り返すことではない。
        (Request::GetStatus, Answer::Rejected(code)) => get_status_rejected(code),

        (Request::SetExpression, Answer::Rejected(code)) => set_expression_rejected(code),
        // 状態設定commandはidempotentである（§9）ので、結果が不明でも新しい`id`で送り直してよい。
        (Request::SetExpression, Answer::NoResponse) => Disposition::RetryLater,
    }
}

const fn hello_rejected(code: Option<ErrorCode>) -> Disposition {
    match code {
        // §3.1。`hello`自身の`stale_session`は`sid`の衝突である。
        Some(ErrorCode::StaleSession) => Disposition::RestartCollision,
        // §5.1の拒否code表。終端ではなく、同じ`(sid, id)`で再送する。
        Some(ErrorCode::RateLimited) => Disposition::RetryHelloSameId,
        // 表の`invalid_payload`と`unsupported_version`は終端である（§5.1の拒否code表）。
        // 終了すれば、processの再起動で新しい`sid`の`startup`になり（§3）、同じ`sid`のまま
        // commandを送らない。表に無い他のcodeは、
        // 再評価しても変わらない拒否（§5.1）として終端に倒す。`busy`と`duplicate_expired`も、
        // `hello`について仕様が再送を定めていないので止める。
        _ => Disposition::ExitFatal,
    }
}

const fn get_status_rejected(code: Option<ErrorCode>) -> Disposition {
    match code {
        // 待てば受け付けられる（§7の`busy`）、通常commandの`rate_limited`、結果が不明なだけの
        // `duplicate_expired`。cooldownの後に、新しい`id`で再要求する（上限は置かない）。
        Some(ErrorCode::Busy | ErrorCode::RateLimited | ErrorCode::DuplicateExpired) => {
            Disposition::AskAgain
        }
        // 決まった拒否（`code`が無い違反を含む）。processを終了する。
        _ => Disposition::ExitFatal,
    }
}

const fn set_expression_rejected(code: Option<ErrorCode>) -> Disposition {
    match code {
        // 待てば受け付けられる（§7の`busy`）。`rate_limited`は通常commandでは最終的な拒否であり、
        // cooldownの後に改めて要求する場合は新しい`id`を使う（§9）。`duplicate_expired`は
        // 結果が不明なだけで、idempotentなので新しい`id`で送り直してよい。
        Some(ErrorCode::Busy | ErrorCode::RateLimited | ErrorCode::DuplicateExpired) => {
            Disposition::RetryLater
        }
        // 内容の拒否。`hardware_unavailable`は「待っても受け付けられない」（§7）。
        // `unknown_type`のACKは、現行のfirmwareからは返らない（`firmware/esp32/src/pi_link.rs`の
        // `other`腕は`set_expression`に応答しない。処理を足すのは#21）。返るなら、内容の拒否として止める。
        _ => Disposition::StopSending,
    }
}
