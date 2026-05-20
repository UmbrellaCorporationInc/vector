use super::*;

#[test]
fn test_format_code() {
    assert_eq!(format_code(1, 5), "00001");
    assert_eq!(format_code(42, 3), "042");
    assert_eq!(format_code(1234, 2), "1234");
}

#[test]
fn test_parse_code() {
    assert_eq!(parse_code_from_filename("rfc-00001-slug.md", "rfc"), Some(1));
    assert_eq!(parse_code_from_filename("rfc-123-slug.md", "rfc"), Some(123));
    assert_eq!(parse_code_from_filename("task-005-other.md", "task"), Some(5));
}

#[test]
fn test_parse_code_invalid() {
    assert_eq!(parse_code_from_filename("other-00001-slug.md", "rfc"), None);
    assert_eq!(parse_code_from_filename("rfc-abc-slug.md", "rfc"), None);
    assert_eq!(parse_code_from_filename("rfc-001.md", "rfc"), None); // Missing slug part
}

#[test]
fn test_is_governed() {
    assert!(is_governed_file("rfc-00001-slug.md", "rfc"));
    assert!(!is_governed_file("other-00001-slug.md", "rfc"));
}
