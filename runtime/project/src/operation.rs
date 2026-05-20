//! Plugin operation for project bootstrapping

use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;

/// A governed project asset.
///
/// # DTO(Plugin operation contracts use public fields for ergonomic data transfer)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ProjectAsset {
    /// Relative path inside the project (e.g. `doc/template/ai/ai-rule.md`).
    pub path: &'static str,
    /// The exact byte content of the asset embedded at compile time.
    pub content: &'static [u8],
}

/// The input contract for creating a project.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CreateProjectInput {
    /// The target root directory where the project will be created.
    pub target_dir: IoPath,
    /// The human-readable name of the project.
    pub project_name: String,
    /// Whether to force overwrite existing files.
    pub force: bool,
}

/// The output contract for creating a project.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CreateProjectOutput {
    /// Lines of log/status messages generated during project creation.
    pub message: String,
    /// Paths that were skipped because they already existed.
    pub skipped_files: Vec<String>,
}

/// Assets to be provisioned when bootstrapping a new project.
const BOOTSTRAP_ASSETS: &[ProjectAsset] = &[
    ProjectAsset {
        path: ".vector/document-types.yaml",
        content: include_bytes!("../assets/.vector/document-types.yaml"),
    },
    ProjectAsset {
        path: ".vector/agents.yaml",
        content: include_bytes!("../assets/.vector/agents.yaml"),
    },
    ProjectAsset {
        path: ".vector/language-rules.yaml",
        content: include_bytes!("../assets/.vector/language-rules.yaml"),
    },
    ProjectAsset {
        path: "doc/template/project/template-00002-spec.md",
        content: include_bytes!("../assets/doc/template/project/template-00002-spec.md"),
    },
    ProjectAsset {
        path: "doc/template/ai/template-00001-ai-rule.md",
        content: include_bytes!("../assets/doc/template/ai/template-00001-ai-rule.md"),
    },
    ProjectAsset {
        path: "doc/template/ai/template-00006-documentation.md",
        content: include_bytes!("../assets/doc/template/ai/template-00006-documentation.md"),
    },
    ProjectAsset {
        path: "doc/template/prompts/template-00003-prompts.md",
        content: include_bytes!("../assets/doc/template/prompts/template-00003-prompts.md"),
    },
    ProjectAsset { path: "CLAUDE.md", content: include_bytes!("../assets/CLAUDE.md") },
    ProjectAsset { path: "AGENTS.md", content: include_bytes!("../assets/AGENTS.md") },
    ProjectAsset { path: "GEMINI.md", content: include_bytes!("../assets/GEMINI.md") },
    ProjectAsset { path: ".editorconfig", content: include_bytes!("../assets/.editorconfig") },
    ProjectAsset { path: ".geminiignore", content: include_bytes!("../assets/.geminiignore") },
    ProjectAsset { path: ".gitattributes", content: include_bytes!("../assets/.gitattributes") },
    ProjectAsset { path: ".gitignore", content: include_bytes!("../assets/.gitignore") },
    ProjectAsset { path: ".mcp.json", content: include_bytes!("../assets/.mcp.json") },
    ProjectAsset { path: "opencode.json", content: include_bytes!("../assets/opencode.json") },
    ProjectAsset {
        path: ".codex/config.toml",
        content: include_bytes!("../assets/.codex/config.toml"),
    },
    ProjectAsset {
        path: ".gemini/settings.json",
        content: include_bytes!("../assets/.gemini/settings.json"),
    },
    ProjectAsset {
        path: ".gemini/antigravity-cli/settings.json",
        content: include_bytes!("../assets/.gemini/antigravity-cli/settings.json"),
    },
    ProjectAsset {
        path: ".gemini/antigravity-cli/mcp_config.json",
        content: include_bytes!("../assets/.gemini/antigravity-cli/mcp_config.json"),
    },
    ProjectAsset {
        path: ".vector/dashboards/project-status.yaml",
        content: include_bytes!("../assets/.vector/dashboards/project-status.yaml"),
    },
    ProjectAsset {
        path: ".vscode/settings.json",
        content: include_bytes!("../assets/.vscode/settings.json"),
    },
    ProjectAsset {
        path: ".claude/.gitkeep",
        content: include_bytes!("../assets/.claude/.gitkeep"),
    },
    ProjectAsset {
        path: ".claude/settings.local.json",
        content: include_bytes!("../assets/.claude/settings.local.json"),
    },
    ProjectAsset {
        path: ".agents/.gitkeep",
        content: include_bytes!("../assets/.agents/.gitkeep"),
    },
    ProjectAsset {
        path: "doc/ai-rule/active/ai-rule-00000-master-dispatcher.md",
        content: include_bytes!("../assets/doc/ai-rule/active/ai-rule-00000-master-dispatcher.md"),
    },
    ProjectAsset {
        path: "doc/ai-rule/active/ai-rule-00001-staff-engineer-expertise.md",
        content: include_bytes!(
            "../assets/doc/ai-rule/active/ai-rule-00001-staff-engineer-expertise.md"
        ),
    },
    ProjectAsset {
        path: "doc/ai-rule/active/ai-rule-00002-english-communication.md",
        content: include_bytes!(
            "../assets/doc/ai-rule/active/ai-rule-00002-english-communication.md"
        ),
    },
    ProjectAsset {
        path: "doc/ai-rule/active/ai-rule-00004-user-decision-validation.md",
        content: include_bytes!(
            "../assets/doc/ai-rule/active/ai-rule-00004-user-decision-validation.md"
        ),
    },
    ProjectAsset {
        path: "doc/template/project/template-00004-doc-type-template.md",
        content: include_bytes!(
            "../assets/doc/template/project/template-00004-doc-type-template.md"
        ),
    },
    ProjectAsset {
        path: "doc/template/project/template-00005-doc-type-prompt.md",
        content: include_bytes!("../assets/doc/template/project/template-00005-doc-type-prompt.md"),
    },
    ProjectAsset {
        path: "doc/prompts/doc-type/prompts-00001-create-doc-type.md",
        content: include_bytes!("../assets/doc/prompts/doc-type/prompts-00001-create-doc-type.md"),
    },
    ProjectAsset {
        path: "doc/prompts/authoring/prompts-00002-create-doc.md",
        content: include_bytes!("../assets/doc/prompts/authoring/prompts-00002-create-doc.md"),
    },
    ProjectAsset {
        path: "doc/prompts/authoring/prompts-00003-create-task.md",
        content: include_bytes!("../assets/doc/prompts/authoring/prompts-00003-create-task.md"),
    },
    ProjectAsset {
        path: "doc/prompts/actions/prompts-00004-execute-task-phase.md",
        content: include_bytes!(
            "../assets/doc/prompts/actions/prompts-00004-execute-task-phase.md"
        ),
    },
    ProjectAsset {
        path: "doc/prompts/actions/prompts-00007-validate-fix-repository-governance-flow.md",
        content: include_bytes!(
            "../assets/doc/prompts/actions/prompts-00007-validate-fix-repository-governance-flow.md"
        ),
    },
    ProjectAsset {
        path: "doc/prompts/form-actions/prompts-00005-create-document.md",
        content: include_bytes!(
            "../assets/doc/prompts/form-actions/prompts-00005-create-document.md"
        ),
    },
    ProjectAsset {
        path: "doc/prompts/form-actions/prompts-00006-update-document.md",
        content: include_bytes!(
            "../assets/doc/prompts/form-actions/prompts-00006-update-document.md"
        ),
    },
    ProjectAsset {
        path: "doc/prompts/quality-gate/prompts-00008-rust.md",
        content: include_bytes!("../assets/doc/prompts/quality-gate/prompts-00008-rust.md"),
    },
    ProjectAsset {
        path: "doc/prompts/quality-gate/prompts-00009-typescript.md",
        content: include_bytes!("../assets/doc/prompts/quality-gate/prompts-00009-typescript.md"),
    },
    ProjectAsset {
        path: "doc/form/form-00001-create-document.md",
        content: include_bytes!("../assets/doc/form/form-00001-create-document.md"),
    },
    ProjectAsset {
        path: "doc/template/project/template-00007-task.md",
        content: include_bytes!("../assets/doc/template/project/template-00007-task.md"),
    },
    ProjectAsset {
        path: "doc/template/project/template-00008-rfc.md",
        content: include_bytes!("../assets/doc/template/project/template-00008-rfc.md"),
    },
    ProjectAsset {
        path: "doc/template/project/template-00009-project-definition-template.md",
        content: include_bytes!(
            "../assets/doc/template/project/template-00009-project-definition-template.md"
        ),
    },
    ProjectAsset {
        path: "doc/template/project/template-00010-language-dependency-governance-template.md",
        content: include_bytes!(
            "../assets/doc/template/project/template-00010-language-dependency-governance-template.md"
        ),
    },
    ProjectAsset {
        path: "doc/template/project/template-00011-project-principles-template.md",
        content: include_bytes!(
            "../assets/doc/template/project/template-00011-project-principles-template.md"
        ),
    },
    ProjectAsset {
        path: "doc/template/project/template-00012-package-readme.md",
        content: include_bytes!("../assets/doc/template/project/template-00012-package-readme.md"),
    },
];

async fn create_project(
    input: CreateProjectInput,
    output: &mut impl PluginSender<CreateProjectOutput>,
) -> RuntimeResult<()> {
    let mut skipped_files = Vec::new();

    for asset in BOOTSTRAP_ASSETS {
        let target_path = input.target_dir.join(asset.path);

        if target_path.as_path().exists() {
            skipped_files.push(asset.path.to_string());
            continue;
        }

        runtime_io::write_file_bytes(&target_path, asset.content.to_vec()).await.map_err(|e| {
            runtime_core::RuntimeError::operation(format!(
                "failed to write project asset '{}': {e}",
                asset.path
            ))
        })?;
    }

    output
        .send(CreateProjectOutput {
            message: format!(
                "Project {} initialized at {}. {} files skipped.",
                input.project_name,
                input.target_dir.as_path().display(),
                skipped_files.len()
            ),
            skipped_files,
        })
        .await?;

    Ok(())
}

declare_plugin_operations! {
    /// Operation to provision the governed project skeleton.
    CreateProjectOp => create_project(CreateProjectInput, CreateProjectOutput)
}

#[cfg(test)]
#[path = "operation_test.rs"]
mod tests;
