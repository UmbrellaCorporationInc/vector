//! Plugin operation for bootstrapping a governed document.

use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::path::Path;

use crate::internal::naming::format_code;
use crate::internal::next_code::next_code_for;
use crate::internal::slug::validate_slug;
use crate::types::{DocumentTypeConfig, load_document_types_config};

fn get_status_folder(type_config: &DocumentTypeConfig) -> Option<&str> {
    type_config.initial_status.as_deref()
}

fn get_category_folder(category: Option<&str>) -> Option<String> {
    let cat = category?;
    Some(cat.to_string())
}

fn derive_target_path(
    root_dir: &Path,
    type_config: &DocumentTypeConfig,
    doc_type: &str,
    code: u32,
    slug: &str,
    category: Option<&str>,
) -> Option<IoPath> {
    let code_str = format_code(code, type_config.code_width);
    let file_name = format!("{doc_type}-{code_str}-{slug}.md");
    let base = root_dir.join("doc").join(doc_type);

    let final_path = if type_config.is_directory_based() {
        base.join(file_name)
    } else if type_config.is_category_based() {
        let folder = get_category_folder(category)?;
        base.join(folder).join(file_name)
    } else {
        // Status based
        let folder = get_status_folder(type_config)?;
        base.join(folder).join(file_name)
    };

    Some(IoPath::new(final_path))
}

fn find_template_path(root_dir: &Path, template_name: &str) -> Option<IoPath> {
    let doc_template = root_dir.join("doc").join("template");
    if !doc_template.exists() {
        return None;
    }
    for entry in std::fs::read_dir(&doc_template).ok()? {
        let entry = entry.ok()?;
        if entry.file_type().ok()?.is_dir() {
            let template_path = entry.path().join(format!("{template_name}.md"));
            if template_path.exists() {
                return Some(IoPath::new(template_path));
            }
        }
    }
    None
}

fn get_template_content(root_dir: &Path, type_config: &DocumentTypeConfig) -> Option<String> {
    let template_name = type_config.template.as_ref()?;
    let template_path = find_template_path(root_dir, template_name)?;
    std::fs::read_to_string(template_path.as_path()).ok()
}

fn create_frontmatter_only_template(doc_type: &str, code: &str, slug: &str, title: &str) -> String {
    format!(
        "---\nid: {doc_type}-{code}-{slug}\ntype: {doc_type}\ncode: \"{code}\"\nslug: {slug}\ntitle: {title}\ndescription: <Description>\ncreated: <YYYY-MM-DD>\nupdated: <YYYY-MM-DD>\ntags: []\nrelated: []\n---\n\n# {title}\n"
    )
}

/// Input for the `bootstrap_doc` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct BootstrapDocInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The slug for the new document.
    pub slug: String,
    /// Optional title for the document. If not provided, uses placeholder.
    pub title: Option<String>,
    /// Optional category override for category-based document types.
    pub category: Option<String>,
}

/// Output for the `bootstrap_doc` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct BootstrapDocOutput {
    /// The absolute path where the document was created.
    pub path: String,
    /// The code that was assigned to the document.
    pub code: String,
}

async fn bootstrap_doc(
    input: BootstrapDocInput,
    output: &mut impl PluginSender<BootstrapDocOutput>,
) -> RuntimeResult<()> {
    validate_slug(&input.slug).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("invalid slug '{}': {e}", input.slug))
    })?;

    let config = load_document_types_config(&input.root_dir).await?;

    let type_config = config.document_types.get(&input.doc_type).ok_or_else(|| {
        runtime_core::RuntimeError::operation(format!("unknown document type '{}'", input.doc_type))
    })?;

    let next_code_result = next_code_for(&input.doc_type, &input.root_dir).await.map_err(|_| {
        runtime_core::RuntimeError::operation(format!(
            "failed to determine next code for document type '{}'",
            input.doc_type
        ))
    })?;
    let code = next_code_result.next_code;

    let target_path = derive_target_path(
        input.root_dir.as_path(),
        type_config,
        &input.doc_type,
        code,
        &input.slug,
        input.category.as_deref(),
    )
    .ok_or_else(|| {
        runtime_core::RuntimeError::operation(format!(
            "failed to derive target path for document type '{}': missing category or status folder",
            input.doc_type
        ))
    })?;

    let code_str = format_code(code, type_config.code_width);
    let title = input.title.as_deref().unwrap_or("<Title>");

    let content = if let Some(template_content) =
        get_template_content(input.root_dir.as_path(), type_config)
    {
        template_content
    } else {
        create_frontmatter_only_template(&input.doc_type, &code_str, &input.slug, title)
    };

    if let Some(parent) = target_path.as_path().parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            runtime_core::RuntimeError::operation(format!(
                "failed to create document directory: {e}"
            ))
        })?;
    }

    std::fs::write(target_path.as_path(), content).map_err(|e| {
        runtime_core::RuntimeError::operation(format!(
            "failed to write document to '{}': {e}",
            target_path.as_path().display()
        ))
    })?;

    output
        .send(BootstrapDocOutput {
            path: target_path.as_path().to_string_lossy().to_string(),
            code: code_str,
        })
        .await?;

    Ok(())
}

declare_plugin_operations! {
    BootstrapDocOp => bootstrap_doc(BootstrapDocInput, BootstrapDocOutput)
}

#[cfg(test)]
#[path = "bootstrap_doc_test.rs"]
mod tests;
