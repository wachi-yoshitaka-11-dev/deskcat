//! `Bounded`の統合test。

use deskcat_config::{Bounded, ConfigError};

#[test]
fn value_within_range_is_accepted() {
    let bounded = Bounded::new(5, 0, 10).expect("5 is within [0, 10]");

    assert_eq!(*bounded.get(), 5);
}

#[test]
fn value_above_max_is_rejected() {
    let result = Bounded::new(11, 0, 10);

    assert!(matches!(result, Err(ConfigError::OutOfRange { .. })));
}

#[test]
fn value_below_min_is_rejected() {
    let result = Bounded::new(-1, 0, 10);

    assert!(matches!(result, Err(ConfigError::OutOfRange { .. })));
}
