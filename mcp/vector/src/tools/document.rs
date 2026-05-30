//! MCP tool group for document capability domain.

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    schemars, tool, tool_handler, tool_router,
};
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_doc::operations::{
    CreateDocInput, CreateDocOp, CreateDocTypeInput, CreateDocTypeOp, FindDocInput, FindDocOp,
    PatchDocInput, PatchDocOp, ValidateInput, ValidateOp,
};
use runtime_io::path::IoPath;
use serde::Deserialize;

/// MCP-facing parameters for the `validate` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValidateParams {
    /// Absolute or relative path to the root directory of the project to validate.
    pub root_dir: String,
    /// When true, attempt to fix auto-correctable issues instead of only reporting them.
    #[serde(default)]
    pub fix: bool,
}

/// MCP-facing parameters for the `validate_fix` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValidateFixParams {
    /// Absolute or relative path to the root directory of the project to validate and fix.
    pub root_dir: String,
}

/// MCP-facing parameters for the `find_doc` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindDocParams {
    /// Reserved package selector for future package-aware lookup. Ignored by the implementation.
    #[serde(default)]
    pub package: String,
    /// Absolute or relative path to the root directory of the project.
    pub root_dir: String,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The numeric code of the document to locate.
    pub code: u32,
}

/// MCP-facing parameters for the `create_doc` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateDocParams {
    /// Absolute or relative path to the root directory of the project.
    pub root_dir: String,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The name/title for the new document (used in template substitution).
    pub name: String,
    /// The kebab-case slug for the new document.
    pub slug: String,
    /// The optional category for category-based document types.
    #[serde(default)]
    pub category: Option<String>,
}

/// MCP-facing parameters for the `create_doc_type` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateDocTypeParams {
    /// Absolute or relative path to the root directory of the project.
    pub root_dir: String,
    /// The document type identifier to create (e.g. "adr", "spec").
    pub doc_type: String,
    /// The layout strategy: "status", "category", or "directory".
    pub layout: String,
    /// Width of the numeric code portion (e.g., 5 for "00001").
    #[serde(rename = "code-width")]
    pub code_width: u8,
    /// Allowed statuses for status-based types. Required when layout is "status".
    #[serde(default)]
    pub statuses: Option<Vec<String>>,
    /// Human-readable purpose of the doc type.
    #[serde(default)]
    pub description: Option<String>,
    /// Searchable labels for the doc type.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Optional template name for this document type.
    #[serde(default)]
    pub template: Option<String>,
}

/// MCP-facing parameters for the `patch_doc` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PatchDocParams {
    /// Absolute or relative path to the root directory of the project.
    pub root_dir: String,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The numeric code of the document to patch.
    pub code: u32,
    /// The unified diff to apply to the document.
    pub git_diff: String,
}

/// MCP tool group for document operations.
///
/// Owns MCP tool definitions for the document capability domain.
/// Reusable execution logic lives in `runtime-doc`; this struct
/// is a thin MCP adapter only.
pub struct DocumentTools {
    tool_router: ToolRouter<Self>,
}

impl DocumentTools {
    /// Construct a new `DocumentTools` adapter.
    #[must_use]
    pub fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }
}

impl Default for DocumentTools {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs `ValidateOp` with the supplied input and formats the output into lines.
async fn run_validate(input: ValidateInput) -> Result<String, String> {
    let (_cancel, mut receiver) = PluginDispatcher::new(ValidateOp::new())
        .input(input)
        .build()
        .map_err(|e| format!("dispatcher build failed: {e}"))?;

    let mut lines: Vec<String> = Vec::new();

    loop {
        match receiver.recv().await {
            Ok(Some(output)) => {
                for error in &output.errors {
                    lines.push(format!("ERROR: {}: {}", error.path, error.error));
                }
                for warning in &output.warnings {
                    lines.push(format!("WARN: {warning}"));
                }
                for fix_result in &output.fixes {
                    lines.push(format!(
                        "FIXED: {}: {} ({})",
                        fix_result.path, fix_result.fix_type, fix_result.detail
                    ));
                }
                if output.valid && output.fixes.is_empty() {
                    lines.push("Validation passed with no issues.".to_string());
                }
            }
            Ok(None) => break,
            Err(e) => return Err(format!("validate failed: {e}")),
        }
    }

    Ok(lines.join("\n"))
}

#[tool_router]
impl DocumentTools {
    /// Validate documentation in the given project root against `document-types.yaml`.
    ///
    /// Executes `ValidateOp` through the standard dispatcher path.
    /// All validation logic lives in `runtime-doc`; this method only maps
    /// MCP params to the runtime input and formats the output for the caller.
    #[tool(
        description = "Validate governed documentation in the project root against document-types.yaml"
    )]
    async fn validate(
        &self,
        Parameters(ValidateParams { root_dir, fix }): Parameters<ValidateParams>,
    ) -> Result<String, String> {
        let input = ValidateInput::new(IoPath::new(root_dir), fix);
        run_validate(input).await
    }

    /// Validate and auto-fix documentation in the given project root.
    ///
    /// Executes `ValidateOp` with `fix: true` through the standard dispatcher path.
    /// Correctable issues are applied automatically; uncorrectable issues are still reported.
    #[tool(
        description = "Validate governed documentation and apply auto-fixes for correctable issues"
    )]
    async fn validate_fix(
        &self,
        Parameters(ValidateFixParams { root_dir }): Parameters<ValidateFixParams>,
    ) -> Result<String, String> {
        let input = ValidateInput::new(IoPath::new(root_dir), true);
        run_validate(input).await
    }

    /// Locate a governed document by type and numeric code.
    ///
    /// Executes `FindDocOp` through the standard dispatcher path.
    /// All lookup logic lives in `runtime-doc`; this method only maps
    /// MCP params to the runtime input and returns path, reserved package field, and document content.
    #[tool(
        description = "Locate a governed document by type and numeric code, returning its path, reserved package field, and document content"
    )]
    async fn find_doc(
        &self,
        Parameters(FindDocParams { root_dir, doc_type, code, package }): Parameters<FindDocParams>,
    ) -> Result<String, String> {
        let input = FindDocInput::new(IoPath::new(root_dir), package, doc_type, code);

        let (_cancel, mut receiver) = PluginDispatcher::new(FindDocOp::new())
            .input(input)
            .build()
            .map_err(|e| format!("dispatcher build failed: {e}"))?;

        match receiver.recv().await {
            Ok(Some(output)) => Ok(format!(
                "path: {}\npackage: {}\n\n{}",
                output.path, output.package, output.content
            )),
            Ok(None) => Err("document not found".to_string()),
            Err(e) => Err(format!("find_doc failed: {e}")),
        }
    }

    /// Create a new governed document and return the resolved authoring payload.
    ///
    /// Executes `CreateDocOp` through the standard dispatcher path.
    /// All creation logic, code assignment, and prompt resolution live in `runtime-doc`;
    /// this method only maps MCP params to the runtime input and formats the output for the caller.
    #[tool(
        description = "Create a new governed document and return its path, assigned code, and resolved authoring prompt"
    )]
    async fn create_doc_prompt(
        &self,
        Parameters(CreateDocParams { root_dir, doc_type, name, slug, category }): Parameters<
            CreateDocParams,
        >,
    ) -> Result<String, String> {
        let input = CreateDocInput::new(IoPath::new(root_dir), doc_type, category, name, slug);

        let (_cancel, mut receiver) = PluginDispatcher::new(CreateDocOp::new())
            .input(input)
            .build()
            .map_err(|e| format!("dispatcher build failed: {e}"))?;

        match receiver.recv().await {
            Ok(Some(output)) => {
                Ok(format!("path: {}\ncode: {}\n\n{}", output.path, output.code, output.prompt))
            }
            Ok(None) => Err("create_doc failed: operation completed with no output".to_string()),
            Err(e) => Err(format!("create_doc failed: {e}")),
        }
    }

    /// Create a new governed document type and return the resolved authoring payload.
    ///
    /// Executes `CreateDocTypeOp` through the standard dispatcher path.
    /// All creation logic, directory scaffolding, and prompt resolution live in `runtime-doc`;
    /// this method only maps MCP params to the runtime input and formats the output for the caller.
    #[tool(
        description = "Create a new governed document type and return the created type, selected layout, and resolved authoring prompt"
    )]
    async fn create_doc_type_prompt(
        &self,
        Parameters(CreateDocTypeParams {
            root_dir,
            doc_type,
            layout,
            code_width,
            statuses,
            description,
            tags,
            template,
        }): Parameters<CreateDocTypeParams>,
    ) -> Result<String, String> {
        let input = CreateDocTypeInput::new(
            IoPath::new(root_dir),
            doc_type,
            layout,
            code_width,
            statuses,
            description,
            tags,
            template,
        );

        let (_cancel, mut receiver) = PluginDispatcher::new(CreateDocTypeOp::new())
            .input(input)
            .build()
            .map_err(|e| format!("dispatcher build failed: {e}"))?;

        match receiver.recv().await {
            Ok(Some(output)) => Ok(format!(
                "doc-type: {}\nlayout: {}\n\n{}",
                output.doc_type, output.layout, output.prompt
            )),
            Ok(None) => {
                Err("create_doc_type failed: operation completed with no output".to_string())
            }
            Err(e) => Err(format!("create_doc_type failed: {e}")),
        }
    }

    /// Apply a unified diff to a governed document and return the final content.
    ///
    /// Executes `PatchDocOp` through the standard dispatcher path.
    /// All patching logic, path authorization, and encoding enforcement live in `runtime-doc`;
    /// this method only maps MCP params to the runtime input and returns the patched content.
    #[tool(
        description = "Apply a unified diff to a governed document and return the final patched content or a structured validation error"
    )]
    async fn patch_doc(
        &self,
        Parameters(PatchDocParams { root_dir, doc_type, code, git_diff }): Parameters<
            PatchDocParams,
        >,
    ) -> Result<String, String> {
        let input = PatchDocInput::new(IoPath::new(root_dir), doc_type, code, git_diff);

        let (_cancel, mut receiver) = PluginDispatcher::new(PatchDocOp::new())
            .input(input)
            .build()
            .map_err(|e| format!("dispatcher build failed: {e}"))?;

        match receiver.recv().await {
            Ok(Some(output)) => Ok(format!("path: {}\n\n{}", output.path, output.content)),
            Ok(None) => Err("patch_doc failed: operation completed with no output".to_string()),
            Err(e) => Err(format!("patch_doc failed: {e}")),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DocumentTools {}

#[cfg(test)]
#[path = "document_test.rs"]
mod tests;
