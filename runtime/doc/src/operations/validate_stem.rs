//! Stem index building and bare-stem detection for the validate operation.

use crate::internal::vector_yaml::validate_vector_yaml_schema_content;
use crate::types::DocumentTypesConfig;
use std::collections::HashSet;
use std::path::Path;

pub type ProtectedRange = (usize, usize);

#[derive(Debug, Clone, Default)]
pub struct DocumentStemIndex {
    pub(crate) stems: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BareStemMatch {
    pub(crate) stem: String,
    pub(crate) replacement: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl BareStemMatch {
    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    pub const fn start(&self) -> usize {
        self.start
    }

    pub const fn end(&self) -> usize {
        self.end
    }
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

pub fn build_document_stem_index(
    root_dir: &Path,
    doc_config: &DocumentTypesConfig,
) -> DocumentStemIndex {
    let mut stems = HashSet::new();

    for doc_type in doc_config.document_types.keys() {
        for path in super::governed_markdown_files(root_dir, doc_type) {
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
                for path in super::governed_markdown_files(&package_root, doc_type) {
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

pub fn protected_ranges_for_line(line: &str) -> Vec<ProtectedRange> {
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

fn document_body_start(content: &str) -> usize {
    content
        .find("---")
        .and_then(|start| {
            let rest = &content[start + 3..];
            rest.find("---").map(|end| start + 3 + end + 3)
        })
        .unwrap_or(0)
}

pub fn find_bare_governed_stems(
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

#[cfg(test)]
#[path = "validate_stem_test.rs"]
mod tests;
