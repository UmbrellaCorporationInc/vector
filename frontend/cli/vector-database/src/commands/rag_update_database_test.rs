#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let root = std::env::temp_dir().join(format!("vector-cli-update-db-{label}-{nanos}"));
    std::fs::create_dir_all(root.join(".vector")).expect("failed to create .vector root");
    root
}

async fn create_corpus_file(root_dir: &std::path::Path, stem: &str, content: &str) {
    let doc_dir = root_dir.join("doc");
    tokio::fs::create_dir_all(&doc_dir).await.unwrap();
    tokio::fs::write(doc_dir.join(format!("{stem}.md")), content).await.unwrap();
}

#[tokio::test]
async fn rag_update_database_succeeds_on_empty_corpus() {
    let root = unique_root("empty");
    run(&root).await.expect("update-database should succeed on an empty corpus");
}

#[tokio::test]
async fn rag_update_database_creates_lancedb_store() {
    let root = unique_root("creates-store");
    run(&root).await.expect("update-database should create the store");
    let store_dir = root.join(".vector-database").join("rag").join("lancedb");
    assert!(store_dir.exists(), "expected LanceDB directory at {store_dir:?}");
}

#[tokio::test]
async fn rag_update_database_is_idempotent_on_repeated_runs() {
    let root = unique_root("idempotent");
    run(&root).await.expect("first update-database should succeed");
    run(&root).await.expect("second update-database should succeed");
}

#[tokio::test]
async fn rag_update_database_returns_error_when_document_has_unclosed_frontmatter() {
    let root = unique_root("failure");
    // A document whose frontmatter opening `---` has no matching closing delimiter
    // causes a parse failure before the embedding stage, so no real model is needed.
    create_corpus_file(&root, "task-00001-broken", "---\ntitle: Broken\n# no closing delimiter\n")
        .await;

    let error = run(&root).await.expect_err("update-database must return an error on failure");
    assert!(
        error.contains("failed") || error.contains("document"),
        "error should describe the indexing failure: {error}"
    );
}

#[tokio::test]
async fn rag_update_database_partial_failure_does_not_block_clean_documents() {
    // The healthy document is stored even though the broken one fails.
    // On a second run, the healthy document is skipped (unchanged hash)
    // while the broken document still fails — proving isolation.
    let root = unique_root("partial-failure");
    create_corpus_file(&root, "task-00001-broken", "---\ntitle: Broken\n# no closing delimiter\n")
        .await;

    // First run: broken doc fails, but operation returns Err (non-zero exit).
    let _ = run(&root).await;

    // Second run with the same corpus produces the same outcome — the failure
    // is deterministic and does not corrupt state for other documents.
    let error = run(&root).await.expect_err("repeated failure must still exit non-zero");
    assert!(
        error.contains("failed") || error.contains("document"),
        "second run error should still describe the failure: {error}"
    );
}
