//! Canonical Phase 9 retrieval context contract.

use crate::{HybridSearchOutput, HybridSearchResult};
use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type SourceKey = (Option<String>, String, Vec<String>);

/// Canonical, model-agnostic retrieved evidence payload.
///
/// # DTO(machine-readable retrieval context consumed by CLI and MCP adapters)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalContext {
    /// Query text used to retrieve the evidence.
    pub query: String,
    /// Successful retrieval status.
    pub status: RetrievalContextStatus,
    /// Final retrieval limit applied to the context result.
    pub limit: usize,
    /// Number of evidence chunks returned.
    pub returned: usize,
    /// Normalized source entries referenced by returned chunks.
    pub sources: Vec<RetrievalContextSource>,
    /// Evidence chunks returned for the query.
    pub chunks: Vec<RetrievalContextChunk>,
    /// Runtime diagnostics for the context result.
    pub diagnostics: RetrievalContextDiagnostics,
}

/// Successful retrieval status for the canonical context result.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalContextStatus {
    /// At least one evidence chunk was returned.
    HasResults,
    /// Retrieval completed successfully with no evidence chunks.
    Empty,
}

/// Normalized source attribution for one or more evidence chunks.
///
/// # DTO(machine-readable retrieval source consumed by CLI and MCP adapters)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalContextSource {
    /// Response-local source identifier referenced by chunks.
    pub source_id: String,
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Heading path for the source section.
    pub heading_path: Vec<String>,
    /// Deterministic human-readable citation label.
    pub citation_label: String,
}

/// One retrieved evidence chunk in the canonical context result.
///
/// # DTO(machine-readable retrieval chunk consumed by CLI and MCP adapters)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalContextChunk {
    /// Response-local context identifier, such as `ctx-1`.
    pub context_id: String,
    /// Response-local source identifier matching an entry in `sources`.
    pub source_id: String,
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Heading path for the chunk section.
    pub heading_path: Vec<String>,
    /// Stable persisted chunk identifier.
    pub chunk_id: String,
    /// Zero-based chunk ordinal within the governed document.
    pub chunk_ordinal: usize,
    /// Retrieved chunk text.
    pub text: String,
    /// Token count emitted by chunking for this stored row.
    pub token_count: usize,
    /// Reason this chunk appears in the context result.
    pub match_reason: RetrievalMatchReason,
}

/// Reason an evidence chunk was selected for the context result.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMatchReason {
    /// The chunk survived Phase 8 ranking and deduplication.
    Primary,
    /// The chunk was added by adjacent chunk expansion.
    Expanded,
}

/// Diagnostics for the canonical context result.
///
/// # DTO(machine-readable retrieval diagnostics consumed by CLI and MCP adapters)
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalContextDiagnostics {
    /// Aggregate token count across returned evidence chunks.
    pub total_token_count: usize,
    /// Number of chunks dropped after final limit enforcement.
    pub dropped_after_limit: usize,
    /// Final retrieval limit applied to the context result.
    pub retrieval_limit: usize,
}

async fn assemble_retrieval_context(
    input: HybridSearchOutput,
    output: &mut impl PluginSender<RetrievalContext>,
) -> RuntimeResult<()> {
    output.send(assemble_retrieval_context_output(input)).await
}

fn assemble_retrieval_context_output(input: HybridSearchOutput) -> RetrievalContext {
    let original_result_count = input.results.len();
    let returned_results = input.results.into_iter().take(input.result_limit).collect::<Vec<_>>();
    let dropped_after_limit = original_result_count.saturating_sub(returned_results.len());

    let mut source_ids_by_key = HashMap::<SourceKey, String>::new();
    let mut sources = Vec::new();
    let mut chunks = Vec::with_capacity(returned_results.len());
    let mut total_token_count = 0_usize;

    for (index, result) in returned_results.into_iter().enumerate() {
        let source_key =
            (result.package.clone(), result.document_stem.clone(), result.heading_path.clone());
        let source_id = source_ids_by_key
            .entry(source_key)
            .or_insert_with(|| {
                let source_id = format!("src-{}", sources.len() + 1);
                sources.push(RetrievalContextSource::new(
                    source_id.clone(),
                    result.package.clone(),
                    result.document_stem.clone(),
                    result.heading_path.clone(),
                    citation_label(
                        result.package.as_deref(),
                        &result.document_stem,
                        &result.heading_path,
                    ),
                ));
                source_id
            })
            .clone();

        total_token_count += result.token_count;
        chunks.push(retrieval_context_chunk(index + 1, source_id, result));
    }

    RetrievalContext::new(
        input.query_text,
        input.result_limit,
        sources,
        chunks,
        RetrievalContextDiagnostics::new(
            total_token_count,
            dropped_after_limit,
            input.result_limit,
        ),
    )
}

fn retrieval_context_chunk(
    response_index: usize,
    source_id: String,
    result: HybridSearchResult,
) -> RetrievalContextChunk {
    let match_reason = if result.was_expanded {
        RetrievalMatchReason::Expanded
    } else {
        RetrievalMatchReason::Primary
    };

    RetrievalContextChunk::new(
        format!("ctx-{response_index}"),
        source_id,
        result.package,
        result.document_stem,
        result.heading_path,
        result.chunk_id,
        result.chunk_ordinal,
        result.text,
        result.token_count,
        match_reason,
    )
}

fn citation_label(package: Option<&str>, document_stem: &str, heading_path: &[String]) -> String {
    let mut label =
        package.map_or_else(|| document_stem.to_owned(), |name| format!("{name}/{document_stem}"));
    for heading in heading_path {
        label.push_str(" > ");
        label.push_str(heading);
    }
    label
}

declare_plugin_operations! {
    /// Operation boundary for Phase 9 canonical retrieval context assembly.
    AssembleRetrievalContextOp => assemble_retrieval_context(HybridSearchOutput, RetrievalContext)
}

impl RetrievalContext {
    /// Construct a canonical retrieval context.
    #[must_use]
    pub const fn new(
        query: String,
        limit: usize,
        sources: Vec<RetrievalContextSource>,
        chunks: Vec<RetrievalContextChunk>,
        diagnostics: RetrievalContextDiagnostics,
    ) -> Self {
        let returned = chunks.len();
        let status = if returned == 0 {
            RetrievalContextStatus::Empty
        } else {
            RetrievalContextStatus::HasResults
        };
        Self { query, status, limit, returned, sources, chunks, diagnostics }
    }

    /// Construct a successful empty retrieval context.
    #[must_use]
    pub const fn empty(query: String, limit: usize) -> Self {
        Self::new(
            query,
            limit,
            Vec::new(),
            Vec::new(),
            RetrievalContextDiagnostics::new(0, 0, limit),
        )
    }
}

impl RetrievalContextSource {
    /// Construct a normalized retrieval context source.
    #[must_use]
    pub const fn new(
        source_id: String,
        package: Option<String>,
        document_stem: String,
        heading_path: Vec<String>,
        citation_label: String,
    ) -> Self {
        Self { source_id, package, document_stem, heading_path, citation_label }
    }
}

impl RetrievalContextChunk {
    /// Construct a retrieval context evidence chunk.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        context_id: String,
        source_id: String,
        package: Option<String>,
        document_stem: String,
        heading_path: Vec<String>,
        chunk_id: String,
        chunk_ordinal: usize,
        text: String,
        token_count: usize,
        match_reason: RetrievalMatchReason,
    ) -> Self {
        Self {
            context_id,
            source_id,
            package,
            document_stem,
            heading_path,
            chunk_id,
            chunk_ordinal,
            text,
            token_count,
            match_reason,
        }
    }
}

impl RetrievalContextDiagnostics {
    /// Construct retrieval context diagnostics.
    #[must_use]
    pub const fn new(
        total_token_count: usize,
        dropped_after_limit: usize,
        retrieval_limit: usize,
    ) -> Self {
        Self { total_token_count, dropped_after_limit, retrieval_limit }
    }
}

impl AssembleRetrievalContextOp {
    /// Construct a new `AssembleRetrievalContextOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for AssembleRetrievalContextOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "retrieval_context_test.rs"]
mod tests;
