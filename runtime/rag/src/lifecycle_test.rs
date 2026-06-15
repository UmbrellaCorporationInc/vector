#![allow(clippy::unwrap_used)]

use super::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_ensure_lancedb_store_creates_phase_six_database_under_governed_path() {
    let root_dir = unique_fixture_root("create-store");
    let request = fixture_request(root_dir.clone(), "BGESmallENV15", 384);

    let status = ensure_lancedb_store(&request).await.unwrap();

    assert_eq!(status.database_dir, lancedb_store_dir(&root_dir));
    assert_eq!(status.table_name, LANCEDB_PRIMARY_CHUNK_TABLE);
    assert!(status.database_dir.is_dir());
    assert!(status.created_table);
    assert!(status.created_text_index);
}

#[tokio::test]
async fn test_ensure_lancedb_store_is_idempotent_for_repeated_runs() {
    let root_dir = unique_fixture_root("idempotent");
    let request = fixture_request(root_dir.clone(), "BGESmallENV15", 384);

    let first = ensure_lancedb_store(&request).await.unwrap();
    let second = ensure_lancedb_store(&request).await.unwrap();

    assert!(first.created_table);
    assert!(!second.created_table);
}

#[tokio::test]
async fn test_ensure_lancedb_store_rejects_incompatible_embedding_contract() {
    let root_dir = unique_fixture_root("incompatible-contract");
    let baseline = fixture_request(root_dir.clone(), "BGESmallENV15", 384);
    ensure_lancedb_store(&baseline).await.unwrap();

    let incompatible = fixture_request(root_dir, "DifferentModel", 768);
    let error = ensure_lancedb_store(&incompatible).await.unwrap_err();

    assert!(matches!(
        error,
        LanceDbStoreError::IncompatibleStoreContract {
            expected_embedding_model,
            actual_embedding_model,
            expected_embedding_dimension,
            actual_embedding_dimension,
            ..
        } if expected_embedding_model == "DifferentModel"
            && actual_embedding_model == "BGESmallENV15"
            && expected_embedding_dimension == 768
            && actual_embedding_dimension == 384
    ));
}

fn fixture_request(
    root_dir: PathBuf,
    embedding_model: &str,
    embedding_dimension: usize,
) -> LanceDbStoreRequest {
    LanceDbStoreRequest {
        root_dir,
        embedding_model: embedding_model.to_owned(),
        embedding_dimension,
    }
}

fn unique_fixture_root(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_nanos());
    let root = std::env::temp_dir().join(format!("vector-runtime-rag-lifecycle-{name}-{nanos}"));
    std::fs::create_dir_all(root.join(".vector")).unwrap();
    root
}
