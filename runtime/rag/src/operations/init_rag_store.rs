//! Plugin operation for creating or validating the Phase 6 `LanceDB` store.

use crate::{LanceDbStoreRequest, ensure_lancedb_store};
use runtime_core::{RuntimeError, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use std::path::PathBuf;

/// Input for the `init_rag_store` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct InitRagStoreInput {
    /// Workspace root used to resolve the Phase 6 `LanceDB` storage path.
    pub root_dir: PathBuf,
    /// Embedding model identifier that owns the active store contract.
    pub embedding_model: String,
    /// Embedding dimension that owns the active store contract.
    pub embedding_dimension: usize,
}

/// Output for the `init_rag_store` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct InitRagStoreOutput {
    /// Resolved local database directory under `.vector-database/rag/lancedb/`.
    pub database_dir: PathBuf,
    /// Primary Phase 6 chunk table name.
    pub table_name: String,
    /// Whether the primary table was created during this call.
    pub created_table: bool,
    /// Whether the full-text index was created during this call.
    pub created_text_index: bool,
}

async fn init_rag_store(
    input: InitRagStoreInput,
    output: &mut impl PluginSender<InitRagStoreOutput>,
) -> RuntimeResult<()> {
    let request = LanceDbStoreRequest {
        root_dir: input.root_dir,
        embedding_model: input.embedding_model,
        embedding_dimension: input.embedding_dimension,
    };
    let status = ensure_lancedb_store(&request)
        .await
        .map_err(|error| RuntimeError::operation(error.to_string()))?;
    output
        .send(InitRagStoreOutput {
            database_dir: status.database_dir,
            table_name: status.table_name,
            created_table: status.created_table,
            created_text_index: status.created_text_index,
        })
        .await?;
    Ok(())
}

declare_plugin_operations! {
    /// Operation for creating or validating the Phase 6 LanceDB RAG store.
    InitRagStoreOp => init_rag_store(InitRagStoreInput, InitRagStoreOutput)
}

impl InitRagStoreInput {
    /// Construct an `InitRagStoreInput` with explicit fields.
    #[must_use]
    pub const fn new(
        root_dir: PathBuf,
        embedding_model: String,
        embedding_dimension: usize,
    ) -> Self {
        Self { root_dir, embedding_model, embedding_dimension }
    }
}

impl InitRagStoreOp {
    /// Construct a new `InitRagStoreOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for InitRagStoreOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "init_rag_store_test.rs"]
mod tests;
