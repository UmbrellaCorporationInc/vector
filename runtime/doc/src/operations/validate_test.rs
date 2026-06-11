#![allow(clippy::unwrap_used, clippy::expect_used)]

use runtime_core::FlowOperation;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

use crate::operations::validate::{Utf8ValidationError, check_utf8_without_bom};
use crate::operations::{ValidateInput, ValidateOutput};

struct MockSender {
    output: Option<ValidateOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { output: None }
    }
}

impl runtime_core::Sender<ValidateOutput> for MockSender {
    async fn send(&mut self, value: ValidateOutput) -> runtime_core::RuntimeResult<()> {
        self.output = Some(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<ValidateOutput> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

async fn run_validate(root: &IoPath, fix: bool) -> ValidateOutput {
    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root.clone(), fix }, &mut sender)
        .await
        .unwrap();
    sender.output.unwrap()
}

fn create_test_project() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let config = "doc-type:
  template: template-00004-doc-type-template
  prompt-template: template-00005-doc-type-prompt
  prompt: prompts-00001-create-doc-type
  create-document-type-form: form-00002-create-document-type
document-types:
  rfc:
    template: template-00001-rfc
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    create-document-form: form-00001-create-document
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
    create-document-form: form-00001-create-document
  task:
    template: template-task
    layout: status
    code-width: 5
    prompt: prompts-00003-create-task
    create-document-form: form-00001-create-document
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
    create-document-form: form-00001-create-document
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    let doc_dir = temp_dir.path().join("doc");
    fs::create_dir_all(&doc_dir).unwrap();

    (temp_dir, root)
}

fn create_valid_rfc_doc() -> String {
    "---\n\
id: rfc-00001-test\n\
type: rfc\n\
code: \"00001\"\n\
slug: test-rfc\n\
title: Test RFC\n\
description: A test RFC document\n\
created: 2026-01-01\n\
tags:\n  - test\n\
status: draft\n\
---\n\n\
# Test RFC
"
    .to_string()
}

#[tokio::test]
async fn test_validate_with_valid_document() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let valid_doc = create_valid_rfc_doc();
    fs::write(draft_dir.join("rfc-00001-test-rfc.md"), &valid_doc).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(output.valid, "Expected valid=true, got errors: {:?}", output.errors);
}

#[tokio::test]
async fn test_validate_with_missing_frontmatter() {
    let (temp_dir, root) = create_test_project();

    let rfc_dir = temp_dir.path().join("doc").join("rfc");
    fs::create_dir_all(&rfc_dir).unwrap();

    let invalid_doc = "# Missing frontmatter\n\nSome content\n";
    fs::write(rfc_dir.join("rfc-00001-test.md"), invalid_doc).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for missing frontmatter");
    assert!(!output.errors.is_empty(), "Expected at least one error");
}

#[tokio::test]
async fn test_validate_with_invalid_status() {
    let (temp_dir, root) = create_test_project();

    let rfc_dir = temp_dir.path().join("doc").join("rfc");
    fs::create_dir_all(&rfc_dir).unwrap();

    let mut invalid_doc = create_valid_rfc_doc();
    invalid_doc = invalid_doc.replace("status: draft", "status: invalid-status");
    fs::write(rfc_dir.join("rfc-00001-test.md"), &invalid_doc).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for invalid status");
    assert!(output.errors.iter().any(|e| e.error.contains("Invalid status")));
}

#[tokio::test]
async fn test_validate_with_wrong_folder_placement() {
    let (temp_dir, root) = create_test_project();

    let rfc_dir = temp_dir.path().join("doc").join("rfc");
    fs::create_dir_all(&rfc_dir).unwrap();

    let doc_in_wrong_folder = rfc_dir.join("review");
    fs::create_dir_all(&doc_in_wrong_folder).unwrap();

    let invalid_doc = create_valid_rfc_doc();
    fs::write(doc_in_wrong_folder.join("rfc-00001-test.md"), &invalid_doc).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for wrong folder placement");
}

#[tokio::test]
async fn test_validate_with_wikilink_having_md_extension() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let mut doc_with_bad_wikilink = create_valid_rfc_doc();
    doc_with_bad_wikilink.push_str("\nSee [[other-doc.md]] for details.\n");
    fs::write(draft_dir.join("rfc-00001-test-rfc.md"), &doc_with_bad_wikilink).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for wikilink with .md extension");
    assert!(output.errors.iter().any(|e| e.error.contains("Wikilink")));
}

#[tokio::test]
async fn test_validate_with_missing_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for missing config");
    assert!(output.errors.iter().any(|e| e.path.contains("document-types.yaml")));
}

#[tokio::test]
async fn test_validate_with_utf8_bom() {
    let (temp_dir, root) = create_test_project();

    let rfc_dir = temp_dir.path().join("doc").join("rfc");
    fs::create_dir_all(&rfc_dir).unwrap();

    let valid_content = create_valid_rfc_doc();
    let bom = [0xEF, 0xBB, 0xBF];
    let mut content_with_bom = Vec::from(bom);
    content_with_bom.extend(valid_content.as_bytes());
    fs::write(rfc_dir.join("rfc-00001-test.md"), content_with_bom).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for UTF-8 BOM");
    assert!(output.errors.iter().any(|e| e.error.contains("BOM")));
}

#[test]
fn test_check_utf8_without_bom_returns_typed_bom_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bom.md");
    fs::write(&path, [0xEF, 0xBB, 0xBF, b'h', b'i']).unwrap();

    let error = check_utf8_without_bom(&path).unwrap_err();

    assert!(matches!(error, Utf8ValidationError::Utf8Bom));
    assert_eq!(error.to_string(), "File contains UTF-8 BOM");
}

#[test]
fn test_check_utf8_without_bom_preserves_io_context() {
    let dir = tempfile::tempdir().unwrap();
    let missing_path = dir.path().join("missing.md");

    let error = check_utf8_without_bom(&missing_path).unwrap_err();

    assert!(matches!(error, Utf8ValidationError::Io { .. }));
    assert!(error.to_string().contains("Cannot read file bytes:"));
}

#[test]
fn test_check_utf8_without_bom_rejects_invalid_utf8() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.md");
    fs::write(&path, [0xFF]).unwrap();

    let error = check_utf8_without_bom(&path).unwrap_err();

    assert!(matches!(error, Utf8ValidationError::InvalidUtf8 { .. }));
    assert!(error.to_string().contains("Cannot read file as UTF-8"));
}

#[test]
fn test_check_utf8_without_bom_rejects_crlf_line_endings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crlf.md");
    fs::write(&path, b"hello\r\n").unwrap();

    let error = check_utf8_without_bom(&path).unwrap_err();

    assert!(matches!(error, Utf8ValidationError::CrlfLineEndings));
    assert!(error.to_string().contains("CRLF line endings"));
}

#[tokio::test]
async fn test_validate_rejects_crlf_and_validate_fix_normalizes_line_endings() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let valid_doc = create_valid_rfc_doc();
    let crlf_doc = valid_doc.replace('\n', "\r\n");
    let doc_path = draft_dir.join("rfc-00001-test-rfc.md");
    fs::write(&doc_path, crlf_doc.as_bytes()).unwrap();

    let validate_output = run_validate(&root, false).await;
    assert!(!validate_output.valid, "Expected validate to reject CRLF line endings");
    assert!(
        validate_output.errors.iter().any(|error| error.error.contains("CRLF line endings")),
        "Expected CRLF validation error, got: {:?}",
        validate_output.errors
    );

    let fix_output = run_validate(&root, true).await;
    assert!(
        fix_output.valid,
        "Expected validate_fix to repair CRLF line endings, got errors: {:?}",
        fix_output.errors
    );
    assert!(
        fix_output.fixes.iter().any(|fix| fix.fix_type == "normalize_line_endings"),
        "Expected line-ending fix, got: {:?}",
        fix_output.fixes
    );

    let written = fs::read(&doc_path).unwrap();
    assert!(!written.windows(2).any(|window| window == b"\r\n"), "Expected CRLF to be removed");
    assert!(written.ends_with(b"\n"), "Expected final newline to be preserved");
    assert_eq!(String::from_utf8(written).unwrap(), valid_doc);
}

#[tokio::test]
async fn test_validate_rejects_invalid_utf8_and_validate_fix_does_not_rewrite() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let doc_path = draft_dir.join("rfc-00001-test-rfc.md");
    let invalid_bytes = b"---\nid: rfc-00001-test\n---\n\xFF\n".to_vec();
    fs::write(&doc_path, &invalid_bytes).unwrap();

    let validate_output = run_validate(&root, false).await;
    assert!(!validate_output.valid, "Expected validate to reject invalid UTF-8");
    assert!(
        validate_output.errors.iter().any(|error| error.error.contains("UTF-8")),
        "Expected UTF-8 validation error, got: {:?}",
        validate_output.errors
    );

    let fix_output = run_validate(&root, true).await;
    assert!(!fix_output.valid, "Expected validate_fix to keep reporting invalid UTF-8");
    assert!(
        fix_output.errors.iter().any(|error| error.error.contains("UTF-8")),
        "Expected UTF-8 validation error after fix, got: {:?}",
        fix_output.errors
    );
    assert!(
        fix_output.fixes.iter().all(|fix| fix.path != doc_path.to_string_lossy()),
        "Expected invalid UTF-8 file to remain unrepaired, got: {:?}",
        fix_output.fixes
    );
    assert_eq!(fs::read(&doc_path).unwrap(), invalid_bytes);
}

#[tokio::test]
async fn test_fix_mode_moves_file_to_correct_status_folder() {
    let (temp_dir, root) = create_test_project();

    let rfc_dir = temp_dir.path().join("doc").join("rfc");
    fs::create_dir_all(&rfc_dir).unwrap();

    let review_dir = rfc_dir.join("review");
    fs::create_dir_all(&review_dir).unwrap();

    let draft_dir = rfc_dir.join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let invalid_doc = create_valid_rfc_doc();
    fs::write(review_dir.join("rfc-00001-test-rfc.md"), &invalid_doc).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: true }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.fixes.is_empty(), "Expected fixes to be applied");
    assert!(output.fixes.iter().any(|f| f.fix_type == "move_file"));
}

#[tokio::test]
async fn test_fix_mode_normalizes_wikilinks() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let mut doc_with_bad_wikilink = create_valid_rfc_doc();
    doc_with_bad_wikilink.push_str("\nSee [[other-doc.md]] for details.\n");
    fs::write(draft_dir.join("rfc-00001-test-rfc.md"), &doc_with_bad_wikilink).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: true }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.fixes.is_empty(), "Expected fixes to be applied");
    assert!(output.fixes.iter().any(|f| f.fix_type == "normalize_wikilinks"));
}

#[tokio::test]
async fn test_fix_mode_rewrites_wikilinks_without_adding_normalize_content_fix() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let doc_needing_fixes = "---\n\
id: rfc-00001-test\n\
type: rfc\n\
code: \"00001\"\n\
slug: test-rfc\n\
title: Test RFC\n\
description: A test RFC document\n\
created: 2026-01-01\n\
tags:\n  - test\n\
status: draft\n\
---\n\
Test RFC\n\
\nSee [[other-doc.md]] for details.\n"
        .to_string();
    let doc_path = draft_dir.join("rfc-00001-test-rfc.md");
    fs::write(&doc_path, &doc_needing_fixes).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: true }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(
        output.fixes.iter().any(|fix| fix.fix_type == "normalize_wikilinks"),
        "Expected wikilink normalization fix, got: {:?}",
        output.fixes
    );
    assert!(
        output.fixes.iter().all(|fix| fix.fix_type != "normalize_content"),
        "Expected specific fix reporting without fallback normalize_content, got: {:?}",
        output.fixes
    );

    let written = fs::read_to_string(doc_path).unwrap();
    assert!(written.contains("[[other-doc]]"), "Expected normalized wikilink, got: {written}");
}

#[tokio::test]
async fn test_fix_mode_removes_bom() {
    let (temp_dir, root) = create_test_project();

    let rfc_dir = temp_dir.path().join("doc").join("rfc");
    fs::create_dir_all(&rfc_dir).unwrap();

    let valid_content = create_valid_rfc_doc();
    let bom = [0xEF, 0xBB, 0xBF];
    let mut content_with_bom = Vec::from(bom);
    content_with_bom.extend(valid_content.as_bytes());
    fs::write(rfc_dir.join("rfc-00001-test.md"), content_with_bom).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: true }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.fixes.is_empty(), "Expected fixes to be applied");
    assert!(output.fixes.iter().any(|f| f.fix_type == "remove_bom"));
}

#[tokio::test]
async fn test_filename_pattern_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let bad_config =
        "doc-type: {template: t, prompt-template: pt, prompt: p, create-document-type-form: f}
document-types:
  rfc:
    template: template-00001-rfc
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    create-document-form: form-00001-create-document
    filename_pattern: \"{type}-{code}-{slug}.md\"
    initial-status: draft
    statuses:
      - draft
  ";
    fs::write(vector_dir.join("document-types.yaml"), bad_config).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false when filename_pattern is set");
}

#[tokio::test]
async fn test_validate_with_invalid_slug() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let mut invalid_doc = create_valid_rfc_doc();
    invalid_doc = invalid_doc.replace("slug: test-rfc", "slug: Test-RFC");
    fs::write(draft_dir.join("rfc-00001-test-rfc.md"), &invalid_doc).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for invalid slug");
    assert!(output.errors.iter().any(|e| e.error.contains("Invalid slug")));
}

#[tokio::test]
async fn test_validate_with_slug_consecutive_hyphens() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let mut invalid_doc = create_valid_rfc_doc();
    invalid_doc = invalid_doc.replace("slug: test-rfc", "slug: test--rfc");
    fs::write(draft_dir.join("rfc-00001-test-rfc.md"), &invalid_doc).unwrap();

    let mut sender = MockSender::new();
    let op = crate::operations::ValidateOp;

    op.run(ValidateInput { root_dir: root, fix: false }, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for slug with consecutive hyphens");
    assert!(output.errors.iter().any(|e| e.error.contains("consecutive hyphens")));
}

// ── Phase B: template exemptions ─────────────────────────────────────────────
//
// Files under doc/template/ carry placeholder frontmatter values.
// The validator skips all field-level checks for that doc_type folder.
// Only BOM, UTF-8 readability, and wikilink format are still enforced.

fn create_test_project_with_template() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let config = "doc-type:
  template: template-00004-doc-type-template
  prompt-template: template-00005-doc-type-prompt
  prompt: prompts-00001-create-doc-type
  create-document-type-form: form-00002-create-document-type
document-types:
  rfc:
    template: template-00001-rfc
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    create-document-form: form-00001-create-document
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
    create-document-form: form-00001-create-document
  template:
    layout: category
    code-width: 5
    prompt: prompts-00005-create-template
    create-document-form: form-00001-create-document
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    let doc_dir = temp_dir.path().join("doc");
    fs::create_dir_all(&doc_dir).unwrap();

    (temp_dir, root)
}

fn write_template_file(temp_dir: &TempDir, file_name: &str, content: &str) {
    let template_dir = temp_dir.path().join("doc").join("template").join("prompts");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join(file_name), content).unwrap();
}

fn template_content() -> String {
    "---\n\
id: rfc-<code>-<slug>\n\
type: rfc\n\
code: \"<code>\"\n\
slug: <slug>\n\
title: <Title>\n\
description: <Description>\n\
created: <YYYY-MM-DD>\n\
tags: []\n\
status: draft\n\
---\n\n\
# <Title>\n"
        .to_string()
}

#[tokio::test]
async fn test_validate_fix_repairs_template_bom_and_crlf_line_endings() {
    let (temp_dir, root) = create_test_project_with_template();

    let template_dir = temp_dir.path().join("doc").join("template").join("prompts");
    fs::create_dir_all(&template_dir).unwrap();
    let template_path = template_dir.join("template-00001-rfc.md");

    let original_content = template_content();
    let crlf_content = original_content.replace('\n', "\r\n");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(crlf_content.as_bytes());
    fs::write(&template_path, bytes).unwrap();

    let validate_output = run_validate(&root, false).await;
    assert!(!validate_output.valid, "Expected validate to reject template BOM/CRLF");
    assert!(
        validate_output.errors.iter().any(|error| error.error.contains("BOM")),
        "Expected template BOM error, got: {:?}",
        validate_output.errors
    );

    let fix_output = run_validate(&root, true).await;
    assert!(
        fix_output.valid,
        "Expected validate_fix to repair template BOM/CRLF, got errors: {:?}",
        fix_output.errors
    );
    assert!(
        fix_output.fixes.iter().any(|fix| fix.fix_type == "remove_bom"),
        "Expected template BOM fix, got: {:?}",
        fix_output.fixes
    );
    assert!(
        fix_output.fixes.iter().any(|fix| fix.fix_type == "normalize_line_endings"),
        "Expected template line-ending fix, got: {:?}",
        fix_output.fixes
    );

    let written = fs::read(&template_path).unwrap();
    assert!(!written.starts_with(&[0xEF, 0xBB, 0xBF]), "Expected BOM to be removed");
    assert!(
        !written.windows(2).any(|window| window == b"\r\n"),
        "Expected template CRLF to be removed"
    );
    assert_eq!(String::from_utf8(written).unwrap(), original_content);
}

#[tokio::test]
async fn test_validate_fix_does_not_rewrite_invalid_utf8_template() {
    let (temp_dir, root) = create_test_project_with_template();

    let template_dir = temp_dir.path().join("doc").join("template").join("prompts");
    fs::create_dir_all(&template_dir).unwrap();
    let template_path = template_dir.join("template-00001-rfc.md");
    let invalid_bytes = b"---\nid: rfc-<code>-<slug>\n---\n\xFF\n".to_vec();
    fs::write(&template_path, &invalid_bytes).unwrap();

    let fix_output = run_validate(&root, true).await;
    assert!(!fix_output.valid, "Expected validate_fix to report invalid template UTF-8");
    assert!(
        fix_output.errors.iter().any(|error| error.error.contains("UTF-8")),
        "Expected invalid UTF-8 error, got: {:?}",
        fix_output.errors
    );
    assert!(
        fix_output.fixes.iter().all(|fix| fix.path != template_path.to_string_lossy()),
        "Expected invalid UTF-8 template to remain unrepaired, got: {:?}",
        fix_output.fixes
    );
    assert_eq!(fs::read(&template_path).unwrap(), invalid_bytes);
}

#[tokio::test]
async fn test_template_with_placeholder_frontmatter_accepted() {
    // Template files have placeholder values like <code>, <slug>, etc. — all valid.
    let (temp_dir, root) = create_test_project_with_template();
    let content = template_content();
    write_template_file(&temp_dir, "template-00001-rfc.md", &content);

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    assert!(
        output.valid,
        "Expected valid=true for template file with placeholder frontmatter, got errors: {:?}",
        output.errors
    );
}

#[tokio::test]
async fn test_template_without_category_field_accepted() {
    // Template files may omit the category field even if their doc_type folder is category-based.
    let (temp_dir, root) = create_test_project_with_template();
    let content = "---\n\
        id: spec-<code>-<slug>\n\
        type: spec\n\
        code: \"<code>\"\n\
        slug: <slug>\n\
        title: <Title>\n\
        description: <Description>\n\
        created: <YYYY-MM-DD>\n\
        tags: []\n\
        ---\n\n\
        # <Title>\n";
    write_template_file(&temp_dir, "template-00002-spec.md", content);

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    let category_errors: Vec<_> =
        output.errors.iter().filter(|e| e.error.contains("category")).collect();
    assert!(
        category_errors.is_empty(),
        "Expected no category errors for template file, got: {category_errors:?}"
    );
}

#[tokio::test]
async fn test_template_with_invalid_slug_placeholder_accepted() {
    // Slugs like <slug> or <api | data> contain angle brackets — still accepted for templates.
    let (temp_dir, root) = create_test_project_with_template();
    let content = "---\n\
        id: spec-<code>-<slug>\n\
        type: spec\n\
        code: \"<code>\"\n\
        slug: <api | data | interface>\n\
        title: <Title>\n\
        description: <Description>\n\
        created: <YYYY-MM-DD>\n\
        tags: []\n\
        ---\n\n\
        # <Title>\n";
    write_template_file(&temp_dir, "template-00003-spec-variant.md", content);

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    let slug_errors: Vec<_> = output.errors.iter().filter(|e| e.error.contains("slug")).collect();
    assert!(
        slug_errors.is_empty(),
        "Expected no slug errors for template file, got: {slug_errors:?}"
    );
}

#[tokio::test]
async fn test_non_template_category_still_required() {
    // Non-template category-based docs still require the category field.
    let (temp_dir, root) = create_test_project_with_template();

    let spec_dir = temp_dir.path().join("doc").join("spec").join("my-cat");
    fs::create_dir_all(&spec_dir).unwrap();

    let spec_doc = "---\n\
        id: spec-00001-no-cat\n\
        type: spec\n\
        code: \"00001\"\n\
        slug: no-cat\n\
        title: No Category Spec\n\
        description: A spec without a category\n\
        created: 2026-01-01\n\
        tags:\n  - test\n\
        ---\n\n\
        # No Category Spec\n";
    fs::write(spec_dir.join("spec-00001-no-cat.md"), spec_doc).unwrap();

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for spec doc missing category");
    assert!(
        output.errors.iter().any(|e| e.error.contains("category")),
        "Expected category error for non-template doc, got: {:?}",
        output.errors
    );
}

#[tokio::test]
async fn test_non_template_slug_still_validated() {
    // Non-template docs still have their slug validated.
    let (temp_dir, root) = create_test_project_with_template();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let rfc_doc = "---\n\
        id: rfc-00001-bad-slug\n\
        type: rfc\n\
        code: \"00001\"\n\
        slug: Bad_Slug\n\
        title: Bad Slug RFC\n\
        description: RFC with invalid slug\n\
        created: 2026-01-01\n\
        tags:\n  - test\n\
        status: draft\n\
        ---\n\n\
        # Bad Slug RFC\n";
    fs::write(draft_dir.join("rfc-00001-bad-slug.md"), rfc_doc).unwrap();

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for rfc doc with invalid slug");
    assert!(
        output.errors.iter().any(|e| e.error.contains("slug")),
        "Expected slug error for non-template doc, got: {:?}",
        output.errors
    );
}

#[tokio::test]
async fn test_validate_directory_based() {
    let temp = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp.path());

    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p, create-document-type-form: f}\ndocument-types:\n  research:\n    layout: directory\n    code-width: 5\n    create-document-form: form-00001",
    )
    .unwrap();

    let research_dir = temp.path().join("doc").join("research");
    fs::create_dir_all(&research_dir).unwrap();

    // 1. Valid placement
    let valid_doc = "---\nid: research-00001-valid\ntype: research\ncode: \"00001\"\nslug: valid\ntitle: Valid\ndescription: desc\ncreated: 2026-05-10\ntags: []\n---\n# Valid";
    fs::write(research_dir.join("research-00001-valid.md"), valid_doc).unwrap();

    // 2. Invalid placement (nested)
    let nested_dir = research_dir.join("draft");
    fs::create_dir_all(&nested_dir).unwrap();
    let nested_doc = "---\nid: research-00002-nested\ntype: research\ncode: \"00002\"\nslug: nested\ntitle: Nested\ndescription: desc\ncreated: 2026-05-10\ntags: []\n---\n# Nested";
    fs::write(nested_dir.join("research-00002-nested.md"), nested_doc).unwrap();

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    // One valid, one invalid
    assert!(!output.valid);
    assert!(
        output.errors.iter().any(|e| e.error.contains("must be directly under 'doc/research'"))
    );
}

#[tokio::test]
async fn test_fix_directory_based_move() {
    let temp = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp.path());

    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p, create-document-type-form: f}\ndocument-types:\n  research:\n    layout: directory\n    code-width: 5\n    create-document-form: form-00001",
    )
    .unwrap();

    let research_dir = temp.path().join("doc").join("research");
    let nested_dir = research_dir.join("draft");
    fs::create_dir_all(&nested_dir).unwrap();
    let nested_doc = "---\nid: research-00001-nested\ntype: research\ncode: \"00001\"\nslug: nested\ntitle: Nested\ndescription: desc\ncreated: 2026-05-10\ntags: []\n---\n# Nested";
    let doc_path = nested_dir.join("research-00001-nested.md");
    fs::write(&doc_path, nested_doc).unwrap();

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: true }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    assert!(!output.fixes.is_empty(), "Expected fixes to be applied");
    assert!(output.fixes.iter().any(|f| f.fix_type == "move_file"));

    let expected_path = research_dir.join("research-00001-nested.md");
    assert!(
        expected_path.exists(),
        "Document should have been moved to {}",
        expected_path.display()
    );
    assert!(!doc_path.exists(), "Old document path should not exist");
}

#[tokio::test]
async fn test_validate_fails_when_missing_create_document_form() {
    let temp = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp.path());

    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p, create-document-type-form: f}
document-types:
  rfc:
    layout: status
    code-width: 5
    statuses:
      - draft
",
    )
    .unwrap();

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false when create-document-form is missing");
    assert!(
        output.errors.iter().any(|e| e.error.contains("document-types.rfc.create-document-form")),
        "Expected error about missing create-document-form, got: {:?}",
        output.errors
    );
}

#[tokio::test]
async fn test_validate_fails_when_missing_create_document_type_form() {
    let temp = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp.path());

    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  rfc:
    layout: status
    code-width: 5
    create-document-form: form-00001
    statuses:
      - draft
",
    )
    .unwrap();

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false when create-document-type-form is missing");
    assert!(
        output.errors.iter().any(|e| e.error.contains("doc-type.create-document-type-form")),
        "Expected error about missing create-document-type-form, got: {:?}",
        output.errors
    );
}

#[tokio::test]
async fn test_validate_rejects_snake_case_placeholder_names() {
    let (temp_dir, root) = create_test_project();

    let draft_dir = temp_dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).unwrap();

    let mut invalid_doc = create_valid_rfc_doc();
    invalid_doc.push_str("\nUse #{doc-type} and #{doc_type}.\n");
    fs::write(draft_dir.join("rfc-00001-test-rfc.md"), &invalid_doc).unwrap();

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for snake_case placeholder");
    assert!(
        output
            .errors
            .iter()
            .any(|e| e.path.contains("rfc-00001-test-rfc.md") && e.error.contains("#{doc_type}")),
        "Expected exact placeholder error, got: {:?}",
        output.errors
    );
}

#[tokio::test]
async fn test_validate_rejects_invalid_vector_yaml_schema_field_names() {
    let (temp_dir, root) = create_test_project();

    fs::write(
        temp_dir.path().join(".vector").join("language-rules.yaml"),
        "rust:\n  quality_gate: prompts-00006-rust\n",
    )
    .unwrap();

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    assert!(!output.valid, "Expected valid=false for snake_case YAML field");
    assert!(
        output
            .errors
            .iter()
            .any(|e| e.path == ".vector/language-rules.yaml" && e.error.contains("quality_gate")),
        "Expected exact YAML field error, got: {:?}",
        output.errors
    );
}

#[tokio::test]
async fn test_validate_allows_dynamic_dashboard_section_keys() {
    let (temp_dir, root) = create_test_project();

    fs::create_dir_all(temp_dir.path().join(".vector").join("dashboards")).unwrap();
    fs::write(
        temp_dir.path().join(".vector").join("dashboards").join("project-status.yaml"),
        "label: Project Status\nsections:\n  todo-tasks:\n    title: TODO tasks\n    doc-type: task\n    statuses: [todo]\n",
    )
    .unwrap();

    let mut sender = MockSender::new();
    crate::operations::ValidateOp
        .run(ValidateInput { root_dir: root, fix: false }, &mut sender)
        .await
        .unwrap();

    let output = sender.output.unwrap();
    let dashboard_errors: Vec<_> = output
        .errors
        .iter()
        .filter(|error| error.path == ".vector/dashboards/project-status.yaml")
        .collect();
    assert!(
        dashboard_errors.is_empty(),
        "Expected dynamic dashboard keys to remain valid, got: {dashboard_errors:?}"
    );
}
