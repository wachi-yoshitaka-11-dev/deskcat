//! `boot`の受理確認・再送（§4.1）と、`sid`選び直し（§3.1「`sid`が衝突した場合」）。
//!
//! Pi linkを持つbuild（`bench-servo-test-17`以外のすべて。`#487`）で使う（`#446` PR B）。`main.rs`の`generate_sid`が選んだ`sid`で
//! `boot`を送り、`crate::pi_link::PiLink`がPi linkのUART（`PI-UART-RX`、`#487`）から受けて渡す`ack`を待つ。§4.1の終了条件の表が定める**state遷移**
//! （いつ再送するか、いつ`sid`を選び直すか、いつ止めるか）を実装する。featureの各構成での
//! build（`cargo build`／`cargo clippy`）が通ることはcompileが通ることの根拠であり、**state遷移が
//! 正しいことの根拠ではない。**host testと実機で確かめていないことは`crate::pi_link`のmodule doc
//! 「確かめていないこと」が持つ。
//! **表が定める`protocol_fault`の送出（wireへの報告）は`PROTO-TBD-018`が
//! 未確定のため実装しない**（下記）。
//!
//! # `PROTO-TBD-017`・`PROTO-TBD-011`の残りは暫定値
//!
//! 再送間隔・backoff係数・通常再送の回数・recovery間隔・recovery budget
//! （`PROTO-TBD-017`）と、`sid`選び直し回数の上限（`PROTO-TBD-011`の残り）は、
//! いずれも確定していない。[`BootRetryPolicy::provisional`]が返す値は
//! `crates/deskcat-serial`の`ReconnectPolicy::provisional`と同じ扱いで
//! （`RetryPolicy::provisional`は`max_retries`が§9の確定値であり、これとは
//! 扱いが異なる。`crates/deskcat-serial/src/config.rs`294〜296行参照）、
//! 「上限が有限である」という性質だけを満たす仮置きであり、数値そのものに
//! 根拠は無い。「通常再送の間隔が単調に伸びる」はbackoff係数だけでは決まらず、値の
//! 大小関係にも依る（通常再送の最後の間隔
//! `initial_interval × backoff_factor^(normal_retries-1)`が`recovery_interval`
//! 以下であること。暫定値ではたまたま成り立つ：200ms×2⁴=3200ms ≤ 5000ms）。
//! `rate_limited_cooldown`（1秒固定）は単調性の対象に含めない。
//!
//! # `protocol_fault`はwireへ出さない
//!
//! §4.1は終端を`protocol_fault`で報告すると定めるが、`protocol_fault`のwire表現は
//! `PROTO-TBD-018`が未確定であり（`#446`の対象外）、`deskcat_protocol::Message`に
//! 対応するvariantが無い。**したがってこのmoduleは`protocol_fault`をwireへ送らない。**
//! 「送出を止める」という動作面だけを実装する（`boot`送出を止めた状態で`main()`は
//! 戻らずheartbeat／health snapshotのloopを続ける）。servo出力の無効化は、
//! Pi linkを持つbuildが`crate::servo`を呼ぶ箇所を持たない（`crate::servo`の呼び出しは
//! `bench-servo-test-17` featureの下にしかなく、そのbuildはPi linkを持たない。`main.rs`を
//! 目視で確認した。`mod servo`自体はfeature条件無しでcompileされる）ため満たされる。網羅的な
//! 呼び出し元探索（例: 静的解析）はしていない。
//!
//! # 停止理由の区別
//!
//! 停止理由は[`TerminalReason`]（[`Phase::Terminated`]が保持する）として内部stateに
//! 持ち、終端したときにUART0（USB）のdebug logへ出す（`#487`。それより前、Pi linkを持つbuild
//! （当時の`pi-protocol-mode`）はloggingを止めており、出力されなかった）。`Health`の
//! `ProtocolCounters`はまだwireへ送る経路が無い（`#12`）。したがって`counter`は
//! 増やさない——増やしても読む経路が無いcounterになる（`#448`で同じ理由から
//! `boot_serialize_errors`を削除した判断に揃えた）。`TerminalReason`として
//! 区別を保持し、将来`status`をwireへ送る段になったら`ProtocolCounters`へ写せる形に
//! しておく。
//!
//! # `hello`による再送の再開（§4.1の「`hello`による再開」）
//!
//! `crate::pi_link::PiLink`は、`hello`を受理してそのACKを書いた後に
//! [`BootSession::on_hello_accepted`]を呼ぶ。再開する条件は§4.1の「`hello`による再開」が持つ。この型は、
//! 再開したかどうかを、Pi sessionごとに1つの記録（`resumed_in_pi_session`）で持ち、
//! `hello`による遷移（[`HelloAccepted::Transition`]）でだけ解除する。再開は同じ`(sid, id)`のまま、
//! 通常再送の最初（`Phase::Normal { attempt: 0 }`）から行う。
//!
//! # ESP32の`(sid, id)`の採番
//!
//! ESP32がPi linkへ送る`boot`以外のmessageの`(sid, id)`は、[`BootSession::next_envelope`]の1か所で払い出す
//! （§3「採番は単一の点で直列化し」）。`boot`は`id`＝1、それ以外は2から単調に増やす。`sid`を
//! 選び直したら初期値へ戻す（§3.1）。`u32`の上限値は`protocol_fault`のために予約し（§3）、払い出さない。
//! `protocol_fault`を送らないため（上記）、上限値に達した後は新しい`id`を要する送出をすべて止める。
//!
//! # debug logへ出すもの
//!
//! `boot`の送出（`boot_tx`。1行の全byteをtx ring bufferへ積み終えた時点であり、wireへ出た
//! 時点ではない。`crate::pi_link::write_line`のcomment参照）、`status: ok`での確立（`boot_established`）、`sid`の
//! 選び直し（`boot_sid_reselected`）、終端（`boot_terminated`）、`hello`による再開（`boot_resumed`）を`log`へ出す。
//! [#446]の受け入れ条件4（実機での`boot`→ACK）の試験で、ESP32側で何が起きたかを
//! Pi側のlogと突き合わせるためである。出力先はUART0（USB）であり、Pi linkの
//! UARTには出ない（`crate::console`参照）。
//!
//! [#446]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446

use std::time::Duration;

use deskcat_protocol::{
    encode_line, limits, Ack, AckStatus, Boot, Envelope, ErrorCode, Frame, Message,
};
use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::uart::UartDriver;

use crate::health::Health;

/// `boot`の`id`（§4.1。`sid`ごとに固定）。
const BOOT_ID: u32 = 1;

/// `PROTO-TBD-017`の暫定値。数値に根拠は無い（module doc参照）。
#[derive(Debug, Clone, Copy)]
pub struct BootRetryPolicy {
    /// 最初の再送までの間隔。
    initial_interval: Duration,
    /// 再送のたびに間隔へ掛ける係数。
    backoff_factor: u32,
    /// 通常再送の回数。**この境界の送出（`normal_retries`回目）はrecoveryへ
    /// 移りながらも送る。**`normal_retries = 5`なら、初回送出1回＋通常間隔での
    /// 再送5回＝計6回送ってからrecovery間隔での再送に移る（`on_deadline`の
    /// `Phase::Normal`腕参照。境界の呼び出しで`self.send_boot`を呼んでから
    /// phaseを`Recovery`へ進める）。
    normal_retries: u32,
    /// Recovery間隔（通常再送を使い切った後の再送間隔。以降固定）。
    recovery_interval: Duration,
    /// Recovery budget。**`normal_retries`とは数え方が違う。**この境界
    /// （`recovery_budget`回目）では送らずに終端する（`on_deadline`の
    /// `Phase::Recovery`腕参照。境界の呼び出しは`return`で抜け、
    /// `self.send_boot`を呼ばない）。したがって、recovery間隔での実際の
    /// 再送回数は`recovery_budget - 1`回である。
    recovery_budget: u32,
    /// `rate_limited`を受けたときのcooldown。
    rate_limited_cooldown: Duration,
    /// `rate_limited`のcooldown後再送の上限回数。数えるのは`on_deadline`が
    /// 実際に`send_boot`を呼んだ回数（`Phase::RateLimited`の`attempt`）であり、
    /// `Normal`／`Recovery`からこの局面へ最初に入るきっかけになった送信
    /// （`attempt`はまだ`0`）は数えない。`recovery_budget`と同じく、境界
    /// （`rate_limited_budget`回目）は送らずに終端するため、実際のcooldown後
    /// 再送回数は`rate_limited_budget - 1`回である
    /// （`on_deadline`の`Phase::RateLimited`腕参照）。
    rate_limited_budget: u32,
    /// `stale_session`による`sid`選び直し回数の上限（`PROTO-TBD-011`の残り）。
    sid_reselect_limit: u32,
}

impl BootRetryPolicy {
    /// 暫定の既定値を返す。**確定値ではない。**
    #[must_use]
    pub const fn provisional() -> Self {
        Self {
            initial_interval: Duration::from_millis(200),
            backoff_factor: 2,
            normal_retries: 5,
            recovery_interval: Duration::from_secs(5),
            recovery_budget: 6,
            rate_limited_cooldown: Duration::from_secs(1),
            rate_limited_budget: 5,
            sid_reselect_limit: 5,
        }
    }
}

/// このsessionが今どの局面にあるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// 通常再送中。`attempt`は0起点の再送回数。
    Normal { attempt: u32 },
    /// 通常再送を使い切り、recovery間隔で再送中。`attempt`はrecovery内の回数。
    Recovery { attempt: u32 },
    /// `rate_limited`のcooldown待ち。`attempt`は**実際に送った（`send_boot`を
    /// 呼んだ）回数**であり、budgetの消費もこれに1対1で対応する。
    ///
    /// `in_cooldown`が2つの副状態を分ける。
    /// - `false`（返事待ち）: 送った直後で、まだその返信を受けていない。
    /// - `true`（cooldown中）: 送った直後の返信として最初の`rate_limited`
    ///   ACKを受け、次の再送までの待ちに入った。
    ///
    /// **`(sid, id)`は`rate_limited`の間ずっと同じであるため、ESP32はどの
    /// ACKがどの送信への返信かを区別できない（Piは§8.2で重複ACKを送りうる）。**
    /// この区別を諦め、代わりに次の規則で「消費は送った回数と1対1」を保証する。
    /// - 返事待ち中に`rate_limited`のACKを受けたら、それを直前の送信への
    ///   最初の返信とみなしcooldownへ移る。**budgetは消費しない。**
    /// - cooldown中に届く`rate_limited`のACKは、重複とみなして無視する
    ///   （期限を延ばさない。`attempt`も動かさない）。
    /// - 期限が来たら（返事待ち中でもcooldown中でも同じ）再送し、`attempt`を
    ///   1つ進める。**budgetの消費はここだけで起こる。**
    ///
    /// 古い重複ACKを新しい送信への返信と取り違えることはありうる
    /// （区別できないため）。その場合もcooldownへ入る時刻が早まるだけで、
    /// 消費されるかどうかは変わらない（`apply_ack`・`on_deadline`のcomment参照）。
    ///
    /// # 返事待ち中に無応答のまま期限が来た場合の扱い（意図した選択）
    ///
    /// - **その後`rate_limited`のACKを一度も受け取っていなくても、終端理由は
    ///   `RateLimitedBudgetExhausted`のままである。**`RateLimited`へ入る条件は
    ///   最初の`rate_limited`受信だけであり、以後は「一度rate_limitedを受けた
    ///   状態」として扱い続ける。
    /// - **`Recovery`から`RateLimited`へ移った場合、`Recovery`側で残っていた
    ///   budget・間隔は捨てる。**`sid`・`id`は変えない（§4.1は`rate_limited`を
    ///   独立した終了条件として定めており、`Normal`／`Recovery`のbudgetと
    ///   合算しない）。
    /// - **返事待ち中の期限にも`rate_limited_cooldown`をそのまま使う**
    ///   （専用の値を持たない。無応答時の再送間隔として妥当という以上の
    ///   根拠は無い）。
    RateLimited { attempt: u32, in_cooldown: bool },
    /// `status: ok`を受けた。以降送出しない。
    Established,
    /// 終端。理由は[`TerminalReason`]。
    ///
    /// `hello`で再開するのは[`TerminalReason::resumable_by_hello`]が`true`を返す理由だけである
    /// （module doc「`hello`による再送の再開」）。それ以外の理由では、processの再起動または
    /// 運用者の明示的なsession reset（§3.1）まで再開しない。運用者のsession resetを受ける経路は
    /// まだ無い。
    Terminated(TerminalReason),
}

/// 終端に至った理由。終端時にdebug logへ出し、区別を内部stateに残す（module doc参照）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // `Debug`での出力以外にはまだ読まれない。将来のstatus送出に備えて保持する。
enum TerminalReason {
    /// Recovery budgetを使い切った。
    RecoveryBudgetExhausted,
    /// `rate_limited`のbudgetを使い切った。
    RateLimitedBudgetExhausted,
    /// `sid`選び直し回数の上限に達した。
    SidReselectLimitReached,
    /// `stale_session`／`rate_limited`以外の`rejected`を受けた。
    Rejected(ErrorCode),
}

impl TerminalReason {
    /// 受理した`hello`で`boot`の再送を再開する終端か（§4.1の「`hello`による再開」）。
    const fn resumable_by_hello(self) -> bool {
        matches!(
            self,
            Self::RecoveryBudgetExhausted | Self::RateLimitedBudgetExhausted
        )
    }
}

/// [`BootSession::on_hello_accepted`]へ渡す、受理した`hello`の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelloAccepted {
    /// 新しいPi `sid`の`hello`で、Pi sessionが遷移した（§5.1の手順1〜4）。
    Transition,
    /// 現在のPi `sid`の`port_reopen`／`resync`の`hello`を、sessionを維持して受理した。
    Maintained,
}

/// `boot`のACK待ち・再送・`sid`選び直しを行うsession。
pub struct BootSession {
    policy: BootRetryPolicy,
    sid: u32,
    /// `boot`は`id`固定（§4.1「確認を得るまで、ESP32は同じ`(sid, id)`で`boot`を
    /// 再送する」）だが、`sid`選び直し時に
    /// 仕様どおり初期値へ戻す操作を明示するため field として持つ。
    id: u32,
    reset_reason: &'static str,
    phase: Phase,
    next_deadline_ms: u64,
    sid_reselect_count: u32,
    /// 次に払い出す`id`（module doc「ESP32の`(sid, id)`の採番」）。
    next_id: u32,
    /// 現在のPi sessionで、`hello`による再開を既に行ったか（module doc「`hello`による再送の再開」）。
    resumed_in_pi_session: bool,
}

impl BootSession {
    /// `boot`のACKを受け、Piとのsessionが確立しているか（`Phase::Established`）。
    /// event（`tapped`）を送ってよいかの判定に使う（Issue #21、F1）。
    #[must_use]
    #[cfg_attr(not(feature = "tap-21"), allow(dead_code))]
    pub fn is_established(&self) -> bool {
        matches!(self.phase, Phase::Established)
    }

    /// 初期`sid`で`boot`を1回送り、sessionを開始する。
    pub fn start(
        policy: BootRetryPolicy,
        sid: u32,
        reset_reason: &'static str,
        health: &Health,
        uart: &mut UartDriver<'_>,
    ) -> Self {
        let mut session = Self {
            policy,
            sid,
            id: BOOT_ID,
            reset_reason,
            phase: Phase::Normal { attempt: 0 },
            next_deadline_ms: 0,
            sid_reselect_count: 0,
            next_id: BOOT_ID + 1,
            resumed_in_pi_session: false,
        };
        session.send_boot(health, uart);
        session.next_deadline_ms = health.uptime_ms() + duration_to_ms(policy.initial_interval);
        session
    }

    /// 次に`on_deadline`を呼ぶべき時刻（ms）。`None`なら、これ以上送出しない
    /// （`Established`または`Terminated`）ため、main loopの締切計算から除いてよい。
    pub fn next_deadline_ms(&self) -> Option<u64> {
        match self.phase {
            Phase::Established | Phase::Terminated(_) => None,
            Phase::Normal { .. } | Phase::Recovery { .. } | Phase::RateLimited { .. } => {
                Some(self.next_deadline_ms)
            }
        }
    }

    /// `crate::pi_link::PiLink`が受けたACKを渡す。§6のEnvelopeの`sid`の照合は`PiLink`が行い、
    /// ここでは`reply_sid`／`reply_to`を自分の`boot`の`(sid, id)`と照合する。
    ///
    /// 一致しないACKと、`Established`／`Terminated`の間に届いたACKは適用せず、logで分類する
    /// （§7「Parser counterでは…session不一致を区別する」。counterは増やさない。
    /// `health.rs`のmodule doc参照）。
    pub fn on_ack(&mut self, ack: Ack, health: &Health, uart: &mut UartDriver<'_>) {
        if matches!(self.phase, Phase::Established | Phase::Terminated(_)) {
            log::error!(
                "boot_rx_ack_after_close reply_sid={} reply_to={}",
                ack.reply_sid,
                ack.reply_to
            );
            return;
        }
        // **`self.sid`／`self.id`を、呼び出しのたびに読み直す。**同じchunk内の先行ACKが
        // `stale_session`で`reselect_sid`を起こすと`self.sid`／`self.id`が変わりうるため、
        // 後続のACKは新しい値と照合しないと、旧`(sid, id)`宛のACK（例: 旧sidへの`ok`）を
        // 現在のsessionへ誤って適用しうる。**`generate_sid`が前回と異なる値を返す保証は無い**
        // （NVS counterの通常経路は単調増加で異なる値になるが、`generate_sid`のdoc
        // 「NVSが使えない場合」の縮退経路（uptime依存）へ落ちた場合、たまたま前回と同じ値に
        // なりうる）。同じ値が返った場合、旧`(sid, id)`宛の重複`stale_session`は次回も一致して
        // しまい、選び直しの回数（`sid_reselect_limit`）を消費し続ける。これは縮退経路自体が
        // 非衝突を主張しないという既存の制約の一部として扱う。
        if ack.reply_sid == self.sid && ack.reply_to == self.id {
            self.apply_ack(ack, health, uart);
        } else {
            log::error!(
                "boot_rx_session_mismatch reply_sid={} reply_to={}",
                ack.reply_sid,
                ack.reply_to
            );
        }
    }

    /// `hello`を受理し、そのACKを書いた後に呼ぶ（§5.1の手順5、§4.1の「`hello`による再開」）。
    pub fn on_hello_accepted(
        &mut self,
        accepted: HelloAccepted,
        health: &Health,
        uart: &mut UartDriver<'_>,
    ) {
        if accepted == HelloAccepted::Transition {
            self.resumed_in_pi_session = false;
        }
        let resume = match self.phase {
            Phase::Established => accepted == HelloAccepted::Transition,
            Phase::Terminated(reason) => reason.resumable_by_hello(),
            Phase::Normal { .. } | Phase::Recovery { .. } | Phase::RateLimited { .. } => false,
        };
        if !resume || self.resumed_in_pi_session {
            return;
        }
        self.resumed_in_pi_session = true;
        let previous = self.phase;
        self.phase = Phase::Normal { attempt: 0 };
        log::info!(
            "boot_resumed accepted={accepted:?} previous={previous:?} sid={} id={}",
            self.sid,
            self.id
        );
        self.next_deadline_ms = health.uptime_ms() + duration_to_ms(self.policy.initial_interval);
        self.send_boot(health, uart);
    }

    /// `boot`以外のmessageに使う、新しい`(sid, id)`のEnvelopeを払い出す
    /// （module doc「ESP32の`(sid, id)`の採番」）。上限値に達していれば`None`を返す。
    pub fn next_envelope(&mut self, ts_ms: u64) -> Option<Envelope> {
        if self.next_id == u32::MAX {
            log::error!("pi_tx_id_exhausted sid={}", self.sid);
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        Some(Envelope {
            v: limits::PROTOCOL_VERSION,
            sid: self.sid,
            id,
            ts_ms,
        })
    }

    /// 締切を過ぎた（＝ACKが届かないまま次の送出時刻に達した）ときに呼ぶ。
    pub fn on_deadline(&mut self, health: &Health, uart: &mut UartDriver<'_>) {
        let now = health.uptime_ms();
        if now < self.next_deadline_ms {
            return;
        }
        match self.phase {
            Phase::Established | Phase::Terminated(_) => {}
            Phase::Normal { attempt } => {
                let next_attempt = attempt + 1;
                if next_attempt >= self.policy.normal_retries {
                    // 通常再送を使い切った。**直ちには止めない。**recovery間隔へ移る
                    // （§4.1「通常再送の上限で直ちに止めないのは…」）。
                    self.phase = Phase::Recovery { attempt: 0 };
                    self.next_deadline_ms = now + duration_to_ms(self.policy.recovery_interval);
                } else {
                    self.phase = Phase::Normal {
                        attempt: next_attempt,
                    };
                    let backoff = self
                        .policy
                        .initial_interval
                        .saturating_mul(self.policy.backoff_factor.saturating_pow(next_attempt));
                    self.next_deadline_ms = now + duration_to_ms(backoff);
                }
                self.send_boot(health, uart);
            }
            Phase::Recovery { attempt } => {
                let next_attempt = attempt + 1;
                if next_attempt >= self.policy.recovery_budget {
                    self.terminate(TerminalReason::RecoveryBudgetExhausted);
                    return;
                }
                self.phase = Phase::Recovery {
                    attempt: next_attempt,
                };
                self.next_deadline_ms = now + duration_to_ms(self.policy.recovery_interval);
                self.send_boot(health, uart);
            }
            Phase::RateLimited { attempt, .. } => {
                // 期限が来たら、返事待ち中でもcooldown中でも同じく再送し、
                // `attempt`を1つ進める。budgetの消費はここだけで起こる
                // （`Phase::RateLimited`のdoc comment参照）。
                let next_attempt = attempt + 1;
                if next_attempt >= self.policy.rate_limited_budget {
                    self.terminate(TerminalReason::RateLimitedBudgetExhausted);
                    return;
                }
                // cooldown経過後、**同じ`(sid, id)`で**再送する（§4.1）。送った
                // 直後なので「返事待ち」（`in_cooldown: false`）へ戻す。
                self.phase = Phase::RateLimited {
                    attempt: next_attempt,
                    in_cooldown: false,
                };
                self.next_deadline_ms = now + duration_to_ms(self.policy.rate_limited_cooldown);
                self.send_boot(health, uart);
            }
        }
    }

    /// §4.1の終了条件の表を適用する。
    fn apply_ack(&mut self, ack: Ack, health: &Health, uart: &mut UartDriver<'_>) {
        match ack.status {
            AckStatus::Ok => {
                self.phase = Phase::Established;
                log::info!("boot_established sid={} id={}", self.sid, self.id);
            }
            AckStatus::Rejected => {
                let code = ack.code;
                match code {
                    Some(ErrorCode::StaleSession) => self.reselect_sid(health, uart),
                    Some(ErrorCode::RateLimited) => {
                        // **budgetを消費しない。**消費するのは`on_deadline`が実際に
                        // `send_boot`を呼ぶ時だけであり、ここでは副状態
                        // （`in_cooldown`）を進めるだけにする
                        // （`Phase::RateLimited`のdoc comment参照）。
                        match self.phase {
                            Phase::RateLimited {
                                in_cooldown: false, ..
                            }
                            | Phase::Normal { .. }
                            | Phase::Recovery { .. } => {
                                // 返事待ち中の最初の返信（あるいはNormal／Recoveryから
                                // 最初にrate_limitedを受けた場合）。cooldownへ移る。
                                let attempt = match self.phase {
                                    Phase::RateLimited { attempt, .. } => attempt,
                                    _ => 0,
                                };
                                self.phase = Phase::RateLimited {
                                    attempt,
                                    in_cooldown: true,
                                };
                                self.next_deadline_ms = health.uptime_ms()
                                    + duration_to_ms(self.policy.rate_limited_cooldown);
                            }
                            Phase::RateLimited {
                                in_cooldown: true, ..
                            } => {
                                // cooldown中に届いた重複ACK。無視する
                                // （期限を延ばさない。`attempt`も動かさない）。
                            }
                            Phase::Established | Phase::Terminated(_) => {
                                // `on_ack`の先頭で弾かれるため到達しない。
                            }
                        }
                    }
                    Some(other) => self.terminate(TerminalReason::Rejected(other)),
                    None => {
                        // `decode_line`が`Ack::check_shape`を通すため、`Outcome::Frame`として
                        // ここへ届く`Ack`は`status: rejected`なら必ず`code`を持つ
                        // （§6「rejectedの場合はcodeを含める」）。到達しない分岐だが、
                        // `code`が`Option`である以上`match`を網羅するために残す。何もしない。
                    }
                }
            }
            // `AckStatus`は`#[non_exhaustive]`（crate境界を越えた追加variantに
            // 備える設計）。`ok`／`rejected`しかvariantを持たず`#[serde(other)]`も
            // 無いため、wire上の未知のstatus文字列はdecodeで`Rejected(Decode)`に
            // なりここには来ない。この腕に来るのは、crate側にvariantが追加された
            // 後にfirmwareをbuildし直した場合だけである。相関できるACKが
            // 届かなかったのと同じく無視する（既存の再送・backoffへ委ねる。
            // terminateしない）。
            _ => {}
        }
    }

    /// `sid`を選び直し、`id`を初期値へ戻して再送する（§3.1「`sid`が衝突した場合」）。
    ///
    /// **`sid_reselect_limit`の数え方は`recovery_budget`・`rate_limited_budget`と
    /// 違う。**それらは`next_attempt >= limit`（境界の回では送らず終端する）
    /// だが、ここは`> limit`（境界の回まで選び直し、その次で終端する）。
    /// `sid_reselect_limit = 5`なら、選び直しは5回まで許され6回目の呼び出しで
    /// 終端する。
    fn reselect_sid(&mut self, health: &Health, uart: &mut UartDriver<'_>) {
        self.sid_reselect_count += 1;
        if self.sid_reselect_count > self.policy.sid_reselect_limit {
            self.terminate(TerminalReason::SidReselectLimitReached);
            return;
        }
        // `generate_sid`は呼び出しのたびに`EspDefaultNvsPartition::take()`する。
        // `take()`のRAII guard（esp-idf-svc 0.52.1 `src/nvs.rs`74〜116行）は
        // `generate_sid`の戻り値を得た時点でdropし解放されるため、他の呼び出し元が
        // partitionを保持し続けていない限り複数回成功しうる。`init`（`nvs_flash_init()`）は
        // 呼び出しのたびに走り直す（同74〜86行）。条件次第で`nvs_flash_erase()`も
        // 起きる（`generate_sid`のdoc「衝突許容確率」節参照）。取れなければ
        // `sid_from_uptime`へ縮退する。
        let previous_sid = self.sid;
        self.sid = crate::generate_sid(health);
        self.id = BOOT_ID;
        self.next_id = BOOT_ID + 1;
        self.phase = Phase::Normal { attempt: 0 };
        log::info!(
            "boot_sid_reselected previous_sid={previous_sid} sid={} count={}",
            self.sid,
            self.sid_reselect_count
        );
        let now = health.uptime_ms();
        self.next_deadline_ms = now + duration_to_ms(self.policy.initial_interval);
        self.send_boot(health, uart);
    }

    fn terminate(&mut self, reason: TerminalReason) {
        self.phase = Phase::Terminated(reason);
        log::warn!(
            "boot_terminated reason={reason:?} sid={} id={}",
            self.sid,
            self.id
        );
    }

    fn send_boot(&self, health: &Health, uart: &mut UartDriver<'_>) {
        let boot = Boot {
            firmware: env!("CARGO_PKG_VERSION").to_owned(),
            board: crate::config::BOARD.to_owned(),
            reset_reason: self.reset_reason.to_owned(),
        };
        let ts_ms = health.uptime_ms();
        let frame = Frame::new(
            Envelope {
                v: limits::PROTOCOL_VERSION,
                sid: self.sid,
                id: self.id,
                ts_ms,
            },
            Message::Boot(boot),
        );
        match encode_line(&frame) {
            Ok(line) => {
                if crate::pi_link::write_line(uart, line.as_bytes()) {
                    log::info!(
                        "boot_tx sid={} id={} ts_ms={ts_ms} phase={:?} bytes={}",
                        self.sid,
                        self.id,
                        self.phase,
                        line.len()
                    );
                }
            }
            Err(err) => {
                // counterは持たない（module doc「停止理由の区別」参照）。
                // `generate_sid`と同じくlogだけは分類しておく。
                log::error!("boot_encode_failed error={err}");
            }
        }
    }
}

/// `Duration`を`u64` msへ切り詰める。`health.uptime_ms()`と同じ単位に揃える。
fn duration_to_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// 残り時間から、`UartDriver::read`へ渡す`TickType_t`を作る。
///
/// **切り上げる。**1ms以上は`TickType::new_millis`（esp-idf-hal 0.46.2
/// `delay.rs`89〜95行）が切り上げる。0msは0 tick（`delay::NON_BLOCK`）になり
/// `UartRxDriver::read`が即時returnする（`uart.rs`1211〜1217行）。**main loopは0 msを渡さず、
/// 1 ms以上へ丸めてから渡す**（`#487`。0 tickの`read`はyieldせずに戻るため。`main.rs`のmain
/// loopのcomment）。
pub fn ticks_until(remaining: Duration) -> esp_idf_svc::hal::delay::TickType_t {
    TickType::from(remaining).into()
}
