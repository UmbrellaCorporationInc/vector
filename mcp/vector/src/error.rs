//! MCP-local error adaptation for the vector server.

use rmcp::service::{ServerInitializeError, ServiceError};
use thiserror::Error;

/// Errors that can occur during MCP server operation.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VectorServerError {
    /// The MCP handshake failed during server initialization.
    #[error("MCP server initialization failed: {0}")]
    Initialize(Box<ServerInitializeError>),

    /// A transport or protocol error occurred after initialization.
    #[error("MCP service error: {0}")]
    Service(#[from] ServiceError),

    /// The background service task panicked or was aborted.
    #[error("MCP service task failed")]
    TaskFailed,
}

impl From<ServerInitializeError> for VectorServerError {
    fn from(error: ServerInitializeError) -> Self {
        Self::Initialize(Box::new(error))
    }
}

#[cfg(test)]
#[path = "error_test.rs"]
mod tests;
