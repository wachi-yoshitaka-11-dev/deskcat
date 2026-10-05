//! linkの一生の1周ぶん（[`Daemon::step`]）。受信、再送、感情の時間進行、送信を順に行う。
//! **portを開くのは呼び出し側であり、linkの断は再接続せずに終了で扱う。**
//!
//! # 1 process = 1 `Session` + 1 `PeerSession`
//!
//! **`Session`と`PeerSession`を作り直さない。**`sid`は[process起動ごとに乱数から選ぶ](crate::sid)
//! ため、Pi側でsessionをやり直す必要が出たら、[`Step::Exit`]で終了し、supervisor（systemdの
//! `Restart=`）に新しい`sid`で起動し直してもらう（[#491の決定]、§10.1）。
//! これで、停止した後の再開、sessionが替わったときの状態の初期化、`id`の使い回し、
//! sessionをまたいで残った`set_expression`の追跡、という問題が起きない。
//! **作り直しの代わりに終了する**ので、状態は1つのsessionの分しか持たない。
//!
//! # linkの断は扱わない
//!
//! **serial linkの断（EOF、切断のI/O error、致命的なI/O error）は、すべてprocessの終了（75）で扱う。**
//! 再接続も、`port_reopen`の`hello`も持たない。supervisorが新しい`sid`でprocessを起動し直し、
//! §10.2の手順に入る（§3は「protocolはその再起動の自動化を禁じない」とする）。
//! §10.1は、serial linkが繋がり直しただけでESP32 processが再起動していない場合を、ESP32の再起動の
//! 手順に入れず、Piは現在のESP32 sessionとduplicate履歴を保持する、と定める。その理由は
//! 「link断を再起動と誤って扱うと、動作中のmotionを不要に停止し、retryを二重実行に変える。」である。
//! **この戒めの対象は、link断を「ESP32の再起動」と誤って扱うこと**であり、このprocessの設計は
//! link断をESP32の再起動とは扱わない（ESP32 sessionは、process終了とともにPiの側で捨てる。ESP32は
//! 再起動していないので、新しいPi processの`hello`の後に`boot`を再送する。§10.2手順5）。
//! **Pi自身のprocessの再起動**は、§10.2の手順（Piが再起動した場合）であり、§3は「protocolはその再起動の
//! 自動化を禁じない」とする。ただし**再起動のたびに、ESP32はPiの旧sessionをretiredにし、実行中の
//! relative motionを安全に停止する**（§10.2手順2〜3）。このprocessはmotionを送らないので失うものは
//! 表情の同期だけだが、linkが切れるたびにそうなる。この代償と引き換えに、再接続と`hello`と再開の
//! 待ちの状態が重なる組み合わせを持たない（2b-iの自己レビューで、この組み合わせから欠陥が続けて出た）。
//!
//! # 終了する場合（[`ExitReason`]）
//!
//! | 条件 | 終了コード | 根拠 |
//! |---|---|---|
//! | linkが切れた、または致命的なI/O errorで`Session`が停止した | 75 | 上の「linkの断は扱わない」 |
//! | 確立済みのESP32 sessionから、別の`sid`の`boot`で遷移した | 75 | §10.1。ESP32が再起動した |
//! | Piの`hello`が`stale_session`で拒否された | 75 | §3.1。`sid`の衝突。processの再起動で選び直す |
//! | `hello`の同じ`(sid, id)`の再送（`rate_limited`の拒否の後、またはACKが無いとき）の回数が、上限（[`HELLO_RETRY_LIMIT`]）を超えた | 75 | §5.1。有限のbudgetを使い切った。**ESP32が起動していないと、約2秒でここに至る**（下の「ESP32が応答しないとき」） |
//! | 再開の回数が上限（[`RESUME_LIMIT`]）を超えた | 75 | 下の「再開」 |
//! | `get_status`が無応答で取り下げられた連続の回数が上限（[`STATUS_RETRY_LIMIT`]）を超えた | 75 | 下の「応答が無いとき」 |
//! | `Session`が停止した（`id`空間の枯渇） | 75 | §3。新しいsessionが要る。再起動が復帰の経路である |
//! | Piの`hello`が上の以外の理由で拒否された、`get_status`が決まった拒否（下の表）を受けた、または`hello`をencodeできない | 1 | 設定か実装の誤り。決まった拒否は再送でも変わらない |
//!
//! **75も1も、再起動の回数の上限はsupervisor（systemdの`StartLimitBurst`）が持つ。**このcrateは持たない。
//! 選び直しの回数の上限は`PROTO-TBD-011`が未確定であり、unit（段階2b-iii）が入るまで、上限の担保は
//! この差分の中に無い。
//!
//! # 要求の結果と振る舞いの表
//!
//! Piが送る要求（`hello`（`startup`だけ）、`get_status`、`set_expression`）と、返りうる結果の
//! **全数**である。`ping`と、`port_reopen`・`resync`の`hello`は送らない。[`crate::disposition::disposition`]が
//! この表を関数にしたもので、`tests/disposition.rs`が`ErrorCode::ALL`の全codeを走査して表と一致することを、
//! `tests/daemon.rs`の`cell_*`が振る舞いを固定している（固定していない組み合わせは、`disposition`単体の
//! testだけが固定する）。**表を直すときは関数とtestを一緒に直す。**
//!
//! 「再要求」「再送」はcooldown（ACK timeoutと同じ長さ。**根拠は無い**。`PROTO-TBD-012`が未確定である）の後に行う。
//!
//! | 結果 | `hello` | `get_status` | `set_expression` |
//! |---|---|---|---|
//! | `ok` | 確立を確かめた。stateを取り直す（§10.1手順4、§10.2手順6） | `status`を待つ。**待ちにtimeoutを持たない**（§5.6）。`status`がまだ要るあいだ、ACK timeoutと同じ間隔で、新しい`id`の`get_status`を送る（§5.6が許す。上限は置かない） | 反映済みとする |
//! | `stale_session` | 終了75。`sid`の衝突（§3.1） | 再開（下記。§3.1、§10.2） | 再開。表情の内容は拒否されていない |
//! | `rate_limited` | cooldownの後、**同じ`(sid, id)`**で再送。[`HELLO_RETRY_LIMIT`]の後は終了75（§5.1の拒否code表） | cooldownの後、新しい`id`で再要求（上限は置かない） | cooldownの後、新しい`id`で再送（§9）。[`crate::expression_sync::TRANSIENT_RETRY_LIMIT`]の後は停止 |
//! | `busy` | 終了1（仕様が`hello`の再送を定めない） | cooldownの後、新しい`id`で再要求（上限は置かない。§7「待てば受け付けられる」） | cooldownの後、新しい`id`で再送（§7「待てば受け付けられる」）。`rate_limited`と同じ`TRANSIENT_RETRY_LIMIT`（3つのcodeで共有）の後は停止 |
//! | `duplicate_expired` | 終了1 | cooldownの後、新しい`id`で再要求（結果が不明なだけで、状態を読むだけのcommand。§5.6） | cooldownの後、新しい`id`で再送。結果が不明なだけで、idempotent（§9）。上と同じ上限の後は停止 |
//! | `invalid_payload` | 終了1（§5.1の拒否code表。終端で、同じ`sid`のままcommandを送らない）。再起動すれば新しい`sid`の`startup`になる（§3） | 終了1（決まった拒否。繰り返しても結果は変わらない） | 停止。内容の拒否 |
//! | `unsupported_version`、`unknown_type`、`invalid_envelope`、`out_of_range`、`line_too_long`、`hardware_unavailable` | 終了1（`unsupported_version`は§5.1が終端。他は再評価しても変わらない拒否） | 終了1（同上） | 停止。`hardware_unavailable`は待っても受け付けられない（§7）。`unknown_type`のACKは、現行のfirmwareからは返らない（`firmware/esp32/src/pi_link.rs`の`other`腕は`set_expression`に応答しない。処理を足すのは#21）。応答なしの行を参照 |
//! | `code`が無い拒否 | 終了1 | 終了1（仕様は`code`を必須とする。違反は安全側に倒す） | 停止（仕様は`code`を必須とする。違反は安全側に倒す） |
//! | 応答なし（ACK timeoutまでACKが無い） | **同じ`(sid, id)`**で再送。`rate_limited`と同じ予算（[`HELLO_RETRY_LIMIT`]）を使い切れば終了75（§5.1。ACKの喪失で`sid`を選び直さない） | 新しい`id`で再要求。連続が[`STATUS_RETRY_LIMIT`]を超えたら終了75 | 新しい`id`で送り直す。結果は不明だが、状態設定commandはidempotent（§9）。**連続が[`crate::expression_sync::SILENT_RETRY_LIMIT`]を超えたら、欲しい表情が変わるまで送らない**（終了しない。表情だけで、再起動しても直らない）。止めたときは`log::error!`と`set_expression_stopped_silent`で報告する（§9「診断を報告する」） |
//! | 相関できず落ちる（`UnapprovedHelloAck`。§6） | 拒否のcodeは**届かない**（下記）。ESP32が応答したので、結果は不明のまま`hello`を済んだものとして扱い、再送しない | 該当しない | 該当しない |
//!
//! `get_status`の再要求が新しい`id`なのは、状態を読むだけで二重実行の問題が無いため（§5.6）。
//! `set_expression`の再送が新しい`id`なのは、通常commandの`rate_limited`は最終的な拒否であり、改めて要求する
//! 場合は新しい`id`を使うため（§9）。`hello`の`rate_limited`の再送とACKが無いときの再送だけが、同じ`(sid, id)`を扱う。
//! `stale_session`の再開の`hello`は、新しい`id`である。
//!
//! # 再開（通常commandが`stale_session`で拒否された場合）
//!
//! §3.1は、`sid`の選び直しの規則について次のように定める。
//! 「この規則は`hello`／`boot`自身が拒否された場合にだけ適用する。通常commandが`stale_session`で
//! 拒否されたのは、そのsessionがまだ確立されていないという意味であり、衝突ではない。この場合は
//! 現在の`sid`のまま`hello`から再開する（§10.2）。」
//! **このときは終了しない。**同じ`sid`の`hello`を、**新しい`id`で**送り直す。
//!
//! **`reason`は`startup`である。`resync`と`port_reopen`は使えない。**
//! 通常commandが`stale_session`になるのは、`sid`がESP32にとって未知か`retired`なときである。
//! §5.1の整合規則は、`port_reopen`／`resync`なのに`sid`が現在のPi sessionと異なる
//! `hello`を`invalid_payload`で拒否し（§5.1は「`resync`はstateの再取得を要求するものであって、
//! sessionをresetしない」とも書く）、`firmware/esp32/src/protocol.rs`の`handle_hello`も、
//! 未知の`sid`は`startup`だけを遷移の候補にする。`resync`ではsessionを確立できない。
//! `startup`は、§5.1がPiのprocessを起動した直後に送るものとして定める`reason`である。
//! このprocessは実際に起動していて、`sid`もこのprocessのものであり、ESP32が一度も受けていない
//! だけなので、意味は事実と一致する。`sid`を選び直していないので、processが動き続けたまま
//! 新しい`sid`を名乗ることを禁じる§3.1にも当たらない。**ただし§3.1は、`startup`で再開してよい
//! 場合を、processの再起動と運用者の明示的なsession resetに限っている。この再開はそのどちらでも
//! ないため、仕様が明示的に許す形ではない。**
//!
//! **前の`hello`のACKが届かなかったとき**は、仕様どおり同じ`(sid, id)`で再送する
//! （§5.1が、処理済みの`startup`の`hello`を現在の`sid`で再送した場合は手順8で保持ACKを返すと定める。§12のfixture表「同一sessionでの`hello`再送」）。
//! 「**拒否ACKが抑制または喪失した場合**も、送信側は`sid`を選び直さない（§3.1）。現在の`sid`のまま同じ`(sid, id)`で
//! 有限のrecovery budget内で再送する。」に従い、ACK timeoutを待って再送し、`rate_limited`の再送と
//! 同じ予算（[`HELLO_RETRY_LIMIT`]。暫定値で、`PROTO-TBD-011`の確定値ではない）を使い切れば終了する。
//! **新しい`id`の`startup`は、ACKの喪失では送らない。**`hello`が既に処理済みで、ACKだけが失われていた場合に、
//! 「`startup`なのに`sid`が現在のPi sessionと同じ」（§5.1の整合規則）で`invalid_payload`になるためである。
//! `deskcat-serial`の`PeerSession::note_hello_sent`は`hello`を自動再送の対象にしないが、それは
//! `deskcat-serial`の側の取り決めであり、`deskcatd`が自分で`Session::resend`で同じ`(sid, id)`を再送することと
//! 矛盾しない。
//!
//! **`hello`は同時に1つだけである（不変条件）。**新しい`hello`を送ってよいかは`Daemon::hello_gate`だけが決め、
//! 起動時の`hello`も再開の`hello`も`queue_startup_hello`を通る。結果待ちの`hello`、または`rate_limited`の
//! 再送待ちの`hello`があるあいだ（結果が確定するか、再送の予算を使い切るまで）は、再開が要ると
//! 分かっても`resume_wanted`で見送る。その`hello`の結果が、`ok`なら見送りを解き（`status`を取り直す）、
//! `stale_session`なら終了し、再送の予算（`rate_limited`とACKの喪失で共有）を使い切れば終了する。
//! 新しい`id`の`startup`を出してよいのは、結果待ちの`hello`が無い状態で再開が要ったときだけである。
//! `tests/daemon.rs`の`invariant_*`が、再開の要求と`hello`の各結果（`ok`、`rate_limited`、`stale_session`、
//! 無応答）の交差を固定している。
//!
//! **起動直後の`hello`は、他のcommandより先に送る**（§5.1。「Piがprocessを起動した直後、およびserial portを開き直した
//! 直後に、他のcommandより先に一度送信する」。このdaemonは再接続しないので、後者は無い）。**§5.1が求めるのは
//! この2つの場合だけであり、再開の`hello`は先であることを保証しない**（同じ周に積まれた`get_status`などの
//! 再送が先に出ることがある。ESP32が余分に`stale_session`で拒否するだけで、`resume_wanted`が吸収する）。
//! `Pi`が起動する前に`ESP32`が出した`boot`がportに残っていても、
//! `step`は最初に`hello`を積み、その後で`boot`を処理する。`boot`へのACK（commandではない）と、確立に続く
//! `get_status`は`hello`の後に出る。`hello`のACKは`boot`の後に届くが、最初の確立では`PeerSession`が`hello`の
//! 待ちを残す（`deskcat-serial`の`pending_hello`のdoc）ので、相関する。
//! 先に`get_status`を送ると、ESP32が`stale_session`で拒否する往復が毎回起こる。
//! `hello`のACKが続けて届く間に再開を重ねると、ESP32が既に現在のPi sessionとした`sid`への`startup`になり、
//! §5.1の整合規則（`startup`なのに`sid`が現在のPi sessionと同じ）で`invalid_payload`になる。
//!
//! **再開に期限は持たない。**再開の`hello`のACKまでは、同じ`(sid, id)`の再送と[`HELLO_RETRY_LIMIT`]で既に有限である。
//! `ok`のACKの後の`status`の待ちは、§5.6の「Piは、応答の`status`を待つtimeoutを持たない」がそのまま当たる区間
//! であり、ここに例外のtimeoutを持ち込まない。`status`の行だけが失われ続けた場合は、通常の`status`の再要求
//! （下の「応答が無いとき」）が続く。外から見えるようにするのは、`DaemonCounters::status_reasked`と`log`、
//! そして段階2b-iiのwatchdogである。
//! **再開の回数には上限を置く**（[`RESUME_LIMIT`]）。**暫定値であり、根拠は無い**。
//! `PROTO-TBD-011`（「`stale_session`を契機とする選び直し回数の上限」「同一identityの最大retry回数」）の
//! 確定値ではなく、その2つとは別の数である。回数を数える単位は「`status`を受けるまでの連続した再開」である。
//!
//! # 起動直後の`hello`の拒否は届かない
//!
//! Piの起動直後は、ESP32の`sid`が未承認である。`hello`へのACKは、`ok`でも拒否でも
//! `deskcat-serial`が`UnapprovedHelloAck`として受理しない（§6）ので、**拒否のcode（`stale_session`の
//! 衝突を含む）はこのcrateに届かない。**ESP32が`boot`を再送すれば確立する（§10.2手順5）が、
//! `hello`が拒否されていて`boot`も再送されなければ、確立しないまま動き続ける。この経路は
//! 起動からの期限（watchdog）で受け持つ（段階2b-ii）。それまでの残るリスクである。
//!
//! # ESP32が応答しないとき（起動の順は保証されない）
//!
//! **`ESP32`と`Pi`の起動の順は保証されない。**`ESP32`が未起動、起動中、未接続のあいだは、`hello`にACKが無い。
//! `hello`を同じ`(sid, id)`で[`HELLO_RETRY_LIMIT`]回再送し、さらにACK timeoutを待っても無ければ、終了する（75）。
//! **この待ちの長さは、`HELLO_RETRY_LIMIT`とACK timeoutの積（暫定値の組で、約2秒）であり、根拠は無い。**
//! その後はsupervisor（systemdの`Restart=`）が新しい`sid`で起動し直すが、再起動の回数が`StartLimitBurst`に
//! 達すると、ESP32が後から起動しても復帰しない。unit（段階2b-iii）の再起動の間隔と上限、
//! および起動を待つ間隔は、段階2b-iiのwatchdogと段階2b-iiiのunitが受け持つ。この差分では、
//! 起動を待つ長さを決めていない。
//!
//! # 応答が無いとき
//!
//! `get_status`が`stale_session`でも`ok`でもなく、ACK timeoutと再送の予算を使い切って取り下げられた
//! 場合は、`hello`を送り直さない（再開の根拠は`stale_session`であり、無応答ではない。ESP32が既に
//! `hello`を処理していれば`invalid_payload`になる）。`get_status`を新しい`id`で送り直し、
//! 連続した回数が[`STATUS_RETRY_LIMIT`]を超えたら終了する。**この値は暫定であり、根拠は無い。**
//!
//! `ok`のACKの後に`status`が届かない場合は、**待ちにtimeoutを持たない**（§5.6の「Piは、応答の`status`を
//! 待つtimeoutを持たない。待ちは、次に届いた`status`か、ESP32 sessionの遷移で解ける」に従う）。§5.6が
//! 許すのは、応答の`status`を得られないまま実stateが要るときに、新しい`id`で`get_status`を送ることである。
//! このprocessは、欲しい表情と表示を比べる必要が残っているあいだ（`status`を受けるまで）、ACK timeoutと
//! 同じ間隔（根拠は無い）で、新しい`id`の`get_status`を送る。**失敗として数えず、終了もしない。**
//!
//! # 認証の境界
//!
//! **`serial link`に送り手の認証は無い。**`UART`は配線で結んだ1対1の物理linkであり、
//! protocolも認証を持たない。`sid`の一致は**sessionの新しさ**であって、認可ではない。
//! このprocessがeventから起こすのは**表情の変更だけ**であり、サーボを動かす経路は繋いでいない
//! （`play_motion`は送らない。`PROTO-TBD-008`が未確定である）。繰り返し届くeventは、
//! [`deskcat_domain::Mind`]の同種eventの抑制と、`set_expression`を同時に1件しか応答待ちにしない
//! こと（[`crate::expression_sync`]）の2段で抑える。**eventには`(sid, id)`のduplicate判定を
//! 適用しない**（`deskcat-serial`の`PeerSession::accept_event`が、eventの再送とduplicateの扱いを
//! §9で未確定としている）。
//! **eventに特権的な動作（サーボ、電源、設定の変更）を繋ぐ前に、送り手の認証の境界と、
//! 繰り返し届くeventの扱いを決めること。**
//!
//! [#491の決定]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/491#issuecomment-5965360345

use deskcat_domain::{ContactEvent, Millis, Mind, Reaction};
use deskcat_protocol::{
    AckStatus, ErrorCode, Frame, Hello, HelloReason, Message, Outcome, SetExpression,
};
use deskcat_serial::{
    BootOutcome, ConnectionState, CorrelatedAck, OutstandingKind, PeerCounters, PeerRejection,
    PeerSession, Pump, Received, RetryOutcome, SendError, Session, SessionCounters, StopReason,
    Transport, handle_frame, retry_due_requests,
};

use crate::disposition::{Answer, Disposition, Request, disposition};
use crate::expression_sync::{ExpressionSync, SILENT_RETRY_LIMIT, from_status_name, to_wire};

/// 通常commandが`stale_session`で拒否されてから再開してよい回数。**暫定値。**
///
/// 根拠は無い。`PROTO-TBD-011`が未確定であり、同じ規則の別の数でもない（module docの「再開」）。
/// 「有限である」という性質だけを満たす。確定したら値だけを置き換える。
pub const RESUME_LIMIT: u32 = 3;

/// `get_status`が無応答で取り下げられてから、送り直してよい連続の回数。**暫定値。**
///
/// 根拠は無い。「有限である」という性質だけを満たす。`PROTO-TBD-011`の数ではない。
pub const STATUS_RETRY_LIMIT: u32 = 3;

/// `hello`が`rate_limited`で拒否されてから、同じ`(sid, id)`で再送してよい回数。**暫定値。**
///
/// §5.1は有限のbudgetを求め、値を`PROTO-TBD-012`に委ねる。根拠は無い。
pub const HELLO_RETRY_LIMIT: u32 = 3;

/// 終了コード: sessionをやり直すための再起動を求める（`EX_TEMPFAIL`）。
pub const EXIT_SESSION_REDO: u8 = 75;

/// 終了コード: 再起動で直るとは限らない失敗。
pub const EXIT_FATAL: u8 = 1;

/// 終了する理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExitReason {
    /// linkが切れた（EOF、切断のI/O error）。再接続しない。
    LinkLost,
    /// `get_status`が、決まった拒否（`stale_session`と一時的な拒否以外）を受けた。
    GetStatusRejected(Option<ErrorCode>),
    /// 確立済みのESP32 sessionから、別の`sid`の`boot`で遷移した（§10.1）。
    EspRebooted,
    /// Piの`hello`が`stale_session`で拒否された。`sid`の衝突（§3.1）。
    HelloCollided,
    /// 再開の回数が[`RESUME_LIMIT`]を超えた。
    ResumeExhausted,
    /// `hello`の再送（`rate_limited`の拒否、またはACKの喪失）の回数が[`HELLO_RETRY_LIMIT`]を超えた。
    HelloRetryExhausted,
    /// `get_status`が無応答で取り下げられた連続の回数が[`STATUS_RETRY_LIMIT`]を超えた。
    StatusUnanswered,
    /// `hello`が`stale_session`以外の理由で拒否された。
    HelloRejected(Option<ErrorCode>),
    /// `hello`をmessageにできなかった。
    HelloUnsendable,
    /// `Session`が停止した（致命的なI/O error、`id`空間の枯渇）。
    SessionStopped(StopReason),
}

impl ExitReason {
    /// 終了コード。75はsessionをやり直すための再起動、1はそれ以外。
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::LinkLost
            | Self::EspRebooted
            | Self::HelloCollided
            | Self::ResumeExhausted
            | Self::StatusUnanswered
            | Self::HelloRetryExhausted
            | Self::SessionStopped(_) => EXIT_SESSION_REDO,
            Self::HelloRejected(_) | Self::GetStatusRejected(_) | Self::HelloUnsendable => {
                EXIT_FATAL
            }
        }
    }
}

/// [`Daemon::step`]の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "Exitを捨てると、終了すべきprocessが動き続ける"]
#[non_exhaustive]
pub enum Step {
    /// 続ける。
    Continue,
    /// processを終了する。
    Exit(ExitReason),
}

/// 観測用のcounter。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DaemonCounters {
    /// 送った`set_expression`（再送を除く）。
    pub set_expression_sent: u64,
    /// 拒否された`set_expression`。
    pub set_expression_rejected: u64,
    /// 一時的な拒否（`busy`、`rate_limited`、`duplicate_expired`）を受けた`set_expression`。
    pub set_expression_retried_later: u64,
    /// ACK待ちを取り下げた`set_expression`。
    pub set_expression_gave_up: u64,
    /// ACKが無いまま[`SILENT_RETRY_LIMIT`]を超えて、送るのを止めた回数。欲しい表情が変わるまで送らない。
    pub set_expression_stopped_silent: u64,
    /// messageにできなかった`set_expression`。
    pub input_faults: u64,
    /// 反応が首の動きを求めたが、送らなかった回数（`PROTO-TBD-008`）。
    pub motions_not_sent: u64,
    /// 反応を起こさずに数えただけの`environment`。
    pub environment_events: u64,
    /// `stale_session`の後に始めた再開。
    pub resumes: u64,
    /// `status`が届かない（または`get_status`が拒否された）ために、新しい`id`で送り直した回数。
    /// 失敗としては数えず、終了もしない（§5.6）。
    pub status_reasked: u64,
}

/// `hello`が名乗るprocessの識別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Host process identity。
    pub host: String,
    /// Host version／build identity。
    pub version: String,
}

/// 新しい`hello`を送ってよいか（[`Daemon::hello_gate`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelloGate {
    /// 送る予定の`hello`がある。
    Queued,
    /// 結果待ちの`hello`、または`rate_limited`の再送待ちの`hello`がある。
    Busy,
    /// 送ってよい。
    Free,
}

/// linkの一生。
#[derive(Debug)]
pub struct Daemon {
    session: Session,
    peer: PeerSession,
    mind: Mind,
    sync: ExpressionSync,
    identity: Identity,
    counters: DaemonCounters,
    /// まだ送れていない`hello`の`reason`（`startup`だけ）。
    hello_to_send: Option<HelloReason>,
    /// 送ったがまだACKを受けていない`hello`の、送信時刻。
    hello_sent: Option<u64>,
    /// 最後に送った`hello`の`id`とmessage。結果が決まるまで持ち、同じ`(sid, id)`の再送に使う。
    hello_msg: Option<(u32, Message)>,
    /// `rate_limited`で拒否された`hello`を、この時刻に再送する。
    hello_retry_due: Option<u64>,
    hello_retries: u32,
    /// ACK timeout（ms）。送ってあるのにACKを受けていない`hello`を待つ長さ。
    ack_timeout_ms: u64,
    /// `stale_session`を受けたが、先に送った`hello`のACKを待っているため再開を見送った。
    resume_wanted: bool,
    /// 確立したESP32 sessionの数。2回目で終了する。
    esp_sessions_established: u32,
    resume_attempts: u32,
    status_unanswered: u32,
    /// `get_status`の結果を待つ期限。`status`が届かない、または拒否された場合に、送り直す。
    status_watch_until: Option<u64>,
    exit: Option<ExitReason>,
}

impl Daemon {
    /// processの起動ごとに1つ作る。`session`の`sid`は[`crate::sid`]で選んだ値である。
    #[must_use]
    pub fn new(session: Session, peer: PeerSession, mind: Mind, identity: Identity) -> Self {
        let ack_timeout_ms =
            u64::try_from(session.config().retry().ack_timeout().as_millis()).unwrap_or(u64::MAX);
        Self {
            session,
            peer,
            mind,
            sync: ExpressionSync::new(),
            identity,
            counters: DaemonCounters::default(),
            hello_to_send: None,
            hello_sent: None,
            hello_msg: None,
            hello_retry_due: None,
            hello_retries: 0,
            ack_timeout_ms,
            resume_wanted: false,
            esp_sessions_established: 0,
            resume_attempts: 0,
            status_unanswered: 0,
            status_watch_until: None,
            exit: None,
        }
    }

    /// portを開けた。`startup`の`hello`を予約する（§5.1。Piのprocessを起動した直後に送る）。
    ///
    /// **1回だけ呼ぶ。**linkが切れたら再接続せずに終了する（module docの「linkの断は扱わない」）。
    pub fn on_connected(&mut self) {
        self.session.note_connected();
        let queued = self.queue_startup_hello();
        debug_assert!(queued, "起動直後は、結果待ちの`hello`は無い");
    }

    /// `Session`の状態。
    #[must_use]
    pub const fn connection_state(&self) -> ConnectionState {
        self.session.state()
    }

    /// `Session`のcounter。
    #[must_use]
    pub const fn session_counters(&self) -> SessionCounters {
        self.session.counters()
    }

    /// `PeerSession`のcounter。
    #[must_use]
    pub const fn peer_counters(&self) -> PeerCounters {
        self.peer.counters()
    }

    /// このdaemonのcounter。
    #[must_use]
    pub const fn counters(&self) -> DaemonCounters {
        self.counters
    }

    /// 1周進める。`now_ms`はprocess起動からの経過時間（単調。wall-clock timeではない）。
    pub fn step<T: Transport>(&mut self, transport: &mut T, now_ms: u64) -> Step {
        // **`hello`を、他のcommandより先に送る**（§5.1。「Piがprocessを起動した直後、およびserial portを
        // 開き直した直後に、他のcommandより先に一度送信する」）。`boot`が先に届いていても、`boot`への
        // ACK（commandではない）と、`boot`の確立に続く`get_status`は、この後に積まれる。
        self.send_hello(now_ms);
        let mut frames: Vec<Frame> = Vec::new();
        let read = self.session.pump_read(transport, |outcome| {
            // 拒否した行は`Session`が数えている。
            if let Outcome::Frame(frame) = outcome {
                frames.push(frame);
            }
        });
        for frame in frames {
            let received = handle_frame(&mut self.session, &mut self.peer, frame, now_ms);
            self.on_received(received, now_ms);
            if let Some(reason) = self.exit {
                return Step::Exit(reason);
            }
        }
        if let ConnectionState::Stopped(reason) = self.session.state() {
            return Step::Exit(ExitReason::SessionStopped(reason));
        }
        if matches!(read, Pump::Disconnected) {
            log::error!("linkが切れた。再接続せずに再起動を求める");
            return Step::Exit(ExitReason::LinkLost);
        }

        let outcomes = retry_due_requests(&mut self.session, &mut self.peer, now_ms);
        self.on_retry_outcomes(&outcomes);
        if self.resume_wanted {
            self.request_resume();
        }
        self.retry_hello(now_ms);
        if self.status_watch_until.is_some_and(|until| now_ms >= until) {
            // `status`を待つのにtimeoutは持たない（§5.6）。欲しい表情と比べる`status`がまだ要るので、
            // 新しい`id`で`get_status`を送る。失敗としては数えない。
            self.status_watch_until = None;
            if !self.sync.is_ready() {
                self.counters.status_reasked += 1;
                log::warn!(
                    "statusが届かない。新しいidでget_statusを送り直す（{}回目。失敗としては数えない）",
                    self.counters.status_reasked
                );
                self.peer.mark_status_sync_pending();
            }
        }
        let reaction = self.mind.tick(Millis(now_ms));
        self.apply_reaction(reaction);
        self.send_hello(now_ms);
        self.send_expression(now_ms);
        if let Some(reason) = self.exit {
            return Step::Exit(reason);
        }

        let write = self.session.pump_write(transport);
        if let ConnectionState::Stopped(reason) = self.session.state() {
            return Step::Exit(ExitReason::SessionStopped(reason));
        }
        if matches!(write, Pump::Disconnected) {
            log::error!("linkが切れた。再接続せずに再起動を求める");
            return Step::Exit(ExitReason::LinkLost);
        }
        Step::Continue
    }

    fn on_received(&mut self, received: Received, now_ms: u64) {
        match received {
            Received::Boot(handled) => {
                if matches!(handled.outcome, BootOutcome::Established { .. }) {
                    self.esp_sessions_established += 1;
                    if self.esp_sessions_established > 1 {
                        log::warn!(
                            "確立済みのESP32 sessionから別のsidへ遷移した。再起動を求める（§10.1）"
                        );
                        self.exit = Some(ExitReason::EspRebooted);
                    }
                }
            }
            Received::Ack(correlated) => self.on_ack(&correlated, now_ms),
            Received::Status(accepted) => {
                let reported = from_status_name(&accepted.status.display.expression);
                if accepted.solicited {
                    self.resume_attempts = 0;
                    self.status_unanswered = 0;
                    self.status_watch_until = None;
                }
                self.sync.status_reported(reported, accepted.solicited);
            }
            Received::Event(frame) => self.on_event(&frame.message, now_ms),
            // ESP32のsidが未承認のため`hello`の結果としては受理されなかったが、ESP32は
            // 応答した。ACKを待つ理由は無い（起動直後の通常の順序である）。
            Received::Rejected {
                rejection: PeerRejection::UnapprovedHelloAck,
                ..
            } => {
                self.hello_sent = None;
                self.hello_msg = None;
                self.hello_retry_due = None;
            }
            // 拒否と未定義のtypeは、`deskcat-serial`がlogとcounterへ残している。
            _ => {}
        }
    }

    /// 要求への結果を、[`disposition`]（module docの表）に従って処理する。
    fn on_ack(&mut self, correlated: &CorrelatedAck, now_ms: u64) {
        let ack = &correlated.ack;
        let answer = if ack.status == AckStatus::Ok {
            Answer::Ok
        } else {
            Answer::Rejected(ack.code)
        };
        match correlated.request {
            OutstandingKind::Hello => self.on_hello_answer(answer, ack.code, now_ms),
            OutstandingKind::GetStatus => {
                // 応答があった（無応答の連続ではない）。
                self.status_unanswered = 0;
                let cooldown_ends = now_ms.saturating_add(self.ack_timeout_ms);
                match disposition(Request::GetStatus, answer) {
                    // ACKの直後に`status`が届く（§5.6）。届かなければ送り直す。
                    Disposition::Proceed => self.status_watch_until = Some(cooldown_ends),
                    Disposition::Resume => self.request_resume(),
                    // 待てば受け付けられる拒否。理由が解ける時間を待ってから、新しい`id`で送り直す。
                    Disposition::AskAgain => {
                        log::warn!("get_statusが拒否された。送り直す: code={:?}", ack.code);
                        self.status_watch_until = Some(cooldown_ends);
                    }
                    // 決まった拒否。繰り返さずに終了する。
                    _ => {
                        log::error!("get_statusが決まった拒否を受けた: code={:?}", ack.code);
                        self.exit = Some(ExitReason::GetStatusRejected(ack.code));
                    }
                }
            }
            OutstandingKind::SetExpression => {
                match disposition(Request::SetExpression, answer) {
                    Disposition::Proceed => self.sync.acked_ok(ack.reply_to),
                    // sessionが未確立なだけで、表情の内容は拒否されていない。再開の後に送り直す。
                    Disposition::Resume => {
                        self.sync.acked_stale(ack.reply_to);
                        self.request_resume();
                    }
                    Disposition::RetryLater => {
                        self.counters.set_expression_retried_later += 1;
                        let retry_at = now_ms.saturating_add(self.ack_timeout_ms);
                        self.sync.acked_retry_later(ack.reply_to, retry_at);
                    }
                    _ => {
                        self.counters.set_expression_rejected += 1;
                        self.sync.acked_rejected(ack.reply_to);
                    }
                }
            }
            // `ping`は送らない。`OutstandingKind`は拡張されうるため、未知の種別も何もしない。
            _ => {}
        }
    }

    fn on_hello_answer(&mut self, answer: Answer, code: Option<ErrorCode>, now_ms: u64) {
        self.hello_sent = None;
        match disposition(Request::Hello, answer) {
            Disposition::Proceed => {
                // 確立（または維持）を確かめた。現在の表示を取り直す（§10.1手順4、§10.2手順6）。
                self.resume_wanted = false;
                self.hello_msg = None;
                self.hello_retries = 0;
                self.hello_retry_due = None;
                self.sync.not_ready();
                self.peer.mark_status_sync_pending();
            }
            Disposition::RestartCollision => {
                log::error!(
                    "helloがstale_sessionで拒否された。sidの衝突として再起動を求める（§3.1）"
                );
                self.exit = Some(ExitReason::HelloCollided);
            }
            Disposition::RetryHelloSameId => {
                if self.hello_retries >= HELLO_RETRY_LIMIT {
                    log::error!(
                        "helloのrate_limitedによる再送が上限（{HELLO_RETRY_LIMIT}）に達した。再起動を求める"
                    );
                    self.exit = Some(ExitReason::HelloRetryExhausted);
                } else {
                    self.hello_retries += 1;
                    self.hello_retry_due = Some(now_ms.saturating_add(self.ack_timeout_ms));
                }
            }
            // `hello`について、表の他の振る舞いは返らない（testが固定している）。
            _ => {
                log::error!("helloが拒否された: code={code:?}");
                self.exit = Some(ExitReason::HelloRejected(code));
            }
        }
    }

    /// `rate_limited`で拒否された`hello`を、cooldownの後に同じ`(sid, id)`で再送する（§5.1）。
    fn retry_hello(&mut self, now_ms: u64) {
        let Some((id, message)) = self.hello_msg.clone() else {
            return;
        };
        // `rate_limited`の拒否の後はcooldown、ACKが無い場合はACK timeoutを待つ。
        let after_rejection = self.hello_retry_due.is_some_and(|due| now_ms >= due);
        let after_silence = self.hello_retry_due.is_none()
            && self.hello_sent.is_some_and(|sent_at| {
                now_ms.saturating_sub(sent_at) >= self.ack_timeout_ms
                    && disposition(Request::Hello, Answer::NoResponse)
                        == Disposition::RetryHelloSameId
            });
        if !(after_rejection || after_silence) {
            return;
        }
        if after_silence && self.hello_retries >= HELLO_RETRY_LIMIT {
            log::error!(
                "helloのACKが無いまま、再送が上限（{HELLO_RETRY_LIMIT}）に達した。再起動を求める"
            );
            self.exit = Some(ExitReason::HelloRetryExhausted);
            return;
        }
        match self.session.resend(id, message, now_ms) {
            Ok(()) => {
                if after_silence {
                    self.hello_retries += 1;
                }
                self.peer.note_hello_sent(id);
                self.hello_sent = Some(now_ms);
                self.hello_retry_due = None;
                log::info!("helloを同じ(sid, id)で再送した: id={id}");
            }
            Err(SendError::Dropped) => log::debug!("helloを再送できなかった。次の周で再試行する"),
            Err(err) if err.is_input_fault() => {
                log::error!("helloをmessageにできない: {err}");
                self.exit = Some(ExitReason::HelloUnsendable);
            }
            // 停止は`step`が`Session`の状態から見る。
            Err(SendError::Stopped(_) | SendError::IdSpaceExhausted) => {}
            // 分類していない失敗は、毎周再試行せず、致命として扱う（`SendError`は拡張されうる）。
            Err(err) => {
                log::error!("helloを送れない（未分類の失敗）: {err}");
                self.exit = Some(ExitReason::HelloUnsendable);
            }
        }
    }

    fn on_event(&mut self, message: &Message, now_ms: u64) {
        let event = match message {
            Message::HeadTouched(_) => Some(ContactEvent::HeadTouched),
            Message::Tapped => Some(ContactEvent::Tapped),
            Message::Lifted(_) => Some(ContactEvent::Lifted),
            Message::Environment(_) => {
                self.counters.environment_events += 1;
                None
            }
            _ => None,
        };
        if let Some(event) = event {
            let reaction = self.mind.on_event(event, Millis(now_ms));
            self.apply_reaction(reaction);
        }
    }

    fn apply_reaction(&mut self, reaction: Reaction) {
        if let Some(expression) = reaction.expression {
            self.sync.set_desired(expression);
        }
        if reaction.motion.is_some() {
            // motion名は`PROTO-TBD-008`で未確定であり、サーボの安全制限はESP32が強制する。
            self.counters.motions_not_sent += 1;
            log::debug!("首の動きの意図は送らない（PROTO-TBD-008）");
        }
    }

    fn on_retry_outcomes(&mut self, outcomes: &[RetryOutcome]) {
        for outcome in outcomes {
            // 再送と再送の失敗は、`deskcat-serial`がlogへ残している。
            if let RetryOutcome::GaveUp(id, kind) = outcome {
                match kind {
                    // 結果は不明だが、状態設定commandはidempotentである（§9）。新しい`id`で送り直す。
                    OutstandingKind::SetExpression
                        if disposition(Request::SetExpression, Answer::NoResponse)
                            == Disposition::RetryLater =>
                    {
                        self.counters.set_expression_gave_up += 1;
                        if self.sync.gave_up(*id) {
                            // §9: ACKが無ければ結果は不明として扱い、診断を報告する。
                            self.counters.set_expression_stopped_silent += 1;
                            log::error!(
                                "set_expressionにACKが無いまま、上限（{SILENT_RETRY_LIMIT}）を超えた。欲しい表情が変わるまで送らない"
                            );
                        }
                    }
                    // 応答が無いまま取り下げた。**helloは送り直さない**（module docの「応答が無いとき」）。
                    OutstandingKind::GetStatus
                        if disposition(Request::GetStatus, Answer::NoResponse)
                            == Disposition::AskAgain =>
                    {
                        self.status_not_delivered();
                    }
                    _ => {}
                }
            }
        }
    }

    /// `status`を得られなかった。`get_status`を新しい`id`で送り直す。連続が上限を超えたら終了する。
    fn status_not_delivered(&mut self) {
        self.status_unanswered += 1;
        if self.status_unanswered > STATUS_RETRY_LIMIT {
            log::error!(
                "statusを得られない回数が上限（{STATUS_RETRY_LIMIT}）を超えた。再起動を求める"
            );
            self.exit = Some(ExitReason::StatusUnanswered);
        } else {
            self.peer.mark_status_sync_pending();
        }
    }

    /// **`hello`は同時に1つだけである**（不変条件）。新しい`hello`を送ってよいかを、ここだけで決める。
    ///
    /// 起動時の`hello`も再開の`hello`も、[`Self::queue_startup_hello`]を通る。`rate_limited`の
    /// 再送（[`Self::retry_hello`]）は、結果待ちの同じ`hello`の再送であり、新しい`hello`ではない。
    fn hello_gate(&self) -> HelloGate {
        if self.hello_to_send.is_some() {
            HelloGate::Queued
        } else if self.hello_msg.is_some() {
            // 結果が確定していない（ACKを待っている、または再送を待っている）。
            HelloGate::Busy
        } else {
            HelloGate::Free
        }
    }

    /// `startup`の`hello`を予約する。結果待ちの`hello`があれば予約せず、`false`を返す。
    fn queue_startup_hello(&mut self) -> bool {
        if self.hello_gate() != HelloGate::Free {
            return false;
        }
        self.hello_to_send = Some(HelloReason::Startup);
        true
    }

    /// 同じ`sid`の`startup`の`hello`を、新しい`id`で送り直す（module docの「再開」）。
    ///
    /// `hello`が1つ残っているあいだは、`resume_wanted`で見送る。その`hello`の結果が、見送りを解く
    /// （`ok`）か、終了させる（`stale_session`、`rate_limited`の再送を使い切った場合）。
    /// ACKが届かない場合は、`retry_hello`が同じ`(sid, id)`で再送し、予算を使い切れば終了する。
    /// 新しい`id`の`startup`は、結果待ちの`hello`が無い状態になってから出す。
    fn request_resume(&mut self) {
        match self.hello_gate() {
            // 送る予定の`startup`の`hello`が、再開を兼ねる。
            HelloGate::Queued => return,
            HelloGate::Busy => {
                self.resume_wanted = true;
                return;
            }
            HelloGate::Free => {}
        }
        if self.resume_attempts >= RESUME_LIMIT {
            log::error!("再開の回数が上限（{RESUME_LIMIT}）に達した。再起動を求める");
            self.exit = Some(ExitReason::ResumeExhausted);
            return;
        }
        self.resume_attempts += 1;
        self.counters.resumes += 1;
        self.resume_wanted = false;
        self.sync.not_ready();
        let queued = self.queue_startup_hello();
        debug_assert!(queued, "`hello_gate`はFreeである");
        log::warn!(
            "stale_sessionのため、同じsidのhello(startup)を新しいidで送り直す（{}/{RESUME_LIMIT}）",
            self.resume_attempts
        );
    }

    fn send_hello(&mut self, now_ms: u64) {
        let Some(reason) = self.hello_to_send else {
            return;
        };
        let hello = Message::Hello(Hello {
            host: self.identity.host.clone(),
            version: self.identity.version.clone(),
            reason,
        });
        match self.session.send(hello.clone(), now_ms) {
            Ok(id) => {
                self.peer.note_hello_sent(id);
                self.hello_sent = Some(now_ms);
                self.hello_msg = Some((id, hello));
                self.hello_retries = 0;
                self.hello_retry_due = None;
                self.hello_to_send = None;
                log::info!("hello({reason:?})を送った: id={id}");
            }
            Err(SendError::Dropped) => log::debug!("helloを送れなかった。次の周で再試行する"),
            Err(err) if err.is_input_fault() => {
                log::error!("helloをmessageにできない: {err}");
                self.exit = Some(ExitReason::HelloUnsendable);
            }
            // 停止は`step`が`Session`の状態から見る。
            Err(SendError::Stopped(_) | SendError::IdSpaceExhausted) => {}
            // 分類していない失敗は、毎周再試行せず、致命として扱う（`SendError`は拡張されうる）。
            Err(err) => {
                log::error!("helloを送れない（未分類の失敗）: {err}");
                self.exit = Some(ExitReason::HelloUnsendable);
            }
        }
    }

    fn send_expression(&mut self, now_ms: u64) {
        let Some(expression) = self.sync.next_to_send(now_ms) else {
            return;
        };
        let message = Message::SetExpression(SetExpression {
            name: to_wire(expression),
            transition_ms: 0,
        });
        match self.session.send(message.clone(), now_ms) {
            Ok(id) => {
                // `set_expression`は必ず追跡対象に分類される。
                let _ = self.peer.note_sent(id, message, now_ms);
                self.sync.sent(id, expression);
                self.counters.set_expression_sent += 1;
            }
            Err(SendError::Dropped) => {
                log::debug!("set_expressionを送れなかった。次の周で再試行する");
            }
            Err(err) if err.is_input_fault() => {
                log::error!("set_expressionをmessageにできない。送り直さない: {err}");
                self.counters.input_faults += 1;
                self.sync.input_fault();
            }
            // 停止は`step`が`Session`の状態から見る。
            Err(SendError::Stopped(_) | SendError::IdSpaceExhausted) => {}
            // 分類していない失敗は、毎周再試行せず、この表情を送り直さない（`SendError`は拡張されうる）。
            Err(err) => {
                log::error!("set_expressionを送れない（未分類の失敗）。送り直さない: {err}");
                self.counters.input_faults += 1;
                self.sync.input_fault();
            }
        }
    }
}
