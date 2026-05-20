#![allow(clippy::unwrap_used, clippy::expect_used)]

use runtime_core::FlowOperation;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

use crate::operations::BootstrapDocTypeOp;
use crate::operations::support::CapturingSender;
use crate::operations::{BootstrapDocTypeInput, BootstrapDocTypeOutput};

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
    prompt: prompts-00001-create-rfc
    initial-status: draft
    statuses:
      - draft
      - review
      - accepted
  template:
    layout: category
    code-width: 5
    prompt: prompts-00009-create-template
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    let doc_dir = temp_dir.path().join("doc");
    fs::create_dir_all(&doc_dir).unwrap();
    let ai_template_dir = doc_dir.join("template").join("ai");
    fs::create_dir_all(&ai_template_dir).unwrap();
    fs::write(
        ai_template_dir.join("template-00006-documentation.md"),
        "---\ncreated: <YYYY-MM-DD>\nupdated: <YYYY-MM-DD>\n---\n\n#{types}\n",
    )
    .unwrap();

    (temp_dir, root)
}

#[tokio::test]
async fn test_bootstrap_doc_type_status_based() {
    let (temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "task".to_string(),
        description: None,
        tags: None,
        prompt: Some("prompts-00002-create-task".to_string()),
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string(), "in_progress".to_string(), "done".to_string()]),
        template: Some("template-task".to_string()),
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
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
}

#[tokio::test]
async fn test_bootstrap_doc_type_category_based() {
    let (temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "spec".to_string(),
        description: None,
        tags: None,
        prompt: Some("prompts-00003-create-spec".to_string()),
        layout: "category".to_string(),
        code_width: 5,
        statuses: None,
        template: Some("template-spec".to_string()),
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    op.run(input, &mut sender).await.unwrap();

    let spec_dir = temp_dir.path().join("doc").join("spec");
    assert!(spec_dir.exists(), "Spec directory should exist");

    let config_content =
        fs::read_to_string(temp_dir.path().join(".vector").join("document-types.yaml")).unwrap();
    assert!(config_content.contains("layout: category"), "Config should have category layout");
}

#[tokio::test]
async fn test_bootstrap_doc_type_preserves_metadata() {
    let (temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "meta".to_string(),
        description: Some("My description".to_string()),
        tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        prompt: Some("my-prompt".to_string()),
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["draft".to_string()]),
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    op.run(input, &mut sender).await.unwrap();

    let config_content =
        fs::read_to_string(temp_dir.path().join(".vector").join("document-types.yaml")).unwrap();
    assert!(config_content.contains("meta:"), "Config should contain meta type");
    assert!(config_content.contains("description: My description"), "Should preserve description");
    assert!(config_content.contains("- tag1"), "Should preserve tags");
    assert!(config_content.contains("- tag2"), "Should preserve tags");
    assert!(config_content.contains("prompt: my-prompt"), "Should preserve prompt");
}

#[tokio::test]
async fn test_bootstrap_doc_type_invalid_name_empty() {
    let (_temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: String::new(),
        description: None,
        tags: None,
        prompt: Some("prompts-00002-create-task".to_string()),
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with empty document type name");
}

#[tokio::test]
async fn test_bootstrap_doc_type_invalid_name_uppercase() {
    let (_temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "InvalidType".to_string(),
        description: None,
        tags: None,
        prompt: Some("prompts-00002-create-task".to_string()),
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with uppercase in document type name");
}

#[tokio::test]
async fn test_bootstrap_doc_type_invalid_layout() {
    let (_temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "newtype".to_string(),
        description: None,
        tags: None,
        prompt: Some("prompts-00004-create-newtype".to_string()),
        layout: "invalid".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with invalid layout");
}

#[tokio::test]
async fn test_bootstrap_doc_type_status_based_missing_statuses() {
    let (_temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "newtype".to_string(),
        description: None,
        tags: None,
        prompt: Some("prompts-00004-create-newtype".to_string()),
        layout: "status".to_string(),
        code_width: 5,
        statuses: None,
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail when status-based type has no statuses");
}

// test_bootstrap_doc_type_category_based_missing_categories removed as categories are no longer supported in input

#[tokio::test]
async fn test_bootstrap_doc_type_creates_template() {
    let (temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "design".to_string(),
        description: None,
        tags: None,
        prompt: Some("prompts-00005-create-design".to_string()),
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["draft".to_string(), "review".to_string()]),
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    op.run(input, &mut sender).await.unwrap();

    let template_dir = temp_dir.path().join("doc").join("template").join("doc");
    let mut entries = fs::read_dir(template_dir).unwrap();
    let has_design_template =
        entries.any(|e| e.unwrap().file_name().to_str().unwrap().contains("design"));
    assert!(has_design_template, "Should create a template for the new document type");
}

#[tokio::test]
async fn test_bootstrap_doc_type_consecutive_hyphens() {
    let (_temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "my--invalid".to_string(),
        description: None,
        tags: None,
        prompt: Some("prompts-00006-create-invalid".to_string()),
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail with consecutive hyphens in name");
}

#[tokio::test]
async fn test_bootstrap_doc_type_hyphen_start_end() {
    let (_temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "-startswith".to_string(),
        description: None,
        tags: None,
        prompt: Some("prompts-00006-create-invalid".to_string()),
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail when name starts with hyphen");
}

#[tokio::test]
async fn test_bootstrap_doc_type_requires_prompt() {
    let (_temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
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

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail when prompt is missing");
}

#[tokio::test]
async fn test_bootstrap_doc_type_fails_when_rule_regeneration_fails() {
    let (temp_dir, root) = create_base_project();
    fs::remove_file(
        temp_dir
            .path()
            .join("doc")
            .join("template")
            .join("ai")
            .join("template-00006-documentation.md"),
    )
    .unwrap();

    let input = BootstrapDocTypeInput {
        root_dir: root,
        doc_type: "task".to_string(),
        description: None,
        tags: None,
        prompt: Some("prompts-00002-create-task".to_string()),
        layout: "status".to_string(),
        code_width: 5,
        statuses: Some(vec!["todo".to_string()]),
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    let result = op.run(input, &mut sender).await;
    assert!(result.is_err(), "Should fail when documentation rule regeneration fails");
}

#[tokio::test]
async fn test_bootstrap_doc_type_directory_based() {
    let (temp_dir, root) = create_base_project();

    let input = BootstrapDocTypeInput {
        root_dir: root.clone(),
        doc_type: "research".to_string(),
        description: Some("Research papers".to_string()),
        tags: Some(vec!["science".to_string()]),
        prompt: Some("prompts-00004-create-research".to_string()),
        layout: "directory".to_string(),
        code_width: 5,
        statuses: None,
        template: None,
    };

    let mut sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    let op = BootstrapDocTypeOp;
    op.run(input, &mut sender).await.unwrap();

    let research_dir = temp_dir.path().join("doc").join("research");
    assert!(research_dir.exists(), "Research directory should exist");

    // Ensure no status subfolders are created
    let entries: Vec<_> = fs::read_dir(&research_dir).unwrap().collect();
    assert_eq!(entries.len(), 0, "Research directory should be empty");

    let config_path = temp_dir.path().join(".vector").join("document-types.yaml");
    let config_content = fs::read_to_string(config_path).unwrap();
    assert!(config_content.contains("research:"), "Config should contain research type");
    assert!(config_content.contains("layout: directory"), "Config should have layout: directory");

    // Check that research entry does not have statuses
    let research_part = config_content.split("research:").nth(1).unwrap();
    assert!(
        !research_part.contains("statuses:"),
        "Directory layout should not have statuses in its config entry"
    );
}
