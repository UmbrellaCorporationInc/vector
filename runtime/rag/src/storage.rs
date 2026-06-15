//! `LanceDB` Phase 6 storage contract for persisted RAG chunks.

use crate::{EmbeddedMarkdownChunkRecord, EmbeddingVector, WORKSPACE_CHUNK_NAMESPACE};
use runtime_markdown::{MarkdownExtractionRecord, MarkdownMetadataValue};
use std::collections::BTreeMap;

/// Primary `LanceDB` table name for persisted retrieval chunks.
pub const LANCEDB_PRIMARY_CHUNK_TABLE: &str = "rag_chunks";

/// Stable selected frontmatter filter value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LanceDbFilterValue {
    /// Scalar value stored as a stable string representation.
    Scalar(String),
    /// Flat string list value used for set-membership style filters.
    StringList(Vec<String>),
}

/// Package-aware governed document identity resolved during extraction.
///
/// # DTO(persisted provenance boundary consumed by later storage and retrieval phases)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GovernedDocumentIdentity {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Parsed governed document type when the stem is valid.
    pub document_type: Option<String>,
    /// Parsed governed document code when the stem is valid.
    pub document_code: Option<String>,
    /// Parsed governed document slug when the stem is valid.
    pub document_slug: Option<String>,
}

/// Query-oriented metadata representation for `LanceDB` filters and debugging.
///
/// # DTO(storage-side filter contract consumed by later retrieval phases)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LanceDbChunkMetadata {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Ordered heading hierarchy from the chunk contract.
    pub heading_path: Vec<String>,
    /// Flattened heading path for debugging and future string filters.
    pub heading_path_text: String,
    /// Stable tag list derived from frontmatter.
    pub tags: Vec<String>,
    /// Selected frontmatter root fields that are safe to filter directly.
    pub frontmatter_fields: BTreeMap<String, LanceDbFilterValue>,
}

/// Persisted `LanceDB` row contract for one embedded Markdown chunk.
///
/// # DTO(embedding-to-storage row contract consumed by later `LanceDB` lifecycle operations)
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LanceDbChunkRow {
    /// Stable chunk identifier derived from package, stem, ordinal, and hash.
    pub chunk_id: String,
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Source document content hash from discovery.
    pub document_hash: String,
    /// Stable chunk content hash from chunking.
    pub chunk_hash: String,
    /// Zero-based chunk ordinal within the document.
    pub chunk_ordinal: usize,
    /// Ordered heading hierarchy associated with this chunk.
    pub heading_path: Vec<String>,
    /// Parsed frontmatter preserved for inspection and later schema shaping.
    pub frontmatter: Option<MarkdownMetadataValue>,
    /// Raw chunk text preserved for inspection and full-text indexing.
    pub text: String,
    /// Token count emitted by chunking.
    pub token_count: usize,
    /// Embedding model identifier stored with the vector payload.
    pub embedding_model: String,
    /// Embedding dimension stored with the vector payload.
    pub embedding_dimension: usize,
    /// Dense embedding vector payload.
    pub vector: EmbeddingVector,
    /// Package-aware governed document identity for provenance.
    pub governed_document: GovernedDocumentIdentity,
    /// Query-oriented metadata representation for filters and debugging.
    pub metadata: LanceDbChunkMetadata,
}

/// Build a stable chunk identifier for persisted `LanceDB` upserts.
#[must_use]
pub fn stable_chunk_id(
    package: Option<&str>,
    document_stem: &str,
    chunk_ordinal: usize,
    chunk_hash: &str,
) -> String {
    let package_namespace = package.unwrap_or(WORKSPACE_CHUNK_NAMESPACE);
    format!("{package_namespace}/{document_stem}/{chunk_ordinal:04}/{chunk_hash}")
}

/// Build the persisted Phase 6 row contract from extraction and embedding output.
#[must_use]
pub fn lancedb_chunk_row(
    extraction: &MarkdownExtractionRecord,
    chunk: &EmbeddedMarkdownChunkRecord,
) -> LanceDbChunkRow {
    let identity = GovernedDocumentIdentity {
        package: extraction.package.clone(),
        document_stem: extraction.document_stem.clone(),
        document_type: extraction.document_type.clone(),
        document_code: extraction.document_code.clone(),
        document_slug: extraction.document_slug.clone(),
    };
    let metadata = lancedb_chunk_metadata(extraction, chunk);

    LanceDbChunkRow {
        chunk_id: chunk.chunk.chunk_id.clone(),
        package: chunk.chunk.package.clone(),
        document_stem: chunk.chunk.document_stem.clone(),
        document_hash: chunk.chunk.document_hash.clone(),
        chunk_hash: chunk.chunk.chunk_hash.clone(),
        chunk_ordinal: chunk.chunk.chunk_ordinal,
        heading_path: chunk.chunk.heading_path.clone(),
        frontmatter: extraction
            .frontmatter
            .as_ref()
            .map(|frontmatter| frontmatter.metadata.clone()),
        text: chunk.chunk.text.clone(),
        token_count: chunk.chunk.token_count,
        embedding_model: chunk.embedding_model.clone(),
        embedding_dimension: chunk.embedding_dimension,
        vector: chunk.embedding.clone(),
        governed_document: identity,
        metadata,
    }
}

fn lancedb_chunk_metadata(
    extraction: &MarkdownExtractionRecord,
    chunk: &EmbeddedMarkdownChunkRecord,
) -> LanceDbChunkMetadata {
    LanceDbChunkMetadata {
        package: chunk.chunk.package.clone(),
        document_stem: chunk.chunk.document_stem.clone(),
        heading_path: chunk.chunk.heading_path.clone(),
        heading_path_text: chunk.chunk.heading_path.join(" / "),
        tags: frontmatter_tags(extraction),
        frontmatter_fields: selected_frontmatter_fields(extraction),
    }
}

fn frontmatter_tags(extraction: &MarkdownExtractionRecord) -> Vec<String> {
    selected_frontmatter_fields(extraction)
        .get("tags")
        .map(|value| match value {
            LanceDbFilterValue::StringList(values) => values.clone(),
            LanceDbFilterValue::Scalar(value) => vec![value.clone()],
        })
        .unwrap_or_default()
}

fn selected_frontmatter_fields(
    extraction: &MarkdownExtractionRecord,
) -> BTreeMap<String, LanceDbFilterValue> {
    let Some(frontmatter) = extraction.frontmatter.as_ref() else {
        return BTreeMap::new();
    };
    let MarkdownMetadataValue::Mapping(mapping) = &frontmatter.metadata else {
        return BTreeMap::new();
    };

    mapping
        .iter()
        .filter_map(|(key, value)| {
            filterable_frontmatter_value(value).map(|value| (key.clone(), value))
        })
        .collect()
}

fn filterable_frontmatter_value(value: &MarkdownMetadataValue) -> Option<LanceDbFilterValue> {
    if let MarkdownMetadataValue::Bool(value) = value {
        return Some(LanceDbFilterValue::Scalar(value.to_string()));
    }
    if let MarkdownMetadataValue::Number(value) | MarkdownMetadataValue::String(value) = value {
        return Some(LanceDbFilterValue::Scalar(value.clone()));
    }
    if let MarkdownMetadataValue::Sequence(values) = value {
        let strings = values.iter().map(sequence_string_value).collect::<Option<Vec<_>>>()?;
        return Some(LanceDbFilterValue::StringList(strings));
    }

    None
}

fn sequence_string_value(value: &MarkdownMetadataValue) -> Option<String> {
    if let MarkdownMetadataValue::Bool(value) = value {
        return Some(value.to_string());
    }
    if let MarkdownMetadataValue::Number(value) | MarkdownMetadataValue::String(value) = value {
        return Some(value.clone());
    }

    None
}

#[cfg(test)]
#[path = "storage_test.rs"]
mod tests;
