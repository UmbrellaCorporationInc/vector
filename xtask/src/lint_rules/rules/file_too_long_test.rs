#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use syn::parse_file;

use super::{FileTooLong, count_code_lines};
use crate::lint_rules::rule::{Rule, RuleViolation};

// ─── helpers ────────────────────────────────────────────────────────────────

fn check(path: &str, source: &str, limit: usize) -> Vec<RuleViolation> {
    let rule = FileTooLong { limit, is_future: true };
    let ast = parse_file(source).expect("test source must parse");
    let mut out = Vec::new();
    rule.check_rust(Path::new(path), &ast, source, &mut out);
    out
}

fn rule_ids(violations: &[RuleViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule_id).collect()
}

// ─── is_active gating ───────────────────────────────────────────────────────

#[test]
fn rule_not_active_without_future_flag() {
    let rule = FileTooLong { limit: 300, is_future: false };
    assert!(!rule.is_active(false));
    assert!(!rule.is_active(true));
}

#[test]
fn rule_active_with_future_flag() {
    let rule = FileTooLong { limit: 300, is_future: true };
    assert!(!rule.is_active(false));
    assert!(rule.is_active(true));
}

// ─── count_code_lines ────────────────────────────────────────────────────────

#[test]
fn count_empty_string_is_zero() {
    assert_eq!(count_code_lines(""), 0);
}

#[test]
fn count_blank_lines_excluded() {
    let src = "\n\n\n";
    assert_eq!(count_code_lines(src), 0);
}

#[test]
fn count_line_comments_excluded() {
    let src = "// this is a comment\n/// doc comment\n//! inner doc\n";
    assert_eq!(count_code_lines(src), 0);
}

#[test]
fn count_code_lines_only() {
    let src = "fn foo() {}\nlet x = 1;\n";
    assert_eq!(count_code_lines(src), 2);
}

#[test]
fn count_mixed_lines() {
    let src = "// comment\n\nfn foo() {}\n/// doc\nlet x = 1;\n\n";
    assert_eq!(count_code_lines(src), 2);
}

#[test]
fn count_indented_comment_excluded() {
    let src = "    // indented comment\n    fn foo() {}\n";
    assert_eq!(count_code_lines(src), 1);
}

// ─── RULE-26: file too long ──────────────────────────────────────────────────

#[test]
fn rule_26_fires_when_over_limit() {
    // Build a source with limit+1 code lines.
    let lines: Vec<&str> = vec!["fn placeholder() {}"; 11];
    let source = lines.join("\n");
    let violations = check("src/big.rs", &source, 10);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-26"), "expected RULE-26, got: {ids:?}");
}

#[test]
fn rule_26_does_not_fire_at_limit() {
    let lines: Vec<&str> = vec!["fn placeholder() {}"; 10];
    let source = lines.join("\n");
    let violations = check("src/exact.rs", &source, 10);
    assert!(violations.is_empty(), "RULE-26 must not fire at exactly the limit");
}

#[test]
fn rule_26_does_not_fire_under_limit() {
    let source = "fn foo() {}\n";
    let violations = check("src/short.rs", source, 300);
    assert!(violations.is_empty(), "RULE-26 must not fire for short file");
}

#[test]
fn rule_26_violation_message_contains_counts() {
    let lines: Vec<&str> = vec!["fn placeholder() {}"; 11];
    let source = lines.join("\n");
    let violations = check("src/big.rs", &source, 10);
    assert_eq!(violations.len(), 1);
    let msg = &violations[0].message;
    assert!(msg.contains("11"), "message must contain actual count: {msg}");
    assert!(msg.contains("10"), "message must contain limit: {msg}");
    assert!(msg.contains("RULE-26") || msg.contains("big.rs"), "message must identify file: {msg}");
}

// ─── exemptions ──────────────────────────────────────────────────────────────

#[test]
fn rule_26_exempt_test_files() {
    let lines: Vec<&str> = vec!["fn placeholder() {}"; 301];
    let source = lines.join("\n");
    let violations = check("src/foo_test.rs", &source, 300);
    assert!(violations.is_empty(), "RULE-26 must not fire on _test.rs files");
}

#[test]
fn rule_26_exempt_main_rs() {
    let lines: Vec<&str> = vec!["fn placeholder() {}"; 301];
    let source = lines.join("\n");
    let violations = check("src/main.rs", &source, 300);
    assert!(violations.is_empty(), "RULE-26 must not fire on main.rs");
}

#[test]
fn rule_26_exempt_lib_rs() {
    let lines: Vec<&str> = vec!["pub mod foo;"; 301];
    let source = lines.join("\n");
    let violations = check("src/lib.rs", &source, 300);
    assert!(violations.is_empty(), "RULE-26 must not fire on lib.rs");
}

#[test]
fn rule_26_exempt_mod_rs() {
    let lines: Vec<&str> = vec!["pub mod foo;"; 301];
    let source = lines.join("\n");
    let violations = check("src/subdir/mod.rs", &source, 300);
    assert!(violations.is_empty(), "RULE-26 must not fire on mod.rs");
}
