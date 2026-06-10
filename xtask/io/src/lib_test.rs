#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::*;

#[test]
fn fs_error_not_found_displays_path() {
    let err = FsError::NotFound("a/b/c.txt".to_string());
    assert_eq!(err.to_string(), "Fs Error: not found at 'a/b/c.txt'");
}

#[test]
fn fs_error_permission_denied_displays_path() {
    let err = FsError::PermissionDenied("/etc/shadow".to_string());
    assert_eq!(err.to_string(), "Fs Error: permission denied at '/etc/shadow'");
}

#[test]
fn fs_error_io_displays_message() {
    let err = FsError::Io("store lock poisoned".to_string());
    assert_eq!(err.to_string(), "Fs Error: store lock poisoned");
}

#[test]
fn fs_error_implements_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(FsError::Io("test".to_string()));
    assert!(!err.to_string().is_empty());
}
