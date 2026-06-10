#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::NoAllowAnywhere;
use crate::lint_rules::rule::{Rule, RuleViolation};
use std::path::Path;
use syn::parse_file;

// ─── helpers ────────────────────────────────────────────────────────────────

fn check(path: &str, source: &str, future: bool) -> Vec<RuleViolation> {
    let ast = parse_file(source).expect("test source must parse");
    let mut out = Vec::new();
    NoAllowAnywhere { is_future: future }.check_rust(Path::new(path), &ast, source, &mut out);
    out
}

fn rule_ids(violations: &[RuleViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule_id).collect()
}

// ─── is_active ───────────────────────────────────────────────────────────────

#[test]
fn rule_f1_always_active() {
    // Rule always runs to check for standard unauthorized allows.
    assert!(NoAllowAnywhere { is_future: false }.is_active(false));
    assert!(NoAllowAnywhere { is_future: true }.is_active(true));
}

// ─── Non-test code: any #[allow(...)] is flagged ─────────────────────────────

#[test]
fn rule_f1_fires_for_any_allow_in_production_fn() {
    let source = r"
        #[allow(dead_code)]
        fn foo() {}
    ";
    let violations = check("src/lib.rs", source, true);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-F1"), "expected RULE-F1, got: {ids:?}");
}

#[test]
fn rule_f1_fires_for_permitted_allow_in_production_code() {
    // Even "permitted" args are forbidden outside test contexts.
    let source = r"
        #[allow(clippy::unwrap_used)]
        fn foo() {}
    ";
    let violations = check("src/lib.rs", source, true);
    let ids = rule_ids(&violations);
    assert!(
        ids.contains(&"RULE-F1"),
        "expected RULE-F1 even for unwrap_used in prod, got: {ids:?}"
    );
}

#[test]
fn rule_f1_fires_for_file_level_allow_in_production_code() {
    let source = r"
        #![allow(unused_imports)]
        fn foo() {}
    ";
    let violations = check("src/lib.rs", source, true);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-F1"), "expected RULE-F1 for file-level allow, got: {ids:?}");
}

#[test]
fn rule_f1_fires_for_allow_on_struct_in_production_code() {
    let source = r"
        #[allow(dead_code)]
        struct Foo;
    ";
    let violations = check("src/lib.rs", source, true);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-F1"), "expected RULE-F1 on struct, got: {ids:?}");
}

// ─── Test files: only non-permitted args are flagged ─────────────────────────

#[test]
fn rule_f1_does_not_fire_for_permitted_allow_in_test_file() {
    let source = r"
        #[allow(clippy::unwrap_used)]
        fn test_foo() {}
        #[allow(clippy::expect_used)]
        fn test_bar() {}
        #[allow(clippy::print_stdout)]
        fn test_baz() {}
    ";
    let violations = check("src/lib_test.rs", source, true);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "permitted allows in *_test.rs must not fire, got: {ids:?}");
}

#[test]
fn rule_f1_fires_for_non_permitted_allow_in_test_file() {
    let source = r"
        #[allow(dead_code)]
        fn test_foo() {}
    ";
    let violations = check("src/lib_test.rs", source, true);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-F1"), "non-permitted allow in *_test.rs must fire, got: {ids:?}");
}

#[test]
fn rule_f1_fires_only_for_non_permitted_in_comma_list_in_test_file() {
    // unwrap_used is permitted; dead_code is not — only dead_code should be flagged.
    let source = r"
        #[allow(clippy::unwrap_used, dead_code)]
        fn test_foo() {}
    ";
    let violations = check("src/lib_test.rs", source, true);
    assert_eq!(violations.len(), 1, "only dead_code should be flagged, got: {violations:?}");
    assert!(violations[0].message.contains("dead_code"));
}

#[test]
fn rule_f1_as_conversions_and_indexing_slicing_permitted_in_test_files() {
    // clippy::as_conversions and clippy::indexing_slicing are in PERMITTED_IN_TESTS —
    // they are always permitted in *_test.rs files regardless of --future mode.
    let source = r"
        #[allow(clippy::as_conversions)]
        fn test_as() {}
        #[allow(clippy::indexing_slicing)]
        fn test_index() {}
    ";

    let violations_std = check("src/lib_test.rs", source, false);
    assert!(violations_std.is_empty(), "must be permitted in test files (future=false)");

    let violations_future = check("src/lib_test.rs", source, true);
    assert!(violations_future.is_empty(), "must be permitted in test files (future=true)");
}

// ─── cfg(test) module: only non-permitted args are flagged ───────────────────

#[test]
fn rule_f1_does_not_fire_for_permitted_allow_in_cfg_test_module() {
    let source = r"
        #[cfg(test)]
        mod tests {
            #[allow(clippy::unwrap_used)]
            fn helper() {}
        }
    ";
    let violations = check("src/lib.rs", source, true);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "permitted allow in #[cfg(test)] mod must not fire, got: {ids:?}");
}

#[test]
fn rule_f1_fires_for_non_permitted_allow_in_cfg_test_module() {
    let source = r"
        #[cfg(test)]
        mod tests {
            #[allow(unused_variables)]
            fn helper() {}
        }
    ";
    let violations = check("src/lib.rs", source, true);
    let ids = rule_ids(&violations);
    assert!(
        ids.contains(&"RULE-F1"),
        "non-permitted allow inside #[cfg(test)] must fire, got: {ids:?}"
    );
}

#[test]
fn rule_f1_does_not_fire_when_no_allow_attrs_present() {
    let source = r"
        fn foo() -> u32 { 42 }
    ";
    let violations = check("src/lib.rs", source, true);
    assert!(
        violations.is_empty(),
        "no allow attrs should produce no violations, got: {violations:?}"
    );
}

// ─── pest grammar files: only missing_docs allows are permitted ──────────────

#[test]
fn rule_f1_does_not_fire_for_permitted_allows_in_pest_grammar_file() {
    // missing_docs and missing_docs_in_private_items are the only permitted
    // allows in a pest-derive parser module.
    let source = r#"
        #![allow(missing_docs, clippy::missing_docs_in_private_items)]

        #[derive(Parser)]
        #[grammar = "grammar/babel.pest"]
        pub struct BabelParser;
    "#;
    let violations = check("src/grammar/parser.rs", source, true);
    assert!(violations.is_empty(), "permitted pest allows must not fire, got: {violations:?}");
}

#[test]
fn rule_f1_fires_for_non_permitted_allow_in_pest_grammar_file() {
    // An allow beyond the permitted set must still be flagged even in pest files.
    let source = r#"
        #![allow(dead_code)]

        #[derive(Parser)]
        #[grammar = "grammar/babel.pest"]
        pub struct BabelParser;
    "#;
    let violations = check("src/grammar/parser.rs", source, true);
    assert!(!violations.is_empty(), "non-permitted allow in pest file must fire");
    assert!(violations[0].message.contains("dead_code"));
}

#[test]
fn rule_f1_fires_normally_without_grammar_attr() {
    // Same #[allow] but no #[grammar = "..."] — treated as regular production code.
    let source = r"
        #![allow(missing_docs)]

        pub struct NotAPestParser;
    ";
    let violations = check("src/grammar/parser.rs", source, true);
    assert!(!violations.is_empty(), "without #[grammar], allow must still fire");
}

#[test]
fn rule_f1_agnostic_permitted_list() {
    let source = r"
        #[allow(clippy::as_conversions)]
        fn foo() {}
    ";

    // Even in non-test (production) code, PERMITTED list is allowed if future=false
    let violations = check("src/lib.rs", source, false);
    assert!(violations.is_empty(), "PERMITTED lints must be allowed everywhere when future=false");

    // But strictly forbidden when future=true
    let violations_future = check("src/lib.rs", source, true);
    assert!(
        !violations_future.is_empty(),
        "PERMITTED lints must be violations everywhere when future=true"
    );
}
