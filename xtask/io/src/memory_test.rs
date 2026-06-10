#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::*;

#[test]
fn write_and_read_utf8() {
    let dir = MemoryDir::new();
    let f = dir.get_file("docs/readme.txt").unwrap();
    f.write_text("hello forge").unwrap();
    assert_eq!(f.read_text().unwrap(), "hello forge");
}

#[test]
fn write_and_read_binary() {
    let dir = MemoryDir::new();
    let f = dir.get_file("data/blob.bin").unwrap();
    let bytes: Vec<u8> = vec![0x00, 0xFF, 0x42, 0x13];
    f.write_bytes(&bytes).unwrap();
    assert_eq!(f.read_bytes().unwrap(), bytes);
}

#[test]
fn delete_removes_resource() {
    let dir = MemoryDir::new();
    let f = dir.get_file("tmp/session.lock").unwrap();
    f.write_text("active").unwrap();
    f.delete().unwrap();
    assert!(matches!(f.read_text(), Err(FsError::NotFound(_))));
}

#[test]
fn idempotent_create_dir() {
    let dir = MemoryDir::new();
    assert!(dir.create_dir("sub/nested").is_ok());
    assert!(dir.create_dir("sub/nested").is_ok());
}

#[test]
fn read_missing_file_returns_not_found() {
    let dir = MemoryDir::new();
    let result = dir.get_file("ghost/file.txt").unwrap().read_text();
    assert!(matches!(result, Err(FsError::NotFound(_))));
}

#[test]
fn nested_path_routing() {
    let dir = MemoryDir::new();
    let sub = dir.subdir("a/b");
    let f = sub.get_file("c.txt").unwrap();
    f.write_text("nested").unwrap();
    // Access through the root handle on the same full path confirms shared store.
    assert_eq!(dir.get_file("a/b/c.txt").unwrap().read_text().unwrap(), "nested");
}

#[test]
fn delete_nonexistent_returns_not_found() {
    let dir = MemoryDir::new();
    let result = dir.get_file("never/existed.txt").unwrap().delete();
    assert!(matches!(result, Err(FsError::NotFound(_))));
}

#[test]
fn overwrite_replaces_content() {
    let dir = MemoryDir::new();
    let f = dir.get_file("state/config.txt").unwrap();
    f.write_text("v1").unwrap();
    f.write_text("v2").unwrap();
    assert_eq!(f.read_text().unwrap(), "v2");
}

#[test]
fn exists_returns_true_after_write_and_false_before() {
    let dir = MemoryDir::new();
    let f = dir.get_file("existence/check.txt").unwrap();
    assert!(!f.exists());
    f.write_text("data").unwrap();
    assert!(f.exists());
    f.delete().unwrap();
    assert!(!f.exists());
}

#[test]
fn read_text_on_invalid_utf8_returns_io_error() {
    let dir = MemoryDir::new();
    let f = dir.get_file("binary/payload.bin").unwrap();
    // 0xFF is never valid in UTF-8
    f.write_bytes(&[0xFF, 0xFE, 0x00]).unwrap();
    assert!(matches!(f.read_text(), Err(FsError::Io(_))));
}

#[test]
fn read_reader_streams_file_contents() {
    let dir = MemoryDir::new();
    let f = dir.get_file("stream/data.bin").unwrap();
    f.write_bytes(&[0x01, 0x02, 0x03]).unwrap();

    let mut reader = f.read_reader().unwrap();
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut buf).unwrap();
    assert_eq!(buf, vec![0x01, 0x02, 0x03]);
}

#[test]
fn root_dir_resolves_files_directly() {
    let dir = MemoryDir::new();
    dir.get_file("root_file.txt").unwrap().write_text("root").unwrap();
    let f = dir.get_file("root_file.txt").unwrap();
    assert_eq!(f.read_text().unwrap(), "root");
}

#[test]
fn multiple_handles_to_same_path_share_store() {
    let dir = MemoryDir::new();
    let f1 = dir.get_file("shared/data.txt").unwrap();
    let f2 = dir.get_file("shared/data.txt").unwrap();

    f1.write_text("written via f1").unwrap();
    assert_eq!(f2.read_text().unwrap(), "written via f1");
}

#[test]
fn subdir_shares_backing_store_with_parent() {
    let root = MemoryDir::new();
    let sub = root.subdir("workspace");
    sub.get_file("config.txt").unwrap().write_text("shared").unwrap();
    // Root can see the file via its full path.
    assert_eq!(root.get_file("workspace/config.txt").unwrap().read_text().unwrap(), "shared");
}

#[test]
fn clone_shares_backing_store() {
    let dir = MemoryDir::new();
    let dir2 = dir.clone();
    dir.get_file("data.txt").unwrap().write_text("hello").unwrap();
    assert_eq!(dir2.get_file("data.txt").unwrap().read_text().unwrap(), "hello");
}

#[test]
fn list_files_returns_all_keys_from_root() {
    let dir = MemoryDir::new();
    dir.get_file("a/b/deep.txt").unwrap().write_text("deep").unwrap();
    dir.get_file("a/top.txt").unwrap().write_text("top").unwrap();
    dir.get_file("root.txt").unwrap().write_text("root").unwrap();

    let mut paths: Vec<String> = dir.list_files().unwrap().map(|r| r.unwrap()).collect();
    paths.sort();

    assert_eq!(paths, vec!["a/b/deep.txt", "a/top.txt", "root.txt"]);
}

#[test]
fn list_files_from_subdir_returns_relative_paths() {
    let root = MemoryDir::new();
    let sub = root.subdir("ns");
    sub.get_file("a.heph").unwrap().write_text("a").unwrap();
    sub.get_file("nested/b.heph").unwrap().write_text("b").unwrap();

    let mut paths: Vec<String> = sub.list_files().unwrap().map(|r| r.unwrap()).collect();
    paths.sort();

    assert_eq!(paths, vec!["a.heph", "nested/b.heph"]);
}

#[test]
fn list_files_subdir_does_not_include_sibling_namespace() {
    let root = MemoryDir::new();
    root.subdir("ns_a").get_file("file_a.heph").unwrap().write_text("a").unwrap();
    root.subdir("ns_b").get_file("file_b.heph").unwrap().write_text("b").unwrap();

    let paths: Vec<String> =
        root.subdir("ns_a").list_files().unwrap().map(|r| r.unwrap()).collect();

    assert_eq!(paths, vec!["file_a.heph"]);
}

#[test]
fn subdir_of_subdir_concatenates_prefixes() {
    // Exercises the `format!("{}/{path}", self.prefix)` branch in subdir()
    // where self.prefix is already non-empty.
    let root = MemoryDir::new();
    let sub = root.subdir("a");
    let nested = sub.subdir("b");
    nested.get_file("c.txt").unwrap().write_text("deep").unwrap();
    assert_eq!(root.get_file("a/b/c.txt").unwrap().read_text().unwrap(), "deep");
}

#[test]
fn last_modified_returns_nonzero_after_write() {
    let dir = MemoryDir::new();
    let f = dir.get_file("meta/ts.txt").unwrap();
    f.write_text("payload").unwrap();
    let ts = f.last_modified().unwrap();
    assert!(ts > 0, "expected a non-zero Unix timestamp after write, got {ts}");
}

#[test]
fn last_modified_is_non_decreasing_across_sequential_writes() {
    let dir = MemoryDir::new();
    let f = dir.get_file("meta/monotonic.txt").unwrap();
    f.write_text("first").unwrap();
    let ts1 = f.last_modified().unwrap();
    f.write_text("second").unwrap();
    let ts2 = f.last_modified().unwrap();
    assert!(ts2 >= ts1, "expected ts2 ({ts2}) >= ts1 ({ts1}) across sequential writes");
}

#[test]
fn last_modified_returns_not_found_for_unwritten_file() {
    let dir = MemoryDir::new();
    let result = dir.get_file("ghost/unwritten.txt").unwrap().last_modified();
    assert!(matches!(result, Err(FsError::NotFound(_))));
}

#[test]
fn write_writer_commits_on_flush() {
    let dir = MemoryDir::new();
    let f = dir.get_file("writer/flush.txt").unwrap();

    let mut w = f.write_writer().unwrap();
    std::io::Write::write_all(&mut w, b"flushed").unwrap();
    std::io::Write::flush(&mut w).unwrap();

    // Must be readable before drop
    assert_eq!(f.read_text().unwrap(), "flushed");
    drop(w);
    assert_eq!(f.read_text().unwrap(), "flushed");
}

#[test]
fn write_writer_commits_on_drop_without_flush() {
    let dir = MemoryDir::new();
    let f = dir.get_file("writer/drop.txt").unwrap();

    {
        let mut w = f.write_writer().unwrap();
        std::io::Write::write_all(&mut w, b"dropped").unwrap();
        // No explicit flush — drop should commit
    }

    assert_eq!(f.read_text().unwrap(), "dropped");
}

#[test]
fn write_writer_double_flush_is_idempotent() {
    let dir = MemoryDir::new();
    let f = dir.get_file("writer/idempotent.txt").unwrap();

    let mut w = f.write_writer().unwrap();
    std::io::Write::write_all(&mut w, b"stable").unwrap();
    std::io::Write::flush(&mut w).unwrap();
    std::io::Write::flush(&mut w).unwrap();

    assert_eq!(f.read_text().unwrap(), "stable");
}
