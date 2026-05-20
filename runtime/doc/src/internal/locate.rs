//! File locator by stem for governed documents.

use runtime_io::path::IoPath;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::types::load_document_types_config;

/// Error returned when no file matches the given stem.
///
/// # DTO(Internal error indicator — not a plugin operation contract)
#[non_exhaustive]
pub struct LocateError {
    /// The stem that was searched for.
    pub stem: String,
    /// The reason why the search failed.
    pub reason: String,
}

impl std::fmt::Debug for LocateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LocateError: stem='{}', reason='{}'", self.stem, self.reason)
    }
}

impl std::fmt::Display for LocateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cannot locate file with stem '{}': {}", self.stem, self.reason)
    }
}

impl std::error::Error for LocateError {}

/// Result of parsing a stem into its component parts.
///
/// # DTO(Internal type alias for tuple-based return)
pub type StemParts = (String, u32, String);

/// Parses a stem into its component parts: `doc_type`, code, and slug.
///
/// Expected stem pattern: `{doc_type}-{code}-{slug}`
/// Example: "rfc-00013-runtime-doc-validation" -> `doc_type="rfc"`, code=13, slug="runtime-doc-validation"
pub(super) fn parse_stem(stem: &str) -> Option<StemParts> {
    let parts: Vec<&str> = stem.split('-').collect();

    if parts.len() < 3 {
        return None;
    }

    let doc_type = parts[0].to_string();
    let code = parts[1].parse::<u32>().ok()?;
    let slug = parts[2..].join("-");

    Some((doc_type, code, slug))
}

/// Locates a governed document file by its stem (filename without extension).
///
/// Scans all subfolders of `doc/{type}/` recursively to find a file whose
/// name without extension matches the given stem.
///
/// # Errors
/// Returns `LocateError` if:
/// - The stem cannot be parsed as a governed document name
/// - The document type is not defined in the configuration
/// - No file matches the given stem
pub async fn locate_file_by_stem(stem: &str, root_dir: &IoPath) -> Result<PathBuf, LocateError> {
    let (doc_type, _code, _slug) = parse_stem(stem).ok_or_else(|| LocateError {
        stem: stem.to_string(),
        reason: "Stem does not match expected pattern {type}-{code}-{slug}".to_string(),
    })?;

    let config = load_document_types_config(root_dir).await.map_err(|error| LocateError {
        stem: stem.to_string(),
        reason: format!("Failed to load document types configuration: {error}"),
    })?;

    if !config.document_types.contains_key(&doc_type) {
        return Err(LocateError {
            stem: stem.to_string(),
            reason: format!("Unknown document type '{doc_type}'"),
        });
    }

    let search_base = root_dir.as_path().join("doc").join(&doc_type);

    if !search_base.exists() {
        return Err(LocateError {
            stem: stem.to_string(),
            reason: format!("Document type folder 'doc/{doc_type}' does not exist"),
        });
    }

    for entry in WalkDir::new(&search_base)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let entry_filename = entry.file_name().to_str();
        let Some(entry_filename) = entry_filename else {
            continue;
        };

        let entry_path = Path::new(entry_filename);
        if !entry_path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
            continue;
        }

        let entry_stem = entry_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        if entry_stem == stem {
            return entry.path().canonicalize().map_err(|_| LocateError {
                stem: stem.to_string(),
                reason: "Failed to canonicalize file path".to_string(),
            });
        }
    }

    Err(LocateError {
        stem: stem.to_string(),
        reason: "No file found with matching stem".to_string(),
    })
}

#[cfg(test)]
#[path = "locate_test.rs"]
mod tests;
