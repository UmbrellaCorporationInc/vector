#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::path::Path;

use super::NoInlineTests;
use crate::lint_rules::rule::{Rule, RuleViolation};

// ─── helpers ────────────────────────────────────────────────────────────────

fn check(path: &str, source: &str) -> Vec<RuleViolation> {
    let ast = syn::parse_file(source).expect("test source must parse");
    let mut out = Vec::new();
    NoInlineTests.check_rust(Path::new(path), &ast, source, &mut out);
    out
}

fn rule_ids(violations: &[RuleViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule_id).collect()
}

// ─── RULE-13A: inline cfg(test) mod in non-test file ────────────────────────

#[test]
fn rule_13a_fires_for_inline_cfg_test_mod_in_non_test_file() {
    // mod with an inline body in a logic file — the violation case.
    let source = r#"
        #[cfg(test)]
        mod tests {
            use super::*;
            #[test]
            fn it_works() {}
        }
    "#;
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-13A"), "expected RULE-13A, got: {ids:?}");
}

#[test]
fn rule_13a_does_not_fire_in_test_file() {
    // Inline body in a _test.rs file — allowed.
    let source = r#"
        #[cfg(test)]
        mod tests {
            use super::*;
            #[test]
            fn it_works() {}
        }
    "#;
    let violations = check("src/foo_test.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-13A"), "RULE-13A must not fire in _test.rs, got: {ids:?}");
}

#[test]
fn rule_13a_does_not_fire_for_canonical_path_delegation_in_logic_file() {
    // Canonical form: body-less mod with #[path] in a logic file — NOT a violation.
    let source = r#"
        #[cfg(test)]
        #[path = "foo_test.rs"]
        mod tests;
    "#;
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(
        !ids.contains(&"RULE-13A"),
        "RULE-13A must not fire for path-delegation form, got: {ids:?}"
    );
}

#[test]
fn rule_13a_does_not_fire_when_no_cfg_test() {
    let source = r#"
        mod helpers;
    "#;
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    // RULE-13D fires here (no cfg(test) mod) — but not 13A.
    assert!(!ids.contains(&"RULE-13A"), "RULE-13A must not fire, got: {ids:?}");
}

// ─── RULE-13B: test mod name is not `tests` ─────────────────────────────────

#[test]
fn rule_13b_fires_when_mod_name_is_not_tests() {
    let source = r#"
        #[cfg(test)]
        #[path = "foo_test.rs"]
        mod unit_tests;
    "#;
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-13B"), "expected RULE-13B, got: {ids:?}");
}

#[test]
fn rule_13b_does_not_fire_when_mod_name_is_tests() {
    // Use a test file to isolate 13B from 13A.
    let source = r#"
        #[cfg(test)]
        #[path = "foo_test.rs"]
        mod tests;
    "#;
    let violations = check("src/foo_test.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-13B"), "RULE-13B must not fire for `tests`, got: {ids:?}");
}

// ─── RULE-13C: #[path] value is not `<stem>_test.rs` ────────────────────────

#[test]
fn rule_13c_fires_when_path_attr_does_not_match_stem() {
    let source = r#"
        #[cfg(test)]
        #[path = "tests.rs"]
        mod tests;
    "#;
    // Use _test.rs file to isolate 13C from 13A.
    let violations = check("src/foo_test.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-13C"), "expected RULE-13C, got: {ids:?}");
}

#[test]
fn rule_13c_does_not_fire_when_path_matches_stem() {
    let source = r#"
        #[cfg(test)]
        #[path = "foo_test.rs"]
        mod tests;
    "#;
    let violations = check("src/foo.rs", source);
    // Path-delegation form in a logic file: no 13A (no inline body), no 13C.
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-13C"), "RULE-13C must not fire, got: {ids:?}");
}

#[test]
fn rule_13c_does_not_fire_when_no_path_attr() {
    // Inline mod with a body — only 13A/13B may fire.
    let source = r#"
        #[cfg(test)]
        mod tests {
            use super::*;
        }
    "#;
    let violations = check("src/foo_test.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-13C"), "RULE-13C must not fire when no #[path], got: {ids:?}");
}

// ─── combined: canonical correct usage ──────────────────────────────────────

#[test]
fn canonical_declaration_emits_no_violations() {
    // Canonical form in a logic file: no body, correct name, correct path.
    // None of 13A/13B/13C/13D must fire.
    let source = r#"
        #[cfg(test)]
        #[path = "foo_test.rs"]
        mod tests;
    "#;
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "canonical declaration must produce no violations, got: {ids:?}");
}

#[test]
fn non_test_mod_items_produce_no_violations() {
    let source = r#"
        pub mod handlers;
        pub mod models;
    "#;
    let violations = check("src/lib.rs", source);
    assert!(violations.is_empty(), "non-test mods must produce no violations, got: {violations:?}");
}

// ─── RULE-13D: file has no #[cfg(test)] mod (standard enforcement) ───────────

#[test]
fn rule_13d_fires_when_no_cfg_test_mod() {
    let source = r#"
        pub fn add(a: i32, b: i32) -> i32 { a + b }
    "#;
    let violations = check("src/math.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-13D"), "expected RULE-13D, got: {ids:?}");
}

#[test]
fn rule_13d_does_not_fire_when_cfg_test_mod_present() {
    let source = r#"
        pub fn add(a: i32, b: i32) -> i32 { a + b }
        #[cfg(test)]
        #[path = "math_test.rs"]
        mod tests;
    "#;
    let violations = check("src/math.rs", source);
    let ids = rule_ids(&violations);
    assert!(
        !ids.contains(&"RULE-13D"),
        "RULE-13D must not fire when test mod present, got: {ids:?}"
    );
}

#[test]
fn rule_13d_exempt_main_rs() {
    let source = r#"
        fn main() {}
    "#;
    let violations = check("src/main.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-13D"), "RULE-13D must not fire on main.rs, got: {ids:?}");
}

#[test]
fn rule_13d_exempt_lib_rs() {
    let source = r#"
        pub mod handlers;
    "#;
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-13D"), "RULE-13D must not fire on lib.rs, got: {ids:?}");
}

#[test]
fn rule_13d_exempt_mod_rs() {
    let source = r#"
        pub use handlers::Handler;
    "#;
    let violations = check("src/http/mod.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-13D"), "RULE-13D must not fire on mod.rs, got: {ids:?}");
}

#[test]
fn rule_13d_exempt_test_files() {
    let source = r#"
        #[test]
        fn it_works() {}
    "#;
    let violations = check("src/math_test.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-13D"), "RULE-13D must not fire on _test.rs files, got: {ids:?}");
}

#[test]
fn rule_13d_exempt_integration_test_folder() {
    // Files under a `tests/` directory are Cargo integration tests — no `mod tests` needed.
    let source = r#"
        #[test]
        fn it_works() {}
    "#;
    let violations = check("cli/babel-cli/tests/cli.rs", source);
    let ids = rule_ids(&violations);
    assert!(
        !ids.contains(&"RULE-13D"),
        "RULE-13D must not fire on files under tests/, got: {ids:?}"
    );
}

#[test]
fn rule_13d_does_not_fire_when_inline_cfg_test_mod_present() {
    // Inline body means RULE-13A fires, but RULE-13D must NOT fire because a
    // #[cfg(test)] mod is present (even if inline).
    let source = r#"
        pub fn foo() {}
        #[cfg(test)]
        mod tests {
            use super::*;
            #[test]
            fn it_works() {}
        }
    "#;
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-13D"), "inline cfg(test) mod satisfies 13D, got: {ids:?}");
}
