//! MCP tool group for language capability domain.

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    schemars, tool, tool_handler, tool_router,
};
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_io::path::IoPath;
use runtime_language::{BestPracticesInput, BestPracticesOp, QualityGateInput, QualityGateOp};
use serde::Deserialize;

/// MCP-facing parameters for the `language-quality-gate` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LanguageQualityGateParams {
    /// Absolute or relative path to the root directory of the project.
    pub root_dir: String,
    /// Ordered language identifiers whose quality-gate prompts should be resolved.
    pub languages: Vec<String>,
}

/// MCP-facing parameters for the `language-best-practices` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LanguageBestPracticesParams {
    /// Absolute or relative path to the root directory of the project.
    pub root_dir: String,
    /// Ordered language identifiers whose best-practices prompts should be resolved.
    pub languages: Vec<String>,
}

/// MCP tool group for language operations.
///
/// Owns MCP tool definitions for the language capability domain.
/// Reusable execution logic lives in `runtime-language`; this struct
/// is a thin MCP adapter only.
pub struct LanguageTools {
    tool_router: ToolRouter<Self>,
}

impl LanguageTools {
    /// Construct a new `LanguageTools` adapter.
    #[must_use]
    pub fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }
}

impl Default for LanguageTools {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl LanguageTools {
    /// Resolve and concatenate governed language quality-gate prompts.
    ///
    /// Executes `QualityGateOp` through the standard dispatcher path.
    /// Prompt lookup, frontmatter stripping, and concatenation remain
    /// inside `runtime-language`; this method only adapts MCP params.
    #[tool(
        name = "language-quality-gate",
        description = "Resolve governed quality-gate prompts for a language list and return the concatenated prompt body"
    )]
    async fn language_quality_gate(
        &self,
        Parameters(LanguageQualityGateParams { root_dir, languages }): Parameters<
            LanguageQualityGateParams,
        >,
    ) -> Result<String, String> {
        let input = QualityGateInput::new(IoPath::new(root_dir), languages);

        let (_cancel, mut receiver) = PluginDispatcher::new(QualityGateOp::new())
            .input(input)
            .build()
            .map_err(|error| format!("dispatcher build failed: {error}"))?;

        match receiver.recv().await {
            Ok(Some(output)) => Ok(output.prompt),
            Ok(None) => {
                Err("language-quality-gate failed: operation completed with no output".to_string())
            }
            Err(error) => Err(format!("language-quality-gate failed: {error}")),
        }
    }

    /// Resolve and concatenate governed language best-practices prompts.
    ///
    /// Executes `BestPracticesOp` through the standard dispatcher path.
    /// Prompt lookup, frontmatter stripping, and concatenation remain
    /// inside `runtime-language`; this method only adapts MCP params.
    #[tool(
        name = "language-best-practices",
        description = "Resolve governed best-practices prompts for a language list and return the concatenated prompt body"
    )]
    async fn language_best_practices(
        &self,
        Parameters(LanguageBestPracticesParams { root_dir, languages }): Parameters<
            LanguageBestPracticesParams,
        >,
    ) -> Result<String, String> {
        let input = BestPracticesInput::new(IoPath::new(root_dir), languages);

        let (_cancel, mut receiver) = PluginDispatcher::new(BestPracticesOp::new())
            .input(input)
            .build()
            .map_err(|error| format!("dispatcher build failed: {error}"))?;

        match receiver.recv().await {
            Ok(Some(output)) => Ok(output.prompt),
            Ok(None) => {
                Err("language-best-practices failed: operation completed with no output"
                    .to_string())
            }
            Err(error) => Err(format!("language-best-practices failed: {error}")),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LanguageTools {}

#[cfg(test)]
#[path = "language_test.rs"]
mod tests;
