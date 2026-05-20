#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;
use crate::memory::{MemReader, MemWriter};
use runtime_core::{Receiver, Sender};

#[tokio::test]
async fn test_text_reader_valid_utf8() {
    let bytes = b"hello world".to_vec();
    let mem = MemReader::new(bytes, 5);
    let mut reader = TextReader::new(mem, 3);

    assert_eq!(reader.recv().await, Ok(Some("hel".into())));
    assert_eq!(reader.recv().await, Ok(Some("lo".into())));
    assert_eq!(reader.recv().await, Ok(Some(" wo".into())));
    assert_eq!(reader.recv().await, Ok(Some("rl".into())));
    assert_eq!(reader.recv().await, Ok(Some("d".into())));
    assert_eq!(reader.recv().await, Ok(None));
}

#[tokio::test]
async fn test_text_reader_split_multibyte() {
    let text = "a🚀b";
    let bytes = text.as_bytes().to_vec();
    let mem = MemReader::new(bytes, 1);
    let mut reader = TextReader::new(mem, 10);

    assert_eq!(reader.recv().await, Ok(Some("a".into())));
    assert_eq!(reader.recv().await, Ok(Some("🚀".into())));
    assert_eq!(reader.recv().await, Ok(Some("b".into())));
    assert_eq!(reader.recv().await, Ok(None));
}

#[tokio::test]
async fn test_text_reader_invalid_utf8() {
    let bytes = vec![0xFF, 0xFF];
    let mem = MemReader::new(bytes, 10);
    let mut reader = TextReader::new(mem, 10);

    assert_eq!(reader.recv().await, Ok(None));
}

#[tokio::test]
async fn test_text_reader_returns_valid_prefix_before_invalid_utf8() {
    let bytes = vec![b'a', 0xF0, 0x28, 0x8C, 0x28];
    let mem = MemReader::new(bytes, 16);
    let mut reader = TextReader::new(mem, 16);

    assert_eq!(reader.recv().await, Ok(Some("a".into())));
    assert_eq!(reader.recv().await, Ok(None));
}

#[tokio::test]
async fn test_text_reader_discards_trailing_incomplete_utf8_at_eof() {
    let bytes = vec![0xF0, 0x9F, 0x9A];
    let mem = MemReader::new(bytes, 16);
    let mut reader = TextReader::new(mem, 16);

    assert_eq!(reader.recv().await, Ok(None));
    assert_eq!(reader.recv().await, Ok(None));
}

#[tokio::test]
async fn test_text_reader_zero_buffer_size_still_yields_progress() {
    let bytes = b"ab".to_vec();
    let mem = MemReader::new(bytes, 16);
    let mut reader = TextReader::new(mem, 0);

    assert_eq!(reader.recv().await, Ok(Some("a".into())));
    assert_eq!(reader.recv().await, Ok(Some("b".into())));
    assert_eq!(reader.recv().await, Ok(None));
}

#[tokio::test]
async fn test_text_writer_buffered() {
    let mem = MemWriter::new();
    let mut writer = TextWriter::new(mem, 3);

    writer.send("🚀".into()).await.expect("send");
    writer.flush().await.expect("flush");

    let final_mem = writer.into_inner();
    let bytes = final_mem.into_inner();
    assert_eq!(bytes, "🚀".as_bytes().to_vec());
}

#[tokio::test]
async fn test_text_writer_close_flushes_remaining_bytes() {
    let mem = MemWriter::new();
    let mut writer = TextWriter::new(mem, 16);

    writer.send("hello".into()).await.expect("send");
    writer.close().await.expect("close");
}

#[tokio::test]
async fn test_text_writer_zero_buffer_size_flushes_one_byte_chunks() {
    let mem = MemWriter::new();
    let mut writer = TextWriter::new(mem, 0);

    writer.send("ab".into()).await.expect("send");
    let final_mem = writer.into_inner();

    assert_eq!(final_mem.as_bytes(), b"ab");
}

#[tokio::test]
async fn test_file_helpers_text() {
    let test_file = IoPath::new("test_file_helpers_text.txt");
    let text = "🚀 Hello, Vector! 🚀".to_string();

    write_file_text(&test_file, text.clone()).await.unwrap();

    let read_data = read_file_text(&test_file).await.unwrap();
    assert_eq!(read_data, text);

    let _ = std::fs::remove_file(test_file);
}
