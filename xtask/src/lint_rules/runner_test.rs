#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn run_returns_empty_on_empty_workspace() {
    let temp_dir = std::env::temp_dir().join("forge_runner_test_empty");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let violations = run(&temp_dir, false);

    // Empty workspace should have no violations
    assert!(violations.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn run_with_future_false() {
    let temp_dir = std::env::temp_dir().join("forge_runner_test_standard");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let violations = run(&temp_dir, false);

    // Type should be Vec<RuleViolation>
    let _: Vec<RuleViolation> = violations;

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn run_with_future_true() {
    let temp_dir = std::env::temp_dir().join("forge_runner_test_future");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let violations = run(&temp_dir, true);

    // Type should be Vec<RuleViolation>
    let _: Vec<RuleViolation> = violations;

    let _ = std::fs::remove_dir_all(&temp_dir);
}
