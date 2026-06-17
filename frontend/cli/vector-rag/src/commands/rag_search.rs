//! Command implementation for RAG retrieval search.

#![allow(clippy::print_stdout)]

use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_rag::{
    AssembleRetrievalContextOp, HybridSearchInput, HybridSearchOp, HybridSearchOutput, RagDefaults,
    RetrievalContext, RetrievalContextChunk, RetrievalContextDiagnostics, RetrievalContextSource,
    RetrievalContextStatus, RetrievalMatchReason,
};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Parsed arguments for `vector-rag rag search`.
#[non_exhaustive]
pub struct RagSearchArgs {
    /// Required free-text query sent to the hybrid retrieval operation.
    query_text: String,
    /// Optional package filter forwarded to the runtime operation.
    package_filter: Option<String>,
    /// Optional governed document stem filter forwarded to the runtime operation.
    document_filter: Option<String>,
    /// Optional final retrieval limit override.
    result_limit: Option<usize>,
    /// Whether to emit stable machine-readable JSON instead of human output.
    json_output: bool,
}

impl RagSearchArgs {
    /// Return the search query text.
    #[must_use]
    pub fn query_text(&self) -> &str {
        &self.query_text
    }

    /// Return the optional package filter.
    #[must_use]
    pub fn package_filter(&self) -> Option<&str> {
        self.package_filter.as_deref()
    }

    /// Return the optional document stem filter.
    #[must_use]
    pub fn document_filter(&self) -> Option<&str> {
        self.document_filter.as_deref()
    }

    /// Return the optional result limit.
    #[must_use]
    pub const fn result_limit(&self) -> Option<usize> {
        self.result_limit
    }

    /// Return whether JSON output was requested.
    #[must_use]
    pub const fn json_output(&self) -> bool {
        self.json_output
    }
}

/// Run the retrieval command through the `runtime-rag` operations.
///
/// # Errors
///
/// Returns an actionable error when argument parsing fails, the dispatcher fails,
/// the retrieval operation fails, or no output is produced.
pub async fn run(root_dir: &std::path::Path, args: RagSearchArgs) -> Result<(), String> {
    let input = HybridSearchInput::new(
        root_dir.to_path_buf(),
        RagDefaults::phase_one(),
        args.query_text,
        args.package_filter,
        args.document_filter,
        args.result_limit,
    );

    let (_cancel, mut receiver) = PluginDispatcher::new(HybridSearchOp::new())
        .input(input)
        .build()
        .map_err(|error| format!("failed to prepare hybrid search operation: {error}"))?;

    let output = receiver
        .recv()
        .await
        .map_err(|error| format!("hybrid search failed: {error}"))?
        .ok_or_else(|| "hybrid search did not produce output".to_owned())?;

    let context = assemble_retrieval_context(output).await?;
    let rendered = if args.json_output {
        render_json_output(&context)?
    } else {
        render_human_output(&context)
    };
    println!("{rendered}");
    Ok(())
}

async fn assemble_retrieval_context(
    output: HybridSearchOutput,
) -> Result<RetrievalContext, String> {
    let (_cancel, mut receiver) = PluginDispatcher::new(AssembleRetrievalContextOp::new())
        .input(output)
        .build()
        .map_err(|error| format!("failed to prepare retrieval context operation: {error}"))?;

    receiver
        .recv()
        .await
        .map_err(|error| format!("retrieval context assembly failed: {error}"))?
        .ok_or_else(|| "retrieval context assembly did not produce output".to_owned())
}

/// Parse CLI arguments for `vector-rag rag search`.
///
/// # Errors
///
/// Returns an error when the query is missing, a flag is unknown, a flag value
/// is missing, or `--limit` is invalid.
pub fn parse_args(args: &[String]) -> Result<RagSearchArgs, String> {
    let mut query_parts = Vec::new();
    let mut package_filter = None;
    let mut document_filter = None;
    let mut result_limit = None;
    let mut json_output = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--package" => {
                index += 1;
                package_filter =
                    Some(args.get(index).cloned().ok_or_else(|| missing_flag_value("--package"))?);
            }
            "--document" => {
                index += 1;
                document_filter =
                    Some(args.get(index).cloned().ok_or_else(|| missing_flag_value("--document"))?);
            }
            "--limit" => {
                index += 1;
                let raw = args.get(index).ok_or_else(|| missing_flag_value("--limit"))?;
                result_limit = Some(parse_limit(raw)?);
            }
            "--json" => {
                json_output = true;
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown rag search flag '{flag}'"));
            }
            value => {
                query_parts.push(value.to_owned());
            }
        }
        index += 1;
    }

    let query_text = query_parts.join(" ");
    if query_text.trim().is_empty() {
        return Err("missing search query".to_owned());
    }

    Ok(RagSearchArgs { query_text, package_filter, document_filter, result_limit, json_output })
}

/// Render the default human-readable search output.
#[must_use]
pub fn render_human_output(context: &RetrievalContext) -> String {
    let mut lines = vec![format!(
        "Retrieval Context\nstatus={} query='{}' limit={} returned={}",
        status_label(context.status),
        context.query,
        context.limit,
        context.returned
    )];

    lines.push("Sources:".to_owned());
    if context.sources.is_empty() {
        lines.push("- none".to_owned());
    } else {
        lines.extend(context.sources.iter().map(format_human_source));
    }

    lines.push("Chunks:".to_owned());
    if context.chunks.is_empty() {
        lines.push("- none".to_owned());
    } else {
        lines.extend(context.chunks.iter().map(format_human_chunk));
    }

    lines.push(format_human_diagnostics(context.diagnostics));
    lines.join("\n\n")
}

/// Render the stable machine-readable search payload.
///
/// # Errors
///
/// Returns an error when the retrieval payload cannot be serialized to JSON.
pub fn render_json_output(context: &RetrievalContext) -> Result<String, String> {
    serde_json::to_string_pretty(context)
        .map_err(|error| format!("failed to serialize retrieval context: {error}"))
}

fn format_human_source(source: &RetrievalContextSource) -> String {
    let package = source.package.as_deref().unwrap_or("<workspace>");
    let heading_path = if source.heading_path.is_empty() {
        "<root>".to_owned()
    } else {
        source.heading_path.join(" > ")
    };
    format!(
        "- {} [{}] {} :: {} ({})",
        source.source_id, package, source.document_stem, heading_path, source.citation_label
    )
}

fn format_human_chunk(chunk: &RetrievalContextChunk) -> String {
    let package = chunk.package.as_deref().unwrap_or("<workspace>");
    let heading_path = if chunk.heading_path.is_empty() {
        "<root>".to_owned()
    } else {
        chunk.heading_path.join(" > ")
    };
    let mut lines = vec![
        format!("- {} [{}] {} :: {}", chunk.context_id, package, chunk.document_stem, heading_path),
        format!(
            "  source={} chunk={} ordinal={} tokens={} match_reason={}",
            chunk.source_id,
            chunk.chunk_id,
            chunk.chunk_ordinal,
            chunk.token_count,
            match_reason_label(chunk.match_reason)
        ),
    ];
    lines.push(format!("  {}", chunk.text));
    lines.join("\n")
}

fn format_human_diagnostics(diagnostics: RetrievalContextDiagnostics) -> String {
    format!(
        "Diagnostics:\ntotal_token_count={} dropped_after_limit={} retrieval_limit={}",
        diagnostics.total_token_count, diagnostics.dropped_after_limit, diagnostics.retrieval_limit
    )
}

const fn status_label(status: RetrievalContextStatus) -> &'static str {
    match status {
        RetrievalContextStatus::HasResults => "has_results",
        RetrievalContextStatus::Empty => "empty",
        _ => "unknown",
    }
}

const fn match_reason_label(match_reason: RetrievalMatchReason) -> &'static str {
    match match_reason {
        RetrievalMatchReason::Primary => "primary",
        RetrievalMatchReason::Expanded => "expanded",
        _ => "unknown",
    }
}

fn parse_limit(raw: &str) -> Result<usize, String> {
    let limit =
        raw.parse::<usize>().map_err(|error| format!("invalid --limit value '{raw}': {error}"))?;
    if limit == 0 {
        return Err("--limit must be greater than zero".to_owned());
    }
    Ok(limit)
}

fn missing_flag_value(flag: &str) -> String {
    format!("missing value for {flag}")
}

#[cfg(test)]
#[path = "rag_search_test.rs"]
mod tests;
