#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let root = std::env::temp_dir().join(format!("vector-rag-update-db-{label}-{nanos}"));
    std::fs::create_dir_all(root.join(".vector")).expect("failed to create .vector root");
    root
}

#[tokio::test]
async fn rag_update_database_succeeds_on_empty_corpus() {
    let root = unique_root("empty");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    run_with_writers(&root, &mut stdout, &mut stderr)
        .await
        .expect("update-database should succeed on an empty corpus");

    let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("initializing-store"), "expected store initialization progress");
    assert!(stdout.contains("discovering-documents"), "expected discovery progress");
    assert!(stdout.contains("Indexed: 0 re-indexed, 0 skipped, 0 deleted."));
    assert!(stderr.is_empty(), "empty corpus should not emit stderr output");
}

#[tokio::test]
async fn rag_update_database_streams_indexed_then_unchanged_document_progress() {
    let root = unique_root("progress");
    let doc_dir = root.join("doc");
    std::fs::create_dir_all(&doc_dir).expect("failed to create doc directory");
    std::fs::write(
        doc_dir.join("spec-00001-progress-check.md"),
        "# Progress Check\n\nThis document is indexed.\n",
    )
    .expect("failed to write governed document");

    let mut first_stdout = Vec::new();
    let mut first_stderr = Vec::new();
    run_with_writers(&root, &mut first_stdout, &mut first_stderr)
        .await
        .expect("first update-database run should succeed");

    let mut second_stdout = Vec::new();
    let mut second_stderr = Vec::new();
    run_with_writers(&root, &mut second_stdout, &mut second_stderr)
        .await
        .expect("second update-database run should succeed");

    let first_stdout = String::from_utf8(first_stdout).expect("stdout should be utf-8");
    let second_stdout = String::from_utf8(second_stdout).expect("stdout should be utf-8");

    assert!(
        first_stdout.contains("indexed document=spec-00001-progress-check"),
        "expected indexed progress for the first run: {first_stdout}"
    );
    assert!(
        second_stdout.contains("unchanged document=spec-00001-progress-check"),
        "expected unchanged progress for the second run: {second_stdout}"
    );
    assert!(first_stderr.is_empty(), "successful indexing should not emit stderr output");
    assert!(second_stderr.is_empty(), "unchanged indexing should not emit stderr output");
}
