//! Markdown chunking contracts for local RAG indexing.

use runtime_markdown::{MarkdownExtractionRecord, MarkdownHeading, MarkdownSourceSpan};

type ParsedAtxHeading<'a> = (u8, &'a str);

/// Package identity used for workspace-local chunk identifiers.
pub const WORKSPACE_CHUNK_NAMESPACE: &str = "workspace";

/// Default heading slug used for root-level content before the first heading.
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
    /// Source start offset for the body inside the original Markdown file.
    pub body_start: usize,
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
            body_start: extraction.body_span.start,
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
/// Phase B adds heading-aware sectioning. Oversized section splitting is
/// implemented in a later phase.
///
/// # Errors
/// Returns [`MarkdownChunkingError::InvalidBodySpan`] when heading source spans
/// do not align with the extracted document body.
pub fn chunk_markdown_document(
    document: &MarkdownChunkDocument,
    _config: MarkdownChunkingConfig,
    token_counter: &impl MarkdownTokenCounter,
) -> Result<Vec<MarkdownChunkRecord>, MarkdownChunkingError> {
    let sections = heading_sections(document)?;
    let chunks = sections
        .into_iter()
        .enumerate()
        .map(|(chunk_ordinal, section)| {
            let token_count = token_counter.count_tokens(&section.text);
            let chunk_hash = stable_chunk_hash(
                document.package.as_deref(),
                &document.document_stem,
                &document.document_hash,
                chunk_ordinal,
                &section.heading_path,
                &section.text,
            );
            let chunk_id = chunk_id(
                document.package.as_deref(),
                &document.document_stem,
                &section.heading_path,
                chunk_ordinal,
                &chunk_hash,
            );

            MarkdownChunkRecord {
                chunk_id,
                package: document.package.clone(),
                document_stem: document.document_stem.clone(),
                document_hash: document.document_hash.clone(),
                chunk_hash,
                chunk_ordinal,
                heading_path: section.heading_path,
                text: section.text,
                token_count,
                previous_chunk_id: None,
                next_chunk_id: None,
            }
        })
        .collect();

    Ok(chunks)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeadingSection {
    heading_path: Vec<String>,
    text: String,
}

fn heading_sections(
    document: &MarkdownChunkDocument,
) -> Result<Vec<HeadingSection>, MarkdownChunkingError> {
    let mut sections = Vec::new();
    let mut sorted_headings = document.headings.iter().collect::<Vec<_>>();
    sorted_headings.sort_by_key(|heading| (heading.source_span.start, heading.ordinal));

    if let Some(first_heading) = sorted_headings.first() {
        let first_heading_start = relative_offset(document, first_heading.source_span.start)?;
        push_section(&mut sections, Vec::new(), &document.body[..first_heading_start]);
    } else {
        push_section(&mut sections, Vec::new(), &document.body);
        return Ok(sections);
    }

    for (index, heading) in sorted_headings.iter().enumerate() {
        let section_start = relative_offset(document, heading.source_span.start)?;
        let direct_content_end = next_heading_start(document, &sorted_headings, index, |_| true)?;
        let section_end = next_heading_start(document, &sorted_headings, index, |next_heading| {
            next_heading.level <= heading.level
        })?;

        if section_start > section_end || section_end > document.body.len() {
            return Err(MarkdownChunkingError::InvalidBodySpan {
                span: heading.source_span,
                source_len: document.body_start + document.body.len(),
            });
        }

        let raw_text = &document.body[section_start..section_end];
        let direct_text = &document.body[section_start..direct_content_end];
        if contains_non_heading_content(direct_text) {
            push_section(&mut sections, heading.path.clone(), raw_text);
        }
    }

    Ok(sections)
}

fn next_heading_start<P>(
    document: &MarkdownChunkDocument,
    headings: &[&MarkdownHeading],
    current_index: usize,
    predicate: P,
) -> Result<usize, MarkdownChunkingError>
where
    P: Fn(&MarkdownHeading) -> bool,
{
    headings[current_index + 1..]
        .iter()
        .find(|heading| predicate(heading))
        .map_or(Ok(document.body.len()), |heading| {
            relative_offset(document, heading.source_span.start)
        })
}

fn relative_offset(
    document: &MarkdownChunkDocument,
    source_offset: usize,
) -> Result<usize, MarkdownChunkingError> {
    let relative = source_offset.checked_sub(document.body_start).ok_or_else(|| {
        MarkdownChunkingError::InvalidBodySpan {
            span: MarkdownSourceSpan::new(source_offset, source_offset),
            source_len: document.body_start + document.body.len(),
        }
    })?;
    if relative <= document.body.len() {
        return Ok(relative);
    }

    Err(MarkdownChunkingError::InvalidBodySpan {
        span: MarkdownSourceSpan::new(source_offset, source_offset),
        source_len: document.body_start + document.body.len(),
    })
}

fn push_section(sections: &mut Vec<HeadingSection>, heading_path: Vec<String>, raw_text: &str) {
    let text = raw_text.trim().to_owned();
    if !text.is_empty() {
        sections.push(HeadingSection { heading_path, text });
    }
}

fn contains_non_heading_content(raw_text: &str) -> bool {
    raw_text
        .lines()
        .skip(1)
        .map(str::trim)
        .any(|line| !line.is_empty() && parse_atx_heading(line).is_none())
}

fn parse_atx_heading(line: &str) -> Option<ParsedAtxHeading<'_>> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|character| *character == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = trimmed[level..].strip_prefix(' ')?;
    let text = rest.trim().trim_end_matches('#').trim();
    if text.is_empty() {
        return None;
    }
    Some((u8::try_from(level).ok()?, text))
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
