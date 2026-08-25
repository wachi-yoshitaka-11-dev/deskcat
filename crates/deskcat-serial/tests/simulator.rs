//! Serial simulator test。
//!
//! 受け入れ条件は「**Serial simulator testで一般的なfailureを網羅する**」であり、
//! 「動く場合」だけのtestにしない。ここで再現するのは次である。
//!
//! - 1byteずつしか読めない／書けない（partial I/O）
//! - lineの途中でEOF（`Ok(0)`）
//! - `WouldBlock`／`Interrupted`／`TimedOut`／`BrokenPipe`／`PermissionDenied`
//! - 行長上限を超えた行が流れてくる
//! - 送信queue満杯時のdropとcounter増加
//! - reconnect試行が上限に達して停止する
//!
//! 実deviceは開かない。[`Transport`]へfakeを注入する。

use core::time::Duration;
use std::collections::VecDeque;
use std::io;

use deskcat_protocol::{
    Cause, Envelope, Frame, Hello, HelloReason, Message, Outcome, encode_line, limits,
};
use deskcat_serial::{
    ConfigError, ConnectionState, Enqueued, Outbox, Pump, ReconnectPolicy, SendError, SerialConfig,
    Session, StopReason, Transport,
};

/// 台本どおりに振る舞うfake transport。
#[derive(Debug, Default)]
struct Sim {
    /// `read`が1回ごとに返すもの。
    reads: VecDeque<io::Result<Vec<u8>>>,
    /// `write`が1回で受け付ける最大byte数。`None`は全部受け付ける。
    write_chunk: Option<usize>,
    /// `write`が返すerror。先頭から消費する。
    write_errors: VecDeque<io::Error>,
    /// 実際に書けたbyte列。
    written: Vec<u8>,
    /// `flush`が呼ばれた回数。**bufferを持つtransportでは、`write`が受け取った
    /// だけではbyteは届かない。**呼ばれていないことを検出できるようにする。
    flushes: usize,
    /// `flush`が返すerror。先頭から消費する。
    flush_errors: VecDeque<io::Error>,
}

impl Sim {
    fn with_reads(reads: Vec<io::Result<Vec<u8>>>) -> Self {
        Self {
            reads: reads.into(),
            ..Self::default()
        }
    }

    /// 入力を1byteずつ配る台本を作る。
    fn byte_at_a_time(input: &[u8]) -> Self {
        Self::with_reads(input.iter().map(|b| Ok(vec![*b])).collect())
    }
}

impl Transport for Sim {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.reads.pop_front() {
            Some(Ok(bytes)) => {
                let n = bytes.len().min(buf.len());
                buf[..n].copy_from_slice(&bytes[..n]);
                // **入りきらなかった分を捨てない。**捨てるfakeは実物より
                // 都合がよく、行長上限のような境界のtestを素通りさせる。
                if n < bytes.len() {
                    self.reads.push_front(Ok(bytes[n..].to_vec()));
                }
                Ok(n)
            }
            Some(Err(err)) => Err(err),
            // 台本を使い切ったら「今はdataが無い」を表す。EOFと混ぜない。
            None => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        }
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(err) = self.write_errors.pop_front() {
            return Err(err);
        }
        let n = self
            .write_chunk
            .map_or(buf.len(), |chunk| chunk.min(buf.len()));
        self.written.extend_from_slice(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        if let Some(err) = self.flush_errors.pop_front() {
            return Err(err);
        }
        Ok(())
    }
}

fn config() -> SerialConfig {
    // device名は台本の中だけの値である。実機のdevice名は未確認であり、
    // ここで確定させない。
    SerialConfig::new("/dev/simulated", 115_200).expect("設定は妥当である")
}

fn connected_session() -> Session {
    let mut session = Session::new(config(), 90_312);
    session.note_connected();
    session
}

fn hello() -> Message {
    Message::Hello(Hello {
        host: "deskcatd".to_owned(),
        version: "0.1.0".to_owned(),
        reason: HelloReason::Startup,
    })
}

fn ping_line(id: u32) -> String {
    let frame = Frame::new(
        Envelope {
            v: limits::PROTOCOL_VERSION,
            sid: 41_207,
            id,
            ts_ms: 100,
        },
        Message::Ping,
    );
    encode_line(&frame).expect("encodeできる")
}

/// 1byteずつしか読めなくてもlineは復元される。
#[test]
fn a_line_delivered_one_byte_at_a_time_is_reassembled() {
    let line = ping_line(7);
    let mut sim = Sim::byte_at_a_time(line.as_bytes());
    let mut session = connected_session();

    let mut frames = Vec::new();
    for _ in 0..line.len() {
        let _ = session.pump_read(&mut sim, |outcome| {
            if let Outcome::Frame(frame) = outcome {
                frames.push(frame);
            }
        });
    }

    assert_eq!(frames.len(), 1, "分割されても1件だけ復元される");
    assert_eq!(frames[0].envelope.identity(), (41_207, 7));
    assert_eq!(session.counters().frames_in, 1);
    assert_eq!(session.state(), ConnectionState::Connected);
}

/// `Ok(0)`はEOFである。「今はdataが無い」と読み違えない。
#[test]
fn eof_in_the_middle_of_a_line_is_a_disconnect_not_a_lack_of_data() {
    let line = ping_line(1);
    let half = &line.as_bytes()[..line.len() / 2];
    let mut sim = Sim::with_reads(vec![Ok(half.to_vec()), Ok(Vec::new())]);
    let mut session = connected_session();

    assert!(matches!(
        session.pump_read(&mut sim, |_| {}),
        Pump::Progress(_)
    ));
    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::Disconnected);
    assert_eq!(session.state(), ConnectionState::Disconnected);
    assert_eq!(session.counters().disconnects, 1);
    assert_eq!(session.counters().frames_in, 0, "途中の行は復元されない");
}

/// `WouldBlock`と`Interrupted`は再試行であって切断ではない。
#[test]
fn wouldblock_and_interrupted_are_retryable_and_do_not_disconnect() {
    let mut sim = Sim::with_reads(vec![
        Err(io::Error::from(io::ErrorKind::WouldBlock)),
        Err(io::Error::from(io::ErrorKind::Interrupted)),
    ]);
    let mut session = connected_session();

    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::Idle);
    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::Idle);

    assert_eq!(session.state(), ConnectionState::Connected);
    assert_eq!(session.counters().disconnects, 0);
    assert_eq!(session.counters().retries, 2);
}

/// timeoutは切断と区別する。
#[test]
fn timed_out_is_distinguished_from_a_disconnect() {
    let mut sim = Sim::with_reads(vec![Err(io::Error::from(io::ErrorKind::TimedOut))]);
    let mut session = connected_session();

    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::TimedOut);
    assert_eq!(session.state(), ConnectionState::Connected);
    assert_eq!(session.counters().timeouts, 1);
    assert_eq!(session.counters().disconnects, 0);
}

/// `BrokenPipe`は再接続の対象、`PermissionDenied`は再接続で直らない。
#[test]
fn broken_pipe_disconnects_while_permission_denied_stops_the_session() {
    let mut sim = Sim::with_reads(vec![Err(io::Error::from(io::ErrorKind::BrokenPipe))]);
    let mut session = connected_session();
    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::Disconnected);
    assert_eq!(session.state(), ConnectionState::Disconnected);

    let mut sim = Sim::with_reads(vec![Err(io::Error::from(io::ErrorKind::PermissionDenied))]);
    let mut session = connected_session();
    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::Fatal);
    assert_eq!(
        session.state(),
        ConnectionState::Stopped(StopReason::Fatal),
        "権限errorを再接続で直そうとしない"
    );
}

/// 行長上限を超えた行は拒否され、bufferは上限を超えて伸びない。
#[test]
fn an_oversize_line_is_rejected_and_the_next_line_still_decodes() {
    let mut oversize = vec![b'x'; limits::MAX_LINE_BYTES * 2];
    oversize.push(b'\n');
    let good = ping_line(9);

    let good_bytes = good.into_bytes();
    let oversize_len = oversize.len();
    let good_len = good_bytes.len();
    let mut sim = Sim::with_reads(vec![Ok(oversize), Ok(good_bytes)]);
    let mut session = connected_session();

    let mut causes = Vec::new();
    let mut frames = 0_usize;
    // oversizeな行は1回のreadに収まらない。読み切るまで回す。
    // **回数は入力長から導出する。**定数で置くと`MAX_LINE_BYTES`を変えたときに
    // 必要な回数が増え、testが黙って落ちる。1回のreadで最低1byteは進むため、
    // 全入力byte数だけ回せば必ず読み切れる。
    for _ in 0..=(oversize_len + good_len) {
        let _ = session.pump_read(&mut sim, |outcome| match outcome {
            Outcome::Rejected(rejection) => causes.push(rejection.cause()),
            Outcome::Frame(_) => frames += 1,
        });
    }

    assert_eq!(causes, vec![Cause::Oversize], "oversizeは1回だけ返る");
    assert_eq!(frames, 1, "oversizeの後続の行は通常どおり復元される");
    assert_eq!(session.counters().rejected_in, 1);
}

/// 送信queueが満杯なら捨ててcounterを増やす。握りつぶさない。
#[test]
fn a_full_outbox_drops_the_send_and_counts_it() {
    let mut session = Session::new(
        config().with_outbox_capacity(1).expect("容量は1以上である"),
        90_312,
    );
    session.note_connected();

    assert!(session.send(hello(), 10).is_ok());
    assert_eq!(
        session.send(Message::Ping, 20),
        Err(SendError::Dropped),
        "容量を超えた送信は明示的に拒否される"
    );
    assert_eq!(session.pending_out(), 1, "容量を超えて保持しない");
    assert_eq!(session.counters().dropped_out, 1);
}

/// 1byteずつしか書けなくても、重複せず順序どおりに出る。
#[test]
fn partial_writes_resume_without_duplicating_bytes() {
    let mut session = connected_session();
    let expected = {
        let mut sim = Sim::default();
        let mut s = connected_session();
        let _ = s.send(hello(), 40).expect("queueへ入る");
        while !matches!(s.pump_write(&mut sim), Pump::Idle) {}
        sim.written
    };

    let _ = session.send(hello(), 40).expect("queueへ入る");
    let mut sim = Sim {
        write_chunk: Some(1),
        ..Sim::default()
    };
    for _ in 0..expected.len() {
        assert_eq!(session.pump_write(&mut sim), Pump::Progress(1));
    }

    assert_eq!(sim.written, expected, "byte列が重複も欠落もしない");
    assert_eq!(session.pending_out(), 0);
    assert_eq!(
        session.counters().bytes_out,
        u64::try_from(expected.len()).expect("行長は有界である")
    );
    assert_eq!(session.pump_write(&mut sim), Pump::Idle, "送るものが無い");
}

/// 書き込み中の`WouldBlock`は進捗を失わない。
#[test]
fn a_wouldblock_during_write_keeps_the_partial_progress() {
    let mut session = connected_session();
    let _ = session.send(hello(), 40).expect("queueへ入る");

    let mut sim = Sim {
        write_chunk: Some(4),
        write_errors: VecDeque::from(vec![io::Error::from(io::ErrorKind::WouldBlock)]),
        ..Sim::default()
    };

    assert_eq!(session.pump_write(&mut sim), Pump::Idle, "最初はWouldBlock");
    assert_eq!(session.counters().bytes_out, 0);

    assert_eq!(session.pump_write(&mut sim), Pump::Progress(4));
    assert_eq!(session.counters().bytes_out, 4);
    assert_eq!(session.state(), ConnectionState::Connected);
}

/// 再接続は上限に達したら止まる。上限後も試行し続けない。
#[test]
fn reconnect_stops_at_the_attempt_limit() {
    let policy = ReconnectPolicy::new(
        3,
        core::time::Duration::from_millis(10),
        core::time::Duration::from_millis(40),
    )
    .expect("方針は妥当である");
    let mut session = Session::new(config().with_reconnect(policy), 90_312);
    session.note_connected();

    let mut sim = Sim::with_reads(vec![Ok(Vec::new())]);
    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::Disconnected);

    let mut backoffs = Vec::new();
    while let Some(delay) = session.begin_reconnect() {
        backoffs.push(delay);
    }

    assert_eq!(backoffs.len(), 3, "上限回数までしか試行しない");
    assert!(
        backoffs.windows(2).all(|w| w[0] <= w[1]),
        "間隔が縮まない: {backoffs:?}"
    );
    assert_eq!(
        *backoffs.last().expect("3件ある"),
        core::time::Duration::from_millis(40),
        "max_backoffで頭打ちになる"
    );
    assert_eq!(
        session.state(),
        ConnectionState::Stopped(StopReason::ReconnectExhausted)
    );
    assert_eq!(session.counters().reconnect_attempts, 3);
}

/// 切断で保留中の送信を捨て、捨てた分をcounterへ計上する。
#[test]
fn a_disconnect_discards_pending_sends_and_counts_them() {
    let mut session = connected_session();
    let _ = session.send(hello(), 40).expect("queueへ入る");
    assert_eq!(session.pending_out(), 1);

    let mut sim = Sim::with_reads(vec![Ok(Vec::new())]);
    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::Disconnected);

    assert_eq!(session.pending_out(), 0, "古い送信を再接続後へ持ち越さない");
    assert_eq!(
        session.counters().discarded_on_disconnect,
        1,
        "切断による破棄として数える"
    );
    assert_eq!(
        session.counters().dropped_out,
        0,
        "溢れは起きていない。原因の違うdropを混ぜない"
    );
}

/// 停止したsessionは送信を受け付けない。
#[test]
fn a_stopped_session_refuses_further_sends() {
    let mut sim = Sim::with_reads(vec![Err(io::Error::from(io::ErrorKind::PermissionDenied))]);
    let mut session = connected_session();
    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::Fatal);

    assert_eq!(
        session.send(Message::Ping, 50),
        Err(SendError::Stopped(StopReason::Fatal))
    );
}

/// `Outbox`単体でも、上限を超えたpushが存在しないことを固定する。
#[test]
fn the_outbox_never_grows_past_its_capacity() {
    let mut outbox = Outbox::new(core::num::NonZeroUsize::new(3).expect("0ではない"));
    for _ in 0..10 {
        let _ = outbox.enqueue(b"line\n".to_vec());
    }
    assert_eq!(outbox.len(), 3);
    assert_eq!(outbox.dropped(), 7);
    assert_eq!(outbox.enqueue(b"x".to_vec()), Enqueued::Dropped);
}

/// `id`の上限に達したらsessionは停止するが、終端報告は1件だけ送れる。
///
/// 仕様§3の`PROTO-TBD-003`である。wrapさせず、上限値は終端報告のために
/// 予約されており、通常の送出はそこで止まる。
#[test]
fn id_exhaustion_stops_the_session_and_leaves_exactly_one_terminal_report() {
    let mut session = Session::with_first_id(config(), 90_312, u32::MAX - 1);
    session.note_connected();

    assert_eq!(
        session.send(Message::Ping, 10),
        Ok(u32::MAX - 1),
        "上限の1つ手前までは通常どおり送れる"
    );

    assert_eq!(
        session.send(Message::Ping, 20),
        Err(SendError::IdSpaceExhausted),
        "上限値は予約済みであり、通常の送出へは払い出さない"
    );
    assert_eq!(
        session.state(),
        ConnectionState::Stopped(StopReason::IdSpaceExhausted)
    );

    // 停止後も終端報告は送れる。予約はこのためにある。
    assert_eq!(
        session.send_terminal(Message::Ping, 30),
        Ok(u32::MAX),
        "終端報告は予約した上限値を使う"
    );
    assert_eq!(
        session.send_terminal(Message::Ping, 40),
        Err(SendError::IdSpaceExhausted),
        "終端報告はちょうど1件である"
    );
}

/// 停止しても、既にqueueへ入ったbyte列は書き切れる。
///
/// 止まるのは新しい`(sid, id)`を要する送出だけである。**予約した`id`で作った
/// 終端報告を書き切れないなら、予約する意味が無い。**
#[test]
fn a_session_stopped_by_id_exhaustion_still_flushes_what_it_already_queued() {
    let mut session = Session::with_first_id(config(), 90_312, u32::MAX);
    session.note_connected();

    assert_eq!(
        session.send(Message::Ping, 10),
        Err(SendError::IdSpaceExhausted)
    );
    let terminal = session
        .send_terminal(Message::Ping, 20)
        .expect("終端報告は送れる");
    assert_eq!(terminal, u32::MAX);
    assert_eq!(session.pending_out(), 1);

    // 停止していてもlinkは繋がったままであり、書き切れる。
    assert!(session.link_connected());
    let mut sim = Sim {
        write_chunk: Some(3),
        ..Sim::default()
    };
    while matches!(session.pump_write(&mut sim), Pump::Progress(_)) {}

    assert_eq!(session.pending_out(), 0, "終端報告を送り切る");
    let written = String::from_utf8(sim.written).expect("UTF-8である");
    assert!(
        written.contains(&format!("\"id\":{}", u32::MAX)),
        "予約した上限値のidで出ている: {written}"
    );
}

/// 再接続で直らないerrorはlinkも落とす。停止後に書き続けない。
#[test]
fn a_fatal_error_drops_the_link_as_well_as_stopping_the_session() {
    let mut sim = Sim::with_reads(vec![Err(io::Error::from(io::ErrorKind::PermissionDenied))]);
    let mut session = connected_session();

    assert_eq!(session.pump_read(&mut sim, |_| {}), Pump::Fatal);
    assert!(!session.link_connected(), "linkは使えない");
    assert_eq!(session.pump_write(&mut sim), Pump::Idle);
}

/// 終端報告は、queueが満杯でも**予約を失わない**。
///
/// 先に予約を消費してからqueueへ入れる実装では、満杯だったときに予約だけが
/// 消え、終端報告を永久に送れなくなる。予約は1件しかなく取り戻せない。
#[test]
fn a_full_outbox_does_not_consume_the_terminal_reservation() {
    // 容量1のqueueを通常の送信でふさいでから、終端報告を試す。
    let mut session = Session::with_first_id(
        config().with_outbox_capacity(1).expect("容量は1以上である"),
        90_312,
        u32::MAX - 1,
    );
    session.note_connected();
    assert_eq!(session.send(Message::Ping, 10), Ok(u32::MAX - 1));
    assert_eq!(
        session.send(Message::Ping, 20),
        Err(SendError::IdSpaceExhausted)
    );

    // queueは満杯である。ここで終端報告は入らないが、予約は残る。
    assert_eq!(
        session.send_terminal(Message::Ping, 30),
        Err(SendError::Dropped),
        "満杯なので入らない"
    );

    // queueが掃ければ、同じ予約で送れる。
    let mut sim = Sim::default();
    while matches!(session.pump_write(&mut sim), Pump::Progress(_)) {}
    assert_eq!(
        session.send_terminal(Message::Ping, 40),
        Ok(u32::MAX),
        "予約は失われていない"
    );
}

/// 不正な設定はpanicではなく分類されたerrorになる。
#[test]
fn invalid_configuration_is_reported_as_a_typed_error() {
    assert_eq!(
        SerialConfig::new("", 115_200).unwrap_err(),
        ConfigError::EmptyPort
    );
    assert_eq!(
        SerialConfig::new("/dev/simulated", 0).unwrap_err(),
        ConfigError::ZeroBaud
    );
    assert_eq!(
        config().with_outbox_capacity(0).unwrap_err(),
        ConfigError::ZeroOutboxCapacity
    );
    assert_eq!(
        ReconnectPolicy::new(3, Duration::ZERO, Duration::from_secs(1)).unwrap_err(),
        ConfigError::ZeroInitialBackoff,
        "0のbackoffはrate limitを消すため受け付けない"
    );
    assert_eq!(
        ReconnectPolicy::new(3, Duration::from_secs(2), Duration::from_secs(1)).unwrap_err(),
        ConfigError::BackoffBoundsInverted
    );
}

/// 送出の準備が失敗しても`id`を消費しない。枯渇を早めない。
#[test]
fn a_dropped_send_does_not_consume_an_id() {
    let mut session = Session::new(
        config().with_outbox_capacity(1).expect("容量は1以上である"),
        90_312,
    );
    session.note_connected();

    assert_eq!(session.send(Message::Ping, 10), Ok(1));
    for _ in 0..5 {
        assert_eq!(session.send(Message::Ping, 20), Err(SendError::Dropped));
    }

    // queueを掃けば、次の`id`は2である。dropで進んでいない。
    let mut sim = Sim::default();
    while matches!(session.pump_write(&mut sim), Pump::Progress(_)) {}
    assert_eq!(
        session.send(Message::Ping, 30),
        Ok(2),
        "dropした5件は`id`を消費していない"
    );
}

/// 先頭messageを書き切ったら`flush`する。bufferに残したまま送れたことにしない。
#[test]
fn a_completed_message_is_flushed() {
    let mut session = connected_session();
    let _ = session.send(hello(), 40).expect("queueへ入る");
    let mut sim = Sim::default();

    while matches!(session.pump_write(&mut sim), Pump::Progress(_)) {}

    assert_eq!(session.pending_out(), 0, "書き切っている");
    assert_eq!(sim.flushes, 1, "書き切った時点で1回だけflushする");
}

/// 書き切っていない間はflushしない。partial writeごとにflushを呼ばない。
#[test]
fn a_partial_write_does_not_flush() {
    let mut session = connected_session();
    let _ = session.send(hello(), 40).expect("queueへ入る");
    let mut sim = Sim {
        write_chunk: Some(1),
        ..Sim::default()
    };

    assert_eq!(session.pump_write(&mut sim), Pump::Progress(1));

    assert!(session.pending_out() > 0, "まだ残っている");
    assert_eq!(sim.flushes, 0, "書き切るまでflushしない");
}

/// `flush`の失敗を握りつぶさない。`write`のerrorと同じ分類へ通す。
#[test]
fn a_flush_failure_is_classified_not_swallowed() {
    let mut session = connected_session();
    let _ = session.send(hello(), 40).expect("queueへ入る");
    let mut sim = Sim {
        flush_errors: VecDeque::from(vec![io::Error::from(io::ErrorKind::BrokenPipe)]),
        ..Sim::default()
    };

    assert_eq!(
        session.pump_write(&mut sim),
        Pump::Disconnected,
        "flushの失敗も切断として分類される"
    );
    assert_eq!(session.counters().disconnects, 1);
}

/// 送出前の自己検証で落ちた送信をcounterに残す。黙って捨てない。
#[test]
fn an_unencodable_message_is_counted() {
    let mut session = connected_session();
    let too_long = Message::Hello(Hello {
        host: "h".repeat(limits::MAX_HOST_BYTES + 1),
        version: "0.1.0".to_owned(),
        reason: HelloReason::Startup,
    });

    let err = session
        .send(too_long, 40)
        .expect_err("上限を超えたhostはencodeできない");

    assert!(matches!(err, SendError::Encode(_)), "分類はEncodeである");
    assert_eq!(session.counters().encode_failed, 1);
    assert_eq!(session.pending_out(), 0, "queueへは入っていない");
    assert_eq!(session.counters().dropped_out, 0, "輻輳とは別に数える");
}

/// 再試行できるerrorで落ちた`flush`を取り残さない。outboxが空になった後も片付ける。
#[test]
fn a_retryable_flush_failure_is_retried_until_it_succeeds() {
    let mut session = connected_session();
    let _ = session.send(hello(), 40).expect("queueへ入る");
    let mut sim = Sim {
        // 書き切った直後のflushだけWouldBlockで落ちる。
        flush_errors: VecDeque::from(vec![io::Error::from(io::ErrorKind::WouldBlock)]),
        ..Sim::default()
    };

    // 書き切りはするが、flushは落ちるため再試行扱いになる。
    while matches!(session.pump_write(&mut sim), Pump::Progress(_)) {}
    assert_eq!(session.pending_out(), 0, "byteはtransportへ渡っている");
    assert_eq!(sim.flushes, 1, "1回目のflushは失敗している");
    assert_eq!(session.counters().retries, 1, "再試行として分類される");

    // **outboxは空である。**ここで再試行できなければbufferの中身は取り残される。
    assert_eq!(session.pump_write(&mut sim), Pump::Idle);
    assert_eq!(sim.flushes, 2, "outboxが空でもflushを片付ける");
    assert_eq!(session.state(), ConnectionState::Connected);
}

/// 切断したlinkへflushを持ち越さない。
#[test]
fn a_disconnect_clears_the_pending_flush() {
    let mut session = connected_session();
    let _ = session.send(hello(), 40).expect("queueへ入る");
    let mut sim = Sim {
        flush_errors: VecDeque::from(vec![io::Error::from(io::ErrorKind::WouldBlock)]),
        ..Sim::default()
    };
    while matches!(session.pump_write(&mut sim), Pump::Progress(_)) {}
    assert_eq!(sim.flushes, 1, "flushは保留になっている");

    // EOFで切断させる。`Ok(0)`は「dataが無い」ではなく切断である。
    sim.reads.push_back(Ok(Vec::new()));
    let _ = session.pump_read(&mut sim, |_| {});
    assert_eq!(session.state(), ConnectionState::Disconnected);

    let flushes_at_disconnect = sim.flushes;
    assert_eq!(session.pump_write(&mut sim), Pump::Idle, "切断中は送らない");
    assert_eq!(
        sim.flushes, flushes_at_disconnect,
        "切断したlinkへflushを持ち越さない"
    );
}

/// I/O errorを分類とcounterだけで済ませず、記録する。
///
/// **`log`のloggerはprocessで1つだけである。**installは`OnceLock`で1回に限り、
/// 記録の検証は`CAPTURE`の中身で行う。
mod logging {
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use deskcat_serial::Pump;

    use super::{Sim, VecDeque, connected_session, hello, io};

    // **captureはthread局所にする。**loggerはprocessで共有されるため、
    // 共有bufferにすると**このmoduleの外のtest**（`a_flush_failure_is_classified_not_swallowed`
    // なども`serial flush`を記録する）のrecordが混ざる。lockで直列化しても、
    // lockを取らないtestからの混入は防げない。cargoはtestごとに別threadで走らせるため、
    // thread局所にすれば各testは自分のrecordだけを見る。
    thread_local! {
        static CAPTURE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
    static INSTALLED: OnceLock<()> = OnceLock::new();

    struct Capture;

    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            let line = format!("{} {}", record.level(), record.args());
            // 記録元のthread（=そのtest）へ入る。
            CAPTURE.with_borrow_mut(|sink| sink.push(line));
        }

        fn flush(&self) {}
    }

    static CAPTURE_LOGGER: Capture = Capture;

    fn install() {
        // **`set_boxed_logger`は`log`の`std` featureを要する。**libraryが要らない
        // featureを増やさないため、`&'static`を取る`set_logger`を使う。
        INSTALLED.get_or_init(|| {
            log::set_logger(&CAPTURE_LOGGER).expect("loggerは1度だけ入れる");
            log::set_max_level(log::LevelFilter::Trace);
        });
    }

    fn take() -> Vec<String> {
        CAPTURE.with_borrow_mut(std::mem::take)
    }

    /// 切断は`warn`で、操作の種別とerrorと分類が残る。
    #[test]
    fn a_disconnect_is_logged_with_the_operation_and_disposition() {
        install();
        let _ = take();

        let mut session = connected_session();
        let _ = session.send(hello(), 40).expect("queueへ入る");
        let mut sim = Sim {
            write_errors: VecDeque::from(vec![io::Error::from(io::ErrorKind::BrokenPipe)]),
            ..Sim::default()
        };

        assert_eq!(session.pump_write(&mut sim), Pump::Disconnected);

        let lines = take();
        let hit = lines
            .iter()
            .find(|l| l.contains("serial write"))
            .expect("write経路のerrorが記録される");
        assert!(hit.starts_with("WARN"), "切断はWARN水準である: {hit}");
        assert!(hit.contains("Disconnected"), "分類が残る: {hit}");
    }

    /// 再試行できるerrorは`debug`。正常運転で頻発するため水準を上げない。
    #[test]
    fn a_retryable_flush_error_is_logged_at_debug() {
        install();
        let _ = take();

        let mut session = connected_session();
        let _ = session.send(hello(), 40).expect("queueへ入る");
        let mut sim = Sim {
            flush_errors: VecDeque::from(vec![io::Error::from(io::ErrorKind::WouldBlock)]),
            ..Sim::default()
        };
        while matches!(session.pump_write(&mut sim), Pump::Progress(_)) {}

        let lines = take();
        let hit = lines
            .iter()
            .find(|l| l.contains("serial flush"))
            .expect("flush経路のerrorが記録される");
        assert!(hit.starts_with("DEBUG"), "再試行はDEBUG水準である: {hit}");
        assert!(hit.contains("Retry"), "分類が残る: {hit}");
    }
}

/// **openは成功するがlinkが死んでいる場合でも、再接続の上限は効く。**
///
/// device nodeは列挙されるが応答しない、という状態は実際に起きる（USB adapterの
/// 半挿し、相手の電源断）。予算のresetを`note_connected`（＝openが成功しただけ）で
/// 行うと、「open成功 → 予算が戻る → すぐ切断 → 再試行」が延々と回り、
/// 受け入れ条件「Reconnectに上限とrate limitがある」が実質的に無効になる。
///
/// 既存の`reconnect_stops_at_the_attempt_limit`はこの経路を踏まない。
/// あちらはfakeに再openが無く、`note_connected`を1度しか呼ばないためである。
#[test]
fn a_link_that_opens_but_never_carries_bytes_still_exhausts_the_reconnect_budget() {
    let policy = ReconnectPolicy::new(3, Duration::from_millis(10), Duration::from_millis(40))
        .expect("方針は妥当である");
    let config = SerialConfig::new("/dev/simulated", 115_200)
        .expect("設定は妥当である")
        .with_reconnect(policy);
    let mut session = Session::new(config, 90_312);

    let mut backoffs = Vec::new();
    for round in 0..16 {
        assert!(round < 15, "上限に達せず再接続が止まらない");

        // 「openは成功した」。だがlinkは死んでおり、すぐEOFになる。
        session.note_connected();
        let mut dead = Sim::with_reads(vec![Ok(Vec::new())]);
        assert_eq!(session.pump_read(&mut dead, |_| {}), Pump::Disconnected);

        match session.begin_reconnect() {
            Some(backoff) => backoffs.push(backoff),
            None => break,
        }
    }

    assert_eq!(backoffs.len(), 3, "上限は3回である");
    assert_eq!(
        backoffs,
        vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(40),
        ],
        "予算が戻っているとbackoffが初期値のまま伸びない"
    );
    assert_eq!(
        session.state(),
        ConnectionState::Stopped(StopReason::ReconnectExhausted)
    );
}

/// 予算が戻るのは、実際にbyteが流れたときである。
///
/// 上のtestの裏側。**正当な復帰まで塞いでいないこと**を固定する。
#[test]
fn the_reconnect_budget_is_restored_by_bytes_not_by_opening() {
    let policy = ReconnectPolicy::new(3, Duration::from_millis(10), Duration::from_millis(40))
        .expect("方針は妥当である");
    let config = SerialConfig::new("/dev/simulated", 115_200)
        .expect("設定は妥当である")
        .with_reconnect(policy);
    let mut session = Session::new(config, 90_312);

    session.note_connected();
    let mut dead = Sim::with_reads(vec![Ok(Vec::new())]);
    assert_eq!(session.pump_read(&mut dead, |_| {}), Pump::Disconnected);
    assert!(session.begin_reconnect().is_some());
    assert_eq!(session.reconnect_attempts(), 1);

    // 再open。openしただけでは戻らない。
    session.note_connected();
    assert_eq!(
        session.reconnect_attempts(),
        1,
        "openが成功しただけでは予算は戻らない"
    );

    // byteが流れたら戻る。
    let incoming = ping_line(1);
    let mut healthy = Sim::with_reads(vec![Ok(incoming.as_bytes().to_vec())]);
    assert!(matches!(
        session.pump_read(&mut healthy, |_| {}),
        Pump::Progress(_)
    ));
    assert_eq!(
        session.reconnect_attempts(),
        0,
        "実際にbyteが読めたら予算は戻る"
    );
    assert_eq!(
        session.counters().reconnect_attempts,
        1,
        "累計のcounterはresetしない"
    );
}
