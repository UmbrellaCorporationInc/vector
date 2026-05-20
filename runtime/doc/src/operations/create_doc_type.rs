//! Plugin operation for creating a new document type with prompt resolution.

use runtime_core::{FlowOperation, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::fs;

use crate::internal::locate::locate_file_by_stem;
use crate::internal::slug::validate_slug;
use crate::operations::bootstrap_doc_type::{
    BootstrapDocTypeInput, BootstrapDocTypeOp, BootstrapDocTypeOutput,
};
use crate::operations::support::CapturingSender;
use crate::types::load_document_types_config;

fn resolve_create_doc_type_placeholders(
    prompt_content: &str,
    doc_type: &str,
    layout: &str,
) -> String {
    #[allow(clippy::literal_string_with_formatting_args)]
    let placeholder_doc_type = "#{doc-type}";
    #[allow(clippy::literal_string_with_formatting_args)]
    let placeholder_layout = "#{layout}";
    let result = prompt_content.replace(placeholder_doc_type, doc_type);
    result.replace(placeholder_layout, layout)
}

fn create_prompt_template_content(doc_type: &str) -> String {
    format!(
        "---\nid: doc-type-prompt-00001-{doc_type}\ntype: doc-type-prompt\ncode: \"00001\"\nslug: {doc_type}\ntitle: <Title>\ndescription: <One sentence describing this prompt template.>\ncategory: {doc_type}\ncreated: <YYYY-MM-DD>\nupdated: <YYYY-MM-DD>\ntags: []\n---\n"
    )
}

async fn ensure_prompt_template_exists(root_dir: &IoPath, doc_type: &str) -> RuntimeResult<String> {
    let template_dir = root_dir.join("doc").join("template").join("doc");
    let io_template_dir = IoPath::new(template_dir.as_path());
    runtime_io::file::create_dir_all(&io_template_dir).await.map_err(|_| {
        runtime_core::RuntimeError::operation("failed to create doc template directory")
    })?;

    let prompt_template_name = format!("doc-type-prompt-{doc_type}");
    let prompt_template_path = template_dir.join(format!("{prompt_template_name}.md"));
    let io_prompt_template_path = IoPath::new(prompt_template_path.as_path());

    let content = create_prompt_template_content(doc_type);
    runtime_io::write_file_text(&io_prompt_template_path, content).await.map_err(|_| {
        runtime_core::RuntimeError::operation("failed to write prompt template file")
    })?;

    Ok(prompt_template_name)
}

/// Input for the `create_doc_type` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CreateDocTypeInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// Human-readable purpose of the doc type.
    pub description: Option<String>,
    /// Searchable labels for the doc type.
    pub tags: Option<Vec<String>>,
    /// Optional prompt template identifier.
    pub prompt: Option<String>,
    /// The layout strategy: "status", "category", or "directory".
    pub layout: String,
    /// Width of the numeric code portion (e.g., 5 for "00001").
    pub code_width: u8,
    /// Allowed statuses for status-based types. Required when layout is "status".
    pub statuses: Option<Vec<String>>,
    /// Optional template name for this document type.
    pub template: Option<String>,
}

impl CreateDocTypeInput {
    /// Construct a new `CreateDocTypeInput`.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        root_dir: IoPath,
        doc_type: impl Into<String>,
        layout: impl Into<String>,
        code_width: u8,
        statuses: Option<Vec<String>>,
        description: Option<String>,
        tags: Option<Vec<String>>,
        template: Option<String>,
    ) -> Self {
        Self {
            root_dir,
            doc_type: doc_type.into(),
            layout: layout.into(),
            code_width,
            statuses,
            description,
            tags,
            template,
            prompt: None,
        }
    }
}

/// Output for the `create_doc_type` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CreateDocTypeOutput {
    /// The document type that was created.
    pub doc_type: String,
    /// The layout that was set ("status", "category", or "directory").
    pub layout: String,
    /// The resolved prompt string with placeholders substituted.
    pub prompt: String,
}

async fn create_doc_type(
    input: CreateDocTypeInput,
    output: &mut impl PluginSender<CreateDocTypeOutput>,
) -> RuntimeResult<()> {
    validate_slug(&input.doc_type).map_err(|e| {
        runtime_core::RuntimeError::operation(format!(
            "invalid document type name '{}': {e}",
            input.doc_type
        ))
    })?;

    let config = load_document_types_config(&input.root_dir).await?;

    let create_doc_type_prompt = &config.doc_type.prompt;
    let prompt_template_name =
        ensure_prompt_template_exists(&input.root_dir, &input.doc_type).await?;

    let bootstrap_input = BootstrapDocTypeInput {
        root_dir: input.root_dir.clone(),
        doc_type: input.doc_type.clone(),
        description: input.description.clone(),
        tags: input.tags.clone(),
        prompt: Some(prompt_template_name),
        layout: input.layout.clone(),
        code_width: input.code_width,
        statuses: input.statuses.clone(),
        template: input.template.clone(),
    };

    let mut bootstrap_sender = CapturingSender::<BootstrapDocTypeOutput>::new();
    BootstrapDocTypeOp.run(bootstrap_input, &mut bootstrap_sender).await?;

    let prompt_path =
        locate_file_by_stem(create_doc_type_prompt, &input.root_dir).await.map_err(|e| {
            runtime_core::RuntimeError::operation(format!(
                "prompt document '{create_doc_type_prompt}' not found on disk: {e}"
            ))
        })?;

    let prompt_content = fs::read_to_string(prompt_path.as_path()).map_err(|e| {
        runtime_core::RuntimeError::operation(format!(
            "prompt document '{}' unreadable: {e}",
            prompt_path.display()
        ))
    })?;

    let resolved_prompt =
        resolve_create_doc_type_placeholders(&prompt_content, &input.doc_type, &input.layout);

    output
        .send(CreateDocTypeOutput {
            doc_type: input.doc_type,
            layout: input.layout,
            prompt: resolved_prompt,
        })
        .await?;

    Ok(())
}

declare_plugin_operations! {
    CreateDocTypeOp => create_doc_type(CreateDocTypeInput, CreateDocTypeOutput)
}

impl CreateDocTypeOp {
    /// Construct a new `CreateDocTypeOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for CreateDocTypeOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "create_doc_type_test.rs"]
mod tests;
