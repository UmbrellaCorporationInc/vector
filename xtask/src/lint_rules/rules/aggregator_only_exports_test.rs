#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use syn::parse_file;

use super::AggregatorOnlyExports;
use crate::lint_rules::rule::{Rule, RuleViolation};

// ─── helpers ────────────────────────────────────────────────────────────────

fn check(path: &str, source: &str) -> Vec<RuleViolation> {
    let ast = parse_file(source).expect("test source must parse");
    let mut out = Vec::new();
    AggregatorOnlyExports.check_rust(Path::new(path), &ast, source, &mut out);
    out
}

fn rule_ids(violations: &[RuleViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule_id).collect()
}

// ─── is_active (always on) ───────────────────────────────────────────────────

#[test]
fn rule_active_regardless_of_future_flag() {
    assert!(AggregatorOnlyExports.is_active(false));
    assert!(AggregatorOnlyExports.is_active(true));
}

// ─── permitted aggregator content ───────────────────────────────────────────

#[test]
fn lib_rs_with_only_pub_use_and_mod_no_violation() {
    let source = r"
        pub use foo::Bar;
        pub(crate) use baz::Qux;
        use std::collections::HashMap;
        pub mod submodule;
        mod internal;
    ";
    let violations = check("src/lib.rs", source);
    assert!(violations.is_empty(), "expected no violations, got: {violations:?}");
}

#[test]
fn mod_rs_with_only_pub_use_and_mod_no_violation() {
    let source = r"
        pub use error::Error;
        mod error;
    ";
    let violations = check("src/subdir/mod.rs", source);
    assert!(violations.is_empty(), "expected no violations, got: {violations:?}");
}

#[test]
fn extern_crate_is_permitted_in_lib_rs() {
    let source = r"
        extern crate std;
        pub use std::collections::HashMap;
    ";
    let violations = check("src/lib.rs", source);
    assert!(violations.is_empty(), "expected no violations for extern crate, got: {violations:?}");
}

// ─── forbidden items in lib.rs ───────────────────────────────────────────────

#[test]
fn lib_rs_with_fn_fires_rule_27() {
    let source = r"
        pub fn parse() {}
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-27"), "expected RULE-27 for fn in lib.rs, got: {ids:?}");
}

#[test]
fn lib_rs_with_struct_fires_rule_27() {
    let source = r"
        pub struct Config {
            pub timeout: u32,
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-27"), "expected RULE-27 for struct in lib.rs, got: {ids:?}");
}

#[test]
fn lib_rs_with_trait_fires_rule_27() {
    let source = r"
        pub trait Parser {}
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-27"), "expected RULE-27 for trait in lib.rs, got: {ids:?}");
}

#[test]
fn lib_rs_with_impl_block_fires_rule_27() {
    let source = r"
        struct Foo;
        impl Foo {
            pub fn new() -> Self { Foo }
        }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-27"), "expected RULE-27 for impl in lib.rs, got: {ids:?}");
}

#[test]
fn lib_rs_with_enum_fires_rule_27() {
    let source = r"
        pub enum Kind { A, B }
    ";
    let violations = check("src/lib.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-27"), "expected RULE-27 for enum in lib.rs, got: {ids:?}");
}

// ─── forbidden items in mod.rs ───────────────────────────────────────────────

#[test]
fn mod_rs_with_const_fires_rule_27() {
    let source = r"
        pub const MAX: usize = 100;
    ";
    let violations = check("src/subdir/mod.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-27"), "expected RULE-27 for const in mod.rs, got: {ids:?}");
}

// ─── non-aggregator files are not affected ───────────────────────────────────

#[test]
fn non_aggregator_file_with_fn_no_violation() {
    let source = r"
        pub fn compute() -> i32 { 42 }
    ";
    let violations = check("src/compute.rs", source);
    assert!(
        violations.is_empty(),
        "RULE-27 must not fire on non-aggregator file, got: {violations:?}"
    );
}

// ─── violation message format ────────────────────────────────────────────────

#[test]
fn violation_message_contains_line_number_and_item_kind() {
    let source = "pub fn example() {}\n";
    let violations = check("runtime/io/src/lib.rs", source);
    assert_eq!(violations.len(), 1);
    let msg = &violations[0].message;
    assert!(msg.contains("fn"), "message must mention item kind 'fn': {msg}");
    assert!(msg.contains("lib.rs"), "message must mention filename: {msg}");
    assert!(msg.contains("aggregator files"), "message must reference aggregator: {msg}");
}
