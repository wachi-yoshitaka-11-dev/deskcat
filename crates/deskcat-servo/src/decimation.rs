//! PWMへの出力をN tickに1回へ間引く時刻の判定（Issue #708、
//! [ADR-0027](../../../docs/decisions/0027-servo-output-decimation-for-ledc-update-latency.md)）。
//!
//! # 何をするか
//!
//! ESP32のLEDCは、`set_duty`が前の更新の効くまで（約40 ms）busy-waitする。制御周期20 msで毎tickに
//! 出力すると`tick`の間隔が許容を外れるため、出力をN tickに1回にする。[`OutputDecimator`]は
//! 「このtickで出力するか」だけを答える。**`Limiter`を持たず、値を持たない。**
//!
//! # `Limiter`は出力するtickでだけ進める
//!
//! 呼び出し側（firmwareの`LimitedServo::tick`）は、出力すると答えたtickでだけ`Limiter::step`を呼び、
//! `dt`に**最後の出力から今回の出力までに測った時間（窓）**を渡す。出力の列が、`Limiter`が測った間隔で制限した
//! 列そのものになるので、窓の間隔が許容の内側で揺れても、速度と加速度の上限は`Limiter::step`の保証のとおり、
//! 窓の間隔`T_k`で数えて超えない（ADR-0027の決定3）。`Limiter`の`ControlPeriod`は、各tickの`ControlPeriod`を
//! N倍した窓の許容である（各tickの間隔が許容の内側なら、N個の和は、f32の丸めの境界ぎりぎりを除き窓の許容の内側。外れたときは`Limiter::step`がrejectしてlatchする（安全側））。各tickの間隔の検査は
//! 呼び出し側が毎tick行う。
//!
//! 以前は`Limiter`を毎tick進めて出力だけを間引いていたが、揺れる間隔で窓の加速度が上限を超える位相が残った
//! （`tests/decimation.rs`の以前の版で、`T_k`で数えて許容の90%まで揺らして約1.07〜1.55倍）。
//!
//! # 出力の位相
//!
//! 数えるのは**間隔の検査を通したtick**だけである（targetが無いとき、基準のtickは数えない）。
//! [`OutputDecimator::new`]の直後の最初の出力は、**ちょうどN回目**である。

/// 出力する時刻の判定。
///
/// **N（`interval`）は呼び出し側が渡す。**このcrateは値を決めない
/// （firmwareは`config.rs`の設計定数を渡す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputDecimator {
    interval: u32,
    /// 最後の出力から数えたtick数。`interval`に達したら出力して0へ戻す。
    ticks_since_output: u32,
}

impl OutputDecimator {
    /// `interval`tickに1回出力する判定を作る。**0は拒否する**（毎tick出力か、一度も出力しないかの
    /// どちらでもない値を、黙って選ばない）。
    #[must_use]
    pub const fn new(interval: u32) -> Option<Self> {
        if interval == 0 {
            None
        } else {
            Some(Self {
                interval,
                ticks_since_output: 0,
            })
        }
    }

    /// 出力するtickの間隔（tickの数）。
    #[must_use]
    pub const fn interval(self) -> u32 {
        self.interval
    }

    /// 間隔の検査を通したtickごとに1回呼ぶ。**このtickで出力するなら`true`**を返し、数え直す。
    /// `true`のtickでだけ`Limiter::step`を進め、出力する。
    pub fn advance(&mut self) -> bool {
        self.ticks_since_output = self.ticks_since_output.saturating_add(1);
        if self.ticks_since_output >= self.interval {
            self.ticks_since_output = 0;
            true
        } else {
            false
        }
    }

    /// 最後の出力から数えたtick数（log・test用）。
    #[must_use]
    pub const fn ticks_since_output(self) -> u32 {
        self.ticks_since_output
    }
}
