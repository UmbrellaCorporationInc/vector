use super::{CommandHandle, CommandSpec};
use crate::IoError;
use std::future::Future;

/// Execution boundary for command specifications.
///
/// V1 uses static dispatch through concrete executors or generic callers.
/// `CommandHandle` remains the stable runtime boundary after execution starts.
pub trait CommandExecutor {
    /// Spawns a running command from a prepared specification.
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = Result<CommandHandle, IoError>> + Send;
}

#[cfg(test)]
#[path = "executor_test.rs"]
mod tests;
