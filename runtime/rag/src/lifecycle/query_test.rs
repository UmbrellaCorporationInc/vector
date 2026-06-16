#![allow(clippy::unwrap_used)]

use super::*;
use crate::{
    Embedder, EmbeddingError, EmbeddingVector, MarkdownChunkingConfig,
    WhitespaceMarkdownTokenCounter, embed_markdown_extraction,
};
use crate::{
    LanceDbChunkWriteRequest, LanceDbStoreRequest, ensure_lancedb_store, lancedb_store_dir,
    persist_embedded_markdown_chunks,
};
use runtime_io::{IoPath, hash_file_content};
use runtime_markdown::{MarkdownDiscoveryRecord, extract_markdown_source};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

fn unique_fixture_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let root = std::env::temp_dir().join(format!("vector-rag-query-test-{label}-{nanos}"));
    std::fs::create_dir_all(root.join(".vector")).unwrap();
    root
}

async fn write_fixture_file(name: &str, source: &str) -> IoPath {
    let path = IoPath::new(std::env::temp_dir().join(format!(
        "vector-rag-query-{name}-{}.md",
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos())
    )));
    if let Some(parent) = path.as_path().parent() {
        fs::create_dir_all(parent).await.unwrap();
    }
    fs::write(path.as_path(), source.as_bytes()).await.unwrap();
    path
}

struct FixedEmbedder {
    dimension: usize,
}

impl Embedder for FixedEmbedder {
    fn model_id(&self) -> &'static str {
        "query-test-model"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        Ok(inputs.iter().map(|_| vec![0.0f32; self.dimension]).collect())
    }
}

async fn insert_fixture_document(root_dir: &std::path::Path, document_stem: &str, source: &str) {
    let embedder = FixedEmbedder { dimension: 3 };
    ensure_lancedb_store(&LanceDbStoreRequest {
        root_dir: root_dir.to_path_buf(),
        embedding_model: embedder.model_id().to_owned(),
        embedding_dimension: embedder.dimension(),
    })
    .await
    .unwrap();

    let path = write_fixture_file(document_stem, source).await;
    let hash = hash_file_content(&path).await.unwrap();
    let record = MarkdownDiscoveryRecord::new(None, document_stem.to_owned(), None, hash, path);
    let extraction = extract_markdown_source(&record, source);
    let outcome = embed_markdown_extraction(
        &extraction,
        source,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
        &embedder,
    );
    let batch = match outcome {
        crate::MarkdownEmbeddingPipelineOutcome::Embedded(b) => *b,
        crate::MarkdownEmbeddingPipelineOutcome::Failed(f) => {
            unreachable!("fixture embed failed: {f:?}")
        }
    };
    persist_embedded_markdown_chunks(&LanceDbChunkWriteRequest {
        root_dir: root_dir.to_path_buf(),
        batch,
    })
    .await
    .unwrap();
}

// ── query_document_hash_indexed ───────────────────────────────────────────────

#[tokio::test]
async fn query_document_hash_indexed_returns_false_when_store_absent() {
    let store_dir = std::env::temp_dir().join("vector-rag-query-absent-store");
    let result =
        query_document_hash_indexed(&store_dir, None, "task-00001-example", "deadbeef").await;
    assert!(!result.unwrap());
}

#[tokio::test]
async fn query_document_hash_indexed_returns_false_for_unknown_hash() {
    let root = unique_fixture_root("unknown-hash");
    let source = "---\ntitle: Doc\n---\n\n## Section\n\nContent.\n";
    insert_fixture_document(&root, "task-00001-known", source).await;

    let store_dir = lancedb_store_dir(&root);
    let result =
        query_document_hash_indexed(&store_dir, None, "task-00001-known", "wrong-hash").await;
    assert!(!result.unwrap(), "wrong hash should return false");
}

#[tokio::test]
async fn query_document_hash_indexed_returns_true_for_stored_hash() {
    use runtime_io::hash_file_content;

    let root = unique_fixture_root("stored-hash");
    let source = "---\ntitle: Doc\n---\n\n## Section\n\nKnown content.\n";
    let path = write_fixture_file("stored-hash-doc", source).await;
    let hash = hash_file_content(&path).await.unwrap();

    insert_fixture_document(&root, "task-00001-stored", source).await;

    let store_dir = lancedb_store_dir(&root);
    let result =
        query_document_hash_indexed(&store_dir, None, "task-00001-stored", hash.as_hex()).await;
    assert!(result.unwrap(), "stored hash must be found");
}

// ── query_document_chunk_embeddings ──────────────────────────────────────────

#[tokio::test]
async fn query_document_chunk_embeddings_returns_empty_when_store_absent() {
    let store_dir = std::env::temp_dir().join("vector-rag-query-absent-embeddings");
    let result = query_document_chunk_embeddings(&store_dir, None, "task-00001-example").await;
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn query_document_chunk_embeddings_returns_map_keyed_by_chunk_hash() {
    let root = unique_fixture_root("chunk-embeddings");
    let source = "---\ntitle: Doc\n---\n\n## Section\n\nEmbedding content.\n";
    insert_fixture_document(&root, "task-00001-embed", source).await;

    let store_dir = lancedb_store_dir(&root);
    let embeddings =
        query_document_chunk_embeddings(&store_dir, None, "task-00001-embed").await.unwrap();

    assert!(!embeddings.is_empty(), "must return chunk embeddings for indexed document");
    for vector in embeddings.values() {
        assert_eq!(vector.len(), 3, "vector dimension must match store contract");
    }
}

// ── query_all_indexed_document_stems ─────────────────────────────────────────

#[tokio::test]
async fn query_all_indexed_document_stems_returns_empty_when_store_absent() {
    let store_dir = std::env::temp_dir().join("vector-rag-query-absent-stems");
    let result = query_all_indexed_document_stems(&store_dir).await;
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn query_all_indexed_document_stems_returns_one_stem_per_indexed_document() {
    let root = unique_fixture_root("all-stems");
    let source_a = "---\ntitle: A\n---\n\n## Section\n\nAlpha.\n";
    let source_b = "---\ntitle: B\n---\n\n## Section\n\nBeta.\n";
    insert_fixture_document(&root, "task-00001-alpha", source_a).await;
    insert_fixture_document(&root, "task-00002-beta", source_b).await;

    let store_dir = lancedb_store_dir(&root);
    let stems = query_all_indexed_document_stems(&store_dir).await.unwrap();

    assert_eq!(stems.len(), 2, "must return one stem per distinct indexed document");
    let stem_names: Vec<&str> = stems.iter().map(|s| s.document_stem.as_str()).collect();
    assert!(stem_names.contains(&"task-00001-alpha"));
    assert!(stem_names.contains(&"task-00002-beta"));
}

// ── delete_indexed_document ───────────────────────────────────────────────────

#[tokio::test]
async fn delete_indexed_document_returns_zero_when_store_absent() {
    let store_dir = std::env::temp_dir().join("vector-rag-query-absent-delete");
    let result = delete_indexed_document(&store_dir, None, "task-00001-example").await;
    assert_eq!(result.unwrap(), 0);
}

#[tokio::test]
async fn delete_indexed_document_removes_rows_and_leaves_store_empty() {
    let root = unique_fixture_root("delete-doc");
    let source = "---\ntitle: Delete me\n---\n\n## Section\n\nRemovable.\n";
    insert_fixture_document(&root, "task-00001-removable", source).await;

    let store_dir = lancedb_store_dir(&root);
    let deleted = delete_indexed_document(&store_dir, None, "task-00001-removable").await.unwrap();
    assert!(deleted > 0, "at least one row must be deleted");

    let stems = query_all_indexed_document_stems(&store_dir).await.unwrap();
    assert!(stems.is_empty(), "no stems must remain after deletion");
}
