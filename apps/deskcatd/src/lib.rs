//! `deskcatd`のlibrary部分。接触eventから表情を決め、ESP32へ送る（Issue #491 段階2b）。
//!
//! wire仕様の正本は`docs/protocol/esp32-pi-protocol.md`、感情の判断は`deskcat-domain`、
//! serial linkと送受信の相関は`deskcat-serial`である。**ここで再実装しない。**
//! このcrateが持つのは、それらをつなぐ呼び出し側（linkの一生）だけである。
//!
//! # 範囲
//!
//! 含むもの:
//!
//! - `sid`の選び方（[`sid`]）
//! - 欲しい表情と、ESP32が出している表情の同期（[`expression_sync`]）
//! - 要求への結果を振る舞いへ写す分類（[`disposition`]）
//! - linkの1周（[`daemon`]）。受信、再送、感情の時間進行、送信、終了の判断
//!
//! 含まないもの（段階2bの後続のPR）:
//!
//! - 実portを開くbinary、logger、引数（2b-ii）
//! - systemdのunit（2b-iii）
//! - 首の動き。`play_motion`は送らない（`PROTO-TBD-008`）
//! - Pi実機での確認（#11の後半）

pub mod daemon;
pub mod disposition;
pub mod expression_sync;
pub mod sid;
