//! Plugin operation for applying a patch to a governed document.

use patcher::{Patch, PatchAlgorithm, Patcher};
use runtime_core::{FlowOperation, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

use crate::operations::find_doc::{FindDocInput, FindDocOp, FindDocOutput};
use crate::operations::support::CapturingSender;

const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";
const SUPPORTED_PATCH_FORMATS: [&str; 2] = ["unified", "apply_patch"];

/// Patch payload formats supported by the `patch_doc` operation contract.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchDocFormat {
    /// Standard unified diff syntax.
    Unified,
    /// Agent-oriented `apply_patch` syntax.
    ApplyPatch,
}

impl PatchDocFormat {
    /// Return the wire-format value for this patch format.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unified => "unified",
            Self::ApplyPatch => "apply_patch",
        }
    }

    /// Parse an optional wire-format value.
    ///
    /// An omitted format defaults to `apply_patch`; `git_diff` compatibility callers should
    /// explicitly select `unified` before constructing runtime input.
    ///
    /// # Errors
    ///
    /// Returns an error when the explicit value is not one of the supported patch formats.
    pub fn parse_optional(value: Option<&str>) -> Result<Self, String> {
        match value {
            None | Some("apply_patch") => Ok(Self::ApplyPatch),
            Some("unified") => Ok(Self::Unified),
            Some(other) => Err(unknown_patch_format_message(other)),
        }
    }

    /// Supported wire-format values.
    #[must_use]
    pub const fn supported_values() -> &'static [&'static str; 2] {
        &SUPPORTED_PATCH_FORMATS
    }
}

fn unknown_patch_format_message(value: &str) -> String {
    format!(
        "unknown patch format '{value}'. Supported values: {}. Omit format to use 'apply_patch'.",
        SUPPORTED_PATCH_FORMATS.join(", ")
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HunkLineCounts {
    old: usize,
    new: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NewlineMode {
    Lf,
    Crlf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalDocument {
    content: String,
    newline_mode: NewlineMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HunkHeader {
    old_start: usize,
    old_count: usize,
    new_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HunkRange {
    start: usize,
    count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HunkContext {
    index: usize,
    header: String,
    old_start: usize,
    expected: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApplyPatchUpdate {
    target: String,
    hunks: Vec<ApplyPatchHunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApplyPatchHunk {
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

type ParsedApplyPatchHunks = (Vec<ApplyPatchHunk>, usize);

#[derive(Debug, Clone, PartialEq, Eq)]
struct InProgressHunkContext {
    header_line: String,
    header: HunkHeader,
    expected: Vec<String>,
}

/// Input for the `patch_doc` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PatchDocInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// Optional synchronized package name for package-qualified lookup.
    ///
    /// When empty, the document is resolved within `root_dir`.
    /// When set, the document is resolved inside `.vector-database/packages/{package}/`.
    pub package: String,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The numeric code of the document to patch.
    pub code: u32,
    /// The patch payload format.
    pub format: PatchDocFormat,
    /// The patch payload to apply to the document.
    pub patch: String,
}

/// Output for the `patch_doc` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PatchDocOutput {
    /// The absolute path of the patched document.
    pub path: String,
    /// The final document content after the patch was applied.
    pub content: String,
}

/// Removes a Markdown code fence wrapper from the diff if present.
fn strip_code_fence(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with("```") {
        return raw.to_string();
    }
    let mut lines = trimmed.lines();
    lines.next(); // consume the opening ``` line
    let mut result = String::new();
    for line in lines {
        if line.trim() == "```" {
            break;
        }
        result.push_str(line);
        result.push('\n');
    }
    if result.is_empty() { raw.to_string() } else { result }
}

/// Extracts the raw unified diff from a potentially prose-wrapped or fenced agent output.
fn normalize_diff(raw: &str) -> String {
    let stripped = strip_code_fence(raw);
    let mut result = String::new();
    let mut found = false;
    for line in stripped.lines() {
        if !found {
            if line.starts_with("diff ") || line.starts_with("--- ") {
                found = true;
            } else {
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }
    if result.is_empty() { stripped } else { result }
}

fn patch_parse_error_message(error: impl Display) -> String {
    let parser_error = error.to_string();
    if parser_error.contains("Chunk line count mismatch") {
        return format!(
            "patch is not a valid unified diff: hunk line-count mismatch. \
             Make the @@ -a,b +c,d @@ counts match the number of old-side lines \
             and new-side lines in the hunk body. Original parser error: {parser_error}"
        );
    }

    format!("patch is not a valid unified diff: {parser_error}")
}

fn unified_format_error_message(error: impl Display) -> String {
    format!("format: \"unified\": {error}")
}

fn unified_operation_error(error: impl Display) -> runtime_core::RuntimeError {
    runtime_core::RuntimeError::operation(unified_format_error_message(error))
}

fn apply_patch_format_error_message(error: impl Display) -> String {
    format!("format: \"apply_patch\": {error}")
}

fn apply_patch_operation_error(error: impl Display) -> runtime_core::RuntimeError {
    runtime_core::RuntimeError::operation(apply_patch_format_error_message(error))
}

fn parse_hunk_range(range: &str, prefix: char) -> Option<HunkRange> {
    let raw_range = range.strip_prefix(prefix)?;
    let (start, count) = raw_range.split_once(',').unwrap_or((raw_range, "1"));
    Some(HunkRange { start: start.parse().ok()?, count: count.parse().ok()? })
}

fn parse_hunk_header(line: &str) -> Option<HunkHeader> {
    if !line.starts_with("@@ ") {
        return None;
    }

    let mut parts = line.strip_prefix("@@ ")?.split_whitespace();
    let old_range = parse_hunk_range(parts.next()?, '-')?;
    let new_range = parse_hunk_range(parts.next()?, '+')?;
    let closing_marker = parts.next()?;
    if closing_marker != "@@" {
        return None;
    }

    Some(HunkHeader {
        old_start: old_range.start,
        old_count: old_range.count,
        new_count: new_range.count,
    })
}

fn parse_hunk_header_counts(line: &str) -> Option<HunkLineCounts> {
    let header = parse_hunk_header(line)?;
    Some(HunkLineCounts { old: header.old_count, new: header.new_count })
}

fn hunk_count_mismatch_message(
    header: &str,
    expected: HunkLineCounts,
    actual: HunkLineCounts,
) -> String {
    format!(
        "patch is not a valid unified diff: hunk line-count mismatch. \
         Hunk header declares (-{}, +{}) but the hunk body contains (-{}, +{}). \
         Make the @@ -a,b +c,d @@ counts match the number of old-side lines \
         and new-side lines in the hunk body. Preflight detail: Header expected (-{}, +{}), \
         Parsed content counts (-{}, +{}). Hunk header: {header}",
        expected.old,
        expected.new,
        actual.old,
        actual.new,
        expected.old,
        expected.new,
        actual.old,
        actual.new
    )
}

fn validate_completed_hunk_count(
    header: &str,
    expected: HunkLineCounts,
    actual: HunkLineCounts,
) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(hunk_count_mismatch_message(header, expected, actual))
    }
}

fn preflight_hunk_line_counts(diff: &str) -> Result<(), String> {
    let mut current_hunk: Option<(String, HunkLineCounts, HunkLineCounts)> = None;

    for line in diff.lines() {
        if let Some(expected) = parse_hunk_header_counts(line) {
            if let Some((header, previous_expected, actual)) = current_hunk.take() {
                validate_completed_hunk_count(&header, previous_expected, actual)?;
            }

            current_hunk = Some((line.to_string(), expected, HunkLineCounts { old: 0, new: 0 }));
            continue;
        }

        if let Some((_header, _expected, actual)) = current_hunk.as_mut() {
            match line.as_bytes().first().copied() {
                Some(b' ') => {
                    actual.old += 1;
                    actual.new += 1;
                }
                Some(b'-') => actual.old += 1,
                Some(b'+') => actual.new += 1,
                _ => {}
            }
        }
    }

    if let Some((header, expected, actual)) = current_hunk {
        validate_completed_hunk_count(&header, expected, actual)?;
    }

    Ok(())
}

fn detect_newline_mode(content: &str) -> Result<NewlineMode, String> {
    let mut crlf = 0usize;
    let mut bare_lf = 0usize;
    let mut bare_cr = 0usize;
    let bytes = content.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                crlf += 1;
                index += 2;
            }
            b'\r' => {
                bare_cr += 1;
                index += 1;
            }
            b'\n' => {
                bare_lf += 1;
                index += 1;
            }
            _ => index += 1,
        }
    }

    if bare_cr > 0 {
        return Err(
            "document uses unsupported bare carriage-return line endings; patch_doc can only normalize LF or CRLF safely"
                .to_string(),
        );
    }

    if crlf > 0 && bare_lf > 0 {
        return Err(
            "document uses mixed LF and CRLF line endings; patch_doc can only normalize a homogeneous newline mode safely"
                .to_string(),
        );
    }

    if crlf > 0 { Ok(NewlineMode::Crlf) } else { Ok(NewlineMode::Lf) }
}

const fn newline_mode_description(mode: NewlineMode) -> &'static str {
    match mode {
        NewlineMode::Lf => "LF",
        NewlineMode::Crlf => "CRLF normalized to LF for patch matching and restored on write",
    }
}

fn canonicalize_document_content(content: &str) -> Result<CanonicalDocument, String> {
    let newline_mode = detect_newline_mode(content)?;
    let canonical = match newline_mode {
        NewlineMode::Lf => content.to_string(),
        NewlineMode::Crlf => content.replace("\r\n", "\n"),
    };

    Ok(CanonicalDocument { content: canonical, newline_mode })
}

fn restore_newline_mode(content: &str, mode: NewlineMode) -> String {
    match mode {
        NewlineMode::Lf => content.to_string(),
        NewlineMode::Crlf => content.replace('\n', "\r\n"),
    }
}

fn logical_lines(content: &str) -> Vec<&str> {
    if content.is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<&str> = content.split('\n').collect();
    if content.ends_with('\n') {
        lines.pop();
    }
    lines
}

fn finish_hunk_context(contexts: &mut Vec<HunkContext>, current: Option<InProgressHunkContext>) {
    if let Some(current) = current {
        contexts.push(HunkContext {
            index: contexts.len() + 1,
            header: current.header_line,
            old_start: current.header.old_start,
            expected: current.expected,
        });
    }
}

fn collect_hunk_contexts(diff: &str) -> Vec<HunkContext> {
    let mut contexts = Vec::new();
    let mut current: Option<InProgressHunkContext> = None;

    for line in diff.lines() {
        if let Some(header) = parse_hunk_header(line) {
            finish_hunk_context(&mut contexts, current.take());
            current = Some(InProgressHunkContext {
                header_line: line.to_string(),
                header,
                expected: Vec::new(),
            });
            continue;
        }

        if let Some(current) = current.as_mut()
            && let Some(b' ' | b'-') = line.as_bytes().first().copied()
        {
            current.expected.push(line[1..].to_string());
        }
    }

    finish_hunk_context(&mut contexts, current);
    contexts
}

fn format_lines_for_diagnostic(lines: &[String]) -> String {
    if lines.is_empty() {
        return "<none>".to_string();
    }

    lines.iter().map(|line| format!("\"{}\"", line.escape_default())).collect::<Vec<_>>().join(", ")
}

fn hunk_context_mismatch_message(
    hunk: &HunkContext,
    observed: &[String],
    newline_mode: NewlineMode,
) -> String {
    format!(
        "failed to apply patch: hunk {} context mismatch. Hunk header: {}. \
         Newline mode: {}. Expected context: [{}]. Observed context at document line {}: [{}].",
        hunk.index,
        hunk.header,
        newline_mode_description(newline_mode),
        format_lines_for_diagnostic(&hunk.expected),
        hunk.old_start,
        format_lines_for_diagnostic(observed)
    )
}

fn validate_hunk_contexts(
    diff: &str,
    canonical_content: &str,
    newline_mode: NewlineMode,
) -> Result<(), String> {
    let document_lines = logical_lines(canonical_content);

    for hunk in collect_hunk_contexts(diff) {
        if hunk.expected.is_empty() {
            continue;
        }

        let start_index = hunk.old_start.saturating_sub(1);
        let observed: Vec<String> = document_lines
            .iter()
            .skip(start_index)
            .take(hunk.expected.len())
            .map(|line| (*line).to_string())
            .collect();

        if observed != hunk.expected {
            return Err(hunk_context_mismatch_message(&hunk, &observed, newline_mode));
        }
    }

    Ok(())
}

fn governed_root_dir(root_dir: &IoPath, package: &str) -> IoPath {
    if package.is_empty() {
        root_dir.clone()
    } else {
        root_dir.join(".vector-database").join("packages").join(package)
    }
}

fn canonical_governed_doc_dir(
    root_dir: &IoPath,
    package: &str,
) -> Result<PathBuf, runtime_core::RuntimeError> {
    dunce::canonicalize(governed_root_dir(root_dir, package).as_path().join("doc")).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("failed to resolve doc/ directory: {e}"))
    })
}

fn unsupported_apply_patch_operation(operation: &str, target: &str) -> String {
    format!(
        "unsupported apply_patch operation '{operation}' for target '{target}'. \
         patch_doc supports only '*** Update File:' for the resolved governed document"
    )
}

fn parse_apply_patch_boundaries(payload: &str) -> Result<Vec<String>, String> {
    let stripped = strip_code_fence(payload);
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
        governed_root_dir(root_dir, package).as_path().join(target_path)
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
    let mut lines: Vec<String> = logical_lines(canonical_content)
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
                    format_lines_for_diagnostic(&hunk.old_lines)
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
                    format_lines_for_diagnostic(&hunk.old_lines)
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

fn apply_apply_patch_format(
    abs_path: &str,
    root_dir: &IoPath,
    package: &str,
    existing_content: &str,
    patch_payload: &str,
) -> RuntimeResult<String> {
    let update = parse_apply_patch_update(patch_payload).map_err(apply_patch_operation_error)?;
    validate_apply_patch_target(abs_path, root_dir, package, &update.target)
        .map_err(apply_patch_operation_error)?;
    let canonical_document =
        canonicalize_document_content(existing_content).map_err(apply_patch_operation_error)?;

    let patched_canonical = apply_apply_patch_hunks(&canonical_document.content, &update.hunks)
        .map_err(apply_patch_operation_error)?;
    let new_content = restore_newline_mode(&patched_canonical, canonical_document.newline_mode);

    if new_content.as_bytes().starts_with(UTF8_BOM) {
        return Err(apply_patch_operation_error(
            "resulting content contains a UTF-8 BOM (\\xEF\\xBB\\xBF); remove the BOM and retry",
        ));
    }

    Ok(new_content)
}

fn apply_unified_patch(
    abs_path: &str,
    existing_content: &str,
    patch_payload: &str,
) -> RuntimeResult<String> {
    let canonical_document =
        canonicalize_document_content(existing_content).map_err(unified_operation_error)?;

    // Normalize and parse the diff
    let normalized = normalize_diff(patch_payload);
    preflight_hunk_line_counts(&normalized).map_err(unified_operation_error)?;

    let patch = Patch::parse(&normalized)
        .map_err(|e| unified_operation_error(patch_parse_error_message(e)))?;

    // Reject delete patches (new_file == /dev/null)
    if patch.new_file == "/dev/null" {
        return Err(unified_operation_error(
            "patch would delete the document; delete operations are not supported",
        ));
    }

    // Reject create patches (old_file == /dev/null)
    if patch.old_file == "/dev/null" {
        return Err(unified_operation_error(
            "patch would create a new file; use create_doc for new documents",
        ));
    }

    // Reject rename patches (old_file != new_file)
    if patch.old_file != patch.new_file {
        return Err(unified_operation_error(format!(
            "patch renames '{}' to '{}'; rename operations are not supported",
            patch.old_file, patch.new_file
        )));
    }

    // Target mismatch: patch filename must match the resolved document filename
    let resolved_name = Path::new(abs_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| unified_operation_error("resolved document path has no filename"))?;

    let patch_name = Path::new(&patch.old_file)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(patch.old_file.as_str());

    if patch_name != resolved_name {
        return Err(unified_operation_error(format!(
            "patch targets '{}' but the resolved document is '{}'",
            patch.old_file, resolved_name
        )));
    }

    validate_hunk_contexts(
        &normalized,
        &canonical_document.content,
        canonical_document.newline_mode,
    )
    .map_err(unified_operation_error)?;

    // Apply the patch against the canonical LF representation, then restore the file's newline mode.
    let patcher = Patcher::new(patch);
    let patched_canonical = patcher.apply(&canonical_document.content, false).map_err(|e| {
        unified_operation_error(format!(
            "failed to apply patch after newline normalization; newline mode: {}; {e}",
            newline_mode_description(canonical_document.newline_mode)
        ))
    })?;
    let new_content = restore_newline_mode(&patched_canonical, canonical_document.newline_mode);

    // BOM check: resulting content must not contain a UTF-8 BOM
    if new_content.as_bytes().starts_with(UTF8_BOM) {
        return Err(unified_operation_error(
            "resulting content contains a UTF-8 BOM (\\xEF\\xBB\\xBF); remove the BOM and retry",
        ));
    }

    Ok(new_content)
}

async fn patch_doc(
    input: PatchDocInput,
    output: &mut impl PluginSender<PatchDocOutput>,
) -> RuntimeResult<()> {
    // Locate the governed document
    let find_input = FindDocInput::new(
        input.root_dir.clone(),
        input.package.clone(),
        input.doc_type.clone(),
        input.code,
    );
    let mut capture = CapturingSender::<FindDocOutput>::new();
    FindDocOp::new().run(find_input, &mut capture).await?;

    let found = capture.into_output().ok_or_else(|| {
        runtime_core::RuntimeError::operation(format!(
            "no document of type '{}' with code {} found",
            input.doc_type, input.code
        ))
    })?;

    let abs_path = found.path;
    let existing_content = found.content;

    // Scope check: the resolved path must be inside doc/
    let doc_dir = canonical_governed_doc_dir(&input.root_dir, &input.package)?;

    if !Path::new(&abs_path).starts_with(&doc_dir) {
        return Err(runtime_core::RuntimeError::operation(
            "resolved document path is outside the doc/ directory",
        ));
    }

    let new_content = match input.format {
        PatchDocFormat::Unified => apply_unified_patch(&abs_path, &existing_content, &input.patch)?,
        PatchDocFormat::ApplyPatch => apply_apply_patch_format(
            &abs_path,
            &input.root_dir,
            &input.package,
            &existing_content,
            &input.patch,
        )?,
    };

    // Write the patched content back to disk
    fs::write(&abs_path, new_content.as_bytes()).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("failed to write patched document: {e}"))
    })?;

    output.send(PatchDocOutput { path: abs_path, content: new_content }).await?;
    Ok(())
}

declare_plugin_operations! {
    PatchDocOp => patch_doc(PatchDocInput, PatchDocOutput)
}

impl PatchDocInput {
    /// Construct a backward-compatible unified-diff `PatchDocInput`.
    #[must_use]
    pub const fn new(root_dir: IoPath, doc_type: String, code: u32, patch: String) -> Self {
        Self {
            root_dir,
            package: String::new(),
            doc_type,
            code,
            format: PatchDocFormat::Unified,
            patch,
        }
    }

    /// Construct a `PatchDocInput` with explicit package, format, and patch payload.
    #[must_use]
    pub const fn with_format(
        root_dir: IoPath,
        package: String,
        doc_type: String,
        code: u32,
        format: PatchDocFormat,
        patch: String,
    ) -> Self {
        Self { root_dir, package, doc_type, code, format, patch }
    }
}

impl PatchDocOp {
    /// Construct a new `PatchDocOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for PatchDocOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "patch_doc_test.rs"]
mod tests;
