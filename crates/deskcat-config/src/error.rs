//! 設定値の検証エラー。

/// 設定の読み込みまたは検証が失敗したこと。
///
/// **panicにしない。**設定はTOML fileや環境変数といった、呼び出し側の外から渡る値である。
/// `assert!`やpanicで落とすとprocessごと終わるため、分類して返し、初期化側がlogと
/// counterへ落とせるようにする（AGENTS.mdの「エラーを握りつぶさず、分類、ログ、
/// カウンタを用意する」。crates/deskcat-serial/src/config.rsの`ConfigError`と同じ形）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigError {
    /// TOMLの構文が不正、または既定値を持たないfieldが欠落している。
    ///
    /// serdeの`missing field`errorもここに含む。既定値を持たないfieldは
    /// `#[serde(default)]`を持たないため、欠落は構文検証と同じ経路でerrorになる。
    TomlSyntax(String),
    /// 値が許容範囲外である。
    OutOfRange {
        /// 渡された値の`Debug`表現。
        value: String,
        /// 許容範囲の下限の`Debug`表現。
        min: String,
        /// 許容範囲の上限の`Debug`表現。
        max: String,
    },
    /// 秘密情報用の環境変数が未設定である。
    MissingSecretEnvVar(String),
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TomlSyntax(message) => {
                write!(f, "TOMLの構文または必須fieldが不正である: {message}")
            }
            Self::OutOfRange { value, min, max } => {
                write!(f, "値{value}が許容範囲[{min}, {max}]の外である")
            }
            Self::MissingSecretEnvVar(var_name) => {
                write!(f, "秘密情報用の環境変数{var_name}が未設定である")
            }
        }
    }
}

// `crates/deskcat-serial/src/config.rs`のdocstringが、このcrateの契約元だと
// 明記している。同fileの`ConfigError`は`core::error::Error`を実装しており、
// ここも契約元へ揃えた。**repository全体がこちらへ統一されているわけではない**
// （`core::`／`std::`は crate ごとに割れている）。
impl core::error::Error for ConfigError {}
