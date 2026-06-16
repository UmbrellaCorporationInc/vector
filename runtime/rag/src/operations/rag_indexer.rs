//! Plugin operation for Phase 7 incremental indexing.

use crate::RagDefaults;
use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use std::path::PathBuf;

/// Per-document indexing failure record.
///
/// # DTO(indexing failure boundary consumed by `IndexResult` callers)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    _input: RagIndexerInput,
    output: &mut impl PluginSender<IndexResult>,
) -> RuntimeResult<()> {
    output.send(IndexResult::default()).await
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
