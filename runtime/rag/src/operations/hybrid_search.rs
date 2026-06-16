//! Plugin operation boundary for Phase 8 hybrid retrieval.

use crate::RagDefaults;
use runtime_core::{RuntimeError, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use std::path::PathBuf;

/// Input for the `hybrid_search` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct HybridSearchInput {
    /// Workspace root used to resolve the governed RAG store.
    pub root_dir: PathBuf,
    /// Governed RAG defaults that own retrieval settings.
    pub config: RagDefaults,
    /// User query text to execute against the retrieval store.
    pub query_text: String,
    /// Optional package filter applied before ranking and fusion.
    pub package_filter: Option<String>,
    /// Optional governed document stem filter applied before ranking and fusion.
    pub document_filter: Option<String>,
    /// Optional final result count override.
    pub result_limit: Option<usize>,
}

/// One machine-readable retrieval result.
///
/// These fields define the stable Phase 8 result contract even before the
/// semantic, lexical, fusion, and expansion stages are implemented.
///
/// # DTO(machine-readable retrieval payload consumed by CLI and future MCP callers)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct HybridSearchResult {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Heading path for the winning section identity.
    pub heading_path: Vec<String>,
    /// Stable chunk identifier for debugging and traceability.
    pub chunk_id: String,
    /// Retrieved chunk text.
    pub text: String,
    /// Semantic rank position when the chunk appears in the vector branch.
    pub semantic_rank: Option<usize>,
    /// Lexical rank position when the chunk appears in the full-text branch.
    pub lexical_rank: Option<usize>,
    /// Reciprocal Rank Fusion score after branch merging.
    pub rrf_score: f32,
    /// Whether the row was added by adjacent chunk expansion.
    pub was_expanded: bool,
}

/// Output for the `hybrid_search` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct HybridSearchOutput {
    /// Normalized query text used for retrieval.
    pub query_text: String,
    /// Optional package filter after normalization.
    pub package_filter: Option<String>,
    /// Optional governed document stem filter after normalization.
    pub document_filter: Option<String>,
    /// Final result limit after governed defaults are resolved.
    pub result_limit: usize,
    /// Machine-readable retrieval results.
    pub results: Vec<HybridSearchResult>,
}

async fn hybrid_search(
    input: HybridSearchInput,
    output: &mut impl PluginSender<HybridSearchOutput>,
) -> RuntimeResult<()> {
    let HybridSearchInput {
        root_dir: _,
        config,
        query_text,
        package_filter,
        document_filter,
        result_limit,
    } = input;
    let query_text = normalize_required(&query_text, "query_text")?;
    let package_filter = normalize_optional(package_filter.as_deref(), "package_filter")?;
    let document_filter = normalize_optional(document_filter.as_deref(), "document_filter")?;
    let result_limit = result_limit.unwrap_or_else(|| config.final_retrieval_limit());
    if result_limit == 0 {
        return Err(RuntimeError::operation("result_limit must be greater than zero".to_owned()));
    }

    output
        .send(HybridSearchOutput {
            query_text,
            package_filter,
            document_filter,
            result_limit,
            results: Vec::new(),
        })
        .await
}

fn normalize_required(value: &str, field_name: &str) -> RuntimeResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RuntimeError::operation(format!("{field_name} must not be empty")));
    }
    Ok(trimmed.to_owned())
}

fn normalize_optional(value: Option<&str>, field_name: &str) -> RuntimeResult<Option<String>> {
    value.map(|text| normalize_required(text, field_name)).transpose()
}

declare_plugin_operations! {
    /// Operation boundary for Phase 8 hybrid retrieval.
    HybridSearchOp => hybrid_search(HybridSearchInput, HybridSearchOutput)
}

impl HybridSearchInput {
    /// Construct a `HybridSearchInput` with explicit fields.
    #[must_use]
    pub const fn new(
        root_dir: PathBuf,
        config: RagDefaults,
        query_text: String,
        package_filter: Option<String>,
        document_filter: Option<String>,
        result_limit: Option<usize>,
    ) -> Self {
        Self { root_dir, config, query_text, package_filter, document_filter, result_limit }
    }
}

impl HybridSearchOp {
    /// Construct a new `HybridSearchOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for HybridSearchOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "hybrid_search_test.rs"]
mod tests;
