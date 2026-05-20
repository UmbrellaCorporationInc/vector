#![allow(clippy::expect_used)]
use super::*;

#[tokio::test]
async fn test_mem_reader_chunks() {
    let data = vec![1, 2, 3, 4, 5];
    let mut reader = MemReader::new(data, 2);

    assert_eq!(reader.recv().await, Ok(Some(vec![1, 2])));
    assert_eq!(reader.recv().await, Ok(Some(vec![3, 4])));
    assert_eq!(reader.recv().await, Ok(Some(vec![5])));
    assert_eq!(reader.recv().await, Ok(None));
}

#[tokio::test]
async fn test_mem_reader_empty() {
    let mut reader = MemReader::new(Vec::new(), 10);
    assert_eq!(reader.recv().await, Ok(None));
}

#[tokio::test]
async fn test_mem_writer_collection() {
    let mut writer = MemWriter::with_capacity(10);

    writer.send(vec![1, 2]).await.expect("send failed");
    writer.send(vec![3, 4, 5]).await.expect("send failed");

    assert_eq!(writer.as_bytes(), &[1, 2, 3, 4, 5]);

    let final_bytes = writer.into_inner();
    assert_eq!(final_bytes, vec![1, 2, 3, 4, 5]);
}
