//! Filesystem error types.

/// Errors emitted by all filesystem backends.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FsError {
    /// The target path does not exist.
    #[error("Fs Error: not found at '{0}'")]
    NotFound(String),
    /// The caller does not have permission to access the target path.
    #[error("Fs Error: permission denied at '{0}'")]
    PermissionDenied(String),
    /// An unclassified I/O failure.
    #[error("Fs Error: {0}")]
    Io(String),
}

#[cfg(test)]
#[path = "error_test.rs"]
mod tests;
