#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_parse_stem_valid() {
    let (doc_type, code, slug) = parse_stem("rfc-00013-runtime-doc-validation").unwrap();
    assert_eq!(doc_type, "rfc");
    assert_eq!(code, 13);
    assert_eq!(slug, "runtime-doc-validation");
}

#[test]
fn test_parse_stem_with_hyphen_in_slug() {
    let (doc_type, code, slug) = parse_stem("task-00001-my-task-slug").unwrap();
    assert_eq!(doc_type, "task");
    assert_eq!(code, 1);
    assert_eq!(slug, "my-task-slug");
}

#[test]
fn test_parse_stem_invalid_too_few_parts() {
    assert!(parse_stem("rfc-00013").is_none());
    assert!(parse_stem("rfc").is_none());
}

#[test]
fn test_parse_stem_invalid_code() {
    assert!(parse_stem("rfc-abc-slug").is_none());
}

fn create_test_env(root: &TempDir) -> (IoPath, std::path::PathBuf) {
    let vector_dir = root.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft, proposed, accepted, rejected]\n  task:\n    layout: category\n    code-width: 5\n    prompt: prompts-00002-create-task",
    ).unwrap();

    let doc_dir = root.path().join("doc");
    let rfc_dir = doc_dir.join("rfc").join("draft");
    fs::create_dir_all(&rfc_dir).unwrap();

    let target_file = rfc_dir.join("rfc-00013-my-rfc.md");
    fs::write(&target_file, "# RFC 13\n").unwrap();

    (IoPath::new(root.path()), target_file)
}

#[tokio::test]
async fn test_locate_file_by_stem_success() {
    let temp = TempDir::new().unwrap();
    let (root, expected_path) = create_test_env(&temp);

    let result = locate_file_by_stem("rfc-00013-my-rfc", &root).await;

    assert!(result.is_ok());
    let found = result.unwrap();
    let expected = expected_path.canonicalize().unwrap();
    assert_eq!(found, expected);
}

#[tokio::test]
async fn test_locate_file_by_stem_not_found() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft]",
    )
    .unwrap();

    let doc_dir = temp.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&doc_dir).unwrap();

    let result = locate_file_by_stem("rfc-00099-nonexistent", &root).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.stem, "rfc-00099-nonexistent");
    assert!(err.reason.contains("No file found with matching stem"));
}

#[test]
fn test_locate_file_by_stem_invalid_stem_format() {
    let result = parse_stem("invalid-stem");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_locate_file_by_stem_rejects_invalid_stem_format() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    let result = locate_file_by_stem("invalid-stem", &root).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.stem, "invalid-stem");
    assert!(err.reason.contains("expected pattern"));
}

#[tokio::test]
async fn test_locate_file_by_stem_unknown_doc_type() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft]",
    )
    .unwrap();

    let result = locate_file_by_stem("unknown-00001-slug", &root).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.stem, "unknown-00001-slug");
    assert!(err.reason.contains("Unknown document type"));
}

#[tokio::test]
async fn test_locate_file_by_stem_missing_folder() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft]",
    )
    .unwrap();

    let result = locate_file_by_stem("rfc-00013-slug", &root).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.reason.contains("does not exist"));
}

#[tokio::test]
async fn test_locate_file_by_stem_missing_config() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    let result = locate_file_by_stem("rfc-00013-slug", &root).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.stem, "rfc-00013-slug");
    assert!(err.reason.contains("Failed to load document types configuration"));
}

#[tokio::test]
async fn test_locate_file_by_stem_multiple_subfolders() {
    let temp = TempDir::new().unwrap();
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft, proposed]",
    ).unwrap();

    let doc_dir = temp.path().join("doc");
    let draft_dir = doc_dir.join("rfc").join("draft");
    let proposed_dir = doc_dir.join("rfc").join("proposed");
    fs::create_dir_all(&draft_dir).unwrap();
    fs::create_dir_all(&proposed_dir).unwrap();

    fs::write(draft_dir.join("rfc-00001-draft-slug.md"), "# Draft").unwrap();
    fs::write(proposed_dir.join("rfc-00002-proposed-slug.md"), "# Proposed").unwrap();

    let root = IoPath::new(temp.path());

    let result = locate_file_by_stem("rfc-00002-proposed-slug", &root).await;
    assert!(result.is_ok());

    let result2 = locate_file_by_stem("rfc-00001-draft-slug", &root).await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_locate_file_by_stem_ignores_non_markdown_files() {
    let temp = TempDir::new().unwrap();
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft]",
    )
    .unwrap();

    let draft_dir = temp.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();
    fs::write(draft_dir.join("rfc-00013-my-rfc.txt"), "not markdown").unwrap();

    let root = IoPath::new(temp.path());
    let result = locate_file_by_stem("rfc-00013-my-rfc", &root).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.reason.contains("No file found with matching stem"));
}

#[test]
fn test_locate_error_debug_and_display() {
    let err = LocateError {
        stem: "rfc-00013-my-rfc".to_string(),
        reason: "No file found with matching stem".to_string(),
    };

    let debug = format!("{err:?}");
    let display = format!("{err}");

    assert!(debug.contains("LocateError"));
    assert!(debug.contains("rfc-00013-my-rfc"));
    assert!(display.contains("Cannot locate file with stem 'rfc-00013-my-rfc'"));
    assert!(display.contains("No file found with matching stem"));
}

#[test]
fn test_locate_error_implements_std_error() {
    let err = LocateError {
        stem: "rfc-00013-my-rfc".to_string(),
        reason: "No file found with matching stem".to_string(),
    };

    let source = std::error::Error::source(&err);
    assert!(source.is_none());
}
