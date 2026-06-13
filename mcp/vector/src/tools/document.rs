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
    PatchDocFormat, PatchDocInput, PatchDocOp, ReplaceDocInput, ReplaceDocOp, ValidateInput,
    ValidateOp,
};
use runtime_io::path::IoPath;
use serde::{Deserialize, Serialize};

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
    /// Optional synchronized package name for package-qualified lookup.
    ///
    /// When empty, the document is resolved within the active workspace at `root_dir`.
    /// When set to a package name (e.g. `"my-pkg"`), the document is resolved inside
    /// `.vector-database/packages/{package}/`.  Returns an actionable error when the
    /// package is unknown or has not been synchronized.
    ///
    /// Wikilinks in governed documents use the format `[[package/doc-id]]`; when you
    /// encounter one, split on the first `/` and pass the left side here.
    #[serde(default)]
    pub package: String,
    /// Absolute or relative path to the root directory of the project.
    pub root_dir: String,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The numeric code of the document to locate.
    pub code: u32,
}

/// Response returned by the `find_doc` tool.
///
/// # DTO(MCP protocol output serialized at the adapter boundary; serde serialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Serialize, Deserialize)]
pub struct FindDocResponse {
    /// The absolute path to the located document.
    pub path: String,
    /// The synchronized package name, or empty string.
    pub package: String,
    /// The document content.
    pub content: String,
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
    /// Optional synchronized package name for package-qualified lookup.
    ///
    /// When empty, the document is resolved within the active workspace at `root_dir`.
    /// When set, the document is resolved inside `.vector-database/packages/{package}/`.
    #[serde(default)]
    pub package: String,
    /// Absolute or relative path to the root directory of the project.
    pub root_dir: String,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The numeric code of the document to patch.
    pub code: u32,
    /// Patch format. Supported values are `unified` and `apply_patch`.
    ///
    /// **Recommended for agent-authored edits:** omit this field and send an `apply_patch`-style
    /// payload in `patch`. When omitted, `format` defaults to `apply_patch`, which is the safer
    /// choice because it does not require numeric hunk headers or line-count arithmetic.
    ///
    /// Use `format: "unified"` only when you already have a source-control-native diff.
    /// When `format` is `unified`, hunk headers must use 1-based line indices —
    /// the first document line is line 1, not line 0.
    #[serde(default)]
    pub format: Option<String>,
    /// The patch payload to apply to the document.
    ///
    /// **Recommended for agent-authored edits:** omit `format` and use an `apply_patch`-style
    /// payload here. This is the safer default because it does not require numeric hunk headers.
    ///
    /// When `format` is `unified`, the payload must be a standard unified diff. Use the full
    /// `@@ -start,count +start,count @@` hunk header form with 1-based line indices. The path
    /// in `---` and `+++` lines must match the document path resolved from `doc_type`, `code`,
    /// and optional `package`; call `find_doc` to obtain that path before constructing the diff.
    /// Use `format: "unified"` only when you already have a source-control-native diff.
    /// When `format` is omitted or `apply_patch`, provide an `apply_patch`-style payload.
    #[serde(default)]
    pub patch: Option<String>,
    /// Deprecated alias for a unified-diff patch payload.
    ///
    /// If this field is used, it is interpreted as `format: "unified"`.
    #[serde(default)]
    pub git_diff: Option<String>,
}

/// MCP-facing parameters for the `replace_doc` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReplaceDocParams {
    /// Optional synchronized package name for package-qualified lookup.
    ///
    /// When empty, the document is resolved within the active workspace at `root_dir`.
    /// When set, the document is resolved inside `.vector-database/packages/{package}/`.
    #[serde(default)]
    pub package: String,
    /// Absolute or relative path to the root directory of the project.
    pub root_dir: String,
    /// The document type identifier (e.g. "rfc", "task").
    pub doc_type: String,
    /// The numeric code of the document to replace.
    pub code: u32,
    /// The complete replacement document content.
    ///
    /// Must be valid UTF-8 without a BOM. The governed front matter identity fields
    /// (`id`, `type`, `code`, `slug`) must match the resolved document.
    pub content: String,
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

fn patch_doc_input_from_params(
    PatchDocParams { package, root_dir, doc_type, code, format, patch, git_diff }: PatchDocParams,
) -> Result<PatchDocInput, String> {
    let root_dir = IoPath::new(root_dir);

    if let Some(git_diff) = git_diff {
        if patch.is_some() {
            return Err(
                "patch_doc accepts either 'patch' or deprecated 'git_diff', not both".to_string()
            );
        }

        if let Some(format) = format
            && format != PatchDocFormat::Unified.as_str()
        {
            return Err(format!(
                "git_diff is a deprecated alias for format: \"unified\"; received format: \
                 \"{format}\". Use 'patch' for other formats."
            ));
        }

        return Ok(PatchDocInput::with_format(
            root_dir,
            package,
            doc_type,
            code,
            PatchDocFormat::Unified,
            git_diff,
        ));
    }

    let patch = patch.ok_or_else(|| {
        "patch_doc requires 'patch' unless deprecated 'git_diff' is provided".to_string()
    })?;
    let format = PatchDocFormat::parse_optional(format.as_deref())?;

    Ok(PatchDocInput::with_format(root_dir, package, doc_type, code, format, patch))
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
    /// MCP params to the runtime input and returns the absolute path, package name, and document content.
    ///
    /// Governed documents use the identifier format `{doc_type}-{code}-{slug}` (e.g. `rfc-00013-my-rfc`).
    /// Wikilinks in governed documents may carry a package-qualified form `{package}/{doc_type}-{code}-{slug}`
    /// (e.g. `my-pkg/rfc-00013-my-rfc`).  To resolve a package-qualified wikilink, split on the first `/`,
    /// use the left side as `package`, and parse the right side to extract `doc_type` and `code`.
    /// When `package` is set, the document is resolved against the synchronized package at
    /// `.vector-database/packages/{package}/` instead of the active workspace.
    #[tool(
        description = "Locate a governed document by type and numeric code, returning its absolute path, package name, and content. Identifiers follow the form `{doc_type}-{code}-{slug}` (e.g. `rfc-00013-my-rfc`); package-qualified wikilinks use `{package}/{doc_type}-{code}-{slug}` — split on `/` and pass the left side as `package`. Leave `package` empty to resolve from the active workspace."
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
            Ok(Some(output)) => {
                let response = FindDocResponse {
                    path: output.path,
                    package: output.package,
                    content: output.content,
                };
                serde_json::to_string(&response)
                    .map_err(|e| format!("failed to serialize find_doc response: {e}"))
            }
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

    /// Apply a patch to a governed document and return the final content.
    ///
    /// Executes `PatchDocOp` through the standard dispatcher path.
    /// All patching logic, path authorization, and encoding enforcement live in `runtime-doc`;
    /// this method only maps MCP params to the runtime input and returns the patched content.
    #[tool(
        description = "Apply a patch to a governed document and return the final patched content or a structured validation error. Recommended for agent-authored edits: omit `format` and send an `apply_patch`-style payload in `patch` — this is the safer default because it does not require numeric hunk headers or line-count arithmetic. Use `format: \"unified\"` only when you already have a source-control-native diff; when doing so, hunk headers must use 1-based line indices (the first document line is line 1) and the full `@@ -start,count +start,count @@` form, and the path in `---` and `+++` lines must be the path resolved by `doc_type`, `code`, and optional `package`. `git_diff` is a deprecated alias for `format: \"unified\"`."
    )]
    async fn patch_doc(
        &self,
        Parameters(params): Parameters<PatchDocParams>,
    ) -> Result<String, String> {
        let input = patch_doc_input_from_params(params)?;

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

    /// Replace a governed document with complete content and return the resolved path and final content.
    ///
    /// Executes `ReplaceDocOp` through the standard dispatcher path.
    /// All path authorization, identity validation, and write rules live in `runtime-doc`;
    /// this method only maps MCP params to the runtime input and returns the result.
    ///
    /// Use `replace_doc` after `create_doc_prompt` to write a fully authored document without
    /// generating a patch against the placeholder template.
    #[tool(
        description = "Replace a governed document with complete content and return the resolved path and final content. Resolves the target from `doc_type`, `code`, and optional `package`; callers do not provide a write path. The replacement `content` must be valid UTF-8 without a BOM and must preserve the governed front matter identity (`id`, `type`, `code`, `slug`). Use `replace_doc` after `create_doc_prompt` to author the full document without generating a patch."
    )]
    async fn replace_doc(
        &self,
        Parameters(ReplaceDocParams { package, root_dir, doc_type, code, content }): Parameters<
            ReplaceDocParams,
        >,
    ) -> Result<String, String> {
        let input = ReplaceDocInput::new(IoPath::new(root_dir), package, doc_type, code, content);

        let (_cancel, mut receiver) = PluginDispatcher::new(ReplaceDocOp::new())
            .input(input)
            .build()
            .map_err(|e| format!("dispatcher build failed: {e}"))?;

        match receiver.recv().await {
            Ok(Some(output)) => Ok(format!("path: {}\n\n{}", output.path, output.content)),
            Ok(None) => Err("replace_doc failed: operation completed with no output".to_string()),
            Err(e) => Err(format!("replace_doc failed: {e}")),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DocumentTools {}

#[cfg(test)]
#[path = "document_test.rs"]
mod tests;
