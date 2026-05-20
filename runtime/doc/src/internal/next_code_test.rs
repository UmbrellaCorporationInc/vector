#![allow(clippy::unwrap_used)]

use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

use crate::internal::next_code::next_code_for;

fn create_test_project() -> (TempDir, IoPath) {
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
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    let doc_dir = temp_dir.path().join("doc");
    fs::create_dir_all(&doc_dir).unwrap();

    (temp_dir, root)
}

#[tokio::test]
async fn test_next_code_returns_one_when_no_files() {
    let (_temp_dir, root) = create_test_project();

    let result = next_code_for("rfc", &root).await.unwrap();
    assert_eq!(result.next_code, 1);
}

#[tokio::test]
async fn test_next_code_finds_highest_existing() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    fs::write(
        draft_dir.join("rfc-00001-test-rfc.md"),
        "---\nid: rfc-00001-test\ntype: rfc\ncode: \"00001\"\nslug: test-rfc\ntitle: Test RFC\ndescription: Test\ncreated: 2026-01-01\ntags: []\nstatus: draft\n---\n\n# Test\n",
    )
    .unwrap();
    fs::write(
        draft_dir.join("rfc-00002-another-rfc.md"),
        "---\nid: rfc-00002-another\ntype: rfc\ncode: \"00002\"\nslug: another-rfc\ntitle: Another RFC\ndescription: Test\ncreated: 2026-01-01\ntags: []\nstatus: draft\n---\n\n# Another\n",
    )
    .unwrap();

    let result = next_code_for("rfc", &root).await.unwrap();
    assert_eq!(result.next_code, 3);
}

#[tokio::test]
async fn test_next_code_scans_all_subfolders() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();
    let review_dir = temp_dir.path().join("doc").join("rfc").join("review");
    fs::create_dir_all(&review_dir).unwrap();

    fs::write(
        draft_dir.join("rfc-00001-test-rfc.md"),
        "---\nid: rfc-00001-test\ntype: rfc\ncode: \"00001\"\nslug: test-rfc\ntitle: Test RFC\ndescription: Test\ncreated: 2026-01-01\ntags: []\nstatus: draft\n---\n\n# Test\n",
    )
    .unwrap();
    fs::write(
        review_dir.join("rfc-00005-review-rfc.md"),
        "---\nid: rfc-00005-review\ntype: rfc\ncode: \"00005\"\nslug: review-rfc\ntitle: Review RFC\ndescription: Test\ncreated: 2026-01-01\ntags: []\nstatus: review\n---\n\n# Review\n",
    )
    .unwrap();

    let result = next_code_for("rfc", &root).await.unwrap();
    assert_eq!(result.next_code, 6);
}

#[tokio::test]
async fn test_next_code_unknown_type_returns_error() {
    let (_temp_dir, root) = create_test_project();

    let result = next_code_for("unknown_type", &root).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_next_code_returns_error_for_malformed_filename() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    fs::write(
        draft_dir.join("rfc-00001-test-rfc.md"),
        "---\nid: rfc-00001-test\ntype: rfc\ncode: \"00001\"\nslug: test-rfc\ntitle: Test RFC\ndescription: Test\ncreated: 2026-01-01\ntags: []\nstatus: draft\n---\n\n# Test\n",
    )
    .unwrap();
    fs::write(
        draft_dir.join("malformed-filename.md"),
        "---\nid: something\ntype: rfc\ncode: \"99999\"\nslug: something\ntitle: Something\ndescription: Test\ncreated: 2026-01-01\ntags: []\nstatus: draft\n---\n\n# Something\n",
    )
    .unwrap();

    let result = next_code_for("rfc", &root).await;
    assert!(result.is_err());
}
