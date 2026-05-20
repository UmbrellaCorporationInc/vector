use super::{CommandEnvironmentVariable, CommandSpec};
use crate::IoError;
use std::path::PathBuf;

/// Builder for data-only shell command specifications.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandBuilder {
    command: String,
    args: Vec<String>,
    current_dir: Option<PathBuf>,
    env: Vec<CommandEnvironmentVariable>,
}

impl CommandBuilder {
    /// Creates a new command builder.
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self { command: command.into(), args: Vec::new(), current_dir: None, env: Vec::new() }
    }

    /// Adds one argument.
    #[must_use]
    pub fn arg(mut self, argument: impl Into<String>) -> Self {
        self.args.push(argument.into());
        self
    }

    /// Adds multiple arguments.
    #[must_use]
    pub fn args(mut self, arguments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(arguments.into_iter().map(Into::into));
        self
    }

    /// Sets the working directory.
    #[must_use]
    pub fn current_dir(mut self, path: impl AsRef<std::path::Path>) -> Self {
        self.current_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Adds one environment variable.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Builds the command specification.
    ///
    /// # Errors
    /// Returns [`IoError::Process`] when the command is empty.
    pub fn build(self) -> Result<CommandSpec, IoError> {
        if self.command.trim().is_empty() {
            return Err(IoError::Process("command cannot be empty".into()));
        }

        Ok(CommandSpec::new(self.command, self.args, self.current_dir, self.env))
    }
}

#[cfg(test)]
#[path = "builder_test.rs"]
mod tests;
