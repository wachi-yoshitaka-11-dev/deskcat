//! 既定値を持たないfieldを型で区別するwrapper。

/// 既定値を持たないことを型で保証するwrapper。
///
/// `T`自体が[`Default`]を実装していても、[`Required<T>`]は実装しない。serdeの
/// `#[serde(default)]`属性は`Default`を要求するため、[`Required<T>`]のfieldへ
/// 誤って付けるとcompile errorになる。「既定値を持たないfieldがどれか、という性質」
/// （crates/deskcat-serial/src/config.rsのdocstring）を、attributeの付け忘れではなく
/// 型で強制する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(transparent)]
pub struct Required<T>(T);

impl<T> Required<T> {
    /// 内側の値を取り出す。
    pub fn into_inner(self) -> T {
        self.0
    }

    /// 内側の値への参照を返す。
    pub const fn get(&self) -> &T {
        &self.0
    }
}

impl<T> core::ops::Deref for Required<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}
