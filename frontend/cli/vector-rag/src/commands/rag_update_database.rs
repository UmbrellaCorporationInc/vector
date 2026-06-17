//! Command implementation for the incremental RAG indexing pipeline.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_rag::{
    IndexResult, IndexWorkspaceInput, IndexWorkspaceOp, IndexWorkspaceOutput,
    IndexWorkspaceProgress, RagDefaults,
};
use serde::Serialize;
use std::io::Write;

/// Parsed CLI options for `vector-rag rag update-database`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RagUpdateDatabaseArgs {
    json_output: bool,
}

impl RagUpdateDatabaseArgs {
    /// Construct update-database CLI options.
    #[must_use]
    pub const fn new(json_output: bool) -> Self {
        Self { json_output }
    }

    /// Return `true` when the command should emit the final JSON contract.
    #[must_use]
    pub const fn json_output(self) -> bool {
        self.json_output
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RagUpdateDatabaseJsonOutput {
    progress: Vec<IndexWorkspaceProgress>,
    summary: IndexResult,
}

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
    run_with_args(root_dir, RagUpdateDatabaseArgs::new(false)).await
}

/// Run the incremental indexing pipeline against the local workspace with
/// output-format options.
///
/// # Errors
///
/// Returns an actionable error when the dispatcher fails, the operation emits
/// no output, or one or more documents fail during indexing.
pub async fn run_with_args(
    root_dir: &std::path::Path,
    args: RagUpdateDatabaseArgs,
) -> Result<(), String> {
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_with_writers(root_dir, args, &mut stdout, &mut stderr).await
}

async fn run_with_writers(
    root_dir: &std::path::Path,
    args: RagUpdateDatabaseArgs,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), String> {
    let config = RagDefaults::phase_one();
    let input = IndexWorkspaceInput::new(root_dir.to_path_buf(), config);

    let (_cancel, mut receiver) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(input)
        .build()
        .map_err(|error| format!("failed to prepare indexing operation: {error}"))?;

    let mut final_result = None;
    let mut progress_events = Vec::new();
    while let Some(output) =
        receiver.recv().await.map_err(|error| format!("incremental indexing failed: {error}"))?
    {
        match output {
            IndexWorkspaceOutput::Progress(progress) => {
                if !args.json_output() {
                    write_progress_line(stdout, &progress)?;
                }
                progress_events.push(progress);
            }
            IndexWorkspaceOutput::Summary(result) => {
                if !args.json_output() {
                    write_summary_line(stdout, &result)?;
                }
                final_result = Some(result);
            }
            _ => {}
        }
    }

    let result =
        final_result.ok_or_else(|| "incremental indexing did not produce output".to_owned())?;

    if args.json_output() {
        write_json_output(stdout, &progress_events, &result)?;
    }

    if result.has_failures() {
        if !args.json_output() {
            write_failure_details(stderr, &result)?;
        }
        return Err(format!("{} document(s) failed during indexing", result.failures.len()));
    }

    Ok(())
}

fn write_progress_line(
    writer: &mut impl Write,
    progress: &IndexWorkspaceProgress,
) -> Result<(), String> {
    let line = format_progress_line(progress);
    writer
        .write_all(line.as_bytes())
        .map_err(|error| format!("failed to write progress output: {error}"))?;
    writer.write_all(b"\n").map_err(|error| format!("failed to write progress output: {error}"))?;
    writer.flush().map_err(|error| format!("failed to flush progress output: {error}"))?;
    Ok(())
}

fn write_summary_line(writer: &mut impl Write, result: &IndexResult) -> Result<(), String> {
    let line = format!(
        "Indexed: {} re-indexed, {} skipped, {} deleted.",
        result.reindexed_count, result.skipped_count, result.deleted_count
    );
    writer
        .write_all(line.as_bytes())
        .map_err(|error| format!("failed to write summary output: {error}"))?;
    writer.write_all(b"\n").map_err(|error| format!("failed to write summary output: {error}"))?;
    writer.flush().map_err(|error| format!("failed to flush summary output: {error}"))?;
    Ok(())
}

fn write_failure_details(writer: &mut impl Write, result: &IndexResult) -> Result<(), String> {
    for failure in &result.failures {
        let pkg = failure.package.as_deref().unwrap_or("<workspace>");
        let line = format!("  failed [{pkg}] {}: {}", failure.document_stem, failure.error);
        writer
            .write_all(line.as_bytes())
            .map_err(|error| format!("failed to write failure output: {error}"))?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("failed to write failure output: {error}"))?;
    }
    writer.flush().map_err(|error| format!("failed to flush failure output: {error}"))?;
    Ok(())
}

fn write_json_output(
    writer: &mut impl Write,
    progress: &[IndexWorkspaceProgress],
    result: &IndexResult,
) -> Result<(), String> {
    let payload =
        RagUpdateDatabaseJsonOutput { progress: progress.to_vec(), summary: result.clone() };
    serde_json::to_writer_pretty(&mut *writer, &payload)
        .map_err(|error| format!("failed to serialize JSON output: {error}"))?;
    writer.write_all(b"\n").map_err(|error| format!("failed to write JSON output: {error}"))?;
    writer.flush().map_err(|error| format!("failed to flush JSON output: {error}"))?;
    Ok(())
}

fn format_progress_line(progress: &IndexWorkspaceProgress) -> String {
    let mut line = progress.label.clone();
    if let Some(package) = progress.package.as_deref() {
        line.push_str(" package=");
        line.push_str(package);
    }
    if let Some(document_stem) = progress.document_stem.as_deref() {
        line.push_str(" document=");
        line.push_str(document_stem);
    }
    if let Some(message) = progress.message.as_deref() {
        line.push_str(" message=");
        line.push_str(message);
    }
    line
}

#[cfg(test)]
#[path = "rag_update_database_test.rs"]
mod tests;
