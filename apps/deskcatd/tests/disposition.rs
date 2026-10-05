//! 要求の種類 × 返りうる結果 の全数が、決まった振る舞いを持つことの固定。
//!
//! 期待は`daemon.rs`のmodule docの表を、ここで独立に書き写したものである。表を直したら、
//! ここも直す（食い違えばtestが落ちる）。`ErrorCode::ALL`を走査するので、codeが増えると
//! このtestが「期待を足せ」と落ちる。

use deskcat_protocol::ErrorCode;
use deskcatd::disposition::{Answer, Disposition, Request, disposition};

use Disposition::{
    AskAgain, ExitFatal, Proceed, RestartCollision, Resume, RetryHelloSameId, RetryLater,
    StopSending,
};

/// 表の「応答なし」と「ok」の列。
#[test]
fn ok_and_no_response_cells() {
    for request in [Request::Hello, Request::GetStatus, Request::SetExpression] {
        assert_eq!(disposition(request, Answer::Ok), Proceed, "{request:?}");
    }
    assert_eq!(
        disposition(Request::Hello, Answer::NoResponse),
        RetryHelloSameId
    );
    assert_eq!(
        disposition(Request::GetStatus, Answer::NoResponse),
        AskAgain
    );
    assert_eq!(
        disposition(Request::SetExpression, Answer::NoResponse),
        RetryLater
    );
}

/// `hello`の11 codeを1つずつ。
fn expected_hello(code: ErrorCode) -> Disposition {
    match code {
        ErrorCode::StaleSession => RestartCollision,
        ErrorCode::RateLimited => RetryHelloSameId,
        ErrorCode::InvalidPayload
        | ErrorCode::UnsupportedVersion
        | ErrorCode::UnknownType
        | ErrorCode::InvalidEnvelope
        | ErrorCode::OutOfRange
        | ErrorCode::LineTooLong
        | ErrorCode::Busy
        | ErrorCode::HardwareUnavailable
        | ErrorCode::DuplicateExpired => ExitFatal,
        other => panic!("表に無いcode: {other:?}。表とこのtestに行を足す"),
    }
}

fn expected_get_status(code: ErrorCode) -> Disposition {
    match code {
        ErrorCode::StaleSession => Resume,
        ErrorCode::Busy | ErrorCode::RateLimited | ErrorCode::DuplicateExpired => AskAgain,
        ErrorCode::UnsupportedVersion
        | ErrorCode::UnknownType
        | ErrorCode::InvalidEnvelope
        | ErrorCode::InvalidPayload
        | ErrorCode::OutOfRange
        | ErrorCode::LineTooLong
        | ErrorCode::HardwareUnavailable => ExitFatal,
        other => panic!("表に無いcode: {other:?}。表とこのtestに行を足す"),
    }
}

fn expected_set_expression(code: ErrorCode) -> Disposition {
    match code {
        ErrorCode::StaleSession => Resume,
        ErrorCode::Busy | ErrorCode::RateLimited | ErrorCode::DuplicateExpired => RetryLater,
        ErrorCode::UnsupportedVersion
        | ErrorCode::UnknownType
        | ErrorCode::InvalidEnvelope
        | ErrorCode::InvalidPayload
        | ErrorCode::OutOfRange
        | ErrorCode::LineTooLong
        | ErrorCode::HardwareUnavailable => StopSending,
        other => panic!("表に無いcode: {other:?}。表とこのtestに行を足す"),
    }
}

#[test]
fn every_code_for_every_request_has_the_documented_disposition() {
    for code in ErrorCode::ALL {
        let rejected = Answer::Rejected(Some(code));
        assert_eq!(
            disposition(Request::Hello, rejected),
            expected_hello(code),
            "hello × {code:?}"
        );
        assert_eq!(
            disposition(Request::GetStatus, rejected),
            expected_get_status(code),
            "get_status × {code:?}"
        );
        assert_eq!(
            disposition(Request::SetExpression, rejected),
            expected_set_expression(code),
            "set_expression × {code:?}"
        );
    }
}

/// 仕様は拒否に`code`を必須とする。欠けていても、止まらず安全側に倒す。
#[test]
fn a_rejection_without_a_code_falls_to_the_safe_side() {
    let rejected = Answer::Rejected(None);
    assert_eq!(disposition(Request::Hello, rejected), ExitFatal);
    assert_eq!(disposition(Request::GetStatus, rejected), ExitFatal);
    assert_eq!(disposition(Request::SetExpression, rejected), StopSending);
}

/// `Resume`を返すのは、通常commandの`stale_session`だけである（`hello`は返さない）。
#[test]
fn only_stale_session_on_a_command_resumes() {
    for code in ErrorCode::ALL {
        let resumes = |request| disposition(request, Answer::Rejected(Some(code))) == Resume;
        assert!(!resumes(Request::Hello));
        assert_eq!(resumes(Request::GetStatus), code == ErrorCode::StaleSession);
        assert_eq!(
            resumes(Request::SetExpression),
            code == ErrorCode::StaleSession
        );
    }
}
