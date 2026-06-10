#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::NoPubStructFields;
use crate::lint_rules::rule::{Rule, RuleViolation};
use std::path::Path;
use syn::parse_file;

// ─── helpers ────────────────────────────────────────────────────────────────

fn check(path: &str, source: &str) -> Vec<RuleViolation> {
    let ast = parse_file(source).expect("test source must parse");
    let mut out = Vec::new();
    NoPubStructFields.check_rust(Path::new(path), &ast, source, &mut out);
    out
}

fn rule_ids(violations: &[RuleViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule_id).collect()
}

// ─── RULE-10: No pub struct fields ──────────────────────────────────────────

#[test]
fn rule_10_fires_for_pub_field() {
    let source = r"
        struct Foo {
            pub x: i32,
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-10"), "expected RULE-10, got: {ids:?}");
}

#[test]
fn rule_10_fires_for_pub_tuple_field() {
    let source = r"
        struct Foo(pub i32);
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-10"), "expected RULE-10, got: {ids:?}");
}

#[test]
fn rule_10_does_not_fire_for_private_field() {
    let source = r"
        struct Foo {
            x: i32,
            pub(crate) y: i32,
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-10 must not fire for non-pub, got: {ids:?}");
}

#[test]
fn rule_10_does_not_fire_for_dto_struct() {
    let source = r"
        /// A sample data object.
        ///
        /// # DTO(data transfer)
        struct Foo {
            pub x: i32,
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-10 must not fire for DTO with explanation, got: {ids:?}");
}

#[test]
fn rule_10_fires_for_dto_without_explanation() {
    let source = r"
        /// A sample data object.
        ///
        /// # DTO
        struct Foo {
            pub x: i32,
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-10"), "RULE-10 must fire for DTO without explanation");
}

#[test]
fn rule_10_fires_for_dto_with_empty_explanation() {
    let source = r"
        /// A sample data object.
        ///
        /// # DTO()
        struct Foo {
            pub x: i32,
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-10"), "RULE-10 must fire for DTO with empty explanation");
}

#[test]
fn rule_10_fires_for_dto_with_whitespace_explanation() {
    let source = r"
        /// A sample data object.
        ///
        /// # DTO(   )
        struct Foo {
            pub x: i32,
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-10"), "RULE-10 must fire for DTO with whitespace explanation");
}

#[test]
fn rule_10_does_not_fire_in_test_file() {
    let source = r"
        struct Foo {
            pub x: i32,
        }
    ";
    let violations = check("src/lib_test.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-10 must not fire in *_test.rs, got: {ids:?}");
}

#[test]
fn rule_10_does_not_fire_in_tests_dir() {
    let source = r"
        struct Foo {
            pub x: i32,
        }
    ";
    let violations = check("cli/foo/tests/integration.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-10 must not fire in tests/ dir, got: {ids:?}");
}
