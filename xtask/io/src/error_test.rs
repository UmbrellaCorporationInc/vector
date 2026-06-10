#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn fs_error_not_found_display() {
    let err = FsError::NotFound("test.txt".to_string());
    assert_eq!(err.to_string(), "Fs Error: not found at 'test.txt'");
}

#[test]
fn fs_error_permission_denied_display() {
    let err = FsError::PermissionDenied("secret.txt".to_string());
    assert_eq!(err.to_string(), "Fs Error: permission denied at 'secret.txt'");
}

#[test]
fn fs_error_io_display() {
    let err = FsError::Io("disk full".to_string());
    assert_eq!(err.to_string(), "Fs Error: disk full");
}

#[test]
fn fs_error_debug() {
    let err = FsError::NotFound("file.rs".to_string());
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("NotFound"));
}
