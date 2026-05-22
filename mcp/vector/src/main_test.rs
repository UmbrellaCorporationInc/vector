#![allow(clippy::expect_used)]

use std::ffi::OsString;

use mcp_vector::release::version::workspace_version;

use super::{ProcessMode, process_mode};

#[test]
fn version_flag_selects_print_version_mode() {
    let mode = process_mode([OsString::from("--version")]);
    assert!(matches!(mode, ProcessMode::PrintVersion(version) if version == workspace_version()));
}

#[test]
fn non_version_arguments_fall_back_to_mcp_server_mode() {
    let mode = process_mode([OsString::from("--help")]);
    assert!(matches!(mode, ProcessMode::ServeMcp));
}

#[test]
fn empty_arguments_start_the_mcp_server() {
    let mode = process_mode(Vec::<OsString>::new());
    assert!(matches!(mode, ProcessMode::ServeMcp));
}
