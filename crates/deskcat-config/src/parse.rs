//! `TOML`文字列のparse。

use crate::error::ConfigError;

/// TOML文字列を型`T`へparseする。
///
/// # Errors
///
/// TOMLの構文が不正、または既定値を持たないfield（[`crate::Required`]で
/// 型付けされたfieldなど）が欠落しているなら[`ConfigError::TomlSyntax`]を返す。
/// **panicしない。**
pub fn parse_toml<T>(text: &str) -> Result<T, ConfigError>
where
    T: serde::de::DeserializeOwned,
{
    toml::from_str(text).map_err(|error| ConfigError::TomlSyntax(error.to_string()))
}
