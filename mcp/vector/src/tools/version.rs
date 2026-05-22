//! MCP tool group for release-version introspection.

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;

use crate::release::version::workspace_version;

/// MCP-facing parameters for the `get_version` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct GetVersionParams {}

/// MCP tool group for version operations.
///
/// Owns the read-only MCP surface that exposes the canonical workspace
/// version declared for `mcp-vector`.
pub struct VersionTools {
    tool_router: ToolRouter<Self>,
}

impl VersionTools {
    /// Construct a new `VersionTools` adapter.
    #[must_use]
    pub fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }
}

impl Default for VersionTools {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl VersionTools {
    /// Return the canonical workspace version for MCP consumers.
    ///
    /// This is a read-only metadata surface. It does not inspect or mutate
    /// the host installation state.
    #[tool(description = "Return the canonical workspace version declared for mcp-vector")]
    async fn get_version(
        &self,
        Parameters(_params): Parameters<GetVersionParams>,
    ) -> Result<String, String> {
        Ok(workspace_version().to_string())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for VersionTools {}

#[cfg(test)]
#[path = "version_test.rs"]
mod tests;
