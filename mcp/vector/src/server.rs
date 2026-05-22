//! MCP server bootstrap and central handler composition.
//!
//! `VectorServer` owns the final MCP surface for the vector system.
//! Tool groups are composed here by delegating to each capability group's
//! `ServerHandler` implementation through rmcp's router merge pattern.

use rmcp::{
    RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams,
        ServerCapabilities, ServerInfo, Tool,
    },
    serve_server,
    service::RequestContext,
    transport::io::stdio,
};

use crate::error::VectorServerError;
use crate::tools::document::DocumentTools;
use crate::tools::language::LanguageTools;
use crate::tools::project::ProjectTools;
use crate::tools::version::VersionTools;

/// Central MCP server handler for the vector system.
///
/// Composes all registered MCP tool groups into one surface.
/// Each group contributes through the handler delegation pattern.
pub struct VectorServer {
    document: DocumentTools,
    language: LanguageTools,
    project: ProjectTools,
    version: VersionTools,
}

impl VectorServer {
    /// Construct the server handler with all registered tool groups.
    #[must_use]
    pub fn new() -> Self {
        Self {
            document: DocumentTools::new(),
            language: LanguageTools::new(),
            project: ProjectTools::new(),
            version: VersionTools::new(),
        }
    }

    /// Start the MCP server over stdio and run until the transport closes.
    ///
    /// Uses the standard rmcp stdio transport. The future resolves when the
    /// client disconnects or the process receives a shutdown signal.
    ///
    /// # Errors
    ///
    /// Returns [`VectorServerError`] if the MCP handshake or transport fails.
    pub async fn serve_stdio(self) -> Result<(), VectorServerError> {
        let running = serve_server(self, stdio()).await?;
        running.waiting().await.map_err(|_| VectorServerError::TaskFailed)?;
        Ok(())
    }
}

impl Default for VectorServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for VectorServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        let mut doc_result = self.document.list_tools(request.clone(), context.clone()).await?;
        let language_result = self.language.list_tools(request.clone(), context.clone()).await?;
        let project_result = self.project.list_tools(request.clone(), context.clone()).await?;
        let version_result = self.version.list_tools(request, context).await?;
        doc_result.tools.extend(language_result.tools);
        doc_result.tools.extend(project_result.tools);
        doc_result.tools.extend(version_result.tools);
        Ok(ListToolsResult::with_all_items(doc_result.tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        if self.document.get_tool(&request.name).is_some() {
            return self.document.call_tool(request, context).await;
        }
        if self.language.get_tool(&request.name).is_some() {
            return self.language.call_tool(request, context).await;
        }
        if self.version.get_tool(&request.name).is_some() {
            return self.version.call_tool(request, context).await;
        }
        self.project.call_tool(request, context).await
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.document
            .get_tool(name)
            .or_else(|| self.language.get_tool(name))
            .or_else(|| self.version.get_tool(name))
            .or_else(|| self.project.get_tool(name))
    }
}

#[cfg(test)]
#[path = "server_test.rs"]
mod tests;
