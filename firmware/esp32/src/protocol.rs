//! Pi peer sessionの状態（ESP32側）。
//!
//! `crates/deskcat-serial`の`peer`module（Piがtrackする「ESP32 peer」の鏡像）と同じ形で、
//! ESP32がtrackする「Pi peer」の状態を持つ。仕様の正本は
//! `docs/protocol/esp32-pi-protocol.md`の§3.1・§5.1・§5.6・§5.7・§6・§8であり、
//! `crates/deskcat-serial/src/peer.rs`のmodule docと同じ理由で、単位時間あたりの
//! 受理上限、session遷移budget、cooldown（`PROTO-TBD-012`）、retired session保持件数
//! （`PROTO-TBD-011`）は**ここでは実装しない。**
//!
//! # 実機への配線について
//!
//! **この型はbyte列やUART peripheralを持たない。**`Hello`／`Ping`／`GetStatus`を
//! 受け取り、返すべき[`Message`]を返すだけである。実serial linkからこの型へ
//! byteを渡す配線は、**GPIO割り当てではなく、UART0の出力先の未決で止まっている。**
//! `docs/hardware/gpio-assignment.md`の`Pi–ESP32間のtransport`節が確定させているとおり、
//! Pi linkはUSB serialであり、ESP32board上のUSB-UARTブリッジICが内部でUART0
//! （GPIO1／GPIO3）へ接続する。GPIO headerへの配線は無く、GPIO割り当ての承認は要らない。
//! **一方で同文書のpin表はUART0を`firmware flashingとdebug log専用`と定めている。**
//! ESP loggerの出力は既にこのUART0（＝Pi linkと同じ物理line）へ出ており、そこへ
//! protocolのJSON Lines streamを重ねるとlogとprotocol messageが同じbyte streamで
//! 混ざる。**この分離方法（loggerの出力先を変えるか、protocol専用に道を分けるか）が
//! 未決であり、それがこの型を実UARTへ配線していない理由である。**
//! この点は`crates/deskcat-serial`側の`SerialDevice`の実機確認が[Issue #11]の後半に
//! 残っているのと対になる。
//!
//! ESP32自身の`sid`の生成方法は`PROTO-TBD-011`が未確定であり、この型は決めない
//! （`crates/deskcat-serial`の`Session::new`が`sid`を呼び出し側から受け取るのと同じ
//! 設計判断である）。呼び出し側が選んだ値を渡す。
//!
//! [Issue #11]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11

use std::collections::VecDeque;

use deskcat_protocol::{Ack, AckStatus, ErrorCode, Hello, HelloReason, Message, Status};

/// retired session集合の上限。**暫定値であり、確定値ではない。**
///
/// 正本は`PROTO-TBD-011`。`crates/deskcat-serial/src/peer.rs`の
/// `RETIRED_CAPACITY`と同じ理由の仮の値である。
const RETIRED_CAPACITY: usize = 4;

/// `hello`を拒否した、または`ping`／`get_status`をsession未確立で受けた理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PiRejection {
    /// 現在承認していない`sid`（retiredまたは未知）からのmessage（§7、§5.1優先順位1・4）。
    StaleSession,
    /// `reason`と`sid`の組が§5.1の整合規則を満たさない
    /// （`startup`なのに現在のPi sessionと同じ`sid`、または`port_reopen`／`resync`なのに
    /// 異なる`sid`）。
    InvalidPayload,
}

impl PiRejection {
    /// 相手へ返す[`ErrorCode`]。
    #[must_use]
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::StaleSession => ErrorCode::StaleSession,
            Self::InvalidPayload => ErrorCode::InvalidPayload,
        }
    }
}

/// `hello`を処理した結果。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HelloOutcome {
    /// 新しいPi sessionを確立した（`reason: startup`、未知の`sid`）。
    Established {
        /// 新しいPi `sid`。
        sid: u32,
    },
    /// 現在sessionを維持する制御message（`port_reopen`／`resync`、現在の`sid`）として受理した。
    Maintained,
    /// 現在sessionの`hello`再送（duplicate）。session状態は変えていない。
    Replayed,
    /// 拒否した。session状態は変えていない。
    Rejected(PiRejection),
}

/// `hello`を処理した結果と、相手へ返すACK。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelloHandled {
    /// 何が起きたか。
    pub outcome: HelloOutcome,
    /// 相手へ返すACK。
    pub reply: Message,
}

/// ESP32がtrackするPi peer sessionの状態。
#[derive(Debug)]
pub struct PiSession {
    /// 現在承認しているPi `sid`。最初の`hello`を受けるまでは`None`。
    pi_sid: Option<u32>,
    /// 直前まで有効だった`sid`の上限付き集合（§3.1、§5.1）。
    retired: VecDeque<u32>,
    /// 現在sessionで処理済みの`hello`の`(id, 返したack)`。1 sessionにつき、正規の
    /// `hello`は`startup`の1件だけであり（`port_reopen`／`resync`は現在の`sid`を
    /// 維持するmaintenance messageであって新しいsessionではない）、`boot`と同じ理由で
    /// 複数idの履歴は要らない。
    processed_hello: Option<(u32, Ack)>,
}

impl Default for PiSession {
    fn default() -> Self {
        Self::new()
    }
}

impl PiSession {
    /// 未確立のsessionを作る。
    #[must_use]
    pub fn new() -> Self {
        Self {
            pi_sid: None,
            retired: VecDeque::with_capacity(RETIRED_CAPACITY),
            processed_hello: None,
        }
    }

    /// 現在承認しているPi `sid`。未確立なら`None`。
    #[must_use]
    pub const fn pi_sid(&self) -> Option<u32> {
        self.pi_sid
    }

    fn is_retired(&self, sid: u32) -> bool {
        self.retired.contains(&sid)
    }

    fn retire_current(&mut self) {
        if let Some(old) = self.pi_sid {
            if !self.retired.contains(&old) {
                self.retired.push_back(old);
                while self.retired.len() > RETIRED_CAPACITY {
                    self.retired.pop_front();
                }
            }
        }
    }

    fn ok_ack(reply_sid: u32, reply_to: u32) -> Ack {
        Ack {
            reply_sid,
            reply_to,
            status: AckStatus::Ok,
            code: None,
            detail: None,
        }
    }

    fn rejection_ack(reply_sid: u32, reply_to: u32, rejection: PiRejection) -> Message {
        Message::Ack(Ack {
            reply_sid,
            reply_to,
            status: AckStatus::Rejected,
            code: Some(rejection.code()),
            detail: None,
        })
    }

    /// `hello`を処理する（§5.1、§8手順8〜10）。
    ///
    /// session遷移budget・cooldown（`PROTO-TBD-012`）は判定しない。呼び出し側が
    /// その上限を別途課す場合は、この呼び出し自体を抑制すること。
    pub fn handle_hello(&mut self, sid: u32, id: u32, hello: &Hello) -> HelloHandled {
        if self.is_retired(sid) {
            let rejection = PiRejection::StaleSession;
            return HelloHandled {
                reply: Self::rejection_ack(sid, id, rejection),
                outcome: HelloOutcome::Rejected(rejection),
            };
        }

        if self.pi_sid == Some(sid) {
            if let Some((processed_id, ack)) = &self.processed_hello {
                if *processed_id == id {
                    return HelloHandled {
                        reply: Message::Ack(ack.clone()),
                        outcome: HelloOutcome::Replayed,
                    };
                }
            }
            // 現在のsidを維持するmaintenance message（§5.1）。`startup`が現在のsidを
            // 名乗るのは不整合であり、遷移も維持もしない。
            return match hello.reason {
                HelloReason::PortReopen | HelloReason::Resync => {
                    let ack = Self::ok_ack(sid, id);
                    HelloHandled {
                        reply: Message::Ack(ack),
                        outcome: HelloOutcome::Maintained,
                    }
                }
                _ => {
                    let rejection = PiRejection::InvalidPayload;
                    HelloHandled {
                        reply: Self::rejection_ack(sid, id, rejection),
                        outcome: HelloOutcome::Rejected(rejection),
                    }
                }
            };
        }

        // 未知のsid: `startup`だけが遷移候補になれる（§5.1）。
        if !matches!(hello.reason, HelloReason::Startup) {
            let rejection = PiRejection::InvalidPayload;
            return HelloHandled {
                reply: Self::rejection_ack(sid, id, rejection),
                outcome: HelloOutcome::Rejected(rejection),
            };
        }

        self.retire_current();
        self.pi_sid = Some(sid);
        let ack = Self::ok_ack(sid, id);
        self.processed_hello = Some((id, ack.clone()));
        HelloHandled {
            reply: Message::Ack(ack),
            outcome: HelloOutcome::Established { sid },
        }
    }

    /// `ping`を処理する（§5.7）。現在のPi sessionからでなければ`stale_session`で拒否する。
    ///
    /// `boot`／`hello`と同じく、応答は`Ack`の`status`／`code`で受理・拒否を運ぶため、
    /// 常に構成できる`Message`を1つ返す（拒否時に呼び出し側がACKを別途組み立てる
    /// 必要はない）。
    pub fn handle_ping(&self, sid: u32, id: u32) -> Message {
        if self.pi_sid == Some(sid) {
            Message::Ack(Self::ok_ack(sid, id))
        } else {
            Self::rejection_ack(sid, id, PiRejection::StaleSession)
        }
    }

    /// `get_status`を処理する（§5.6）。ACKと、受理できた場合の`status`を返す。
    ///
    /// `status`のpayload自体はこの型が持たない（display／servo／sensor stateを
    /// 知らない）。呼び出し側が組み立てた[`Status`]をそのまま包む。`sid`が現在の
    /// Pi sessionと異なる場合は`status`を`None`とし、ACKは拒否済みのものを返す
    /// （`handle_ping`と同じく、拒否時も呼び出し側がACKを別途組み立てずに済む）。
    pub fn handle_get_status(
        &self,
        sid: u32,
        id: u32,
        status: Status,
    ) -> (Message, Option<Message>) {
        if self.pi_sid == Some(sid) {
            let ack = Message::Ack(Self::ok_ack(sid, id));
            (ack, Some(Message::Status(Box::new(status))))
        } else {
            (
                Self::rejection_ack(sid, id, PiRejection::StaleSession),
                None,
            )
        }
    }
}
