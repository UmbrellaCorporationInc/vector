use crate::{Bytes, IoError};
use runtime_core::{Receiver, RuntimeError, RuntimeResult, Sender};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};

/// Outcome of a shell command execution.
///
/// # DTO(CommandExit is a simple data container for process termination status)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommandExit {
    /// True when the process exited successfully.
    pub success: bool,
    /// The process exit code when available.
    pub code: Option<i32>,
}

/// Reader for process output streams.
#[derive(Debug)]
pub struct CommandOutput {
    source: OutputSource,
}

#[derive(Debug)]
enum OutputSource {
    Stdout(ChildStdout),
    Stderr(ChildStderr),
    Mock(VecDeque<Bytes>),
    None,
}

impl CommandOutput {
    pub(crate) const fn from_stdout(stdout: ChildStdout) -> Self {
        Self { source: OutputSource::Stdout(stdout) }
    }

    pub(crate) const fn from_stderr(stderr: ChildStderr) -> Self {
        Self { source: OutputSource::Stderr(stderr) }
    }

    pub(crate) fn from_chunks(chunks: Vec<Bytes>) -> Self {
        Self { source: OutputSource::Mock(VecDeque::from(chunks)) }
    }
}

impl Receiver<Bytes> for CommandOutput {
    async fn recv(&mut self) -> RuntimeResult<Option<Bytes>> {
        let mut buf = vec![0_u8; 8192];
        let res = match &mut self.source {
            OutputSource::Stdout(stdout) => stdout.read(&mut buf).await,
            OutputSource::Stderr(stderr) => stderr.read(&mut buf).await,
            OutputSource::Mock(chunks) => return Ok(chunks.pop_front()),
            OutputSource::None => return Ok(None),
        };

        match res {
            Ok(0) | Err(_) => {
                self.source = OutputSource::None;
                Ok(None)
            }
            Ok(count) => {
                buf.truncate(count);
                Ok(Some(buf))
            }
        }
    }
}

/// Writer for process input streams.
#[derive(Debug)]
pub struct CommandInput {
    sink: InputSink,
}

#[derive(Debug)]
enum InputSink {
    Process(ChildStdin),
    Mock(Arc<Mutex<Vec<Bytes>>>),
    Closed,
}

impl CommandInput {
    pub(crate) const fn new(stdin: ChildStdin) -> Self {
        Self { sink: InputSink::Process(stdin) }
    }

    pub(crate) const fn from_recorded_input(stdin: Arc<Mutex<Vec<Bytes>>>) -> Self {
        Self { sink: InputSink::Mock(stdin) }
    }

    /// Closes the standard input stream.
    pub fn close(&mut self) {
        self.sink = InputSink::Closed;
    }
}

impl Sender<Bytes> for CommandInput {
    async fn send(&mut self, value: Bytes) -> RuntimeResult<()> {
        match &mut self.sink {
            InputSink::Process(stdin) => {
                if stdin.write_all(&value).await.is_err() {
                    self.sink = InputSink::Closed;
                    return Err(RuntimeError::operation("failed to write to process stdin"));
                }

                Ok(())
            }
            InputSink::Mock(recorded) => {
                recorded
                    .lock()
                    .map_err(|_| RuntimeError::operation("mock stdin lock poisoned"))?
                    .push(value);
                Ok(())
            }
            InputSink::Closed => Err(RuntimeError::operation("write to closed stdin")),
        }
    }
}

/// Handle to a running shell command.
#[derive(Debug)]
pub struct CommandHandle {
    backend: WaitBackend,
    stdout: CommandOutput,
    stderr: CommandOutput,
    stdin: CommandInput,
}

#[derive(Debug)]
enum WaitBackend {
    Process(Box<Child>),
    Mock(Option<Result<CommandExit, IoError>>),
}

impl CommandHandle {
    pub(crate) fn try_new(mut child: Child) -> Result<Self, IoError> {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| IoError::Process("piped stdout is missing".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| IoError::Process("piped stderr is missing".into()))?;
        let stdin =
            child.stdin.take().ok_or_else(|| IoError::Process("piped stdin is missing".into()))?;

        Ok(Self {
            backend: WaitBackend::Process(Box::new(child)),
            stdout: CommandOutput::from_stdout(stdout),
            stderr: CommandOutput::from_stderr(stderr),
            stdin: CommandInput::new(stdin),
        })
    }

    pub(crate) fn mock(
        stdout: Vec<Bytes>,
        stderr: Vec<Bytes>,
        exit: Result<CommandExit, IoError>,
        stdin: Arc<Mutex<Vec<Bytes>>>,
    ) -> Self {
        Self {
            backend: WaitBackend::Mock(Some(exit)),
            stdout: CommandOutput::from_chunks(stdout),
            stderr: CommandOutput::from_chunks(stderr),
            stdin: CommandInput::from_recorded_input(stdin),
        }
    }

    /// Returns mutable stdout access.
    pub const fn stdout(&mut self) -> &mut CommandOutput {
        &mut self.stdout
    }

    /// Returns mutable stderr access.
    pub const fn stderr(&mut self) -> &mut CommandOutput {
        &mut self.stderr
    }

    /// Returns mutable stdin access.
    pub const fn stdin(&mut self) -> &mut CommandInput {
        &mut self.stdin
    }

    /// Waits for the process to complete.
    ///
    /// # Errors
    /// Returns [`IoError::Process`] when waiting on the child process fails.
    pub async fn wait(mut self) -> Result<CommandExit, IoError> {
        self.stdin.close();

        match &mut self.backend {
            WaitBackend::Process(child) => match child.wait().await {
                Ok(status) => Ok(CommandExit { success: status.success(), code: status.code() }),
                Err(error) => Err(IoError::Process(error.to_string())),
            },
            WaitBackend::Mock(exit) => exit
                .take()
                .unwrap_or_else(|| Err(IoError::Process("mock command already completed".into()))),
        }
    }
}

impl Drop for CommandHandle {
    fn drop(&mut self) {
        self.stdin.close();
        if let WaitBackend::Process(child) = &mut self.backend {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
#[path = "handle_test.rs"]
mod tests;
