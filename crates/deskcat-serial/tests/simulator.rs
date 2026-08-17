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

    let mut sim = Sim::with_reads(vec![Ok(oversize), Ok(good.into_bytes())]);
    let mut session = connected_session();

    let mut causes = Vec::new();
    let mut frames = 0_usize;
    // oversizeな行は1回のreadに収まらない。読み切るまで回す。
    for _ in 0..16 {
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
