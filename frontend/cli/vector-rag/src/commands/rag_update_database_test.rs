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

    run(&root).await.expect("update-database should succeed on an empty corpus");
}
