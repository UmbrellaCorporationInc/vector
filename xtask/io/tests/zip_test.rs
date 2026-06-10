//! Integration tests for `ZipDir` backend.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use io::{Directory, ZipDir};
use std::io::{Cursor, Read, Write};
use zip::CompressionMethod;
use zip::write::FileOptions;

fn create_sample_zip() -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);

        zip.start_file("A.txt", options).unwrap();
        zip.write_all(b"content A").unwrap();

        zip.start_file("B.txt", options).unwrap();
        zip.write_all(b"content B").unwrap();

        zip.finish().unwrap();
    }
    buf
}

#[test]
fn test_concurrent_readers() {
    let data = create_sample_zip();
    let zip_dir = ZipDir::new(Cursor::new(data)).expect("mount zip");

    let file_a = zip_dir.get_file("A.txt").unwrap();
    let file_b = zip_dir.get_file("B.txt").unwrap();

    let mut reader_a = file_a.read_reader().unwrap();
    let mut reader_b = file_b.read_reader().unwrap();

    // Read concurrently (or sequentially in this test, but with active readers)
    let mut buf_a = String::new();
    let mut buf_b = String::new();

    reader_a.read_to_string(&mut buf_a).unwrap();
    reader_b.read_to_string(&mut buf_b).unwrap();

    assert_eq!(buf_a, "content A");
    assert_eq!(buf_b, "content B");
}

#[test]
fn test_last_modified_conversion() {
    let data = create_sample_zip();
    let zip_dir = ZipDir::new(Cursor::new(data)).expect("mount zip");
    let file = zip_dir.get_file("A.txt").unwrap();

    let ts = file.last_modified().unwrap();
    assert!(ts > 0, "Last modified should be a valid Unix timestamp");
}
