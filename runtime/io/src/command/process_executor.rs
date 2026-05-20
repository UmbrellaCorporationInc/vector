use super::{CommandExecutor, CommandHandle, CommandSpec};
use crate::IoError;
use std::{future::ready, process::Stdio};

/// Operating-system-backed command executor.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct ProcessCommandExecutor;

impl CommandExecutor for ProcessCommandExecutor {
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl std::future::Future<Output = Result<CommandHandle, IoError>> + Send {
        let mut command = tokio::process::Command::new(spec.command());
        command
            .args(spec.args())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .kill_on_drop(true);

        if let Some(current_dir) = spec.current_dir() {
            command.current_dir(current_dir);
        }

        for (key, value) in spec.env() {
            command.env(key, value);
        }

        let result = command.spawn().map_err(|error| IoError::Process(error.to_string()));
        ready(result.and_then(CommandHandle::try_new))
    }
}

#[cfg(test)]
#[path = "process_executor_test.rs"]
mod tests;
