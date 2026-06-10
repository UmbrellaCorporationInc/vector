#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::NoWorkspaceDependency;
use crate::lint_rules::rule::{Rule, RuleViolation};
use std::path::Path;

fn check(path: &str, content: &str) -> Vec<RuleViolation> {
    let mut out = Vec::new();
    NoWorkspaceDependency.check_toml(Path::new(path), content, &mut out);
    out
}

#[test]
fn rule_15_fires_for_direct_version_in_child_toml() {
    let content = r#"
[package]
name = "child"

[dependencies]
serde = "1.0"
    "#;
    let violations = check("runtime/child/Cargo.toml", content);
    assert!(!violations.is_empty(), "expected violation for direct version");
    assert_eq!(violations[0].rule_id, "RULE-15");
    assert!(violations[0].message.contains("serde"));
}

#[test]
fn rule_15_fires_for_features_without_workspace_in_child_toml() {
    let content = r#"
[dependencies]
tokio = { version = "1.0", features = ["full"] }
    "#;
    let violations = check("runtime/child/Cargo.toml", content);
    assert!(!violations.is_empty(), "expected violation for version key");
}

#[test]
fn rule_15_does_not_fire_for_workspace_true() {
    let content = r#"
[dependencies]
serde = { workspace = true }
clap = { workspace = true, features = ["derive"] }
    "#;
    // Note: The rule currently flags anything that doesn't have `workspace` and `true`.
    // Wait, my implementation checks for `workspace` AND `true`.
    // `clap = { workspace = true, features = ["derive"] }` contains both.
    let violations = check("runtime/child/Cargo.toml", content);
    assert!(
        violations.is_empty(),
        "expected no violations for workspace = true, got: {violations:?}"
    );
}

#[test]
fn rule_15_exempts_root_toml() {
    // Root TOML is identified by containing `[workspace]`
    let content = r#"
[workspace]
members = ["child"]

[workspace.dependencies]
serde = "1.0"
    "#;
    let violations = check("Cargo.toml", content);
    assert!(violations.is_empty(), "root Cargo.toml must be exempt");
}

#[test]
fn rule_15_ignores_non_dependency_sections() {
    let content = r#"
[package]
name = "child"
version = "0.1.0"

[dependencies]
serde = { workspace = true }
    "#;
    let violations = check("runtime/child/Cargo.toml", content);
    assert!(violations.is_empty(), "package version should not be flagged");
}

#[test]
fn rule_15_handles_dev_dependencies() {
    let content = r#"
[dev-dependencies]
tokio = "1.0"
    "#;
    let violations = check("runtime/child/Cargo.toml", content);
    assert!(!violations.is_empty(), "expected violation in dev-dependencies");
}
