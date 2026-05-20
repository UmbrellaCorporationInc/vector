use crate::error::RuntimeError;

/// Canonical runtime-core result alias.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[cfg(test)]
#[path = "result_test.rs"]
mod tests;
