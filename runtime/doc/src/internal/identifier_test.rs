#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn parse_unqualified_single_segment_doc_type() {
    let id = parse_doc_identifier("rfc-00013-my-rfc").unwrap();
    assert_eq!(id.package, None);
    assert_eq!(id.doc_type, "rfc");
    assert_eq!(id.code, 13);
    assert_eq!(id.slug, "my-rfc");
}

#[test]
fn parse_unqualified_multi_segment_slug() {
    let id = parse_doc_identifier("task-00054-implement-rfc-00030-vscode").unwrap();
    assert_eq!(id.package, None);
    assert_eq!(id.doc_type, "task");
    assert_eq!(id.code, 54);
    assert_eq!(id.slug, "implement-rfc-00030-vscode");
}

#[test]
fn parse_unqualified_multi_segment_doc_type() {
    let id = parse_doc_identifier("ai-rule-00000-master-dispatcher").unwrap();
    assert_eq!(id.package, None);
    assert_eq!(id.doc_type, "ai-rule");
    assert_eq!(id.code, 0);
    assert_eq!(id.slug, "master-dispatcher");
}

#[test]
fn parse_package_qualified_single_segment_doc_type() {
    let id = parse_doc_identifier("my-pkg/rfc-00013-my-rfc").unwrap();
    assert_eq!(id.package, Some("my-pkg".to_string()));
    assert_eq!(id.doc_type, "rfc");
    assert_eq!(id.code, 13);
    assert_eq!(id.slug, "my-rfc");
}

#[test]
fn parse_package_qualified_multi_segment_doc_type() {
    let id = parse_doc_identifier("shared-lib/ai-rule-00001-some-rule").unwrap();
    assert_eq!(id.package, Some("shared-lib".to_string()));
    assert_eq!(id.doc_type, "ai-rule");
    assert_eq!(id.code, 1);
    assert_eq!(id.slug, "some-rule");
}

#[test]
fn parse_returns_none_for_empty_string() {
    assert!(parse_doc_identifier("").is_none());
}

#[test]
fn parse_returns_none_for_missing_slug() {
    assert!(parse_doc_identifier("rfc-00013").is_none());
}

#[test]
fn parse_returns_none_for_non_numeric_code() {
    assert!(parse_doc_identifier("rfc-invalid-my-rfc").is_none());
}

#[test]
fn parse_returns_none_when_code_is_first_segment() {
    assert!(parse_doc_identifier("00013-rfc-my-rfc").is_none());
}

#[test]
fn parse_returns_none_for_leading_slash() {
    assert!(parse_doc_identifier("/rfc-00013-my-rfc").is_none());
}

#[test]
fn parse_returns_none_for_trailing_slash() {
    assert!(parse_doc_identifier("my-pkg/").is_none());
}

#[test]
fn parse_returns_none_for_only_slash() {
    assert!(parse_doc_identifier("/").is_none());
}

#[test]
fn parse_package_name_with_hyphen() {
    let id = parse_doc_identifier("vector-lib/spec-00001-api").unwrap();
    assert_eq!(id.package, Some("vector-lib".to_string()));
    assert_eq!(id.doc_type, "spec");
    assert_eq!(id.code, 1);
    assert_eq!(id.slug, "api");
}
