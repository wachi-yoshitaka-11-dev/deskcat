//! `parse_toml`と`Required`の統合test。

use deskcat_config::{Required, parse_toml};
use serde::Deserialize;

/// testだけに使う、既定値を持たないfieldと持つfieldを混在させた設定。
#[derive(Debug, Deserialize)]
struct SampleConfig {
    /// 既定値を持たない（`Required`でwrapし、`#[serde(default)]`を付けられない）。
    name: Required<String>,
    /// 既定値を持つ。
    #[serde(default = "default_retries")]
    retries: u32,
}

const fn default_retries() -> u32 {
    3
}

#[test]
fn valid_toml_parses_into_expected_type() {
    let config: SampleConfig = parse_toml("name = \"desk\"\nretries = 7\n").expect("valid TOML");

    assert_eq!(config.name.get(), "desk");
    assert_eq!(config.retries, 7);
}

#[test]
fn valid_toml_without_field_with_default_uses_the_default() {
    let config: SampleConfig = parse_toml("name = \"desk\"\n").expect("valid TOML");

    assert_eq!(config.retries, 3);
}

#[test]
fn invalid_toml_syntax_is_rejected() {
    let result: Result<SampleConfig, _> = parse_toml("name = \n");

    assert!(result.is_err());
}

#[test]
fn missing_field_without_default_is_rejected() {
    let result: Result<SampleConfig, _> = parse_toml("retries = 7\n");

    assert!(result.is_err());
}
