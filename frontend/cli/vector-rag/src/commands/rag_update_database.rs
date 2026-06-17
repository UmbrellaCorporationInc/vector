//! Command implementation for the incremental RAG indexing pipeline.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_rag::{IndexWorkspaceInput, IndexWorkspaceOp, RagDefaults};

/// Run the incremental indexing pipeline against the local workspace.
///
/// Initializes the `LanceDB` store if needed, then indexes governed Markdown
/// documents and skips files whose content hash is unchanged.
///
/// # Errors
///
/// Returns an actionable error when the dispatcher fails, the operation emits
/// no output, or one or more documents fail during indexing.
pub async fn run(root_dir: &std::path::Path) -> Result<(), String> {
    let config = RagDefaults::phase_one();
    let input = IndexWorkspaceInput::new(root_dir.to_path_buf(), config);

    let (_cancel, mut receiver) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(input)
        .build()
        .map_err(|error| format!("failed to prepare indexing operation: {error}"))?;

    let output = receiver
        .recv()
        .await
        .map_err(|error| format!("incremental indexing failed: {error}"))?
        .ok_or_else(|| "incremental indexing did not produce output".to_owned())?;

    let result = &output.result;
    println!(
        "Indexed: {} re-indexed, {} skipped, {} deleted.",
        result.reindexed_count, result.skipped_count, result.deleted_count
    );

    if result.has_failures() {
        for failure in &result.failures {
            let pkg = failure.package.as_deref().unwrap_or("<workspace>");
            eprintln!("  failed [{}] {}: {}", pkg, failure.document_stem, failure.error);
        }
        return Err(format!("{} document(s) failed during indexing", result.failures.len()));
    }

    Ok(())
}

#[cfg(test)]
#[path = "rag_update_database_test.rs"]
mod tests;
