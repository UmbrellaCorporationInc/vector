//! Markdown extraction APIs.

use crate::MarkdownDiscoveryRecord;
use runtime_io::{IoError, read_file_text};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

type FrontmatterCloserSpan = (usize, usize);
type ParsedAtxHeading<'a> = (u8, &'a str);
type ReferenceDefinition = (String, String);
type SplitLinkTarget<'a> = (&'a str, Option<&'a str>);

/// Source byte range in a Markdown file.
///
/// # DTO(extraction output serialized for tests, debugging, and future indexer boundaries)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MarkdownSourceSpan {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

impl MarkdownSourceSpan {
    /// Create a source span.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Supported frontmatter formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MarkdownFrontmatterFormat {
    /// YAML frontmatter delimited by `---`.
    Yaml,
    /// TOML frontmatter delimited by `+++` or `---toml`.
    Toml,
    /// JSON frontmatter delimited by `---json`.
    Json,
}

/// Structured frontmatter metadata value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum MarkdownMetadataValue {
    /// Null value.
    Null,
    /// Boolean value.
    Bool(bool),
    /// Numeric value represented as a stable string.
    Number(String),
    /// String value.
    String(String),
    /// Sequence value.
    Sequence(Vec<Self>),
    /// Mapping value.
    Mapping(BTreeMap<String, Self>),
}

/// Parsed frontmatter metadata.
///
/// # DTO(extraction output serialized for tests, debugging, and future indexer boundaries)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MarkdownFrontmatter {
    /// Frontmatter format.
    pub format: MarkdownFrontmatterFormat,
    /// Parsed structured metadata.
    pub metadata: MarkdownMetadataValue,
    /// Source span for the full frontmatter block including delimiters.
    pub source_span: MarkdownSourceSpan,
}

/// Extracted Markdown heading.
///
/// # DTO(extraction output serialized for tests, debugging, and future indexer boundaries)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MarkdownHeading {
    /// Heading level, from 1 to 6.
    pub level: u8,
    /// Visible heading text.
    pub text: String,
    /// Normalized heading anchor.
    pub anchor: String,
    /// Stable zero-based heading ordinal.
    pub ordinal: usize,
    /// Heading hierarchy path ending with this heading.
    pub path: Vec<String>,
    /// Source span for the heading line.
    pub source_span: MarkdownSourceSpan,
}

/// Extracted outbound link kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MarkdownLinkKind {
    /// Governed wikilink such as `[[rfc-00033-markdown-extraction]]`.
    Wikilink,
    /// Inline Markdown link such as `[label](target)`.
    Inline,
    /// Autolink such as `<https://example.com>`.
    Autolink,
    /// Resolved reference-style link.
    Reference,
}

/// Extracted outbound link.
///
/// # DTO(extraction output serialized for tests, debugging, and future indexer boundaries)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MarkdownLink {
    /// Link kind.
    pub kind: MarkdownLinkKind,
    /// Raw link source text.
    pub raw: String,
    /// Link target.
    pub target: String,
    /// Optional link label.
    pub label: Option<String>,
    /// Optional heading fragment or wikilink heading.
    pub heading: Option<String>,
    /// Source span for the link.
    pub source_span: MarkdownSourceSpan,
}

/// Non-fatal extraction diagnostic.
///
/// # DTO(extraction output serialized for tests, debugging, and future indexer boundaries)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MarkdownExtractionDiagnostic {
    /// Stable diagnostic kind.
    pub kind: String,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Optional source span for the diagnostic.
    pub source_span: Option<MarkdownSourceSpan>,
}

/// Successful Markdown extraction record.
///
/// # DTO(extraction output serialized for tests, debugging, and future indexer boundaries)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MarkdownExtractionRecord {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem.
    pub document_stem: String,
    /// Governed document type when the stem matches the governed naming shape.
    pub document_type: Option<String>,
    /// Governed document code when the stem matches the governed naming shape.
    pub document_code: Option<String>,
    /// Governed document slug when the stem matches the governed naming shape.
    pub document_slug: Option<String>,
    /// Content hash preserved from discovery.
    pub document_hash: String,
    /// Parsed frontmatter, if present.
    pub frontmatter: Option<MarkdownFrontmatter>,
    /// Extracted heading hierarchy.
    pub headings: Vec<MarkdownHeading>,
    /// Extracted outbound links.
    pub links: Vec<MarkdownLink>,
    /// Body range after frontmatter removal.
    pub body_span: MarkdownSourceSpan,
    /// Non-fatal extraction diagnostics.
    pub diagnostics: Vec<MarkdownExtractionDiagnostic>,
}

/// File-scoped extraction error.
///
/// # DTO(extraction output serialized for tests, debugging, and future indexer boundaries)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MarkdownExtractionErrorRecord {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem.
    pub document_stem: String,
    /// Content hash preserved from discovery.
    pub document_hash: String,
    /// Extraction error.
    pub error: MarkdownExtractionError,
}

/// Extraction error details.
///
/// # DTO(extraction output serialized for tests, debugging, and future indexer boundaries)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MarkdownExtractionError {
    /// Stable error kind.
    pub kind: String,
    /// Human-readable error message.
    pub message: String,
    /// Source span for the failing input.
    pub source_span: MarkdownSourceSpan,
    /// Structured string details.
    pub details: BTreeMap<String, String>,
}

/// Extraction outcome for one Markdown file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
#[non_exhaustive]
pub enum MarkdownExtractionOutcome {
    /// Extraction succeeded.
    Extracted(MarkdownExtractionRecord),
    /// Extraction failed for this file only.
    Failed(MarkdownExtractionErrorRecord),
}

/// Fatal extraction failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownExtractionFailure {
    /// The source file could not be read.
    ReadFile {
        /// File path display string.
        path: String,
        /// Underlying IO error message.
        message: String,
    },
}

impl std::fmt::Display for MarkdownExtractionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFile { path, message } => {
                write!(formatter, "Markdown extraction failed to read {path}: {message}")
            }
        }
    }
}

impl std::error::Error for MarkdownExtractionFailure {}

/// Extract Markdown metadata from a discovered Markdown file record.
///
/// # Errors
/// Returns [`MarkdownExtractionFailure::ReadFile`] when the source file cannot
/// be read through the `runtime-io` boundary.
pub async fn extract_markdown_file(
    record: &MarkdownDiscoveryRecord,
) -> Result<MarkdownExtractionOutcome, MarkdownExtractionFailure> {
    let source = read_file_text(record.internal_read_path()).await.map_err(|error| {
        MarkdownExtractionFailure::ReadFile {
            path: record.internal_read_path().as_path().display().to_string(),
            message: io_error_message(&error),
        }
    })?;

    Ok(extract_markdown_source(record, &source))
}

/// Extract Markdown metadata from an in-memory Markdown source.
#[must_use]
pub fn extract_markdown_source(
    record: &MarkdownDiscoveryRecord,
    source: &str,
) -> MarkdownExtractionOutcome {
    let parsed_frontmatter = match parse_frontmatter(source) {
        Ok(frontmatter) => frontmatter,
        Err(error) => {
            return MarkdownExtractionOutcome::Failed(MarkdownExtractionErrorRecord {
                package: record.package().map(ToOwned::to_owned),
                document_stem: record.governed_document_stem().to_owned(),
                document_hash: record.content_hash().to_string(),
                error,
            });
        }
    };

    let body_start = parsed_frontmatter.as_ref().map_or(0, |frontmatter| {
        frontmatter.source_span.end
            + trailing_line_break_len(&source[frontmatter.source_span.end..])
    });
    let body_span = MarkdownSourceSpan::new(body_start, source.len());
    let body = &source[body_start..];
    let mut diagnostics = Vec::new();
    diagnostics.push(MarkdownExtractionDiagnostic {
        kind: "parser_dependency_spike_deferred".to_owned(),
        message: "markdown::to_mdast and pulldown-cmark were not added because neither parser dependency is approved for runtime-markdown yet.".to_owned(),
        source_span: None,
    });

    let headings = extract_headings(body, body_start, &mut diagnostics);
    let links = extract_links(body, body_start, &mut diagnostics);
    let identity = parse_governed_identity(record.governed_document_stem());

    MarkdownExtractionOutcome::Extracted(MarkdownExtractionRecord {
        package: record.package().map(ToOwned::to_owned),
        document_stem: record.governed_document_stem().to_owned(),
        document_type: identity.as_ref().map(|value| value.document_type.clone()),
        document_code: identity.as_ref().map(|value| value.code.clone()),
        document_slug: identity.as_ref().map(|value| value.slug.clone()),
        document_hash: record.content_hash().to_string(),
        frontmatter: parsed_frontmatter,
        headings,
        links,
        body_span,
        diagnostics,
    })
}

fn parse_frontmatter(source: &str) -> Result<Option<MarkdownFrontmatter>, MarkdownExtractionError> {
    let Some(first_line_end) = source.find('\n') else {
        return Ok(None);
    };
    let opener = source[..first_line_end].trim_end_matches('\r');
    let (format, closer) = match opener {
        "---" | "---yaml" | "---yml" => (MarkdownFrontmatterFormat::Yaml, "---"),
        "---json" => (MarkdownFrontmatterFormat::Json, "---"),
        "---toml" => (MarkdownFrontmatterFormat::Toml, "---"),
        "+++" => (MarkdownFrontmatterFormat::Toml, "+++"),
        _ => return Ok(None),
    };

    let content_start = first_line_end + 1;
    let Some((closer_start, closer_end)) = find_frontmatter_closer(source, content_start, closer)
    else {
        return Err(frontmatter_error(
            format,
            "frontmatter closing delimiter was not found.",
            MarkdownSourceSpan::new(0, source.len()),
            None,
        ));
    };
    let raw = &source[content_start..closer_start];
    let metadata = match format {
        MarkdownFrontmatterFormat::Yaml | MarkdownFrontmatterFormat::Json => {
            parse_yaml_metadata(raw, format, MarkdownSourceSpan::new(0, closer_end))?
        }
        MarkdownFrontmatterFormat::Toml => {
            parse_toml_metadata(raw, MarkdownSourceSpan::new(0, closer_end))?
        }
    };

    Ok(Some(MarkdownFrontmatter {
        format,
        metadata,
        source_span: MarkdownSourceSpan::new(0, closer_end),
    }))
}

fn find_frontmatter_closer(
    source: &str,
    content_start: usize,
    closer: &str,
) -> Option<FrontmatterCloserSpan> {
    let mut offset = content_start;
    for segment in source[content_start..].split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        if line == closer {
            return Some((offset, offset + line.len()));
        }
        offset += segment.len();
    }
    None
}

fn parse_yaml_metadata(
    raw: &str,
    format: MarkdownFrontmatterFormat,
    source_span: MarkdownSourceSpan,
) -> Result<MarkdownMetadataValue, MarkdownExtractionError> {
    let value = serde_yaml::from_str::<serde_yaml::Value>(raw).map_err(|error| {
        frontmatter_error(
            format,
            match format {
                MarkdownFrontmatterFormat::Json => "JSON frontmatter could not be parsed.",
                MarkdownFrontmatterFormat::Yaml => "YAML frontmatter could not be parsed.",
                MarkdownFrontmatterFormat::Toml => "TOML frontmatter could not be parsed.",
            },
            source_span,
            Some(error.to_string()),
        )
    })?;
    Ok(yaml_to_metadata(value))
}

fn yaml_to_metadata(value: serde_yaml::Value) -> MarkdownMetadataValue {
    match value {
        serde_yaml::Value::Null => MarkdownMetadataValue::Null,
        serde_yaml::Value::Bool(value) => MarkdownMetadataValue::Bool(value),
        serde_yaml::Value::Number(value) => MarkdownMetadataValue::Number(value.to_string()),
        serde_yaml::Value::String(value) => MarkdownMetadataValue::String(value),
        serde_yaml::Value::Sequence(values) => {
            MarkdownMetadataValue::Sequence(values.into_iter().map(yaml_to_metadata).collect())
        }
        serde_yaml::Value::Mapping(mapping) => MarkdownMetadataValue::Mapping(
            mapping
                .into_iter()
                .filter_map(|(key, value)| match key {
                    serde_yaml::Value::String(key) => Some((key, yaml_to_metadata(value))),
                    _ => None,
                })
                .collect(),
        ),
        serde_yaml::Value::Tagged(tagged) => yaml_to_metadata(tagged.value),
    }
}

fn parse_toml_metadata(
    raw: &str,
    source_span: MarkdownSourceSpan,
) -> Result<MarkdownMetadataValue, MarkdownExtractionError> {
    let mut mapping = BTreeMap::new();
    for (line_index, raw_line) in raw.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(frontmatter_error(
                MarkdownFrontmatterFormat::Toml,
                "TOML frontmatter could not be parsed.",
                source_span,
                Some(format!("line {} is not a key-value pair", line_index + 1)),
            ));
        };
        mapping.insert(key.trim().to_owned(), parse_toml_value(value.trim()));
    }
    Ok(MarkdownMetadataValue::Mapping(mapping))
}

fn parse_toml_value(value: &str) -> MarkdownMetadataValue {
    if value == "true" {
        return MarkdownMetadataValue::Bool(true);
    }
    if value == "false" {
        return MarkdownMetadataValue::Bool(false);
    }
    if let Some(string) = unquote(value) {
        return MarkdownMetadataValue::String(string);
    }
    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        return MarkdownMetadataValue::Sequence(
            inner
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(parse_toml_value)
                .collect(),
        );
    }
    MarkdownMetadataValue::Number(value.to_owned())
}

fn unquote(value: &str) -> Option<String> {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return Some(value[1..value.len() - 1].to_owned());
    }
    None
}

fn frontmatter_error(
    format: MarkdownFrontmatterFormat,
    message: &str,
    source_span: MarkdownSourceSpan,
    parser_message: Option<String>,
) -> MarkdownExtractionError {
    let mut details = BTreeMap::from([("format".to_owned(), format_name(format).to_owned())]);
    if let Some(parser_message) = parser_message {
        details.insert("parser-message".to_owned(), parser_message);
    }
    MarkdownExtractionError {
        kind: "malformed_frontmatter".to_owned(),
        message: message.to_owned(),
        source_span,
        details,
    }
}

const fn format_name(format: MarkdownFrontmatterFormat) -> &'static str {
    match format {
        MarkdownFrontmatterFormat::Yaml => "yaml",
        MarkdownFrontmatterFormat::Toml => "toml",
        MarkdownFrontmatterFormat::Json => "json",
    }
}

fn extract_headings(
    body: &str,
    body_start: usize,
    diagnostics: &mut Vec<MarkdownExtractionDiagnostic>,
) -> Vec<MarkdownHeading> {
    let mut headings = Vec::new();
    let mut stack: Vec<(u8, String)> = Vec::new();
    let mut anchors = BTreeSet::new();
    let mut in_fence = false;

    for (line_start, line, line_end) in markdown_lines(body, body_start) {
        if toggles_fence(line) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let Some((level, text)) = parse_atx_heading(line) else {
            continue;
        };
        let anchor = normalize_anchor(text);
        if !anchors.insert(anchor.clone()) {
            diagnostics.push(MarkdownExtractionDiagnostic {
                kind: "duplicate_anchor".to_owned(),
                message: format!("duplicate heading anchor `{anchor}`"),
                source_span: Some(MarkdownSourceSpan::new(line_start, line_end)),
            });
        }
        while stack.last().is_some_and(|(stack_level, _)| *stack_level >= level) {
            stack.pop();
        }
        stack.push((level, text.to_owned()));
        headings.push(MarkdownHeading {
            level,
            text: text.to_owned(),
            anchor,
            ordinal: headings.len(),
            path: stack.iter().map(|(_, value)| value.clone()).collect(),
            source_span: MarkdownSourceSpan::new(line_start, line_end),
        });
    }
    headings
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

fn extract_links(
    body: &str,
    body_start: usize,
    diagnostics: &mut Vec<MarkdownExtractionDiagnostic>,
) -> Vec<MarkdownLink> {
    let references = collect_reference_definitions(body);
    let mut links = Vec::new();
    let mut in_fence = false;

    for (line_start, line, _) in markdown_lines(body, body_start) {
        if toggles_fence(line) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || is_reference_definition(line) {
            continue;
        }
        extract_wikilinks(line, line_start, &mut links);
        extract_inline_links(line, line_start, &mut links);
        extract_autolinks(line, line_start, &mut links);
        extract_reference_links(line, line_start, &references, &mut links, diagnostics);
    }
    links.sort_by_key(|link| link.source_span.start);
    links
}

fn collect_reference_definitions(body: &str) -> BTreeMap<String, String> {
    markdown_lines(body, 0).filter_map(|(_, line, _)| parse_reference_definition(line)).collect()
}

fn parse_reference_definition(line: &str) -> Option<ReferenceDefinition> {
    let trimmed = line.trim_start();
    let closing = trimmed.find("]:")?;
    if !trimmed.starts_with('[') {
        return None;
    }
    let label = trimmed[1..closing].trim().to_ascii_lowercase();
    let target = trimmed[closing + 2..].split_whitespace().next()?.to_owned();
    Some((label, target))
}

fn is_reference_definition(line: &str) -> bool {
    parse_reference_definition(line).is_some()
}

fn extract_wikilinks(line: &str, line_start: usize, links: &mut Vec<MarkdownLink>) {
    let mut search_start = 0;
    while let Some(relative_start) = line[search_start..].find("[[") {
        let start = search_start + relative_start;
        let Some(relative_end) = line[start + 2..].find("]]") else {
            break;
        };
        let end = start + 2 + relative_end + 2;
        let raw = &line[start..end];
        let inner = &line[start + 2..end - 2];
        let (target_part, label) =
            inner.split_once('|').map_or((inner, None), |(target, label)| (target, Some(label)));
        let (target, heading) = split_heading(target_part);
        links.push(MarkdownLink {
            kind: MarkdownLinkKind::Wikilink,
            raw: raw.to_owned(),
            target: target.to_owned(),
            label: label.map(ToOwned::to_owned),
            heading: heading.map(ToOwned::to_owned),
            source_span: MarkdownSourceSpan::new(line_start + start, line_start + end),
        });
        search_start = end;
    }
}

fn extract_inline_links(line: &str, line_start: usize, links: &mut Vec<MarkdownLink>) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'[' || (index > 0 && bytes[index - 1] == b'!') {
            index += 1;
            continue;
        }
        let Some(label_end_relative) = line[index + 1..].find(']') else {
            break;
        };
        let label_end = index + 1 + label_end_relative;
        if line[label_end + 1..].starts_with('(')
            && let Some(target_end_relative) = line[label_end + 2..].find(')')
        {
            let target_end = label_end + 2 + target_end_relative;
            let raw_end = target_end + 1;
            let raw = &line[index..raw_end];
            let label = &line[index + 1..label_end];
            let target_text = &line[label_end + 2..target_end];
            let (target, heading) = split_heading(target_text);
            links.push(MarkdownLink {
                kind: MarkdownLinkKind::Inline,
                raw: raw.to_owned(),
                target: target.to_owned(),
                label: Some(label.to_owned()),
                heading: heading.map(ToOwned::to_owned),
                source_span: MarkdownSourceSpan::new(line_start + index, line_start + raw_end),
            });
            index = raw_end;
            continue;
        }
        index = label_end + 1;
    }
}

fn extract_autolinks(line: &str, line_start: usize, links: &mut Vec<MarkdownLink>) {
    for scheme in ["<http://", "<https://", "<mailto:"] {
        let mut search_start = 0;
        while let Some(relative_start) = line[search_start..].find(scheme) {
            let start = search_start + relative_start;
            let Some(relative_end) = line[start..].find('>') else {
                break;
            };
            let end = start + relative_end + 1;
            let target = &line[start + 1..end - 1];
            links.push(MarkdownLink {
                kind: MarkdownLinkKind::Autolink,
                raw: line[start..end].to_owned(),
                target: target.to_owned(),
                label: None,
                heading: None,
                source_span: MarkdownSourceSpan::new(line_start + start, line_start + end),
            });
            search_start = end;
        }
    }
}

fn extract_reference_links(
    line: &str,
    line_start: usize,
    references: &BTreeMap<String, String>,
    links: &mut Vec<MarkdownLink>,
    diagnostics: &mut Vec<MarkdownExtractionDiagnostic>,
) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'[' || (index > 0 && bytes[index - 1] == b'!') {
            index += 1;
            continue;
        }
        let Some(label_end_relative) = line[index + 1..].find(']') else {
            break;
        };
        let label_end = index + 1 + label_end_relative;
        if !line[label_end + 1..].starts_with('[') {
            index = label_end + 1;
            continue;
        }
        let Some(reference_end_relative) = line[label_end + 2..].find(']') else {
            break;
        };
        let reference_end = label_end + 2 + reference_end_relative;
        let raw_end = reference_end + 1;
        let label = &line[index + 1..label_end];
        let reference = &line[label_end + 2..reference_end];
        let lookup = if reference.is_empty() { label } else { reference }.to_ascii_lowercase();
        if let Some(target_text) = references.get(&lookup) {
            let (target, heading) = split_heading(target_text);
            links.push(MarkdownLink {
                kind: MarkdownLinkKind::Reference,
                raw: line[index..raw_end].to_owned(),
                target: target.to_owned(),
                label: Some(label.to_owned()),
                heading: heading.map(ToOwned::to_owned),
                source_span: MarkdownSourceSpan::new(line_start + index, line_start + raw_end),
            });
        } else {
            diagnostics.push(MarkdownExtractionDiagnostic {
                kind: "unresolved_reference_link".to_owned(),
                message: format!("reference-style link `{lookup}` could not be resolved"),
                source_span: Some(MarkdownSourceSpan::new(
                    line_start + index,
                    line_start + raw_end,
                )),
            });
        }
        index = raw_end;
    }
}

fn split_heading(target: &str) -> SplitLinkTarget<'_> {
    target.split_once('#').map_or((target, None), |(target, heading)| (target, Some(heading)))
}

fn normalize_anchor(text: &str) -> String {
    let mut anchor = String::new();
    let mut previous_dash = false;
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            anchor.push(character);
            previous_dash = false;
        } else if (character.is_whitespace() || character == '-')
            && !previous_dash
            && !anchor.is_empty()
        {
            anchor.push('-');
            previous_dash = true;
        }
    }
    anchor.trim_matches('-').to_owned()
}

fn markdown_lines(source: &str, base_offset: usize) -> impl Iterator<Item = (usize, &str, usize)> {
    let mut offset = base_offset;
    source.split_inclusive('\n').map(move |segment| {
        let start = offset;
        offset += segment.len();
        let line = segment.trim_end_matches(['\r', '\n']);
        (start, line, start + line.len())
    })
}

fn toggles_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn trailing_line_break_len(source: &str) -> usize {
    if source.starts_with("\r\n") { 2 } else { usize::from(source.starts_with('\n')) }
}

struct GovernedIdentity {
    document_type: String,
    code: String,
    slug: String,
}

fn parse_governed_identity(stem: &str) -> Option<GovernedIdentity> {
    let parts = stem.split('-').collect::<Vec<_>>();
    let code_index =
        parts.iter().position(|part| part.chars().all(|character| character.is_ascii_digit()))?;
    if code_index == 0 || code_index + 1 >= parts.len() {
        return None;
    }
    Some(GovernedIdentity {
        document_type: parts[..code_index].join("-"),
        code: parts[code_index].to_owned(),
        slug: parts[code_index + 1..].join("-"),
    })
}

fn io_error_message(error: &IoError) -> String {
    error.to_string()
}

#[cfg(test)]
#[path = "extraction_test.rs"]
mod tests;
