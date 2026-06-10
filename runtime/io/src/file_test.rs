#![allow(clippy::unwrap_used)]
use super::*;
use std::thread;
use std::time::{Duration, SystemTime};
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

#[tokio::test]
async fn test_hash_file_content_returns_same_hash_for_same_bytes_across_paths() {
    let fixture = HashFixture::new("same-bytes");
    let left = fixture.file_path("left.bin");
    let right = fixture.file_path("right.bin");

    write_file_bytes(&left, b"same content".to_vec()).await.unwrap();
    write_file_bytes(&right, b"same content".to_vec()).await.unwrap();

    let left_hash = hash_file_content(&left).await.unwrap();
    let right_hash = hash_file_content(&right).await.unwrap();

    assert_eq!(left_hash, right_hash);
    assert_eq!(left_hash.as_hex(), left_hash.as_hex().to_lowercase());
    assert_eq!(left_hash.as_hex().len(), 64);

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_hash_file_content_changes_when_bytes_change() {
    let fixture = HashFixture::new("changed-bytes");
    let path = fixture.file_path("content.bin");

    write_file_bytes(&path, b"before".to_vec()).await.unwrap();
    let before_hash = hash_file_content(&path).await.unwrap();

    write_file_bytes(&path, b"after".to_vec()).await.unwrap();
    let after_hash = hash_file_content(&path).await.unwrap();

    assert_ne!(before_hash, after_hash);

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_hash_file_content_ignores_modified_time_without_content_change() {
    let fixture = HashFixture::new("modified-time");
    let path = fixture.file_path("content.bin");

    write_file_bytes(&path, b"stable content".to_vec()).await.unwrap();
    let before_hash = hash_file_content(&path).await.unwrap();
    let before_modified = fs::metadata(path.as_path()).await.unwrap().modified().unwrap();

    let after_modified = wait_for_modified_time_change(&path, before_modified).await;
    let after_hash = hash_file_content(&path).await.unwrap();

    assert!(after_modified > before_modified);
    assert_eq!(before_hash, after_hash);

    fixture.cleanup().await;
}

struct HashFixture {
    root: IoPath,
}

impl HashFixture {
    fn new(name: &str) -> Self {
        Self { root: IoPath::new(format!("test_file_hash_{name}")) }
    }

    fn file_path(&self, name: &str) -> IoPath {
        self.root.join(name)
    }

    async fn cleanup(self) {
        let _ = fs::remove_dir_all(self.root).await;
    }
}

async fn wait_for_modified_time_change(path: &IoPath, before: SystemTime) -> SystemTime {
    for _ in 0..3 {
        thread::sleep(Duration::from_secs(1));
        let bytes = read_file_bytes(path).await.unwrap();
        write_file_bytes(path, bytes).await.unwrap();

        let modified = fs::metadata(path.as_path()).await.unwrap().modified().unwrap();
        if modified > before {
            return modified;
        }
    }

    fs::metadata(path.as_path()).await.unwrap().modified().unwrap()
}
