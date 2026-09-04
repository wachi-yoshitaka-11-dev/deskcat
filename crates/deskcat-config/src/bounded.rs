//! 強制安全上限の検証。

use crate::error::ConfigError;

/// `[min, max]`の範囲内であることを検証済みの値。
///
/// README（crates/deskcat-config/README.md）の「強制安全上限を外れる値の拒否」の実装。
/// [`Self::new`]を通った値だけが存在でき、範囲内であることが型で保証される。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounded<T> {
    value: T,
    min: T,
    max: T,
}

impl<T> Bounded<T>
where
    T: PartialOrd + core::fmt::Debug,
{
    /// `value`が`[min, max]`の範囲内であることを検証して構築する。
    ///
    /// # Errors
    ///
    /// `value`が`min`未満または`max`超過なら[`ConfigError::OutOfRange`]を返す。
    /// **panicしない。**呼び出し側から渡る値をprocessの終了で扱わない。
    pub fn new(value: T, min: T, max: T) -> Result<Self, ConfigError> {
        if value < min || value > max {
            return Err(ConfigError::OutOfRange {
                value: format!("{value:?}"),
                min: format!("{min:?}"),
                max: format!("{max:?}"),
            });
        }
        Ok(Self { value, min, max })
    }

    /// 検証済みの値。
    pub const fn get(&self) -> &T {
        &self.value
    }

    /// 許容範囲の下限。
    pub const fn min(&self) -> &T {
        &self.min
    }

    /// 許容範囲の上限。
    pub const fn max(&self) -> &T {
        &self.max
    }
}
