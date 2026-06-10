//! Markdown chunking contracts for local RAG indexing.

use runtime_markdown::{MarkdownExtractionRecord, MarkdownHeading, MarkdownSourceSpan};

type ParsedAtxHeading<'a> = (u8, &'a str);
type HeadingContextSplit<'a> = (Option<String>, &'a str);
type MarkdownBlockParse = (MarkdownBlock, usize);
type FenceMarker = (char, usize);
type TableBlockParse = (Vec<MarkdownBlock>, usize);

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
    /// A Markdown block cannot fit without being split inside an unsafe structure.
    OversizedMarkdownBlock {
        /// Number of tokens in the unsplittable block.
        token_count: usize,
        /// Configured maximum chunk token count.
        maximum_token_count: usize,
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
            Self::OversizedMarkdownBlock { token_count, maximum_token_count } => write!(
                formatter,
                "Markdown block with {token_count} tokens exceeds maximum chunk token count {maximum_token_count}"
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
    config: MarkdownChunkingConfig,
    token_counter: &impl MarkdownTokenCounter,
) -> Result<Vec<MarkdownChunkRecord>, MarkdownChunkingError> {
    let sections = heading_sections(document)?;
    let mut chunk_texts = Vec::new();
    for section in sections {
        let split_sections = split_oversized_section(&section, config, token_counter)?;
        chunk_texts.extend(
            split_sections
                .into_iter()
                .map(|text| HeadingSection { heading_path: section.heading_path.clone(), text }),
        );
    }

    let mut chunks = chunk_texts
        .into_iter()
        .enumerate()
        .map(|(chunk_ordinal, chunk)| {
            let token_count = token_counter.count_tokens(&chunk.text);
            let chunk_hash = stable_chunk_hash(
                document.package.as_deref(),
                &document.document_stem,
                chunk_ordinal,
                &chunk.heading_path,
                &chunk.text,
            );
            let chunk_id = chunk_id(
                document.package.as_deref(),
                &document.document_stem,
                &chunk.heading_path,
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
                heading_path: chunk.heading_path,
                text: chunk.text,
                token_count,
                previous_chunk_id: None,
                next_chunk_id: None,
            }
        })
        .collect::<Vec<_>>();

    populate_neighbor_chunk_ids(&mut chunks);

    Ok(chunks)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeadingSection {
    heading_path: Vec<String>,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkdownBlock {
    text: String,
    splittable: bool,
}

fn split_oversized_section(
    section: &HeadingSection,
    config: MarkdownChunkingConfig,
    token_counter: &impl MarkdownTokenCounter,
) -> Result<Vec<String>, MarkdownChunkingError> {
    if token_counter.count_tokens(&section.text) <= config.maximum_token_count() {
        return Ok(vec![section.text.clone()]);
    }

    let (heading_context, body) = split_heading_context(&section.text);
    if body.trim().is_empty() {
        return Ok(vec![section.text.clone()]);
    }

    let blocks = split_long_splittable_blocks(
        markdown_blocks(body),
        heading_context.as_deref(),
        config.maximum_token_count(),
        token_counter,
    )?;

    pack_oversized_blocks(heading_context.as_deref(), &blocks, config, token_counter)
}

fn split_heading_context(section_text: &str) -> HeadingContextSplit<'_> {
    let Some(first_line_end) = section_text.find('\n') else {
        return if parse_atx_heading(section_text).is_some() {
            (Some(section_text.to_owned()), "")
        } else {
            (None, section_text)
        };
    };

    let first_line = &section_text[..first_line_end];
    if parse_atx_heading(first_line).is_none() {
        return (None, section_text);
    }

    let body_start = first_line_end + 1;
    (Some(first_line.to_owned()), section_text[body_start..].trim_start_matches('\n'))
}

fn split_long_splittable_blocks(
    blocks: Vec<MarkdownBlock>,
    heading_context: Option<&str>,
    maximum_token_count: usize,
    token_counter: &impl MarkdownTokenCounter,
) -> Result<Vec<MarkdownBlock>, MarkdownChunkingError> {
    let mut split_blocks = Vec::new();

    for block in blocks {
        let block_token_count =
            token_counter.count_tokens(&text_with_heading_context(heading_context, &[&block.text]));
        if block_token_count <= maximum_token_count {
            split_blocks.push(block);
            continue;
        }

        if !block.splittable {
            return Err(MarkdownChunkingError::OversizedMarkdownBlock {
                token_count: block_token_count,
                maximum_token_count,
            });
        }

        split_blocks.extend(split_plain_text_block(
            &block.text,
            heading_context,
            maximum_token_count,
            token_counter,
        )?);
    }

    Ok(split_blocks)
}

fn split_plain_text_block(
    block: &str,
    heading_context: Option<&str>,
    maximum_token_count: usize,
    token_counter: &impl MarkdownTokenCounter,
) -> Result<Vec<MarkdownBlock>, MarkdownChunkingError> {
    let mut blocks = Vec::new();
    let mut current = String::new();

    for word in block.split_whitespace() {
        let candidate =
            if current.is_empty() { word.to_owned() } else { format!("{current} {word}") };
        let candidate_token_count =
            token_counter.count_tokens(&text_with_heading_context(heading_context, &[&candidate]));

        if candidate_token_count <= maximum_token_count {
            current = candidate;
        } else if current.is_empty() {
            return Err(MarkdownChunkingError::OversizedMarkdownBlock {
                token_count: candidate_token_count,
                maximum_token_count,
            });
        } else {
            blocks.push(MarkdownBlock { text: std::mem::take(&mut current), splittable: true });
            word.clone_into(&mut current);
        }
    }

    if !current.is_empty() {
        blocks.push(MarkdownBlock { text: current, splittable: true });
    }

    Ok(blocks)
}

fn pack_oversized_blocks(
    heading_context: Option<&str>,
    blocks: &[MarkdownBlock],
    config: MarkdownChunkingConfig,
    token_counter: &impl MarkdownTokenCounter,
) -> Result<Vec<String>, MarkdownChunkingError> {
    let mut chunks = Vec::new();
    let mut current = Vec::<MarkdownBlock>::new();

    for block in blocks {
        if current.is_empty() {
            ensure_block_fits(heading_context, block, config.maximum_token_count(), token_counter)?;
            current.push(block.clone());
            continue;
        }

        let candidate = candidate_blocks(&current, block);
        let candidate_token_count =
            token_counter.count_tokens(&blocks_with_heading_context(heading_context, &candidate));
        if candidate_token_count <= config.target_token_count()
            || candidate_token_count <= config.maximum_token_count()
                && current_token_count(&current, heading_context, token_counter)
                    < config.target_token_count()
        {
            current.push(block.clone());
            continue;
        }

        chunks.push(blocks_with_heading_context(heading_context, &current));
        current =
            overlap_blocks(&current, heading_context, config.overlap_token_count(), token_counter);

        let candidate = candidate_blocks(&current, block);
        let candidate_token_count =
            token_counter.count_tokens(&blocks_with_heading_context(heading_context, &candidate));
        if !current.is_empty() && candidate_token_count > config.maximum_token_count() {
            current.clear();
        }

        ensure_block_fits(heading_context, block, config.maximum_token_count(), token_counter)?;
        current.push(block.clone());
    }

    if !current.is_empty() {
        chunks.push(blocks_with_heading_context(heading_context, &current));
    }

    Ok(chunks)
}

fn current_token_count(
    current: &[MarkdownBlock],
    heading_context: Option<&str>,
    token_counter: &impl MarkdownTokenCounter,
) -> usize {
    token_counter.count_tokens(&blocks_with_heading_context(heading_context, current))
}

fn ensure_block_fits(
    heading_context: Option<&str>,
    block: &MarkdownBlock,
    maximum_token_count: usize,
    token_counter: &impl MarkdownTokenCounter,
) -> Result<(), MarkdownChunkingError> {
    let token_count =
        token_counter.count_tokens(&text_with_heading_context(heading_context, &[&block.text]));
    if token_count <= maximum_token_count {
        return Ok(());
    }

    Err(MarkdownChunkingError::OversizedMarkdownBlock { token_count, maximum_token_count })
}

fn candidate_blocks(current: &[MarkdownBlock], block: &MarkdownBlock) -> Vec<MarkdownBlock> {
    let mut candidate = current.to_vec();
    candidate.push(block.clone());
    candidate
}

fn overlap_blocks(
    blocks: &[MarkdownBlock],
    heading_context: Option<&str>,
    overlap_token_count: usize,
    token_counter: &impl MarkdownTokenCounter,
) -> Vec<MarkdownBlock> {
    if overlap_token_count == 0 {
        return Vec::new();
    }

    let mut overlap = Vec::new();
    for block in blocks.iter().rev() {
        let mut candidate = vec![block.clone()];
        candidate.extend(overlap);
        let candidate_token_count =
            token_counter.count_tokens(&blocks_with_heading_context(heading_context, &candidate));
        if candidate_token_count > overlap_token_count {
            return candidate.into_iter().skip(1).collect();
        }
        overlap = candidate;
    }

    overlap
}

fn blocks_with_heading_context(heading_context: Option<&str>, blocks: &[MarkdownBlock]) -> String {
    let block_texts = blocks.iter().map(|block| block.text.as_str()).collect::<Vec<_>>();
    text_with_heading_context(heading_context, &block_texts)
}

fn text_with_heading_context(heading_context: Option<&str>, body_blocks: &[&str]) -> String {
    let body = body_blocks
        .iter()
        .copied()
        .filter(|block| !block.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    match (heading_context, body.is_empty()) {
        (Some(heading), false) => format!("{heading}\n\n{body}"),
        (Some(heading), true) => heading.to_owned(),
        (None, false) => body,
        (None, true) => String::new(),
    }
}

fn markdown_blocks(text: &str) -> Vec<MarkdownBlock> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut blocks = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if lines[index].trim().is_empty() {
            index += 1;
            continue;
        }

        if let Some((block, next_index)) = fenced_code_block(&lines, index) {
            blocks.push(block);
            index = next_index;
        } else if is_table_start(&lines, index) {
            let (table_blocks, next_index) = table_blocks(&lines, index);
            blocks.extend(table_blocks);
            index = next_index;
        } else if is_list_item_start(lines[index]) {
            let (block, next_index) = list_item_block(&lines, index);
            blocks.push(block);
            index = next_index;
        } else {
            let (block, next_index) = paragraph_block(&lines, index);
            blocks.push(block);
            index = next_index;
        }
    }

    blocks
}

fn fenced_code_block(lines: &[&str], start: usize) -> Option<MarkdownBlockParse> {
    let fence = fence_marker(lines[start])?;
    let mut end = start + 1;

    while end < lines.len() {
        if closing_fence(lines[end], fence) {
            end += 1;
            break;
        }
        end += 1;
    }

    Some((MarkdownBlock { text: lines[start..end].join("\n"), splittable: false }, end))
}

fn fence_marker(line: &str) -> Option<FenceMarker> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }

    let count = trimmed.chars().take_while(|character| *character == marker).count();
    if count >= 3 { Some((marker, count)) } else { None }
}

fn closing_fence(line: &str, fence: FenceMarker) -> bool {
    let trimmed = line.trim_start();
    let count = trimmed.chars().take_while(|character| *character == fence.0).count();
    count >= fence.1
}

fn is_table_start(lines: &[&str], index: usize) -> bool {
    index + 1 < lines.len()
        && looks_like_table_row(lines[index])
        && is_table_separator(lines[index + 1])
}

fn looks_like_table_row(line: &str) -> bool {
    line.trim().contains('|')
}

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return false;
    }

    trimmed
        .trim_matches('|')
        .split('|')
        .all(|cell| cell.trim().chars().all(|character| matches!(character, '-' | ':' | ' ')))
}

fn table_blocks(lines: &[&str], start: usize) -> TableBlockParse {
    let header = lines[start];
    let separator = lines[start + 1];
    let mut blocks = Vec::new();
    let mut index = start + 2;

    while index < lines.len() && looks_like_table_row(lines[index]) {
        let text = [header, separator, lines[index]].join("\n");
        blocks.push(MarkdownBlock { text, splittable: false });
        index += 1;
    }

    if blocks.is_empty() {
        blocks.push(MarkdownBlock { text: [header, separator].join("\n"), splittable: false });
    }

    (blocks, index)
}

fn is_list_item_start(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("+ ")
        || ordered_list_marker(trimmed).is_some()
}

fn ordered_list_marker(trimmed: &str) -> Option<()> {
    let marker_end = trimmed.find(['.', ')'])?;
    if marker_end == 0 {
        return None;
    }
    let marker = &trimmed[..marker_end];
    let rest = trimmed[marker_end + 1..].strip_prefix(' ')?;
    if marker.chars().all(|character| character.is_ascii_digit()) && !rest.is_empty() {
        Some(())
    } else {
        None
    }
}

fn list_item_block(lines: &[&str], start: usize) -> MarkdownBlockParse {
    let mut end = start + 1;

    while end < lines.len() {
        if lines[end].trim().is_empty() || is_list_item_start(lines[end]) {
            break;
        }
        end += 1;
    }

    (MarkdownBlock { text: lines[start..end].join("\n"), splittable: false }, end)
}

fn paragraph_block(lines: &[&str], start: usize) -> MarkdownBlockParse {
    let mut end = start + 1;

    while end < lines.len() {
        if lines[end].trim().is_empty()
            || fence_marker(lines[end]).is_some()
            || is_list_item_start(lines[end])
            || is_table_start(lines, end)
        {
            break;
        }
        end += 1;
    }

    (MarkdownBlock { text: lines[start..end].join("\n"), splittable: true }, end)
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

fn populate_neighbor_chunk_ids(chunks: &mut [MarkdownChunkRecord]) {
    let chunk_ids = chunks.iter().map(|chunk| chunk.chunk_id.clone()).collect::<Vec<_>>();

    for (index, chunk) in chunks.iter_mut().enumerate() {
        chunk.previous_chunk_id =
            index.checked_sub(1).map(|previous_index| chunk_ids[previous_index].clone());
        chunk.next_chunk_id = chunk_ids.get(index + 1).cloned();
    }
}

fn stable_chunk_hash(
    package: Option<&str>,
    document_stem: &str,
    chunk_ordinal: usize,
    heading_path: &[String],
    text: &str,
) -> String {
    let ordinal = chunk_ordinal.to_string();
    let heading_path = heading_path.join("\u{1f}");
    let normalized_text = normalized_chunk_text(text);
    let components = [
        package.unwrap_or(WORKSPACE_CHUNK_NAMESPACE),
        document_stem,
        &ordinal,
        &heading_path,
        &normalized_text,
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

fn normalized_chunk_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
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
