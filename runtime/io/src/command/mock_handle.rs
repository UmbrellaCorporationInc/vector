use super::{CommandExit, CommandHandle};
use crate::{Bytes, IoError};
use std::sync::{Arc, Mutex};

/// Built mock command handle parts.
pub type MockCommandHandleParts = (CommandHandle, MockCommandHandleProbe);

/// Builder for deterministic mock command handles.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MockCommandHandleBuilder {
    stdout: Vec<Bytes>,
    stderr: Vec<Bytes>,
    wait_result: MockWaitResult,
}

/// Probe for inspecting one mock command handle interaction.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MockCommandHandleProbe {
    recorded_stdin: Arc<Mutex<Vec<Bytes>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MockWaitResult {
    Exit(CommandExit),
    Error(String),
}

impl MockCommandHandleBuilder {
    /// Creates a new mock command handle builder.
    #[must_use]
    pub const fn new(exit: CommandExit) -> Self {
        Self { stdout: Vec::new(), stderr: Vec::new(), wait_result: MockWaitResult::Exit(exit) }
    }

    /// Configures one stdout chunk.
    #[must_use]
    pub fn stdout(mut self, chunk: impl Into<Bytes>) -> Self {
        self.stdout.push(chunk.into());
        self
    }

    /// Configures one stderr chunk.
    #[must_use]
    pub fn stderr(mut self, chunk: impl Into<Bytes>) -> Self {
        self.stderr.push(chunk.into());
        self
    }

    /// Configures a wait failure.
    #[must_use]
    pub fn wait_error(mut self, message: impl Into<String>) -> Self {
        self.wait_result = MockWaitResult::Error(message.into());
        self
    }

    /// Builds one mock command handle and its probe.
    #[must_use]
    pub fn build(&self) -> MockCommandHandleParts {
        let recorded_stdin = Arc::new(Mutex::new(Vec::new()));
        let probe = MockCommandHandleProbe { recorded_stdin: Arc::clone(&recorded_stdin) };
        let wait_result = match &self.wait_result {
            MockWaitResult::Exit(exit) => Ok(exit.clone()),
            MockWaitResult::Error(message) => Err(IoError::Process(message.clone())),
        };
        let handle = CommandHandle::mock(
            self.stdout.clone(),
            self.stderr.clone(),
            wait_result,
            recorded_stdin,
        );

        (handle, probe)
    }
}

impl MockCommandHandleProbe {
    /// Returns the stdin chunks recorded for this handle.
    ///
    /// # Errors
    /// Returns [`IoError::Process`] when the mock probe state is poisoned.
    pub fn recorded_stdin(&self) -> Result<Vec<Bytes>, IoError> {
        Ok(self
            .recorded_stdin
            .lock()
            .map_err(|_| IoError::Process("mock command handle probe state is poisoned".into()))?
            .clone())
    }
}

#[cfg(test)]
#[path = "mock_handle_test.rs"]
mod tests;
