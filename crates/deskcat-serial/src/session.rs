//! 接続stateと、上限付きのread／writeを持つsession。
//!
//! **domain動作を含めない。**感情、性格、行動判断、独り言はここに入らない。
//! 公開するのはconnection stateとcounterだけである（Issue #11の目的文）。
//!
//! session遷移の確定、duplicate履歴、受理budgetは**この層では扱わない**。
//! それらは受信側のstateを要し、Issue #12の範囲である。

use core::time::Duration;
use std::io;

use deskcat_protocol::{
    Envelope, Frame, LineReceiver, Message, Outcome, encode_line, error::DecodeError,
};

use crate::config::SerialConfig;
use crate::ids::{IdAllocator, IdSpaceExhausted};
use crate::outbox::{Enqueued, Outbox};
use crate::transport::{IoDisposition, Transport};

/// 接続state。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConnectionState {
    /// 未接続。再接続の対象である。
    Disconnected,
    /// 接続済み。
    Connected,
    /// 停止した。**自動では再開しない。**
    Stopped(StopReason),
}

/// 停止した理由。
///
/// いずれも仕様が定める停止状態に対応する。復帰はprocessの再起動、または
/// 運用者の明示的なsession resetによる（§3.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StopReason {
    /// `id`空間を使い切った（§3、`PROTO-TBD-003`）。
    IdSpaceExhausted,
    /// 再接続の試行回数が上限に達した。
    ReconnectExhausted,
    /// 再接続で直らないI/O error（権限、device不在など）。
    Fatal,
}

/// 1回のpumpで起きたこと。
///
/// **errorを握りつぶさないため、分類した結果を返す。**呼び出し側は
/// `io::Error`を直接扱わず、この分類に対して動く。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "分類を見ずに捨てると、切断と一時的な失敗を区別できない"]
#[non_exhaustive]
pub enum Pump {
    /// byteが動いた。
    Progress(usize),
    /// 今は進められない。同じ操作を後で再試行してよい。
    Idle,
    /// timeoutした。切断とは区別する。
    TimedOut,
    /// 切断を検知した。stateは[`ConnectionState::Disconnected`]へ移る。
    Disconnected,
    /// 再接続で直らない。stateは[`ConnectionState::Stopped`]へ移る。
    Fatal,
}

/// 観測できるcounter。
///
/// **握りつぶさず、分類・log・counterを用意する**という変更規則に対応する。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SessionCounters {
    /// 読み込んだbyte数。
    pub bytes_in: u64,
    /// 書き出したbyte数。
    pub bytes_out: u64,
    /// 復元できたframe数。
    pub frames_in: u64,
    /// 拒否したline数（分類は[`deskcat_protocol::Rejection`]が持つ）。
    pub rejected_in: u64,
    /// 送信queueが**溢れて**捨てた件数。輻輳の指標である。
    pub dropped_out: u64,
    /// **切断で**破棄した保留送信の件数。linkの障害の指標であり、
    /// `dropped_out`とは原因が違うため分けて数える。
    pub discarded_on_disconnect: u64,
    /// 切断を検知した回数。
    pub disconnects: u64,
    /// 再接続を試みた回数。
    pub reconnect_attempts: u64,
    /// timeoutした回数。
    pub timeouts: u64,
    /// 進捗が無く再試行になった回数。
    pub retries: u64,
    /// 送出前の自己検証で送信を却下した回数（encode失敗と空payload）。
    /// **`dropped_out`とは原因が違う。**あちらは輻輳、こちらは自分のmessageが
    /// wire規則を満たさないことを表す。継続的に失敗している状況を外から観測できる
    /// ようにするため分けて数える。
    pub encode_failed: u64,
}

/// 送信を受け付けられなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SendError {
    /// `id`空間を使い切った。新しい`(sid, id)`は作れない。
    IdSpaceExhausted,
    /// 送信queueが満杯だったため捨てた。
    Dropped,
    /// sessionが停止している。
    Stopped(StopReason),
    /// encodeに失敗した。**送出前に自分のmessageを検証している。**
    Encode(DecodeError),
    /// encode結果が空だった。内部経路では起こらないが、握りつぶさず分類する。
    EmptyPayload,
}

impl core::fmt::Display for SendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IdSpaceExhausted => f.write_str("idの上限に達した"),
            Self::Dropped => f.write_str("送信queueが満杯のため捨てた"),
            Self::Stopped(reason) => write!(f, "sessionが停止している: {reason:?}"),
            Self::Encode(err) => write!(f, "encodeに失敗した: {err}"),
            Self::EmptyPayload => f.write_str("encode結果が空だった"),
        }
    }
}

impl core::error::Error for SendError {}

impl From<IdSpaceExhausted> for SendError {
    fn from(_: IdSpaceExhausted) -> Self {
        Self::IdSpaceExhausted
    }
}

/// Host側のserial session。
#[derive(Debug)]
pub struct Session {
    config: SerialConfig,
    sid: u32,
    ids: IdAllocator,
    receiver: LineReceiver,
    outbox: Outbox,
    /// linkが繋がっているか。**停止とは独立に持つ。**`id`枯渇は送出の停止で
    /// あってlinkの切断ではなく、両者を1つのfieldに畳むと、停止後に終端報告を
    /// 書き切れなくなる。
    link_connected: bool,
    /// 書き切ったmessageの`flush`が未完了か。
    /// **`WouldBlock`や`TimedOut`は再試行できるerrorであり、1回失敗しただけでは
    /// bufferの中身は消えない。**flagを残さないと、outboxが空になった後は
    /// `flush`を呼ぶ契機が無くなり、byteがbufferに残ったままになる。
    flush_pending: bool,
    /// 停止した理由。停止していなければ`None`。
    stopped: Option<StopReason>,
    counters: SessionCounters,
    reconnect_attempts: u32,
    read_buf: Vec<u8>,
}

impl Session {
    /// 1回のreadで受け取るbyte数の上限。
    ///
    /// 行長上限（`MAX_LINE_BYTES`）とは別物である。**受信bufferの上限は
    /// [`LineReceiver`]が型の性質として持つ**ため、ここで行長を再実装しない。
    const READ_CHUNK: usize = 512;

    /// 設定と`sid`からsessionを作る。
    ///
    /// `sid`の生成方法と衝突許容確率は`PROTO-TBD-011`であり、**このcrateは決めない。**
    /// 呼び出し側が選んだ値を受け取る。
    #[must_use]
    pub fn new(config: SerialConfig, sid: u32) -> Self {
        Self::with_first_id(config, sid, 1)
    }

    /// 採番の初期値を指定してsessionを作る。
    ///
    /// 通常は[`Self::new`]を使う。これを分けてあるのは、**`id`空間の境界の
    /// 振る舞いを試験できるようにするため**である。上限まで4,294,967,295件を
    /// 実際に送るtestは書けない。
    #[must_use]
    pub fn with_first_id(config: SerialConfig, sid: u32, first_id: u32) -> Self {
        let outbox = Outbox::new(config.outbox_capacity());
        Self {
            config,
            sid,
            ids: IdAllocator::starting_at(first_id),
            receiver: LineReceiver::with_protocol_limit(),
            outbox,
            link_connected: false,
            flush_pending: false,
            stopped: None,
            counters: SessionCounters::default(),
            reconnect_attempts: 0,
            read_buf: vec![0_u8; Self::READ_CHUNK],
        }
    }

    /// 現在の接続state。
    #[must_use]
    pub const fn state(&self) -> ConnectionState {
        match self.stopped {
            Some(reason) => ConnectionState::Stopped(reason),
            None if self.link_connected => ConnectionState::Connected,
            None => ConnectionState::Disconnected,
        }
    }

    /// linkが繋がっているか。
    ///
    /// 停止していても`true`でありうる。`id`枯渇で停止したsessionは、
    /// 予約した`id`で作った終端報告を書き切る必要がある。
    #[must_use]
    pub const fn link_connected(&self) -> bool {
        self.link_connected
    }

    /// counterの現在値。
    #[must_use]
    pub const fn counters(&self) -> SessionCounters {
        self.counters
    }

    /// 設定。
    #[must_use]
    pub const fn config(&self) -> &SerialConfig {
        &self.config
    }

    /// このsessionの`sid`。
    #[must_use]
    pub const fn sid(&self) -> u32 {
        self.sid
    }

    /// 保留中の送信件数。
    #[must_use]
    pub fn pending_out(&self) -> usize {
        self.outbox.len()
    }

    /// transportが確立したことを通知する。
    ///
    /// **実deviceのopenはこのcrateが行わない。**呼び出し側がtransportを用意し、
    /// ここでstateを揃える。再接続の試行回数はここでresetする。
    pub fn note_connected(&mut self) {
        if self.stopped.is_some() {
            return;
        }
        self.link_connected = true;
        self.reconnect_attempts = 0;
        // port再openとreconnectでは受信中の断片を捨てる（§10）。
        // 途中まで読んだlineの続きが、新しい接続の先頭と繋がってはならない。
        self.receiver.reset();
    }

    /// 切断を記録する。
    fn note_disconnected(&mut self) {
        if self.link_connected {
            self.counters.disconnects += 1;
        }
        self.link_connected = false;
        self.receiver.reset();
        // 切断したlinkへはflushできない。保留を持ち越すと再接続後に
        // 別のlinkへ対して意味のないflushを1回呼ぶことになる。
        self.flush_pending = false;
        self.counters.discarded_on_disconnect += self.outbox.clear();
    }

    fn stop(&mut self, reason: StopReason) {
        // 最初の理由を残す。後から起きた失敗で上書きすると、根本の原因が
        // 見えなくなる（`id`枯渇で止まった後のI/O errorなど）。
        let _ = self.stopped.get_or_insert(reason);
    }

    /// pumpを回してよいか。**linkが繋がっていることだけを見る。**
    ///
    /// `id`枯渇で停止していても、queueへ積んだ終端報告は書き切る。
    /// 予約した`id`を送れないなら、予約する意味が無い。
    const fn can_pump(&self) -> bool {
        self.link_connected
    }

    /// 次の再接続まで待つ時間を返す。上限に達していれば`None`。
    ///
    /// `None`を返したときsessionは[`StopReason::ReconnectExhausted`]で停止する。
    /// **上限後も自動試行を続けると、上限が実質的に無くなる。**
    pub fn begin_reconnect(&mut self) -> Option<Duration> {
        if self.stopped.is_some() {
            return None;
        }
        let policy = self.config.reconnect();
        if self.reconnect_attempts >= policy.max_attempts() {
            self.stop(StopReason::ReconnectExhausted);
            return None;
        }
        let backoff = policy.backoff(self.reconnect_attempts);
        self.reconnect_attempts += 1;
        self.counters.reconnect_attempts += 1;
        Some(backoff)
    }

    /// これまでの再接続試行回数。
    #[must_use]
    pub const fn reconnect_attempts(&self) -> u32 {
        self.reconnect_attempts
    }

    /// messageを送信queueへ入れる。割り当てた`id`を返す。
    ///
    /// `ts_ms`は送信側のuptime（milliseconds）である。wall-clock timeではない（§3）。
    ///
    /// # Errors
    ///
    /// - `id`空間を使い切った場合は[`SendError::IdSpaceExhausted`]を返し、
    ///   sessionを[`StopReason::IdSpaceExhausted`]で停止させる。
    /// - queueが満杯なら[`SendError::Dropped`]。
    /// - encodeに失敗したら[`SendError::Encode`]。
    pub fn send(&mut self, message: Message, ts_ms: u64) -> Result<u32, SendError> {
        if let Some(reason) = self.stopped {
            return Err(SendError::Stopped(reason));
        }
        let id = match self.ids.peek_next() {
            Ok(id) => id,
            Err(exhausted) => {
                self.stop(StopReason::IdSpaceExhausted);
                return Err(exhausted.into());
            }
        };
        // **queueへ入ってから`id`を消費する。**先に消費すると、encode失敗や
        // queue満杯のたびに`id`が減る。仕様は単調増加を求めるが連続は求めない
        // ため飛びは問題ないが、失敗を繰り返すと上限へ早く到達する。
        self.encode_and_enqueue(id, ts_ms, message)?;
        self.ids.commit(id);
        Ok(id)
    }

    /// 終端報告を、予約しておいた上限値の`id`で1件だけ送る。
    ///
    /// 仕様§3の「終端報告のために最後の1件を残す」に対応する。呼び出し側は
    /// `id`空間を使い切ったあと、`protocol_fault`に相当するmessageをここへ渡す。
    ///
    /// # Errors
    ///
    /// 予約を既に使っている場合は[`SendError::IdSpaceExhausted`]を返す。
    pub fn send_terminal(&mut self, message: Message, ts_ms: u64) -> Result<u32, SendError> {
        let Some(id) = self.ids.peek_terminal() else {
            return Err(SendError::IdSpaceExhausted);
        };
        // **queueへ入ってから予約を消費する。**先に消費すると、queueが満杯だった
        // 場合に予約を失ったまま終端報告を送れなくなる。予約は1件しかなく、
        // 取り戻せない。呼び出し側はqueueが掃けてから再試行できる。
        self.encode_and_enqueue(id, ts_ms, message)?;
        let taken = self.ids.take_terminal();
        debug_assert_eq!(taken, Some(id), "覗いた予約と消費した予約が一致する");
        Ok(id)
    }

    fn encode_and_enqueue(
        &mut self,
        id: u32,
        ts_ms: u64,
        message: Message,
    ) -> Result<(), SendError> {
        let frame = Frame::new(
            Envelope {
                v: deskcat_protocol::limits::PROTOCOL_VERSION,
                sid: self.sid,
                id,
                ts_ms,
            },
            message,
        );
        let line = match encode_line(&frame) {
            Ok(line) => line,
            Err(err) => {
                // 行長上限超過などで失敗しうる。counterが無いと、送信が継続的に
                // 失敗している状況を外から観測できない。
                self.counters.encode_failed += 1;
                return Err(SendError::Encode(err));
            }
        };
        match self.outbox.enqueue(line.into_bytes()) {
            Enqueued::Accepted => Ok(()),
            Enqueued::Dropped => {
                self.counters.dropped_out = self.outbox.dropped();
                Err(SendError::Dropped)
            }
            // `encode_line`は必ず改行を含む行を返すため、内部経路では起こらない。
            // 起きたならencode側の契約が変わっている。握りつぶさず分類して返す。
            Enqueued::Empty => {
                self.counters.encode_failed += 1;
                Err(SendError::EmptyPayload)
            }
        }
    }

    /// transportから読み、復元できたものを`on_outcome`へ渡す。
    ///
    /// **`Ok(0)`はEOFであり、「今はdataが無い」ではない。**切断として扱う。
    pub fn pump_read<T: Transport>(
        &mut self,
        transport: &mut T,
        mut on_outcome: impl FnMut(Outcome),
    ) -> Pump {
        if !self.can_pump() {
            return Pump::Idle;
        }
        match transport.read(&mut self.read_buf) {
            Ok(0) => {
                self.note_disconnected();
                Pump::Disconnected
            }
            Ok(n) => {
                self.counters.bytes_in += n as u64;
                let mut frames = 0_u64;
                let mut rejected = 0_u64;
                self.receiver.drain(&self.read_buf[..n], |outcome| {
                    match outcome {
                        Outcome::Frame(_) => frames += 1,
                        Outcome::Rejected(_) => rejected += 1,
                    }
                    on_outcome(outcome);
                });
                self.counters.frames_in += frames;
                self.counters.rejected_in += rejected;
                Pump::Progress(n)
            }
            Err(err) => self.handle_io_error(&err),
        }
    }

    /// 送信queueの先頭を書き出す。partial writeを進捗として記録する。
    ///
    /// **`write_all`へ丸投げしない。**途中で失敗したとき何byte出たか分からなくなり、
    /// 再送で重複したbyte列を送ることになる。
    pub fn pump_write<T: Transport>(&mut self, transport: &mut T) -> Pump {
        if !self.can_pump() {
            return Pump::Idle;
        }
        // 前回の書き切りで`flush`が終わっていなければ、先に片付ける。
        // **outboxが空になると書き込み経路を通らなくなる。**ここで再試行しないと
        // 再試行できるerrorで落ちたflushが永久に取り残される。
        if self.flush_pending {
            match transport.flush() {
                Ok(()) => self.flush_pending = false,
                Err(err) => return self.handle_io_error(&err),
            }
        }
        // 書き出しの結果だけを取り出し、`outbox`のborrowをここで終える。
        // **copyしない。**1回に1byteしか書けない相手では`pump_write`が
        // byte数だけ呼ばれるため、毎回のcopyは行長に対して二乗の仕事になる。
        let result = {
            let Some(chunk) = self.outbox.peek() else {
                return Pump::Idle;
            };
            transport.write(chunk)
        };
        match result {
            Ok(0) => {
                // 書けるはずの状況で0が返るのは、下層が受け付けなくなったことを表す。
                self.note_disconnected();
                Pump::Disconnected
            }
            Ok(n) => {
                let pending_before = self.outbox.len();
                self.outbox.advance(n);
                self.counters.bytes_out += n as u64;
                // 先頭messageを書き切った時点で`flush`する。**bufferを持つtransport
                // では、`write`が受け取っただけでbyteはbufferに残る。**flushしないと
                // `bytes_out`が増えて`pending_out`が0になり、呼び出し側からは
                // 送り切ったように見える。
                if self.outbox.len() < pending_before {
                    self.flush_pending = true;
                }
                if self.flush_pending {
                    match transport.flush() {
                        Ok(()) => self.flush_pending = false,
                        // **byteは既に`write`が受け取っており、取り消せない。**
                        // 進捗の記録とflag は残したまま、失敗の分類だけを返す。
                        Err(err) => return self.handle_io_error(&err),
                    }
                }
                Pump::Progress(n)
            }
            Err(err) => self.handle_io_error(&err),
        }
    }

    fn handle_io_error(&mut self, err: &io::Error) -> Pump {
        match IoDisposition::classify(err) {
            IoDisposition::Retry => {
                self.counters.retries += 1;
                Pump::Idle
            }
            IoDisposition::TimedOut => {
                self.counters.timeouts += 1;
                Pump::TimedOut
            }
            IoDisposition::Disconnected => {
                self.note_disconnected();
                Pump::Disconnected
            }
            IoDisposition::Fatal => {
                // 再接続で直らない。linkも使えないものとして落とす。
                self.note_disconnected();
                self.stop(StopReason::Fatal);
                Pump::Fatal
            }
        }
    }
}
