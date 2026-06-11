//! Fix logic for the validate operation.

use std::borrow::Cow;
use std::path::Path;
use std::sync::LazyLock;

use crate::types::{DocumentTypeConfig, DocumentTypesConfig};

use super::validate::{
    DocumentStemIndex, FixResult, ScanResult, build_document_stem_index, find_bare_governed_stems,
    governed_markdown_files, parse_frontmatter, scan_governed_files, status_folder_names,
};

pub(super) type FixScanResult = (ScanResult, Vec<FixResult>);
type WikilinkFix = (String, FixResult);
type GovernedReferenceFix = (String, FixResult);
type HeadingFix = (String, FixResult);
static WIKILINK_EXTENSION_REGEX: LazyLock<Result<regex::Regex, regex::Error>> =
    LazyLock::new(|| regex::Regex::new(r"\[\[([^\]]+)\.md\]\]"));

pub(super) fn fix_bom_if_present(path: &Path) -> Option<FixResult> {
    let Ok(mut content) = std::fs::read(path) else {
        return None;
    };
    if !content.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return None;
    }
    if std::str::from_utf8(&content[3..]).is_err() {
        return None;
    }
    content = content[3..].to_vec();
    if std::fs::write(path, &content).is_ok() {
        Some(FixResult {
            path: path.to_string_lossy().to_string(),
            fix_type: "remove_bom".to_string(),
            detail: "Removed UTF-8 BOM".to_string(),
        })
    } else {
        None
    }
}

pub(super) fn fix_crlf_line_endings_if_present(path: &Path) -> Option<FixResult> {
    let Ok(content) = std::fs::read(path) else {
        return None;
    };
    if !content.windows(2).any(|window| window == b"\r\n") {
        return None;
    }
    if std::str::from_utf8(&content).is_err() {
        return None;
    }

    let mut normalized = Vec::with_capacity(content.len());
    let mut index = 0;
    while index < content.len() {
        if content.get(index..index + 2) == Some(b"\r\n") {
            normalized.push(b'\n');
            index += 2;
        } else {
            normalized.push(content[index]);
            index += 1;
        }
    }

    if std::fs::write(path, &normalized).is_ok() {
        Some(FixResult {
            path: path.to_string_lossy().to_string(),
            fix_type: "normalize_line_endings".to_string(),
            detail: "Converted CRLF line endings to LF".to_string(),
        })
    } else {
        None
    }
}

fn fix_status_based_move(
    path: &Path,
    path_str: &str,
    doc_type: &str,
    frontmatter: &std::collections::HashMap<String, String>,
    type_config: &DocumentTypeConfig,
    status_folders: &[&str],
) -> Option<FixResult> {
    let status = frontmatter.get("status")?;
    if !type_config.statuses.contains(status) {
        return None;
    }
    let parent = path.parent()?;
    let folder_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !status_folders.contains(&folder_name) || folder_name == *status {
        return None;
    }
    let code = frontmatter.get("code").cloned().unwrap_or_default();
    let slug = frontmatter.get("slug").cloned().unwrap_or_default();
    let new_file_name = format!("{doc_type}-{code}-{slug}.md");
    let parent_of_parent = parent.parent()?;
    let new_folder = parent_of_parent.join(status);
    if !new_folder.exists() {
        return None;
    }
    let new_path = new_folder.join(&new_file_name);
    let _ = std::fs::rename(path, &new_path);
    Some(FixResult {
        path: path_str.to_string(),
        fix_type: "move_file".to_string(),
        detail: format!("Moved from '{folder_name}' to '{status}'"),
    })
}

fn fix_category_based_move(
    path: &Path,
    path_str: &str,
    doc_type: &str,
    frontmatter: &std::collections::HashMap<String, String>,
) -> Option<FixResult> {
    let category = frontmatter.get("category")?;
    let parent = path.parent()?;
    let folder_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if folder_name == *category {
        return None;
    }
    let code = frontmatter.get("code").cloned().unwrap_or_default();
    let slug = frontmatter.get("slug").cloned().unwrap_or_default();
    let new_file_name = format!("{doc_type}-{code}-{slug}.md");
    let parent_of_parent = parent.parent()?;
    let new_folder = parent_of_parent.join(category);
    if !new_folder.exists() {
        return None;
    }
    let new_path = new_folder.join(&new_file_name);
    let _ = std::fs::rename(path, &new_path);
    Some(FixResult {
        path: path_str.to_string(),
        fix_type: "move_file".to_string(),
        detail: format!("Moved from '{folder_name}' to '{category}'"),
    })
}

fn fix_directory_based_move(
    path: &Path,
    path_str: &str,
    doc_type: &str,
    frontmatter: &std::collections::HashMap<String, String>,
) -> Option<FixResult> {
    let parent = path.parent()?;
    let folder_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if folder_name == doc_type {
        return None;
    }
    let code = frontmatter.get("code").cloned().unwrap_or_default();
    let slug = frontmatter.get("slug").cloned().unwrap_or_default();
    let new_file_name = format!("{doc_type}-{code}-{slug}.md");

    let mut current = parent;
    let mut target_folder = None;
    while let Some(p) = current.parent() {
        if p.file_name().is_some_and(|n| n == "doc") {
            target_folder = Some(current.to_path_buf());
            break;
        }
        current = p;
    }

    let target_folder = target_folder?;
    if target_folder.file_name().is_some_and(|n| n != doc_type) {
        return None;
    }

    if !target_folder.exists() {
        return None;
    }

    let new_path = target_folder.join(&new_file_name);
    if new_path.exists() {
        return None; // Collision avoidance
    }

    let _ = std::fs::rename(path, &new_path);
    Some(FixResult {
        path: path_str.to_string(),
        fix_type: "move_file".to_string(),
        detail: format!("Moved from nested folder '{folder_name}' to root 'doc/{doc_type}'"),
    })
}

fn fix_wikilinks(path_str: &str, content: &str) -> Option<WikilinkFix> {
    let regex = WIKILINK_EXTENSION_REGEX.as_ref().ok()?;
    let new_wikilinks = regex.replace_all(content, "[[$1]]");
    if new_wikilinks == content {
        return None;
    }
    Some((
        new_wikilinks.to_string(),
        FixResult {
            path: path_str.to_string(),
            fix_type: "normalize_wikilinks".to_string(),
            detail: "Removed .md extension from wikilinks".to_string(),
        },
    ))
}

fn fix_bare_governed_references(
    path_str: &str,
    content: &str,
    stem_index: &DocumentStemIndex,
) -> Option<GovernedReferenceFix> {
    let matches = find_bare_governed_stems(content, stem_index);
    if matches.is_empty() {
        return None;
    }

    let mut new_content = String::with_capacity(content.len() + matches.len() * 4);
    let mut previous_end = 0;
    for bare_match in matches {
        new_content.push_str(&content[previous_end..bare_match.start()]);
        new_content.push_str(bare_match.replacement());
        previous_end = bare_match.end();
    }
    new_content.push_str(&content[previous_end..]);

    Some((
        new_content,
        FixResult {
            path: path_str.to_string(),
            fix_type: "normalize_governed_references".to_string(),
            detail: "Wrapped bare governed document stems in wikilinks".to_string(),
        },
    ))
}

fn extract_content_after_frontmatter(content: &str) -> &str {
    content
        .find("---")
        .and_then(|start| {
            let rest = &content[start + 3..];
            rest.find("---").map(|end| &rest[end + 3..])
        })
        .unwrap_or(content)
}

fn fix_heading_if_needed(path_str: &str, content: &str) -> Option<HeadingFix> {
    let content_without_frontmatter = extract_content_after_frontmatter(content);
    let starts_with_heading = content_without_frontmatter.starts_with("\n# ")
        || content_without_frontmatter.starts_with("# ");
    if starts_with_heading {
        return None;
    }
    let first_line = content_without_frontmatter.lines().next()?;
    if first_line.is_empty() || first_line.starts_with('#') {
        return None;
    }
    let line_idx = content.find(content_without_frontmatter).unwrap_or(0);
    let heading_line_idx = line_idx + content_without_frontmatter.find(first_line).unwrap_or(0);
    let before = &content[..heading_line_idx];
    let after = &content[heading_line_idx + first_line.len()..];
    let new_content = format!("{}# {}{}", before, first_line.trim(), after);
    Some((
        new_content,
        FixResult {
            path: path_str.to_string(),
            fix_type: "normalize_heading".to_string(),
            detail: "Added heading markup to first line".to_string(),
        },
    ))
}

fn fix_governed_file(
    path: &Path,
    _doc_config: &DocumentTypesConfig,
    doc_type: &str,
    type_config: &DocumentTypeConfig,
    status_folders: &[&str],
    stem_index: &DocumentStemIndex,
) -> Vec<FixResult> {
    let mut fixes = Vec::new();
    let path_str = path.to_string_lossy().to_string();

    if let Some(fix) = fix_bom_if_present(path) {
        fixes.push(fix);
    }

    if let Some(fix) = fix_crlf_line_endings_if_present(path) {
        fixes.push(fix);
    }

    // Template files are governed by placeholder content — skip content-level fix logic.
    if doc_type == "template" {
        return fixes;
    }

    let Ok(content) = std::fs::read_to_string(path) else {
        return fixes;
    };

    let Some(frontmatter) = parse_frontmatter(&content) else {
        return fixes;
    };

    let mut new_content = Cow::Borrowed(content.as_str());

    if type_config.is_status_based()
        && let Some(fix) = fix_status_based_move(
            path,
            &path_str,
            doc_type,
            &frontmatter,
            type_config,
            status_folders,
        )
    {
        fixes.push(fix);
        return fixes;
    }

    if type_config.is_category_based()
        && let Some(fix) = fix_category_based_move(path, &path_str, doc_type, &frontmatter)
    {
        fixes.push(fix);
    }

    if type_config.is_directory_based()
        && let Some(fix) = fix_directory_based_move(path, &path_str, doc_type, &frontmatter)
    {
        fixes.push(fix);
        return fixes;
    }

    if let Some((fixed_content, fix)) = fix_wikilinks(&path_str, new_content.as_ref()) {
        new_content = Cow::Owned(fixed_content);
        fixes.push(fix);
    }

    if let Some((fixed_content, fix)) =
        fix_bare_governed_references(&path_str, new_content.as_ref(), stem_index)
    {
        new_content = Cow::Owned(fixed_content);
        fixes.push(fix);
    }

    if let Some((fixed_content, fix)) = fix_heading_if_needed(&path_str, new_content.as_ref()) {
        new_content = Cow::Owned(fixed_content);
        fixes.push(fix);
    }

    if new_content.as_ref() != content
        && std::fs::write(path, new_content.as_ref()).is_ok()
        && !fixes.iter().any(|f| f.path == path_str)
    {
        fixes.push(FixResult {
            path: path_str,
            fix_type: "normalize_content".to_string(),
            detail: "Normalized markdown structure".to_string(),
        });
    }

    fixes
}

pub(super) fn scan_and_fix_governed_files(
    root_dir: &Path,
    doc_config: &DocumentTypesConfig,
) -> FixScanResult {
    let mut errors = Vec::new();
    let warnings = Vec::new();
    let mut fixes = Vec::new();

    let status_folders = status_folder_names(doc_config);
    let stem_index = build_document_stem_index(root_dir, doc_config);

    for doc_type in doc_config.document_types.keys() {
        let Some(type_config) = doc_config.document_types.get(doc_type) else {
            continue;
        };
        for path in governed_markdown_files(root_dir, doc_type) {
            let file_fixes = fix_governed_file(
                &path,
                doc_config,
                doc_type,
                type_config,
                &status_folders,
                &stem_index,
            );
            fixes.extend(file_fixes);
        }
    }

    let (post_fix_errors, _) = scan_governed_files(root_dir, doc_config);
    errors.extend(post_fix_errors);

    ((errors, warnings), fixes)
}

#[cfg(test)]
#[path = "validate_fix_test.rs"]
mod tests;
