//! 接触eventから表情と首の動きの意図を決め、時間でneutralへ戻す（Issue #491）。
//!
//! # 範囲
//!
//! 含むもの:
//!
//! - 感情の状態（[`Expression`]）と、接触event（[`ContactEvent`]）による遷移
//! - 判断の出力（[`Reaction`]）。表情の名前と、首の動きの**意図**（[`MotionIntent`]）
//! - 時間の進行でneutralへ戻ること（[`Mind::tick`]）
//! - 同じ種類のeventが短い間隔で続いたときの抑制と、その計数（[`Counters`]）
//!
//! 含まないもの:
//!
//! - serial、file、network、GPIO、GUI。**時計も読まない。**時刻は呼び出し側が
//!   [`Millis`]で渡す。同じ入力列から必ず同じ出力列が出る
//! - wire形式（`deskcat-protocol`）への変換。`apps/deskcatd`が持つ
//! - 首の角度、速度、回数。motion名は`PROTO-TBD-008`で未確定であり、サーボの安全制限は
//!   設計上ESP32が強制する分担である（AGENTS.mdのプロジェクト境界）
//! - 表情の見た目（#21のコメントの決定）
//! - protocolの`(sid, id)`による重複の判定。`deskcat-serial`の責務である。
//!   ここで抑えるのは、別の`id`で短い間隔に届いた同じ種類のeventである
//!
//! # 遷移
//!
//! | 入力 | 表情 | 首の意図 |
//! |---|---|---|
//! | [`ContactEvent::HeadTouched`] | [`Expression::Happy`] | [`MotionIntent::Nod`] |
//! | [`ContactEvent::Tapped`] | [`Expression::Surprised`] | なし |
//! | [`ContactEvent::Lifted`] | [`Expression::Surprised`] | なし |
//! | 保つ時間（[`Timing`]）の経過 | [`Expression::Neutral`] | なし |
//!
//! 受け付けたeventは今の表情を上書きし、保つ時間を測り直す。
//!
//! `Surprised`に首の意図を付けないのは、首を動かさなければ自分の振動でtapを拾い直す
//! 経路が生まれないためである（#22の受け入れ条件。#22はmotionを任意としている）。
//!
//! # 残っている相互作用（段階1では抑えない）
//!
//! **[`MotionIntent::Nod`]の振動を加速度がtapとして拾うと、[`ContactEvent::Tapped`]が
//! `Happy`を`Surprised`で上書きしうる。**このcrateはmotionがいつ終わるかを知らない
//! （Protocol §4.7の完了eventは未導入）。どこで抑えるか（ESP32のclassifierか、
//! 完了eventを使う`deskcatd`か）は#22と#491の段階2で決める。

mod mind;

pub use mind::{
    ContactEvent, Counters, EventCounts, Expression, Millis, Mind, MotionIntent, Reaction, Timing,
};
