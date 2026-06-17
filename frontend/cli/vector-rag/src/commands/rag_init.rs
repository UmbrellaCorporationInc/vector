//! Command implementation to create or validate the local RAG `LanceDB` store.

#![allow(clippy::print_stdout)]

use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_rag::{InitRagStoreInput, InitRagStoreOp, RagDefaults};

/// Create or validate the local RAG store.
///
/// # Errors
///
/// Returns an actionable error when the dispatcher fails, the operation emits
/// no output, or the store contract is invalid or incompatible.
pub async fn run(root_dir: &std::path::Path) -> Result<(), String> {
    let defaults = RagDefaults::phase_one();
    let input = InitRagStoreInput::new(
        root_dir.to_path_buf(),
        defaults.embedding_model_identifier().to_owned(),
        defaults.embedding_dimension(),
    );

    let (_cancel, mut receiver) = PluginDispatcher::new(InitRagStoreOp::new())
        .input(input)
        .build()
        .map_err(|error| format!("failed to prepare RAG store operation: {error}"))?;

    let output = receiver
        .recv()
        .await
        .map_err(|error| format!("RAG store initialization failed: {error}"))?
        .ok_or_else(|| "RAG store initialization did not produce output".to_owned())?;

    let action = if output.created_table || output.created_text_index {
        "created or updated"
    } else {
        "validated"
    };

    println!(
        "RAG store {action} at '{}' using table '{}'.",
        output.database_dir.display(),
        output.table_name
    );

    Ok(())
}

#[cfg(test)]
#[path = "rag_init_test.rs"]
mod tests;
