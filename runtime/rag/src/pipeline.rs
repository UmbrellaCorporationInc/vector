//! RAG pipeline orchestration boundaries.

use crate::{
    EmbeddedMarkdownChunkRecord, Embedder, EmbeddingError, MarkdownChunkDocument,
    MarkdownChunkRecord, MarkdownChunkingConfig, MarkdownChunkingError, MarkdownTokenCounter,
    chunk_markdown_document, embed_markdown_chunks,
};
use runtime_markdown::{
    MarkdownExtractionError, MarkdownExtractionErrorRecord, MarkdownExtractionOutcome,
    MarkdownSourceSpan,
};
use std::collections::BTreeMap;

/// Chunks emitted for one extracted Markdown document before embedding.
///
/// # DTO(extraction-to-embedding boundary consumed by later RAG phases)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarkdownChunkBatch {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem.
    pub document_stem: String,
    /// Source document content hash from discovery.
    pub document_hash: String,
    /// Ordered chunks ready for embedding and storage.
    pub chunks: Vec<MarkdownChunkRecord>,
}

/// File-scoped chunking pipeline outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownChunkingPipelineOutcome {
    /// Extraction and chunking succeeded.
    Chunked(MarkdownChunkBatch),
    /// The document could not be chunked, but unrelated documents can continue.
    Failed(MarkdownChunkingFailureRecord),
}

/// File-scoped chunking failure record.
///
/// # DTO(indexing diagnostic boundary consumed by later RAG phases)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarkdownChunkingFailureRecord {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem.
    pub document_stem: String,
    /// Source document content hash from discovery.
    pub document_hash: String,
    /// Actionable failure details.
    pub error: MarkdownChunkingPipelineError,
}

/// Embedded chunks emitted for one extracted Markdown document before storage.
///
/// # DTO(embedding-to-storage boundary consumed by later RAG phases)
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct EmbeddedMarkdownChunkBatch {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem.
    pub document_stem: String,
    /// Source document content hash from discovery.
    pub document_hash: String,
    /// Ordered chunks carrying embedding vectors and model metadata.
    pub chunks: Vec<EmbeddedMarkdownChunkRecord>,
}

/// File-scoped extraction, chunking, and embedding pipeline outcome.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MarkdownEmbeddingPipelineOutcome {
    /// Extraction, chunking, and embedding succeeded.
    Embedded(EmbeddedMarkdownChunkBatch),
    /// The document could not be embedded, but unrelated documents can continue.
    Failed(MarkdownEmbeddingFailureRecord),
}

/// File-scoped embedding pipeline failure record.
///
/// # DTO(indexing diagnostic boundary consumed by later RAG phases)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarkdownEmbeddingFailureRecord {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem.
    pub document_stem: String,
    /// Source document content hash from discovery.
    pub document_hash: String,
    /// Actionable failure details.
    pub error: MarkdownEmbeddingPipelineError,
}

/// Actionable extraction-to-chunking failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownChunkingPipelineError {
    /// Extraction output was malformed or could not be adapted into chunker input.
    MalformedExtractionInput {
        /// Stable error kind.
        kind: String,
        /// Human-readable error message.
        message: String,
        /// Optional source span for the invalid input.
        source_span: Option<MarkdownSourceSpan>,
        /// Structured string details.
        details: BTreeMap<String, String>,
    },

    /// Extraction identified a Markdown structure the current pipeline cannot support.
    UnsupportedMarkdownStructure {
        /// Stable error kind.
        kind: String,
        /// Human-readable error message.
        message: String,
        /// Optional source span for the unsupported structure.
        source_span: Option<MarkdownSourceSpan>,
        /// Structured string details.
        details: BTreeMap<String, String>,
    },

    /// A Markdown block cannot be split safely within the configured token limit.
    UnsplittableOversizedBlock {
        /// Human-readable error message.
        message: String,
        /// Number of tokens in the unsplittable block.
        token_count: usize,
        /// Configured maximum chunk token count.
        maximum_token_count: usize,
    },
}

impl std::fmt::Display for MarkdownChunkingPipelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedExtractionInput { message, .. }
            | Self::UnsupportedMarkdownStructure { message, .. }
            | Self::UnsplittableOversizedBlock { message, .. } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for MarkdownChunkingPipelineError {}

/// Actionable extraction-to-embedding failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownEmbeddingPipelineError {
    /// The Markdown input could not be extracted or chunked.
    Chunking(MarkdownChunkingPipelineError),
    /// The chunk batch could not be embedded.
    Embedding(EmbeddingError),
}

impl std::fmt::Display for MarkdownEmbeddingPipelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Chunking(error) => write!(formatter, "Markdown chunking failed: {error}"),
            Self::Embedding(error) => write!(formatter, "Markdown embedding failed: {error}"),
        }
    }
}

impl std::error::Error for MarkdownEmbeddingPipelineError {}

/// Run chunking immediately after normalized Markdown extraction.
///
/// The returned batch is intentionally limited to chunk records and stable
/// document identity so later embedding and storage phases do not depend on
/// Markdown parser internals.
#[must_use]
pub fn chunk_markdown_extraction(
    outcome: &MarkdownExtractionOutcome,
    source: &str,
    config: MarkdownChunkingConfig,
    token_counter: &impl MarkdownTokenCounter,
) -> MarkdownChunkingPipelineOutcome {
    match outcome {
        MarkdownExtractionOutcome::Extracted(extraction) => {
            let document = match MarkdownChunkDocument::from_extraction_record(extraction, source) {
                Ok(document) => document,
                Err(error) => {
                    return MarkdownChunkingPipelineOutcome::Failed(failure_from_chunking_error(
                        extraction.package.clone(),
                        extraction.document_stem.clone(),
                        extraction.document_hash.clone(),
                        &error,
                    ));
                }
            };

            match chunk_markdown_document(&document, config, token_counter) {
                Ok(chunks) => MarkdownChunkingPipelineOutcome::Chunked(MarkdownChunkBatch {
                    package: extraction.package.clone(),
                    document_stem: extraction.document_stem.clone(),
                    document_hash: extraction.document_hash.clone(),
                    chunks,
                }),
                Err(error) => MarkdownChunkingPipelineOutcome::Failed(failure_from_chunking_error(
                    extraction.package.clone(),
                    extraction.document_stem.clone(),
                    extraction.document_hash.clone(),
                    &error,
                )),
            }
        }
        MarkdownExtractionOutcome::Failed(failure) => {
            MarkdownChunkingPipelineOutcome::Failed(failure_from_extraction_error(failure))
        }
        _ => MarkdownChunkingPipelineOutcome::Failed(MarkdownChunkingFailureRecord {
            package: None,
            document_stem: "unknown".to_owned(),
            document_hash: String::new(),
            error: MarkdownChunkingPipelineError::MalformedExtractionInput {
                kind: "unsupported_extraction_outcome".to_owned(),
                message: "Markdown extraction returned an unsupported outcome variant.".to_owned(),
                source_span: None,
                details: BTreeMap::new(),
            },
        }),
    }
}

/// Run embedding immediately after governed Markdown chunk generation.
///
/// Embedding receives the full chunk text batch for the document, preserving
/// model metadata and vector shape validation before later storage phases.
#[must_use]
pub fn embed_markdown_extraction(
    outcome: &MarkdownExtractionOutcome,
    source: &str,
    config: MarkdownChunkingConfig,
    token_counter: &impl MarkdownTokenCounter,
    embedder: &impl Embedder,
) -> MarkdownEmbeddingPipelineOutcome {
    match chunk_markdown_extraction(outcome, source, config, token_counter) {
        MarkdownChunkingPipelineOutcome::Chunked(batch) => {
            match embed_markdown_chunks(embedder, &batch.chunks) {
                Ok(chunks) => {
                    MarkdownEmbeddingPipelineOutcome::Embedded(EmbeddedMarkdownChunkBatch {
                        package: batch.package,
                        document_stem: batch.document_stem,
                        document_hash: batch.document_hash,
                        chunks,
                    })
                }
                Err(error) => {
                    MarkdownEmbeddingPipelineOutcome::Failed(MarkdownEmbeddingFailureRecord {
                        package: batch.package,
                        document_stem: batch.document_stem,
                        document_hash: batch.document_hash,
                        error: MarkdownEmbeddingPipelineError::Embedding(error),
                    })
                }
            }
        }
        MarkdownChunkingPipelineOutcome::Failed(failure) => {
            MarkdownEmbeddingPipelineOutcome::Failed(MarkdownEmbeddingFailureRecord {
                package: failure.package,
                document_stem: failure.document_stem,
                document_hash: failure.document_hash,
                error: MarkdownEmbeddingPipelineError::Chunking(failure.error),
            })
        }
    }
}

fn failure_from_extraction_error(
    failure: &MarkdownExtractionErrorRecord,
) -> MarkdownChunkingFailureRecord {
    MarkdownChunkingFailureRecord {
        package: failure.package.clone(),
        document_stem: failure.document_stem.clone(),
        document_hash: failure.document_hash.clone(),
        error: pipeline_error_from_extraction_error(&failure.error),
    }
}

fn pipeline_error_from_extraction_error(
    error: &MarkdownExtractionError,
) -> MarkdownChunkingPipelineError {
    if error.kind == "unsupported_markdown_structure" {
        return MarkdownChunkingPipelineError::UnsupportedMarkdownStructure {
            kind: error.kind.clone(),
            message: error.message.clone(),
            source_span: Some(error.source_span),
            details: error.details.clone(),
        };
    }

    MarkdownChunkingPipelineError::MalformedExtractionInput {
        kind: error.kind.clone(),
        message: error.message.clone(),
        source_span: Some(error.source_span),
        details: error.details.clone(),
    }
}

fn failure_from_chunking_error(
    package: Option<String>,
    document_stem: String,
    document_hash: String,
    error: &MarkdownChunkingError,
) -> MarkdownChunkingFailureRecord {
    let pipeline_error = match error {
        MarkdownChunkingError::InvalidBodySpan { span, source_len } => {
            let mut details = BTreeMap::new();
            details.insert("source_len".to_owned(), source_len.to_string());
            MarkdownChunkingPipelineError::MalformedExtractionInput {
                kind: "invalid_body_span".to_owned(),
                message: format!(
                    "Markdown extraction body span {}..{} is invalid for source length {source_len}",
                    span.start, span.end
                ),
                source_span: Some(*span),
                details,
            }
        }
        MarkdownChunkingError::OversizedMarkdownBlock { token_count, maximum_token_count } => {
            MarkdownChunkingPipelineError::UnsplittableOversizedBlock {
                message: format!(
                    "Markdown block with {token_count} tokens cannot be split without breaking Markdown structure; maximum is {maximum_token_count}"
                ),
                token_count: *token_count,
                maximum_token_count: *maximum_token_count,
            }
        }
    };

    MarkdownChunkingFailureRecord { package, document_stem, document_hash, error: pipeline_error }
}

#[cfg(test)]
#[path = "pipeline_test.rs"]
mod tests;
