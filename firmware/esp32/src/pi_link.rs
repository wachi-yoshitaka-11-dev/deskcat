//! Pi linkの受信の振り分けと、Pi linkのUARTへの書き込み（§8）。
//!
//! Pi linkを持つbuild（`bench-servo-test-17`を有効にするbuild以外のすべて。`#487`）で使う。main loopがPi linkの
//! UARTから読んだbyteを[`PiLink::on_bytes`]へ渡す。`PiLink`は行を組み立ててdecodeし、受信の順に
//! 次のとおり扱う。
//!
//! | 受けたもの | 扱い |
//! |---|---|
//! | `ack` | Envelopeの`sid`を照合し（下記）、`crate::boot_session::BootSession::on_ack`へ渡す |
//! | `hello` | `crate::protocol::PiSession::handle_hello`のACKを書く。受理した場合は、その後に`BootSession::on_hello_accepted`を呼ぶ（§5.1の手順4・5） |
//! | `ping` | `PiSession::handle_ping`のACKを書く |
//! | `get_status` | `PiSession::handle_get_status`のACKと`status`を書く（下記） |
//! | `set_expression` | `deskcat_face::judge`で判定する（session→値の範囲→LCDの準備の順。§8）。受理なら`crate::face::FaceState`へ記録して`ok`のACKを書く。拒否なら該当のcodeの拒否ACKを書く。**製品buildと`face-21`試験buildが受理できる。表情を持たない試験buildは`hardware_unavailable`、bring-upの間は`busy`で拒否する**（§7） |
//! | `show_text` | 現在のPi sessionと`boot`のACKを確認し、LCDが使える場合は`crate::face::FaceDrawer::show_text`へ渡す。受理なら`ok`、文字・時間が描画範囲外なら`out_of_range`、LCDが使えなければ`busy`または`hardware_unavailable`、session未確立なら`stale_session`のACKを書く |
//! | （受信ではない）`tapped`、`shaken`（揺れの設定に値が入ったときだけ） | 製品buildと`tap-21`試験buildが、bootのACKを受けた後に[`send_event`]で書く。受信の振り分けとは別の経路である（`crate::tap`） |
//! | `boot` | `unknown_type`の拒否ACKを書く（§8の表「方向が逆のsession確立message」） |
//! | `play_motion` | session権限（現在のPi `sid`でない、またはESP32の`boot`がACKされていない間は`stale_session`。§4.1、§8手順8）→`deskcat_servo::MotionDispatch::handle`の順に扱い、**どの経路でも拒否ACKを書く**（下記「`play_motion`」）。**`LimitedServo`を作らない間（`crate::servo_path::product`が`None`）は、どのservoのcodeにも渡さない**（[Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)） |
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
//! 送出と、製品build／`tap-21`試験buildが[`send_event`]で送る`tapped`と`shaken`（揺れの設定に値が入ったときだけ）と、製品buildが同じく送る`head_touched`と`environment`だけであり、どれもmain loopの同じtaskから呼ぶ。そのため、連結した2行の間に他の行は
//! 入らない。health snapshot（`main.rs`の`emit_health_snapshot`）はUART0のlogであり、Pi linkへ
//! `status`を送らない。
//!
//! # `play_motion`
//!
//! `MotionDispatch`のcatalogは`MotionCatalog::empty()`のままで、**これを変更する経路は無い**（motion名は
//! 未確定。Protocol §5.3）。そのため、現在の`sid`の`play_motion`は、初めての`id`なら
//! `invalid_payload`（`MotionCatalog`に無い名前）で拒否してその結果を`MotionDedup`へ記録し、同じ`id`の再送には
//! 記録した結果を返す。履歴から失われた`id`は`duplicate_expired`、現在のsessionでない`sid`は
//! `stale_session`である（**ESP32の`boot`がACKされていない間も`stale_session`で、履歴へは記録しない**。§4.1の停止状態を含む）。**`LimitedServo`を作らない間（`crate::servo_path::product`が`None`）は、`admit`は`UnknownMotion`で拒否し、
//! `LimitedServo`も`Limiter`も呼ばない。****そのため`busy`は返らない。**
//! catalogに名前がある場合も、servoへ渡す経路が無いので`invalid_payload`で拒否し、logに残す。
//!
//! 履歴を`clear`するのは、`hello`が新しいPi sessionを確立したとき（`HelloOutcome::Established`）だけである（§9）。保持件数と保持期間で失われた`id`の再送は`duplicate_expired`になる。
//! **保持件数と保持期間は暫定値である**（[`MOTION_DUPLICATE_CAPACITY`]、[`MOTION_DUPLICATE_RETENTION`]）。
//! Session・duplicate・catalogの判定はhostでも試せる`MotionDispatch`と共用する。
//! 実行側のhost testは仮のrelative motionを使い、実機のmotion受理を証明しない。
//!
//! # 実装していないもの
//!
//! - `play_motion`の保持した結果の再送の送出上限（§8.2のretry quota。`PROTO-TBD-012`が未決）。
//!   同じ`(sid, id)`の再送には、そのたびに保持した結果を返す。
//! - `ping`／`get_status`、`set_expression`、`show_text`、`port_reopen`／`resync`の`hello`、拒否した`hello`の処理済みの結果を保持しない
//!   （§8の手順8・9。保持件数は`PROTO-TBD-005`）。同じ`(sid, id)`の再送は、もう一度処理する。
//! - §8.1／§8.2の流量制限、`hello`の拒否ACKの保留table、§5.1の遷移の上限とcooldown（`PROTO-TBD-012`）。
//!   影響は`docs/protocol/esp32-pi-protocol.md`§2の既知の逸脱(iii)。
//! - `ProtocolCounters`を増やすこと（`health.rs`のmodule doc）。分類はlogだけであり、`get_status`へ返す
//!   `status`のcounterは0のままである。
//!
//! # 確かめていないこと
//!
//! **このmoduleと`crate::boot_session`、`crate::protocol`の振る舞いは、host testでも実機でも
//! 確かめていない。**共用する`MotionDispatch`にはhost testがあるが、firmwareでの呼び出しまで
//! 含めて試したものではない。`firmware/esp32`はhostのworkspaceから除外されており、`esp_idf_svc`へ依存する
//! ため、host testでは動かせない。実機の確認（`boot`→ACK、`hello`の後の`boot`の再送、`get_status`の
//! ACKと`status`の順序、受信のring bufferが溢れないこと）はIssue #446と#12が追跡する。

use std::time::Duration;

use deskcat_face::DisplayState;
use deskcat_protocol::{
    encode_line, Ack, AckStatus, Cause, DuplicatePolicy, ErrorCode, Frame, LineReceiver, Message,
    Outcome, PlayMotion, Rejection,
};
use deskcat_servo::{
    admit_pi, drive_tick, fail_safe_on_loss, DedupOutcome, Drive, DriveOutcome, LimiterCounters,
    LinkWatch, MotionCatalog, MotionDecision, MotionDispatch, MotionSession, MotionSink, PiMotion,
    TickSchedule,
};
use esp_idf_svc::hal::uart::UartDriver;

use crate::boot_session::{BootSession, HelloAccepted};
use crate::face::{FaceDrawer, FaceState};
use crate::health::Health;
use crate::protocol::{HelloOutcome, PiSession};
use crate::servo_path::{resolve_target, tick_period_ms, ServoPath, LINK_LOSS_TIMEOUT_MS};

/// `play_motion`の保持件数（暫定）。**確定値ではない。**`PROTO-TBD-005`が未決で、数値そのものに根拠は無い。
///
/// `0`でないという性質だけを満たす仮置きである。catalogが空の間は、motionを実行しないので、この値がservoの動きを変えることは無い。
/// **ただし、保持件数と保持期間を超えた再送には、記録した`invalid_payload`ではなく`duplicate_expired`を返す**ので、再送へ返すcodeはこの値で変わる。**catalogが空でなくなる（motionを受理し始める）前に、確定値にする**
/// （Protocol §5.3の有効化の条件と同じ扱い）。
const MOTION_DUPLICATE_CAPACITY: usize = 8;

/// `play_motion`の保持期間（暫定）。**確定値ではない。**`PROTO-TBD-005`が未決で、数値そのものに根拠は無い。
/// 扱いは[`MOTION_DUPLICATE_CAPACITY`]と同じ。
const MOTION_DUPLICATE_RETENTION: Duration = Duration::from_secs(10);

/// Pi linkが参照するLCDの状態と描画器。表情を持たない試験構成では描画器を持たない。
pub struct FaceTargets<'a, 'd> {
    pub state: &'a mut FaceState,
    pub drawer: Option<&'a mut FaceDrawer<'d>>,
}

/// Pi linkの受信の状態。
pub struct PiLink {
    receiver: LineReceiver,
    pi: PiSession,
    /// 現在のPi sessionの`play_motion`の履歴と空catalog。session遷移で履歴を消す。
    motion_dispatch: MotionDispatch,
    /// `LimitedServo`を持つ経路。製品buildは値が無い間`None`（`crate::servo_path::product`）。
    servo: Option<ServoPath<'static>>,
    /// `tick`を刻む期限。`servo`が`Some`で、周期（`crate::servo_path::tick_period_ms`）が決まっているときだけ`Some`。
    tick_schedule: Option<TickSchedule>,
    /// Piからの受信の途切れの見張り（`LINK_LOSS_TIMEOUT_MS`）。
    link_watch: LinkWatch,
}

impl PiLink {
    /// 未確立のPi sessionで作る。`now_ms`は`Health::uptime_ms`（`tick`の期限の起点）。
    ///
    /// **周期が決まっていない`ServoPath`は刻めないので、`stop`して手放す**（失敗はlogへ残す。drop後の出力は保証しない。
    /// `product`は、周期が決まるまで`None`）。
    pub fn new(mut servo: Option<ServoPath<'static>>, now_ms: u64) -> Self {
        let tick_schedule = tick_period_ms().and_then(|period| TickSchedule::new(period, now_ms));
        if tick_schedule.is_none() {
            if let Some(mut path) = servo.take() {
                log::error!("servo_tick_period_missing (stopして手放す)");
                if let Err(err) = path.stop() {
                    log::error!("servo_stop_failed error={err}");
                }
            }
        }
        // 途切れの時間が決まっていない`ServoPath`は、途切れでstopできないので、同じく手放す。
        if !LinkWatch::new(LINK_LOSS_TIMEOUT_MS).is_armed() {
            if let Some(mut path) = servo.take() {
                log::error!("servo_link_loss_timeout_missing (stopして手放す)");
                if let Err(err) = path.stop() {
                    log::error!("servo_stop_failed error={err}");
                }
            }
        }
        // servoが無ければ刻まない（期限だけがmain loopの待ちを起こすことを避ける）。
        let tick_schedule = tick_schedule.filter(|_| servo.is_some());
        Self {
            servo,
            tick_schedule,
            link_watch: LinkWatch::new(LINK_LOSS_TIMEOUT_MS),
            receiver: LineReceiver::with_protocol_limit(),
            pi: PiSession::new(),
            motion_dispatch: MotionDispatch::new(
                DuplicatePolicy::new(MOTION_DUPLICATE_CAPACITY, MOTION_DUPLICATE_RETENTION)
                    .expect("暫定値は0ではない"),
                MotionCatalog::empty(),
            ),
        }
    }

    /// debug consoleの1行を、Pi由来の`play_motion`と同じ`MotionDispatch`のcatalogへ渡す
    /// （`debug-motion-19` feature付きのbuildだけ。[Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)の受け入れ条件3）。
    ///
    /// `boot_established`は`BootSession::is_established`である。`boot`がACKされていない間（§4.1の停止状態を含む）は、
    /// Pi由来と同じく`admit`へ届かず拒否する。**session（`sid`）・duplicateの検査は通らない**
    /// （`MotionDispatch::handle_debug`のdoc）。**`LimitedServo`を作らない間は何も渡さず、`UnknownMotion`で拒否する**（`on_play_motion`と同じ）。
    /// debugの指示が同じ`Limiter`（同じcounter）へ届くことは、`deskcat_servo`のhost testが示す。
    /// firmwareでは、`LimitedServo`を作る後続の変更まで、limiterに届かない。
    #[cfg(feature = "debug-motion-19")]
    pub fn on_debug_line(&mut self, line: &str, boot_established: bool) {
        let motion = match deskcat_servo::parse_debug_line(line) {
            Ok(motion) => motion,
            Err(err) => {
                log::warn!("debug_motion_unparsed reason={err:?}");
                return;
            }
        };
        let name = motion.name;
        let sink = self.servo.as_mut().map(|path| path as &mut dyn MotionSink);
        let result = deskcat_servo::admit_debug(
            &self.motion_dispatch,
            boot_established,
            name,
            motion.target,
            sink,
        );
        match result {
            Ok(()) => log::info!("debug_motion name={name:?} outcome=admitted"),
            Err(refusal) => log::error!("debug_motion_rejected name={name:?} reason={refusal:?}"),
        }
    }

    /// `status`の`servo.limiter`へ渡す累計。`LimitedServo`が無ければ`None`（省略）。
    pub fn servo_counters(&self) -> Option<LimiterCounters> {
        self.servo.as_ref().map(MotionSink::counters)
    }

    /// 次の`tick`の期限（`Health::uptime_ms`の時刻）。刻まないなら`None`。main loopの待ち（`until`）へ足す。
    pub fn next_servo_deadline_ms(&self) -> Option<u64> {
        self.tick_schedule
            .as_ref()
            .map(TickSchedule::next_deadline_ms)
    }

    /// 受信が途切れたと判定する時刻。見張っていなければ`None`。main loopの待ち（`until`）へ足す。
    pub fn next_link_deadline_ms(&self) -> Option<u64> {
        self.servo.as_ref()?;
        self.link_watch.deadline_ms()
    }

    /// main loopの1周ごとに呼ぶ。Piからの受信が途切れたと最初に判定したとき、`stop`してlogへ残す。
    /// `stop`が失敗したら、`tick_servo`が試し直す。
    pub fn check_link(&mut self, now_ms: u64) {
        let Some(path) = self.servo.as_mut() else {
            return;
        };
        if let Some(result) = fail_safe_on_loss(&mut self.link_watch, path, now_ms) {
            log::error!(
                "pi_link_lost now_ms={now_ms} losses={}",
                self.link_watch.losses()
            );
            if let Err(err) = result {
                log::error!(
                    "servo_stop_failed error={err} stop_failures={}",
                    path.stop_failures()
                );
            }
        }
    }

    /// main loopの1周ごとに呼ぶ。**期限が来たときだけ`tick`する**（制御周期。blockingしない）。`LimitedServo`が無ければ何もしない。
    /// `stop`が失敗したままなら、`tick`せずに`stop`を試し直す。`tick`が失敗したら`stop`を呼ぶ。どちらの失敗もlogへ残す。
    pub fn tick_servo(&mut self, now_ms: u64) {
        let (Some(path), Some(schedule)) = (self.servo.as_mut(), self.tick_schedule.as_mut())
        else {
            return;
        };
        if !schedule.due(now_ms) {
            return;
        }
        if path.stop_pending() {
            if let Err(err) = path.stop() {
                log::error!(
                    "servo_stop_retry_failed error={err} stop_failures={}",
                    path.stop_failures()
                );
            }
            return;
        }
        if let DriveOutcome::Stopped { tick_error, stop } = drive_tick(path) {
            log::error!("servo_tick_failed_stopped error={tick_error}");
            if let Err(err) = stop {
                log::error!(
                    "servo_stop_failed error={err} stop_failures={}",
                    path.stop_failures()
                );
            }
        }
    }

    /// Pi linkのUARTから読めた生byteを渡す。`now_ms`は単調に増える時刻（`Health::uptime_ms`）である。
    pub fn on_bytes(
        &mut self,
        buf: &[u8],
        now_ms: u64,
        boot: &mut BootSession,
        health: &Health,
        face: &mut FaceTargets<'_, '_>,
        uart: &mut UartDriver<'_>,
    ) {
        // `drain`のclosureは`&mut self`を借りられないため、結果を受信の順に集めてから扱う。
        let mut outcomes: Vec<Outcome> = Vec::new();
        self.receiver.drain(buf, |outcome| outcomes.push(outcome));
        for outcome in outcomes {
            match outcome {
                Outcome::Frame(frame) => self.on_frame(frame, now_ms, boot, health, face, uart),
                Outcome::Rejected(rejection) => on_rejected(&rejection, boot, health, uart),
            }
        }
    }

    fn on_frame(
        &mut self,
        frame: Frame,
        now_ms: u64,
        boot: &mut BootSession,
        health: &Health,
        face: &mut FaceTargets<'_, '_>,
        uart: &mut UartDriver<'_>,
    ) {
        let Frame { envelope, message } = frame;
        let (sid, id) = (envelope.sid, envelope.id);
        // 仮置き：現在のPi sessionのframeを受信とみなす（heartbeatの方式はPROTO-TBD-010）。
        if self.pi.pi_sid() == Some(sid) {
            self.link_watch.note_rx(now_ms);
        }
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
                if self.pi.pi_sid() == Some(sid) {
                    self.link_watch.note_rx(now_ms);
                }
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
                // 新しいPi sessionの確立は、旧sessionの履歴を捨てる唯一の経路である（§9）。
                if matches!(handled.outcome, HelloOutcome::Established { .. }) {
                    self.motion_dispatch.on_new_session();
                }
                if let Some(accepted) = accepted {
                    boot.on_hello_accepted(accepted, health, uart);
                }
            }
            Message::Ping => {
                send(vec![self.pi.handle_ping(sid, id)], boot, health, uart);
            }
            Message::GetStatus => {
                let limiter = self.servo_counters();
                let (ack, status) = self
                    .pi
                    .handle_get_status(sid, id, health.to_status(limiter));
                let mut replies = vec![ack];
                replies.extend(status);
                send(replies, boot, health, uart);
            }
            Message::SetExpression(command) => {
                let current = self.pi.pi_sid() == Some(sid);
                let reply = match deskcat_face::judge(&command, current, face.state.display_state())
                {
                    Ok(name) => {
                        face.state.request(name);
                        log::info!("pi_rx_set_expression sid={sid} id={id} name={name:?} accepted");
                        Message::Ack(Ack {
                            reply_sid: sid,
                            reply_to: id,
                            status: AckStatus::Ok,
                            code: None,
                            detail: None,
                        })
                    }
                    Err(code) => {
                        log::error!(
                            "pi_rx_set_expression_rejected sid={sid} id={id} code={}",
                            code.as_str()
                        );
                        rejection_ack(sid, id, code)
                    }
                };
                send(vec![reply], boot, health, uart);
            }
            Message::ShowText(command) => {
                let result = if self.pi.pi_sid() != Some(sid) || !boot.is_established() {
                    Err(ErrorCode::StaleSession)
                } else {
                    match face.state.display_state() {
                        DisplayState::Unavailable => Err(ErrorCode::HardwareUnavailable),
                        DisplayState::Busy => Err(ErrorCode::Busy),
                        DisplayState::Ready => match face.drawer.as_deref_mut() {
                            Some(drawer) => drawer
                                .show_text(&command.text, command.duration_ms, now_ms)
                                .map_err(deskcat_face::text::TextError::code),
                            None => {
                                log::error!(
                                    "pi_rx_show_text_ready_without_drawer sid={sid} id={id}"
                                );
                                Err(ErrorCode::HardwareUnavailable)
                            }
                        },
                    }
                };
                let reply = match result {
                    Ok(()) => {
                        log::info!("pi_rx_show_text sid={sid} id={id} accepted");
                        Message::Ack(Ack {
                            reply_sid: sid,
                            reply_to: id,
                            status: AckStatus::Ok,
                            code: None,
                            detail: None,
                        })
                    }
                    Err(code) => {
                        log::error!(
                            "pi_rx_show_text_rejected sid={sid} id={id} code={}",
                            code.as_str()
                        );
                        rejection_ack(sid, id, code)
                    }
                };
                send(vec![reply], boot, health, uart);
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
            Message::PlayMotion(command) => {
                let reply = self.on_play_motion(sid, id, &command, now_ms, boot.is_established());
                send(vec![reply], boot, health, uart);
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

impl PiLink {
    /// `play_motion`を、session権限→duplicateの順に扱い、返すACKを作る（§8手順8・10）。
    ///
    /// `boot_established`は`BootSession::is_established`である。**`boot`がACKされていない間（§4.1の停止状態を含む）は、
    /// motion commandを`stale_session`で拒否し、履歴へ記録しない**（§4.1、§12）。
    ///
    /// **`LimitedServo`を作らない間は何も渡さない。**空catalogのため`admit`は呼ばれない。
    fn on_play_motion(
        &mut self,
        sid: u32,
        id: u32,
        command: &PlayMotion,
        now_ms: u64,
        boot_established: bool,
    ) -> Message {
        let sink = self.servo.as_mut().map(|path| path as &mut dyn MotionSink);
        let outcome = admit_pi(
            &mut self.motion_dispatch,
            MotionSession::new(self.pi.pi_sid(), boot_established),
            PiMotion {
                sid,
                id,
                command,
                now_ms,
            },
            resolve_target,
            sink,
        );
        match outcome.code() {
            None => {
                log::info!("pi_rx_play_motion sid={sid} id={id} outcome={outcome:?}");
                Message::Ack(Ack {
                    reply_sid: sid,
                    reply_to: id,
                    status: AckStatus::Ok,
                    code: None,
                    detail: None,
                })
            }
            Some(code) => {
                let replayed = matches!(outcome, MotionDecision::Dedup(DedupOutcome::Replayed(_)));
                log::error!(
                    "pi_rx_play_motion_rejected sid={sid} id={id} code={} replayed={replayed}",
                    code.as_str()
                );
                rejection_ack(sid, id, code)
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

/// ESP32からのevent（`tapped`など。§4）を1つ送る。
///
/// **ACKを待たず、再送しない。**§8の表はeventを要求として扱わない（応答を返さない）が、ACKの要否そのものは§9の
/// 受け入れ前TBD（「ACKを必要とするmessage」）で未決である。`write_line`が失敗した場合は、その行のeventを失い、
/// logにだけ残る（`pi_uart_write_failed`か`pi_uart_write_stalled`）。
///
/// sessionが確立している（`BootSession::is_established`）ことは、呼び出し側が確かめる。
#[cfg_attr(not(feature = "tap-21"), allow(dead_code))]
pub fn send_event(
    message: Message,
    boot: &mut BootSession,
    health: &Health,
    uart: &mut UartDriver<'_>,
) {
    send(vec![message], boot, health, uart);
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
