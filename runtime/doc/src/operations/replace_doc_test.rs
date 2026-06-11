#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

struct MockSender {
    outputs: Vec<ReplaceDocOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { outputs: Vec::new() }
    }
}

impl runtime_core::Sender<ReplaceDocOutput> for MockSender {
    async fn send(&mut self, value: ReplaceDocOutput) -> runtime_core::RuntimeResult<()> {
        self.outputs.push(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<ReplaceDocOutput> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

fn write_doc_config(temp: &TempDir, doc_type: &str) {
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        format!(
            "doc-type: {{template: t, prompt-template: pt, prompt: p}}\ndocument-types:\n  {doc_type}:\n    layout: status\n    code-width: 5\n    prompt: p\n    statuses: [draft]"
        ),
    )
    .unwrap();
}

fn create_doc_file(temp: &TempDir, doc_type: &str, filename: &str, content: &str) {
    let dir = temp.path().join("doc").join(doc_type).join("draft");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(filename), content).unwrap();
}

fn valid_replacement_content() -> String {
    "\
---
id: task-00001-foo
type: task
code: \"00001\"
slug: foo
title: Updated
description: Updated description
created: 2026-06-11
updated: 2026-06-11
tags: []
status: draft
---

# Updated
"
    .to_string()
}

#[tokio::test]
async fn test_replace_doc_successfully_replaces_content_and_returns_path() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    let original = "\
---
id: task-00001-foo
type: task
code: \"00001\"
slug: foo
title: Original
description: Original description
created: 2026-06-11
updated: 2026-06-11
tags: []
status: draft
---

# Original
";
    create_doc_file(&temp, "task", filename, original);

    let replacement = valid_replacement_content();
    let input =
        ReplaceDocInput::new(root, String::new(), "task".to_string(), 1, replacement.clone());
    let mut sender = MockSender::new();
    replace_doc(input, &mut sender).await.unwrap();

    assert_eq!(sender.outputs.len(), 1);
    assert_eq!(sender.outputs[0].content, replacement);
    assert!(sender.outputs[0].path.ends_with(filename));

    let doc_path = temp.path().join("doc").join("task").join("draft").join(filename);
    let on_disk = fs::read_to_string(doc_path).unwrap();
    assert_eq!(on_disk, replacement);
}

#[tokio::test]
async fn test_replace_doc_missing_document_returns_error() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");

    let input = ReplaceDocInput::new(
        root,
        String::new(),
        "task".to_string(),
        99,
        valid_replacement_content(),
    );
    let mut sender = MockSender::new();
    let result = replace_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected error for missing document");
    assert!(sender.outputs.is_empty(), "missing document must not emit output");
}

#[tokio::test]
async fn test_replace_doc_rejects_mismatched_identity_without_write() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    let original = valid_replacement_content();
    create_doc_file(&temp, "task", filename, &original);

    let replacement = original.replace("slug: foo", "slug: other");
    let input = ReplaceDocInput::new(root, String::new(), "task".to_string(), 1, replacement);
    let mut sender = MockSender::new();
    let result = replace_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected mismatched identity rejection");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("frontmatter field 'slug'"), "{err}");
    assert!(err.contains("expected 'foo'"), "{err}");
    assert!(sender.outputs.is_empty(), "mismatched identity must not emit output");

    let doc_path = temp.path().join("doc").join("task").join("draft").join(filename);
    let on_disk = fs::read_to_string(doc_path).unwrap();
    assert_eq!(on_disk, original, "file must not be modified when identity mismatches");
}

#[tokio::test]
async fn test_replace_doc_rejects_utf8_bom_without_write() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    let original = valid_replacement_content();
    create_doc_file(&temp, "task", filename, &original);

    let replacement = format!("\u{feff}{}", valid_replacement_content());
    let input = ReplaceDocInput::new(root, String::new(), "task".to_string(), 1, replacement);
    let mut sender = MockSender::new();
    let result = replace_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected BOM rejection");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("UTF-8 BOM"), "{err}");
    assert!(sender.outputs.is_empty(), "BOM rejection must not emit output");

    let doc_path = temp.path().join("doc").join("task").join("draft").join(filename);
    let on_disk = fs::read_to_string(doc_path).unwrap();
    assert_eq!(on_disk, original, "file must not be modified when BOM is detected");
}

#[tokio::test]
async fn test_replace_doc_output_returns_final_content() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    create_doc_file(&temp, "task", filename, &valid_replacement_content());

    let replacement = "\
---
id: task-00001-foo
type: task
code: \"00001\"
slug: foo
title: Final
description: Final description
created: 2026-06-11
updated: 2026-06-11
tags: []
status: draft
---

# Final
"
    .to_string();
    let input =
        ReplaceDocInput::new(root, String::new(), "task".to_string(), 1, replacement.clone());
    let mut sender = MockSender::new();
    replace_doc(input, &mut sender).await.unwrap();

    assert_eq!(sender.outputs[0].content, replacement);
}
