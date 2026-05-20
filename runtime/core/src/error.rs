use thiserror::Error;

/// Root runtime-core error boundary.
#[derive(Debug, Error, PartialEq)]
#[non_exhaustive]
pub enum RuntimeError {
    /// Operation failure with a caller-meaningful message.
    ///
    /// Prefer [`RuntimeError::operation`] over constructing this variant directly
    /// so all call sites use a consistent format.
    #[error("runtime operation failed: {0}")]
    Operation(String),
    /// The remote channel endpoint has been dropped.
    #[error("channel closed")]
    ChannelClosed,
    /// A cancel-aware operation was attempted after cancellation was signalled.
    #[error("cancelled")]
    Cancelled,
    /// Failed to decode bytes as UTF-8.
    #[error("encoding error: {0}")]
    Encoding(String),
}

impl RuntimeError {
    /// Construct an [`RuntimeError::Operation`] with a caller-meaningful message.
    #[must_use]
    pub fn operation(msg: impl Into<String>) -> Self {
        Self::Operation(msg.into())
    }
}

#[cfg(test)]
#[path = "error_test.rs"]
mod tests;
