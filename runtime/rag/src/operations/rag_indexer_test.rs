#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::{
    Embedder, EmbeddingError, EmbeddingVector, ensure_lancedb_store, lifecycle::LanceDbStoreRequest,
};
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_fixture_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    std::env::temp_dir().join(format!("vector-rag-indexer-test-{label}-{nanos}"))
}

/// Fake embedder that counts calls and records inputs for assertions.
#[derive(Debug, Clone)]
struct TrackingEmbedder {
    model_id: String,
    dimension: usize,
    embedded_texts: Arc<Mutex<Vec<String>>>,
}

impl TrackingEmbedder {
    fn new(model_id: &str, dimension: usize) -> Self {
        Self {
            model_id: model_id.to_owned(),
            dimension,
            embedded_texts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn total_texts_embedded(&self) -> usize {
        self.embedded_texts.lock().unwrap().len()
    }
}

impl Embedder for TrackingEmbedder {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        self.embedded_texts.lock().unwrap().extend(inputs.iter().map(ToString::to_string));
        Ok(inputs
            .iter()
            .map(|input| {
                let mut v = vec![0.0f32; self.dimension];
                if let Some(first) = v.first_mut() {
                    *first = u16::try_from(input.len()).unwrap_or(u16::MAX).into();
                }
                v
            })
            .collect())
    }
}

async fn create_corpus_file(root_dir: &std::path::Path, stem: &str, content: &str) {
    let doc_dir = root_dir.join("doc");
    tokio::fs::create_dir_all(&doc_dir).await.unwrap();
    tokio::fs::write(doc_dir.join(format!("{stem}.md")), content).await.unwrap();
}

async fn init_store_for_root(root_dir: &std::path::Path, embedder: &TrackingEmbedder) {
    ensure_lancedb_store(&LanceDbStoreRequest {
        root_dir: root_dir.to_path_buf(),
        embedding_model: embedder.model_id().to_owned(),
        embedding_dimension: embedder.dimension(),
    })
    .await
    .unwrap();
}

// ── existing contract tests ────────────────────────────────────────────────

#[tokio::test]
async fn rag_indexer_op_delivers_empty_index_result_through_dispatcher() {
    let root_dir = unique_fixture_root("empty-result");
    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut receiver) = PluginDispatcher::new(RagIndexerOp::new())
        .input(input)
        .build()
        .expect("dispatcher build failed");

    let result = receiver.recv().await.expect("channel error").expect("no output from operation");

    assert_eq!(result.skipped_count, 0);
    assert_eq!(result.reindexed_count, 0);
    assert_eq!(result.deleted_count, 0);
    assert!(result.failures.is_empty());
    assert!(!result.has_failures());
}

#[tokio::test]
async fn rag_indexer_op_receiver_is_none_after_single_output() {
    let root_dir = unique_fixture_root("single-output");
    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut rx) =
        PluginDispatcher::new(RagIndexerOp::new()).input(input).build().unwrap();

    let first = rx.recv().await;
    let second = rx.recv().await;

    assert!(first.is_ok());
    assert!(first.unwrap().is_some(), "expected one output");
    assert!(second.is_ok(), "channel error on second recv");
    assert!(second.unwrap().is_none(), "channel must be closed after single output");
}

#[test]
fn index_result_has_failures_returns_false_for_empty_failures() {
    let result = IndexResult::default();
    assert!(!result.has_failures());
}

#[test]
fn index_result_has_failures_returns_true_when_failures_present() {
    let result = IndexResult {
        failures: vec![IndexFailureRecord {
            package: None,
            document_stem: "task-00001-example".to_owned(),
            error: "extraction failed".to_owned(),
        }],
        ..IndexResult::default()
    };
    assert!(result.has_failures());
}

#[test]
fn index_failure_record_carries_package_document_stem_and_error() {
    let record = IndexFailureRecord {
        package: Some("my-pkg".to_owned()),
        document_stem: "rfc-00001-my-rfc".to_owned(),
        error: "chunking failed: oversized block".to_owned(),
    };
    assert_eq!(record.package.as_deref(), Some("my-pkg"));
    assert_eq!(record.document_stem, "rfc-00001-my-rfc");
    assert!(record.error.contains("oversized"));
}

// ── Phase B: document-level skip ──────────────────────────────────────────

#[tokio::test]
async fn run_pass_indexes_new_document_and_increments_reindexed_count() {
    let root_dir = unique_fixture_root("index-new-doc");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00001-example",
        "---\ntitle: Example\n---\n\n## Section\n\nHello world.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());
    let result = run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    assert_eq!(result.reindexed_count, 1);
    assert_eq!(result.skipped_count, 0);
    assert!(result.failures.is_empty());
    assert!(embedder.total_texts_embedded() > 0, "embedder should have been called");
}

#[tokio::test]
async fn run_pass_skips_document_when_content_hash_is_unchanged() {
    let root_dir = unique_fixture_root("skip-unchanged-doc");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    let content = "---\ntitle: Stable\n---\n\n## Section\n\nUnchanged body.\n";
    create_corpus_file(&root_dir, "task-00001-stable", content).await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());

    let first_result = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(first_result.reindexed_count, 1, "first run should index");
    assert_eq!(first_result.skipped_count, 0);

    let texts_after_first = embedder.total_texts_embedded();

    let second_result = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(second_result.skipped_count, 1, "second run should skip unchanged document");
    assert_eq!(second_result.reindexed_count, 0);

    assert_eq!(
        embedder.total_texts_embedded(),
        texts_after_first,
        "embedder must not be called for skipped document"
    );
}

#[tokio::test]
async fn run_pass_is_idempotent_for_unchanged_corpus() {
    let root_dir = unique_fixture_root("idempotent-corpus");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "rfc-00001-idempotent",
        "---\ntitle: Idempotent\n---\n\n## Alpha\n\nAlpha body.\n\n## Beta\n\nBeta body.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());
    let _ = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    let _ = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    let result = run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    assert_eq!(result.skipped_count, 1, "third run skips unchanged document");
    assert_eq!(result.reindexed_count, 0);
}

// ── Phase B: chunk-level embedding skip ───────────────────────────────────

#[tokio::test]
async fn run_pass_reuses_stored_embeddings_for_unchanged_chunks_on_document_change() {
    let root_dir = unique_fixture_root("chunk-reuse");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    let initial_content = "---\ntitle: Doc\n---\n\n## Section A\n\nAlpha body unchanged.\n\n## Section B\n\nBeta body initial.\n";
    create_corpus_file(&root_dir, "task-00001-chunk-reuse", initial_content).await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());
    let first = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(first.reindexed_count, 1);
    let texts_after_first = embedder.total_texts_embedded();

    let updated_content = "---\ntitle: Doc\n---\n\n## Section A\n\nAlpha body unchanged.\n\n## Section B\n\nBeta body updated!\n";
    create_corpus_file(&root_dir, "task-00001-chunk-reuse", updated_content).await;

    let second = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(second.reindexed_count, 1, "document should be re-indexed");
    assert_eq!(second.skipped_count, 0);

    let newly_embedded = embedder.total_texts_embedded() - texts_after_first;
    assert_eq!(
        newly_embedded, 1,
        "only the changed chunk should be re-embedded; unchanged chunk must reuse stored embedding"
    );
}

// ── Phase B: deterministic row identity ───────────────────────────────────

#[tokio::test]
async fn chunk_ids_are_stable_and_repeated_runs_skip_unchanged_corpus() {
    let root_dir = unique_fixture_root("stable-chunk-id");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00001-stable-id",
        "---\ntitle: Stable\n---\n\n## Section\n\nDeterministic content.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());
    let first = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(first.reindexed_count, 1);

    let second = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(second.skipped_count, 1, "second run must skip when content is unchanged");
    assert_eq!(second.reindexed_count, 0);

    let third = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(third.skipped_count, 1, "third run must also skip; row identity is deterministic");
    assert_eq!(third.reindexed_count, 0);
}
