//! `Limiter`を毎stepで進め、PWMへの出力をN stepに1回へ間引く時刻の判定（Issue #708、
//! [ADR-0027](../../../docs/decisions/0027-servo-output-decimation-for-ledc-update-latency.md)）。
//!
//! # 何をするか
//!
//! ESP32のLEDCは、`set_duty`が前の更新の効くまで（約40 ms）busy-waitする。制御周期20 msで毎stepに
//! 出力すると`tick`の間隔が許容を外れるため、`Limiter`は毎step進めたまま、出力だけをN stepに1回にする。
//! [`OutputDecimator`]は「このstepで出力するか」だけを答える。**`Limiter`を持たず、値を持たない。**
//! 出力する・しないを決めても、`Limiter`の状態は動かさない。
//!
//! # 出力の位相
//!
//! 数えるのは**`Limiter::step`を実際に進めたstep**だけである（targetが無いとき、基準の`tick`、reject
//! されたstepは数えない）。[`OutputDecimator::new`]の直後の最初の出力は、**ちょうどNstep目**である。
//! 静止から動き始める最初の出力が、動き始めてNstep分の窓を持つ形になる。
//!
//! 窓の途中で動き始めても（最初の出力以後の、静止中の再target）、**測った間隔が公称の周期のとき**は速度・加速度の
//! 上限は破れない。
//!
//! **決定3（ADR-0027）を満たすのは、測った間隔が公称の周期のときだけである。間隔が許容幅の内側で揺れると、出力の列の加速度は、
//! 今回の窓の間隔`T_k`で数えても、窓の中心どうしの間隔（`(T_k + T_(k−1)) / 2`）で数えても、上限を超える位相が残る**
//! （間引かない`Limiter`は`最大加速度 × そのstepの間隔`で制限するので、step自身の間隔で数えれば超えない）。
//! `T_k`だけで割った超過は、総当たりの範囲で、許容の90%（±1.8 ms／±9 ms）まで揺らした3値で約1.07倍／約1.55倍だった
//! （間隔の最大と最小の比以内。**証明ではない**）。参考：窓の中心の間隔で数えると、間引いた出力は、間引かない場合より
//! 悪くならなかった（同じ3値の範囲で）。
//!
//! 公称の周期のときの理由：窓`k`の平均速度を`v_k = Σ(v_i·dt_i) / T_k`とすると、`Limiter`が1stepで変える速度は
//! `最大加速度 × dt`以内なので、連続する2つの窓の平均速度の差は`最大加速度 × T_k`以内に収まる。
//! `tests/decimation.rs`が、Nと動き始めの位相と再targetの位相を総当たりして確かめている。

/// 出力する時刻の判定。
///
/// **N（`interval`）は呼び出し側が渡す。**このcrateは値を決めない
/// （firmwareは`config.rs`の設計定数を渡す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputDecimator {
    interval: u32,
    /// 最後の出力から進めたstep数。`interval`に達したら出力して0へ戻す。
    steps_since_output: u32,
}

impl OutputDecimator {
    /// `interval`stepに1回出力する判定を作る。**0は拒否する**（毎step出力か、一度も出力しないかの
    /// どちらでもない値を、黙って選ばない）。
    #[must_use]
    pub const fn new(interval: u32) -> Option<Self> {
        if interval == 0 {
            None
        } else {
            Some(Self {
                interval,
                steps_since_output: 0,
            })
        }
    }

    /// 出力するstepの間隔（stepの数）。
    #[must_use]
    pub const fn interval(self) -> u32 {
        self.interval
    }

    /// `Limiter::step`を1回進めた後に呼ぶ。**このstepで出力するなら`true`**を返し、数え直す。
    pub fn advance(&mut self) -> bool {
        self.steps_since_output = self.steps_since_output.saturating_add(1);
        if self.steps_since_output >= self.interval {
            self.steps_since_output = 0;
            true
        } else {
            false
        }
    }

    /// 最後の出力から進めたstep数（log・test用）。
    #[must_use]
    pub const fn steps_since_output(self) -> u32 {
        self.steps_since_output
    }
}
