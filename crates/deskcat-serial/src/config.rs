//! Serial linkの設定値。
//!
//! **port名とbaudはsecretではなく設定として扱う**（Issue #11の受け入れ条件）。
//! このcrateは値を埋め込まず、呼び出し側から受け取る。
//!
//! `deskcat-config`（型付き設定と検証）は未作成である。将来そちらへ移す前提で、
//! 当面はこのcrate内に置く。移すときも、**既定値を持たないfieldがどれか**という
//! 性質は保つ。

use core::time::Duration;

/// Serial linkのport設定。
///
/// **既定値を持たない。**`Default`を実装していないのは意図的である。
/// device名（`/dev/ttyUSB*`など）を「たぶんこれ」で埋めると、確認していない値が
/// 設定の既定として固定される。実機のdevice名はまだ確認されていない
/// （USB OTG の変換 cable が未入手であり、[Issue #8]の受け入れ条件にも含まれない）。
///
/// baudも同様に呼び出し側が渡す。仕様§2の`115200`は`Candidate`であり、
/// 確定値は`PROTO-TBD-001`である。
///
/// [Issue #8]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialConfig {
    /// Serial deviceのpath。**このcrateはopenしない。**値を保持するだけである。
    port: String,
    /// Baud rate。確定値は`PROTO-TBD-001`。
    baud: u32,
    /// 送信queueの容量（message数）。0は許さない。
    outbox_capacity: usize,
    /// 再接続の方針。
    reconnect: ReconnectPolicy,
}

impl SerialConfig {
    /// port名とbaudを指定して設定を作る。
    ///
    /// `outbox_capacity`と`reconnect`は[`Self::with_outbox_capacity`]と
    /// [`Self::with_reconnect`]で置き換える。既定は[`ReconnectPolicy::provisional`]と
    /// [`Self::DEFAULT_OUTBOX_CAPACITY`]であり、**いずれも暫定値である。**
    ///
    /// # Panics
    ///
    /// `port`が空のときにpanicする。空のpathは設定漏れであり、実行時に
    /// 「開けなかった」として現れるより、構築時に落とすほうが早く気づける。
    #[must_use]
    pub fn new(port: impl Into<String>, baud: u32) -> Self {
        let port = port.into();
        assert!(!port.is_empty(), "port pathが空である");
        Self {
            port,
            baud,
            outbox_capacity: Self::DEFAULT_OUTBOX_CAPACITY,
            reconnect: ReconnectPolicy::provisional(),
        }
    }

    /// 送信queue容量の暫定既定値。
    ///
    /// **正本は`PROTO-TBD-012`（応答の送出上限と保留table）であり、負荷試験待ちである。**
    /// この値は「上限が存在する」という性質を満たすための暫定値であって、
    /// 測定に基づく確定値ではない。
    pub const DEFAULT_OUTBOX_CAPACITY: usize = 32;

    /// 送信queueの容量を差し替える。
    ///
    /// # Panics
    ///
    /// `capacity`が0のときにpanicする。0容量のqueueは送信を常にdropする。
    #[must_use]
    pub fn with_outbox_capacity(mut self, capacity: usize) -> Self {
        assert!(capacity > 0, "outbox容量は1以上である");
        self.outbox_capacity = capacity;
        self
    }

    /// 再接続方針を差し替える。
    #[must_use]
    pub fn with_reconnect(mut self, reconnect: ReconnectPolicy) -> Self {
        self.reconnect = reconnect;
        self
    }

    /// Serial deviceのpath。
    #[must_use]
    pub fn port(&self) -> &str {
        &self.port
    }

    /// Baud rate。
    #[must_use]
    pub const fn baud(&self) -> u32 {
        self.baud
    }

    /// 送信queueの容量。
    #[must_use]
    pub const fn outbox_capacity(&self) -> usize {
        self.outbox_capacity
    }

    /// 再接続方針。
    #[must_use]
    pub const fn reconnect(&self) -> &ReconnectPolicy {
        &self.reconnect
    }
}

/// 再接続の上限とrate limit。
///
/// **受け入れ条件は「Reconnectに上限とrate limitがある」ことであり、
/// 具体的な数値はこのcrateが決めない。**正本は`PROTO-TBD-017`（`boot`再送契約の
/// parameter）と`PROTO-TBD-012`（cooldownと送出上限）であり、
/// **どちらも負荷試験・reconnect試験待ちである。**
///
/// [`Self::provisional`]が返す値は暫定であり、確定値として扱わない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPolicy {
    max_attempts: u32,
    initial_backoff: Duration,
    max_backoff: Duration,
}

impl ReconnectPolicy {
    /// 上限とbackoffを指定して方針を作る。
    ///
    /// # Panics
    ///
    /// `initial_backoff`が`max_backoff`を超えるときにpanicする。
    #[must_use]
    pub fn new(max_attempts: u32, initial_backoff: Duration, max_backoff: Duration) -> Self {
        assert!(
            initial_backoff <= max_backoff,
            "initial_backoffがmax_backoffを超えている"
        );
        Self {
            max_attempts,
            initial_backoff,
            max_backoff,
        }
    }

    /// 暫定の既定値を返す。
    ///
    /// **確定値ではない。**`PROTO-TBD-017`と`PROTO-TBD-012`が決まるまでの仮置きであり、
    /// 「上限が有限である」「試行間隔が単調に伸びる」という性質だけを満たす。
    /// 数値そのものに根拠は無い。
    #[must_use]
    pub fn provisional() -> Self {
        Self::new(5, Duration::from_millis(100), Duration::from_secs(5))
    }

    /// 再接続試行の上限回数。
    #[must_use]
    pub const fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// `attempt`回目（0起点）の試行前に待つ時間。
    ///
    /// `initial_backoff`を2倍ずつ伸ばし、`max_backoff`で頭打ちにする。
    /// これがrate limitである。上限に達した後も一定間隔で試行し続けないよう、
    /// 回数の上限は[`Self::max_attempts`]が別に持つ。
    #[must_use]
    pub fn backoff(&self, attempt: u32) -> Duration {
        let factor = 1_u32.checked_shl(attempt).unwrap_or(u32::MAX);
        self.initial_backoff
            .saturating_mul(factor)
            .min(self.max_backoff)
    }
}
