#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

struct MockSender {
    outputs: Vec<FindDocOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { outputs: Vec::new() }
    }
}

impl runtime_core::Sender<FindDocOutput> for MockSender {
    async fn send(&mut self, value: FindDocOutput) -> runtime_core::RuntimeResult<()> {
        self.outputs.push(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<FindDocOutput> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[tokio::test]
async fn test_find_doc_success() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    // Setup config
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft]",
    )
    .unwrap();

    // Setup doc folder
    let rfc_dir = temp.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&rfc_dir).unwrap();
    let target_file = rfc_dir.join("rfc-00013-my-rfc.md");
    fs::write(&target_file, "content").unwrap();

    let input = FindDocInput { root_dir: root, doc_type: "rfc".to_string(), code: 13 };

    let mut sender = MockSender::new();
    let result = find_doc(input, &mut sender).await;

    assert!(result.is_ok());
    assert_eq!(sender.outputs.len(), 1);

    let expected_path = dunce::canonicalize(target_file).unwrap().to_string_lossy().to_string();
    assert_eq!(sender.outputs[0].path, expected_path);
}

#[tokio::test]
async fn test_find_doc_not_found() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    // Setup config
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft]",
    )
    .unwrap();

    let input = FindDocInput { root_dir: root, doc_type: "rfc".to_string(), code: 99 };

    let mut sender = MockSender::new();
    let result = find_doc(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_find_doc_invalid_type() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    // Setup config (empty)
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types: {}",
    )
    .unwrap();

    let input = FindDocInput { root_dir: root, doc_type: "unknown".to_string(), code: 1 };

    let mut sender = MockSender::new();
    let result = find_doc(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_find_doc_directory_based() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    // Setup config
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  research:\n    layout: directory\n    code-width: 5",
    )
    .unwrap();

    // Create a research document directly under doc/research/
    let research_dir = temp.path().join("doc").join("research");
    fs::create_dir_all(&research_dir).unwrap();
    let doc_path = research_dir.join("research-00001-study.md");
    fs::write(&doc_path, "# Study").unwrap();

    let input = FindDocInput { root_dir: root, doc_type: "research".to_string(), code: 1 };

    let mut sender = MockSender::new();
    find_doc(input, &mut sender).await.unwrap();

    let output = sender.outputs.first().expect("Output should be sent");
    let expected_path = dunce::canonicalize(doc_path).unwrap().to_string_lossy().to_string();
    assert_eq!(output.path, expected_path);
}
