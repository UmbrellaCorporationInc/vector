#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_core::FlowOperation;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

use crate::operations::support::CapturingSender;
use crate::operations::{CreateDocTypeInput, CreateDocTypeOutput};

type MockSender = CapturingSender<CreateDocTypeOutput>;

fn create_base_project() -> (TempDir, IoPath) {
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
    prompt: prompts-00002-create-rfc
    initial-status: draft
    statuses:
      - draft
      - review
      - accepted
  prompts:
    template: template-00003-prompts
    layout: category
    code-width: 5
    prompt: prompts-00003-create-prompts
  template:
    layout: category
    code-width: 5
    prompt: prompts-00005-create-template
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    let doc_dir = temp_dir.path().join("doc");
    let prompts_dir = doc_dir.join("prompts").join("doc-type");
    fs::create_dir_all(&prompts_dir).unwrap();

    let prompt_content = "---
id: prompts-00001-create-doc-type
type: prompts
code: \"00001\"
slug: create-doc-type
title: Create Document Type
description: Governed prompt for creating a new document type.
category: doc-type
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
tags: []
---

# Prompt: Create Document Type

You are creating a new document type: `#{doc-type}`

## Layout
The document type uses the `#{layout}` layout.
";
    fs::write(prompts_dir.join("prompts-00001-create-doc-type.md"), prompt_content).unwrap();

    let template_dir = doc_dir.join("template").join("project");
    fs::create_dir_all(&template_dir).unwrap();
    let ai_template_dir = doc_dir.join("template").join("ai");
    fs::create_dir_all(&ai_template_dir).unwrap();
    fs::write(
        ai_template_dir.join("template-00006-documentation.md"),
        "---\ncreated: <YYYY-MM-DD>\nupdated: <YYYY-MM-DD>\n---\n\n#{types}\n",
    )
    .unwrap();

    let template_content = "---
id: doc-type-00001-<slug>
type: doc-type
code: \"00001\"
slug: <slug>
title: <Title>
description: <One sentence describing this document type.>
layout: <status | category>
code-width: <number>
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
tags: []
---
";
    fs::write(template_dir.join("template-00004-doc-type-template.md"), template_content).unwrap();

    let prompt_template_content = "---
id: doc-type-prompt-00001-<slug>
type: doc-type-prompt
code: \"00001\"
slug: <slug>
title: <Title>
description: <One sentence describing this prompt template.>
category: <Category>
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
tags: []
---
";
    fs::write(template_dir.join("template-00005-doc-type-prompt.md"), prompt_template_content)
        .unwrap();

    (temp_dir, root)
}

fn create_project_without_doc_type_definition() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let config = "document-types:
  rfc:
    template: template-00001-rfc
    layout: status
    code-width: 5
    prompt: prompts-00002-create-rfc
    initial-status: draft
    statuses:
      - draft
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    (temp_dir, root)
}

fn create_project_with_doc_type_missing_prompt() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let config = "doc-type:
  template: template-00004-doc-type-template
  prompt-template: template-00005-doc-type-prompt
document-types:
  rfc:
    layout: status
    code_width: 5
    prompt: prompts-00002-create-rfc
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    (temp_dir, root)
}

#[test]
fn test_create_prompt_template_content_contains_expected_fields() {
    let content = create_prompt_template_content("design");

    assert!(content.contains("id: doc-type-prompt-00001-design"));
    assert!(content.contains("type: doc-type-prompt"));
    assert!(content.contains("slug: design"));
    assert!(content.contains("category: design"));
}

#[test]
fn test_resolve_create_doc_type_placeholders_replaces_all_tokens() {
    let prompt = "Type: #{doc-type}\nLayout: #{layout}\nRepeat #{doc-type}";
    let resolved = resolve_create_doc_type_placeholders(prompt, "spec", "category");

    assert_eq!(resolved, "Type: spec\nLayout: category\nRepeat spec");
}

#[tokio::test]
async fn test_create_doc_type_status_based() {
    let (temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "task".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string(), "in_progress".to_string(), "done".to_string()]),
        template: Some("template-task".to_string()),
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    op.run(input, &mut sender).await.unwrap();

    let task_dir = temp_dir.path().join("doc").join("task");
    assert!(task_dir.exists(), "Task directory should exist");

    let todo_dir = task_dir.join("todo");
    assert!(todo_dir.exists(), "Todo subdirectory should exist");

    let in_progress_dir = task_dir.join("in_progress");
    assert!(in_progress_dir.exists(), "In_progress subdirectory should exist");

    let done_dir = task_dir.join("done");
    assert!(done_dir.exists(), "Done subdirectory should exist");

    let config_content =
        fs::read_to_string(temp_dir.path().join(".vector").join("document-types.yaml")).unwrap();
    assert!(config_content.contains("task:"), "Config should contain task type");
    assert!(config_content.contains("layout: status"), "Config should have status layout");

    let output = sender.into_output().expect("output should exist");
    assert_eq!(output.doc_type, "task");
    assert_eq!(output.layout, "status");
    assert!(output.prompt.contains("task"));
    assert!(output.prompt.contains("status"));
}

#[tokio::test]
async fn test_create_doc_type_category_based() {
    let (temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "spec".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "category".to_string(),
        code_width: 5,
        statuses: None,
        template: Some("template-spec".to_string()),
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    op.run(input, &mut sender).await.unwrap();

    let spec_dir = temp_dir.path().join("doc").join("spec");
    assert!(spec_dir.exists(), "Spec directory should exist");

    let config_content =
        fs::read_to_string(temp_dir.path().join(".vector").join("document-types.yaml")).unwrap();
    assert!(config_content.contains("spec:"), "Config should contain spec type");
    assert!(config_content.contains("layout: category"), "Config should have category layout");

    let output = sender.into_output().expect("output should exist");
    assert_eq!(output.doc_type, "spec");
    assert_eq!(output.layout, "category");
    assert!(output.prompt.contains("spec"));
    assert!(output.prompt.contains("category"));
}

#[tokio::test]
async fn test_create_doc_type_invalid_name_empty() {
    let (_temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: String::new(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with empty document type name");
}

#[tokio::test]
async fn test_create_doc_type_missing_doc_type_definition() {
    let (_temp_dir, root) = create_project_without_doc_type_definition();

    let input = CreateDocTypeInput {
        root_dir: root,
        doc_type: "task".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail when doc_type config is missing");
}

#[tokio::test]
async fn test_create_doc_type_missing_prompt_field_on_doc_type_definition() {
    let (_temp_dir, root) = create_project_with_doc_type_missing_prompt();

    let input = CreateDocTypeInput {
        root_dir: root,
        doc_type: "task".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail when doc_type prompt field is missing");
}

#[tokio::test]
async fn test_create_doc_type_invalid_name_uppercase() {
    let (_temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "InvalidType".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with uppercase in document type name");
}

#[tokio::test]
async fn test_create_doc_type_invalid_layout() {
    let (_temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "newtype".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "invalid".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with invalid layout");
}

#[tokio::test]
async fn test_create_doc_type_status_based_missing_statuses() {
    let (_temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "newtype".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: None,
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail when status-based type has no statuses");
}

// test_create_doc_type_category_based_missing_categories removed as categories are no longer supported in input

#[tokio::test]
async fn test_create_doc_type_creates_template_and_prompt_template() {
    let (temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "design".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["draft".to_string(), "review".to_string()]),
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    op.run(input, &mut sender).await.unwrap();

    let template_dir = temp_dir.path().join("doc").join("template").join("doc");

    let mut entries = fs::read_dir(&template_dir).unwrap();
    let has_design_template =
        entries.any(|e| e.unwrap().file_name().to_str().unwrap().contains("design"));
    assert!(has_design_template, "Should create a template for the new document type");

    let prompt_template_path = template_dir.join("doc-type-prompt-design.md");
    assert!(
        prompt_template_path.exists(),
        "Should create a prompt template for the new document type"
    );

    let config_content =
        fs::read_to_string(temp_dir.path().join(".vector").join("document-types.yaml")).unwrap();
    assert!(
        config_content.contains("prompt: doc-type-prompt-design"),
        "Config should have prompt field set"
    );
}

#[tokio::test]
async fn test_create_doc_type_consecutive_hyphens() {
    let (_temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "my--invalid".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with consecutive hyphens in name");
}

#[tokio::test]
async fn test_create_doc_type_hyphen_start_end() {
    let (_temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "-startswith".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail when name starts with hyphen");
}

#[tokio::test]
async fn test_create_doc_type_resolves_placeholders() {
    let (_temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "mydoc".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "category".to_string(),
        code_width: 5,
        statuses: None,
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    op.run(input, &mut sender).await.unwrap();

    let output = sender.into_output().expect("output should exist");
    assert!(output.prompt.contains("mydoc"), "Prompt should contain doc_type");
    assert!(output.prompt.contains("category"), "Prompt should contain layout");
    assert!(
        !output.prompt.contains("#{doc-type}"),
        "Prompt should not have unreplaced doc-type placeholder"
    );
    assert!(
        !output.prompt.contains("#{layout}"),
        "Prompt should not have unreplaced layout placeholder"
    );
}

#[tokio::test]
async fn test_create_doc_type_fails_when_prompt_document_is_missing() {
    let (temp_dir, root) = create_base_project();
    fs::remove_file(
        temp_dir
            .path()
            .join("doc")
            .join("prompts")
            .join("doc-type")
            .join("prompts-00001-create-doc-type.md"),
    )
    .unwrap();

    let input = CreateDocTypeInput {
        root_dir: root,
        doc_type: "task".to_string(),
        description: None,
        tags: None,
        prompt: None,
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string(), "done".to_string()]),
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    let result = op.run(input, &mut sender).await;

    assert!(result.is_err(), "Should fail when the configured prompt document is missing");
}

#[tokio::test]
async fn test_create_doc_type_directory_based() {
    let (temp_dir, root) = create_base_project();

    let input = CreateDocTypeInput {
        root_dir: root.clone(),
        doc_type: "research".to_string(),
        description: Some("Research papers".to_string()),
        tags: Some(vec!["science".to_string()]),
        prompt: None,
        layout: "directory".to_string(),
        code_width: 5,
        statuses: None,
        template: None,
    };

    let mut sender = MockSender::new();
    let op = CreateDocTypeOp;
    op.run(input, &mut sender).await.unwrap();

    let output = sender.into_output().expect("Output should be sent");
    assert_eq!(output.doc_type, "research");
    assert_eq!(output.layout, "directory");
    assert!(output.prompt.contains("The document type uses the `directory` layout."));

    let config_path = temp_dir.path().join(".vector").join("document-types.yaml");
    let config_content = fs::read_to_string(config_path).unwrap();
    assert!(config_content.contains("research:"));
    assert!(config_content.contains("layout: directory"));
}
