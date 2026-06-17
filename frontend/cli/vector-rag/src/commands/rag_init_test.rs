#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let root = std::env::temp_dir().join(format!("vector-rag-init-{label}-{nanos}"));
    std::fs::create_dir_all(root.join(".vector")).expect("failed to create .vector root");
    root
}

#[tokio::test]
async fn rag_init_creates_local_store_from_companion_cli_command() {
    let root = unique_root("create");

    run(&root).await.expect("rag init should create the local store");

    let store_dir = root.join(".vector-database").join("rag").join("lancedb");
    assert!(store_dir.exists(), "expected local LanceDB directory at {store_dir:?}");
}
