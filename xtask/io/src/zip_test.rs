#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Write};

use zip::CompressionMethod;
use zip::write::FileOptions;

use crate::{Directory, ZipDir};

fn create_sample_zip() -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);

        zip.start_file("hello.txt", options).unwrap();
        zip.write_all(b"hello world").unwrap();

        zip.start_file("src/main.rs", options).unwrap();
        zip.write_all(b"fn main() {}").unwrap();

        zip.finish().unwrap();
    }
    buf
}

#[test]
fn test_zip_list_files() {
    let data = create_sample_zip();
    let zip_dir = ZipDir::new(Cursor::new(data)).expect("mount zip");

    let mut files: Vec<String> = zip_dir.list_files().unwrap().map(|r| r.unwrap()).collect();
    files.sort();
    assert_eq!(files.len(), 2);
    assert_eq!(files, vec!["hello.txt", "src/main.rs"]);
}

#[test]
fn test_zip_read_file() {
    let data = create_sample_zip();
    let zip_dir = ZipDir::new(Cursor::new(data)).expect("mount zip");

    let file = zip_dir.get_file("hello.txt").unwrap();
    assert_eq!(file.read_text().unwrap(), "hello world");

    let file = zip_dir.get_file("src/main.rs").unwrap();
    assert_eq!(file.read_text().unwrap(), "fn main() {}");
}

#[test]
fn test_zip_subdir() {
    let data = create_sample_zip();
    let zip_dir = ZipDir::new(Cursor::new(data)).expect("mount zip");
    let src_dir = zip_dir.subdir("src");

    let files: Vec<String> = src_dir.list_files().unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(files, vec!["main.rs".to_string()]);

    let file = src_dir.get_file("main.rs").unwrap();
    assert_eq!(file.read_text().unwrap(), "fn main() {}");
}

#[test]
fn test_zip_exists() {
    let data = create_sample_zip();
    let zip_dir = ZipDir::new(Cursor::new(data)).expect("mount zip");

    assert!(zip_dir.get_file("hello.txt").unwrap().exists());
    assert!(!zip_dir.get_file("nonexistent").unwrap().exists());
}

#[test]
fn test_zip_read_only() {
    let data = create_sample_zip();
    let zip_dir = ZipDir::new(Cursor::new(data)).expect("mount zip");
    let file = zip_dir.get_file("hello.txt").unwrap();

    assert!(file.write_text("fail").is_err());
    assert!(file.delete().is_err());

    // create_dir should be idempotent if path already exists
    assert!(zip_dir.create_dir("src").is_ok());
    // but fail if it's a truly new directory
    assert!(zip_dir.create_dir("new_dir").is_err());
}
