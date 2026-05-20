//! Plugin operation for creating a governed document with resolved prompt.

use runtime_core::{FlowOperation, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::fs;

use crate::internal::locate::locate_file_by_stem;
use crate::internal::slug::validate_slug;
use crate::operations::bootstrap_doc::{BootstrapDocInput, BootstrapDocOp, BootstrapDocOutput};
use crate::operations::support::CapturingSender;
use crate::types::load_document_types_config;

const DEFAULT_CREATE_DOC_PROMPT: &str = "prompts-00002-create-doc";

fn resolve_prompt_placeholders(
    prompt_content: &str,
    doc_type: &str,
    code: &str,
    slug: &str,
    file_path: &str,
) -> String {
    #[allow(clippy::literal_string_with_formatting_args)]
    let placeholder_doc_type = "#{doc-type}";
    #[allow(clippy::literal_string_with_formatting_args)]
    let placeholder_code = "#{code}";
    #[allow(clippy::literal_string_with_formatting_args)]
    let placeholder_slug = "#{slug}";
    #[allow(clippy::literal_string_with_formatting_args)]
    let placeholder_file_path = "#{file-path}";

    let result = prompt_content.replace(placeholder_doc_type, doc_type);
    let result = result.replace(placeholder_code, code);
    let result = result.replace(placeholder_slug, slug);
    result.replace(placeholder_file_path, file_path)
}

/// Input for the `create_doc` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CreateDocInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The optional category for category-based document types.
    pub category: Option<String>,
    /// The name/title for the document (used in template substitution).
    pub name: String,
    /// The slug for the new document.
    pub slug: String,
}

/// Output for the `create_doc` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CreateDocOutput {
    /// The absolute path where the document was created.
    pub path: String,
    /// The code that was assigned to the document.
    pub code: String,
    /// The resolved prompt string with placeholders substituted.
    pub prompt: String,
}

async fn create_doc(
    input: CreateDocInput,
    output: &mut impl PluginSender<CreateDocOutput>,
) -> RuntimeResult<()> {
    validate_slug(&input.slug).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("invalid slug '{}': {e}", input.slug))
    })?;

    let config = load_document_types_config(&input.root_dir).await?;

    let type_config = config.document_types.get(&input.doc_type).ok_or_else(|| {
        runtime_core::RuntimeError::operation(format!(
            "unknown document type '{}': not defined in document-types.yaml",
            input.doc_type
        ))
    })?;

    let prompt_template = if type_config.prompt.is_empty() {
        DEFAULT_CREATE_DOC_PROMPT.to_string()
    } else {
        type_config.prompt.clone()
    };

    let bootstrap_input = BootstrapDocInput {
        root_dir: input.root_dir.clone(),
        doc_type: input.doc_type.clone(),
        slug: input.slug.clone(),
        title: Some(input.name.clone()),
        category: input.category.clone(),
    };

    let mut bootstrap_sender = CapturingSender::<BootstrapDocOutput>::new();
    BootstrapDocOp.run(bootstrap_input, &mut bootstrap_sender).await?;

    let bootstrap_output = bootstrap_sender.into_output().ok_or_else(|| {
        runtime_core::RuntimeError::operation("document bootstrap produced no output")
    })?;

    let prompt_path =
        locate_file_by_stem(&prompt_template, &input.root_dir).await.map_err(|e| {
            runtime_core::RuntimeError::operation(format!(
                "prompt document '{prompt_template}' not found on disk: {e}"
            ))
        })?;

    let prompt_content = fs::read_to_string(prompt_path.as_path()).map_err(|e| {
        runtime_core::RuntimeError::operation(format!(
            "prompt document '{}' unreadable: {e}",
            prompt_path.display()
        ))
    })?;

    let resolved_prompt = resolve_prompt_placeholders(
        &prompt_content,
        &input.doc_type,
        &bootstrap_output.code,
        &input.slug,
        &bootstrap_output.path,
    );

    output
        .send(CreateDocOutput {
            path: bootstrap_output.path,
            code: bootstrap_output.code,
            prompt: resolved_prompt,
        })
        .await?;

    Ok(())
}

declare_plugin_operations! {
    CreateDocOp => create_doc(CreateDocInput, CreateDocOutput)
}

impl CreateDocInput {
    /// Construct a new `CreateDocInput`.
    #[must_use]
    pub const fn new(
        root_dir: IoPath,
        doc_type: String,
        category: Option<String>,
        name: String,
        slug: String,
    ) -> Self {
        Self { root_dir, doc_type, category, name, slug }
    }
}

impl CreateDocOp {
    /// Construct a new `CreateDocOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for CreateDocOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "create_doc_test.rs"]
mod tests;
