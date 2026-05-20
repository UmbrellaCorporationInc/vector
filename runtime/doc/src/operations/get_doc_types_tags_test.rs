#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

struct MockSender {
    outputs: Vec<GetDocTypesTagsOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { outputs: Vec::new() }
    }
}

impl runtime_core::Sender<GetDocTypesTagsOutput> for MockSender {
    async fn send(&mut self, value: GetDocTypesTagsOutput) -> runtime_core::RuntimeResult<()> {
        self.outputs.push(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<GetDocTypesTagsOutput> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[tokio::test]
async fn test_get_doc_types_tags_success() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    // Setup config
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    tags: [rust, runtime]\n  task:\n    layout: status\n    code-width: 5\n    prompt: prompts-00002-create-task\n    tags: [chore, rust]",
    ).unwrap();

    let input = GetDocTypesTagsInput { root_dir: root };
    let mut sender = MockSender::new();
    let result = get_doc_types_tags(input, &mut sender).await;

    assert!(result.is_ok());
    assert_eq!(sender.outputs.len(), 1);
    assert_eq!(sender.outputs[0].tags, "chore,runtime,rust");
}

#[tokio::test]
async fn test_get_doc_types_tags_empty() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    // Setup config without tags
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc",
    )
    .unwrap();

    let input = GetDocTypesTagsInput { root_dir: root };
    let mut sender = MockSender::new();
    let result = get_doc_types_tags(input, &mut sender).await;

    assert!(result.is_ok());
    assert_eq!(sender.outputs.len(), 1);
    assert_eq!(sender.outputs[0].tags, "");
}
