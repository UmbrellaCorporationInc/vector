#![allow(clippy::expect_used)]

use super::*;
use runtime_io::{CommandExit, CommandHandle, CommandSpec, IoError, MockCommandHandleBuilder};
use std::future::{Future, ready};
use std::sync::Mutex;

#[derive(Debug, Clone)]
struct RecordedCommand {
    command: String,
    args: Vec<String>,
    current_dir: Option<std::path::PathBuf>,
}

struct MockExecutor {
    response: Mutex<Option<Result<CommandHandle, IoError>>>,
    recorded: Mutex<Vec<RecordedCommand>>,
}

impl MockExecutor {
    fn new(response: Result<CommandHandle, IoError>) -> Self {
        Self { response: Mutex::new(Some(response)), recorded: Mutex::new(Vec::new()) }
    }

    fn recorded_commands(&self) -> Vec<RecordedCommand> {
        self.recorded.lock().expect("recorded lock").clone()
    }
}

impl CommandExecutor for MockExecutor {
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = Result<CommandHandle, IoError>> + Send {
        self.recorded.lock().expect("recorded lock").push(RecordedCommand {
            command: spec.command().to_owned(),
            args: spec.args().to_vec(),
            current_dir: spec.current_dir().map(std::path::Path::to_path_buf),
        });

        let result = self
            .response
            .lock()
            .expect("response lock")
            .take()
            .unwrap_or_else(|| Err(IoError::Process("mock executor exhausted".into())));
        ready(result)
    }
}

fn handle(exit: CommandExit) -> CommandHandle {
    MockCommandHandleBuilder::new(exit).build().0
}

#[tokio::test]
async fn delegates_rag_args_to_vector_rag_with_workspace_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executor = MockExecutor::new(Ok(handle(CommandExit::new(true, Some(0)))));
    let args = vec!["search".to_owned(), "hybrid retrieval".to_owned(), "--json".to_owned()];

    let exit = run(&executor, temp.path(), &args).await.expect("delegation should succeed");

    assert_eq!(exit, DelegatedExit::Success);
    let commands = executor.recorded_commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].command, "vector-rag");
    assert_eq!(commands[0].args, vec!["rag", "search", "hybrid retrieval", "--json"]);
    assert_eq!(commands[0].current_dir.as_deref(), Some(temp.path()));
}

#[tokio::test]
async fn returns_companion_exit_code_without_converting_to_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executor = MockExecutor::new(Ok(handle(CommandExit::new(false, Some(42)))));
    let args = vec!["update-database".to_owned()];

    let exit = run(&executor, temp.path(), &args).await.expect("non-zero exit should pass through");

    assert_eq!(exit, DelegatedExit::Failure(Some(42)));
    assert_eq!(exit.code(), 42);
}

#[tokio::test]
async fn streams_companion_stdout_and_stderr() {
    let temp = tempfile::tempdir().expect("tempdir");
    let companion = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout("indexed\n")
        .stderr("warning\n")
        .build()
        .0;
    let executor = MockExecutor::new(Ok(companion));
    let args = vec!["init".to_owned()];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit = run_with_output(
        &executor,
        temp.path(),
        &args,
        &mut |bytes| stdout.extend_from_slice(bytes),
        &mut |bytes| stderr.extend_from_slice(bytes),
    )
    .await
    .expect("delegation should succeed");

    assert_eq!(exit, DelegatedExit::Success);
    assert_eq!(stdout, b"indexed\n");
    assert_eq!(stderr, b"warning\n");
}

#[tokio::test]
async fn forwards_multiple_stdout_chunks_incrementally() {
    let temp = tempfile::tempdir().expect("tempdir");
    let companion = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout("indexed doc-1\n")
        .stdout("unchanged doc-2\n")
        .build()
        .0;
    let executor = MockExecutor::new(Ok(companion));
    let args = vec!["update-database".to_owned()];
    let mut stdout_chunks = Vec::new();

    let exit = run_with_output(
        &executor,
        temp.path(),
        &args,
        &mut |bytes| stdout_chunks.push(bytes.to_vec()),
        &mut |_bytes| {},
    )
    .await
    .expect("delegation should succeed");

    assert_eq!(exit, DelegatedExit::Success);
    assert_eq!(stdout_chunks.len(), 2, "expected one callback per stdout chunk");
    assert_eq!(stdout_chunks[0], b"indexed doc-1\n");
    assert_eq!(stdout_chunks[1], b"unchanged doc-2\n");
}

#[tokio::test]
async fn returns_install_guidance_when_vector_rag_cannot_spawn() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executor = MockExecutor::new(Err(IoError::Process("not found".to_owned())));
    let args = vec!["init".to_owned()];

    let error = run(&executor, temp.path(), &args).await.expect_err("spawn failure should fail");

    assert_eq!(
        error,
        "vector-rag is not available on PATH. Install RAG support with `get-vector install rag` and try again."
    );
}
