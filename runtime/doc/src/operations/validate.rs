//! Plugin operation for validating documentation layout.

use regex::Regex;
use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;
use thiserror::Error;

use crate::internal::slug::validate_slug;
use crate::internal::vector_yaml::{
    is_kebab_case_identifier, relative_display_path, validate_vector_yaml_schema_content,
};
use crate::types::{DocumentTypeConfig, DocumentTypesConfig, load_document_types_config};

const MINIMUM_FRONTMATTER_FIELDS: &[&str] =
    &["id", "type", "code", "slug", "title", "description", "created", "tags"];
static PLACEHOLDER_VARIABLE_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"#\{([^}]+)\}"));

pub(crate) type ValidationResult = (Vec<ValidationError>, Vec<String>);
pub(crate) type ScanResult = (Vec<ValidationError>, Vec<String>);

#[derive(Debug, Clone, Default)]
pub(super) struct DocumentStemIndex {
    stems: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BareStemMatch {
    stem: String,
    replacement: String,
    start: usize,
    end: usize,
}

impl BareStemMatch {
    pub(super) fn replacement(&self) -> &str {
        &self.replacement
    }

    pub(super) const fn start(&self) -> usize {
        self.start
    }

    pub(super) const fn end(&self) -> usize {
        self.end
    }
}

type ProtectedRange = (usize, usize);

#[derive(Debug, Error)]
pub(crate) enum Utf8ValidationError {
    #[error("File contains UTF-8 BOM")]
    Utf8Bom,
    #[error("File contains CRLF line endings; governed Markdown files must use LF line endings")]
    CrlfLineEndings,
    #[error("Cannot read file as UTF-8: {source}")]
    InvalidUtf8 {
        #[source]
        source: std::str::Utf8Error,
    },
    #[error("Cannot read file bytes: {source}")]
    Io {
        #[source]
        source: std::io::Error,
    },
}

/// Input for the validate operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ValidateInput {
    /// The root directory of the project to validate.
    pub root_dir: IoPath,
    /// When true, attempt to fix validation issues rather than just reporting them.
    pub fix: bool,
}

impl Default for ValidateInput {
    fn default() -> Self {
        Self { root_dir: IoPath::new(std::path::Path::new(".")), fix: false }
    }
}

/// Output for the validate operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ValidateOutput {
    /// Whether the validation passed.
    pub valid: bool,
    /// List of validation errors found.
    pub errors: Vec<ValidationError>,
    /// List of warnings (non-critical issues).
    pub warnings: Vec<String>,
    /// List of fixes applied when fix mode is enabled.
    pub fixes: Vec<FixResult>,
}

/// A single validation error.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Path to the file where the error occurred.
    pub path: String,
    /// Description of the error.
    pub error: String,
}

/// Result of a fix applied during validate --fix.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct FixResult {
    /// Path to the file that was fixed.
    pub path: String,
    /// Description of what was fixed.
    pub fix_type: String,
    /// Detail about the fix.
    pub detail: String,
}

pub(crate) fn parse_frontmatter(
    content: &str,
) -> Option<std::collections::HashMap<String, String>> {
    let start = content.find("---")? + 3;
    let rest = &content[start..];
    let end = rest.find("---")?;
    let frontmatter_block = &rest[..end];
    let mut fields = std::collections::HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();
    let mut array_items = Vec::new();
    let trim_quotes = |s: &str| {
        let s = s.trim();
        if s.starts_with('"') && s.ends_with('"') {
            s[1..s.len() - 1].to_string()
        } else {
            s.to_string()
        }
    };

    for line in frontmatter_block.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('-') && trimmed.len() > 1 {
            let item = trimmed[1..].trim();
            array_items.push(trim_quotes(item));
            continue;
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            if let Some(prev_key) = current_key.take()
                && (!array_items.is_empty() || !current_value.trim().is_empty())
            {
                let final_value = if array_items.is_empty() {
                    trim_quotes(&current_value)
                } else {
                    array_items.join(", ")
                };
                fields.insert(prev_key, final_value);
                array_items.clear();
            }
            current_key = Some(key.trim().to_string());
            current_value = value.trim().to_string();
        } else if !trimmed.is_empty() && current_key.is_some() {
            current_value.push(' ');
            current_value.push_str(trimmed);
        }
    }

    if let Some(key) = current_key.take() {
        let final_value = if array_items.is_empty() {
            trim_quotes(&current_value)
        } else {
            array_items.join(", ")
        };
        fields.insert(key, final_value);
    }

    Some(fields)
}

pub(crate) fn check_utf8_without_bom(path: &Path) -> Result<(), Utf8ValidationError> {
    let content = std::fs::read(path).map_err(|source| Utf8ValidationError::Io { source })?;
    if content.len() >= 3 && content[0] == 0xEF && content[1] == 0xBB && content[2] == 0xBF {
        return Err(Utf8ValidationError::Utf8Bom);
    }
    std::str::from_utf8(&content).map_err(|source| Utf8ValidationError::InvalidUtf8 { source })?;
    if content.windows(2).any(|window| window == b"\r\n") {
        return Err(Utf8ValidationError::CrlfLineEndings);
    }
    Ok(())
}

fn validate_frontmatter_fields(
    path_str: &str,
    frontmatter: &std::collections::HashMap<String, String>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for field in MINIMUM_FRONTMATTER_FIELDS {
        if !frontmatter.contains_key(*field) {
            errors.push(ValidationError {
                path: path_str.to_string(),
                error: format!("Missing required frontmatter field: {field}"),
            });
        }
    }
    errors
}

fn validate_status_based(
    path: &Path,
    path_str: &str,
    frontmatter: &std::collections::HashMap<String, String>,
    type_config: &DocumentTypeConfig,
    status_folders: &[&str],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if !frontmatter.contains_key("status") {
        errors.push(ValidationError {
            path: path_str.to_string(),
            error: "Missing required frontmatter field: status".to_string(),
        });
    } else if let Some(status) = frontmatter.get("status")
        && !type_config.statuses.contains(status)
    {
        errors.push(ValidationError {
            path: path_str.to_string(),
            error: format!("Invalid status '{status}'. Allowed: {:?}", type_config.statuses),
        });
    }

    if let Some(parent) = path.parent() {
        let folder_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if status_folders.contains(&folder_name)
            && let Some(status) = frontmatter.get("status")
            && folder_name != *status
        {
            errors.push(ValidationError {
                path: path_str.to_string(),
                error: format!(
                    "File is in folder '{folder_name}' but frontmatter status is '{status}'"
                ),
            });
        }
    }
    errors
}

fn validate_category_based(
    path: &Path,
    path_str: &str,
    frontmatter: &std::collections::HashMap<String, String>,
    category_folders: &[String],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if !frontmatter.contains_key("category") {
        errors.push(ValidationError {
            path: path_str.to_string(),
            error: "Missing required frontmatter field: category".to_string(),
        });
    }

    if let Some(parent) = path.parent() {
        let folder_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !category_folders.contains(&folder_name.to_string())
            && let Some(category) = frontmatter.get("category")
            && folder_name != *category
        {
            errors.push(ValidationError {
                path: path_str.to_string(),
                error: format!(
                    "File is in folder '{folder_name}' but frontmatter category is '{category}'"
                ),
            });
        }
    }
    errors
}

fn validate_directory_based(path: &Path, path_str: &str, doc_type: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if let Some(parent) = path.parent() {
        let folder_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if folder_name != doc_type {
            errors.push(ValidationError {
                path: path_str.to_string(),
                error: format!(
                    "File for directory-based layout must be directly under 'doc/{doc_type}' (found in '{folder_name}')"
                ),
            });
        }
    }
    errors
}

fn validate_filename_pattern(
    path: &Path,
    path_str: &str,
    doc_type: &str,
    frontmatter: &std::collections::HashMap<String, String>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let code = frontmatter.get("code").cloned().unwrap_or_default();
        let slug = frontmatter.get("slug").cloned().unwrap_or_default();
        let expected_pattern = format!("{doc_type}-{code}-{slug}.md");
        if file_name != expected_pattern {
            errors.push(ValidationError {
                path: path_str.to_string(),
                error: format!(
                    "Filename '{file_name}' does not match pattern '{{doc_type}}-{{code}}-{{slug}}.md' (expected: '{expected_pattern}')"
                ),
            });
        }
    }
    errors
}

fn validate_wikilinks(path_str: &str, content: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for line in content.lines() {
        if let Some(start_idx) = line.find("[[")
            && let Some(end_idx) = line.find("]]")
            && end_idx > start_idx + 2
        {
            let link_content = &line[start_idx + 2..end_idx];
            let has_md_ext = std::path::Path::new(link_content)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
            if has_md_ext {
                errors.push(ValidationError {
                    path: path_str.to_string(),
                    error: format!("Wikilink '{link_content}' should not include .md extension"),
                });
            }
        }
    }
    errors
}

fn document_body_start(content: &str) -> usize {
    content
        .find("---")
        .and_then(|start| {
            let rest = &content[start + 3..];
            rest.find("---").map(|end| start + 3 + end + 3)
        })
        .unwrap_or(0)
}

fn stem_from_path(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(std::string::ToString::to_string)
}

fn load_document_types_config_from_path(path: &Path) -> Option<DocumentTypesConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    let display_path = path.to_string_lossy().replace('\\', "/");
    validate_vector_yaml_schema_content(&display_path, &content).ok()?;
    serde_yaml::from_str(&content).ok()
}

pub(super) fn build_document_stem_index(
    root_dir: &Path,
    doc_config: &DocumentTypesConfig,
) -> DocumentStemIndex {
    let mut stems = HashSet::new();

    for doc_type in doc_config.document_types.keys() {
        for path in governed_markdown_files(root_dir, doc_type) {
            if let Some(stem) = stem_from_path(&path) {
                stems.insert(stem);
            }
        }
    }

    let packages_dir = root_dir.join(".vector-database").join("packages");
    if packages_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(packages_dir)
    {
        for entry in entries.filter_map(std::result::Result::ok) {
            let package_root = entry.path();
            if !package_root.is_dir() {
                continue;
            }
            let Some(package_name) = package_root.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let package_config_path = package_root.join(".vector").join("document-types.yaml");
            let Some(package_config) = load_document_types_config_from_path(&package_config_path)
            else {
                continue;
            };
            for doc_type in package_config.document_types.keys() {
                for path in governed_markdown_files(&package_root, doc_type) {
                    if let Some(stem) = stem_from_path(&path) {
                        stems.insert(format!("{package_name}/{stem}"));
                    }
                }
            }
        }
    }

    let mut stems: Vec<_> = stems.into_iter().collect();
    stems.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    DocumentStemIndex { stems }
}

const fn is_stem_boundary_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '/')
}

fn starts_filename_extension(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('.') else {
        return false;
    };
    let extension_len =
        rest.chars().take_while(char::is_ascii_alphanumeric).map(char::len_utf8).sum::<usize>();
    extension_len > 0
}

fn has_stem_boundary(line: &str, start: usize, end: usize) -> bool {
    let before_ok = line[..start].chars().next_back().is_none_or(|ch| !is_stem_boundary_char(ch));
    let after = &line[end..];
    let after_ok = after.chars().next().is_none_or(|ch| !is_stem_boundary_char(ch))
        && !starts_filename_extension(after);
    before_ok && after_ok
}

fn protected_ranges_for_line(line: &str) -> Vec<ProtectedRange> {
    let mut ranges = Vec::new();
    let mut search_start = 0;
    while let Some(relative_start) = line[search_start..].find("[[") {
        let start = search_start + relative_start;
        let Some(relative_end) = line[start + 2..].find("]]") else {
            break;
        };
        let end = start + 2 + relative_end + 2;
        ranges.push((start, end));
        search_start = end;
    }

    search_start = 0;
    while let Some(relative_start) = line[search_start..].find('`') {
        let start = search_start + relative_start;
        let Some(relative_end) = line[start + 1..].find('`') else {
            ranges.push((start, line.len()));
            break;
        };
        let end = start + 1 + relative_end + 1;
        ranges.push((start, end));
        search_start = end;
    }

    for scheme_marker in ["://"] {
        search_start = 0;
        while let Some(relative_marker) = line[search_start..].find(scheme_marker) {
            let marker = search_start + relative_marker;
            let start = line[..marker]
                .rfind(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')))
                .map_or(0, |idx| idx + 1);
            let end = line[marker..]
                .find(|ch: char| ch.is_whitespace() || matches!(ch, ')' | ']'))
                .map_or(line.len(), |idx| marker + idx);
            ranges.push((start, end));
            search_start = end;
        }
    }

    ranges.sort_unstable();
    let mut merged: Vec<ProtectedRange> = Vec::new();
    for (start, end) in ranges {
        if let Some((_, merged_end)) = merged.last_mut()
            && start <= *merged_end
        {
            *merged_end = (*merged_end).max(end);
            continue;
        }
        merged.push((start, end));
    }
    merged
}

fn protected_end_at(position: usize, ranges: &[ProtectedRange]) -> Option<usize> {
    ranges.iter().find(|(start, end)| position >= *start && position < *end).map(|(_, end)| *end)
}

fn is_fence_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

pub(super) fn find_bare_governed_stems(
    content: &str,
    stem_index: &DocumentStemIndex,
) -> Vec<BareStemMatch> {
    let mut bare_matches = Vec::new();
    let mut body_offset = document_body_start(content);
    let mut in_fenced_code = false;

    for line in content[body_offset..].split_inclusive('\n') {
        if is_fence_line(line) {
            in_fenced_code = !in_fenced_code;
            body_offset += line.len();
            continue;
        }

        if !in_fenced_code {
            let protected_ranges = protected_ranges_for_line(line);
            let mut line_offset = 0;
            while line_offset < line.len() {
                if let Some(protected_end) = protected_end_at(line_offset, &protected_ranges) {
                    line_offset = protected_end;
                    continue;
                }

                let mut stem_hit = None;
                for stem in &stem_index.stems {
                    let end = line_offset + stem.len();
                    if end <= line.len()
                        && line[line_offset..].starts_with(stem)
                        && has_stem_boundary(line, line_offset, end)
                    {
                        stem_hit = Some((stem, end));
                        break;
                    }
                }

                if let Some((stem, end)) = stem_hit {
                    bare_matches.push(BareStemMatch {
                        stem: stem.clone(),
                        replacement: format!("[[{stem}]]"),
                        start: body_offset + line_offset,
                        end: body_offset + end,
                    });
                    line_offset = end;
                } else if let Some(ch) = line[line_offset..].chars().next() {
                    line_offset += ch.len_utf8();
                } else {
                    break;
                }
            }
        }

        body_offset += line.len();
    }

    bare_matches
}

fn validate_governed_document_references(
    path_str: &str,
    content: &str,
    stem_index: &DocumentStemIndex,
) -> Vec<ValidationError> {
    find_bare_governed_stems(content, stem_index)
        .into_iter()
        .map(|bare_match| ValidationError {
            path: path_str.to_string(),
            error: format!(
                "Bare governed document stem '{}' should be a wikilink; expected '{}'",
                bare_match.stem, bare_match.replacement
            ),
        })
        .collect()
}

fn validate_placeholder_variables(path_str: &str, content: &str) -> Vec<ValidationError> {
    let Ok(placeholder_pattern) = &*PLACEHOLDER_VARIABLE_REGEX else {
        return vec![ValidationError {
            path: path_str.to_string(),
            error: "Internal placeholder validation regex failed to compile".to_string(),
        }];
    };
    let mut errors = Vec::new();

    for captures in placeholder_pattern.captures_iter(content) {
        let Some(variable_name) = captures.get(1).map(|capture| capture.as_str()) else {
            continue;
        };

        if !is_kebab_case_identifier(variable_name) {
            errors.push(ValidationError {
                path: path_str.to_string(),
                error: format!(
                    "Invalid substitution variable '#{{{variable_name}}}'; variable names must be kebab-case"
                ),
            });
        }
    }

    errors
}

fn validate_slug_field(
    path_str: &str,
    frontmatter: &std::collections::HashMap<String, String>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if let Some(slug) = frontmatter.get("slug")
        && let Err(e) = validate_slug(slug)
    {
        errors.push(ValidationError {
            path: path_str.to_string(),
            error: format!("Invalid slug: {e}"),
        });
    }
    errors
}

fn validate_governed_file(
    path: &Path,
    _doc_config: &DocumentTypesConfig,
    doc_type: &str,
    type_config: &DocumentTypeConfig,
    status_folders: &[&str],
    category_folders: &[String],
    stem_index: &DocumentStemIndex,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let path_str = path.to_string_lossy().to_string();

    if let Err(e) = check_utf8_without_bom(path) {
        errors.push(ValidationError { path: path_str, error: e.to_string() });
        return errors;
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(ValidationError {
                path: path_str,
                error: format!("Cannot read file as UTF-8: {e}"),
            });
            return errors;
        }
    };

    let Some(frontmatter) = parse_frontmatter(&content) else {
        errors.push(ValidationError {
            path: path_str,
            error: "Missing or malformed frontmatter".to_string(),
        });
        return errors;
    };

    // Template files carry placeholder values in every frontmatter field.
    // Detection is by the doc_type folder: files under doc/template/ are templates.
    // Skip all field-level validation; only structural checks (BOM, UTF-8, wikilinks) apply.
    if doc_type == "template" {
        errors.extend(validate_wikilinks(&path_str, &content));
        errors.extend(validate_governed_document_references(&path_str, &content, stem_index));
        errors.extend(validate_placeholder_variables(&path_str, &content));
        return errors;
    }

    errors.extend(validate_frontmatter_fields(&path_str, &frontmatter));
    errors.extend(validate_slug_field(&path_str, &frontmatter));

    if type_config.is_status_based() {
        errors.extend(validate_status_based(
            path,
            &path_str,
            &frontmatter,
            type_config,
            status_folders,
        ));
    }

    if type_config.is_category_based() {
        errors.extend(validate_category_based(path, &path_str, &frontmatter, category_folders));
    }

    if type_config.is_directory_based() {
        errors.extend(validate_directory_based(path, &path_str, doc_type));
    }

    errors.extend(validate_filename_pattern(path, &path_str, doc_type, &frontmatter));
    errors.extend(validate_wikilinks(&path_str, &content));
    errors.extend(validate_governed_document_references(&path_str, &content, stem_index));
    errors.extend(validate_placeholder_variables(&path_str, &content));

    errors
}

fn validate_config_create_forms(doc_config: &DocumentTypesConfig) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if doc_config.doc_type.create_document_type_form.is_empty() {
        errors.push(ValidationError {
            path: ".vector/document-types.yaml".to_string(),
            error: "Missing required field: doc-type.create-document-type-form".to_string(),
        });
    }

    for (doc_type, type_config) in &doc_config.document_types {
        if type_config.create_document_form.is_empty() {
            errors.push(ValidationError {
                path: ".vector/document-types.yaml".to_string(),
                error: format!(
                    "Missing required field: document-types.{doc_type}.create-document-form"
                ),
            });
        }
    }

    errors
}

pub(super) fn scan_governed_files(
    root_dir: &Path,
    doc_config: &DocumentTypesConfig,
) -> ValidationResult {
    let mut errors = Vec::new();
    let warnings = Vec::new();

    let status_folders = status_folder_names(doc_config);
    let category_folders = category_folder_names(root_dir);
    let stem_index = build_document_stem_index(root_dir, doc_config);

    for doc_type in doc_config.document_types.keys() {
        let Some(type_config) = doc_config.document_types.get(doc_type) else {
            continue;
        };
        for path in governed_markdown_files(root_dir, doc_type) {
            let file_errors = validate_governed_file(
                &path,
                doc_config,
                doc_type,
                type_config,
                &status_folders,
                &category_folders,
                &stem_index,
            );
            errors.extend(file_errors);
        }
    }

    (errors, warnings)
}

pub(crate) fn status_folder_names(doc_config: &DocumentTypesConfig) -> Vec<&str> {
    doc_config
        .document_types
        .values()
        .filter(|config| config.is_status_based())
        .flat_map(|config| config.statuses.iter().map(std::string::String::as_str))
        .collect()
}

pub(crate) fn category_folder_names(root_dir: &Path) -> Vec<String> {
    let mut folders = Vec::new();
    let doc_path = root_dir.join("doc");
    if !doc_path.exists() {
        return folders;
    }

    for entry in walkdir::WalkDir::new(&doc_path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if entry.file_type().is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            folders.push(name.to_string());
        }
    }

    folders
}

pub(crate) fn governed_markdown_files(root_dir: &Path, doc_type: &str) -> Vec<std::path::PathBuf> {
    let search_base = root_dir.join("doc").join(doc_type);
    if !search_base.exists() {
        return Vec::new();
    }

    walkdir::WalkDir::new(search_base)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.into_path();
            let extension = path.extension().and_then(|ext| ext.to_str());
            match extension {
                Some("md") | None => Some(path),
                Some(_) => None,
            }
        })
        .collect()
}

fn validate_vector_yaml_files(root_dir: &Path) -> Vec<ValidationError> {
    let vector_root = root_dir.join(".vector");
    if !vector_root.exists() {
        return Vec::new();
    }

    let mut errors = Vec::new();
    for entry in walkdir::WalkDir::new(&vector_root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("yaml")) {
            continue;
        }

        let path_str = relative_display_path(root_dir, path);
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                errors.push(ValidationError {
                    path: path_str,
                    error: format!("Cannot read YAML file as UTF-8: {error}"),
                });
                continue;
            }
        };

        if let Err(field_errors) = validate_vector_yaml_schema_content(&path_str, &content) {
            errors.extend(field_errors.into_iter().map(|field_error| ValidationError {
                path: field_error.path().to_string(),
                error: field_error.message(),
            }));
        }
    }

    errors
}

/// Operation to validate documentation layout.
async fn validate(
    input: ValidateInput,
    output: &mut impl PluginSender<ValidateOutput>,
) -> RuntimeResult<()> {
    let config = match load_document_types_config(&input.root_dir).await {
        Ok(config) => config,
        Err(error) => {
            output
                .send(ValidateOutput {
                    valid: false,
                    errors: vec![ValidationError {
                        path: ".vector/document-types.yaml".to_string(),
                        error: error.to_string(),
                    }],
                    warnings: Vec::new(),
                    fixes: Vec::new(),
                })
                .await?;
            return Ok(());
        }
    };

    let root_path = input.root_dir.as_path();
    let config_errors = validate_config_create_forms(&config);
    let (errors, warnings, fixes) = if input.fix {
        let ((errs, warns), fix_results) =
            super::validate_fix::scan_and_fix_governed_files(root_path, &config);
        (errs, warns, fix_results)
    } else {
        let (errs, warns) = scan_governed_files(root_path, &config);
        (errs, warns, Vec::new())
    };

    let mut all_errors = config_errors;
    all_errors.extend(validate_vector_yaml_files(root_path));
    all_errors.extend(errors);

    output
        .send(ValidateOutput { valid: all_errors.is_empty(), errors: all_errors, warnings, fixes })
        .await?;

    Ok(())
}

declare_plugin_operations! {
    /// Operation to validate documentation layout against document-types.yaml.
    ValidateOp => validate(ValidateInput, ValidateOutput)
}

impl ValidateInput {
    /// Construct a `ValidateInput` with explicit fields.
    #[must_use]
    pub const fn new(root_dir: IoPath, fix: bool) -> Self {
        Self { root_dir, fix }
    }
}

impl ValidateOp {
    /// Construct a new `ValidateOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for ValidateOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "validate_test.rs"]
mod tests;
