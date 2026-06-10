#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::*;
use crate::{DiskDir, MemoryDir};

#[test]
fn file_path_returns_none_for_memory_backend() {
    let dir = MemoryDir::new();
    let file = dir.get_file("virtual.txt").unwrap();
    file.write_text("x").unwrap();
    assert!(file.path().is_none());
}

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

#[test]
fn list_files_memory_and_disk_return_equivalent_paths() {
    let mem = MemoryDir::new();
    mem.get_file("x/y/z.txt").unwrap().write_text("z").unwrap();
    mem.get_file("x/a.txt").unwrap().write_text("a").unwrap();
    mem.get_file("b.txt").unwrap().write_text("b").unwrap();

    let root = std::env::temp_dir().join("forge_fs_roundtrip_list");
    let disk = DiskDir::new(&root);
    disk.create_dir("x/y").unwrap();
    disk.get_file("x/y/z.txt").unwrap().write_text("z").unwrap();
    disk.get_file("x/a.txt").unwrap().write_text("a").unwrap();
    disk.get_file("b.txt").unwrap().write_text("b").unwrap();

    let mut mem_paths: Vec<String> = mem.list_files().unwrap().map(|r| r.unwrap()).collect();
    let mut disk_paths: Vec<String> = disk.list_files().unwrap().map(|r| r.unwrap()).collect();
    mem_paths.sort();
    disk_paths.sort();

    assert_eq!(mem_paths, disk_paths);
    let _ = std::fs::remove_dir_all(&root);
}
