//! 型付き設定のparseと検証。
//!
//! 責務はREADME（crates/deskcat-config/README.md）が定める。TOMLのparse、値の検証、
//! 秘密情報を含まない安全なdefault、秘密情報への明示的な環境変数override、
//! 強制安全上限を外れる値の拒否。
//!
//! # 具体的な設定schemaの移設はこの作業に含まない
//!
//! `crates/deskcat-serial/src/config.rs`のdocstringに移設の前提がある。
//! `SerialConfig`／`ConfigError`／`ReconnectPolicy`が、このcrateが受け持つ形の実例だが、
//! 移設自体は別作業である。ここにあるのは、「既定値を持たないfieldがどれか、という
//! 性質」を型で保つための、schemaに依存しない基盤である（[`Required`]、[`Bounded`]、
//! [`SecretEnv`]、[`parse_toml`]）。
//!
//! # 「秘密情報を含まないdefault」は一部だけ型で強制している
//!
//! [`SecretEnv`]は`Deserialize`を実装していない。したがって秘密情報を`TOML`の
//! fieldとしては書けず、`#[serde(default)]`に置くこともできない。**「秘密情報が
//! defaultへ入る」経路のうち、`TOML`経由のものは型で塞いである。**
//!
//! ただし、これは「defaultに置いた値が秘密情報でないこと」の全体を保証しない。
//! [`Required`]は「既定値を持たない」ことを型で強制する（逆方向の性質）。一方、
//! `#[serde(default)]`に置いた*通常の*値（`SecretEnv`を経由しない、`String`や
//! 数値のfield）の中身が秘密情報かどうかは、値そのものを見なければ判定できず、
//! 構文的な型の性質にできない。この部分はこのcrateの型やlintでは強制しない。
//!
//! 契約は次の運用ルールとして持つ。**秘密情報は`SecretEnv`経由でのみ受け取り、
//! 通常のdeserializable fieldとしては持たせない。**`#[serde(default)]`に置く値は、
//! `crates/deskcat-serial/src/config.rs`の`ReconnectPolicy::provisional`のように、
//! 非秘密の定数だけにする。この規約自体を守る責務はreviewにある。

mod bounded;
mod error;
mod parse;
mod required;
mod secret;

pub use bounded::Bounded;
pub use error::ConfigError;
pub use parse::parse_toml;
pub use required::Required;
pub use secret::SecretEnv;
