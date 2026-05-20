//! MCP tool group for project capability domain.

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    schemars, tool, tool_handler, tool_router,
};
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_io::path::IoPath;
use runtime_project::{ProjectSetupInput, ProjectSetupOp};
use serde::Deserialize;

/// MCP-facing parameters for the `create_project` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateProjectParams {
    /// Absolute or relative path to the directory where the project will be created.
    pub target_dir: String,
    /// Human-readable name for the new project.
    pub project_name: String,
    /// When true, existing files at the target path are overwritten.
    #[serde(default)]
    pub force: bool,
}

/// MCP-facing parameters for the `update_project` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateProjectParams {
    /// Absolute or relative path to the project root directory.
    pub target_dir: String,
    /// Human-readable name for the project.
    pub project_name: String,
}

/// MCP tool group for project operations.
///
/// Owns MCP tool definitions for the project capability domain.
/// Reusable execution logic lives in `runtime-project`; this struct
/// is a thin MCP adapter only.
pub struct ProjectTools {
    tool_router: ToolRouter<Self>,
}

impl ProjectTools {
    /// Construct a new `ProjectTools` adapter.
    #[must_use]
    pub fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }
}

impl Default for ProjectTools {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl ProjectTools {
    /// Create a new governed project at the given path.
    ///
    /// Executes `ProjectSetupOp` through the standard dispatcher path.
    /// The operation bootstraps the project skeleton and installs the
    /// documentation extension in a single transport-agnostic workflow.
    #[tool(description = "Create a new governed vector project at the specified directory")]
    async fn create_project(
        &self,
        Parameters(CreateProjectParams { target_dir, project_name, force }): Parameters<
            CreateProjectParams,
        >,
    ) -> Result<String, String> {
        let input = ProjectSetupInput::new(IoPath::new(target_dir), project_name, force);

        let (_cancel, mut receiver) = PluginDispatcher::new(ProjectSetupOp::default())
            .input(input)
            .build()
            .map_err(|e| format!("dispatcher build failed: {e}"))?;

        let mut message = String::new();
        loop {
            match receiver.recv().await {
                Ok(Some(result)) => {
                    if !message.is_empty() {
                        message.push('\n');
                    }
                    message.push_str(&result.project.message);
                }
                Ok(None) => break,
                Err(e) => return Err(format!("create_project failed: {e}")),
            }
        }

        Ok(message)
    }

    /// Update an existing governed project by provisioning missing assets.
    ///
    /// Executes `ProjectSetupOp` with `force: false` through the standard
    /// dispatcher path. Any asset that is already present is left untouched;
    /// only missing files are created.
    #[tool(
        description = "Update an existing vector project by adding missing governed assets without overwriting existing files"
    )]
    async fn update_project(
        &self,
        Parameters(UpdateProjectParams { target_dir, project_name }): Parameters<
            UpdateProjectParams,
        >,
    ) -> Result<String, String> {
        let input = ProjectSetupInput::new(IoPath::new(target_dir), project_name, false);

        let (_cancel, mut receiver) = PluginDispatcher::new(ProjectSetupOp::default())
            .input(input)
            .build()
            .map_err(|e| format!("dispatcher build failed: {e}"))?;

        let mut message = String::new();
        loop {
            match receiver.recv().await {
                Ok(Some(result)) => {
                    if !message.is_empty() {
                        message.push('\n');
                    }
                    message.push_str(&result.project.message);
                }
                Ok(None) => break,
                Err(e) => return Err(format!("update_project failed: {e}")),
            }
        }

        Ok(message)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ProjectTools {}

#[cfg(test)]
#[path = "project_test.rs"]
mod tests;
