//! Canonical Phase 9 retrieval context contract.

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
#[path = "retrieval_context_test.rs"]
mod tests;
