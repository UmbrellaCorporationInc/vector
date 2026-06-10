#![allow(clippy::unwrap_used)]

use super::*;
use crate::{read_file_bytes, read_file_text};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[tokio::test]
async fn test_list_directory_returns_direct_children_in_path_order() {
    let fixture = DirectoryFixture::create("list").await;
    fixture.write_file("b.txt").await;
    fixture.write_file("a.txt").await;
    fixture.create_dir("nested").await;

    let entries = list_directory(fixture.root()).await.unwrap();
    let relative_paths = fixture.relative_paths(&entries);

    assert_eq!(relative_paths, vec!["a.txt", "b.txt", "nested"]);
    assert!(entries.iter().any(DirectoryEntry::is_file));
    assert!(entries.iter().any(DirectoryEntry::is_directory));
    assert!(entries.iter().all(|entry| entry.modified().is_some()));

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_traverse_directory_returns_descendants_in_stable_order() {
    let fixture = DirectoryFixture::create("traverse").await;
    fixture.write_file("z.md").await;
    fixture.create_dir("b").await;
    fixture.write_file("b/two.md").await;
    fixture.create_dir("a").await;
    fixture.write_file("a/one.md").await;

    let first = traverse_directory(fixture.root()).await.unwrap();
    let second = traverse_directory(fixture.root()).await.unwrap();

    assert_eq!(fixture.relative_paths(&first), fixture.relative_paths(&second));
    assert_eq!(fixture.relative_paths(&first), vec!["a", "a/one.md", "b", "b/two.md", "z.md"]);

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_traverse_directories_sorts_multiple_roots() {
    let left = DirectoryFixture::create("left-root").await;
    let right = DirectoryFixture::create("right-root").await;
    left.write_file("b.txt").await;
    right.write_file("a.txt").await;

    let entries = traverse_directories(&[right.root().clone(), left.root().clone()]).await.unwrap();
    assert_eq!(entries[0].path().as_path().strip_prefix(left.root().as_path()).unwrap(), "b.txt");
    assert_eq!(entries[1].path().as_path().strip_prefix(right.root().as_path()).unwrap(), "a.txt");

    left.cleanup().await;
    right.cleanup().await;
}

#[tokio::test]
async fn test_list_directory_returns_typed_error_for_missing_directory() {
    let fixture = DirectoryFixture::create("missing").await;
    let missing = fixture.root().join("missing");

    let error = list_directory(&missing).await.unwrap_err();

    assert!(matches!(error, IoError::File(_)));

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_traversal_paths_can_be_read_as_bytes_and_text_deterministically() {
    let fixture = DirectoryFixture::create("read-composition").await;
    fixture.write_file_with_content("z.md", "zulu").await;
    fixture.create_dir("a").await;
    fixture.write_file_with_content("a/one.md", "alpha").await;
    fixture.create_dir("b").await;
    fixture.write_file_with_content("b/two.md", "bravo").await;

    let first = read_file_records(fixture.root()).await;
    let second = read_file_records(fixture.root()).await;

    assert_eq!(first, second);
    assert_eq!(
        first,
        vec![
            ("a/one.md".to_string(), "alpha".to_string(), b"alpha".to_vec()),
            ("b/two.md".to_string(), "bravo".to_string(), b"bravo".to_vec()),
            ("z.md".to_string(), "zulu".to_string(), b"zulu".to_vec()),
        ]
    );

    fixture.cleanup().await;
}

struct DirectoryFixture {
    root: IoPath,
}

impl DirectoryFixture {
    async fn create(name: &str) -> Self {
        let root = IoPath::new(unique_fixture_path(name));
        fs::create_dir_all(root.as_path()).await.unwrap();
        Self { root }
    }

    fn root(&self) -> &IoPath {
        &self.root
    }

    async fn create_dir(&self, relative_path: &str) {
        fs::create_dir_all(self.root.join(relative_path).as_path()).await.unwrap();
    }

    async fn write_file(&self, relative_path: &str) {
        self.write_file_with_content(relative_path, "test").await;
    }

    async fn write_file_with_content(&self, relative_path: &str, content: &str) {
        let path = self.root.join(relative_path);
        if let Some(parent) = path.as_path().parent() {
            fs::create_dir_all(parent).await.unwrap();
        }
        fs::write(path.as_path(), content.as_bytes()).await.unwrap();
    }

    fn relative_paths(&self, entries: &[DirectoryEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| {
                entry
                    .path()
                    .as_path()
                    .strip_prefix(self.root.as_path())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    async fn cleanup(self) {
        let _ = fs::remove_dir_all(self.root.as_path()).await;
    }
}

fn unique_fixture_path(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("vector-runtime-io-directory-{name}-{nanos}"))
}

async fn read_file_records(root: &IoPath) -> Vec<(String, String, Vec<u8>)> {
    let entries = traverse_directory(root).await.unwrap();
    let mut records = Vec::new();

    for entry in entries.iter().filter(|entry| entry.is_file()) {
        let relative_path = entry
            .path()
            .as_path()
            .strip_prefix(root.as_path())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let text = read_file_text(entry.path()).await.unwrap();
        let bytes = read_file_bytes(entry.path()).await.unwrap();
        records.push((relative_path, text, bytes));
    }

    records
}
