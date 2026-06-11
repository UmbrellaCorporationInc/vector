#![allow(clippy::unwrap_used)]

use super::{ApplyPatchHunk, ApplyPatchUpdate, apply_apply_patch_format};
use runtime_io::IoPath;
use std::path::PathBuf;

fn tmp_doc(name: &str, content: &str) -> (PathBuf, IoPath) {
    let doc_path = std::env::temp_dir().join(format!("vector-patch-apply-patch-test-{name}.md"));
    std::fs::write(&doc_path, content).unwrap();
    let root = doc_path.parent().unwrap().to_path_buf();
    (doc_path, IoPath::new(root))
}

fn make_payload(target: &str, hunk_lines: &str) -> String {
    format!("*** Begin Patch\n*** Update File: {target}\n@@\n{hunk_lines}*** End Patch\n")
}

#[test]
fn test_apply_patch_format_replaces_matching_context_lines() {
    let original = "Line one\nLine two\nLine three\n";
    let (doc_path, root) = tmp_doc("replace-lines", original);
    let abs_path = dunce::canonicalize(&doc_path).unwrap();
    let target = abs_path.to_string_lossy().replace('\\', "/");
    let payload = make_payload(&target, " Line one\n-Line two\n+Line TWO\n Line three\n");

    let result =
        apply_apply_patch_format(&abs_path.to_string_lossy(), &root, "", original, &payload);

    std::fs::remove_file(&doc_path).ok();
    let patched = result.unwrap();
    assert!(patched.contains("Line TWO"), "replacement line should appear");
    assert!(!patched.contains("Line two"), "original line should be gone");
}

#[test]
fn test_apply_patch_format_returns_error_for_missing_begin_boundary() {
    let original = "Content\n";
    let (doc_path, root) = tmp_doc("missing-begin", original);
    let abs_path = dunce::canonicalize(&doc_path).unwrap();
    let target = abs_path.to_string_lossy().replace('\\', "/");
    let bad_payload = format!("*** Update File: {target}\n@@\n Content\n*** End Patch\n");

    let result =
        apply_apply_patch_format(&abs_path.to_string_lossy(), &root, "", original, &bad_payload);

    std::fs::remove_file(&doc_path).ok();
    assert!(result.is_err(), "missing Begin Patch boundary should fail");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Begin Patch"), "error should mention the missing boundary");
}

#[test]
fn test_apply_patch_format_returns_error_for_ambiguous_hunk_context() {
    let original = "A\nB\nA\nB\n";
    let (doc_path, root) = tmp_doc("ambiguous-hunk", original);
    let abs_path = dunce::canonicalize(&doc_path).unwrap();
    let target = abs_path.to_string_lossy().replace('\\', "/");
    let payload = make_payload(&target, " A\n-B\n+X\n");

    let result =
        apply_apply_patch_format(&abs_path.to_string_lossy(), &root, "", original, &payload);

    std::fs::remove_file(&doc_path).ok();
    assert!(result.is_err(), "ambiguous context should fail");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("ambiguous"), "error should mention ambiguity");
}

#[test]
fn test_apply_patch_hunk_struct_holds_expected_lines() {
    let hunk =
        ApplyPatchHunk { old_lines: vec!["old".to_owned()], new_lines: vec!["new".to_owned()] };
    assert_eq!(hunk.old_lines, vec!["old"]);
    assert_eq!(hunk.new_lines, vec!["new"]);
}

#[test]
fn test_apply_patch_update_struct_holds_target_and_hunks() {
    let update = ApplyPatchUpdate {
        target: "some/file.md".to_owned(),
        hunks: vec![ApplyPatchHunk {
            old_lines: vec!["a".to_owned()],
            new_lines: vec!["b".to_owned()],
        }],
    };
    assert_eq!(update.target, "some/file.md");
    assert_eq!(update.hunks.len(), 1);
}
