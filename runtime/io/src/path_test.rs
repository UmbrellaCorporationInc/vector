#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;
use crate::{FileReader, FileWriter};
use std::path::PathBuf;

#[test]
fn test_io_path_creation_and_conversion() {
    let path = IoPath::new("/tmp/test");
    assert_eq!(path.as_path().to_str().unwrap().replace('\\', "/"), "/tmp/test");
}

#[test]
fn test_io_path_join() {
    let path = IoPath::new("/tmp");
    let joined = path.join("test.txt");
    assert_eq!(joined.as_path().to_str().unwrap().replace('\\', "/"), "/tmp/test.txt");
}

#[test]
fn test_io_path_as_ref() {
    let path = IoPath::new("/tmp/test");
    let std_path: &std::path::Path = path.as_ref();
    assert_eq!(std_path.to_str().unwrap().replace('\\', "/"), "/tmp/test");
}

#[test]
fn test_io_path_from_pathbuf() {
    let pb = PathBuf::from("/tmp/test");
    let path = IoPath::from(pb);
    assert_eq!(path.as_path().to_str().unwrap().replace('\\', "/"), "/tmp/test");
}

#[tokio::test]
async fn test_io_path_with_file_api() {
    let temp_dir = std::env::temp_dir();
    let file_path = IoPath::new(temp_dir).join("vector_io_path_test.txt");

    // IoPath should be accepted by FileWriter and FileReader because it implements AsRef<Path>
    let writer = FileWriter::create(&file_path, 1024).await.expect("create");
    writer.close().await.expect("close");

    let reader = FileReader::open(&file_path, 1024).await.expect("open");
    // Ensure it opened
    let _ = reader;

    let _ = std::fs::remove_file(file_path.as_path());
}
