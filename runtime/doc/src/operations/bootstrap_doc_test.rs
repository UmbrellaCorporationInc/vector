#![allow(clippy::unwrap_used, clippy::expect_used)]

use runtime_core::FlowOperation;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

use crate::operations::BootstrapDocOp;
use crate::operations::support::CapturingSender;
use crate::operations::{BootstrapDocInput, BootstrapDocOutput};

fn create_test_project() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let config = "doc-type:
  template: template-00004-doc-type-template
  prompt-template: template-00005-doc-type-prompt
  prompt: prompts-00001-create-doc-type
document-types:
  rfc:
    template: template-00001-rfc
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    initial-status: draft
    statuses:
      - draft
      - review
      - accepted
  spec:
    template: template-00003-spec
    layout: category
    code-width: 5
    prompt: prompts-00002-create-spec
  task:
    template: template-task
    layout: status
    code-width: 5
    prompt: prompts-00003-create-task
    initial-status: todo
    statuses:
      - todo
      - in_progress
      - done
  project:
    template: template-project
    layout: category
    code-width: 5
    prompt: prompts-00004-create-project
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    let doc_dir = temp_dir.path().join("doc");
    fs::create_dir_all(&doc_dir).unwrap();

    (temp_dir, root)
}

#[tokio::test]
async fn test_bootstrap_doc_with_valid_slug() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "rfc".to_string(),
        slug: "my-new-rfc".to_string(),
        title: None,
        category: None,
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;
    op.run(input, &mut sender).await.unwrap();

    let output = sender.into_output().expect("Output not set");
    assert!(output.path.contains("rfc"));
    assert!(output.path.contains("00001"));
    assert!(output.path.contains("my-new-rfc.md"));
    assert_eq!(output.code, "00001");

    let file_path = std::path::Path::new(&output.path);
    assert!(file_path.exists(), "Document file should exist");

    let content = fs::read_to_string(file_path).unwrap();
    assert!(content.contains("---"));
    assert!(content.contains("type: rfc"));
    assert!(content.contains("slug: my-new-rfc"));
}

#[tokio::test]
async fn test_bootstrap_doc_with_invalid_slug() {
    let (_temp_dir, root) = create_test_project();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "rfc".to_string(),
        slug: "Invalid-Slug".to_string(),
        title: None,
        category: None,
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;

    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with invalid slug containing uppercase");
}

#[tokio::test]
async fn test_bootstrap_doc_with_consecutive_hyphens() {
    let (_temp_dir, root) = create_test_project();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "rfc".to_string(),
        slug: "my--invalid--slug".to_string(),
        title: None,
        category: None,
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;

    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with consecutive hyphens");
}

#[tokio::test]
async fn test_bootstrap_doc_with_hyphen_start_end() {
    let (_temp_dir, root) = create_test_project();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "rfc".to_string(),
        slug: "-starts-with-hyphen".to_string(),
        title: None,
        category: None,
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;

    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail when slug starts with hyphen");
}

#[tokio::test]
async fn test_bootstrap_doc_empty_slug() {
    let (_temp_dir, root) = create_test_project();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "rfc".to_string(),
        slug: String::new(),
        title: None,
        category: None,
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;

    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with empty slug");
}

#[tokio::test]
async fn test_bootstrap_doc_unknown_doc_type() {
    let (_temp_dir, root) = create_test_project();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "nonexistent".to_string(),
        slug: "valid-slug".to_string(),
        title: None,
        category: None,
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;

    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with unknown doc type");
}

#[tokio::test]
async fn test_bootstrap_doc_code_incrementing() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    fs::write(
        draft_dir.join("rfc-00001-first-rfc.md"),
        "---\n\
id: rfc-00001-first\n\
type: rfc\n\
code: \"00001\"\n\
slug: first-rfc\n\
title: First RFC\n\
description: First RFC\n\
created: 2026-01-01\n\
tags: []\nrelated: []\n---\n\n# First RFC\n",
    )
    .unwrap();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "rfc".to_string(),
        slug: "second-rfc".to_string(),
        title: None,
        category: None,
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;
    op.run(input, &mut sender).await.unwrap();

    let output = sender.into_output().expect("Output not set");
    assert_eq!(output.code, "00002");
}

#[tokio::test]
async fn test_bootstrap_doc_category_based() {
    let (_temp_dir, root) = create_test_project();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "spec".to_string(),
        slug: "my-spec".to_string(),
        title: None,
        category: Some("api".to_string()),
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;
    op.run(input, &mut sender).await.unwrap();

    let output = sender.into_output().expect("Output not set");
    assert!(output.path.contains("spec"));
    assert_eq!(output.code, "00001");
}

#[tokio::test]
async fn test_bootstrap_doc_naming_invariant() {
    let (_temp_dir, root) = create_test_project();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "rfc".to_string(),
        slug: "test-rfc".to_string(),
        title: None,
        category: None,
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;
    op.run(input, &mut sender).await.unwrap();

    let output = sender.into_output().expect("Output not set");
    let file_name =
        std::path::Path::new(&output.path).file_name().and_then(|n| n.to_str()).unwrap();
    assert!(file_name.starts_with("rfc-"), "File name should start with doc_type");
    assert!(file_name.contains("-00001-"), "File name should contain code");
    assert!(file_name.ends_with("-test-rfc.md"), "File name should end with slug");
}

#[tokio::test]
async fn test_bootstrap_doc_directory_based() {
    let (temp_dir, root) = create_test_project();

    // Add a directory based type to config
    let config_path = temp_dir.path().join(".vector").join("document-types.yaml");
    let mut config_content = fs::read_to_string(&config_path).unwrap();
    config_content.push_str("\n  research:\n    layout: directory\n    code-width: 5\n");
    fs::write(&config_path, config_content).unwrap();

    let input = BootstrapDocInput {
        root_dir: root.clone(),
        doc_type: "research".to_string(),
        slug: "study-results".to_string(),
        title: Some("Study Results".to_string()),
        category: None,
    };

    let mut sender = CapturingSender::<BootstrapDocOutput>::new();
    let op = BootstrapDocOp;
    op.run(input, &mut sender).await.unwrap();

    let output = sender.into_output().expect("Output should be sent");
    assert_eq!(output.code, "00001");

    // Path should be directly under doc/research/
    let expected_path =
        temp_dir.path().join("doc").join("research").join("research-00001-study-results.md");
    assert!(expected_path.exists(), "Document should exist at {}", expected_path.display());
}
