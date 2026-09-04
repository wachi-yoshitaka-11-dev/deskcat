//! 秘密情報への明示的な環境変数override。

use crate::error::ConfigError;

/// 秘密情報を、TOMLではなく環境変数から明示的に読む。
///
/// README（crates/deskcat-config/README.md）の「秘密情報に対する明示的な環境変数
/// override」の実装。秘密情報はTOML fileへ書けないため、version管理下のfileへ
/// 秘密情報が入る経路をそもそも作らない。
///
/// **意図的に`Deserialize`を実装しない。**実装すると、この型を含む構造体を
/// `TOML`からparseした時点で秘密情報が`TOML`のfieldとして書けてしまい、
/// 「環境変数経由でのみ受け取る」という契約が崩れる。
pub struct SecretEnv;

impl SecretEnv {
    /// 環境変数`var_name`の値を読む。
    ///
    /// # Errors
    ///
    /// `var_name`が未設定、または有効なUnicodeでないなら
    /// [`ConfigError::MissingSecretEnvVar`]を返す。**panicしない。**
    pub fn required(var_name: &str) -> Result<String, ConfigError> {
        std::env::var(var_name).map_err(|_| ConfigError::MissingSecretEnvVar(var_name.to_string()))
    }
}
