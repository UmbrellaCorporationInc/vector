//! Plugin operation for replacing a governed document with complete content.

use runtime_core::{FlowOperation, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::fs;
use std::path::{Path, PathBuf};

use crate::operations::find_doc::{FindDocInput, FindDocOp, FindDocOutput};
use crate::operations::support::CapturingSender;
use crate::operations::validate::parse_frontmatter;

const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";

/// Input for the `replace_doc` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ReplaceDocInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// Optional synchronized package name for package-qualified lookup.
    ///
    /// When empty, the document is resolved within `root_dir`.
    /// When set, the document is resolved inside `.vector-database/packages/{package}/`.
    pub package: String,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The numeric code of the document to replace.
    pub code: u32,
    /// The complete replacement content.
    pub content: String,
}

/// Output for the `replace_doc` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ReplaceDocOutput {
    /// The absolute path of the replaced document.
    pub path: String,
    /// The final document content after the replacement write.
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedIdentity {
    id: String,
    doc_type: String,
    code: String,
    slug: String,
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

fn expected_identity_from_path(path: &str) -> RuntimeResult<ExpectedIdentity> {
    let stem = Path::new(path).file_stem().and_then(|stem| stem.to_str()).ok_or_else(|| {
        runtime_core::RuntimeError::operation("resolved document path has no stem")
    })?;

    let mut parts = stem.split('-');
    let Some(doc_type) = parts.next() else {
        return Err(runtime_core::RuntimeError::operation(
            "resolved document stem does not match expected pattern {{type}}-{{code}}-{{slug}}",
        ));
    };
    let Some(code) = parts.next() else {
        return Err(runtime_core::RuntimeError::operation(
            "resolved document stem does not match expected pattern {{type}}-{{code}}-{{slug}}",
        ));
    };
    let slug_parts: Vec<_> = parts.collect();
    if slug_parts.is_empty() {
        return Err(runtime_core::RuntimeError::operation(
            "resolved document stem does not match expected pattern {{type}}-{{code}}-{{slug}}",
        ));
    }
    let slug = slug_parts.join("-");

    Ok(ExpectedIdentity {
        id: stem.to_string(),
        doc_type: doc_type.to_string(),
        code: code.to_string(),
        slug,
    })
}

fn validate_replacement_content_identity(
    content: &str,
    expected: &ExpectedIdentity,
) -> RuntimeResult<()> {
    if content.as_bytes().starts_with(UTF8_BOM) {
        return Err(runtime_core::RuntimeError::operation(
            "replacement content contains a UTF-8 BOM (\\xEF\\xBB\\xBF); remove the BOM and retry",
        ));
    }

    let frontmatter = parse_frontmatter(content).ok_or_else(|| {
        runtime_core::RuntimeError::operation(
            "replacement content must include governed frontmatter with matching id, type, code, and slug",
        )
    })?;

    for (field, expected_value) in [
        ("id", expected.id.as_str()),
        ("type", expected.doc_type.as_str()),
        ("code", expected.code.as_str()),
        ("slug", expected.slug.as_str()),
    ] {
        let Some(actual) = frontmatter.get(field) else {
            return Err(runtime_core::RuntimeError::operation(format!(
                "replacement content is missing governed frontmatter field '{field}'"
            )));
        };

        if actual != expected_value {
            return Err(runtime_core::RuntimeError::operation(format!(
                "replacement content frontmatter field '{field}' must match the resolved document identity: expected '{expected_value}', found '{actual}'"
            )));
        }
    }

    Ok(())
}

async fn replace_doc(
    input: ReplaceDocInput,
    output: &mut impl PluginSender<ReplaceDocOutput>,
) -> RuntimeResult<()> {
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

    let doc_dir = canonical_governed_doc_dir(&input.root_dir, &input.package)?;
    let resolved_path = Path::new(&found.path);
    if !resolved_path.starts_with(&doc_dir) {
        return Err(runtime_core::RuntimeError::operation(
            "resolved document path is outside the doc/ directory",
        ));
    }

    let expected_identity = expected_identity_from_path(&found.path)?;
    validate_replacement_content_identity(&input.content, &expected_identity)?;

    fs::write(&found.path, input.content.as_bytes()).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("failed to write replacement document: {e}"))
    })?;

    output.send(ReplaceDocOutput { path: found.path, content: input.content }).await?;
    Ok(())
}

declare_plugin_operations! {
    ReplaceDocOp => replace_doc(ReplaceDocInput, ReplaceDocOutput)
}

impl ReplaceDocInput {
    /// Construct a `ReplaceDocInput` with explicit fields.
    #[must_use]
    pub const fn new(
        root_dir: IoPath,
        package: String,
        doc_type: String,
        code: u32,
        content: String,
    ) -> Self {
        Self { root_dir, package, doc_type, code, content }
    }
}

impl ReplaceDocOp {
    /// Construct a new `ReplaceDocOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for ReplaceDocOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "replace_doc_test.rs"]
mod tests;
