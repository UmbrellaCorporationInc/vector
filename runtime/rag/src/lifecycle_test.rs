#![allow(clippy::unwrap_used)]

use super::*;
use crate::{
    Embedder, EmbeddingError, EmbeddingVector, MarkdownChunkingConfig,
    WhitespaceMarkdownTokenCounter, embed_markdown_extraction,
};
use runtime_io::{IoPath, hash_file_content};
use runtime_markdown::{MarkdownDiscoveryRecord, extract_markdown_source};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

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

#[tokio::test]
async fn test_persist_embedded_markdown_chunks_upserts_rows_and_deletes_stale_document_rows() {
    let root_dir = unique_fixture_root("persist-upsert");
    let initial = embedded_fixture(
        "persist-upsert-initial",
        None,
        "spec-00011-rag-plan-implementation",
        "# Title\n\n## First\n\nAlpha body.\n\n## Second\n\nBeta body.\n",
        &DeterministicLifecycleEmbedder::new("BGESmallENV15", 3),
    )
    .await;

    let first = persist_embedded_markdown_chunks(&LanceDbChunkWriteRequest {
        root_dir: root_dir.clone(),
        batch: initial,
    })
    .await
    .unwrap();

    assert_eq!(first.inserted_rows, 2);
    assert_eq!(first.updated_rows, 0);
    assert_eq!(first.deleted_rows, 0);
    assert!(first.created_vector_index);

    let updated = embedded_fixture(
        "persist-upsert-updated",
        None,
        "spec-00011-rag-plan-implementation",
        "# Title\n\n## First\n\nAlpha body updated.\n",
        &DeterministicLifecycleEmbedder::new("BGESmallENV15", 3),
    )
    .await;

    let second = persist_embedded_markdown_chunks(&LanceDbChunkWriteRequest {
        root_dir: root_dir.clone(),
        batch: updated,
    })
    .await
    .unwrap();

    assert_eq!(second.inserted_rows, 1);
    assert_eq!(second.updated_rows, 0);
    assert_eq!(second.deleted_rows, 2);

    let table = open_primary_table(&lancedb_store_dir(&root_dir)).await.unwrap();
    assert_eq!(table.count_rows(None).await.unwrap(), 1);
    assert!(table.list_indices().await.unwrap().iter().any(|index| index.columns == ["vector"]));
    assert_eq!(
        table
            .count_rows(Some(
                "document_stem = 'spec-00011-rag-plan-implementation' AND text = '## First\n\nAlpha body updated.'"
                    .to_owned(),
            ))
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn test_persist_embedded_markdown_chunks_rejects_mixed_embedding_contracts_before_write() {
    let root_dir = unique_fixture_root("persist-contract-mismatch");
    let mut batch = embedded_fixture(
        "persist-contract-mismatch",
        None,
        "spec-00011-rag-plan-implementation",
        "# Title\n\n## First\n\nAlpha body.\n\n## Second\n\nBeta body.\n",
        &DeterministicLifecycleEmbedder::new("BGESmallENV15", 3),
    )
    .await;
    batch.chunks[1].embedding_model = "DifferentModel".to_owned();

    let error = persist_embedded_markdown_chunks(&LanceDbChunkWriteRequest {
        root_dir: root_dir.clone(),
        batch,
    })
    .await
    .unwrap_err();

    assert!(matches!(error, LanceDbStoreError::InvalidRequest { .. }));
    let store_dir = lancedb_store_dir(&root_dir);
    if store_dir.exists() {
        let table = open_primary_table(&store_dir).await.unwrap();
        assert_eq!(table.count_rows(None).await.unwrap(), 0);
    }
}

#[tokio::test]
async fn test_delete_document_chunks_removes_workspace_document_rows() {
    let root_dir = unique_fixture_root("delete-document");
    let batch = embedded_fixture(
        "delete-document",
        None,
        "spec-00011-rag-plan-implementation",
        "# Title\n\n## First\n\nAlpha body.\n",
        &DeterministicLifecycleEmbedder::new("BGESmallENV15", 3),
    )
    .await;
    persist_embedded_markdown_chunks(&LanceDbChunkWriteRequest {
        root_dir: root_dir.clone(),
        batch,
    })
    .await
    .unwrap();

    let status = delete_document_chunks(&LanceDbDocumentDeleteRequest {
        root_dir: root_dir.clone(),
        package: None,
        document_stem: "spec-00011-rag-plan-implementation".to_owned(),
        embedding_model: "BGESmallENV15".to_owned(),
        embedding_dimension: 3,
    })
    .await
    .unwrap();

    assert_eq!(status.deleted_rows, 1);
    let table = open_primary_table(&lancedb_store_dir(&root_dir)).await.unwrap();
    assert_eq!(table.count_rows(None).await.unwrap(), 0);
}

#[tokio::test]
async fn test_persist_embedded_markdown_chunks_keeps_raw_text_inspectable() {
    let root_dir = unique_fixture_root("persist-fts");
    let batch = embedded_fixture(
        "persist-fts",
        Some("shared-docs"),
        "rfc-00038-phase-6-lancedb-integration",
        "# Title\n\n## Errors\n\nPersist exact error strings like LANCEDB_CONTRACT_MISMATCH.\n",
        &DeterministicLifecycleEmbedder::new("BGESmallENV15", 3),
    )
    .await;
    persist_embedded_markdown_chunks(&LanceDbChunkWriteRequest {
        root_dir: root_dir.clone(),
        batch,
    })
    .await
    .unwrap();

    let table = open_primary_table(&lancedb_store_dir(&root_dir)).await.unwrap();
    let persisted_rows = table
        .count_rows(Some(
            "package = 'shared-docs' AND document_stem = 'rfc-00038-phase-6-lancedb-integration' AND text LIKE '%LANCEDB_CONTRACT_MISMATCH%'"
                .to_owned(),
        ))
        .await
        .unwrap();

    assert_eq!(persisted_rows, 1);
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

async fn embedded_fixture(
    name: &str,
    package: Option<&str>,
    document_stem: &str,
    source: &str,
    embedder: &(impl Embedder + Sync),
) -> crate::EmbeddedMarkdownChunkBatch {
    let path = write_fixture_file(name, source).await;
    let content_hash = hash_file_content(&path).await.unwrap();
    let record = MarkdownDiscoveryRecord::new(
        package.map(ToOwned::to_owned),
        document_stem.to_owned(),
        None,
        content_hash,
        path.clone(),
    );
    let extraction = extract_markdown_source(&record, source);
    let _ = fs::remove_file(path.as_path()).await;

    let outcome = embed_markdown_extraction(
        &extraction,
        source,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
        embedder,
    );

    match outcome {
        crate::MarkdownEmbeddingPipelineOutcome::Embedded(batch) => *batch,
        crate::MarkdownEmbeddingPipelineOutcome::Failed(failure) => {
            unreachable!("embedding fixture failed: {failure:?}")
        }
    }
}

async fn write_fixture_file(name: &str, source: &str) -> IoPath {
    let path = IoPath::new(std::env::temp_dir().join(format!(
        "vector-runtime-rag-lifecycle-{name}-{}.md",
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_nanos())
    )));
    if let Some(parent) = path.as_path().parent() {
        fs::create_dir_all(parent).await.unwrap();
    }
    fs::write(path.as_path(), source.as_bytes()).await.unwrap();
    path
}

#[derive(Debug)]
struct DeterministicLifecycleEmbedder {
    model_id: String,
    dimension: usize,
}

impl DeterministicLifecycleEmbedder {
    fn new(model_id: &str, dimension: usize) -> Self {
        Self { model_id: model_id.to_owned(), dimension }
    }
}

impl Embedder for DeterministicLifecycleEmbedder {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        Ok(inputs
            .iter()
            .map(|input| {
                let mut embedding = vec![0.0; self.dimension];
                if let Some(first) = embedding.first_mut() {
                    *first = f32::from(u16::try_from(input.len()).unwrap());
                }
                if let Some(second) = embedding.get_mut(1) {
                    *second = f32::from(u16::try_from(input.split_whitespace().count()).unwrap());
                }
                for (index, value) in embedding.iter_mut().enumerate().skip(2) {
                    *value = f32::from(u16::try_from(index).unwrap());
                }
                embedding
            })
            .collect())
    }
}
