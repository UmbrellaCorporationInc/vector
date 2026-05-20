use std::path::{Path, PathBuf};

/// Environment entry for one command specification.
pub type CommandEnvironmentVariable = (String, String);

/// Data-only shell command specification.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommandSpec {
    command: String,
    args: Vec<String>,
    current_dir: Option<PathBuf>,
    env: Vec<CommandEnvironmentVariable>,
}

impl CommandSpec {
    pub(crate) const fn new(
        command: String,
        args: Vec<String>,
        current_dir: Option<PathBuf>,
        env: Vec<CommandEnvironmentVariable>,
    ) -> Self {
        Self { command, args, current_dir, env }
    }

    /// Returns the executable command.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Returns the ordered arguments.
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Returns the configured working directory.
    #[must_use]
    pub fn current_dir(&self) -> Option<&Path> {
        self.current_dir.as_deref()
    }

    /// Returns the environment overlay.
    #[must_use]
    pub fn env(&self) -> &[CommandEnvironmentVariable] {
        &self.env
    }
}

#[cfg(test)]
#[path = "spec_test.rs"]
mod tests;
