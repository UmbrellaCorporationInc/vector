#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::internal::slug::validate_slug;

#[test]
fn test_valid_slug() {
    assert!(validate_slug("test-slug").is_ok());
    assert!(validate_slug("abc").is_ok());
    assert!(validate_slug("123").is_ok());
    assert!(validate_slug("a1b2-c3d4").is_ok());
    assert!(validate_slug("rfc-00001-test").is_ok());
}

#[test]
fn test_empty_slug() {
    let result = validate_slug("");
    assert!(result.is_err());
    assert!(result.unwrap_err().reason.contains("cannot be empty"));
}

#[test]
fn test_slug_starts_with_hyphen() {
    let result = validate_slug("-test");
    assert!(result.is_err());
    assert!(result.unwrap_err().reason.contains("must not start or end with a hyphen"));
}

#[test]
fn test_slug_ends_with_hyphen() {
    let result = validate_slug("test-");
    assert!(result.is_err());
    assert!(result.unwrap_err().reason.contains("must not start or end with a hyphen"));
}

#[test]
fn test_slug_consecutive_hyphens() {
    let result = validate_slug("test--slug");
    assert!(result.is_err());
    assert!(result.unwrap_err().reason.contains("must not contain consecutive hyphens"));
}

#[test]
fn test_slug_uppercase_letters() {
    let result = validate_slug("Test");
    assert!(result.is_err());
    assert!(result.unwrap_err().reason.contains("only lowercase"));
}

#[test]
fn test_slug_special_characters() {
    let result = validate_slug("test_slug");
    assert!(result.is_err());
    assert!(result.unwrap_err().reason.contains("only lowercase"));
}

#[test]
fn test_slug_spaces() {
    let result = validate_slug("test slug");
    assert!(result.is_err());
    assert!(result.unwrap_err().reason.contains("only lowercase"));
}
