//! Configuration model and validation for the `instructions-dir` field in `.vector/agents.yaml`.
//!
//! This module provides the same observable validation semantics as the TypeScript
//! implementation in `agentsConfig.ts`, implemented independently without shared fixtures.

use std::env;
use std::path::Path;

/// The only supported variable expression in `instructions-dir`.
const SYSTEM_TEMP_VAR: &str = "${system-temp}";

/// Error variants for `instructions-dir` validation.
///
/// Each variant maps directly to a specific, actionable validation failure so that
/// tests can match on the exact failure case rather than parsing error strings.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionsDirError {
    /// The `instructions-dir` key is absent from the YAML document.
    Missing,
    /// The value is present but is not a string (e.g. a mapping or sequence).
    NotAString,
    /// The value is a string that is empty or contains only whitespace.
    Empty,
    /// The value contains a `${...}` variable expression other than `${system-temp}`.
    UnsupportedVariable(String),
    /// The value contains no `${system-temp}` variable expression.
    MissingSystemTempVar,
    /// The value contains `${system-temp}` more than once.
    RepeatedSystemTempVar,
    /// `${system-temp}` is present but is not the first character sequence in the value.
    NotStartingWithSystemTemp,
    /// The path suffix contains a `..` component.
    ContainsTraversal,
    /// After substituting `${system-temp}`, the resolved path escapes the system temp directory.
    EscapesSystemTemp,
}

impl InstructionsDirError {
    /// Return an actionable, user-facing message for this error.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Missing => ".vector/agents.yaml: 'instructions-dir' is required".to_string(),
            Self::NotAString => {
                ".vector/agents.yaml: 'instructions-dir' must be a non-empty string".to_string()
            }
            Self::Empty => ".vector/agents.yaml: 'instructions-dir' must not be empty".to_string(),
            Self::UnsupportedVariable(v) => format!(
                ".vector/agents.yaml: 'instructions-dir' contains unsupported variable \
                 expression '{v}'; only ${{system-temp}} is supported"
            ),
            Self::MissingSystemTempVar | Self::NotStartingWithSystemTemp => {
                ".vector/agents.yaml: 'instructions-dir' must begin with ${system-temp}".to_string()
            }
            Self::RepeatedSystemTempVar => {
                ".vector/agents.yaml: 'instructions-dir' must contain ${system-temp} exactly once"
                    .to_string()
            }
            Self::ContainsTraversal => {
                ".vector/agents.yaml: 'instructions-dir' must not contain path traversal (..)"
                    .to_string()
            }
            Self::EscapesSystemTemp => {
                ".vector/agents.yaml: 'instructions-dir' resolves outside the system \
                 temporary directory"
                    .to_string()
            }
        }
    }
}

/// Validate the `instructions-dir` string value from `.vector/agents.yaml`.
///
/// Covers: empty, no variable, unsupported variable, repeated variable,
/// variable not at start, path traversal, and escape outside system temp.
///
/// # Errors
/// Returns the first `InstructionsDirError` that describes the failure.
pub fn validate_instructions_dir(value: &str) -> Result<(), InstructionsDirError> {
    if value.trim().is_empty() {
        return Err(InstructionsDirError::Empty);
    }

    let (system_temp_count, first_unsupported) = scan_variables(value);

    if let Some(unsupported) = first_unsupported {
        return Err(InstructionsDirError::UnsupportedVariable(unsupported));
    }

    match system_temp_count {
        0 => return Err(InstructionsDirError::MissingSystemTempVar),
        1 => {}
        _ => return Err(InstructionsDirError::RepeatedSystemTempVar),
    }

    if !value.starts_with(SYSTEM_TEMP_VAR) {
        return Err(InstructionsDirError::NotStartingWithSystemTemp);
    }

    let suffix = &value[SYSTEM_TEMP_VAR.len()..];

    // Reject path traversal: any segment equal to `..` is forbidden.
    for segment in suffix.split(['/', '\\']) {
        if segment == ".." {
            return Err(InstructionsDirError::ContainsTraversal);
        }
    }

    // Verify the resolved path stays within the system temporary directory.
    let tmp_dir = env::temp_dir();
    let suffix_rel = suffix.trim_start_matches(['/', '\\']);
    let resolved = if suffix_rel.is_empty() { tmp_dir.clone() } else { tmp_dir.join(suffix_rel) };
    let resolved_normalized = normalize_path_components(&resolved);
    let tmp_normalized = normalize_path_components(&tmp_dir);

    if !resolved_normalized.starts_with(&tmp_normalized) {
        return Err(InstructionsDirError::EscapesSystemTemp);
    }

    Ok(())
}

/// Validate the `instructions-dir` field from a parsed YAML value.
///
/// Handles `None` (absent key) and non-string YAML values before delegating to
/// [`validate_instructions_dir`] for string-level checks.
///
/// # Errors
/// Returns the first `InstructionsDirError` that describes the failure.
pub fn validate_instructions_dir_from_yaml(
    raw: Option<&noyalib::compat::serde_yaml::Value>,
) -> Result<(), InstructionsDirError> {
    match raw {
        None => Err(InstructionsDirError::Missing),
        Some(noyalib::compat::serde_yaml::Value::String(s)) => validate_instructions_dir(s),
        Some(_) => Err(InstructionsDirError::NotAString),
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Result of scanning `${...}` variable expressions in a string value.
///
/// The first element is the count of `${system-temp}` occurrences;
/// the second is the first unsupported variable expression found, if any.
type VariableScanResult = (usize, Option<String>);

/// Scan `value` for `${...}` variable expressions.
///
/// Returns `(system_temp_count, first_unsupported_expression)`.
fn scan_variables(value: &str) -> VariableScanResult {
    let mut system_temp_count: usize = 0;
    let mut first_unsupported: Option<String> = None;
    let mut remaining = value;

    while let Some(open) = remaining.find("${") {
        let after_open = &remaining[open + 2..];
        let Some(close_offset) = after_open.find('}') else {
            // Malformed — no closing brace; stop scanning.
            break;
        };
        let expr = &remaining[open..=(open + 2 + close_offset)];
        if expr == SYSTEM_TEMP_VAR {
            system_temp_count += 1;
        } else if first_unsupported.is_none() {
            first_unsupported = Some(expr.to_string());
        }
        remaining = &remaining[open + 2 + close_offset + 1..];
    }

    (system_temp_count, first_unsupported)
}

/// Normalize a path by resolving `.` components (without touching `..`,
/// which is rejected before this function is called).
fn normalize_path_components(path: &Path) -> std::path::PathBuf {
    let mut components: Vec<std::path::Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
