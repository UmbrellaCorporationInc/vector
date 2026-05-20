#![allow(clippy::unwrap_used)]

use std::fs;
use tempfile::TempDir;

use super::{fix_bom_if_present, fix_heading_if_needed};

fn write_file_with_bom(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let bom = [0xEF, 0xBB, 0xBF];
    let mut bytes = Vec::from(bom);
    bytes.extend_from_slice(content.as_bytes());
    fs::write(&path, bytes).unwrap();
    path
}

fn write_file_without_bom(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn fix_bom_removes_bom_and_reports_fix() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file_with_bom(&dir, "file.md", "hello");

    let result = fix_bom_if_present(&path);

    assert!(result.is_some(), "Expected a fix result for BOM file");
    let fix = result.unwrap();
    assert_eq!(fix.fix_type, "remove_bom");

    let written = fs::read(&path).unwrap();
    assert!(!written.starts_with(&[0xEF, 0xBB, 0xBF]), "BOM should be removed from file");
    assert_eq!(written, b"hello");
}

#[test]
fn fix_bom_returns_none_for_clean_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file_without_bom(&dir, "file.md", "hello");

    let result = fix_bom_if_present(&path);

    assert!(result.is_none(), "Expected no fix result for clean file");
}

#[test]
fn fix_bom_returns_none_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.md");

    let result = fix_bom_if_present(&path);

    assert!(result.is_none(), "Expected no fix result when file bytes cannot be read");
}

#[test]
fn fix_heading_adds_markdown_heading_to_first_content_line() {
    let content = "---\nid: sample\n---Title\nBody\n";

    let result = fix_heading_if_needed("doc/sample.md", content);

    let (fixed_content, fix) = result.unwrap();
    assert_eq!(fix.fix_type, "normalize_heading");
    assert!(fixed_content.contains("---# Title\nBody\n"));
}
