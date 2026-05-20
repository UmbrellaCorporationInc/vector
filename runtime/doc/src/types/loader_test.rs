#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Tests for document types configuration loader.

use runtime_io::path::IoPath;

fn temp_config_file(content: &str) -> (tempfile::TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().expect("tempdir should create");
    let path = temp_dir.path().to_path_buf();
    let config_path = path.join(".vector").join("document-types.yaml");
    std::fs::create_dir_all(config_path.parent().expect("parent should exist"))
        .expect("dir creation should succeed");
    std::fs::write(&config_path, content).expect("file write should succeed");
    (temp_dir, IoPath::new(path))
}

fn temp_explicit_config_file(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp_dir = tempfile::tempdir().expect("tempdir should create");
    let config_path = temp_dir.path().join("custom-document-types.yaml");
    std::fs::write(&config_path, content).expect("file write should succeed");
    (temp_dir, config_path)
}

#[tokio::test]
async fn test_load_valid_config() {
    let content = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  rfc:
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    statuses:
      - draft
      - accepted
";
    let (_dir, root) = temp_config_file(content);
    let config =
        crate::types::load_document_types_config(&root).await.expect("load should succeed");
    assert_eq!(config.document_types.len(), 1);
    assert_eq!(config.document_types["rfc"].prompt, "prompts-00001-create-rfc");
}

#[tokio::test]
async fn test_load_missing_file() {
    let temp_dir = tempfile::tempdir().expect("tempdir should create");
    let root = IoPath::new(temp_dir.path());
    let result = crate::types::load_document_types_config(&root).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_load_malformed_yaml() {
    let content = "not: valid: yaml: [";
    let (_dir, root) = temp_config_file(content);
    let result = crate::types::load_document_types_config(&root).await;
    assert!(result.is_err());
}
#[tokio::test]
async fn test_load_config_with_prompt() {
    let content = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  rfc:
    layout: status
    code-width: 5
    prompt: rfc-prompt
";
    let (_dir, root) = temp_config_file(content);
    let config =
        crate::types::load_document_types_config(&root).await.expect("load should succeed");
    let rfc = config.document_types.get("rfc").unwrap();
    assert_eq!(rfc.prompt, "rfc-prompt");
}

#[tokio::test]
async fn test_load_rejects_filename_pattern() {
    let content = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  rfc:
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    filename_pattern: \"{type}-{code}-{slug}.md\"
    statuses:
      - draft
";
    let (_dir, root) = temp_config_file(content);
    let result = crate::types::load_document_types_config(&root).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_load_from_path_success() {
    let content = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  spec:
    layout: category
    code-width: 5
    prompt: prompts-00002-create-spec
";
    let (_dir, config_path) = temp_explicit_config_file(content);
    let config = crate::types::load_from_path(&config_path).await.expect("load should succeed");
    assert!(config.document_types.contains_key("spec"));
}

#[tokio::test]
async fn test_load_from_path_missing_file() {
    let temp_dir = tempfile::tempdir().expect("tempdir should create");
    let missing_path = temp_dir.path().join("missing-document-types.yaml");
    let result = crate::types::load_from_path(&missing_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_load_from_path_rejects_filename_pattern() {
    let content = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  task:
    layout: status
    code-width: 5
    prompt: prompts-00003-create-task
    filename_pattern: \"{type}-{code}-{slug}.md\"
    statuses:
      - todo
";
    let (_dir, config_path) = temp_explicit_config_file(content);
    let result = crate::types::load_from_path(&config_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_load_defaults_missing_document_type_prompt_to_empty() {
    let content = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  rfc:
    layout: status
    code-width: 5
    statuses:
      - draft
";
    let (_dir, root) = temp_config_file(content);
    let config =
        crate::types::load_document_types_config(&root).await.expect("load should succeed");
    assert_eq!(config.document_types["rfc"].prompt, "");
}

#[tokio::test]
async fn test_load_rejects_snake_case_schema_fields() {
    let content = "doc-type: {template: t, prompt_template: pt, prompt: p}
document-types:
  rfc:
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    create-document-form: form-00001-create-document
    statuses:
      - draft
";
    let (_dir, root) = temp_config_file(content);
    let result = crate::types::load_document_types_config(&root).await;
    let error = result.expect_err("snake_case field must be rejected");
    assert!(error.to_string().contains("prompt_template"));
}
