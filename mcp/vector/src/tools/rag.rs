//! MCP tool group for RAG capability domain.

use std::path::PathBuf;

use rmcp::{
    RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    schemars, tool, tool_handler, tool_router,
};
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_rag::{AssembleRetrievalContextOp, HybridSearchInput, HybridSearchOp, RagDefaults};
use serde::Deserialize;

/// MCP-facing parameters for the `search` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RagSearchParams {
    /// Free-text query sent to the local RAG retrieval operation.
    pub query: String,
    /// Optional final retrieval limit override applied before context assembly.
    pub limit: Option<usize>,
    /// Optional package filter applied before ranking and fusion.
    pub package: Option<String>,
    /// Optional governed document stem filter applied before ranking and fusion.
    pub document: Option<String>,
}

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
    #[tool(
        name = "search",
        description = "Query the local RAG index for this workspace and return relevant governed document context."
    )]
    async fn search(
        &self,
        context: rmcp::service::RequestContext<RoleServer>,
        Parameters(params): Parameters<RagSearchParams>,
    ) -> Result<String, String> {
        let workspace_root = resolve_workspace_root_from_runtime_context(&context)?;

        let query = params.query.trim().to_owned();
        if query.is_empty() {
            return Err("rag.search requires a non-empty query".to_owned());
        }

        let input = HybridSearchInput::new(
            workspace_root,
            RagDefaults::phase_one(),
            query,
            params.package,
            params.document,
            params.limit,
        );

        let (_cancel, mut receiver) = PluginDispatcher::new(HybridSearchOp::new())
            .input(input)
            .build()
            .map_err(|error| format!("rag.search failed to prepare retrieval: {error}"))?;

        let search_output = receiver
            .recv()
            .await
            .map_err(|error| format!("rag.search retrieval failed: {error}"))?
            .ok_or_else(|| "rag.search retrieval did not produce output".to_owned())?;

        let (_cancel, mut receiver) = PluginDispatcher::new(AssembleRetrievalContextOp::new())
            .input(search_output)
            .build()
            .map_err(|error| format!("rag.search failed to prepare context assembly: {error}"))?;

        let context_result = receiver
            .recv()
            .await
            .map_err(|error| format!("rag.search context assembly failed: {error}"))?
            .ok_or_else(|| "rag.search context assembly did not produce output".to_owned())?;

        serde_json::to_string_pretty(&context_result)
            .map_err(|error| format!("rag.search failed to serialize retrieval context: {error}"))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for RagTools {}

#[cfg(test)]
#[path = "rag_test.rs"]
mod tests;
