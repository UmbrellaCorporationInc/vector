//! Plugin operation for generating the documentation rule.

use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::fs;

use crate::internal::locate::locate_file_by_stem;
use crate::types::load_document_types_config;

const DOCUMENTATION_RULE_TEMPLATE_STEM: &str = "template-00006-documentation";
const DOCUMENTATION_RULE_OUTPUT_SUBPATH: &str = "doc/ai-rule/active/ai-rule-00003-documentation.md";

/// Constructs the standard [`CreateDocumentRuleInput`] for the documentation rule.
///
/// Both `bootstrap_doc_type` and `project_extension_setup` target the same template
/// and output path, so this function is the single source of truth for those values.
#[must_use]
pub fn documentation_rule_input(root_dir: IoPath) -> CreateDocumentRuleInput {
    let output_path = root_dir.join(DOCUMENTATION_RULE_OUTPUT_SUBPATH);
    CreateDocumentRuleInput {
        root_dir,
        output_path,
        template_stem: DOCUMENTATION_RULE_TEMPLATE_STEM.to_string(),
    }
}

/// Input for the `create_document_rule` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CreateDocumentRuleInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// The destination path for the generated rule.
    pub output_path: IoPath,
    /// The template stem to use for generation.
    pub template_stem: String,
}

/// Output for the `create_document_rule` operation.
///
/// # DTO(Plugin operation input/output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CreateDocumentRuleOutput {
    /// The path where the rule was written.
    pub written_path: IoPath,
}

async fn create_document_rule(
    input: CreateDocumentRuleInput,
    output: &mut impl PluginSender<CreateDocumentRuleOutput>,
) -> RuntimeResult<()> {
    let config = load_document_types_config(&input.root_dir).await?;

    let template_path =
        locate_file_by_stem(&input.template_stem, &input.root_dir).await.map_err(|e| {
            runtime_core::RuntimeError::operation(format!(
                "documentation rule template '{}' not found: {e}",
                input.template_stem
            ))
        })?;

    let template_content = fs::read_to_string(template_path.as_path()).map_err(|e| {
        runtime_core::RuntimeError::operation(format!(
            "failed to read documentation rule template '{}': {e}",
            template_path.display()
        ))
    })?;

    let mut types_block = Vec::new();

    // Sort keys to have a stable order since we use HashMap
    let mut keys: Vec<_> = config.document_types.keys().collect();
    keys.sort();

    for key in keys {
        let type_config = &config.document_types[key];
        let tags = type_config.tags.as_ref().map_or_else(
            || "-".to_string(),
            |t| if t.is_empty() { "-".to_string() } else { t.join(", ") },
        );

        let description = type_config.description.as_deref().unwrap_or("-");

        types_block.push(format!(
            "**document type:** {key}\n**tags:** {tags}\n**description:** {description}"
        ));
    }

    let types_replacement = types_block.join("\n\n");
    let mut new_content = template_content.replace("#{types}", &types_replacement);

    // Try to get created date from existing file
    let existing_created = if input.output_path.as_path().exists() {
        let content = fs::read_to_string(input.output_path.as_path()).ok();
        content.and_then(|c| {
            c.lines()
                .find(|l| l.trim().starts_with("created:"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
    } else {
        None
    };

    // Update frontmatter dates
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if let Some(start) = new_content.find("---") {
        let rest = &new_content[start + 3..];
        if let Some(end) = rest.find("---") {
            let frontmatter = &rest[..end];
            let mut lines: Vec<String> = frontmatter.lines().map(String::from).collect();

            let mut created_found = false;
            let mut updated_found = false;

            for line in &mut lines {
                if line.trim().starts_with("created:") {
                    created_found = true;
                    if let Some(ref existing) = existing_created {
                        *line = format!("created: {existing}");
                    } else if line.contains("<YYYY-MM-DD>") || line.contains("TODO") {
                        *line = format!("created: {today}");
                    }
                } else if line.trim().starts_with("updated:") {
                    updated_found = true;
                    *line = format!("updated: {today}");
                }
            }

            if !created_found {
                lines.push(format!(
                    "created: {}",
                    existing_created.unwrap_or_else(|| today.clone())
                ));
            }
            if !updated_found {
                lines.push(format!("updated: {today}"));
            }

            let new_frontmatter = lines.join("\n");
            let before = &new_content[..start + 3];
            let after = &new_content[start + 3 + end..];
            new_content = format!("{before}{new_frontmatter}\n{after}");
        }
    }

    if let Some(parent) = input.output_path.as_path().parent() {
        fs::create_dir_all(parent).map_err(|e| {
            runtime_core::RuntimeError::operation(format!(
                "failed to create output directory for documentation rule: {e}"
            ))
        })?;
    }

    fs::write(input.output_path.as_path(), new_content).map_err(|e| {
        runtime_core::RuntimeError::operation(format!(
            "failed to write documentation rule to '{}': {e}",
            input.output_path.as_path().display()
        ))
    })?;

    output.send(CreateDocumentRuleOutput { written_path: input.output_path }).await?;

    Ok(())
}

declare_plugin_operations! {
    CreateDocumentRuleOp => create_document_rule(CreateDocumentRuleInput, CreateDocumentRuleOutput)
}

#[cfg(test)]
#[path = "create_document_rule_test.rs"]
mod tests;
