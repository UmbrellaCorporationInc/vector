//! MCP tool group for RAG capability domain.

use std::path::PathBuf;

use rmcp::{
    RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;

/// MCP-facing parameters for the `search` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct RagSearchParams {}

/// MCP tool group for RAG operations.
///
/// Owns MCP tool definitions for the RAG capability domain.
/// Reusable retrieval logic lives outside `mcp-vector`; this struct is
/// only the MCP adapter boundary.
pub struct RagTools {
    tool_router: ToolRouter<Self>,
}

impl RagTools {
    /// Construct a new `RagTools` adapter.
    #[must_use]
    pub fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }
}

impl Default for RagTools {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_workspace_root_from_runtime_context(
    _context: &rmcp::service::RequestContext<RoleServer>,
) -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|error| {
        format!("rag.search failed to resolve the MCP runtime workspace root: {error}")
    })
}

#[tool_router]
impl RagTools {
    /// Search the local RAG index for governed document context.
    ///
    /// Phase A establishes the MCP registration boundary. Later phases add
    /// input validation, retrieval execution, and canonical context output.
    #[tool(
        name = "search",
        description = "Query the local RAG index for this workspace and return relevant governed document context."
    )]
    async fn search(
        &self,
        context: rmcp::service::RequestContext<RoleServer>,
        Parameters(_params): Parameters<RagSearchParams>,
    ) -> Result<String, String> {
        let workspace_root = resolve_workspace_root_from_runtime_context(&context)?;
        Ok(format!("rag.search is registered for workspace root: {}", workspace_root.display()))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for RagTools {}

#[cfg(test)]
#[path = "rag_test.rs"]
mod tests;
