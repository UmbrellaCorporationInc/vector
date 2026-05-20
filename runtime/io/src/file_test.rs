#![allow(clippy::unwrap_used)]
use super::*;
use tokio::fs;

#[tokio::test]
async fn test_file_read_write_end_of_stream() {
    let test_file = IoPath::new("test_file_read_write.txt");

    // Write data with buffer size 3
    let mut writer = FileWriter::create(&test_file, 3).await.unwrap();
    writer.send(vec![1, 2]).await.unwrap(); // Buffer: [1, 2]
    writer.send(vec![3, 4]).await.unwrap(); // Buffer: [1, 2, 3, 4] -> Flush
    writer.send(vec![5]).await.unwrap(); // Buffer: [5]
    writer.close().await.unwrap(); // Flush remaining [5]

    // Read data with buffer size 2
    let mut reader = FileReader::open(&test_file, 2).await.unwrap();
    assert_eq!(reader.recv().await, Ok(Some(vec![1, 2])));
    assert_eq!(reader.recv().await, Ok(Some(vec![3, 4])));
    assert_eq!(reader.recv().await, Ok(Some(vec![5])));
    assert_eq!(reader.recv().await, Ok(None)); // EOF

    // Cleanup
    let _ = fs::remove_file(test_file).await;
}

#[tokio::test]
async fn test_file_error_on_not_found() {
    let err = FileReader::open(&IoPath::new("does_not_exist_xyz.txt"), 10).await;
    assert!(err.is_err());
}
#[tokio::test]
async fn test_file_helpers_bytes() {
    let test_file = IoPath::new("test_file_helpers_bytes.bin");

    let data = vec![10, 20, 30, 40, 50];
    write_file_bytes(&test_file, data.clone()).await.unwrap();

    let read_data = read_file_bytes(&test_file).await.unwrap();
    assert_eq!(read_data, data);

    let _ = fs::remove_file(test_file).await;
}

#[tokio::test]
async fn test_create_dir_all_and_parent_guarantee() {
    let base_dir = IoPath::new("test_parent_dir_creation");
    let nested_file = base_dir.join("nested").join("file.bin");

    // Write file in nested structure (guarantees parent creation)
    let data = vec![1, 2, 3];
    write_file_bytes(&nested_file, data.clone()).await.unwrap();

    let read_data = read_file_bytes(&nested_file).await.unwrap();
    assert_eq!(read_data, data);

    // Test explicit create_dir_all
    let explicit_dir = base_dir.join("explicit_dir");
    create_dir_all(&explicit_dir).await.unwrap();
    assert!(explicit_dir.as_path().exists());
    assert!(explicit_dir.as_path().is_dir());

    // Cleanup
    let _ = fs::remove_dir_all(base_dir).await;
}
