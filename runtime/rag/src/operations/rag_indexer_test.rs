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

async fn create_package_corpus_file(
    root_dir: &std::path::Path,
    package: &str,
    stem: &str,
    content: &str,
) {
    let doc_dir = root_dir.join(".vector-database").join("packages").join(package).join("doc");
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

// ── Phase C: store reconciliation and deletion ────────────────────────────

#[tokio::test]
async fn run_pass_deletes_rows_for_removed_source_document() {
    let root_dir = unique_fixture_root("removed-doc");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00001-to-remove",
        "---\ntitle: Will be removed\n---\n\n## Section\n\nRemovable content.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());

    let first = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(first.reindexed_count, 1, "document should be indexed on first run");
    assert_eq!(first.deleted_count, 0);

    let doc_path = root_dir.join("doc").join("task-00001-to-remove.md");
    tokio::fs::remove_file(&doc_path).await.unwrap();

    let second = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(second.deleted_count, 1, "removed document should be deleted from store");
    assert_eq!(second.reindexed_count, 0);
    assert_eq!(second.skipped_count, 0);

    let third = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(third.deleted_count, 0, "no deletions when store already reflects empty corpus");
    assert_eq!(third.reindexed_count, 0);
    assert_eq!(third.skipped_count, 0);
}

#[tokio::test]
async fn run_pass_deletion_is_scoped_to_removed_document_only() {
    let root_dir = unique_fixture_root("scoped-deletion");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00001-remove-me",
        "---\ntitle: Remove me\n---\n\n## Section\n\nRemovable.\n",
    )
    .await;
    create_corpus_file(
        &root_dir,
        "task-00002-keep-me",
        "---\ntitle: Keep me\n---\n\n## Section\n\nStable content.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());

    let first = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(first.reindexed_count, 2, "both documents should be indexed");

    let doc_path = root_dir.join("doc").join("task-00001-remove-me.md");
    tokio::fs::remove_file(&doc_path).await.unwrap();

    let second = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(second.deleted_count, 1, "only the removed document should be deleted");
    assert_eq!(second.skipped_count, 1, "the retained document should be skipped as unchanged");
    assert_eq!(second.reindexed_count, 0);
}

#[tokio::test]
async fn run_pass_no_indexed_stems_remain_after_document_removal() {
    use crate::lifecycle::{lancedb_store_dir, query_all_indexed_document_stems};

    let root_dir = unique_fixture_root("no-stems-after-removal");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00001-verify-removal",
        "---\ntitle: Verify removal\n---\n\n## Section\n\nContent.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());
    run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    let store_dir = lancedb_store_dir(&root_dir);
    let stems_before = query_all_indexed_document_stems(&store_dir).await.unwrap();
    assert_eq!(stems_before.len(), 1, "one document stem should be indexed before removal");

    let doc_path = root_dir.join("doc").join("task-00001-verify-removal.md");
    tokio::fs::remove_file(&doc_path).await.unwrap();

    run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    let stems_after = query_all_indexed_document_stems(&store_dir).await.unwrap();
    assert!(stems_after.is_empty(), "no indexed stems should remain after removal");
}

#[tokio::test]
async fn run_pass_changed_document_rows_are_replaced_and_subsequent_run_skips() {
    use crate::lifecycle::{lancedb_store_dir, query_all_indexed_document_stems};

    let root_dir = unique_fixture_root("rows-replaced");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00001-change-test",
        "---\ntitle: Change test\n---\n\n## Section A\n\nOriginal content.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());
    run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    create_corpus_file(
        &root_dir,
        "task-00001-change-test",
        "---\ntitle: Change test\n---\n\n## Section A\n\nUpdated content A.\n\n## Section B\n\nNew section B.\n",
    )
    .await;

    let second = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(second.reindexed_count, 1, "changed document should be re-indexed");
    assert_eq!(second.deleted_count, 0, "re-index must not be counted as a deletion");

    let store_dir = lancedb_store_dir(&root_dir);
    let stems = query_all_indexed_document_stems(&store_dir).await.unwrap();
    assert_eq!(stems.len(), 1, "only one document stem should exist after re-index");
    assert_eq!(stems[0].document_stem, "task-00001-change-test");

    let third = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(third.skipped_count, 1, "re-indexed document must be skipped on subsequent run");
    assert_eq!(third.reindexed_count, 0);
}

// ── Phase D: failure isolation and reporting ──────────────────────────────

/// Embedder that injects a backend failure when any input text contains `fail_marker`.
struct PartiallyFailingEmbedder {
    fail_marker: String,
    dimension: usize,
}

impl PartiallyFailingEmbedder {
    fn new(fail_marker: &str, dimension: usize) -> Self {
        Self { fail_marker: fail_marker.to_owned(), dimension }
    }
}

impl Embedder for PartiallyFailingEmbedder {
    fn model_id(&self) -> &'static str {
        "partially-failing-model"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        if inputs.iter().any(|t| t.contains(self.fail_marker.as_str())) {
            return Err(EmbeddingError::Backend {
                message: format!("injected failure for marker '{}'", self.fail_marker),
            });
        }
        Ok(inputs.iter().map(|_| vec![0.0f32; self.dimension]).collect())
    }
}

#[tokio::test]
async fn per_document_failure_is_recorded_without_aborting_remaining_documents() {
    let root_dir = unique_fixture_root("failure-isolation");
    let embedder = PartiallyFailingEmbedder::new("EMBED_FAIL_MARKER", 4);
    init_store_for_root(&root_dir, &TrackingEmbedder::new("partially-failing-model", 4)).await;

    create_corpus_file(
        &root_dir,
        "task-00001-good-doc",
        "---\ntitle: Good document\n---\n\n## Section\n\nThis content will succeed.\n",
    )
    .await;

    create_corpus_file(
        &root_dir,
        "task-00002-failing-doc",
        "---\ntitle: Failing document\n---\n\n## Section\n\nEMBED_FAIL_MARKER triggers an injected embedding failure.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());
    let result = run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    assert_eq!(result.reindexed_count, 1, "the healthy document must be re-indexed");
    assert_eq!(result.skipped_count, 0);
    assert_eq!(result.failures.len(), 1, "the failing document must be recorded in failures");
    assert!(result.has_failures(), "has_failures must return true for a partial run");

    let failure = &result.failures[0];
    assert_eq!(failure.document_stem, "task-00002-failing-doc");
    assert!(failure.package.is_none());
    assert!(
        failure.error.contains("injected failure") || failure.error.contains("EMBED_FAIL_MARKER"),
        "failure error should contain actionable context: {}",
        failure.error
    );
}

#[tokio::test]
async fn per_document_failure_leaves_healthy_document_indexed_on_subsequent_skip() {
    let root_dir = unique_fixture_root("failure-then-skip");
    let embedder = PartiallyFailingEmbedder::new("EMBED_FAIL_MARKER", 4);
    init_store_for_root(&root_dir, &TrackingEmbedder::new("partially-failing-model", 4)).await;

    create_corpus_file(
        &root_dir,
        "task-00001-persistent-doc",
        "---\ntitle: Persistent document\n---\n\n## Section\n\nThis content remains stable.\n",
    )
    .await;
    create_corpus_file(
        &root_dir,
        "task-00002-failing-doc",
        "---\ntitle: Failing document\n---\n\n## Section\n\nEMBED_FAIL_MARKER triggers failure.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());

    let first = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(first.reindexed_count, 1, "healthy document indexed on first run");
    assert_eq!(first.failures.len(), 1, "failing document recorded on first run");

    let second = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(second.skipped_count, 1, "healthy document skipped on second run — hash unchanged");
    assert_eq!(second.failures.len(), 1, "failing document still fails on second run");
}

#[tokio::test]
async fn clean_run_does_not_report_failures() {
    let root_dir = unique_fixture_root("clean-run");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00001-clean",
        "---\ntitle: Clean\n---\n\n## Section\n\nClean content.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());
    let result = run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    assert!(!result.has_failures(), "a fully successful run must not report failures");
    assert_eq!(result.reindexed_count, 1);
    assert!(result.failures.is_empty());
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

// Phase H: workspace and synchronized package corpora

#[tokio::test]
async fn run_pass_indexes_workspace_and_package_documents_together() {
    use crate::lifecycle::{lancedb_store_dir, query_all_indexed_document_stems};

    let root_dir = unique_fixture_root("workspace-and-package");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00001-workspace-doc",
        "---\ntitle: Workspace doc\n---\n\n## Section\n\nWorkspace content.\n",
    )
    .await;
    create_package_corpus_file(
        &root_dir,
        "shared-docs",
        "task-00002-package-doc",
        "---\ntitle: Package doc\n---\n\n## Section\n\nPackage content.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());
    let result = run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    assert_eq!(result.reindexed_count, 2);
    assert_eq!(result.skipped_count, 0);
    assert!(result.failures.is_empty());

    let stems = query_all_indexed_document_stems(&lancedb_store_dir(&root_dir)).await.unwrap();
    assert!(
        stems
            .iter()
            .any(|stem| stem.package.is_none() && stem.document_stem == "task-00001-workspace-doc")
    );
    assert!(stems.iter().any(|stem| {
        stem.package.as_deref() == Some("shared-docs")
            && stem.document_stem == "task-00002-package-doc"
    }));
}

#[tokio::test]
async fn run_pass_skips_unchanged_package_document_on_repeat_runs() {
    let root_dir = unique_fixture_root("package-skip");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_package_corpus_file(
        &root_dir,
        "shared-docs",
        "task-00003-stable-package-doc",
        "---\ntitle: Stable package doc\n---\n\n## Section\n\nPackage content.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());

    let first = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(first.reindexed_count, 1);

    let texts_after_first = embedder.total_texts_embedded();
    let second = run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    assert_eq!(second.skipped_count, 1);
    assert_eq!(second.reindexed_count, 0);
    assert_eq!(embedder.total_texts_embedded(), texts_after_first);
}

#[tokio::test]
async fn run_pass_deletes_removed_package_document_without_touching_workspace_copy() {
    let root_dir = unique_fixture_root("package-delete");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00004-shared-stem",
        "---\ntitle: Workspace copy\n---\n\n## Section\n\nWorkspace content.\n",
    )
    .await;
    create_package_corpus_file(
        &root_dir,
        "shared-docs",
        "task-00005-package-delete-me",
        "---\ntitle: Package copy\n---\n\n## Section\n\nPackage content.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());
    let first = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(first.reindexed_count, 2);

    let package_doc_path = root_dir
        .join(".vector-database")
        .join("packages")
        .join("shared-docs")
        .join("doc")
        .join("task-00005-package-delete-me.md");
    tokio::fs::remove_file(package_doc_path).await.unwrap();

    let second = run_incremental_indexing_pass(&input, &embedder).await.unwrap();
    assert_eq!(second.deleted_count, 1);
    assert_eq!(second.skipped_count, 1);
    assert_eq!(second.reindexed_count, 0);
}

#[tokio::test]
async fn run_pass_keeps_workspace_and_package_documents_with_same_stem_distinct() {
    use crate::lifecycle::{lancedb_store_dir, query_all_indexed_document_stems};

    let root_dir = unique_fixture_root("same-stem");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    create_corpus_file(
        &root_dir,
        "task-00006-shared-stem",
        "---\ntitle: Workspace version\n---\n\n## Section\n\nWorkspace content.\n",
    )
    .await;
    create_package_corpus_file(
        &root_dir,
        "shared-docs",
        "task-00006-shared-stem",
        "---\ntitle: Package version\n---\n\n## Section\n\nPackage content.\n",
    )
    .await;

    let input = RagIndexerInput::new(root_dir.clone(), RagDefaults::phase_one());
    let result = run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    assert_eq!(result.reindexed_count, 2);
    let stems = query_all_indexed_document_stems(&lancedb_store_dir(&root_dir)).await.unwrap();

    assert_eq!(
        stems.iter().filter(|stem| stem.document_stem == "task-00006-shared-stem").count(),
        2,
        "workspace and package identities must both be preserved"
    );
    assert!(
        stems
            .iter()
            .any(|stem| stem.package.is_none() && stem.document_stem == "task-00006-shared-stem")
    );
    assert!(stems.iter().any(|stem| {
        stem.package.as_deref() == Some("shared-docs")
            && stem.document_stem == "task-00006-shared-stem"
    }));
}

#[tokio::test]
async fn run_pass_reports_missing_package_doc_directory_as_package_structure_issue() {
    let root_dir = unique_fixture_root("missing-package-doc");
    let embedder = TrackingEmbedder::new("fake-model", 4);
    init_store_for_root(&root_dir, &embedder).await;

    let package_root = root_dir.join(".vector-database").join("packages").join("broken-package");
    tokio::fs::create_dir_all(&package_root).await.unwrap();

    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());
    let result = run_incremental_indexing_pass(&input, &embedder).await.unwrap();

    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].package.as_deref(), Some("broken-package"));
    assert!(result.failures[0].error.contains("Package structure issue"));
    assert!(result.has_failures());
}
