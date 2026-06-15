#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use runtime_rag::InitRagStoreInput;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let root = std::env::temp_dir().join(format!("vector-cli-rag-init-{label}-{nanos}"));
    std::fs::create_dir_all(root.join(".vector")).expect("failed to create .vector root");
    root
}

#[tokio::test]
async fn rag_init_creates_phase_six_store_from_cli_command() {
    let root = unique_root("create");

    run(&root).await.expect("rag init should create the local store");

    let store_dir = root.join(".vector-database").join("rag").join("lancedb");
    assert!(store_dir.exists(), "expected local LanceDB directory at {store_dir:?}");
}

#[tokio::test]
async fn rag_init_is_idempotent_for_repeated_cli_runs() {
    let root = unique_root("idempotent");

    run(&root).await.expect("first rag init should succeed");
    run(&root).await.expect("second rag init should succeed");

    let store_dir = root.join(".vector-database").join("rag").join("lancedb");
    assert!(store_dir.exists(), "expected local LanceDB directory at {store_dir:?}");
}

#[tokio::test]
async fn rag_init_surfaces_actionable_error_for_incompatible_store_contract() {
    let root = unique_root("incompatible");
    let incompatible = InitRagStoreInput::new(root.clone(), "DifferentModel".to_owned(), 768);
    let (_cancel, mut receiver) = PluginDispatcher::new(InitRagStoreOp::new())
        .input(incompatible)
        .build()
        .expect("dispatcher build failed");
    receiver.recv().await.unwrap().unwrap();

    let error = run(&root).await.unwrap_err();

    assert!(
        error.contains("incompatible") || error.contains("DifferentModel"),
        "expected actionable compatibility error, got: {error}"
    );
}
