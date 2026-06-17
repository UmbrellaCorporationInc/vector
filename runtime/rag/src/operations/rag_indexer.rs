//! Plugin operation for Phase 7 incremental indexing.

use crate::{
    EmbeddedMarkdownChunkBatch, EmbeddedMarkdownChunkRecord, Embedder, EmbeddingVector,
    MarkdownChunkRecord, MarkdownChunkingConfig, RagDefaults, WhitespaceMarkdownTokenCounter,
    lancedb_store_dir,
    lifecycle::{
        LanceDbChunkWriteRequest, LanceDbStoreError, StoredChunkEmbeddings,
        delete_indexed_document, query_all_indexed_document_stems, query_document_chunk_embeddings,
        query_document_hash_indexed,
    },
    persist_embedded_markdown_chunks,
    pipeline::{MarkdownChunkingPipelineOutcome, chunk_markdown_extraction},
};
use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_markdown::{
    MarkdownDiscoveryFailure, MarkdownDiscoveryIssue, MarkdownDiscoveryRecord,
    MarkdownDiscoveryRequest, MarkdownExtractionOutcome, PackageMarkdownRoot,
    discover_markdown_files, extract_markdown_source,
};
use serde::{Deserialize, Serialize};
use std::future::{Future, ready};
use std::path::PathBuf;

/// Per-document indexing failure record.
///
/// # DTO(indexing failure boundary consumed by `IndexResult` callers)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexFailureRecord {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Actionable error message for the failed document.
    pub error: String,
}

/// Indexing run summary returned by `RagIndexerOp` and forwarded by `IndexWorkspaceOp`.
///
/// # DTO(indexing result boundary consumed by CLI and other operation callers)
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexResult {
    /// Number of documents skipped because their content hash was unchanged.
    pub skipped_count: usize,
    /// Number of documents successfully re-indexed.
    pub reindexed_count: usize,
    /// Number of documents removed from the store during this run.
    pub deleted_count: usize,
    /// Per-document failures that did not abort the indexing run.
    pub failures: Vec<IndexFailureRecord>,
}

/// Input for the `rag_indexer` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct RagIndexerInput {
    /// Workspace root used to resolve corpus paths and the `LanceDB` storage path.
    pub root_dir: PathBuf,
    /// Phase 1 RAG defaults governing the embedding model and corpus paths.
    pub config: RagDefaults,
}

async fn rag_indexer(
    input: RagIndexerInput,
    output: &mut impl PluginSender<IndexResult>,
) -> RuntimeResult<()> {
    use runtime_core::RuntimeError;

    let embedder = LazyFastembedEmbedder::new();
    let mut ignore_progress = NoopIndexingProgressSink;
    let result = run_incremental_indexing_pass(&input, &embedder, &mut ignore_progress)
        .await
        .map_err(|error| RuntimeError::operation(error.to_string()))?;

    output.send(result).await
}

/// Embedder that defers model initialization until `embed_batch` is first called.
///
/// Skipping initialization on runs with no documents to embed avoids a model
/// download when the corpus is empty or all documents are unchanged.
pub(crate) struct LazyFastembedEmbedder {
    inner:
        std::sync::Mutex<Option<std::sync::Arc<crate::embedding::FastembedBgeSmallEnV15Embedder>>>,
}

impl LazyFastembedEmbedder {
    pub(crate) const fn new() -> Self {
        Self { inner: std::sync::Mutex::new(None) }
    }
}

/// Incremental indexing progress update emitted while a workspace run is active.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexingProgressUpdate {
    /// Discovery of governed Markdown files has started.
    DiscoveringDocuments,
    /// Discovery completed and document indexing is about to begin.
    IndexingDocuments {
        /// Number of governed Markdown documents discovered in this run.
        discovered_documents: usize,
    },
    /// One document was indexed or re-indexed.
    Indexed {
        /// Package identity, or `None` for a workspace-local document.
        package: Option<String>,
        /// Governed document stem in `<doc-type>-<code>-<slug>` form.
        document_stem: String,
    },
    /// One unchanged document was skipped.
    Unchanged {
        /// Package identity, or `None` for a workspace-local document.
        package: Option<String>,
        /// Governed document stem in `<doc-type>-<code>-<slug>` form.
        document_stem: String,
    },
    /// One stale indexed document was deleted from the local store.
    Deleted {
        /// Package identity, or `None` for a workspace-local document.
        package: Option<String>,
        /// Governed document stem in `<doc-type>-<code>-<slug>` form.
        document_stem: String,
    },
    /// One document-level failure was recorded.
    Failed {
        /// Package identity, or `None` for a workspace-local document.
        package: Option<String>,
        /// Governed document stem in `<doc-type>-<code>-<slug>` form.
        document_stem: String,
        /// Actionable document-level failure message.
        error: String,
    },
}

/// Async sink for incremental indexing progress events.
pub(crate) trait IndexingProgressSink {
    fn emit(&mut self, progress: IndexingProgressUpdate) -> impl Future<Output = ()> + Send;
}

pub(crate) struct NoopIndexingProgressSink;

impl IndexingProgressSink for NoopIndexingProgressSink {
    fn emit(&mut self, _progress: IndexingProgressUpdate) -> impl Future<Output = ()> + Send {
        ready(())
    }
}

impl crate::Embedder for LazyFastembedEmbedder {
    fn model_id(&self) -> &str {
        crate::defaults::EMBEDDING_MODEL_IDENTIFIER
    }

    fn dimension(&self) -> usize {
        crate::defaults::EMBEDDING_DIMENSION
    }

    fn embed_batch(
        &self,
        inputs: &[&str],
    ) -> Result<Vec<crate::EmbeddingVector>, crate::EmbeddingError> {
        let embedder = {
            let mut guard = self.inner.lock().map_err(|_| crate::EmbeddingError::Backend {
                message: "lazy embedder lock was poisoned".to_owned(),
            })?;
            if guard.is_none() {
                *guard = Some(std::sync::Arc::new(
                    crate::embedding::FastembedBgeSmallEnV15Embedder::try_new()?,
                ));
            }
            guard
                .as_ref()
                .ok_or_else(|| crate::EmbeddingError::Backend {
                    message: "lazy embedder did not initialize".to_owned(),
                })?
                .clone()
        }; // mutex is released before calling embed_batch
        embedder.embed_batch(inputs)
    }
}

/// Run the incremental indexing pass with an injectable embedder.
///
/// Discovers all governed Markdown documents under the corpus roots, skips
/// documents whose persisted `document_hash` matches the current file hash,
/// and re-embeds only chunks whose `chunk_hash` is new or changed.
pub(crate) async fn run_incremental_indexing_pass(
    input: &RagIndexerInput,
    embedder: &(impl Embedder + Sync),
    progress: &mut impl IndexingProgressSink,
) -> Result<IndexResult, LanceDbStoreError> {
    let workspace_doc_root = input.root_dir.join(input.config.workspace_corpus_root());
    let workspace_doc_roots =
        if workspace_doc_root.exists() { vec![workspace_doc_root] } else { Vec::new() };
    let package_roots = discover_synchronized_package_roots(&input.root_dir, input.config).await?;
    let discovery_request = MarkdownDiscoveryRequest::new(workspace_doc_roots, package_roots);
    progress.emit(IndexingProgressUpdate::DiscoveringDocuments).await;
    let report = match discover_markdown_files(&discovery_request).await {
        Ok(report) => report,
        Err(MarkdownDiscoveryFailure::WorkspaceDiscovery { .. }) => {
            return Ok(IndexResult::default());
        }
        Err(error) => {
            return Err(LanceDbStoreError::InvalidRequest { message: error.to_string() });
        }
    };

    let store_dir = lancedb_store_dir(&input.root_dir);
    let mut result = IndexResult::default();
    progress
        .emit(IndexingProgressUpdate::IndexingDocuments {
            discovered_documents: report.records().len(),
        })
        .await;
    append_discovery_issues(report.issues(), &mut result, progress).await;

    // Phase C: delete rows for source documents removed from the corpus.
    let discovered: std::collections::HashSet<(Option<String>, String)> = report
        .records()
        .iter()
        .map(|r| {
            (r.package().map(std::borrow::ToOwned::to_owned), r.governed_document_stem().to_owned())
        })
        .collect();

    for stem in query_all_indexed_document_stems(&store_dir).await? {
        if !discovered.contains(&(stem.package.clone(), stem.document_stem.clone())) {
            delete_indexed_document(&store_dir, stem.package.as_deref(), &stem.document_stem)
                .await?;
            result.deleted_count += 1;
            progress
                .emit(IndexingProgressUpdate::Deleted {
                    package: stem.package,
                    document_stem: stem.document_stem,
                })
                .await;
        }
    }

    for record in report.records() {
        if let Err(failure) =
            index_document(record, &store_dir, embedder, &mut result, progress).await
        {
            progress
                .emit(IndexingProgressUpdate::Failed {
                    package: failure.package.clone(),
                    document_stem: failure.document_stem.clone(),
                    error: failure.error.clone(),
                })
                .await;
            result.failures.push(failure);
        }
    }

    Ok(result)
}

async fn discover_synchronized_package_roots(
    root_dir: &std::path::Path,
    config: RagDefaults,
) -> Result<Vec<PackageMarkdownRoot>, LanceDbStoreError> {
    let package_storage_root = root_dir.join(config.package_storage_root());
    if !package_storage_root.exists() {
        return Ok(Vec::new());
    }

    let mut entries = tokio::fs::read_dir(&package_storage_root).await.map_err(|error| {
        LanceDbStoreError::InvalidRequest {
            message: format!(
                "failed to read synchronized package directory '{}': {error}",
                package_storage_root.display()
            ),
        }
    })?;

    let mut package_roots = Vec::new();
    while let Some(entry) =
        entries.next_entry().await.map_err(|error| LanceDbStoreError::InvalidRequest {
            message: format!(
                "failed to enumerate synchronized package directory '{}': {error}",
                package_storage_root.display()
            ),
        })?
    {
        let file_type =
            entry.file_type().await.map_err(|error| LanceDbStoreError::InvalidRequest {
                message: format!(
                    "failed to inspect synchronized package entry '{}': {error}",
                    entry.path().display()
                ),
            })?;
        if !file_type.is_dir() {
            continue;
        }

        let package_name = entry.file_name().to_string_lossy().into_owned();
        let doc_root = entry.path().join(config.package_document_dir());
        package_roots.push(PackageMarkdownRoot::new(package_name, doc_root));
    }

    package_roots.sort_by(|left, right| left.package().cmp(right.package()));
    Ok(package_roots)
}

async fn append_discovery_issues(
    issues: &[MarkdownDiscoveryIssue],
    result: &mut IndexResult,
    progress: &mut impl IndexingProgressSink,
) {
    for failure in issues.iter().map(discovery_issue_to_failure) {
        progress
            .emit(IndexingProgressUpdate::Failed {
                package: failure.package.clone(),
                document_stem: failure.document_stem.clone(),
                error: failure.error.clone(),
            })
            .await;
        result.failures.push(failure);
    }
}

fn discovery_issue_to_failure(issue: &MarkdownDiscoveryIssue) -> IndexFailureRecord {
    match issue {
        MarkdownDiscoveryIssue::PackageStructure { package, doc_root, message } => {
            make_document_failure(
                Some(package.as_str()),
                doc_root.as_path().file_name().and_then(|value| value.to_str()).unwrap_or("doc"),
                format!(
                    "Package structure issue for '{}': {} ({message})",
                    package,
                    doc_root.as_path().display()
                ),
            )
        }
        MarkdownDiscoveryIssue::InvalidGovernedDocumentStem { package, path, stem } => {
            make_document_failure(
                package.as_deref(),
                stem,
                format!(
                    "Invalid governed document stem for '{}': {}",
                    path.as_path().display(),
                    stem
                ),
            )
        }
        MarkdownDiscoveryIssue::ContentHash { package, path, message } => make_document_failure(
            package.as_deref(),
            path.as_path().file_stem().and_then(|value| value.to_str()).unwrap_or("<unknown>"),
            format!("Failed to hash '{}': {message}", path.as_path().display()),
        ),
        _ => make_document_failure(
            None,
            "<discovery-issue>",
            "Unsupported Markdown discovery issue reported by runtime-markdown",
        ),
    }
}

fn make_document_failure(
    package: Option<&str>,
    document_stem: &str,
    error: impl std::fmt::Display,
) -> IndexFailureRecord {
    IndexFailureRecord {
        package: package.map(std::borrow::ToOwned::to_owned),
        document_stem: document_stem.to_owned(),
        error: error.to_string(),
    }
}

async fn index_document(
    record: &MarkdownDiscoveryRecord,
    store_dir: &std::path::Path,
    embedder: &(impl Embedder + Sync),
    result: &mut IndexResult,
    progress: &mut impl IndexingProgressSink,
) -> Result<(), IndexFailureRecord> {
    let package = record.package();
    let document_stem = record.governed_document_stem();
    let document_hash = record.content_hash().as_hex();

    if query_document_hash_indexed(store_dir, package, document_stem, document_hash)
        .await
        .map_err(|e| make_document_failure(package, document_stem, e))?
    {
        result.skipped_count += 1;
        progress
            .emit(IndexingProgressUpdate::Unchanged {
                package: package.map(std::borrow::ToOwned::to_owned),
                document_stem: document_stem.to_owned(),
            })
            .await;
        return Ok(());
    }

    let stored_embeddings = query_document_chunk_embeddings(store_dir, package, document_stem)
        .await
        .map_err(|e| make_document_failure(package, document_stem, e))?;

    let source =
        tokio::fs::read_to_string(record.internal_read_path().as_path()).await.map_err(|e| {
            make_document_failure(
                package,
                document_stem,
                format!("Failed to read '{document_stem}': {e}"),
            )
        })?;

    let extraction_outcome = extract_markdown_source(record, &source);

    let batch = build_embedded_batch(
        &extraction_outcome,
        &source,
        document_stem,
        embedder,
        &stored_embeddings,
    )
    .map_err(|e| make_document_failure(package, document_stem, e))?;

    let root_dir = root_dir_from_store_dir(store_dir);

    // Phase C: explicit delete-before-write so a crash between delete and write
    // leaves the store in an empty state for this document, not a mixed state.
    delete_indexed_document(store_dir, package, document_stem)
        .await
        .map_err(|e| make_document_failure(package, document_stem, e))?;

    persist_embedded_markdown_chunks(&LanceDbChunkWriteRequest { root_dir, batch })
        .await
        .map_err(|e| make_document_failure(package, document_stem, e))?;

    result.reindexed_count += 1;
    progress
        .emit(IndexingProgressUpdate::Indexed {
            package: package.map(std::borrow::ToOwned::to_owned),
            document_stem: document_stem.to_owned(),
        })
        .await;
    Ok(())
}

fn build_embedded_batch(
    extraction_outcome: &MarkdownExtractionOutcome,
    source: &str,
    document_stem: &str,
    embedder: &impl Embedder,
    stored_embeddings: &StoredChunkEmbeddings,
) -> Result<EmbeddedMarkdownChunkBatch, LanceDbStoreError> {
    let extraction = match extraction_outcome {
        MarkdownExtractionOutcome::Extracted(record) => record,
        MarkdownExtractionOutcome::Failed(failure) => {
            return Err(LanceDbStoreError::InvalidRequest {
                message: format!(
                    "Extraction failed for '{document_stem}': {}",
                    failure.error.message
                ),
            });
        }
        _ => {
            return Err(LanceDbStoreError::InvalidRequest {
                message: format!("Unsupported extraction outcome for '{document_stem}'"),
            });
        }
    };

    let config = MarkdownChunkingConfig::phase_four_defaults();
    let token_counter = WhitespaceMarkdownTokenCounter;
    let chunk_batch =
        match chunk_markdown_extraction(extraction_outcome, source, config, &token_counter) {
            MarkdownChunkingPipelineOutcome::Chunked(batch) => batch,
            MarkdownChunkingPipelineOutcome::Failed(failure) => {
                return Err(LanceDbStoreError::InvalidRequest {
                    message: format!("Chunking failed for '{document_stem}': {}", failure.error),
                });
            }
        };

    let embedded_chunks =
        embed_chunks_with_reuse(&chunk_batch.chunks, embedder, stored_embeddings)?;

    Ok(EmbeddedMarkdownChunkBatch {
        package: extraction.package.clone(),
        document_stem: extraction.document_stem.clone(),
        document_hash: extraction.document_hash.clone(),
        extraction: extraction.clone(),
        chunks: embedded_chunks,
    })
}

fn embed_chunks_with_reuse(
    chunks: &[MarkdownChunkRecord],
    embedder: &impl Embedder,
    stored_embeddings: &StoredChunkEmbeddings,
) -> Result<Vec<EmbeddedMarkdownChunkRecord>, LanceDbStoreError> {
    let mut new_chunk_indices: Vec<usize> = Vec::new();
    let mut new_chunk_texts: Vec<&str> = Vec::new();

    for (index, chunk) in chunks.iter().enumerate() {
        if !stored_embeddings.contains_key(&chunk.chunk_hash) {
            new_chunk_indices.push(index);
            new_chunk_texts.push(chunk.text.as_str());
        }
    }

    let new_vectors = if new_chunk_texts.is_empty() {
        Vec::new()
    } else {
        embedder.embed_batch(&new_chunk_texts).map_err(|error| {
            LanceDbStoreError::InvalidRequest { message: format!("Embedding failed: {error}") }
        })?
    };

    let mut new_vector_map: std::collections::HashMap<usize, EmbeddingVector> =
        new_chunk_indices.into_iter().zip(new_vectors).collect();

    chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            let embedding = stored_embeddings
                .get(&chunk.chunk_hash)
                .cloned()
                .or_else(|| new_vector_map.remove(&index))
                .ok_or_else(|| LanceDbStoreError::InvalidRequest {
                    message: format!(
                        "No embedding available for chunk '{}' at index {index}",
                        chunk.chunk_id
                    ),
                })?;
            Ok(EmbeddedMarkdownChunkRecord {
                chunk: chunk.clone(),
                embedding_model: embedder.model_id().to_owned(),
                embedding_dimension: embedder.dimension(),
                embedding,
            })
        })
        .collect()
}

fn root_dir_from_store_dir(store_dir: &std::path::Path) -> PathBuf {
    // store_dir is root/.vector-database/rag/lancedb — go up 3 levels
    store_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map_or_else(|| store_dir.to_path_buf(), PathBuf::from)
}

declare_plugin_operations! {
    /// Operation for the Phase 7 incremental indexing pass.
    RagIndexerOp => rag_indexer(RagIndexerInput, IndexResult)
}

impl IndexResult {
    /// Return `true` when the run recorded at least one per-document failure.
    #[must_use]
    pub const fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }
}

impl RagIndexerInput {
    /// Construct a `RagIndexerInput` with explicit fields.
    #[must_use]
    pub const fn new(root_dir: PathBuf, config: RagDefaults) -> Self {
        Self { root_dir, config }
    }
}

impl RagIndexerOp {
    /// Construct a new `RagIndexerOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for RagIndexerOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "rag_indexer_test.rs"]
mod tests;
