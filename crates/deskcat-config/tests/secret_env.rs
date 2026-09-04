//! `SecretEnv`の統合test。
//!
//! `std::env::set_var`はRust 2024でunsafeになり、このworkspaceは
//! `unsafe_code = "forbid"`のためtestからも使えない。そのため「値がある」場合の
//! testは、processに既に存在するvar（`PATH`）で代替する。

use deskcat_config::{ConfigError, SecretEnv};

#[test]
fn missing_secret_env_var_is_rejected() {
    let var_name = "DESKCAT_CONFIG_TEST_MISSING_SECRET";
    // 事故防止: 万一この名前が環境に存在していたら先に検出する。
    assert!(
        std::env::var(var_name).is_err(),
        "test env varが既に設定されている"
    );

    let result = SecretEnv::required(var_name);

    assert_eq!(
        result,
        Err(ConfigError::MissingSecretEnvVar(var_name.to_string()))
    );
}

#[test]
fn present_secret_env_var_is_returned() {
    let expected = std::env::var("PATH").expect("test processはPATHを持つ");

    let result = SecretEnv::required("PATH");

    assert_eq!(result, Ok(expected));
}
