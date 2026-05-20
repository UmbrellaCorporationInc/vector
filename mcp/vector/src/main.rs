//! Executable entrypoint for the `mcp-vector` stdio server.

use mcp_vector::server::VectorServer;

#[tokio::main]
async fn main() -> Result<(), mcp_vector::error::VectorServerError> {
    VectorServer::new().serve_stdio().await
}
