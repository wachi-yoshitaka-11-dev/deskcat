//! 実serial portへ繋いでlinkを確かめる実行体。
//!
//! `SerialDevice`を開き、`Session`のpumpを回し、切断したら方針の範囲で再接続する。
//! **このcrateはloopを持たない**（`Session`はtransportを所有せず、pumpの引数で受け取る）
//! ため、その呼び出し側をここに1つ置く。[Issue #11]の後半（Pi実機）は、これを走らせる。
//!
//! # これで確かめられること／確かめられないこと
//!
//! 行の層では、open、byteのread／write、行の復元、切断の観測、再接続の上限、partial I/Oを確かめられる。
//! protocolの層は、次のように[`deskcat_serial::PeerSession`]（[Issue #12]、
//! `crates/deskcat-serial/src/peer.rs`と`coordinator.rs`）へつないである。
//!
//! - 接続のたびに`hello`を1件送り、その`id`を[`deskcat_serial::PeerSession::note_hello_sent`]で記録する。
//!   `reason`は初回が`Startup`、再接続が`PortReopen`である（仕様§5.1）。
//! - 受信したframeは、種類を問わず[`deskcat_serial::handle_frame`]へ渡す。`boot`にはACKを返し、
//!   新しいsessionを確立した場合は`get_status`を1件送る。`ack`は送ったrequestと相関させる。`status`は
//!   `get_status`への応答かどうかを分ける（仕様§5.6）。
//! - `--ping-count`を付けたときだけ、ESP32 sessionの確立のたびに`ping`を1件ずつ最大その件数送る
//!   （前の`ping`がACKの相関か取り下げで決着してから次を送る。ESP32 sessionの確立でも未決は手放す。周期は持たない。`PROTO-TBD-010`）。
//! - pumpの1周ごとに[`deskcat_serial::retry_due_requests`]を呼ぶ。ACK timeoutした`ping`と`get_status`を
//!   同じ`id`で送り直し（仕様§9）、確立の直後にqueueへ入れられなかった`get_status`を送る。
//!
//! **ESP32が`boot`→ACKより先の往復に応えるのは、`firmware/esp32`を[Issue #487]のPR B2
//! （[PR #529]）以降のsourceから、Pi linkを持つbuild（`bench-servo-test-17`以外）で書き込んだ場合である。**PR B2で、
//! ESP32は`hello`／`ping`／`get_status`にACKを返し、`get_status`にはACKの直後に`status`を返す
//! （`firmware/esp32/src/pi_link.rs`のmodule doc）。そのため、実機で次の往復が成り立つはずである。
//!
//! 1. この実行体の`hello`にESP32がACKを返す。初回の接続で`boot`より先に届いたACKは、ESP32の
//!    `sid`をまだ承認していないため`hello`の結果として受理せず、`unapproved_hello_acks`へ数える
//!    （仕様§6）。届く順序とESP32の状態によって数え方は変わる。その正本は`PeerSession`の
//!    `pending_hello`のdocと、`tests/simulator.rs`の`hello`への`ack`を扱うtestである。
//! 2. ESP32が`boot`を送る（起動後の再送、または`hello`による再開）。`boot`が届く条件は仕様§4.1の
//!    表と「`hello`による再開」が持つ。この実行体はACKを返し、`get_status`を送る。
//! 3. ESP32が`get_status`へACKと`status`を返す。`status`のcounterは0のままである
//!    （仕様§2の既知の逸脱(ii)）。
//!
//! **3が成り立つのは、ESP32がこの実行体の`hello`を受理し、その後に再起動していない場合だけである。**
//! それ以外では、ESP32は`get_status`を`stale_session`で拒否し、`status`を返さない
//! （`firmware/esp32/src/protocol.rs`の`PiSession::handle_get_status`）。`hello`がESP32へ届かなかった
//! 場合が当たる（この実行体は`hello`を再送しない。`PROTO-TBD-011`）。
//! 仕様は、Piが`stale_session`を受けたら`hello`から再開すると定める（§10.2）。**この実行体は
//! それを実装していない**（[`deskcat_serial::handle_frame`]のdocは、再開を呼び出し側の責務とする）。
//! 再接続で送るのも`PortReopen`の`hello`だけである。そのため、この場合は実行体を起動し直す。
//!
//! **この往復は、実機ではまだ確かめていない。**`ESP32`側の振る舞いはsourceを読んで導いたもので
//! あり、ESP32側の振る舞いを確かめるhost testは無い（`pi_link.rs`のmodule doc「確かめていないこと」）。
//! host（VM）では擬似端末で確かめたが、相手はfixtureの行で応える偽のESP32である
//! （`crates/deskcat-serial/README.md`の「host（VM）で確認済みの挙動」）。実機の確認は
//! [Issue #446]と[Issue #12]が追跡する。
//! **`ESP32`側がprotocolを話すとは限らない**という留保は、実機で確かめるまで変わらない。
//! 記録するときは「行が通った」と「protocolが成立した」を書き分ける。
//!
//! # 使い方
//!
//! ```text
//! cargo run --example serial_link -- --port <path> --baud <rate> \
//!     --duplicate-capacity <n> --duplicate-retention-ms <ms> [--seconds <n>] [--ping-count <n>] [--verbose]
//! ```
//!
//! `--port`と`--baud`は**どちらも必須である。**既定値を持たせない。device名は未確認で
//! あり（確定はIssue #11の後半）、baudの正本は`PROTO-TBD-001`でいずれも`Candidate`である。
//! **確認していない値を既定として固定しない。**渡した値は記録にそのまま残る。
//!
//! `--duplicate-capacity`と`--duplicate-retention-ms`（`boot`のduplicate履歴の保持件数と
//! 保持期間）も同じ理由で必須である。正本は`PROTO-TBD-005`で未確定である。
//!
//! `--seconds`を省くと、再接続の上限に達してsessionが停止するまで走り続ける。
//!
//! `--ping-count <n>`は1以上で、省くと`ping`を送らない。終了時の`counters: pings_sent=… ping_acks=…
//! ping_rejected=… ping_retries=… ping_gave_up=…`は通算である。**`pings_sent`と`ping_acks`が等しく、
//! n以上であれば、送った`ping`のすべてに`ok`のACKを相関できた（n未満なら、まだ送り終えていない）。**等しくなければ、拒否・取り下げ・
//! 未決のまま終わった、またはESP32 sessionの切替で捨てた`ping`がある（`peer: session_switches`は最初の確立でも1になる。2以上なら切替があった）。
//! これは「`ping`へのACKを相関した」ことの根拠であり、`boot`→ACK→`get_status`→`status`の往復の成立とは
//! 別に記録する。
//!
//! `--verbose`を付けない限り`Info`までを出す。`Debug`まで上げるとread timeoutごとに
//! 1行出るため（既定50 msなので毎秒20行）、長時間の観察では本当のeventが埋まる。
//! 切り分けが要るときだけ上げる。
//!
//! # 記録に残さないもの
//!
//! **device名を出力へ書かない。**`Version Record Template`が禁じている項目であり、
//! この出力をそのまま記録へ貼れるようにしておく。開いた事実だけを出す。
//!
//! [Issue #11]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11
//! [Issue #12]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12
//! [Issue #446]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446
//! [Issue #487]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487
//! [PR #529]: https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/529

use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use deskcat_protocol::{AckStatus, Frame, Hello, HelloReason, Message, Outcome};
use deskcat_serial::{
    BootOutcome, ConnectionState, DuplicatePolicy, OutstandingKind, PeerSession, Pump, Received,
    RetryOutcome, SendError, SerialConfig, SerialDevice, Session, handle_frame, retry_due_requests,
};

/// 呼び出し側の引数。
struct Args {
    port: String,
    baud: u32,
    /// `--duplicate-capacity`と`--duplicate-retention-ms`から作る。
    duplicate_policy: DuplicatePolicy,
    seconds: Option<u64>,
    /// `--ping-count`。省くと`ping`を送らない。
    ping_count: Option<u32>,
    verbose: bool,
}

fn usage() -> &'static str {
    "usage: serial_link --port <path> --baud <rate> \
     --duplicate-capacity <n> --duplicate-retention-ms <ms> [--seconds <n>] [--ping-count <n>] [--verbose]\n\
     \n\
     --port、--baud、--duplicate-capacity、--duplicate-retention-ms は必須である。既定値を持たせない。\n\
     device名は未確認であり、baudの正本は PROTO-TBD-001、duplicate履歴の正本は PROTO-TBD-005 である。\n\
     --ping-count を省くと ping を送らない。"
}

/// 引数の解析結果。
///
/// `--help`は**成功である。**`Err`へ畳むとexit codeが1になり、
/// script側から「使い方を聞いた」と「引数を間違えた」を区別できない。
enum Parsed {
    /// 実行する。
    Run(Box<Args>),
    /// 使い方を出して正常終了する。
    Help,
}

fn parse_args() -> Result<Parsed, String> {
    let mut port = None;
    let mut baud = None;
    let mut duplicate_capacity = None;
    let mut duplicate_retention_ms = None;
    let mut seconds = None;
    let mut ping_count = None;
    let mut verbose = false;
    let mut argv = std::env::args().skip(1);

    while let Some(flag) = argv.next() {
        let mut value = || argv.next().ok_or_else(|| format!("{flag}に値が無い"));
        match flag.as_str() {
            "--port" => port = Some(value()?),
            "--baud" => {
                baud = Some(
                    value()?
                        .parse::<u32>()
                        .map_err(|e| format!("--baudが数値でない: {e}"))?,
                );
            }
            "--duplicate-capacity" => {
                duplicate_capacity = Some(
                    value()?
                        .parse::<usize>()
                        .map_err(|e| format!("--duplicate-capacityが数値でない: {e}"))?,
                );
            }
            "--duplicate-retention-ms" => {
                duplicate_retention_ms = Some(
                    value()?
                        .parse::<u64>()
                        .map_err(|e| format!("--duplicate-retention-msが数値でない: {e}"))?,
                );
            }
            "--seconds" => {
                seconds = Some(
                    value()?
                        .parse::<u64>()
                        .map_err(|e| format!("--secondsが数値でない: {e}"))?,
                );
            }
            "--ping-count" => {
                let n = value()?
                    .parse::<u32>()
                    .map_err(|e| format!("--ping-countが数値でない: {e}"))?;
                if n == 0 {
                    return Err("--ping-countは1以上である（省けばpingを送らない）".to_owned());
                }
                ping_count = Some(n);
            }
            "--verbose" => verbose = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            other => return Err(format!("不明な引数: {other}\n\n{}", usage())),
        }
    }

    Ok(Parsed::Run(Box::new(Args {
        port: port.ok_or_else(|| format!("--portが必要である\n\n{}", usage()))?,
        baud: baud.ok_or_else(|| format!("--baudが必要である\n\n{}", usage()))?,
        duplicate_policy: DuplicatePolicy::new(
            duplicate_capacity
                .ok_or_else(|| format!("--duplicate-capacityが必要である\n\n{}", usage()))?,
            Duration::from_millis(
                duplicate_retention_ms.ok_or_else(|| {
                    format!("--duplicate-retention-msが必要である\n\n{}", usage())
                })?,
            ),
        )
        .map_err(|e| format!("duplicate履歴の設定が不正である: {e}"))?,
        seconds,
        ping_count,
        verbose,
    })))
}

/// backoffを`--seconds`の残りで頭打ちにする。
///
/// 頭打ちにしないと、期限が切れているのにbackoffを寝過ごしてから終了する。
/// `--seconds`を省いた場合（`deadline`が`None`）はbackoffをそのまま使う。
fn capped_backoff(backoff: Duration, deadline: Option<Instant>) -> Duration {
    match deadline {
        Some(d) => backoff.min(d.saturating_duration_since(Instant::now())),
        None => backoff,
    }
}

/// 起動からの経過（milliseconds）。仕様§3の`ts_ms`はwall-clock timeではない。
///
/// `u128`から`u64`へは飽和させる。切り捨てると値が巻き戻り、**単調増加という
/// 仕様上の性質が壊れる。**飽和なら止まるだけで、逆行しない。
fn uptime_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// 接続ごとに1件送る`hello`（仕様§5.1）。ACKとの相関と、ESP32側での扱いはmodule docを読む。
///
/// `reason`は初回だけ[`HelloReason::Startup`]で、**再接続では
/// [`HelloReason::PortReopen`]である。**仕様§5.1が両者を別の値として定義しており
/// （`Startup`は「新しい`sid`を使う」、`PortReopen`は「現在の`sid`を維持する」）、
/// この実行体は`sid`をprocessごとに1つしか持たない。**再接続で`Startup`を送ると
/// 「新しいsidを使う」と宣言しながら同じsidを送り続けることになる。**
fn hello(reason: HelloReason) -> Message {
    Message::Hello(Hello {
        host: "serial_link".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        reason,
    })
}

/// `hello`をqueueへ入れ、ESP32からのACKをこの`hello`と相関させるために記録する（#12）。
fn send_hello(session: &mut Session, peer: &mut PeerSession, reason: HelloReason, now_ms: u64) {
    match session.send(hello(reason), now_ms) {
        Ok(id) => {
            peer.note_hello_sent(id);
            log::info!("hello を queue へ入れた（id={id}, reason={reason:?}）");
        }
        Err(error) => log::warn!("hello を送れない: {error}"),
    }
}

/// `--ping-count`が指す`ping`の送信を、1件ずつ直列に進める状態（#12）。
///
/// **周期も間隔も持たない**（heartbeatの送り元は`PROTO-TBD-010`が決める）。ESP32 sessionが
/// 確立するたびに`total`件を数え直し、直前の`ping`が決着（ACKの相関、または再送予算の使い切りによる
/// 取り下げ）してから次を送る。未決の`ping`は高々1件である。ACKの相関と、ACK timeoutでの同じ`id`の
/// 再送は[`handle_frame`]と[`retry_due_requests`]が持つ。ここは回数と直列化と数え上げだけである。
#[derive(Debug)]
struct PingProbe {
    total: u32,
    remaining: u32,
    in_flight: Option<u32>,
    /// ESP32 sessionを確立してから、送信に失敗して止めるまでの間だけ`true`。
    armed: bool,
    counters: PingCounters,
}

/// `ping`の数え上げ。library（`SessionCounters`、`PeerCounters`）は`ping`のACKも再送も数えない
/// （`SessionCounters::retries`は送信の再送ではなく、I/Oの進捗が無かった回数である）。
///
/// **通算である。**ESP32 sessionが替わると未決の`ping`は数えずに捨てるため
/// （[`PingProbe::on_established`]）、`sent`は`acked`＋`rejected`＋`gave_up`より大きくなりうる。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct PingCounters {
    /// queueへ入れた`ping`の件数（再送を含まない）。
    sent: u64,
    /// `status: ok`のACKを相関した件数。
    acked: u64,
    /// `ok`でないACK（`status: rejected`）を相関した件数。
    rejected: u64,
    /// 再送予算を使い切って取り下げた件数。
    gave_up: u64,
    /// 同じ`id`で再送した回数（再送の失敗は含まない。失敗はlibraryが`log::warn!`へ出す）。
    retried: u64,
}

impl PingProbe {
    fn new(total: u32) -> Self {
        Self {
            total,
            remaining: total,
            in_flight: None,
            armed: false,
            counters: PingCounters::default(),
        }
    }

    /// 新しいESP32 sessionを確立した。**旧sessionの未決の`ping`は`PeerSession`が破棄する**
    /// ため、ここでも手放して数え直す。
    fn on_established(&mut self) {
        self.armed = true;
        self.remaining = self.total;
        self.in_flight = None;
    }

    /// いま`ping`を送ってよいか。
    fn wants_send(&self) -> bool {
        self.armed && self.remaining > 0 && self.in_flight.is_none()
    }

    /// `ping`をqueueへ入れた。`(何件目, 全件)`を返す。
    fn on_sent(&mut self, id: u32) -> (u32, u32) {
        self.remaining -= 1;
        self.in_flight = Some(id);
        self.counters.sent += 1;
        (self.total - self.remaining, self.total)
    }

    /// 再試行しても直らない送信失敗である。次のESP32 sessionの確立まで送らない。
    fn disarm(&mut self) {
        self.armed = false;
    }

    /// `ping`のACKを相関した。未決の`ping`のACKだけを数える。
    fn on_ack(&mut self, reply_to: u32, status: AckStatus) {
        if self.in_flight != Some(reply_to) {
            return;
        }
        self.in_flight = None;
        match status {
            AckStatus::Ok => self.counters.acked += 1,
            AckStatus::Rejected => self.counters.rejected += 1,
            // `AckStatus`は`#[non_exhaustive]`である。増えたvariantを黙って捨てず、`ok`でないものとして数える。
            other => {
                log::warn!("未知のAckStatus: {other:?}。rejectedとして数える");
                self.counters.rejected += 1;
            }
        }
    }

    /// `ping`を再送予算の使い切りで取り下げた。
    fn on_gave_up(&mut self, id: u32) {
        if self.in_flight != Some(id) {
            return;
        }
        self.in_flight = None;
        self.counters.gave_up += 1;
    }

    /// [`handle_frame`]の結果から、この`ping`に関わるものを拾う。
    fn observe(&mut self, received: &Received) {
        match received {
            Received::Boot(handled)
                if matches!(handled.outcome, BootOutcome::Established { .. }) =>
            {
                self.on_established();
            }
            Received::Ack(correlated) if matches!(correlated.request, OutstandingKind::Ping) => {
                self.on_ack(correlated.ack.reply_to, correlated.ack.status);
            }
            _ => {}
        }
    }

    /// [`retry_due_requests`]の結果から、`ping`の再送と取り下げを拾う。
    fn observe_retries(&mut self, outcomes: &[RetryOutcome]) {
        for outcome in outcomes {
            match outcome {
                RetryOutcome::Resent(id, OutstandingKind::Ping) if self.in_flight == Some(*id) => {
                    self.counters.retried += 1;
                }
                RetryOutcome::GaveUp(id, OutstandingKind::Ping) => self.on_gave_up(*id),
                _ => {}
            }
        }
    }
}

/// 送ってよいなら`ping`を1件queueへ入れ、ACKを相関させるために記録する。
fn send_ping(session: &mut Session, peer: &mut PeerSession, probe: &mut PingProbe, now_ms: u64) {
    if !probe.wants_send() {
        return;
    }
    match session.send(Message::Ping, now_ms) {
        Ok(id) => {
            // `Message::Ping`は必ず追跡対象に分類される（`OutstandingKind::classify`）ため、`None`は来ない。
            let _ = peer.note_sent(id, Message::Ping, now_ms);
            let (n, total) = probe.on_sent(id);
            log::info!("ping を queue へ入れた（id={id}, {n}/{total}）");
        }
        // queueが満杯なだけである。書き出せば空くので、次のtickで再試行する。
        Err(SendError::Dropped) => log::warn!("ping を送れない。次のtickで再試行する"),
        Err(error) => {
            log::error!("ping を送れない。次のESP32 sessionの確立まで送らない: {error}");
            probe.disarm();
        }
    }
}

fn report(session: &Session, peer: &PeerSession, probe: Option<&PingProbe>) {
    let (counters, peer, ping, state) = (
        session.counters(),
        peer.counters(),
        probe.map(|p| p.counters),
        session.state(),
    );
    log::info!("state: {state:?}");
    log::info!(
        "counters: bytes_in={} bytes_out={} frames_in={} rejected_in={}",
        counters.bytes_in,
        counters.bytes_out,
        counters.frames_in,
        counters.rejected_in
    );
    log::info!(
        "counters: disconnects={} reconnect_attempts={} timeouts={} retries={}",
        counters.disconnects,
        counters.reconnect_attempts,
        counters.timeouts,
        counters.retries
    );
    log::info!(
        "counters: dropped_out={} discarded_on_disconnect={} encode_failed={}",
        counters.dropped_out,
        counters.discarded_on_disconnect,
        counters.encode_failed
    );
    log::info!(
        "peer: session_switches={} duplicate_replays={} stale_sessions={} duplicate_expired={}",
        peer.session_switches,
        peer.duplicate_replays,
        peer.stale_sessions,
        peer.duplicate_expired
    );
    log::info!(
        "peer: invalid_payloads={} rate_limited={} unmatched_acks={} unapproved_hello_acks={} unknown_types={}",
        peer.invalid_payloads,
        peer.rate_limited,
        peer.unmatched_acks,
        peer.unapproved_hello_acks,
        peer.unknown_types
    );
    if let Some(ping) = ping {
        log::info!(
            "counters: pings_sent={} ping_acks={} ping_rejected={} ping_retries={} ping_gave_up={}",
            ping.sent,
            ping.acked,
            ping.rejected,
            ping.retried,
            ping.gave_up
        );
    }
}

/// [`handle_frame`]の判断を1行のlogにする。**`status`とeventの中身は出さない。**`boot`の`outcome`（確立した
/// `boot`の`firmware`／`board`／`reset_reason`を含む）と、ACKの`reply_to`／`status`／`code`と、
/// eventの`type`と`id`は出す。
///
/// `boot`への応答（ACKと、確立時の`get_status`）は[`handle_frame`]が送る。ここは出力だけである。
fn log_received(received: &Received) {
    match received {
        Received::Boot(handled) => log::info!("bootを処理した: outcome={:?}", handled.outcome),
        Received::Ack(correlated) => log::info!(
            "ackを相関した: request={:?} reply_to={} status={:?} code={:?}",
            correlated.request,
            correlated.ack.reply_to,
            correlated.ack.status,
            correlated.ack.code
        ),
        Received::Status(accepted) => {
            log::info!("statusを受理した: solicited={}", accepted.solicited);
        }
        Received::Event(frame) => log::info!(
            "eventを受けた: type={} id={}",
            frame.message.type_str(),
            frame.envelope.id
        ),
        // 拒否と無視は`handle_frame`自身が`log::warn!`へ残している。ここでは重ねて出さない。
        Received::Rejected { .. } | Received::UndefinedType(_) => {}
        // `Received`は`#[non_exhaustive]`である。増えたvariantを黙って捨てない。
        other => log::warn!("未知のReceived: {other:?}"),
    }
}

/// [`retry_due_requests`]の再送をlogにする。
///
/// 再送の失敗と取り下げは[`retry_due_requests`]自身が`log::warn!`へ残している。ここでは重ねて出さない。
fn log_retries(outcomes: &[RetryOutcome]) {
    for outcome in outcomes {
        match outcome {
            RetryOutcome::Resent(id, kind) => log::info!("再送した: id={id} kind={kind:?}"),
            RetryOutcome::ResendFailed(..) | RetryOutcome::GaveUp(..) => {}
            // `RetryOutcome`は`#[non_exhaustive]`である。増えたvariantを黙って捨てない。
            other => log::warn!("未知のRetryOutcome: {other:?}"),
        }
    }
}

/// `--seconds`から期限を作る。表現できなければlogを出して`Err`を返す。
///
/// **`started + Duration`にしない。**`--seconds`は任意の`u64`を受け取るため、
/// 表現できないdeadlineでpanicする（実測: `u64::MAX`で
/// `overflow when adding duration to instant`）。引数の誤りをpanicで返さない。
fn deadline_after(started: Instant, seconds: Option<u64>) -> Result<Option<Instant>, ()> {
    let Some(s) = seconds else {
        return Ok(None);
    };
    started
        .checked_add(Duration::from_secs(s))
        .map(Some)
        .ok_or_else(|| {
            log::error!("--seconds が大きすぎる: {s}");
        })
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(Parsed::Run(args)) => *args,
        Ok(Parsed::Help) => {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    // **既定はInfoである。**`Debug`にすると、dataが来ていないだけの状態でも
    // read timeoutごとに1行出る（既定50 msなので毎秒20行）。長時間の実機観察では
    // それが本当のeventを埋めてしまう。切り分けが要るときだけ`--verbose`で上げる。
    let level = if args.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    if let Err(error) = logger::install(level) {
        // logが使えないのでstderrへ直接出す。ここだけは例外である。
        eprintln!("loggerを入れられない: {error}");
        return ExitCode::FAILURE;
    }

    // **device名を出力しない。**開いた事実と設定値だけを出す。
    log::info!("baud={} で serial portを開く", args.baud);

    let config = match SerialConfig::new(&args.port, args.baud) {
        Ok(config) => config,
        Err(error) => {
            // loggerは導入済みである。ここだけeprintlnにすると出力先と整形が揃わない。
            log::error!("設定が不正である: {error}");
            return ExitCode::FAILURE;
        }
    };

    // sidは起動ごとに新しい値を選ぶ（仕様§3）。**自動で選び直さない。**
    // ここではprocessのidを種にする。乱数を持ち込まない。
    let sid = std::process::id();
    let mut session = Session::new(config.clone(), sid);
    // ESP32側のsession state。**再接続をまたいで1つだけ持つ。**Piの再接続
    // （`PortReopen`）はESP32側のsessionを暗黙に変えない。ESP32が再起動すれば
    // 新しい`sid`の`boot`として届き、`PeerSession`自身がsession遷移として扱う（§3.1）。
    let mut peer = PeerSession::new(args.duplicate_policy.clone());
    let started = Instant::now();
    let Ok(deadline) = deadline_after(started, args.seconds) else {
        return ExitCode::FAILURE;
    };

    log::info!("sid={sid} で開始する");

    // 初回接続かどうか。`hello`の`reason`を分けるために持つ。
    let mut first_connect = true;
    // `--ping-count`を省いたら持たない。
    let mut probe = args.ping_count.map(PingProbe::new);

    loop {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            log::info!("--seconds に達した");
            break;
        }

        // **openのerrorをIoDisposition::classifyへ渡さない。**再列挙中の一時的な失敗
        // （ENOENT／EACCES／EBUSY）をFatalにすると、復帰しようとしている場面で止まる。
        let mut device = match SerialDevice::open(&config) {
            Ok(device) => device,
            Err(error) => {
                log::warn!("openに失敗した: {error}");
                let Some(backoff) = session.begin_reconnect() else {
                    log::error!("再接続の上限に達した");
                    break;
                };
                let wait = capped_backoff(backoff, deadline);
                log::info!("{wait:?} 待って再試行する");
                sleep(wait);
                continue;
            }
        };

        log::info!("portを開いた");
        session.note_connected();

        // 接続ごとに`hello`を1件送る（module doc）。
        let reason = if first_connect {
            HelloReason::Startup
        } else {
            HelloReason::PortReopen
        };
        first_connect = false;
        send_hello(&mut session, &mut peer, reason, uptime_ms(started));

        let disconnected = pump_until_break(
            &mut session,
            &mut peer,
            &mut probe,
            &mut device,
            started,
            deadline,
        );

        if !disconnected {
            break; // deadline到達、または停止済み。summaryはloopの外で1度だけ出す
        }

        // 切断ごとの区切りとして出す。**loopを抜けた後にもう一度出さない**
        // （同じ数字が2回並ぶと、どちらが最終値か読めない）。
        report(&session, &peer, probe.as_ref());

        let Some(backoff) = session.begin_reconnect() else {
            log::error!("再接続の上限に達した。停止する");
            break;
        };
        let wait = capped_backoff(backoff, deadline);
        log::info!("切断した。{wait:?} 待って再接続する");
        sleep(wait);
    }

    report(&session, &peer, probe.as_ref());
    log::info!("経過 {:?}", started.elapsed());

    // **握りつぶしていないことをここで示す。**0件でも出す。
    let failures = logger::write_failures();
    if failures > 0 {
        eprintln!("log出力の失敗: {failures} 件");
    } else {
        log::info!("log出力の失敗: 0 件");
    }

    if matches!(session.state(), ConnectionState::Stopped(_)) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// linkが切れるか、期限に達するまでpumpを回す。切断で抜けたときだけ`true`。
fn pump_until_break(
    session: &mut Session,
    peer: &mut PeerSession,
    probe: &mut Option<PingProbe>,
    device: &mut SerialDevice,
    started: Instant,
    deadline: Option<Instant>,
) -> bool {
    loop {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return false;
        }

        // frameを集めてから、`pump_read`（`session`を`&mut self`で借用中）を
        // 抜けたあとに[`handle_frame`]へ渡す。**その場で`handle_frame(session, ...)`は
        // 呼べない**（`session`の二重可変借用）。`firmware/esp32/src/pi_link.rs`の
        // `PiLink::on_bytes`が採る「集めてから扱う」パターンと同じ理由である。
        let mut frames: Vec<Frame> = Vec::new();
        let read = session.pump_read(device, |outcome| match outcome {
            Outcome::Frame(frame) => {
                let (sid, id) = frame.envelope.identity();
                // **payloadを出さない。**種類だけを出す。中身の扱いは`handle_frame`である。
                log::info!(
                    "行を復元した: sid={sid} id={id} type={}",
                    frame.message.type_str()
                );
                frames.push(frame);
            }
            Outcome::Rejected(rejection) => {
                log::warn!(
                    "行を拒否した: code={:?} cause={:?} detail={}",
                    rejection.code(),
                    rejection.cause(),
                    rejection.detail()
                );
            }
        });
        for frame in frames {
            let received = handle_frame(session, peer, frame, uptime_ms(started));
            log_received(&received);
            if let Some(probe) = probe.as_mut() {
                probe.observe(&received);
            }
        }
        // **切断を観測した周回では、再送も新規の`ping`も出さない。**`note_disconnected`が送信queueを
        // 捨てた後に積むと、再接続の`hello`より先に、別のlinkへ出る。未決の要求は`PeerSession`に残り、
        // 再接続後の周回で再送か取り下げに至る。
        if !matches!(read, Pump::Disconnected) {
            // ACK timeoutした`ping`／`get_status`を同じ`id`で送り直す（§9）。対象は`get_status`と、
            // `--ping-count`を付けたときの`ping`である。`hello`は対象外
            // （`PROTO-TBD-011`。`OutstandingKind::Hello`のdoc参照）。
            let retries = retry_due_requests(session, peer, uptime_ms(started));
            log_retries(&retries);
            if let Some(probe) = probe.as_mut() {
                probe.observe_retries(&retries);
                send_ping(session, peer, probe, uptime_ms(started));
            }
        }
        let write = session.pump_write(device);

        for pump in [read, write] {
            match pump {
                Pump::Disconnected => {
                    log::warn!("切断を観測した");
                    return true;
                }
                Pump::Fatal => {
                    log::error!("再接続では直らないerrorである。停止する");
                    return false;
                }
                Pump::TimedOut => log::debug!("timeout（書き出しが詰まっている）"),
                Pump::Progress(_) | Pump::Idle => {}
                // `Pump`は`#[non_exhaustive]`である。増えたvariantを
                // 「進捗あり」と同じ扱いへ落とさない。見えるようにしておく。
                other => log::warn!("未知のPump: {other:?}"),
            }
        }

        if matches!(session.state(), ConnectionState::Stopped(_)) {
            return false;
        }
    }
}

/// 出力先を持たない最小のlogger。
///
/// **libraryは`log` facadeだけに依存し、実装の選択は呼び出し側に残す**という
/// `Cargo.toml`の方針に従い、ここで選ぶ。依存を増やさないため自前で書く。
mod logger {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 出力に失敗した件数。
    ///
    /// **loggerの失敗をlogへ書けない。**書けばそれ自体が失敗しうる経路へ再入する。
    /// counterへ落として、終了時に1度だけ読む。**握りつぶしとの違いはここである。**
    static WRITE_FAILURES: AtomicU64 = AtomicU64::new(0);

    /// 出力に失敗した件数を読む。
    pub fn write_failures() -> u64 {
        WRITE_FAILURES.load(Ordering::Relaxed)
    }

    struct Stderr;

    impl log::Log for Stderr {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            let mut err = std::io::stderr().lock();
            if writeln!(err, "[{:5}] {}", record.level(), record.args()).is_err() {
                WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
            }
        }

        fn flush(&self) {
            if std::io::stderr().lock().flush().is_err() {
                WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    static LOGGER: Stderr = Stderr;

    /// loggerを1度だけ入れる。
    ///
    /// `set_boxed_logger`ではなく`set_logger`を使うのは、`log`の`std` featureを
    /// 要求しないためである（`tests/simulator.rs`と同じ理由）。
    ///
    /// `level`より下は`log`側で捨てられる。
    ///
    /// # Errors
    ///
    /// 既にloggerが入っている場合に[`log::SetLoggerError`]を返す。**捨てない。**
    /// 捨てると、logが1行も出ない状態のまま走り続ける。
    pub fn install(level: log::LevelFilter) -> Result<(), log::SetLoggerError> {
        log::set_logger(&LOGGER)?;
        log::set_max_level(level);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use deskcat_protocol::{Ack, Boot, ErrorCode};
    use deskcat_serial::{BootHandled, CorrelatedAck};

    use super::*;

    fn established() -> Received {
        Received::Boot(BootHandled {
            outcome: BootOutcome::Established {
                sid: 41_207,
                boot: Boot {
                    firmware: "0.1.0".to_owned(),
                    board: "esp32".to_owned(),
                    reset_reason: "power_on".to_owned(),
                },
            },
            reply: Message::Ack(Ack {
                reply_sid: 1,
                reply_to: 1,
                status: AckStatus::Ok,
                code: None,
                detail: None,
            }),
        })
    }

    fn replayed() -> Received {
        Received::Boot(BootHandled {
            outcome: BootOutcome::Replayed,
            reply: Message::Ack(Ack {
                reply_sid: 1,
                reply_to: 1,
                status: AckStatus::Ok,
                code: None,
                detail: None,
            }),
        })
    }

    fn ack(request: OutstandingKind, reply_to: u32, status: AckStatus) -> Received {
        Received::Ack(CorrelatedAck {
            request,
            ack: Ack {
                reply_sid: 1,
                reply_to,
                code: matches!(status, AckStatus::Rejected).then_some(ErrorCode::StaleSession),
                status,
                detail: None,
            },
        })
    }

    /// 確立した`PingProbe`を作る。
    fn armed(total: u32) -> PingProbe {
        let mut probe = PingProbe::new(total);
        probe.observe(&established());
        probe
    }

    #[test]
    fn nothing_is_sent_before_an_esp32_session_is_established() {
        assert!(!PingProbe::new(3).wants_send());
    }

    #[test]
    fn a_replayed_boot_does_not_arm_the_probe() {
        let mut probe = PingProbe::new(3);
        probe.observe(&replayed());
        assert!(!probe.wants_send());
    }

    #[test]
    fn pings_are_sent_one_at_a_time_until_the_count_is_reached() {
        let mut probe = armed(2);
        assert!(probe.wants_send());
        assert_eq!(probe.on_sent(10), (1, 2));
        assert!(!probe.wants_send(), "未決の間は次を送らない");

        probe.observe(&ack(OutstandingKind::Ping, 10, AckStatus::Ok));
        assert!(probe.wants_send());
        assert_eq!(probe.on_sent(11), (2, 2));
        probe.observe(&ack(OutstandingKind::Ping, 11, AckStatus::Ok));

        assert!(!probe.wants_send(), "回数に達したら送らない");
        assert_eq!(
            probe.counters,
            PingCounters {
                sent: 2,
                acked: 2,
                rejected: 0,
                retried: 0,
                gave_up: 0
            }
        );
    }

    #[test]
    fn a_rejected_ack_is_counted_apart_from_an_ok_ack() {
        let mut probe = armed(1);
        probe.on_sent(10);
        probe.observe(&ack(OutstandingKind::Ping, 10, AckStatus::Rejected));
        assert_eq!((probe.counters.acked, probe.counters.rejected), (0, 1));
        assert!(!probe.wants_send(), "拒否されても回数は戻らない");
    }

    #[test]
    fn an_ack_for_another_request_or_id_is_not_counted() {
        let mut probe = armed(2);
        probe.on_sent(10);
        probe.observe(&ack(OutstandingKind::GetStatus, 10, AckStatus::Ok));
        probe.observe(&ack(OutstandingKind::Ping, 99, AckStatus::Ok));
        assert_eq!(probe.counters.acked, 0);
        assert!(!probe.wants_send(), "未決のままである");
    }

    #[test]
    fn giving_up_frees_the_slot_and_is_counted() {
        let mut probe = armed(2);
        probe.on_sent(10);
        probe.observe_retries(&[RetryOutcome::GaveUp(7, OutstandingKind::GetStatus)]);
        assert_eq!(probe.counters.gave_up, 0, "別の要求の取り下げは数えない");

        probe.observe_retries(&[RetryOutcome::GaveUp(10, OutstandingKind::Ping)]);
        assert_eq!(probe.counters.gave_up, 1);
        assert!(probe.wants_send(), "取り下げの後は次を送る");
    }

    #[test]
    fn a_resend_of_the_pending_ping_is_counted_but_not_a_resend_of_another_request() {
        let mut probe = armed(1);
        probe.on_sent(10);
        probe.observe_retries(&[
            RetryOutcome::Resent(10, OutstandingKind::Ping),
            RetryOutcome::Resent(7, OutstandingKind::GetStatus),
            RetryOutcome::Resent(99, OutstandingKind::Ping),
        ]);
        assert_eq!(probe.counters.retried, 1);
        assert!(!probe.wants_send(), "再送しても未決のままである");
    }

    #[test]
    fn a_new_esp32_session_restarts_the_count_and_drops_the_pending_ping() {
        let mut probe = armed(1);
        probe.on_sent(10);
        assert!(!probe.wants_send(), "回数も使い切り、未決でもある");

        probe.observe(&established());
        assert!(
            probe.wants_send(),
            "数え直し、旧sessionの未決のpingを手放す"
        );
        assert_eq!(probe.counters.sent, 1, "数え上げは通算である");
    }

    #[test]
    fn a_disarmed_probe_is_armed_again_by_the_next_established_session() {
        let mut probe = armed(3);
        probe.disarm();
        assert!(!probe.wants_send());

        probe.observe(&established());
        assert!(probe.wants_send());
    }
}
