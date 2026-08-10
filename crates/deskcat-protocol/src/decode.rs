//! 1 lineのdecodeとencode。
//!
//! 検証の順序は`docs/protocol/esp32-pi-protocol.md` §8手順7および§5.1
//! 「遷移は完全な検証を通してから確定する」の手順1〜3に一致させる。
//! **順序自体がconformanceの対象**であり、入れ替えると返すべきcodeが変わる。

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::envelope::{Envelope, Frame};
use crate::error::{DecodeError, ErrorCode};
use crate::limits;
use crate::message::{Ack, Boot, Hello, Message, Status};

/// envelopeだけを解釈し、payloadは未解釈のまま保持する中間表現。
///
/// payloadを[`RawValue`]で受けるのは、`unknown_type`と`invalid_payload`を区別するためである。
/// serdeのadjacently tagged enum（`#[serde(tag = "type", content = "payload")]`）では
/// 失敗理由が「どのvariantにも一致しない」に潰れ、§8手順7が要求する
/// 「typeに対応するschemaが解決できない時点で拒否する」を表現できない。
#[derive(Deserialize)]
struct RawEnvelope<'a> {
    v: u16,
    sid: u32,
    id: u32,
    ts_ms: u64,
    /// `&str`ではなく[`Cow`]で受ける。escape sequenceを含むJSON stringは借用できず、
    /// `&str`で受けると`invalid_envelope`になってしまう。escapeの有無で同じtypeの
    /// 判定が変わるのは、仕様上の根拠がない。
    #[serde(rename = "type", borrow)]
    type_name: Cow<'a, str>,
    #[serde(borrow)]
    payload: &'a RawValue,
}

/// encode時のwire表現。field順が§3の例と一致するよう宣言順を保つ。
#[derive(Serialize)]
struct WireEnvelope<'a> {
    v: u16,
    sid: u32,
    id: u32,
    ts_ms: u64,
    #[serde(rename = "type")]
    type_name: &'a str,
    payload: &'a RawValue,
}

/// 1 lineをdecodeする。
///
/// 入力は改行を含んでも含まなくてもよい。含まない場合も、[`limits::MAX_LINE_BYTES`]の
/// 判定では改行1 byteを加えて数える。§2の「受信可能なline ending」に従い、末尾の`\n`と
/// その直前の`\r`は取り除く。
///
/// byte列としての受信、分割されたlineの結合、invalid UTF-8の分類はこのcrateの範囲外であり、
/// firmware側のincremental receiver（Issue #10）が扱う。
///
/// # 検証順序
///
/// | # | 判定 | 失敗時のcode |
/// |---|---|---|
/// | 1 | 改行を含むline長 | [`ErrorCode::LineTooLong`] |
/// | 2 | JSONとして解析でき、envelope fieldが型どおり | [`ErrorCode::InvalidEnvelope`] |
/// | 3 | `payload`がobject | [`ErrorCode::InvalidEnvelope`] |
/// | 4 | `v`が対応major version | [`ErrorCode::UnsupportedVersion`] |
/// | 5 | `type`が既知 | [`ErrorCode::UnknownType`] |
/// | 6 | type固有payload schema | [`ErrorCode::InvalidPayload`] |
/// | 7 | 上限のある値の範囲 | [`ErrorCode::OutOfRange`] |
///
/// # Errors
///
/// 上表のいずれかに違反した場合、対応する[`ErrorCode`]を持つ[`DecodeError`]を返す。
pub fn decode_line(line: &str) -> Result<Frame, DecodeError> {
    let wire_len = if line.ends_with('\n') {
        line.len()
    } else {
        line.len() + 1
    };
    if wire_len > limits::MAX_LINE_BYTES {
        return Err(DecodeError::new(
            ErrorCode::LineTooLong,
            format!(
                "line is {wire_len} bytes including the newline, limit is {}",
                limits::MAX_LINE_BYTES
            ),
        ));
    }

    let body = line.strip_suffix('\n').unwrap_or(line);
    let body = body.strip_suffix('\r').unwrap_or(body);

    let raw: RawEnvelope<'_> = serde_json::from_str(body)
        .map_err(|err| DecodeError::new(ErrorCode::InvalidEnvelope, err.to_string()))?;

    if !payload_is_object(raw.payload) {
        return Err(DecodeError::new(
            ErrorCode::InvalidEnvelope,
            "`payload` must be an object; use `{}` instead of `null` for an empty payload",
        ));
    }

    if raw.v != limits::PROTOCOL_VERSION {
        return Err(DecodeError::new(
            ErrorCode::UnsupportedVersion,
            format!(
                "`v` is {}, this implementation accepts {}",
                raw.v,
                limits::PROTOCOL_VERSION
            ),
        ));
    }

    let message = decode_payload(raw.type_name.as_ref(), raw.payload)?;
    message.check_bounds()?;

    Ok(Frame::new(
        Envelope {
            v: raw.v,
            sid: raw.sid,
            id: raw.id,
            ts_ms: raw.ts_ms,
        },
        message,
    ))
}

/// `type`を既知typeへ解決し、そのschemaでpayloadをdeserializeする。
fn decode_payload(type_name: &str, payload: &RawValue) -> Result<Message, DecodeError> {
    match type_name {
        "boot" => parse::<Boot>(payload).map(Message::Boot),
        "hello" => parse::<Hello>(payload).map(Message::Hello),
        "ping" => parse_empty(payload).map(|()| Message::Ping),
        "get_status" => parse_empty(payload).map(|()| Message::GetStatus),
        "status" => parse::<Status>(payload).map(|status| Message::Status(Box::new(status))),
        "ack" => {
            let ack = parse::<Ack>(payload)?;
            ack.check_shape()?;
            Ok(Message::Ack(ack))
        }
        unknown => Err(DecodeError::new(
            ErrorCode::UnknownType,
            format!("unknown message type `{unknown}`"),
        )),
    }
}

/// type固有schemaでpayloadを解釈する。未知の追加fieldは§3に従って無視する。
fn parse<T: serde::de::DeserializeOwned>(payload: &RawValue) -> Result<T, DecodeError> {
    serde_json::from_str(payload.get())
        .map_err(|err| DecodeError::new(ErrorCode::InvalidPayload, err.to_string()))
}

/// payloadを持たないtypeを検査する。objectであることは呼び出し前に確認済みである。
fn parse_empty(payload: &RawValue) -> Result<(), DecodeError> {
    serde_json::from_str::<serde::de::IgnoredAny>(payload.get())
        .map(|_| ())
        .map_err(|err| DecodeError::new(ErrorCode::InvalidPayload, err.to_string()))
}

/// raw payloadがJSON objectかを判定する。
fn payload_is_object(payload: &RawValue) -> bool {
    payload.get().trim_start().starts_with('{')
}

/// frameを1 lineへencodeする。末尾に`\n`を付ける。
///
/// field順は§3の例と同じ`v`、`sid`、`id`、`ts_ms`、`type`、`payload`になる。
///
/// # Errors
///
/// encode結果が[`limits::MAX_LINE_BYTES`]を超える場合は[`ErrorCode::LineTooLong`]を返す。
/// payloadのserializeに失敗した場合は[`ErrorCode::InvalidPayload`]を返す。
pub fn encode_line(frame: &Frame) -> Result<String, DecodeError> {
    let payload = encode_payload(&frame.message)?;
    let payload = RawValue::from_string(payload)
        .map_err(|err| DecodeError::new(ErrorCode::InvalidPayload, err.to_string()))?;

    let wire = WireEnvelope {
        v: frame.envelope.v,
        sid: frame.envelope.sid,
        id: frame.envelope.id,
        ts_ms: frame.envelope.ts_ms,
        type_name: frame.message.type_str(),
        payload: &payload,
    };

    let mut line = serde_json::to_string(&wire)
        .map_err(|err| DecodeError::new(ErrorCode::InvalidPayload, err.to_string()))?;
    line.push('\n');

    if line.len() > limits::MAX_LINE_BYTES {
        return Err(DecodeError::new(
            ErrorCode::LineTooLong,
            format!(
                "encoded line is {} bytes including the newline, limit is {}",
                line.len(),
                limits::MAX_LINE_BYTES
            ),
        ));
    }

    Ok(line)
}

/// type固有payloadをJSON文字列へserializeする。
fn encode_payload(message: &Message) -> Result<String, DecodeError> {
    let result = match message {
        Message::Boot(boot) => serde_json::to_string(boot),
        Message::Hello(hello) => serde_json::to_string(hello),
        Message::Ping | Message::GetStatus => Ok("{}".to_owned()),
        Message::Status(status) => serde_json::to_string(status),
        Message::Ack(ack) => serde_json::to_string(ack),
    };
    result.map_err(|err| DecodeError::new(ErrorCode::InvalidPayload, err.to_string()))
}
