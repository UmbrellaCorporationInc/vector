//! Plugin operation for bootstrapping a new document type.

use runtime_core::{
    FlowOperation, RuntimeError, RuntimeResult, declare_plugin_operations, plugin::PluginSender,
};
use runtime_io::path::IoPath;
use std::path::Path;

use super::create_document_rule::{CreateDocumentRuleOp, documentation_rule_input};
use super::support::DiscardSender;

use crate::internal::naming::format_code;
use crate::internal::slug::validate_slug;
use crate::types::load_document_types_config;

fn build_doc_type_config(
    description: Option<&str>,
    tags: Option<Vec<String>>,
    prompt: Option<&str>,
    layout: &str,
    code_width: u8,
    statuses: Option<Vec<String>>,
    template: Option<&str>,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("layout: {layout}"));

    lines.push(format!("code-width: {code_width}"));

    if let Some(d) = description {
        lines.push(format!("description: {d}"));
    }

    if let Some(tgs) = tags {
        if tgs.is_empty() {
            lines.push("tags: []".to_string());
        } else {
            lines.push("tags:".to_string());
            for t in tgs {
                lines.push(format!("  - {t}"));
            }
        }
    }

    if let Some(p) = prompt {
        lines.push(format!("prompt: {p}"));
    }

    if let Some(t) = template {
        lines.push(format!("template: {t}"));
    }

    if layout == "status"
        && let Some(sts) = statuses
        && !sts.is_empty()
    {
        lines.push("statuses:".to_string());
        for s in &sts {
            lines.push(format!("  - {s}"));
        }
        lines.push(format!("initial-status: {}", sts[0]));
    }

    lines.join("\n")
}

async fn save_document_types_config(
    root_dir: &IoPath,
    config_content: String,
) -> RuntimeResult<()> {
    let config_path = root_dir.join(".vector").join("document-types.yaml");
    let io_path = IoPath::new(config_path.as_path());
    runtime_io::write_file_text(&io_path, config_content)
        .await
        .map_err(|_| runtime_core::RuntimeError::operation("failed to write document-types.yaml"))
}

struct DocTypeUpdateParams<'a> {
    doc_type_name: &'a str,
    description: Option<&'a str>,
    tags: Option<Vec<String>>,
    prompt: Option<&'a str>,
    layout: &'a str,
    code_width: u8,
    statuses: Option<Vec<String>>,
    template: Option<&'a str>,
}

async fn update_document_types_yaml(
    root_dir: &IoPath,
    params: DocTypeUpdateParams<'_>,
) -> RuntimeResult<()> {
    let config_path = root_dir.join(".vector").join("document-types.yaml");

    let content = {
        let io_path = IoPath::new(config_path.as_path());
        runtime_io::read_file_text(&io_path).await.map_err(|error| {
            runtime_core::RuntimeError::operation(format!(
                "failed to read .vector/document-types.yaml: {error}"
            ))
        })?
    };

    let mut config: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|error| {
        runtime_core::RuntimeError::operation(format!(
            "failed to parse .vector/document-types.yaml: {error}"
        ))
    })?;

    let root = config.as_mapping_mut().ok_or_else(|| {
        runtime_core::RuntimeError::operation("document-types.yaml root is not a mapping")
    })?;
    let doc_types_key = serde_yaml::Value::String("document-types".to_string());
    let doc_types =
        root.get_mut(&doc_types_key).and_then(|v| v.as_mapping_mut()).ok_or_else(|| {
            RuntimeError::operation("document-types.yaml is missing 'document-types' mapping")
        })?;

    let type_config = build_doc_type_config(
        params.description,
        params.tags,
        params.prompt,
        params.layout,
        params.code_width,
        params.statuses,
        params.template,
    );
    let type_value: serde_yaml::Value = serde_yaml::from_str(&type_config).map_err(|_| {
        runtime_core::RuntimeError::operation(
            "failed to serialize new document type config as YAML",
        )
    })?;

    doc_types.insert(serde_yaml::Value::String(params.doc_type_name.to_string()), type_value);

    let new_content = serde_yaml::to_string(&config).map_err(|_| {
        runtime_core::RuntimeError::operation("failed to serialize updated document-types.yaml")
    })?;

    save_document_types_config(root_dir, new_content).await
}

fn create_template_content(doc_type: &str, code_width: u8) -> String {
    let code_placeholder = format_code(1, code_width);
    format!(
        "---\nid: {doc_type}-{code_placeholder}\ntype: {doc_type}\ncode: \"{code_placeholder}\"\nslug: <slug>\ntitle: <Title>\ndescription: <Description>\ncreated: <YYYY-MM-DD>\nupdated: <YYYY-MM-DD>\ntags: []\nrelated: []\n---\n\n# <Title>\n"
    )
}

async fn ensure_template_exists(
    root_dir: &IoPath,
    doc_type: &str,
    code_width: u8,
) -> RuntimeResult<()> {
    let template_dir = root_dir.join("doc").join("template").join("doc");
    let io_template_dir = IoPath::new(template_dir.as_path());
    runtime_io::file::create_dir_all(&io_template_dir).await.map_err(|_| {
        runtime_core::RuntimeError::operation("failed to create doc template directory")
    })?;

    let template_name = format!("template-{code_width:05}-{doc_type}");
    let template_path = template_dir.join(format!("{template_name}.md"));
    let io_template_path = IoPath::new(template_path.as_path());

    let content = create_template_content(doc_type, code_width);
    runtime_io::write_file_text(&io_template_path, content).await.map_err(|_| {
        runtime_core::RuntimeError::operation("failed to write doc type template file")
    })?;

    Ok(())
}

fn create_status_folders(
    root_dir: &Path,
    doc_type: &str,
    statuses: &[String],
) -> std::io::Result<()> {
    for status in statuses {
        let folder_path = root_dir.join("doc").join(doc_type).join(status);
        std::fs::create_dir_all(folder_path)?;
    }
    Ok(())
}

fn create_initial_folder(root_dir: &Path, doc_type: &str) -> std::io::Result<()> {
    let folder_path = root_dir.join("doc").join(doc_type);
    std::fs::create_dir_all(folder_path)
}

/// Input for the `bootstrap_doc_type` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct BootstrapDocTypeInput {
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

/// Output for the `bootstrap_doc_type` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct BootstrapDocTypeOutput {
    /// The document type that was created.
    pub doc_type: String,
    /// The layout that was set.
    pub layout: String,
}

async fn bootstrap_doc_type(
    input: BootstrapDocTypeInput,
    _output: &mut impl PluginSender<BootstrapDocTypeOutput>,
) -> RuntimeResult<()> {
    validate_slug(&input.doc_type).map_err(|e| {
        runtime_core::RuntimeError::operation(format!(
            "invalid document type name '{}': {e}",
            input.doc_type
        ))
    })?;

    if input.layout != "status" && input.layout != "category" && input.layout != "directory" {
        return Err(runtime_core::RuntimeError::operation(format!(
            "unsupported layout '{}': must be 'status', 'category', or 'directory'",
            input.layout
        )));
    }

    if input.prompt.as_deref().is_none() {
        return Err(runtime_core::RuntimeError::operation(format!(
            "missing prompt configuration for document type '{}'",
            input.doc_type
        )));
    }

    let root_path = input.root_dir.as_path();

    load_document_types_config(&input.root_dir).await?;

    if input.layout == "status" {
        let statuses = input.statuses.as_ref().ok_or_else(|| {
            runtime_core::RuntimeError::operation(
                "statuses are required for a status-based document type",
            )
        })?;
        if statuses.is_empty() {
            return Err(runtime_core::RuntimeError::operation(
                "at least one status is required for a status-based document type",
            ));
        }
    }

    if input.layout == "status" {
        if let Some(ref statuses) = input.statuses {
            create_status_folders(root_path, &input.doc_type, statuses).map_err(|e| {
                runtime_core::RuntimeError::operation(format!(
                    "failed to create status folders for '{}': {e}",
                    input.doc_type
                ))
            })?;
        }
    } else {
        // Both 'category' and 'directory' layouts just need the base folder doc/<type>/
        create_initial_folder(root_path, &input.doc_type).map_err(|e| {
            runtime_core::RuntimeError::operation(format!(
                "failed to create document folder for '{}': {e}",
                input.doc_type
            ))
        })?;
    }

    update_document_types_yaml(
        &input.root_dir,
        DocTypeUpdateParams {
            doc_type_name: &input.doc_type,
            description: input.description.as_deref(),
            tags: input.tags.clone(),
            prompt: input.prompt.as_deref(),
            layout: &input.layout,
            code_width: input.code_width,
            statuses: input.statuses.clone(),
            template: input.template.as_deref(),
        },
    )
    .await?;

    ensure_template_exists(&input.root_dir, &input.doc_type, input.code_width).await?;

    // Refresh documentation rule
    let rule_input = documentation_rule_input(input.root_dir.clone());
    let mut rule_sender = DiscardSender;
    CreateDocumentRuleOp.run(rule_input, &mut rule_sender).await?;

    Ok(())
}

declare_plugin_operations! {
    BootstrapDocTypeOp => bootstrap_doc_type(BootstrapDocTypeInput, BootstrapDocTypeOutput)
}

#[cfg(test)]
#[path = "bootstrap_doc_type_test.rs"]
mod tests;
