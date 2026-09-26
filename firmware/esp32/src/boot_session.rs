//! `boot`の受理確認・再送（§4.1）と、`sid`選び直し（§3.1「`sid`が衝突した場合」）。
//!
//! `pi-protocol-mode`でだけ使う（`#446` PR B）。`main.rs`の`generate_sid`が選んだ`sid`で
//! `boot`を送り、UART0から届く`ack`を待つ。§4.1の終了条件の表が定める**state遷移**
//! （いつ再送するか、いつ`sid`を選び直すか、いつ止めるか）を実装する狙いで
//! 書いたものであり、**state遷移が§4.1の表どおりに正しいことをhost側の
//! unit testでは確かめていない**（`firmware/esp32`はhostのworkspaceから
//! 除外されており、`esp_idf_svc`の型へ直接依存するため）。3構成
//! （既定・`pi-protocol-mode`・`bringup-display-13`）でのbuild（`cargo build`／
//! `cargo clippy`）が通ることは、compileが通ることの根拠であり、**state遷移が
//! 正しいことの根拠ではない。**state遷移の正しさの根拠は、この`boot_session.rs`
//! を導入したPull Request（`#446` PR B）の自己レビュー記録（複数回のcode
//! review。AIによる査読を含む。PR本文参照）だけである。**host testと実機確認
//! （受信経路がsoftware ring buffer（`config::PI_PROTOCOL_UART_RX_BUFFER_BYTES`）
//! を溢れなく動くか、実際に`boot`→ACKが成立するか）が前提として残る
//! （Issue #446の追跡を見る）。
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
//! `pi-protocol-mode`のmain loopが`crate::servo`を呼ぶ箇所を持たない
//! （`main.rs`の`#[cfg(feature = "pi-protocol-mode")]`区間を目視で確認した。
//! `mod servo`自体はfeature条件無しでcompileされるが、呼び出しは
//! `bench-servo-test-17` featureの下にしかない）ため満たされる。網羅的な
//! 呼び出し元探索（例: 静的解析）はしていない。
//!
//! # 停止理由の区別
//!
//! 停止理由は[`TerminalReason`]（[`Phase::Terminated`]が保持する）として内部stateに
//! 持つ。`pi-protocol-mode`はloggingを止めているため出力されず、`Health`の
//! `ProtocolCounters`もまだwireへ送る経路が無い（`#12`）。したがって`counter`は
//! 増やさない——増やしても観測経路が無いcounterになる（`#448`で同じ理由から
//! `boot_serialize_errors`を削除した判断に揃えた）。`TerminalReason`として
//! 区別だけは保持し、将来`status`をwireへ送る段になったら`ProtocolCounters`へ写せる形に
//! しておく。

use std::time::Duration;

use deskcat_protocol::{
    encode_line, limits, Ack, AckStatus, Boot, Cause, Envelope, ErrorCode, Frame, LineReceiver,
    Message, Outcome,
};
use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::uart::UartDriver;

use crate::health::Health;

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
    /// §4.1の表は、`RecoveryBudgetExhausted`で終端した場合に限り、Piから
    /// 有効な`hello`を受信したら同じ`(sid, id)`のまま1回だけ再開してよいと
    /// 定めている（`hello`はESP32が受信する側のmessageであり、`crate::protocol`の
    /// `PiSession::handle_hello`が別途実装しているが、`pi-protocol-mode`の
    /// runtime経路（`main.rs`のmain loop）へは配線していない）。**この実装は
    /// 再開を使わない。**§4.1の規定は「再開してよい」であって「しなければ
    /// ならない」ではなく（許可であり義務ではない）、再開しないことは
    /// より厳しい側の選択であるため、どのMUSTにも反しない。それ以外の
    /// 終端理由（`RateLimitedBudgetExhausted`・`SidReselectLimitReached`・
    /// `Rejected`）には、そもそも再開の規定が無い。以降、processの再起動
    /// までは自動では再開しない（`pi-protocol-mode`は`Ack`以外を受け付けない
    /// ため、運用者が明示的にsession resetを指示する受信経路も無い）。
    Terminated(TerminalReason),
}

/// 終端に至った理由。ログへは出ないが、区別だけ内部stateに残す（module doc参照）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // 現状は観測経路が無いため書き込まれるだけで読まれない。将来のstatus送出に備えて保持する。
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
    receiver: LineReceiver,
}

impl BootSession {
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
            id: 1,
            reset_reason,
            phase: Phase::Normal { attempt: 0 },
            next_deadline_ms: 0,
            sid_reselect_count: 0,
            receiver: LineReceiver::with_protocol_limit(),
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

    /// UART0から読めた生byteを渡す。0 byteでもよい（timeoutで戻った場合）。
    ///
    /// 自分宛でないframe（`(sid, id)`が一致しないACK等）と、壊れた行
    /// （`Outcome::Rejected`）は捨てる。**§4.1の終了条件はACKの到達・不到達で
    /// 決まるものであり、受信できた壊れたbyte列の量では決めない。**壊れた行を
    /// カウントして独自の終端条件を作ると、仕様に無い停止経路を持ち込むことになる。
    ///
    /// # `Ack`以外のmessage（`hello`／`get_status`等）を処理せずに捨てる
    ///
    /// §8はidentityを復元できる要求に相関ACKを返すよう定めるが、この実装は
    /// `Ack`以外を一切処理しない（応答もしない）。**これは§8に反する既知の
    /// 逸脱として扱う。**義務は受信側の規範であり、実装しないことで消える
    /// ものではない。`pi-protocol-mode`はPi→ESP32方向のrequest処理
    /// （`hello`・`get_status`等の受理）を実装していない——`crate::protocol`の
    /// `PiSession`は既定buildだけでcompileされ（`main.rs`の`mod protocol`は
    /// `#[cfg(not(feature = "pi-protocol-mode"))]`）、`pi-protocol-mode`の
    /// buildにはそもそも存在しない。この逸脱を解消するにはhello／get_status
    /// 処理自体を`pi-protocol-mode`へ実装する必要があり、このPRの範囲外である
    /// （console.rsのmodule doc「実際のPi hostへ接続しないこと」の理由でもある）。
    ///
    /// **§7「Parser counterでは…を区別する」の義務も満たしていない。**確立前
    /// （`Normal`／`Recovery`／`RateLimited`）に受けたbyteに限り、`Ack`以外の
    /// frame・`Outcome::Rejected`の`Cause`（`InvalidUtf8`／`Decode`／`Oversize`）・
    /// session不一致のACKを`log::error!`で分類するが、`ProtocolCounters`へ実際に
    /// 加算する経路は無い（`Health::counters`を増やす経路が無いことは
    /// `health.rs`のmodule doc参照。`send_boot`・`generate_sid`のエラー分類と
    /// 同じ扱い）。**`Established`／`Terminated`へ移った後に届いたbyteは、
    /// `on_bytes`冒頭で`receiver.drain`へ通さずreturnするため、種類を問わず
    /// 未分類のまま読み捨てる**（確立後の受信を分類する処理は、hello処理と
    /// 同じくこのPRの範囲外）。`pi-protocol-mode`はloggingを止めているため、
    /// 分類したものも外部からは観測できない。
    ///
    /// # Envelopeの`sid`を検査しない理由
    ///
    /// §6は、ACKの受理条件としてEnvelopeの`sid`（応答送信側=Piのsession）も
    /// 確認対象に含めるが、この実装は`reply_sid`／`reply_to`（要求送信側=ESP32の
    /// sid／id）しか見ない。`boot`確立前はESP32がまだ「現在承認しているPi
    /// session」を持たない（確立するのがこの`boot`のACKそのものであるため）ため、
    /// この条件は適用対象が無い。確立後（`Established`）はこのmoduleが受信を
    /// 止めるため、この検査を要する局面に達しない。
    pub fn on_bytes(&mut self, buf: &[u8], health: &Health, uart: &mut UartDriver<'_>) {
        if matches!(self.phase, Phase::Established | Phase::Terminated(_)) {
            return;
        }
        // `drain`のclosureは`&mut self`を借りられないため、`Ack`だけを
        // 取り出してloopの外で適用する。**受信順を保って全件検査する。**1回の
        // `read`chunkに複数のACKが含まれる場合がありうる（例: 古い
        // `rate_limited`の重複ACKと、それに続く新しい`ok`）。
        let mut acks: Vec<Ack> = Vec::new();
        self.receiver.drain(buf, |outcome| match outcome {
            Outcome::Frame(Frame {
                message: Message::Ack(ack),
                ..
            }) => acks.push(ack),
            // 分類の理由は`on_bytes`のdoc「Ack以外のmessageを処理せずに捨てる」参照。
            Outcome::Frame(other) => {
                log::error!("boot_rx_unhandled_frame type={}", other.message.type_str());
            }
            Outcome::Rejected(rejection) => match rejection.cause() {
                Cause::InvalidUtf8 { valid_up_to } => {
                    log::error!("boot_rx_invalid_utf8 valid_up_to={valid_up_to}");
                }
                Cause::Decode => log::error!("boot_rx_decode_rejected"),
                Cause::Oversize => log::error!("boot_rx_oversize_line"),
                _ => log::error!("boot_rx_rejected_unknown_cause"),
            },
        });
        for ack in acks {
            // 直前の適用で`Established`／`Terminated`へ移っていたら、以降の
            // ACKは適用しない（`on_bytes`冒頭と同じ判定）。§7「Parser counterでは
            // …を区別する」の対象として分類だけしておく。
            if matches!(self.phase, Phase::Established | Phase::Terminated(_)) {
                log::error!(
                    "boot_rx_ack_after_close reply_sid={} reply_to={}",
                    ack.reply_sid,
                    ack.reply_to
                );
                continue;
            }
            // **`self.sid`／`self.id`を、loopの外で1回だけ捕えたものではなく
            // ここで毎回読み直す。**同じchunk内の先行ACKが`stale_session`で
            // `reselect_sid`を起こすと`self.sid`／`self.id`が変わりうるため、
            // それより後ろのACKは新しい値と照合しないと、旧`(sid, id)`宛の
            // ACK（例: 旧sidへの`ok`）を現在のsessionへ誤って適用しうる。
            // 一致しないACKは適用しない。**`generate_sid`が前回と異なる値を
            // 返す保証は無い**（NVS counterの通常経路は単調増加で異なる値になるが、
            // `generate_sid`のdoc「NVSが使えない場合」の縮退経路（uptime依存）へ
            // 落ちた場合、たまたま前回と同じ値になりうる）。同じ値が返った場合、
            // 旧`(sid, id)`宛の重複`stale_session`は次回も一致してしまい、
            // 選び直しの回数（`sid_reselect_limit`）を消費し続ける。これは
            // 縮退経路自体が非衝突を主張しないという既存の制約の一部として扱う。
            if ack.reply_sid == self.sid && ack.reply_to == self.id {
                self.apply_ack(ack, health, uart);
            } else {
                // §7「Parser counterでは…session不一致を区別する」の対象。
                // 観測経路は無いが`generate_sid`等と同じ理由でlogだけ分類する。
                log::error!(
                    "boot_rx_session_mismatch reply_sid={} reply_to={}",
                    ack.reply_sid,
                    ack.reply_to
                );
            }
        }
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
                                // `on_bytes`の先頭で弾かれるため到達しない。
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
        self.sid = crate::generate_sid(health);
        self.id = 1;
        self.phase = Phase::Normal { attempt: 0 };
        let now = health.uptime_ms();
        self.next_deadline_ms = now + duration_to_ms(self.policy.initial_interval);
        self.send_boot(health, uart);
    }

    fn terminate(&mut self, reason: TerminalReason) {
        self.phase = Phase::Terminated(reason);
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
                // §2は行の分断を禁じている。この保証は、esp-idf-hal
                // `UartDriver::write`が呼ぶ`uart_write_bytes`→`uart_tx_all`が
                // `portMAX_DELAY`でblockし、1回の呼び出しで渡した全byteを
                // tx ring bufferへ積み終える（wireへ送り終えるまでではない）か
                // （成功時は常に`Ok(bytes.len())`）、入力検証エラーで即座に
                // 失敗するかのどちらかであること（`esp_driver_uart/src/uart.c`
                // 1562〜1631行で確認済み。部分書き込みで戻る経路が無い）に依る。
                // **このloopが部分書き込みを繰り返して完了させているわけではない。**
                // `Ok(0)`／`Err`は現状の実装では実質到達しないが、型として
                // 有り得る以上、到達したら送出を諦める（次の再送に委ねる。
                // 行の途中で戻り値を無視して送り続けることはしない）。
                let bytes = line.as_bytes();
                let mut written = 0;
                while written < bytes.len() {
                    match uart.write(&bytes[written..]) {
                        Ok(0) => {
                            // これ以上進まない。送出を諦める（次の再送に委ねる）。
                            log::error!("boot_uart_write_stalled written={written}");
                            break;
                        }
                        Ok(n) => written += n,
                        Err(err) => {
                            log::error!("boot_uart_write_failed error={err} written={written}");
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                // 観測経路が無いためcounterは持たない（module doc「停止理由の区別」参照）。
                // `generate_sid`と同じ理由でlogだけは分類しておく。
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
/// `UartRxDriver::read`が即時returnする（`uart.rs`1211〜1217行）。呼び出し側で
/// 「締切に達した」場合は`Duration::ZERO`を渡し、即時returnさせる。
pub fn ticks_until(remaining: Duration) -> esp_idf_svc::hal::delay::TickType_t {
    TickType::from(remaining).into()
}
