//! Typed error surface for runtime-io.

use thiserror::Error;

/// Error variants for IO operations.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum IoError {
    /// File operation failed.
    #[error("file operation failed: {0}")]
    File(String),

    /// Path operation failed.
    #[error("path operation failed: {0}")]
    Path(String),

    /// UTF-8 encoding or decoding failed.
    #[error("utf-8 processing failed: {0}")]
    Text(String),

    /// Shell process execution failed.
    #[error("process execution failed: {0}")]
    Process(String),
}

#[cfg(test)]
#[path = "error_test.rs"]
mod tests;
