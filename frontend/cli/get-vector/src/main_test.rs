#![allow(clippy::expect_used)]

use super::*;

#[test]
fn test_parse_args_help() {
    assert_eq!(parse_args(&["get-vector".to_string()]), CliAction::Missing);
    assert_eq!(parse_args(&["get-vector".to_string(), "--help".to_string()]), CliAction::Help);
    assert_eq!(parse_args(&["get-vector".to_string(), "-h".to_string()]), CliAction::Help);
}

#[test]
fn test_parse_args_version() {
    assert_eq!(
        parse_args(&["get-vector".to_string(), "--version".to_string()]),
        CliAction::Version
    );
    assert_eq!(parse_args(&["get-vector".to_string(), "-V".to_string()]), CliAction::Version);
}

#[test]
fn test_parse_args_update() {
    assert_eq!(
        parse_args(&["get-vector".to_string(), "update-mcp-vector".to_string()]),
        CliAction::Update
    );
}

#[test]
fn test_parse_args_unknown() {
    assert_eq!(
        parse_args(&["get-vector".to_string(), "unknown-cmd".to_string()]),
        CliAction::Unknown("unknown-cmd".to_string())
    );
}
