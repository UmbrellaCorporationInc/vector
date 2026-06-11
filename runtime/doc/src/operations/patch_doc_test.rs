#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

struct MockSender {
    outputs: Vec<PatchDocOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { outputs: Vec::new() }
    }
}

impl runtime_core::Sender<PatchDocOutput> for MockSender {
    async fn send(&mut self, value: PatchDocOutput) -> runtime_core::RuntimeResult<()> {
        self.outputs.push(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<PatchDocOutput> for MockSender {
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

fn make_diff(filename: &str, old_line: &str, new_line: &str) -> String {
    format!("--- a/{filename}\n+++ b/{filename}\n@@ -1,1 +1,1 @@\n-{old_line}\n+{new_line}\n")
}

// ── happy path ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_patch_doc_valid_patch_succeeds() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    create_doc_file(&temp, "task", filename, "old content\n");

    let diff = make_diff(filename, "old content", "new content");

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff);
    let mut sender = MockSender::new();
    patch_doc(input, &mut sender).await.unwrap();

    assert_eq!(sender.outputs.len(), 1);
    assert_eq!(sender.outputs[0].content, "new content\n");

    // Verify file was written
    let doc_path = temp.path().join("doc").join("task").join("draft").join(filename);
    let written = fs::read_to_string(&doc_path).unwrap();
    assert_eq!(written, "new content\n");
}

#[tokio::test]
async fn test_patch_doc_rejects_find_doc_content_patch_when_hunk_counts_overstate_body_lines() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    let original = "line one\nline two\nline three\n";
    create_doc_file(&temp, "task", filename, original);

    let mut find_sender = CapturingSender::<FindDocOutput>::new();
    FindDocOp::new()
        .run(
            FindDocInput::new(root.clone(), String::new(), "task".to_string(), 1),
            &mut find_sender,
        )
        .await
        .unwrap();
    let found = find_sender.into_output().expect("find_doc must return document content");
    assert!(
        found.content.contains("line one\nline two\nline three\n"),
        "Phase A setup must generate the patch from the content returned by find_doc"
    );

    let mut lines = found.content.lines();
    let first_line = lines.next().expect("document must have a first line");
    let second_line = lines.next().expect("document must have a second line");
    let diff = format!(
        "\
--- a/{filename}
+++ b/{filename}
@@ -1,3 +1,3 @@
-{first_line}
-{second_line}
+LINE ONE
+LINE TWO
"
    );

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff);
    let mut sender = MockSender::new();
    let result = patch_doc(input, &mut sender).await;

    assert!(
        result.is_err(),
        "Phase A regression path must reproduce: patch_doc rejects a unified diff generated from find_doc content when the hunk header overstates the body line counts"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("hunk line-count mismatch"),
        "Phase A localized failing path: rejection is caused by hunk line-count mismatch during preflight, not trailing newline handling, CRLF normalization, context offset drift, or patcher.apply matching; got: {err}"
    );
    assert!(err.contains("Hunk header declares (-3, +3)"), "{err}");
    assert!(err.contains("hunk body contains (-2, +2)"), "{err}");
    assert!(sender.outputs.is_empty(), "rejected patches must not emit patched content");

    let doc_path = temp.path().join("doc").join("task").join("draft").join(filename);
    let on_disk = fs::read_to_string(&doc_path).unwrap();
    assert_eq!(on_disk, original, "file must not be modified when hunk count preflight fails");
}

// ── missing document ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_patch_doc_missing_document_returns_error() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    // No file created — code 99 doesn't exist
    let doc_dir = temp.path().join("doc").join("task").join("draft");
    fs::create_dir_all(doc_dir).unwrap();

    let input = PatchDocInput::new(root, "task".to_string(), 99, "irrelevant".to_string());
    let mut sender = MockSender::new();
    let result = patch_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected error for missing document");
}

// ── target mismatch (covers scope enforcement) ────────────────────────────────

#[tokio::test]
async fn test_patch_doc_target_mismatch_rejected() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    create_doc_file(&temp, "task", "task-00001-foo.md", "content\n");

    // Diff references a different file (outside doc/ — would be rejected by target mismatch)
    let diff = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n";

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff.to_string());
    let mut sender = MockSender::new();
    let result = patch_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected error for target mismatch");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("main.rs") || err.contains("task-00001"), "{err}");
}

// ── unsupported diff shapes ───────────────────────────────────────────────────

#[tokio::test]
async fn test_patch_doc_delete_patch_rejected() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    create_doc_file(&temp, "task", "task-00001-foo.md", "line one\nline two\n");

    let diff = "--- a/task-00001-foo.md\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-line one\n-line two\n";

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff.to_string());
    let mut sender = MockSender::new();
    let result = patch_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected error for delete patch");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("delete"), "{err}");
}

#[tokio::test]
async fn test_patch_doc_rename_patch_rejected() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    create_doc_file(&temp, "task", "task-00001-foo.md", "content\n");

    let diff =
        "--- a/task-00001-foo.md\n+++ b/task-00001-bar.md\n@@ -1,1 +1,1 @@\n-content\n+content\n";

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff.to_string());
    let mut sender = MockSender::new();
    let result = patch_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected error for rename patch");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("rename") || err.contains("renames"), "{err}");
}

#[tokio::test]
async fn test_patch_doc_malformed_diff_rejected() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    create_doc_file(&temp, "task", "task-00001-foo.md", "content\n");

    // Completely invalid diff — no --- or +++ headers
    let diff = "this is not a diff at all";

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff.to_string());
    let mut sender = MockSender::new();
    let result = patch_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected error for malformed diff");
}

#[tokio::test]
async fn test_patch_doc_hunk_count_mismatch_rejected_during_preflight_without_write() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    let original = "line one\nline two\nline three\n";
    create_doc_file(&temp, "task", filename, original);

    let diff = "\
--- a/src/main.rs
+++ b/src/main.rs
@@ -37,10 +37,10 @@ input:
-old one
-old two
-old three
-old four
-old five
-old six
-old seven
+new one
+new two
+new three
+new four
+new five
+new six
+new seven
";

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff.to_string());
    let mut sender = MockSender::new();
    let result = patch_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected hunk count mismatch to be rejected during preflight");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("patch is not a valid unified diff"), "{err}");
    assert!(err.contains("hunk line-count mismatch"), "{err}");
    assert!(err.contains("Hunk header declares (-10, +10)"), "{err}");
    assert!(err.contains("hunk body contains (-7, +7)"), "{err}");
    assert!(err.contains("Make the @@ -a,b +c,d @@ counts match"), "{err}");
    assert!(err.contains("old-side lines and new-side lines"), "{err}");
    assert!(err.contains("Preflight detail:"), "{err}");
    assert!(err.contains("Header expected (-10, +10)"), "{err}");
    assert!(err.contains("Parsed content counts (-7, +7)"), "{err}");
    assert!(
        !err.contains("patch targets"),
        "malformed hunk counts must fail during preflight before target mismatch checks: {err}"
    );
    assert!(sender.outputs.is_empty(), "preflight failures must not emit patched content");

    let doc_path = temp.path().join("doc").join("task").join("draft").join(filename);
    let on_disk = fs::read_to_string(&doc_path).unwrap();
    assert_eq!(on_disk, original, "file must not be modified when preflight fails");
}

#[tokio::test]
async fn test_patch_doc_valid_multi_hunk_diff_survives_hunk_count_preflight() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    create_doc_file(
        &temp,
        "task",
        filename,
        "line one\nline two\nline three\nline four\nline five\nline six\n",
    );

    let diff = "\
--- a/task-00001-foo.md
+++ b/task-00001-foo.md
@@ -1,2 +1,2 @@
-line one
+LINE ONE
 line two
@@ -4,2 +4,2 @@
 line four
-line five
+LINE FIVE
";

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff.to_string());
    let mut sender = MockSender::new();
    patch_doc(input, &mut sender).await.unwrap();

    assert_eq!(sender.outputs.len(), 1);
    assert_eq!(
        sender.outputs[0].content,
        "LINE ONE\nline two\nline three\nline four\nLINE FIVE\nline six\n"
    );
}

// ── BOM rejection ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_patch_doc_bom_in_result_rejected_without_write() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    let original = "original content\n";
    create_doc_file(&temp, "task", filename, original);

    // The diff introduces a BOM at the start of the new content
    // BOM bytes: \xEF\xBB\xBF
    let bom_line = "\u{feff}new content";
    let diff = format!(
        "--- a/{filename}\n+++ b/{filename}\n@@ -1,1 +1,1 @@\n-original content\n+{bom_line}\n"
    );

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff);
    let mut sender = MockSender::new();
    let result = patch_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected BOM rejection");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("BOM") || err.contains("bom") || err.contains("\\xEF"), "{err}");

    // File must not have been written
    let doc_path = temp.path().join("doc").join("task").join("draft").join(filename);
    let on_disk = fs::read_to_string(&doc_path).unwrap();
    assert_eq!(on_disk, original, "file must not be modified when BOM is detected");
}

// ── normalization ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_patch_doc_normalizes_markdown_code_fence() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    create_doc_file(&temp, "task", filename, "old\n");

    let diff_wrapped =
        format!("```diff\n--- a/{filename}\n+++ b/{filename}\n@@ -1,1 +1,1 @@\n-old\n+new\n```");

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff_wrapped);
    let mut sender = MockSender::new();
    patch_doc(input, &mut sender).await.unwrap();

    assert_eq!(sender.outputs[0].content, "new\n");
}

// ── op struct constructors and FlowOperation delegation ───────────────────────

#[tokio::test]
async fn test_patch_doc_op_new_and_default_are_equivalent() {
    // Exercises PatchDocOp::new() and PatchDocOp::default() for coverage.
    let op_new = PatchDocOp::new();
    let op_default = PatchDocOp::default();
    // Both should produce a working op — run a happy-path case through each.
    let temp = TempDir::new().unwrap();
    write_doc_config(&temp, "task");
    let filename = "task-00001-foo.md";
    create_doc_file(&temp, "task", filename, "before\n");
    let diff = make_diff(filename, "before", "after");
    let root = IoPath::new(temp.path());

    let input = PatchDocInput::new(root.clone(), "task".to_string(), 1, diff.clone());
    let mut s1 = MockSender::new();
    op_new.run(input, &mut s1).await.unwrap();
    assert_eq!(s1.outputs[0].content, "after\n");

    // Reset file for second run
    let doc_path = temp.path().join("doc").join("task").join("draft").join(filename);
    std::fs::write(&doc_path, "before\n").unwrap();

    let input2 = PatchDocInput::new(root, "task".to_string(), 1, diff);
    let mut s2 = MockSender::new();
    op_default.run(input2, &mut s2).await.unwrap();
    assert_eq!(s2.outputs[0].content, "after\n");
}

// ── create patch rejection ────────────────────────────────────────────────────

#[tokio::test]
async fn test_patch_doc_create_patch_rejected() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_doc_config(&temp, "task");
    create_doc_file(&temp, "task", "task-00001-foo.md", "content\n");

    let diff = "--- /dev/null\n+++ b/task-00001-foo.md\n@@ -0,0 +1,1 @@\n+content\n";

    let input = PatchDocInput::new(root, "task".to_string(), 1, diff.to_string());
    let mut sender = MockSender::new();
    let result = patch_doc(input, &mut sender).await;

    assert!(result.is_err(), "expected error for create patch");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("create") || err.contains("new file"), "{err}");
}
