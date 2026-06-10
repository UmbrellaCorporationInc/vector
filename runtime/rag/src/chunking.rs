//! Markdown chunking contracts for local RAG indexing.

use runtime_markdown::{MarkdownExtractionRecord, MarkdownHeading, MarkdownSourceSpan};

/// Package identity used for workspace-local chunk identifiers.
pub const WORKSPACE_CHUNK_NAMESPACE: &str = "workspace";

/// Default heading slug used before heading-aware sectioning is implemented.
pub const ROOT_CHUNK_HEADING_SLUG: &str = "root";

/// Chunking configuration shared by Markdown chunking and the embedding boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarkdownChunkingConfig {
    target: usize,
    maximum: usize,
    overlap: usize,
}

impl MarkdownChunkingConfig {
    /// Create a chunking configuration.
    #[must_use]
    pub const fn new(
        target_token_count: usize,
        maximum_token_count: usize,
        overlap_token_count: usize,
    ) -> Self {
        Self {
            target: target_token_count,
            maximum: maximum_token_count,
            overlap: overlap_token_count,
        }
    }

    /// Return the Phase 4 defaults from the local RAG plan.
    #[must_use]
    pub const fn phase_four_defaults() -> Self {
        Self { target: crate::CHUNK_TOKEN_TARGET, maximum: crate::CHUNK_TOKEN_MAXIMUM, overlap: 0 }
    }

    /// Return the target chunk token count.
    #[must_use]
    pub const fn target_token_count(&self) -> usize {
        self.target
    }

    /// Return the maximum chunk token count.
    #[must_use]
    pub const fn maximum_token_count(&self) -> usize {
        self.maximum
    }

    /// Return the local overlap token count for oversized sections.
    #[must_use]
    pub const fn overlap_token_count(&self) -> usize {
        self.overlap
    }
}

impl Default for MarkdownChunkingConfig {
    fn default() -> Self {
        Self::phase_four_defaults()
    }
}

/// Token counting boundary used by chunking before embedding exists.
pub trait MarkdownTokenCounter {
    /// Count tokens in Markdown text.
    fn count_tokens(&self, text: &str) -> usize;
}

/// Temporary deterministic token counter based on Unicode whitespace.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct WhitespaceMarkdownTokenCounter;

impl MarkdownTokenCounter for WhitespaceMarkdownTokenCounter {
    fn count_tokens(&self, text: &str) -> usize {
        text.split_whitespace().count()
    }
}

/// Chunker input derived from the normalized Phase 3 extraction record.
///
/// # DTO(extraction-to-chunking boundary serialized by callers in later phases)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarkdownChunkDocument {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem.
    pub document_stem: String,
    /// Source document content hash from discovery.
    pub document_hash: String,
    /// Markdown body text after frontmatter removal.
    pub body: String,
    /// Extracted heading hierarchy from Phase 3.
    pub headings: Vec<MarkdownHeading>,
}

impl MarkdownChunkDocument {
    /// Build a chunker input document from normalized extraction output and source text.
    ///
    /// # Errors
    /// Returns [`MarkdownChunkingError::InvalidBodySpan`] when the extraction
    /// body span does not align with UTF-8 source boundaries.
    pub fn from_extraction_record(
        extraction: &MarkdownExtractionRecord,
        source: &str,
    ) -> Result<Self, MarkdownChunkingError> {
        let body = source
            .get(extraction.body_span.start..extraction.body_span.end)
            .ok_or(MarkdownChunkingError::InvalidBodySpan {
                span: extraction.body_span,
                source_len: source.len(),
            })?
            .to_owned();

        Ok(Self {
            package: extraction.package.clone(),
            document_stem: extraction.document_stem.clone(),
            document_hash: extraction.document_hash.clone(),
            body,
            headings: extraction.headings.clone(),
        })
    }
}

/// Markdown chunk record emitted before embedding.
///
/// # DTO(chunking output stored and embedded by later RAG phases)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarkdownChunkRecord {
    /// Stable chunk identifier.
    pub chunk_id: String,
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem.
    pub document_stem: String,
    /// Source document content hash from discovery.
    pub document_hash: String,
    /// Stable hash of normalized chunk text and structural metadata.
    pub chunk_hash: String,
    /// Zero-based ordinal within the document.
    pub chunk_ordinal: usize,
    /// Heading hierarchy associated with this chunk.
    pub heading_path: Vec<String>,
    /// Markdown text to embed and store.
    pub text: String,
    /// Token count used by chunking limit enforcement.
    pub token_count: usize,
    /// Previous adjacent chunk identifier in the same document.
    pub previous_chunk_id: Option<String>,
    /// Next adjacent chunk identifier in the same document.
    pub next_chunk_id: Option<String>,
}

/// Chunking failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownChunkingError {
    /// Extraction body span did not align with source text.
    InvalidBodySpan {
        /// Invalid extraction body span.
        span: MarkdownSourceSpan,
        /// Source text length.
        source_len: usize,
    },
}

impl std::fmt::Display for MarkdownChunkingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBodySpan { span, source_len } => write!(
                formatter,
                "Markdown chunking input body span {}..{} is invalid for source length {source_len}",
                span.start, span.end
            ),
        }
    }
}

impl std::error::Error for MarkdownChunkingError {}

/// Produce deterministic contract-level chunks for a normalized Markdown document.
///
/// Phase A establishes the output shape and tokenizer boundary. Heading-aware
/// sectioning and oversized section splitting are implemented in later phases.
///
/// # Errors
/// This function currently has no runtime error cases.
pub fn chunk_markdown_document(
    document: &MarkdownChunkDocument,
    _config: MarkdownChunkingConfig,
    token_counter: &impl MarkdownTokenCounter,
) -> Result<Vec<MarkdownChunkRecord>, MarkdownChunkingError> {
    let text = document.body.trim().to_owned();
    if text.is_empty() {
        return Ok(Vec::new());
    }

    let heading_path =
        document.headings.first().map_or_else(Vec::new, |heading| heading.path.clone());
    let token_count = token_counter.count_tokens(&text);
    let chunk_ordinal = 0;
    let chunk_hash = stable_chunk_hash(
        document.package.as_deref(),
        &document.document_stem,
        &document.document_hash,
        chunk_ordinal,
        &heading_path,
        &text,
    );
    let chunk_id = chunk_id(
        document.package.as_deref(),
        &document.document_stem,
        &heading_path,
        chunk_ordinal,
        &chunk_hash,
    );

    Ok(vec![MarkdownChunkRecord {
        chunk_id,
        package: document.package.clone(),
        document_stem: document.document_stem.clone(),
        document_hash: document.document_hash.clone(),
        chunk_hash,
        chunk_ordinal,
        heading_path,
        text,
        token_count,
        previous_chunk_id: None,
        next_chunk_id: None,
    }])
}

fn chunk_id(
    package: Option<&str>,
    document_stem: &str,
    heading_path: &[String],
    chunk_ordinal: usize,
    chunk_hash: &str,
) -> String {
    let package_namespace = package.unwrap_or(WORKSPACE_CHUNK_NAMESPACE);
    let heading_slug = heading_path
        .last()
        .map_or_else(|| ROOT_CHUNK_HEADING_SLUG.to_owned(), |heading| slug_component(heading));

    format!("{package_namespace}/{document_stem}/{heading_slug}/{chunk_ordinal:04}/{chunk_hash}")
}

fn stable_chunk_hash(
    package: Option<&str>,
    document_stem: &str,
    document_hash: &str,
    chunk_ordinal: usize,
    heading_path: &[String],
    text: &str,
) -> String {
    let ordinal = chunk_ordinal.to_string();
    let heading_path = heading_path.join("\u{1f}");
    let components = [
        package.unwrap_or(WORKSPACE_CHUNK_NAMESPACE),
        document_stem,
        document_hash,
        &ordinal,
        &heading_path,
        text,
    ];
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for component in components {
        for byte in component.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("{hash:016x}")
}

fn slug_component(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_separator = false;

    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_separator = false;
        } else if !previous_separator && !slug.is_empty() {
            slug.push('-');
            previous_separator = true;
        }
    }

    if previous_separator {
        slug.pop();
    }
    if slug.is_empty() { ROOT_CHUNK_HEADING_SLUG.to_owned() } else { slug }
}

#[cfg(test)]
#[path = "chunking_test.rs"]
mod tests;
