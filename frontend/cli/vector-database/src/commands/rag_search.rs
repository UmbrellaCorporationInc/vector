//! Command implementation for Phase 8 hybrid retrieval search.

#![allow(clippy::print_stdout)]

use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_rag::{
    HybridSearchInput, HybridSearchOp, HybridSearchOutput, HybridSearchResult, RagDefaults,
};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Parsed arguments for `vector-database rag search`.
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

/// Run the Phase 8 hybrid retrieval command through the runtime-rag operation.
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

    let rendered =
        if args.json_output { render_json_output(&output)? } else { render_human_output(&output) };
    println!("{rendered}");
    Ok(())
}

/// Parse CLI arguments for `vector-database rag search`.
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
pub fn render_human_output(output: &HybridSearchOutput) -> String {
    if output.results.is_empty() {
        return format!(
            "No retrieval results for '{}' (limit {}).",
            output.query_text, output.result_limit
        );
    }

    let mut lines = vec![format!(
        "Retrieved {} result(s) for '{}' (limit {}).",
        output.results.len(),
        output.query_text,
        output.result_limit
    )];
    for (index, result) in output.results.iter().enumerate() {
        lines.push(format_human_result(index + 1, result));
    }
    lines.join("\n\n")
}

/// Render the stable machine-readable search payload.
///
/// # Errors
///
/// Returns an error when the retrieval payload cannot be serialized to JSON.
pub fn render_json_output(output: &HybridSearchOutput) -> Result<String, String> {
    let payload = json!({
        "query_text": output.query_text,
        "package_filter": output.package_filter,
        "document_filter": output.document_filter,
        "result_limit": output.result_limit,
        "results": output.results.iter().map(result_json).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("failed to serialize hybrid search output: {error}"))
}

fn format_human_result(index: usize, result: &HybridSearchResult) -> String {
    let package = result.package.as_deref().unwrap_or("<workspace>");
    let heading_path = if result.heading_path.is_empty() {
        "<root>".to_owned()
    } else {
        result.heading_path.join(" > ")
    };
    let mut lines = vec![
        format!("{index}. [{}] {} :: {}", package, result.document_stem, heading_path),
        format!(
            "chunk={} ordinal={} score={:.6} semantic_rank={} lexical_rank={} expanded={}",
            result.chunk_id,
            result.chunk_ordinal,
            result.rrf_score,
            optional_rank(result.semantic_rank),
            optional_rank(result.lexical_rank),
            result.was_expanded
        ),
    ];
    if let Some(expanded_from) = &result.expanded_from_chunk_id {
        lines.push(format!("expanded_from={expanded_from}"));
    }
    lines.push(result.text.clone());
    lines.join("\n")
}

fn result_json(result: &HybridSearchResult) -> Value {
    json!({
        "package": result.package,
        "document_stem": result.document_stem,
        "heading_path": result.heading_path,
        "chunk_id": result.chunk_id,
        "chunk_ordinal": result.chunk_ordinal,
        "text": result.text,
        "token_count": result.token_count,
        "semantic_rank": result.semantic_rank,
        "lexical_rank": result.lexical_rank,
        "rrf_score": result.rrf_score,
        "previous_chunk_id": result.previous_chunk_id,
        "next_chunk_id": result.next_chunk_id,
        "was_expanded": result.was_expanded,
        "expanded_from_chunk_id": result.expanded_from_chunk_id,
    })
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

fn optional_rank(rank: Option<usize>) -> String {
    rank.map_or_else(|| "-".to_owned(), |value| value.to_string())
}

#[cfg(test)]
#[path = "rag_search_test.rs"]
mod tests;
