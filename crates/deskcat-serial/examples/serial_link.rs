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
//! - 接続のたびに`hello`を送り、その`id`を[`deskcat_serial::PeerSession::note_hello_sent`]で記録する。
//!   未ACKの`hello`は同じ`(sid, id)`と`reason`で送り直す。新しい`hello`の`reason`は
//!   初回が`Startup`、再接続が`PortReopen`である（仕様§5.1）。
//! - 通常時の受信frameは[`deskcat_serial::handle_frame`]へ渡す。`boot`にはACKを返し、
//!   新しいsessionを確立した場合は`get_status`を1件送る。`ack`は送ったrequestと相関させる。`status`は
//!   `get_status`への応答かどうかを分ける（仕様§5.6）。診断中のmotionと切替用helloのACKだけは
//!   この実行体がreply identityとESP32 sidを照合し、専用counterへ記録する。
//! - `--ping-count`を付けたときだけ、ESP32 sessionの確立のたびに`ping`を1件ずつ最大その件数送る
//!   （前の`ping`がACKの相関か取り下げで決着してから次を送る。ESP32 sessionの確立でも未決は手放す。周期は持たない。`PROTO-TBD-010`）。
//! - GPIO UARTの無応答を検出するため、上の診断用pingとは別に、1件ずつ周期的なlink probeを送る。
//!   相関ACKが再送込みで連続して得られなければlink喪失として再openする。周期と閾値は
//!   `LinkWatch`の暫定値であり、`PROTO-TBD-010`と実機試験で見直す。
//! - 通常時はpumpの1周ごとに[`deskcat_serial::retry_due_requests`]を呼ぶ。ACK timeoutした`ping`と`get_status`を
//!   同じ`id`で送り直し（仕様§9）、確立の直後にqueueへ入れられなかった`get_status`を送る。
//!
//! **ESP32が`boot`→ACKより先の往復に応えるのは、`firmware/esp32`を[Issue #487]のPR B2
//! （[PR #529]）以降のsourceから、Pi linkを持つbuild（`bench-servo-test-17`を有効にするbuild以外）で書き込んだ場合である。**PR B2で、
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
//! 場合が当たる（この実行体は同一port内で`hello`のACK timeout再送をしない。`PROTO-TBD-011`）。
//! 仕様は、Piが`stale_session`を受けたら`hello`から再開すると定める（§10.2）。**この実行体は
//! それを実装していない**（[`deskcat_serial::handle_frame`]のdocは、再開を呼び出し側の責務とする）。
//! 再接続で送るのも`PortReopen`の`hello`だけである。そのため、この場合は実行体を起動し直す。
//!
//! [EXP-029]で、Piの`/dev/serial0`から`boot`→ACK→`get_status`→`status`と、診断用`ping`3件への
//! ACKを実portで観察した。host（VM）の擬似端末試験はfixtureの行で応える偽のESP32を使う
//! （`crates/deskcat-serial/README.md`の「host（VM）で確認済みの挙動」）。
//! **実portでのlink喪失判定と再接続は未確認である。**[Issue #11]で追跡する。
//! 記録するときは「行が通った」と「protocolが成立した」を書き分ける。
//!
//! # 使い方
//!
//! ```text
//! cargo run --example serial_link -- --port <path> --baud <rate> \
//!     --duplicate-capacity <n> --duplicate-retention-ms <ms> [--seconds <n>] [--ping-count <n>] [--motion-expire-after-ms <ms>] [--verbose]
//! ```
//!
//! `--port`と`--baud`は**どちらも必須である。**既定値を持たせない。Pi linkの`/dev/serial0`は
//! [EXP-029]でopenと往復を観察した。baudの確定は`PROTO-TBD-001`で、115200は`Candidate`である。
//! **確認していない値を既定として固定しない。**渡した値は記録にそのまま残る。
//!
//! `--duplicate-capacity`と`--duplicate-retention-ms`（`boot`のduplicate履歴の保持件数と
//! 保持期間）も同じ理由で必須である。正本は`PROTO-TBD-005`で未確定である。
//!
//! `--seconds`を省くと、再接続の上限に達してsessionが停止するまで走り続ける。
//!
//! `--ping-count <n>`は1以上で、省くと診断用`ping`を送らない（link probeは動く）。終了時の`counters: pings_sent=… ping_acks=…
//! ping_rejected=… ping_retries=… ping_gave_up=…`は通算である。**`pings_sent`と`ping_acks`が等しく、
//! n以上であれば、送った`ping`のすべてに`ok`のACKを相関できた（n未満なら、まだ送り終えていない）。**等しくなければ、拒否・取り下げ・
//! 未決のまま終わった、またはESP32 sessionの切替で捨てた`ping`がある（`peer: session_switches`は最初の確立でも1になる。2以上なら切替があった）。
//! これは「`ping`へのACKを相関した」ことの根拠であり、`boot`→ACK→`get_status`→`status`の往復の成立とは
//! 別に記録する。
//!
//! `--motion-expire-after-ms <ms>`は#12の拒否経路の診断を有効にする。`--seconds`も必須で、
//! `--ping-count`とは同時に使わない。**実行前にfirmwareが`b3d86ce`以降でcatalogが空の
//! 拒否専用buildであると確認する。**送る名前は`__deskcat_issue_12_reject_only_probe__`だけで、
//! 実在しそうなmotion名は使わない。firmwareの暫定保持期間は`pi_link.rs`の
//! `MOTION_DUPLICATE_RETENTION`（現版10秒、`PROTO-TBD-005`）なので、指定する待ち時間は
//! それより長くする。Piの`--duplicate-retention-ms`は待ち時間より十分長くし、切替後の
//! `boot`再送をPiが拒否しないようにする。例（baudの115200も`PROTO-TBD-001`の候補値）:
//!
//! ```text
//! cargo run --example serial_link -- --port /dev/serial0 --baud 115200 \
//!     --duplicate-capacity 8 --duplicate-retention-ms 30000 \
//!     --motion-expire-after-ms 12000 --seconds 60
//! ```
//!
//! `get_status`への応答を受けたあとに、同じ`(sid, id)`の`play_motion`を2回送り、
//! 保持期間を越えてもう1回送り、新しいPi `sid`の`hello`と`boot`再送のACKを経て旧`sid`から送る。
//! 期待するmotion ACKは順に`rejected/invalid_payload`、同じ`rejected/invalid_payload`、
//! `rejected/duplicate_expired`、`rejected/stale_session`。終了時の専用counterは
//! `motion: first_rejected=1 replay_rejected=1 duplicate_expired=1 stale_session=1 complete=true`。
//! firmware logでも初回の`replayed=false`、2回目の`replayed=true`、期限切れの
//! `duplicate_expired`、旧sidの`stale_session`を同じidentityと突き合わせる。
//! firmwareの`status.payload.protocol` counterは現版では0固定なので、診断の根拠にしない。
//! 空catalogとservoへ渡さない経路の下での拒否試験であり、実motionの非二重実行を実機で
//! 観測したことにはならない。未確定の`PROTO-TBD-005`・`008`・`012`・`013`は残る。
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
//! [EXP-029]: https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/docs/hardware/experiment-log.md
//! [Issue #446]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446
//! [Issue #487]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/487
//! [PR #529]: https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/529

use std::io::Read;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use deskcat_protocol::{
    AckStatus, ErrorCode, FiniteF32, Frame, Hello, HelloReason, Message, Outcome, PlayMotion,
};
use deskcat_serial::{
    BootOutcome, ConnectionState, DuplicatePolicy, OutstandingKind, PeerSession, Pump, Received,
    RetryOutcome, SendError, SerialConfig, SerialDevice, Session, Transport, handle_frame,
    retry_due_requests,
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
    /// 診断で最初のIDの保持期間を越えるまで待つ時間（PROTO-TBD-005）。
    motion_expire_after_ms: Option<u64>,
    verbose: bool,
}

fn usage() -> &'static str {
    "usage: serial_link --port <path> --baud <rate> \
     --duplicate-capacity <n> --duplicate-retention-ms <ms> [--seconds <n>] [--ping-count <n>] [--motion-expire-after-ms <ms>] [--verbose]\n\
     \n\
     --port、--baud、--duplicate-capacity、--duplicate-retention-ms は必須である。既定値を持たせない。\n\
     Pi linkの /dev/serial0 は EXP-029 でopenと往復を観察した。baudの確定は PROTO-TBD-001、duplicate履歴は PROTO-TBD-005 である。\n\
     --ping-count を省くと診断用pingを送らない。link probeは常に動く。"
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
    let mut motion_expire_after_ms = None;
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
                    return Err(
                        "--ping-countは1以上である（省けば診断用pingを送らない）".to_owned()
                    );
                }
                ping_count = Some(n);
            }
            "--motion-expire-after-ms" => {
                let ms = value()?
                    .parse::<u64>()
                    .map_err(|e| format!("--motion-expire-after-msが数値でない: {e}"))?;
                if ms == 0 {
                    return Err("--motion-expire-after-msは1以上である".to_owned());
                }
                motion_expire_after_ms = Some(ms);
            }
            "--verbose" => verbose = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            other => return Err(format!("不明な引数: {other}\n\n{}", usage())),
        }
    }

    if motion_expire_after_ms.is_some() && seconds.is_none() {
        return Err("play_motion診断では--secondsも必要である".to_owned());
    }
    if motion_expire_after_ms.is_some() && ping_count.is_some() {
        return Err("play_motion診断と--ping-countは同時に使えない".to_owned());
    }
    if let (Some(expire), Some(host_retention)) = (motion_expire_after_ms, duplicate_retention_ms)
        && host_retention <= expire
    {
        return Err("--duplicate-retention-msは--motion-expire-after-msより大きくする".to_owned());
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
        motion_expire_after_ms,
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
fn send_hello(
    session: &mut Session,
    peer: &mut PeerSession,
    pending: &mut Option<(u32, HelloReason)>,
    reason: HelloReason,
    now_ms: u64,
) {
    if let Some((id, pending_reason)) = *pending {
        match session.resend(id, hello(pending_reason), now_ms) {
            Ok(()) => log::info!("未ACKのhelloを再送した（id={id}, reason={pending_reason:?}）"),
            Err(error) => log::warn!("未ACKのhelloを再送できない: {error}"),
        }
        return;
    }
    match session.send(hello(reason), now_ms) {
        Ok(id) => {
            peer.note_hello_sent(id);
            *pending = Some((id, reason));
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

/// GPIO UARTの相手応答を監視する。周期と喪失閾値は実機評価前の暫定値
/// （PROTO-TBD-010、PROTO-TBD-004）であり、確定したheartbeat仕様ではない。
/// 1件ずつ送るため、未ACK要求と送信queueは有界のままである。
struct LinkWatch {
    in_flight: Option<u32>,
    next_probe_ms: u64,
    missed: u32,
}

struct Monitoring {
    probe: Option<PingProbe>,
    link_watch: LinkWatch,
    pending_hello: Option<(u32, HelloReason)>,
    motion: Option<MotionDiagnostic>,
}

/// 空catalogに存在しない診断専用名。実行するfirmwareのcatalogが空であることを先に確認する。
const DIAGNOSTIC_MOTION_NAME: &str = "__deskcat_issue_12_reject_only_probe__";

fn diagnostic_motion() -> Message {
    Message::PlayMotion(PlayMotion {
        name: DIAGNOSTIC_MOTION_NAME.to_owned(),
        speed: FiniteF32::new(0.0).expect("0は有限"),
        repeat: 1,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionStep {
    WaitBoot,
    FirstAck,
    ReplayAck,
    WaitExpiry(u64),
    ExpiredAck,
    NewHelloAck(u32),
    WaitNewBoot,
    StaleAck,
    Done,
    Failed,
}

struct MotionDiagnostic {
    step: MotionStep,
    expire_after_ms: u64,
    first_id: Option<u32>,
    old_sid: u32,
    old_session: Option<Session>,
    ready: bool,
    new_boot_seen: bool,
    first_rejections: u32,
    replay_rejections: u32,
    expired_rejections: u32,
    stale_rejections: u32,
}

impl MotionDiagnostic {
    fn new(old_sid: u32, expire_after_ms: u64) -> Self {
        Self {
            step: MotionStep::WaitBoot,
            expire_after_ms,
            first_id: None,
            old_sid,
            old_session: None,
            ready: false,
            new_boot_seen: false,
            first_rejections: 0,
            replay_rejections: 0,
            expired_rejections: 0,
            stale_rejections: 0,
        }
    }

    fn fail(&mut self, reason: &str) {
        log::error!("play_motion診断を停止した: {reason}");
        self.step = MotionStep::Failed;
    }

    fn send_first(&mut self, session: &mut Session, now_ms: u64) {
        match session.send(diagnostic_motion(), now_ms) {
            Ok(id) => {
                self.first_id = Some(id);
                self.step = MotionStep::FirstAck;
                log::info!("play_motion診断: 初回 sid={} id={id}", self.old_sid);
            }
            Err(SendError::Dropped) => {} // 次のpumpで再試行する。
            Err(error) => self.fail(&format!("初回をqueueへ入れられない: {error}")),
        }
    }

    /// motionと切替用helloのACKはPeerSessionの追跡対象外なので、reply identityをここで照合する。
    /// 対象のACKを消費した場合だけ`true`を返す。
    fn on_ack(
        &mut self,
        frame: &Frame,
        now_ms: u64,
        session: &mut Session,
        esp32_sid: Option<u32>,
    ) -> bool {
        let Message::Ack(ack) = &frame.message else {
            return false;
        };
        let expected = match self.step {
            MotionStep::FirstAck | MotionStep::ReplayAck | MotionStep::ExpiredAck => {
                self.first_id.map(|id| (self.old_sid, id))
            }
            MotionStep::NewHelloAck(id) => Some((session.sid(), id)),
            MotionStep::StaleAck => self.first_id.map(|id| (self.old_sid, id)),
            _ => None,
        };
        if expected != Some((ack.reply_sid, ack.reply_to)) {
            return false;
        }
        if esp32_sid != Some(frame.envelope.sid) {
            self.fail("ACKの送信sidが承認したESP32 sidと違う");
            return true;
        }
        let expected_code = match self.step {
            MotionStep::FirstAck | MotionStep::ReplayAck => Some(ErrorCode::InvalidPayload),
            MotionStep::ExpiredAck => Some(ErrorCode::DuplicateExpired),
            MotionStep::StaleAck => Some(ErrorCode::StaleSession),
            MotionStep::NewHelloAck(_) => None,
            _ => unreachable!(),
        };
        let expected_status = if expected_code.is_some() {
            AckStatus::Rejected
        } else {
            AckStatus::Ok
        };
        if ack.status != expected_status || ack.code != expected_code {
            self.fail(&format!(
                "ACKが期待と違う: status={:?} code={:?}",
                ack.status, ack.code
            ));
            return true;
        }
        session.note_peer_responsive();
        log::info!(
            "play_motion診断: ACK reply_sid={} reply_to={} status={:?} code={:?}",
            ack.reply_sid,
            ack.reply_to,
            ack.status,
            ack.code
        );
        match self.step {
            MotionStep::FirstAck => {
                self.first_rejections += 1;
                self.resend_first(session, now_ms, MotionStep::ReplayAck);
            }
            MotionStep::ReplayAck => {
                self.replay_rejections += 1;
                self.step = MotionStep::WaitExpiry(now_ms.saturating_add(self.expire_after_ms));
            }
            MotionStep::ExpiredAck => {
                self.expired_rejections += 1;
                self.switch_session(session, now_ms);
            }
            MotionStep::NewHelloAck(_) => {
                // bootの再送とそのACKを待ってから旧sidのcommandを送る。
                self.step = MotionStep::WaitNewBoot;
            }
            MotionStep::StaleAck => {
                self.stale_rejections += 1;
                self.step = MotionStep::Done;
            }
            _ => unreachable!(),
        }
        true
    }

    fn resend_first(&mut self, session: &mut Session, now_ms: u64, next: MotionStep) {
        let Some(id) = self.first_id else {
            self.fail("初回IDが無い");
            return;
        };
        match session.resend(id, diagnostic_motion(), now_ms) {
            Ok(()) => self.step = next,
            Err(error) => self.fail(&format!("再送をqueueへ入れられない: {error}")),
        }
    }

    fn switch_session(&mut self, session: &mut Session, now_ms: u64) {
        let new_sid = match random_sid_except(self.old_sid) {
            Ok(sid) => sid,
            Err(error) => {
                self.fail(&format!("新sidを選べない: {error}"));
                return;
            }
        };
        let mut next = Session::new(session.config().clone(), new_sid);
        next.note_connected();
        let id = match next.send(hello(HelloReason::Startup), now_ms) {
            Ok(id) => id,
            Err(error) => {
                self.fail(&format!("新sidのhelloを送れない: {error}"));
                return;
            }
        };
        self.old_session = Some(std::mem::replace(session, next));
        self.step = MotionStep::NewHelloAck(id);
        log::info!("play_motion診断: 新sid={new_sid} のhelloを送った id={id}");
    }

    fn tick(&mut self, session: &mut Session, now_ms: u64) {
        match self.step {
            MotionStep::WaitBoot if self.ready => self.send_first(session, now_ms),
            MotionStep::WaitExpiry(deadline) if now_ms >= deadline => {
                self.resend_first(session, now_ms, MotionStep::ExpiredAck);
            }
            MotionStep::WaitNewBoot if self.new_boot_seen && session.pending_out() == 0 => {
                let Some(old) = self.old_session.as_mut() else {
                    self.fail("旧sessionが無い");
                    return;
                };
                let Some(id) = self.first_id else {
                    self.fail("初回IDが無い");
                    return;
                };
                match old.resend(id, diagnostic_motion(), now_ms) {
                    Ok(()) => {
                        self.step = MotionStep::StaleAck;
                        log::info!("play_motion診断: 旧sid={} id={id}を再送", self.old_sid);
                    }
                    Err(error) => self.fail(&format!("旧sidの送出に失敗: {error}")),
                }
            }
            _ => {}
        }
    }

    fn report(&self) {
        log::info!(
            "motion: first_rejected={} replay_rejected={} duplicate_expired={} stale_session={} complete={}",
            self.first_rejections,
            self.replay_rejections,
            self.expired_rejections,
            self.stale_rejections,
            self.step == MotionStep::Done
        );
    }
}

fn random_sid_except(excluded: u32) -> std::io::Result<u32> {
    let mut random = std::fs::File::open("/dev/urandom")?;
    loop {
        let mut bytes = [0_u8; 4];
        random.read_exact(&mut bytes)?;
        let sid = u32::from_le_bytes(bytes);
        if sid != 0 && sid != excluded {
            return Ok(sid);
        }
    }
}

/// 診断は2つのPi sessionを作るので、両方ともOS乱数からsidを選ぶ（仕様§3.1）。
fn initial_sid(motion_diagnostic: bool) -> std::io::Result<u32> {
    if motion_diagnostic {
        random_sid_except(0)
    } else {
        // 通常実行のPID由来のsidは既存の挙動として残す。
        Ok(std::process::id())
    }
}

impl LinkWatch {
    const INTERVAL_MS: u64 = 1_000; // 暫定。heartbeatの周期はPROTO-TBD-010。
    const MISSED_LIMIT: u32 = 2; // 暫定。1回のGaveUpは初回と再送の2回のACK timeout。

    fn new() -> Self {
        Self {
            in_flight: None,
            next_probe_ms: 0,
            missed: 0,
        }
    }

    fn observe(&mut self, received: &Received, now_ms: u64) {
        match received {
            Received::Boot(handled)
                if matches!(handled.outcome, BootOutcome::Established { .. }) =>
            {
                // PeerSessionはsession遷移時に旧outstandingを捨てる。
                self.in_flight = None;
                self.missed = 0;
                self.next_probe_ms = now_ms;
            }
            Received::Ack(ack)
                if matches!(ack.request, OutstandingKind::Ping)
                    && self.in_flight == Some(ack.ack.reply_to) =>
            {
                // rejectedでも相関ACKが返ったなら、対向processは応答している。
                self.in_flight = None;
                self.missed = 0;
                self.next_probe_ms = now_ms.saturating_add(Self::INTERVAL_MS);
            }
            Received::Ack(_) => {
                // 別の要求への相関ACKも、双方向のlinkが生きている証拠である。
                // probe自身の未完了IDはそのまま追跡する。
                self.missed = 0;
            }
            _ => {}
        }
    }

    fn observe_retries(&mut self, outcomes: &[RetryOutcome], now_ms: u64) -> bool {
        for outcome in outcomes {
            if let RetryOutcome::GaveUp(id, OutstandingKind::Ping) = outcome
                && self.in_flight == Some(*id)
            {
                self.in_flight = None;
                self.missed += 1;
                self.next_probe_ms = now_ms.saturating_add(Self::INTERVAL_MS);
                log::warn!(
                    "link probeのACKが無い: {}/{}",
                    self.missed,
                    Self::MISSED_LIMIT
                );
                if self.missed >= Self::MISSED_LIMIT {
                    return true;
                }
            }
        }
        false
    }

    fn send_due(&mut self, session: &mut Session, peer: &mut PeerSession, now_ms: u64) {
        // 相手のsidをbootで承認するまで、pingのACKは相関できない。
        if self.in_flight.is_some() || now_ms < self.next_probe_ms || peer.esp32_sid().is_none() {
            return;
        }
        match session.send(Message::Ping, now_ms) {
            Ok(id) => {
                let _ = peer.note_sent(id, Message::Ping, now_ms);
                self.in_flight = Some(id);
                log::debug!("link probeを送った: id={id}");
            }
            Err(SendError::Dropped) => log::warn!("link probeのqueueが満杯。次のtickで再試行する"),
            Err(error) => log::error!("link probeを送れない: {error}"),
        }
    }

    fn on_disconnect(&mut self, peer: &mut PeerSession) {
        if let Some(id) = self.in_flight.take() {
            let _ = peer.forget_sent(id);
        }
        self.missed = 0;
        self.next_probe_ms = 0;
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

fn next_hello_reason(first_connect: &mut bool) -> HelloReason {
    let reason = if *first_connect {
        HelloReason::Startup
    } else {
        HelloReason::PortReopen
    };
    *first_connect = false;
    reason
}

fn install_logger(verbose: bool) -> Result<(), ()> {
    // 長時間観察のread timeoutを埋めないよう、既定はInfoとする。
    let level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    logger::install(level).map_err(|error| {
        eprintln!("loggerを入れられない: {error}");
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

    if install_logger(args.verbose).is_err() {
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

    let sid = initial_sid(args.motion_expire_after_ms.is_some())
        .map_err(|error| log::error!("診断の初期sidを選べない: {error}"));
    let Ok(sid) = sid else {
        return ExitCode::FAILURE;
    };
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
    let mut monitoring = Monitoring {
        probe: args.ping_count.map(PingProbe::new),
        link_watch: LinkWatch::new(),
        pending_hello: None,
        motion: args
            .motion_expire_after_ms
            .map(|ms| MotionDiagnostic::new(sid, ms)),
    };

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
        let reason = next_hello_reason(&mut first_connect);
        send_hello(
            &mut session,
            &mut peer,
            &mut monitoring.pending_hello,
            reason,
            uptime_ms(started),
        );

        let disconnected = pump_until_break(
            &mut session,
            &mut peer,
            &mut monitoring,
            &mut device,
            started,
            deadline,
        );

        if !disconnected {
            break; // deadline到達、または停止済み。summaryはloopの外で1度だけ出す
        }

        monitoring.link_watch.on_disconnect(&mut peer);
        if let Some(motion) = monitoring.motion.as_mut() {
            motion.fail("診断中にlinkが切れた。再接続後の続行はしない");
        }

        // 切断ごとの区切りとして出す。**loopを抜けた後にもう一度出さない**
        // （同じ数字が2回並ぶと、どちらが最終値か読めない）。
        report(&session, &peer, monitoring.probe.as_ref());

        if monitoring.motion.is_some() {
            break;
        }

        let Some(backoff) = session.begin_reconnect() else {
            log::error!("再接続の上限に達した。停止する");
            break;
        };
        let wait = capped_backoff(backoff, deadline);
        log::info!("切断した。{wait:?} 待って再接続する");
        sleep(wait);
    }

    finish_run(&session, &peer, &monitoring, started)
}

fn finish_run(
    session: &Session,
    peer: &PeerSession,
    monitoring: &Monitoring,
    started: Instant,
) -> ExitCode {
    report(session, peer, monitoring.probe.as_ref());
    if let Some(motion) = monitoring.motion.as_ref() {
        if let Some(old) = motion.old_session.as_ref() {
            let counters = old.counters();
            log::info!(
                "motion old_session sid={} state={:?}",
                old.sid(),
                old.state()
            );
            log::info!(
                "motion old_session counters: bytes_in={} bytes_out={} frames_in={} rejected_in={}",
                counters.bytes_in,
                counters.bytes_out,
                counters.frames_in,
                counters.rejected_in
            );
            log::info!(
                "motion old_session counters: disconnects={} reconnect_attempts={} timeouts={} retries={}",
                counters.disconnects,
                counters.reconnect_attempts,
                counters.timeouts,
                counters.retries
            );
            log::info!(
                "motion old_session counters: dropped_out={} discarded_on_disconnect={} encode_failed={}",
                counters.dropped_out,
                counters.discarded_on_disconnect,
                counters.encode_failed
            );
        }
        motion.report();
    }
    log::info!("経過 {:?}", started.elapsed());

    // **握りつぶしていないことをここで示す。**0件でも出す。
    let failures = logger::write_failures();
    if failures > 0 {
        eprintln!("log出力の失敗: {failures} 件");
    } else {
        log::info!("log出力の失敗: 0 件");
    }

    if matches!(session.state(), ConnectionState::Stopped(_))
        || monitoring
            .motion
            .as_ref()
            .is_some_and(|motion| motion.step != MotionStep::Done)
    {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn process_frames(
    frames: Vec<Frame>,
    session: &mut Session,
    peer: &mut PeerSession,
    monitoring: &mut Monitoring,
    started: Instant,
) {
    for frame in frames {
        let now_ms = uptime_ms(started);
        if let Some(motion) = monitoring.motion.as_mut()
            && motion.on_ack(&frame, now_ms, session, peer.esp32_sid())
        {
            if session.sid() != motion.old_sid {
                monitoring.pending_hello = None;
            }
            continue;
        }
        let had_esp32_session = peer.esp32_sid().is_some();
        let received = handle_frame(session, peer, frame, now_ms);
        if let Some(motion) = monitoring.motion.as_mut()
            && matches!(&received, Received::Status(accepted) if accepted.solicited)
        {
            motion.ready = true;
        }
        if let Some(motion) = monitoring.motion.as_mut()
            && motion.step == MotionStep::WaitNewBoot
            && let Received::Boot(handled) = &received
        {
            match handled.outcome {
                BootOutcome::Replayed => motion.new_boot_seen = true,
                BootOutcome::Established { .. } => {
                    motion.fail("新Pi sidへの切替中にESP32 sidが変わった");
                }
                _ => {}
            }
        }
        if matches!(received, Received::Ack(_)) {
            session.note_peer_responsive();
        }
        if matches!(&received, Received::Ack(ack) if matches!(ack.request, OutstandingKind::Hello))
        {
            monitoring.pending_hello = None;
        }
        if had_esp32_session
            && matches!(&received, Received::Boot(handled) if matches!(handled.outcome, BootOutcome::Established { .. }))
        {
            monitoring.pending_hello = None; // PeerSessionも旧sessionへのhello追跡を捨てた。
        }
        log_received(&received);
        monitoring.link_watch.observe(&received, now_ms);
        if let Some(probe) = monitoring.probe.as_mut() {
            probe.observe(&received);
        }
    }
}

/// 旧sidの1行がpartial writeになっても、新sidの行のbyteを途中に挟まない。
fn pump_writes<T: Transport>(
    session: &mut Session,
    monitoring: &mut Monitoring,
    device: &mut T,
) -> (Pump, Pump) {
    let old_pending = monitoring
        .motion
        .as_ref()
        .and_then(|motion| motion.old_session.as_ref())
        .is_some_and(|old| old.pending_out() > 0);
    let active = if old_pending {
        Pump::Idle
    } else {
        session.pump_write(device)
    };
    let old = monitoring
        .motion
        .as_mut()
        .and_then(|motion| motion.old_session.as_mut())
        .map_or(Pump::Idle, |old| old.pump_write(device));
    (active, old)
}

/// linkが切れるか、期限に達するまでpumpを回す。切断で抜けたときだけ`true`。
fn pump_until_break(
    session: &mut Session,
    peer: &mut PeerSession,
    monitoring: &mut Monitoring,
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
        process_frames(frames, session, peer, monitoring, started);
        if !matches!(read, Pump::Disconnected | Pump::Fatal)
            && let Some(motion) = monitoring.motion.as_mut()
        {
            motion.tick(session, uptime_ms(started));
        }
        // **切断を観測した周回では、再送も新規の`ping`も出さない。**`note_disconnected`が送信queueを
        // 捨てた後に積むと、再接続の`hello`より先に、別のlinkへ出る。未決の要求は`PeerSession`に残り、
        // 再接続後の周回で再送か取り下げに至る。
        if !matches!(read, Pump::Disconnected)
            && !monitoring
                .motion
                .as_ref()
                .is_some_and(|motion| motion.ready)
        {
            // ACK timeoutした`ping`／`get_status`を同じ`id`で送り直す（§9）。対象は`get_status`と、
            // `--ping-count`を付けたときの`ping`である。`hello`は対象外
            // （`PROTO-TBD-011`。`OutstandingKind::Hello`のdoc参照）。
            let retries = retry_due_requests(session, peer, uptime_ms(started));
            log_retries(&retries);
            if monitoring.motion.is_none()
                && monitoring
                    .link_watch
                    .observe_retries(&retries, uptime_ms(started))
            {
                session.note_peer_unresponsive();
                return true;
            }
            if let Some(probe) = monitoring.probe.as_mut() {
                probe.observe_retries(&retries);
                send_ping(session, peer, probe, uptime_ms(started));
            }
            if monitoring.motion.is_none() {
                monitoring
                    .link_watch
                    .send_due(session, peer, uptime_ms(started));
            }
        }
        let (write, old_write) = pump_writes(session, monitoring, device);
        match old_write {
            Pump::Disconnected => {
                log::warn!("旧sidの送出中に切断した");
                return true;
            }
            Pump::Fatal => {
                log::error!("旧sidの送出中に回復不能なI/O error");
                return false;
            }
            _ => {}
        }

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
        if monitoring
            .motion
            .as_ref()
            .is_some_and(|motion| matches!(motion.step, MotionStep::Done | MotionStep::Failed))
        {
            return false;
        }
    }
}

#[cfg(test)]
mod simulator_tests {
    use super::*;
    use deskcat_protocol::{Ack, Envelope, decode_line, limits};
    use std::io::{self, Cursor};

    struct PartialWire {
        bytes: Vec<u8>,
    }

    impl Transport for PartialWire {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::WouldBlock.into())
        }

        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let n = buf.len().min(3);
            self.bytes.extend_from_slice(&buf[..n]);
            Ok(n)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn ack(reply_sid: u32, reply_to: u32, status: AckStatus, code: Option<ErrorCode>) -> Frame {
        Frame::new(
            Envelope {
                v: limits::PROTOCOL_VERSION,
                sid: 42,
                id: 100,
                ts_ms: 0,
            },
            Message::Ack(Ack {
                reply_sid,
                reply_to,
                status,
                code,
                detail: None,
            }),
        )
    }

    fn written(session: &mut Session) -> Frame {
        let mut wire = Cursor::new(Vec::new());
        assert!(matches!(session.pump_write(&mut wire), Pump::Progress(_)));
        let line = String::from_utf8(wire.into_inner()).expect("UTF-8のJSONL");
        decode_line(line.trim_end()).expect("送出したJSONLをdecodeできる")
    }

    #[test]
    fn motion_diagnostic_sends_same_identity_then_expired_then_old_sid() {
        let config = SerialConfig::new("/dev/simulated", 115_200).unwrap();
        let mut session = Session::new(config, 7);
        session.note_connected();
        let mut diagnostic = MotionDiagnostic::new(7, 100);
        diagnostic.ready = true; // fake ESP32のboot→ACK→get_status→statusを受理した状態。
        diagnostic.tick(&mut session, 0);
        let first = written(&mut session);
        let id = first.envelope.id;
        assert_eq!(first.envelope.sid, 7);
        assert_eq!(first.message, diagnostic_motion());

        assert!(diagnostic.on_ack(
            &ack(7, id, AckStatus::Rejected, Some(ErrorCode::InvalidPayload)),
            1,
            &mut session,
            Some(42)
        ));
        let replay = written(&mut session);
        assert_eq!(replay.envelope.identity(), first.envelope.identity());
        assert_eq!(replay.message, first.message);

        assert!(diagnostic.on_ack(
            &ack(7, id, AckStatus::Rejected, Some(ErrorCode::InvalidPayload)),
            2,
            &mut session,
            Some(42)
        ));
        diagnostic.tick(&mut session, 101);
        assert_eq!(diagnostic.step, MotionStep::WaitExpiry(102));
        diagnostic.tick(&mut session, 102);
        let expired = written(&mut session);
        assert_eq!(expired.envelope.identity(), first.envelope.identity());

        assert!(diagnostic.on_ack(
            &ack(
                7,
                id,
                AckStatus::Rejected,
                Some(ErrorCode::DuplicateExpired)
            ),
            103,
            &mut session,
            Some(42)
        ));
        let new_sid = session.sid();
        assert_ne!(new_sid, 7);
        let new_hello = written(&mut session);
        assert_eq!(new_hello.envelope.sid, new_sid);
        assert!(matches!(
            new_hello.message,
            Message::Hello(Hello {
                reason: HelloReason::Startup,
                ..
            })
        ));

        assert!(diagnostic.on_ack(
            &ack(new_sid, new_hello.envelope.id, AckStatus::Ok, None),
            104,
            &mut session,
            Some(42)
        ));
        diagnostic.new_boot_seen = true; // fake ESP32のboot再送を受理した状態。
        diagnostic.tick(&mut session, 105);
        let stale = written(diagnostic.old_session.as_mut().unwrap());
        assert_eq!(stale.envelope.identity(), first.envelope.identity());
        assert_eq!(stale.message, first.message);
        assert!(diagnostic.on_ack(
            &ack(7, id, AckStatus::Rejected, Some(ErrorCode::StaleSession)),
            106,
            &mut session,
            Some(42)
        ));
        assert_eq!(diagnostic.step, MotionStep::Done);
        assert_eq!(
            (
                diagnostic.first_rejections,
                diagnostic.replay_rejections,
                diagnostic.expired_rejections,
                diagnostic.stale_rejections
            ),
            (1, 1, 1, 1)
        );
    }

    #[test]
    fn diagnostic_does_not_accept_an_ack_from_a_different_esp32_session() {
        let config = SerialConfig::new("/dev/simulated", 115_200).unwrap();
        let mut session = Session::new(config, 7);
        session.note_connected();
        let mut diagnostic = MotionDiagnostic::new(7, 100);
        diagnostic.ready = true;
        diagnostic.tick(&mut session, 0);
        let id = diagnostic.first_id.unwrap();
        assert!(diagnostic.on_ack(
            &ack(7, id, AckStatus::Rejected, Some(ErrorCode::InvalidPayload)),
            1,
            &mut session,
            Some(99)
        ));
        assert_eq!(diagnostic.step, MotionStep::Failed);
        assert_eq!(diagnostic.first_rejections, 0);
    }

    #[test]
    fn diagnostic_stops_when_the_firmware_returns_the_wrong_rejection() {
        let config = SerialConfig::new("/dev/simulated", 115_200).unwrap();
        let mut session = Session::new(config, 7);
        session.note_connected();
        let mut diagnostic = MotionDiagnostic::new(7, 100);
        diagnostic.ready = true;
        diagnostic.tick(&mut session, 0);
        let id = diagnostic.first_id.unwrap();
        assert!(diagnostic.on_ack(
            &ack(7, id, AckStatus::Rejected, Some(ErrorCode::StaleSession)),
            1,
            &mut session,
            Some(42)
        ));
        assert_eq!(diagnostic.step, MotionStep::Failed);
        assert_eq!(diagnostic.first_rejections, 0);
    }

    #[test]
    fn old_and_new_sessions_do_not_interleave_partial_lines() {
        let config = SerialConfig::new("/dev/simulated", 115_200).unwrap();
        let mut old = Session::new(config.clone(), 7);
        old.note_connected();
        old.send(diagnostic_motion(), 0).unwrap();
        let mut active = Session::new(config, 8);
        active.note_connected();
        active.send(hello(HelloReason::Startup), 0).unwrap();
        let mut motion = MotionDiagnostic::new(7, 100);
        motion.old_session = Some(old);
        let mut monitoring = Monitoring {
            probe: None,
            link_watch: LinkWatch::new(),
            pending_hello: None,
            motion: Some(motion),
        };
        let mut wire = PartialWire { bytes: Vec::new() };
        for _ in 0..200 {
            let (active_result, old_result) = pump_writes(&mut active, &mut monitoring, &mut wire);
            assert!(matches!(active_result, Pump::Idle | Pump::Progress(_)));
            assert!(matches!(old_result, Pump::Idle | Pump::Progress(_)));
            if active.pending_out() == 0
                && monitoring
                    .motion
                    .as_ref()
                    .unwrap()
                    .old_session
                    .as_ref()
                    .unwrap()
                    .pending_out()
                    == 0
            {
                break;
            }
        }
        let lines = String::from_utf8(wire.bytes).unwrap();
        let frames: Vec<Frame> = lines
            .lines()
            .map(|line| decode_line(line).unwrap())
            .collect();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].envelope.sid, 7);
        assert_eq!(frames[0].message, diagnostic_motion());
        assert_eq!(frames[1].envelope.sid, 8);
        assert!(matches!(frames[1].message, Message::Hello(_)));
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

    #[test]
    fn reconnect_resends_unanswered_hello_with_the_same_identity_and_reason() {
        let config = SerialConfig::new("/dev/simulated", 115_200).unwrap();
        let mut session = Session::new(config, 7);
        let duplicate_policy = DuplicatePolicy::new(2, Duration::from_secs(1)).unwrap();
        let mut peer = PeerSession::new(duplicate_policy);
        let mut pending = None;

        session.note_connected();
        send_hello(
            &mut session,
            &mut peer,
            &mut pending,
            HelloReason::Startup,
            0,
        );
        let first = pending;
        session.note_peer_unresponsive();
        session.note_connected();
        send_hello(
            &mut session,
            &mut peer,
            &mut pending,
            HelloReason::PortReopen,
            1_000,
        );
        assert_eq!(pending, first);
        assert_eq!(
            session.pending_out(),
            1,
            "切断時のqueueを捨てて同じhelloだけを再送"
        );
    }

    #[test]
    fn link_watch_requires_consecutive_exhausted_ping_retries() {
        let mut watch = LinkWatch::new();
        watch.in_flight = Some(10);
        assert!(!watch.observe_retries(&[RetryOutcome::GaveUp(99, OutstandingKind::Ping)], 1_000));
        assert!(!watch.observe_retries(&[RetryOutcome::GaveUp(10, OutstandingKind::Ping)], 1_000));
        assert_eq!(watch.missed, 1);
        assert_eq!(watch.next_probe_ms, 2_000, "次のprobeまで間隔を空ける");

        watch.in_flight = Some(11);
        watch.observe(&ack(OutstandingKind::Ping, 11, AckStatus::Ok), 2_000);
        assert_eq!(watch.missed, 0, "相関ACKで連続喪失数を戻す");

        watch.in_flight = Some(12);
        assert!(!watch.observe_retries(&[RetryOutcome::GaveUp(12, OutstandingKind::Ping)], 3_000));
        watch.in_flight = Some(13);
        assert!(watch.observe_retries(&[RetryOutcome::GaveUp(13, OutstandingKind::Ping)], 5_000));
    }

    #[test]
    fn link_watch_starts_probing_only_after_boot_establishes_the_peer_session() {
        let config = SerialConfig::new("/dev/simulated", 115_200).unwrap();
        let mut session = Session::new(config, 7);
        let duplicate_policy = DuplicatePolicy::new(2, Duration::from_secs(1)).unwrap();
        let mut peer = PeerSession::new(duplicate_policy);
        let mut watch = LinkWatch::new();

        session.note_connected();
        watch.send_due(&mut session, &mut peer, 0);
        assert_eq!(watch.in_flight, None);
        assert_eq!(session.pending_out(), 0);

        let handled = peer.handle_boot(
            41_207,
            1,
            Boot {
                firmware: "0.1.0".to_owned(),
                board: "esp32".to_owned(),
                reset_reason: "power_on".to_owned(),
            },
            1,
        );
        watch.observe(&Received::Boot(handled), 1);
        watch.send_due(&mut session, &mut peer, 1);
        assert!(watch.in_flight.is_some());
        assert_eq!(session.pending_out(), 1);
    }

    #[test]
    fn link_watch_keeps_its_probe_but_recognizes_other_correlated_acks() {
        let mut watch = LinkWatch::new();
        watch.in_flight = Some(10);
        watch.missed = 1;
        watch.observe(&ack(OutstandingKind::Ping, 99, AckStatus::Ok), 1_000);
        watch.observe(&ack(OutstandingKind::GetStatus, 10, AckStatus::Ok), 1_000);
        assert_eq!(watch.in_flight, Some(10));
        assert_eq!(watch.missed, 0);
    }

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
