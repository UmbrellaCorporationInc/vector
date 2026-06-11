//! `apply_patch` format parser and applier for `patch_doc`.

use runtime_core::RuntimeResult;
use runtime_io::path::IoPath;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ApplyPatchUpdate {
    pub(super) target: String,
    pub(super) hunks: Vec<ApplyPatchHunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ApplyPatchHunk {
    pub(super) old_lines: Vec<String>,
    pub(super) new_lines: Vec<String>,
}

type ParsedApplyPatchHunks = (Vec<ApplyPatchHunk>, usize);

fn apply_patch_format_error_message(error: impl std::fmt::Display) -> String {
    format!("format: \"apply_patch\": {error}")
}

fn apply_patch_operation_error(error: impl std::fmt::Display) -> runtime_core::RuntimeError {
    runtime_core::RuntimeError::operation(apply_patch_format_error_message(error))
}

fn unsupported_apply_patch_operation(operation: &str, target: &str) -> String {
    format!(
        "unsupported apply_patch operation '{operation}' for target '{target}'. \
         patch_doc supports only '*** Update File:' for the resolved governed document"
    )
}

fn parse_apply_patch_boundaries(payload: &str) -> Result<Vec<String>, String> {
    let stripped = super::strip_code_fence(payload);
    let lines: Vec<&str> = stripped.lines().collect();
    let begin_index = lines.iter().position(|line| !line.trim().is_empty()).ok_or_else(|| {
        "missing apply_patch boundary '*** Begin Patch' before any operations".to_string()
    })?;

    if lines[begin_index].trim() != "*** Begin Patch" {
        return Err(
            "missing apply_patch boundary '*** Begin Patch' before any operations".to_string()
        );
    }

    let end_index = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .ok_or_else(|| "missing apply_patch boundary '*** End Patch'".to_string())?;

    if end_index <= begin_index || lines[end_index].trim() != "*** End Patch" {
        return Err("missing apply_patch boundary '*** End Patch' after operations".to_string());
    }

    Ok(lines[begin_index + 1..end_index].iter().map(|line| (*line).to_string()).collect())
}

fn parse_apply_patch_hunks(
    lines: &[String],
    mut index: usize,
    target: &str,
) -> Result<ParsedApplyPatchHunks, String> {
    let mut hunks = Vec::new();

    while index < lines.len() {
        let trimmed = lines[index].trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }

        if let Some(target) = trimmed.strip_prefix("*** Add File: ") {
            return Err(unsupported_apply_patch_operation("Add File", target));
        }
        if let Some(target) = trimmed.strip_prefix("*** Delete File: ") {
            return Err(unsupported_apply_patch_operation("Delete File", target));
        }
        if let Some(target) = trimmed.strip_prefix("*** Move to: ") {
            return Err(unsupported_apply_patch_operation("Move to", target));
        }
        if trimmed.starts_with("*** Update File: ") {
            break;
        }

        if !trimmed.starts_with("@@") {
            return Err(format!(
                "expected an '@@' hunk header after '*** Update File: {target}', found '{}'",
                lines[index]
            ));
        }

        index += 1;
        let mut old_lines = Vec::new();
        let mut new_lines = Vec::new();
        let mut body_lines = 0usize;

        while index < lines.len() {
            let line = lines[index].as_str();
            let trimmed = line.trim();
            if trimmed.starts_with("@@") || trimmed.starts_with("*** ") {
                break;
            }
            if line == r"\ No newline at end of file" {
                index += 1;
                continue;
            }

            let mut chars = line.chars();
            let Some(prefix) = chars.next() else {
                return Err("empty hunk body line; prefix blank context lines with a single space"
                    .to_string());
            };
            let content = chars.as_str().to_string();
            match prefix {
                ' ' => {
                    old_lines.push(content.clone());
                    new_lines.push(content);
                }
                '-' => old_lines.push(content),
                '+' => new_lines.push(content),
                _ => {
                    return Err(format!(
                        "invalid hunk body line '{line}'; each hunk line must start with ' ', '+', or '-'"
                    ));
                }
            }

            body_lines += 1;
            index += 1;
        }

        if body_lines == 0 {
            return Err("apply_patch hunk has no body lines after '@@'".to_string());
        }

        hunks.push(ApplyPatchHunk { old_lines, new_lines });
    }

    if hunks.is_empty() {
        return Err(format!(
            "'*** Update File: {target}' requires at least one '@@' hunk to apply"
        ));
    }

    Ok((hunks, index))
}

fn parse_apply_patch_update(payload: &str) -> Result<ApplyPatchUpdate, String> {
    let lines = parse_apply_patch_boundaries(payload)?;
    let mut index = 0usize;
    let mut update: Option<ApplyPatchUpdate> = None;

    while index < lines.len() {
        let trimmed = lines[index].trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }

        if let Some(target) = trimmed.strip_prefix("*** Add File: ") {
            return Err(unsupported_apply_patch_operation("Add File", target));
        }
        if let Some(target) = trimmed.strip_prefix("*** Delete File: ") {
            return Err(unsupported_apply_patch_operation("Delete File", target));
        }
        if let Some(target) = trimmed.strip_prefix("*** Move to: ") {
            return Err(unsupported_apply_patch_operation("Move to", target));
        }

        let Some(target) = trimmed.strip_prefix("*** Update File: ") else {
            return Err(format!("expected '*** Update File:' operation, found '{}'", lines[index]));
        };

        if update.is_some() {
            return Err(
                "multiple apply_patch operations are not supported; provide one Update File operation for the resolved governed document"
                    .to_string(),
            );
        }

        let target = target.trim();
        if target.is_empty() {
            return Err("'*** Update File:' requires a target path".to_string());
        }

        let (hunks, next_index) = parse_apply_patch_hunks(&lines, index + 1, target)?;
        update = Some(ApplyPatchUpdate { target: target.to_string(), hunks });
        index = next_index;
    }

    update.ok_or_else(|| {
        "apply_patch payload must contain one '*** Update File:' operation".to_string()
    })
}

fn validate_apply_patch_target(
    abs_path: &str,
    root_dir: &IoPath,
    package: &str,
    target: &str,
) -> Result<(), String> {
    let resolved = dunce::canonicalize(Path::new(abs_path))
        .map_err(|e| format!("failed to canonicalize resolved document path: {e}"))?;
    let target_path = Path::new(target);
    let candidate = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        super::governed_root_dir(root_dir, package).as_path().join(target_path)
    };
    let candidate = dunce::canonicalize(&candidate).map_err(|e| {
        format!(
            "patch targets '{target}', but that path does not resolve to the governed document: {e}"
        )
    })?;

    if candidate == resolved {
        Ok(())
    } else {
        Err(format!(
            "patch targets '{}' but the resolved document is '{}'",
            target,
            resolved.display()
        ))
    }
}

fn find_apply_patch_hunk_matches(
    lines: &[String],
    expected: &[String],
    start_index: usize,
) -> Vec<usize> {
    if expected.is_empty() || expected.len() > lines.len().saturating_sub(start_index) {
        return Vec::new();
    }

    lines[start_index..]
        .windows(expected.len())
        .enumerate()
        .filter_map(|(offset, window)| (window == expected).then_some(start_index + offset))
        .collect()
}

fn apply_apply_patch_hunks(
    canonical_content: &str,
    hunks: &[ApplyPatchHunk],
) -> Result<String, String> {
    let had_trailing_newline = canonical_content.ends_with('\n');
    let mut lines: Vec<String> = super::logical_lines(canonical_content)
        .into_iter()
        .map(std::string::ToString::to_string)
        .collect();
    let mut cursor = 0usize;

    for (hunk_index, hunk) in hunks.iter().enumerate() {
        let display_index = hunk_index + 1;
        if hunk.old_lines.is_empty() {
            return Err(format!(
                "ambiguous context in hunk {display_index}: the hunk has no old-side context. \
                 Include at least one context or removal line."
            ));
        }

        let matches = find_apply_patch_hunk_matches(&lines, &hunk.old_lines, cursor);
        match matches.as_slice() {
            [] => {
                return Err(format!(
                    "missing context for hunk {display_index}. Expected old-side lines: [{}]",
                    super::format_lines_for_diagnostic(&hunk.old_lines)
                ));
            }
            [start] => {
                let end = start + hunk.old_lines.len();
                lines.splice(*start..end, hunk.new_lines.clone());
                cursor = start + hunk.new_lines.len();
            }
            _ => {
                return Err(format!(
                    "ambiguous context in hunk {display_index}: old-side lines match {} locations. \
                     Add more context around: [{}]",
                    matches.len(),
                    super::format_lines_for_diagnostic(&hunk.old_lines)
                ));
            }
        }
    }

    let mut content = lines.join("\n");
    if had_trailing_newline && !lines.is_empty() {
        content.push('\n');
    }
    Ok(content)
}

pub(super) fn apply_apply_patch_format(
    abs_path: &str,
    root_dir: &IoPath,
    package: &str,
    existing_content: &str,
    patch_payload: &str,
) -> RuntimeResult<String> {
    let update = parse_apply_patch_update(patch_payload).map_err(apply_patch_operation_error)?;
    validate_apply_patch_target(abs_path, root_dir, package, &update.target)
        .map_err(apply_patch_operation_error)?;
    let canonical_document = super::canonicalize_document_content(existing_content)
        .map_err(apply_patch_operation_error)?;

    let patched_canonical = apply_apply_patch_hunks(&canonical_document.content, &update.hunks)
        .map_err(apply_patch_operation_error)?;
    let new_content =
        super::restore_newline_mode(&patched_canonical, canonical_document.newline_mode);

    if new_content.as_bytes().starts_with(super::UTF8_BOM) {
        return Err(apply_patch_operation_error(
            "resulting content contains a UTF-8 BOM (\\xEF\\xBB\\xBF); remove the BOM and retry",
        ));
    }

    Ok(new_content)
}

#[cfg(test)]
#[path = "patch_doc_apply_patch_test.rs"]
mod tests;
