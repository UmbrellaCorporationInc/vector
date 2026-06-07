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

#[test]
fn create_project_with_no_name_selects_create_project_mode() {
    let mode = process_mode([OsString::from("create-project")]);
    assert!(matches!(mode, ProcessMode::CreateProject { project_name: None }));
}

#[test]
fn create_project_with_name_selects_create_project_mode_with_name() {
    let mode = process_mode([OsString::from("create-project"), OsString::from("my-project")]);
    assert!(
        matches!(mode, ProcessMode::CreateProject { project_name: Some(ref name) } if name == "my-project")
    );
}
