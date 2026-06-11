//! Plugin operation for applying a unified diff to a governed document.

use patcher::{Patch, PatchAlgorithm, Patcher};
use runtime_core::{FlowOperation, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::fmt::Display;
use std::fs;
use std::path::Path;

use crate::operations::find_doc::{FindDocInput, FindDocOp, FindDocOutput};
use crate::operations::support::CapturingSender;

const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HunkLineCounts {
    old: usize,
    new: usize,
}

/// Input for the `patch_doc` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PatchDocInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The numeric code of the document to patch.
    pub code: u32,
    /// The unified diff to apply to the document.
    pub git_diff: String,
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

fn parse_hunk_range_count(range: &str, prefix: char) -> Option<usize> {
    let raw_range = range.strip_prefix(prefix)?;
    let (_start, count) = raw_range.split_once(',').unwrap_or((raw_range, "1"));
    count.parse().ok()
}

fn parse_hunk_header_counts(line: &str) -> Option<HunkLineCounts> {
    if !line.starts_with("@@ ") {
        return None;
    }

    let mut parts = line.strip_prefix("@@ ")?.split_whitespace();
    let old = parse_hunk_range_count(parts.next()?, '-')?;
    let new = parse_hunk_range_count(parts.next()?, '+')?;
    let closing_marker = parts.next()?;
    if closing_marker != "@@" {
        return None;
    }

    Some(HunkLineCounts { old, new })
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

async fn patch_doc(
    input: PatchDocInput,
    output: &mut impl PluginSender<PatchDocOutput>,
) -> RuntimeResult<()> {
    // Locate the governed document
    let find_input = FindDocInput::new(
        input.root_dir.clone(),
        String::new(),
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
    let doc_dir = dunce::canonicalize(input.root_dir.as_path().join("doc")).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("failed to resolve doc/ directory: {e}"))
    })?;

    if !Path::new(&abs_path).starts_with(&doc_dir) {
        return Err(runtime_core::RuntimeError::operation(
            "resolved document path is outside the doc/ directory",
        ));
    }

    // Normalize and parse the diff
    let normalized = normalize_diff(&input.git_diff);
    preflight_hunk_line_counts(&normalized).map_err(runtime_core::RuntimeError::operation)?;

    let patch = Patch::parse(&normalized)
        .map_err(|e| runtime_core::RuntimeError::operation(patch_parse_error_message(e)))?;

    // Reject delete patches (new_file == /dev/null)
    if patch.new_file == "/dev/null" {
        return Err(runtime_core::RuntimeError::operation(
            "patch would delete the document; delete operations are not supported",
        ));
    }

    // Reject create patches (old_file == /dev/null)
    if patch.old_file == "/dev/null" {
        return Err(runtime_core::RuntimeError::operation(
            "patch would create a new file; use create_doc for new documents",
        ));
    }

    // Reject rename patches (old_file != new_file)
    if patch.old_file != patch.new_file {
        return Err(runtime_core::RuntimeError::operation(format!(
            "patch renames '{}' to '{}'; rename operations are not supported",
            patch.old_file, patch.new_file
        )));
    }

    // Target mismatch: patch filename must match the resolved document filename
    let resolved_name =
        Path::new(&abs_path).file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            runtime_core::RuntimeError::operation("resolved document path has no filename")
        })?;

    let patch_name = Path::new(&patch.old_file)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(patch.old_file.as_str());

    if patch_name != resolved_name {
        return Err(runtime_core::RuntimeError::operation(format!(
            "patch targets '{}' but the resolved document is '{}'",
            patch.old_file, resolved_name
        )));
    }

    // Apply the patch
    let patcher = Patcher::new(patch);
    let new_content = patcher.apply(&existing_content, false).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("failed to apply patch: {e}"))
    })?;

    // BOM check: resulting content must not contain a UTF-8 BOM
    if new_content.as_bytes().starts_with(UTF8_BOM) {
        return Err(runtime_core::RuntimeError::operation(
            "resulting content contains a UTF-8 BOM (\\xEF\\xBB\\xBF); remove the BOM and retry",
        ));
    }

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
    /// Construct a new `PatchDocInput`.
    #[must_use]
    pub const fn new(root_dir: IoPath, doc_type: String, code: u32, git_diff: String) -> Self {
        Self { root_dir, doc_type, code, git_diff }
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
