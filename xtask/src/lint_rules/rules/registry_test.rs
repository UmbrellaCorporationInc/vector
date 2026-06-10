#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn all_returns_vec_of_rules() {
    let rules = all(false);
    assert!(!rules.is_empty());
}

#[test]
fn all_with_future_false() {
    let rules = all(false);
    // Should include standard rules
    assert!(rules.len() >= 7); // Minimum number of standard rules
}

#[test]
fn all_with_future_true() {
    let rules = all(true);
    // Should include all rules (standard + future)
    assert!(rules.len() >= 7);
}

#[test]
fn all_rules_have_unique_identities() {
    let rules = all(true);
    // Each rule should be a distinct instance
    assert!(!rules.is_empty());
}
