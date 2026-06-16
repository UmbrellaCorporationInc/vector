//! Plugin operation that orchestrates Phase 6 store initialization with Phase 7 incremental indexing.

use crate::{
    RagDefaults,
    operations::{
        IndexResult, RagIndexerInput, RagIndexerOp,
        init_rag_store::{InitRagStoreInput, InitRagStoreOp},
        support::CapturingSender,
    },
};
use runtime_core::{
    FlowOperation, RuntimeError, RuntimeResult, declare_plugin_operations, plugin::PluginSender,
};
use std::path::PathBuf;

/// Input for the `index_workspace` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct IndexWorkspaceInput {
    /// Workspace root used to resolve corpus paths and the `LanceDB` storage path.
    pub root_dir: PathBuf,
    /// Phase 1 RAG defaults governing the embedding model and corpus paths.
    pub config: RagDefaults,
}

/// Output for the `index_workspace` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexWorkspaceOutput {
    /// Indexing run summary produced by `RagIndexerOp`.
    pub result: IndexResult,
}

async fn index_workspace(
    input: IndexWorkspaceInput,
    output: &mut impl PluginSender<IndexWorkspaceOutput>,
) -> RuntimeResult<()> {
    let init_input = InitRagStoreInput::new(
        input.root_dir.clone(),
        input.config.embedding_model_identifier().to_owned(),
        input.config.embedding_dimension(),
    );
    let mut init_sender = CapturingSender::new();
    InitRagStoreOp::new().run(init_input, &mut init_sender).await.map_err(|error| {
        RuntimeError::operation(format!("RAG store initialization failed: {error}"))
    })?;
    init_sender.into_output().ok_or_else(|| {
        RuntimeError::operation("RAG store initialization produced no output".to_owned())
    })?;

    let indexer_input = RagIndexerInput::new(input.root_dir, input.config);
    let mut indexer_sender = CapturingSender::<IndexResult>::new();
    RagIndexerOp::new().run(indexer_input, &mut indexer_sender).await.map_err(|error| {
        RuntimeError::operation(format!("RAG incremental indexing failed: {error}"))
    })?;
    let result = indexer_sender.into_output().ok_or_else(|| {
        RuntimeError::operation("RAG incremental indexing produced no output".to_owned())
    })?;

    output.send(IndexWorkspaceOutput { result }).await
}

declare_plugin_operations! {
    /// Orchestrating operation composing Phase 6 store initialization and Phase 7 incremental indexing.
    IndexWorkspaceOp => index_workspace(IndexWorkspaceInput, IndexWorkspaceOutput)
}

impl IndexWorkspaceInput {
    /// Construct an `IndexWorkspaceInput` with explicit fields.
    #[must_use]
    pub const fn new(root_dir: PathBuf, config: RagDefaults) -> Self {
        Self { root_dir, config }
    }
}

impl IndexWorkspaceOp {
    /// Construct a new `IndexWorkspaceOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for IndexWorkspaceOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "index_workspace_test.rs"]
mod tests;
