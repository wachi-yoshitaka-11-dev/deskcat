//! Piからの受信が途切れたことの判定と、途切れたときに`stop`を呼ぶ関数
//! （[Issue #20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20)）。
//!
//! **値を持たない。**途切れたと判定するまでの時間は呼び出し側が渡す（製品buildの値は`firmware/esp32`の`LINK_LOSS_TIMEOUT_MS`。HW-TBD-017）。
//! `None`の間は、途切れたと判定しない。復帰後の再開はこのmoduleの範囲外である（HW-TBD-018）。

use crate::Drive;

/// [`LinkWatch::check`]の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkVerdict {
    /// 見張っていない（時間が決まっていない、またはまだ1度も受信していない）。
    Idle,
    /// 受信が続いている。
    Alive,
    /// 受信が途切れている。`first`は、この途切れで最初の判定のときだけ`true`。
    Lost {
        /// この途切れで最初の判定か。
        first: bool,
    },
}

/// 受信の途切れの見張り。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkWatch {
    timeout_ms: Option<u32>,
    last_rx_ms: Option<u64>,
    lost: bool,
    losses: u32,
}

impl LinkWatch {
    /// `timeout_ms`が`None`または0の間は、見張らない。
    #[must_use]
    pub const fn new(timeout_ms: Option<u32>) -> Self {
        Self {
            timeout_ms: match timeout_ms {
                Some(0) => None,
                other => other,
            },
            last_rx_ms: None,
            lost: false,
            losses: 0,
        }
    }

    /// 見張る時間が決まっているか。
    #[must_use]
    pub const fn is_armed(&self) -> bool {
        self.timeout_ms.is_some()
    }

    /// Piから受信した時刻を記録する。途切れの状態は解く（stopした出力は戻さない）。
    pub fn note_rx(&mut self, now_ms: u64) {
        self.last_rx_ms = Some(self.last_rx_ms.map_or(now_ms, |last| last.max(now_ms)));
        self.lost = false;
    }

    /// 受信を記録して見張りを始めたか（1度でも[`Self::note_rx`]を呼んだか）。
    #[must_use]
    pub const fn has_started(&self) -> bool {
        self.last_rx_ms.is_some()
    }

    /// **見張りを始めてよいとき（`may_start`）か、既に始めているときだけ**、受信を記録する。
    ///
    /// 見張りの開始を、受信とは別の条件（firmwareでは`boot`のACKによるsessionの確立）に合わせるためである。
    /// 始めた後は`may_start`によらず記録する（[`Self::note_rx`]と同じ）。
    pub fn note_rx_gated(&mut self, now_ms: u64, may_start: bool) {
        if may_start || self.has_started() {
            self.note_rx(now_ms);
        }
    }

    /// 見張りを始めていて、最後の受信から時間が過ぎていないか。**読むだけで、途切れの判定（[`Self::check`]）と
    /// 累計を変えない。**表示（firmwareの`LED-COMM`）のためである。時間が決まっていなければ`false`。
    #[must_use]
    pub fn is_alive(&self, now_ms: u64) -> bool {
        match (self.timeout_ms, self.last_rx_ms) {
            (Some(timeout), Some(last)) => now_ms.saturating_sub(last) < u64::from(timeout),
            _ => false,
        }
    }

    /// [`Self::is_alive`]が`false`へ変わる時刻。見張りを始めていなければ`None`。
    #[must_use]
    pub fn alive_until_ms(&self) -> Option<u64> {
        Some(self.last_rx_ms?.saturating_add(u64::from(self.timeout_ms?)))
    }

    /// 途切れたと判定する時刻（単調に増える時刻、ms）。見張っていなければ`None`。
    #[must_use]
    pub fn deadline_ms(&self) -> Option<u64> {
        if self.lost {
            return None;
        }
        Some(self.last_rx_ms?.saturating_add(u64::from(self.timeout_ms?)))
    }

    /// 途切れの判定を返す。
    pub fn check(&mut self, now_ms: u64) -> LinkVerdict {
        let (Some(timeout), Some(last)) = (self.timeout_ms, self.last_rx_ms) else {
            return LinkVerdict::Idle;
        };
        if now_ms.saturating_sub(last) < u64::from(timeout) {
            return LinkVerdict::Alive;
        }
        let first = !self.lost;
        if first {
            self.lost = true;
            self.losses = self.losses.saturating_add(1);
        }
        LinkVerdict::Lost { first }
    }

    /// 途切れと判定した累計回数。
    #[must_use]
    pub const fn losses(&self) -> u32 {
        self.losses
    }
}

/// `watch`が途切れを最初に判定したときだけ`drive.stop()`を呼び、その結果を返す。
/// **`stop`の失敗は握りつぶさず返す**（試し直しは呼び出し側の`StopLatch`）。
pub fn fail_safe_on_loss<D: Drive>(
    watch: &mut LinkWatch,
    drive: &mut D,
    now_ms: u64,
) -> Option<Result<(), D::StopError>> {
    match watch.check(now_ms) {
        LinkVerdict::Lost { first: true } => Some(drive.stop()),
        _ => None,
    }
}

/// [`RxGapStats`]の分布の区間の数。
pub const RX_GAP_BINS: usize = 10;

/// [`RxGapStats`]の分布の区間の上端（ms、この値以下）。最後の区間は、これを超えたもの。
///
/// **計測の解像度であり、安全値ではない。**途切れの判定時間（`HW-TBD-017`）の候補でも、
/// 判定の閾値でもない。分布の形が分かるように2の冪で刻んだだけである。間隔の最大は区間に丸めず、
/// [`RxGapStats::max_ms`]がそのまま持つ。
pub const RX_GAP_BIN_UPPER_MS: [u64; RX_GAP_BINS - 1] =
    [16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

/// Piからのframeの到着の間隔の統計（Issue #20、`HW-TBD-017`の実測用）。
///
/// **観測だけを持つ。**[`LinkWatch`]へも`Drive`へも触れず、途切れの判定とサーボ出力を変えない。
/// 値の量は固定（区間は[`RX_GAP_BINS`]個）で、記録のたびに増えるものは無い。
/// 時刻は、[`LinkWatch::note_rx`]へ渡すのと同じ単調な時刻（ms）を渡す。
/// 同じ呼び出しで受けたframeは同じ時刻になるので、間隔は0になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RxGapStats {
    last_rx_ms: Option<u64>,
    gaps: u32,
    max_ms: u64,
    max_at_ms: u64,
    window_max_ms: u64,
    sum_ms: u64,
    bins: [u32; RX_GAP_BINS],
}

impl RxGapStats {
    /// 空の統計で作る。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_rx_ms: None,
            gaps: 0,
            max_ms: 0,
            max_at_ms: 0,
            window_max_ms: 0,
            sum_ms: 0,
            bins: [0; RX_GAP_BINS],
        }
    }

    /// 現在のPi sessionのframeを受けた時刻を記録する。直前のframeがあれば、その間隔を足す。
    /// 時刻が戻った場合は0として数える（`LinkWatch::note_rx`が`max`を取るのと同じ向き）。
    pub fn note_rx(&mut self, now_ms: u64) {
        if let Some(last) = self.last_rx_ms {
            let gap = now_ms.saturating_sub(last);
            self.gaps = self.gaps.saturating_add(1);
            self.sum_ms = self.sum_ms.saturating_add(gap);
            // 同じ最大なら、先に起きたほうを残す。
            if gap > self.max_ms {
                self.max_ms = gap;
                self.max_at_ms = now_ms;
            }
            self.window_max_ms = self.window_max_ms.max(gap);
            let bin = RX_GAP_BIN_UPPER_MS
                .iter()
                .position(|upper| gap <= *upper)
                .unwrap_or(RX_GAP_BINS - 1);
            self.bins[bin] = self.bins[bin].saturating_add(1);
        }
        self.last_rx_ms = Some(self.last_rx_ms.map_or(now_ms, |last| last.max(now_ms)));
    }

    /// Pi sessionが替わったとき（`hello`で新しいsessionを確立したとき）に呼ぶ。
    /// **次のframeを基準にし、sessionをまたぐ間隔は数えない。**累計は消さない。
    pub fn restart_session(&mut self) {
        self.last_rx_ms = None;
    }

    /// 間隔を数えた回数（frame数から基準の分を引いたもの）。
    #[must_use]
    pub const fn gaps(&self) -> u32 {
        self.gaps
    }

    /// 起動から今までの間隔の最大（ms）。
    #[must_use]
    pub const fn max_ms(&self) -> u64 {
        self.max_ms
    }

    /// 最大の間隔を数えた時刻（ms。その間隔の終わりのframeを受けた時刻）。
    #[must_use]
    pub const fn max_at_ms(&self) -> u64 {
        self.max_at_ms
    }

    /// 間隔の合計（ms）。平均は`sum_ms / gaps`。
    #[must_use]
    pub const fn sum_ms(&self) -> u64 {
        self.sum_ms
    }

    /// 区間ごとの件数。上端は[`RX_GAP_BIN_UPPER_MS`]、最後は超えたもの。
    #[must_use]
    pub const fn bins(&self) -> [u32; RX_GAP_BINS] {
        self.bins
    }

    /// 前回の`take_window_max_ms`から今までの間隔の最大（ms）を返し、0へ戻す。
    /// log 1行ごとの最大を出すために使う。累計の最大（[`Self::max_ms`]）は戻さない。
    pub fn take_window_max_ms(&mut self) -> u64 {
        core::mem::take(&mut self.window_max_ms)
    }
}
