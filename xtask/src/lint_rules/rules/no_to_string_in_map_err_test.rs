#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::path::Path;

use super::NoToStringInMapErr;
use crate::lint_rules::rule::{Rule, RuleViolation};

// ─── helpers ────────────────────────────────────────────────────────────────

fn check(path: &str, source: &str) -> Vec<RuleViolation> {
    let ast = syn::parse_file(source).expect("test source must parse");
    let mut out = Vec::new();
    NoToStringInMapErr.check_rust(Path::new(path), &ast, source, &mut out);
    out
}

fn rule_ids(violations: &[RuleViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule_id).collect()
}

// ─── RULE-12: No .to_string() in .map_err() ─────────────────────────────────

#[test]
fn rule_12_fires_for_simple_map_err_to_string() {
    let source = r"
        fn foo() {
            let res = some_result().map_err(|e| e.to_string());
        }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-12"), "expected RULE-12, got: {ids:?}");
}

#[test]
fn rule_12_fires_for_map_err_to_string_in_block() {
    let source = r"
        fn foo() {
            let res = some_result().map_err(|e| {
                e.to_string()
            });
        }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-12"), "expected RULE-12 (block), got: {ids:?}");
}

#[test]
fn rule_12_does_not_fire_for_unrelated_map_err() {
    let source = r"
        fn foo() {
            let res = some_result().map_err(|e| MyError::new(e));
            let res2 = some_result().map_err(MyError::from);
        }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-12 must not fire for unrelated map_err, got: {ids:?}");
}

#[test]
fn rule_12_does_not_fire_for_to_string_outside_map_err() {
    let source = r"
        fn foo() {
            let s = some_val.to_string();
            let res = some_result().map(|v| v.to_string());
        }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-12 must not fire for .to_string() outside map_err, got: {ids:?}");
}
