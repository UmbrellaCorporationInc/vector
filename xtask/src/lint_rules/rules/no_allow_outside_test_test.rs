#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::NoAllowOutsideTest;
use crate::lint_rules::rule::{Rule, RuleViolation};
use std::path::Path;
use syn::parse_file;

// ─── helpers ────────────────────────────────────────────────────────────────

fn check(path: &str, source: &str) -> Vec<RuleViolation> {
    let ast = parse_file(source).expect("test source must parse");
    let mut out = Vec::new();
    NoAllowOutsideTest.check_rust(Path::new(path), &ast, source, &mut out);
    out
}

fn rule_ids(violations: &[RuleViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule_id).collect()
}

// ─── RULE-7: No #[allow(unwrap/expect)] outside tests ───────────────────────

#[test]
fn rule_7_fires_for_allow_unwrap_in_production_fn() {
    let source = r"
        #[allow(clippy::unwrap_used)]
        fn foo() {
            let x = Some(1).unwrap();
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-7"), "expected RULE-7, got: {ids:?}");
}

#[test]
fn rule_7_fires_for_allow_expect_in_production_struct() {
    let source = r"
        #[allow(clippy::expect_used)]
        struct Foo;
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-7"), "expected RULE-7, got: {ids:?}");
}

#[test]
fn rule_7_fires_for_allow_unwrap_in_file_level_attribute() {
    let source = r"
        #![allow(clippy::unwrap_used)]
        fn foo() {}
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-7"), "expected RULE-7, got: {ids:?}");
}

#[test]
fn rule_7_does_not_fire_in_test_file() {
    let source = r"
        #[allow(clippy::unwrap_used)]
        fn test_foo() {
            let x = Some(1).unwrap();
        }
    ";
    let violations = check("src/lib_test.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-7 must not fire in *_test.rs file, got: {ids:?}");
}

#[test]
fn rule_7_does_not_fire_in_test_module() {
    let source = r"
        #[cfg(test)]
        mod tests {
            #[allow(clippy::unwrap_used)]
            fn test_foo() {
                let x = Some(1).unwrap();
            }
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-7 must not fire in #[cfg(test)] module, got: {ids:?}");
}

#[test]
fn rule_7_does_not_fire_on_test_fn() {
    let source = r"
        #[test]
        #[allow(clippy::expect_used)]
        fn test_it() {}
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-7 must not fire on #[test] function, got: {ids:?}");
}

#[test]
fn rule_7_handles_comma_separated_allow() {
    let source = r"
        #[allow(dead_code, clippy::unwrap_used)]
        fn foo() {}
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-7"), "expected RULE-7 for comma-separated allow, got: {ids:?}");
}
