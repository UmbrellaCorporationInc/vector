//! Next code resolver for document types.

use runtime_core::{RuntimeError, RuntimeResult};
use runtime_io::path::IoPath;
use std::path::Path;
use walkdir::WalkDir;

use crate::internal::naming::{is_governed_file, parse_code_from_filename};
use crate::types::{DocumentTypesConfig, load_document_types_config};

/// Result of the next code resolution.
///
/// # DTO(Internal utility result — not a plugin operation contract)
#[non_exhaustive]
pub struct NextCodeResult {
    /// The next available code for the document type.
    pub next_code: u32,
}

/// Error returned when a file in the doc folder does not match the governed pattern.
///
/// # DTO(Internal error indicator — not a plugin operation contract)
#[non_exhaustive]
pub struct MalformedFileError {
    /// Path to the malformed file.
    pub path: String,
}

fn find_highest_code_in_folder(
    folder: &Path,
    doc_type: &str,
) -> Result<Option<u32>, MalformedFileError> {
    let mut highest = None;

    for entry in WalkDir::new(folder)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let filename = entry.file_name().to_str().ok_or_else(|| MalformedFileError {
            path: entry.path().to_string_lossy().to_string(),
        })?;
        if !Path::new(filename).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
            continue;
        }

        if !is_governed_file(filename, doc_type) {
            return Err(MalformedFileError { path: entry.path().to_string_lossy().to_string() });
        }

        let Some(code) = parse_code_from_filename(filename, doc_type) else {
            return Err(MalformedFileError { path: entry.path().to_string_lossy().to_string() });
        };

        highest = Some(highest.map_or(code, |h| std::cmp::max(h, code)));
    }

    Ok(highest)
}

fn get_type_config<'a>(
    doc_config: &'a DocumentTypesConfig,
    doc_type: &str,
) -> RuntimeResult<&'a crate::types::DocumentTypeConfig> {
    doc_config
        .document_types
        .get(doc_type)
        .ok_or_else(|| RuntimeError::operation(format!("unknown document type '{doc_type}'")))
}

/// Resolve the next available code for a document type.
///
/// Scans all `.md` files under `doc/{doc_type}/` recursively (all subfolders —
/// status and category alike), parses the numeric code from each file name using
/// the `{type}-{code}-{slug}.md` pattern, and returns `highest + 1`.
/// Returns `1` when no files exist yet for that type.
///
/// # Errors
/// Returns `RuntimeError::Operation` if:
/// - The document-types.yaml cannot be loaded or parsed
/// - The document type is not defined in the configuration
/// - A file in the folder tree does not match the governed filename pattern
pub async fn next_code_for(doc_type: &str, root_dir: &IoPath) -> RuntimeResult<NextCodeResult> {
    let config = load_document_types_config(root_dir).await?;

    let _type_config = get_type_config(&config, doc_type)?;

    let search_base = root_dir.as_path().join("doc").join(doc_type);

    if !search_base.exists() {
        return Ok(NextCodeResult { next_code: 1 });
    }

    let highest = find_highest_code_in_folder(&search_base, doc_type).map_err(|e| {
        RuntimeError::operation(format!(
            "malformed file found in doc/{doc_type}/: {} does not match the governed filename pattern",
            e.path
        ))
    })?;

    Ok(NextCodeResult { next_code: highest.map_or(1, |h| h + 1) })
}

#[cfg(test)]
#[path = "next_code_test.rs"]
mod tests;
