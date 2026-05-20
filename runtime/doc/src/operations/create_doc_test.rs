#![allow(clippy::unwrap_used, clippy::expect_used)]

use runtime_core::FlowOperation;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

use crate::operations::CreateDocOp;
use crate::operations::support::CapturingSender;
use crate::operations::{CreateDocInput, CreateDocOutput};

fn create_test_project_with_prompt() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let config = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  rfc:
    template: template-00001-rfc
    layout: status
    code-width: 5
    initial-status: draft
    statuses:
      - draft
      - review
      - accepted
    prompt: prompts-00001-create-rfc
  prompts:
    template: template-00003-prompts
    layout: category
    code-width: 5
    prompt: prompts-00002-create-prompts
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    let doc_dir = temp_dir.path().join("doc");
    let rfc_dir = doc_dir.join("rfc").join("draft");
    let prompt_dir = doc_dir.join("prompts").join("authoring");
    let template_dir = doc_dir.join("template").join("project");
    fs::create_dir_all(&rfc_dir).unwrap();
    fs::create_dir_all(&prompt_dir).unwrap();
    fs::create_dir_all(&template_dir).unwrap();

    fs::write(rfc_dir.join("rfc-00001-existing-rfc.md"), "# RFC 1\n").unwrap();

    let prompt_content = "---
id: prompts-00001-create-rfc
type: prompts
code: \"00001\"
slug: create-rfc
title: Create RFC Prompt
description: Prompt for creating RFC documents.
category: authoring
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
tags: []
---

# Prompt: Create RFC

Type: #{doc-type}
Code: #{code}
Slug: #{slug}
Path: #{file-path}
";
    fs::write(prompt_dir.join("prompts-00001-create-rfc.md"), prompt_content).unwrap();

    let template_content = "---
id: rfc-00001-<slug>
type: rfc
code: \"00001\"
slug: <slug>
title: <Title>
description: <Description>
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
tags: []
related: []
---

# <Title>
";
    fs::write(template_dir.join("template-00001-rfc.md"), template_content).unwrap();

    (temp_dir, root)
}

fn create_test_project_no_prompt() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let config = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  spec:
    template: template-spec
    layout: category
    code-width: 5
  prompts:
    template: template-00003-prompts
    layout: category
    code-width: 5
    prompt: prompts-00002-create-doc
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    let doc_dir = temp_dir.path().join("doc");
    let spec_dir = doc_dir.join("spec").join("notes");
    let prompt_dir = doc_dir.join("prompts").join("authoring");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::create_dir_all(&prompt_dir).unwrap();

    let default_prompt_content = "---
id: prompts-00002-create-doc
type: prompts
code: \"00002\"
slug: create-doc
title: Create Document
description: Default prompt for creating documents.
category: authoring
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
tags: []
---

Type: #{doc-type}
Code: #{code}
Slug: #{slug}
Path: #{file-path}
";
    fs::write(prompt_dir.join("prompts-00002-create-doc.md"), default_prompt_content).unwrap();

    (temp_dir, root)
}

#[tokio::test]
async fn test_create_doc_with_valid_inputs() {
    let (_temp_dir, root) = create_test_project_with_prompt();

    let input = CreateDocInput {
        root_dir: root,
        doc_type: "rfc".to_string(),
        category: None,
        name: "My New RFC".to_string(),
        slug: "my-new-rfc".to_string(),
    };

    let mut sender = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    let result = op.run(input, &mut sender).await;

    assert!(result.is_ok());
    let output = sender.into_output().expect("output should exist");
    assert!(output.path.contains("rfc-00002-my-new-rfc.md"));
    assert_eq!(output.code, "00002");
    assert!(output.prompt.contains("Type: rfc"));
    assert!(output.prompt.contains("Code: 00002"));
    assert!(output.prompt.contains("Slug: my-new-rfc"));
    assert!(output.prompt.contains("Path:"));
    assert!(!output.prompt.contains("#{doc-type}"));
    assert!(!output.prompt.contains("#{code}"));
    assert!(!output.prompt.contains("#{slug}"));
    assert!(!output.prompt.contains("#{file-path}"));

    let created_path = std::path::Path::new(&output.path);
    assert!(created_path.exists(), "Created document should exist");
}

#[tokio::test]
async fn test_create_doc_validates_slug() {
    let (_temp_dir, root) = create_test_project_with_prompt();

    let input = CreateDocInput {
        root_dir: root,
        doc_type: "rfc".to_string(),
        category: None,
        name: "Test".to_string(),
        slug: "Invalid Slug With Spaces".to_string(),
    };

    let mut sender = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    let result = op.run(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_doc_rejects_slug_with_consecutive_hyphens() {
    let (_temp_dir, root) = create_test_project_with_prompt();

    let input = CreateDocInput {
        root_dir: root,
        doc_type: "rfc".to_string(),
        category: None,
        name: "Test".to_string(),
        slug: "invalid--slug".to_string(),
    };

    let mut sender = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    let result = op.run(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_doc_unknown_doc_type() {
    let (_temp_dir, root) = create_test_project_with_prompt();

    let input = CreateDocInput {
        root_dir: root,
        doc_type: "unknown_type".to_string(),
        category: None,
        name: "Test".to_string(),
        slug: "test-slug".to_string(),
    };

    let mut sender = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    let result = op.run(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_doc_uses_default_prompt_when_prompt_field_is_missing() {
    let (_temp_dir, root) = create_test_project_no_prompt();

    let input = CreateDocInput {
        root_dir: root,
        doc_type: "spec".to_string(),
        category: Some("notes".to_string()),
        name: "Test".to_string(),
        slug: "test-spec".to_string(),
    };

    let mut sender = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    let result = op.run(input, &mut sender).await;

    assert!(result.is_ok());
    let output = sender.into_output().expect("output should exist");
    assert_eq!(output.code, "00001");
    assert!(output.path.contains("spec-00001-test-spec.md"));
    assert!(output.prompt.contains("Type: spec"));
    assert!(output.prompt.contains("Code: 00001"));
    assert!(output.prompt.contains("Slug: test-spec"));
}

#[tokio::test]
async fn test_create_doc_fails_when_prompt_document_is_missing() {
    let (temp_dir, root) = create_test_project_with_prompt();
    fs::remove_file(
        temp_dir
            .path()
            .join("doc")
            .join("prompts")
            .join("authoring")
            .join("prompts-00001-create-rfc.md"),
    )
    .unwrap();

    let input = CreateDocInput {
        root_dir: root,
        doc_type: "rfc".to_string(),
        category: None,
        name: "Test".to_string(),
        slug: "missing-prompt-doc".to_string(),
    };

    let mut sender = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    let result = op.run(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_doc_increments_code() {
    let (_temp_dir, root) = create_test_project_with_prompt();

    let input1 = CreateDocInput {
        root_dir: root.clone(),
        doc_type: "rfc".to_string(),
        category: None,
        name: "First RFC".to_string(),
        slug: "first-rfc".to_string(),
    };

    let mut sender1 = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    let result1 = op.run(input1, &mut sender1).await;
    assert!(result1.is_ok());
    assert_eq!(sender1.into_output().expect("first output").code, "00002");

    let input2 = CreateDocInput {
        root_dir: root,
        doc_type: "rfc".to_string(),
        category: None,
        name: "Second RFC".to_string(),
        slug: "second-rfc".to_string(),
    };

    let mut sender2 = CapturingSender::<CreateDocOutput>::new();
    let result2 = op.run(input2, &mut sender2).await;
    assert!(result2.is_ok());
    assert_eq!(sender2.into_output().expect("second output").code, "00003");
}

#[tokio::test]
async fn test_create_doc_rejects_slug_starting_with_hyphen() {
    let (_temp_dir, root) = create_test_project_with_prompt();

    let input = CreateDocInput {
        root_dir: root,
        doc_type: "rfc".to_string(),
        category: None,
        name: "Test".to_string(),
        slug: "-invalid-start".to_string(),
    };

    let mut sender = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    let result = op.run(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_doc_rejects_empty_slug() {
    let (_temp_dir, root) = create_test_project_with_prompt();

    let input = CreateDocInput {
        root_dir: root,
        doc_type: "rfc".to_string(),
        category: None,
        name: "Test".to_string(),
        slug: String::new(),
    };

    let mut sender = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    let result = op.run(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_doc_directory_based() {
    let (temp_dir, root) = create_test_project_no_prompt();

    // Add directory based type to config with explicit prompt that exists in this helper
    let config_path = temp_dir.path().join(".vector").join("document-types.yaml");
    let mut config_content = fs::read_to_string(&config_path).unwrap();
    config_content.push_str("\n  research:\n    layout: directory\n    code-width: 5\n    prompt: prompts-00002-create-doc\n");
    fs::write(&config_path, config_content).unwrap();

    let input = CreateDocInput {
        root_dir: root.clone(),
        doc_type: "research".to_string(),
        category: None,
        name: "Study".to_string(),
        slug: "study-results".to_string(),
    };

    let mut sender = CapturingSender::<CreateDocOutput>::new();
    let op = CreateDocOp;
    op.run(input, &mut sender).await.unwrap();

    let output = sender.into_output().expect("Output should be sent");
    assert_eq!(output.code, "00001");
    assert!(output.prompt.contains("study-results"));
    assert!(output.prompt.contains("research"));

    let expected_path =
        temp_dir.path().join("doc").join("research").join("research-00001-study-results.md");
    assert!(expected_path.exists(), "Document should exist at {}", expected_path.display());
}
