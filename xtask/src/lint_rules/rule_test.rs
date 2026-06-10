#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn test_rule_violation_creation() {
    let violation = RuleViolation {
        file: PathBuf::from("test.rs"),
        line: Some(42),
        column: Some(10),
        rule_id: "RULE-13A",
        message: "Test violation".to_string(),
    };

    assert_eq!(violation.file, PathBuf::from("test.rs"));
    assert_eq!(violation.line, Some(42));
    assert_eq!(violation.column, Some(10));
    assert_eq!(violation.rule_id, "RULE-13A");
    assert_eq!(violation.message, "Test violation");
}

#[test]
fn test_rule_violation_no_location() {
    let violation = RuleViolation {
        file: PathBuf::from("test.rs"),
        line: None,
        column: None,
        rule_id: "RULE-99",
        message: "No location".to_string(),
    };

    assert!(violation.line.is_none());
    assert!(violation.column.is_none());
}

#[test]
fn test_rule_violation_debug() {
    let violation = RuleViolation {
        file: PathBuf::from("test.rs"),
        line: Some(1),
        column: Some(1),
        rule_id: "TEST",
        message: "Debug test".to_string(),
    };

    let debug_str = format!("{violation:?}");
    assert!(debug_str.contains("test.rs"));
    assert!(debug_str.contains("TEST"));
}

struct MockRule;

impl Rule for MockRule {
    fn is_active(&self, future: bool) -> bool {
        future
    }
}

#[test]
fn test_mock_rule_is_active_with_future() {
    let rule = MockRule;
    assert!(rule.is_active(true));
    assert!(!rule.is_active(false));
}

#[test]
fn test_mock_rule_check_rust_default() {
    let rule = MockRule;
    let path = std::path::Path::new("test.rs");
    let ast = syn::parse_str::<syn::File>("fn main() {}").unwrap();
    let mut violations = Vec::new();

    // Default implementation should do nothing
    rule.check_rust(path, &ast, "fn main() {}", &mut violations);
    assert!(violations.is_empty());
}

#[test]
fn test_mock_rule_check_toml_default() {
    let rule = MockRule;
    let path = std::path::Path::new("Cargo.toml");
    let mut violations = Vec::new();

    // Default implementation should do nothing
    rule.check_toml(path, "[package]\nname = \"test\"", &mut violations);
    assert!(violations.is_empty());
}
