#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::*;

#[test]
fn nested_dir_creation() {
    let root = std::env::temp_dir().join("forge_fs_disk_nested");
    let dir = DiskDir::new(&root);
    dir.create_dir("a/b/c").unwrap();
    assert!(root.join("a/b/c").is_dir());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn write_read_roundtrip() {
    let root = std::env::temp_dir().join("forge_fs_disk_rw");
    let dir = DiskDir::new(&root);
    dir.create_dir(".").unwrap();
    let f = dir.get_file("data.bin").unwrap();
    let bytes: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF];
    f.write_bytes(&bytes).unwrap();
    assert_eq!(f.read_bytes().unwrap(), bytes);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn disk_file_path_returns_some_with_absolute_path() {
    let root = std::env::temp_dir().join("forge_fs_disk_path_test");
    let dir = DiskDir::new(&root);
    dir.create_dir("sub").unwrap();
    let f = dir.get_file("sub/file.txt").unwrap();
    f.write_text("data").unwrap();
    let path = f.path().expect("DiskFile must return Some(path)");
    assert!(path.ends_with("sub/file.txt") || path.ends_with("sub\\file.txt"));
    assert!(path.is_absolute());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn read_reader_streams_file_contents() {
    let root = std::env::temp_dir().join("forge_fs_disk_reader");
    let dir = DiskDir::new(&root);
    dir.create_dir(".").unwrap();
    let f = dir.get_file("stream.txt").unwrap();
    f.write_text("streamed").unwrap();

    let mut reader = f.read_reader().unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut reader, &mut content).unwrap();

    assert_eq!(content, "streamed");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn read_missing_file_returns_not_found() {
    let root = std::env::temp_dir().join("forge_fs_disk_missing");
    let dir = DiskDir::new(&root);
    let result = dir.get_file("non_existent.txt").unwrap().read_text();
    assert!(matches!(result, Err(FsError::NotFound(_))));
}

#[test]
fn idempotent_create_dir() {
    let root = std::env::temp_dir().join("forge_fs_disk_idempotent");
    let dir = DiskDir::new(&root);
    assert!(dir.create_dir("repeated").is_ok());
    assert!(dir.create_dir("repeated").is_ok());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn file_reads_content_via_dir() {
    let root = std::env::temp_dir().join("forge_fs_disk_at_test");
    let dir = DiskDir::new(&root);
    dir.create_dir(".").unwrap();
    dir.get_file("target.txt").unwrap().write_bytes(b"absolute").unwrap();
    let content = dir.get_file("target.txt").unwrap().read_text().unwrap();
    assert_eq!(content, "absolute");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[allow(clippy::permissions_set_readonly_false)]
fn write_to_readonly_file_returns_permission_denied() {
    let root = std::env::temp_dir().join("forge_fs_disk_readonly_test");
    let dir = DiskDir::new(&root);
    dir.create_dir(".").unwrap();
    dir.get_file("locked.txt").unwrap().write_bytes(b"initial").unwrap();

    let path = root.join("locked.txt");
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms.clone()).unwrap();

    let result = dir.get_file("locked.txt").unwrap().write_bytes(b"override");

    perms.set_readonly(false);
    std::fs::set_permissions(&path, perms).ok();
    let _ = std::fs::remove_dir_all(&root);

    assert!(matches!(result, Err(FsError::PermissionDenied(_))));
}

#[test]
fn exists_returns_true_after_write_and_false_before() {
    let root = std::env::temp_dir().join("forge_fs_disk_exists");
    let dir = DiskDir::new(&root);
    dir.create_dir(".").unwrap();
    let f = dir.get_file("check.txt").unwrap();
    assert!(!f.exists());
    f.write_text("data").unwrap();
    assert!(f.exists());
    f.delete().unwrap();
    assert!(!f.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn read_bytes_on_missing_file_returns_not_found() {
    let root = std::env::temp_dir().join("forge_fs_disk_read_bytes_missing");
    let dir = DiskDir::new(&root);
    let result = dir.get_file("does_not_exist.bin").unwrap().read_bytes();
    assert!(matches!(result, Err(FsError::NotFound(_))));
}

#[test]
fn delete_missing_file_returns_not_found() {
    let root = std::env::temp_dir().join("forge_fs_disk_delete_missing");
    let dir = DiskDir::new(&root);
    let result = dir.get_file("ghost.txt").unwrap().delete();
    assert!(matches!(result, Err(FsError::NotFound(_))));
}

#[test]
fn create_dir_blocked_by_existing_file_returns_error() {
    let root = std::env::temp_dir().join("forge_fs_disk_createdir_blocked");
    let _ = std::fs::create_dir_all(&root);
    // Place a regular file where we will try to create a directory
    std::fs::write(root.join("blocker"), b"file").unwrap();
    // "blocker" is a file — "blocker/sub" cannot be created
    let sub = DiskDir::new(root.join("blocker"));
    let result = sub.create_dir("sub");
    let _ = std::fs::remove_dir_all(&root);
    assert!(result.is_err());
}

#[test]
fn list_files_returns_all_files_recursively() {
    let root = std::env::temp_dir().join("forge_fs_disk_list_files");
    let dir = DiskDir::new(&root);
    dir.create_dir("a/b").unwrap();
    dir.get_file("a/b/deep.txt").unwrap().write_text("deep").unwrap();
    dir.get_file("a/top.txt").unwrap().write_text("top").unwrap();
    dir.get_file("root.txt").unwrap().write_text("root").unwrap();

    let mut paths: Vec<String> = dir.list_files().unwrap().map(|r| r.unwrap()).collect();
    paths.sort();

    assert_eq!(paths, vec!["a/b/deep.txt", "a/top.txt", "root.txt"]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn list_files_excludes_directories() {
    let root = std::env::temp_dir().join("forge_fs_disk_list_dirs");
    let dir = DiskDir::new(&root);
    dir.create_dir("empty_subdir").unwrap();
    dir.get_file("file.txt").unwrap().write_text("data").unwrap();

    let paths: Vec<String> = dir.list_files().unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(paths, vec!["file.txt"]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn list_files_normalizes_separators_to_forward_slash() {
    let root = std::env::temp_dir().join("forge_fs_disk_list_sep");
    let dir = DiskDir::new(&root);
    dir.create_dir("sub/nested").unwrap();
    dir.get_file("sub/nested/file.txt").unwrap().write_text("data").unwrap();

    let paths: Vec<String> = dir.list_files().unwrap().map(|r| r.unwrap()).collect();
    assert!(paths.iter().all(|p| !p.contains('\\')), "paths must use '/' separator");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn subpath_resolution_traverses_nested_dirs() {
    let root = std::env::temp_dir().join("forge_fs_disk_subpath");
    let dir = DiskDir::new(&root);
    dir.create_dir("a/b").unwrap();
    dir.get_file("a/b/nested.txt").unwrap().write_text("deep").unwrap();
    assert_eq!(dir.get_file("a/b/nested.txt").unwrap().read_text().unwrap(), "deep");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn last_modified_returns_nonzero_after_write() {
    let root = std::env::temp_dir().join("forge_fs_disk_last_modified");
    let dir = DiskDir::new(&root);
    dir.create_dir(".").unwrap();
    let f = dir.get_file("ts.txt").unwrap();
    f.write_text("stamp").unwrap();
    let ts = f.last_modified().unwrap();
    assert!(ts > 0, "expected non-zero Unix timestamp after write, got {ts}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn last_modified_returns_not_found_for_missing_file() {
    let root = std::env::temp_dir().join("forge_fs_disk_last_modified_missing");
    let dir = DiskDir::new(&root);
    let result = dir.get_file("ghost.txt").unwrap().last_modified();
    assert!(matches!(result, Err(FsError::NotFound(_))));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn write_writer_round_trip() {
    let root = std::env::temp_dir().join("forge_fs_disk_write_writer_rt");
    let dir = DiskDir::new(&root);
    dir.create_dir(".").unwrap();
    let f = dir.get_file("streamed.txt").unwrap();

    let mut w = f.write_writer().unwrap();
    std::io::Write::write_all(&mut w, b"streamed content").unwrap();
    std::io::Write::flush(&mut w).unwrap();
    drop(w);

    assert_eq!(f.read_text().unwrap(), "streamed content");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn write_writer_truncates_existing() {
    let root = std::env::temp_dir().join("forge_fs_disk_write_writer_trunc");
    let dir = DiskDir::new(&root);
    dir.create_dir(".").unwrap();
    let f = dir.get_file("data.txt").unwrap();
    f.write_text("old content that is longer").unwrap();

    let mut w = f.write_writer().unwrap();
    std::io::Write::write_all(&mut w, b"new").unwrap();
    std::io::Write::flush(&mut w).unwrap();
    drop(w);

    assert_eq!(f.read_text().unwrap(), "new");
    let _ = std::fs::remove_dir_all(&root);
}
