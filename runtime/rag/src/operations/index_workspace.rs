//! Plugin operation that orchestrates Phase 6 store initialization with Phase 7 incremental indexing.

use crate::{
    RagDefaults,
    operations::{
        IndexResult, RagIndexerInput,
        init_rag_store::{InitRagStoreInput, InitRagStoreOp},
        rag_indexer::{
            IndexingProgressSink, IndexingProgressUpdate, LazyFastembedEmbedder,
            run_incremental_indexing_pass,
        },
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
pub enum IndexWorkspaceOutput {
    /// Incremental progress emitted while the indexing operation is still running.
    Progress(IndexWorkspaceProgress),
    /// Final indexing run summary emitted after progress is complete.
    Summary(IndexResult),
}

/// Incremental progress event emitted by `IndexWorkspaceOp`.
///
/// # DTO(progress boundary consumed by the CLI and MCP passthrough layers)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexWorkspaceProgress {
    /// Stable progress label for scanning or parsing command output.
    pub label: String,
    /// Package identity when the event applies to one synchronized package document.
    pub package: Option<String>,
    /// Governed document stem when the event applies to one document.
    pub document_stem: Option<String>,
    /// Human-readable detail for non-document lifecycle steps or actionable failures.
    pub message: Option<String>,
}

async fn index_workspace(
    input: IndexWorkspaceInput,
    output: &mut impl PluginSender<IndexWorkspaceOutput>,
) -> RuntimeResult<()> {
    output
        .send(IndexWorkspaceOutput::Progress(IndexWorkspaceProgress::message(
            "initializing-store",
            "Preparing the local RAG store.",
        )))
        .await?;

    let init_input = InitRagStoreInput::new(
        input.root_dir.clone(),
        input.config.embedding_model_identifier().to_owned(),
        input.config.embedding_dimension(),
    );
    let mut init_sender = CapturingSender::new();
    InitRagStoreOp::new().run(init_input, &mut init_sender).await.map_err(|error| {
        RuntimeError::operation(format!("RAG store initialization failed: {error}"))
    })?;
    let init_output = init_sender.into_output().ok_or_else(|| {
        RuntimeError::operation("RAG store initialization produced no output".to_owned())
    })?;
    let init_message = if init_output.created_table || init_output.created_text_index {
        format!(
            "Store ready at '{}' (created_table={}, created_text_index={}).",
            init_output.database_dir.display(),
            init_output.created_table,
            init_output.created_text_index
        )
    } else {
        format!("Store ready at '{}'.", init_output.database_dir.display())
    };
    output
        .send(IndexWorkspaceOutput::Progress(IndexWorkspaceProgress::message(
            "store-ready",
            init_message,
        )))
        .await?;

    let indexer_input = RagIndexerInput::new(input.root_dir, input.config);
    let embedder = LazyFastembedEmbedder::new();
    let mut progress_sink = WorkspaceProgressSink { output };
    let result = run_incremental_indexing_pass(&indexer_input, &embedder, &mut progress_sink)
        .await
        .map_err(|error| {
            RuntimeError::operation(format!("RAG incremental indexing failed: {error}"))
        })?;

    output.send(IndexWorkspaceOutput::Summary(result)).await
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

impl IndexWorkspaceProgress {
    /// Construct a non-document progress event with a stable label and message.
    #[must_use]
    pub fn message(label: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            package: None,
            document_stem: None,
            message: Some(message.into()),
        }
    }
}

impl From<IndexingProgressUpdate> for IndexWorkspaceProgress {
    fn from(value: IndexingProgressUpdate) -> Self {
        match value {
            IndexingProgressUpdate::DiscoveringDocuments => {
                Self::message("discovering-documents", "Discovering governed Markdown documents.")
            }
            IndexingProgressUpdate::IndexingDocuments { discovered_documents } => Self::message(
                "indexing-documents",
                format!("Indexing {discovered_documents} discovered document(s)."),
            ),
            IndexingProgressUpdate::Indexed { package, document_stem } => Self {
                label: "indexed".to_owned(),
                package,
                document_stem: Some(document_stem),
                message: None,
            },
            IndexingProgressUpdate::Unchanged { package, document_stem } => Self {
                label: "unchanged".to_owned(),
                package,
                document_stem: Some(document_stem),
                message: None,
            },
            IndexingProgressUpdate::Deleted { package, document_stem } => Self {
                label: "deleted".to_owned(),
                package,
                document_stem: Some(document_stem),
                message: Some("Removed stale indexed chunks.".to_owned()),
            },
            IndexingProgressUpdate::Failed { package, document_stem, error } => Self {
                label: "failed".to_owned(),
                package,
                document_stem: Some(document_stem),
                message: Some(error),
            },
        }
    }
}

impl Default for IndexWorkspaceOp {
    fn default() -> Self {
        Self::new()
    }
}

struct WorkspaceProgressSink<'a, Output> {
    output: &'a mut Output,
}

impl<Output> IndexingProgressSink for WorkspaceProgressSink<'_, Output>
where
    Output: PluginSender<IndexWorkspaceOutput>,
{
    fn emit(
        &mut self,
        progress: IndexingProgressUpdate,
    ) -> impl std::future::Future<Output = ()> + Send {
        let output = &mut *self.output;
        let progress_output =
            IndexWorkspaceOutput::Progress(IndexWorkspaceProgress::from(progress));
        async move {
            let _ = output.send(progress_output).await;
        }
    }
}

#[cfg(test)]
#[path = "index_workspace_test.rs"]
mod tests;
