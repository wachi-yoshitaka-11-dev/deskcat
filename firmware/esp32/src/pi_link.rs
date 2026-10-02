//! Pi linkの受信の振り分けと、Pi linkのUARTへの書き込み（§8）。
//!
//! Pi linkを持つbuild（`bench-servo-test-17`以外のすべて。`#487`）で使う。main loopがPi linkの
//! UARTから読んだbyteを[`PiLink::on_bytes`]へ渡す。`PiLink`は行を組み立ててdecodeし、受信の順に
//! 次のとおり扱う。
//!
//! | 受けたもの | 扱い |
//! |---|---|
//! | `ack` | Envelopeの`sid`を照合し（下記）、`crate::boot_session::BootSession::on_ack`へ渡す |
//! | `hello` | `crate::protocol::PiSession::handle_hello`のACKを書く。受理した場合は、その後に`BootSession::on_hello_accepted`を呼ぶ（§5.1の手順4・5） |
//! | `ping` | `PiSession::handle_ping`のACKを書く |
//! | `get_status` | `PiSession::handle_get_status`のACKと`status`を書く（下記） |
//! | `boot` | `unknown_type`の拒否ACKを書く（§8の表「方向が逆のsession確立message」） |
//! | `status` | 応答しない（ESP32→Piのmessageである）。logで分類する |
//! | decodeで拒否した行 | oversizeの行から`(sid, id)`と`hello`／`ping`／`get_status`のtypeを復元できた場合は`line_too_long`の拒否ACKを書く（§7）。それ以外はlogで分類し、応答しない（§2の既知の逸脱） |
//!
//! 応答のEnvelopeの`(sid, id)`は`BootSession::next_envelope`が払い出す（`crate::boot_session`の
//! module doc「ESP32の`(sid, id)`の採番」）。
//!
//! # ACKのEnvelopeの`sid`（§6）
//!
//! `PiSession`が現在のPi `sid`を持つ間は、Envelopeの`sid`がそれと一致しないACKを`BootSession`へ
//! 渡さない。持たない間は照合しない。`hello`が届く前に`boot`のACKが届く場合（`hello`の喪失など）が
//! あるためである。
//!
//! # `get_status`への`status`の順序（§5.6）
//!
//! `get_status`への`status: ok`のACKと`status`は、1つのbufferへ連結し、1回の[`write_line`]で
//! 送信のring bufferへ積む。Pi linkのUARTへ書くのは、このmoduleの応答と`BootSession`の`boot`の
//! 送出だけであり、どちらもmain loopの同じtaskから呼ぶ。そのため、連結した2行の間に他の行は
//! 入らない。health snapshot（`main.rs`の`emit_health_snapshot`）はUART0のlogであり、Pi linkへ
//! `status`を送らない。
//!
//! # 実装していないもの
//!
//! - `ping`／`get_status`、`port_reopen`／`resync`の`hello`、拒否した`hello`の処理済みの結果を保持しない
//!   （§8の手順8・9。保持件数は`PROTO-TBD-005`）。同じ`(sid, id)`の再送は、もう一度処理する。
//! - §8.1／§8.2の流量制限と、`hello`の拒否ACKの保留table（`PROTO-TBD-012`）。
//! - `ProtocolCounters`を増やすこと（`health.rs`のmodule doc）。分類はlogだけであり、`get_status`へ返す
//!   `status`のcounterは0のままである。
//!
//! # 確かめていないこと
//!
//! **このmoduleと`crate::boot_session`、`crate::protocol`の振る舞いは、host testでも実機でも
//! 確かめていない。**`firmware/esp32`はhostのworkspaceから除外されており、`esp_idf_svc`へ依存する
//! ため、host testでは動かせない。実機の確認（`boot`→ACK、`hello`の後の`boot`の再送、`get_status`の
//! ACKと`status`の順序、受信のring bufferが溢れないこと）はIssue #446と#12が追跡する。

use deskcat_protocol::{
    encode_line, Ack, AckStatus, Cause, ErrorCode, Frame, LineReceiver, Message, Outcome, Rejection,
};
use esp_idf_svc::hal::uart::UartDriver;

use crate::boot_session::{BootSession, HelloAccepted};
use crate::health::Health;
use crate::protocol::{HelloOutcome, PiSession};

/// Pi linkの受信の状態。
pub struct PiLink {
    receiver: LineReceiver,
    pi: PiSession,
}

impl PiLink {
    /// 未確立のPi sessionで作る。
    pub fn new() -> Self {
        Self {
            receiver: LineReceiver::with_protocol_limit(),
            pi: PiSession::new(),
        }
    }

    /// Pi linkのUARTから読めた生byteを渡す。
    pub fn on_bytes(
        &mut self,
        buf: &[u8],
        boot: &mut BootSession,
        health: &Health,
        uart: &mut UartDriver<'_>,
    ) {
        // `drain`のclosureは`&mut self`を借りられないため、結果を受信の順に集めてから扱う。
        let mut outcomes: Vec<Outcome> = Vec::new();
        self.receiver.drain(buf, |outcome| outcomes.push(outcome));
        for outcome in outcomes {
            match outcome {
                Outcome::Frame(frame) => self.on_frame(frame, boot, health, uart),
                Outcome::Rejected(rejection) => on_rejected(&rejection, boot, health, uart),
            }
        }
    }

    fn on_frame(
        &mut self,
        frame: Frame,
        boot: &mut BootSession,
        health: &Health,
        uart: &mut UartDriver<'_>,
    ) {
        let Frame { envelope, message } = frame;
        let (sid, id) = (envelope.sid, envelope.id);
        match message {
            Message::Ack(ack) => {
                if let Some(pi_sid) = self.pi.pi_sid() {
                    if sid != pi_sid {
                        log::error!(
                            "pi_rx_ack_sid_mismatch sid={sid} pi_sid={pi_sid} reply_sid={} reply_to={}",
                            ack.reply_sid,
                            ack.reply_to
                        );
                        return;
                    }
                }
                boot.on_ack(ack, health, uart);
            }
            Message::Hello(hello) => {
                let handled = self.pi.handle_hello(sid, id, &hello);
                log::info!(
                    "pi_rx_hello sid={sid} id={id} reason={:?} outcome={:?}",
                    hello.reason,
                    handled.outcome
                );
                send(vec![handled.reply], boot, health, uart);
                let accepted = match handled.outcome {
                    HelloOutcome::Established { .. } => Some(HelloAccepted::Transition),
                    HelloOutcome::Maintained => Some(HelloAccepted::Maintained),
                    _ => None,
                };
                if let Some(accepted) = accepted {
                    boot.on_hello_accepted(accepted, health, uart);
                }
            }
            Message::Ping => {
                send(vec![self.pi.handle_ping(sid, id)], boot, health, uart);
            }
            Message::GetStatus => {
                let (ack, status) = self.pi.handle_get_status(sid, id, health.to_status());
                let mut replies = vec![ack];
                replies.extend(status);
                send(replies, boot, health, uart);
            }
            Message::Boot(_) => {
                log::error!("pi_rx_reverse_boot sid={sid} id={id}");
                send(
                    vec![rejection_ack(sid, id, ErrorCode::UnknownType)],
                    boot,
                    health,
                    uart,
                );
            }
            other => {
                log::error!(
                    "pi_rx_unhandled_frame type={} sid={sid} id={id}",
                    other.type_str()
                );
            }
        }
    }
}

/// decodeで拒否した行を扱う（module docの表）。
fn on_rejected(
    rejection: &Rejection,
    boot: &mut BootSession,
    health: &Health,
    uart: &mut UartDriver<'_>,
) {
    match rejection.cause() {
        Cause::InvalidUtf8 { valid_up_to } => {
            log::error!("pi_rx_invalid_utf8 valid_up_to={valid_up_to}");
        }
        Cause::Decode => log::error!("pi_rx_decode_rejected code={}", rejection.code().as_str()),
        Cause::Oversize => {
            log::error!(
                "pi_rx_oversize_line identity={:?} type={:?}",
                rejection.identity(),
                rejection.type_name()
            );
            if let (Some((sid, id)), Some("hello" | "ping" | "get_status")) =
                (rejection.identity(), rejection.type_name())
            {
                send(
                    vec![rejection_ack(sid, id, ErrorCode::LineTooLong)],
                    boot,
                    health,
                    uart,
                );
            }
        }
        _ => log::error!("pi_rx_rejected_unknown_cause"),
    }
}

fn rejection_ack(reply_sid: u32, reply_to: u32, code: ErrorCode) -> Message {
    Message::Ack(Ack {
        reply_sid,
        reply_to,
        status: AckStatus::Rejected,
        code: Some(code),
        detail: None,
    })
}

/// `messages`を順に新しい`(sid, id)`でencodeし、連結して1回の[`write_line`]で書く。
///
/// `id`を払い出せない、またはencodeできないmessageがあれば、そこから後ろは書かない
/// （`get_status`のACKを書けないときに、`status`だけを書かないため。§5.6）。
fn send(
    messages: Vec<Message>,
    boot: &mut BootSession,
    health: &Health,
    uart: &mut UartDriver<'_>,
) {
    let mut out = String::new();
    let mut sent: Vec<(&'static str, u32)> = Vec::new();
    for message in messages {
        let Some(envelope) = boot.next_envelope(health.uptime_ms()) else {
            break;
        };
        let type_name = message.type_str();
        let id = envelope.id;
        match encode_line(&Frame::new(envelope, message)) {
            Ok(line) => {
                out.push_str(&line);
                sent.push((type_name, id));
            }
            Err(err) => {
                log::error!("pi_tx_encode_failed type={type_name} id={id} error={err}");
                break;
            }
        }
    }
    if !out.is_empty() && write_line(uart, out.as_bytes()) {
        log::info!("pi_tx frames={sent:?} bytes={}", out.len());
    }
}

/// encode済みの1行、または連結した複数行を、Pi linkのUARTの送信のring bufferへ積む。
/// 全byteを積めたら`true`を返す。
pub fn write_line(uart: &mut UartDriver<'_>, bytes: &[u8]) -> bool {
    // §2は行の分断を禁じている。この保証は、esp-idf-hal
    // `UartDriver::write`が呼ぶ`uart_write_bytes`→`uart_tx_all`が
    // `portMAX_DELAY`でblockし、1回の呼び出しで渡した全byteを
    // tx ring bufferへ積み終える（wireへ送り終えるまでではない）か
    // （成功時は常に`Ok(bytes.len())`）、入力検証エラーで即座に
    // 失敗するかのどちらかであること（`esp_driver_uart/src/uart.c`の
    // `uart_tx_all`で確認済み。部分書き込みで戻る経路が無い）に依る。
    // **このloopが部分書き込みを繰り返して完了させているわけではない。**
    // `Ok(0)`／`Err`は現状の実装では実質到達しないが、型として
    // 有り得る以上、到達したら送出を諦める（行の途中で戻り値を無視して
    // 送り続けることはしない）。
    let mut written = 0;
    while written < bytes.len() {
        match uart.write(&bytes[written..]) {
            Ok(0) => {
                log::error!("pi_uart_write_stalled written={written}");
                return false;
            }
            Ok(n) => written += n,
            Err(err) => {
                log::error!("pi_uart_write_failed error={err} written={written}");
                return false;
            }
        }
    }
    true
}
