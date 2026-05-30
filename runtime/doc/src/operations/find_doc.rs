//! Plugin operation for finding a governed document by type and code.

use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::path::Path;
use walkdir::WalkDir;

use crate::internal::naming::parse_code_from_filename;
use crate::types::load_document_types_config;

/// Input for the `find_doc` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct FindDocInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// Reserved package selector for future package-aware lookup.
    pub package: String,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The numeric code of the document.
    pub code: u32,
}

/// Output for the `find_doc` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct FindDocOutput {
    /// The absolute path of the matching file.
    pub path: String,
    /// Reserved package output. Always empty for now.
    pub package: String,
    /// The current document content.
    pub content: String,
}

async fn find_doc(
    input: FindDocInput,
    output: &mut impl PluginSender<FindDocOutput>,
) -> RuntimeResult<()> {
    // Load config to verify the document type exists.
    let config = load_document_types_config(&input.root_dir).await?;

    if !config.document_types.contains_key(&input.doc_type) {
        return Err(runtime_core::RuntimeError::operation(format!(
            "unknown document type '{}'",
            input.doc_type
        )));
    }

    let search_base = input.root_dir.as_path().join("doc").join(&input.doc_type);

    if !search_base.exists() {
        return Err(runtime_core::RuntimeError::operation(format!(
            "document folder 'doc/{}' does not exist",
            input.doc_type
        )));
    }

    let mut found_path = None;

    for entry in WalkDir::new(&search_base)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let filename = entry.file_name().to_str().ok_or_else(|| {
            runtime_core::RuntimeError::operation("encountered a file with a non-UTF-8 name")
        })?;
        if !Path::new(filename).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
            continue;
        }

        if parse_code_from_filename(filename, &input.doc_type)
            .is_some_and(|code| code == input.code)
        {
            found_path = Some(entry.path().to_path_buf());
            break;
        }
    }

    if let Some(path) = found_path {
        // We want the absolute path as requested by RFC.
        // canonicalize() returns an absolute, normalized path.
        let abs_path = dunce::canonicalize(path).map_err(|e| {
            runtime_core::RuntimeError::operation(format!(
                "failed to canonicalize document path: {e}"
            ))
        })?;
        let content = std::fs::read_to_string(&abs_path).map_err(|e| {
            runtime_core::RuntimeError::operation(format!("failed to read document content: {e}"))
        })?;

        output
            .send(FindDocOutput {
                path: abs_path.to_string_lossy().to_string(),
                package: String::new(),
                content,
            })
            .await?;
        Ok(())
    } else {
        Err(runtime_core::RuntimeError::operation(format!(
            "no document of type '{}' with code {} found",
            input.doc_type, input.code
        )))
    }
}

declare_plugin_operations! {
    FindDocOp => find_doc(FindDocInput, FindDocOutput)
}

impl FindDocInput {
    /// Construct a `FindDocInput` with explicit fields.
    #[must_use]
    pub const fn new(root_dir: IoPath, package: String, doc_type: String, code: u32) -> Self {
        Self { root_dir, package, doc_type, code }
    }
}

impl FindDocOp {
    /// Construct a new `FindDocOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for FindDocOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "find_doc_test.rs"]
mod tests;
