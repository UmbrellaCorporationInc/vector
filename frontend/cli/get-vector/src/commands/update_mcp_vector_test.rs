#![allow(clippy::expect_used)]

use std::{
    collections::VecDeque,
    future::{Future, ready},
    sync::Mutex,
};

use runtime_io::{
    CommandExecutor, CommandExit, CommandHandle, CommandSpec, IoError, MockCommandHandleBuilder,
};

use super::{UpdateError, UpdateOutcome, run, run_rag};

// ---------------------------------------------------------------------------
// Mock executor
// ---------------------------------------------------------------------------

struct MockExecutor {
    responses: Mutex<VecDeque<Result<CommandHandle, IoError>>>,
    recorded: Mutex<Vec<CommandSpec>>,
}

impl MockExecutor {
    fn new(responses: Vec<Result<CommandHandle, IoError>>) -> Self {
        Self { responses: Mutex::new(VecDeque::from(responses)), recorded: Mutex::new(Vec::new()) }
    }

    fn recorded_commands(&self) -> Vec<(String, Vec<String>)> {
        self.recorded
            .lock()
            .expect("recorded lock")
            .iter()
            .map(|spec| (spec.command().to_owned(), spec.args().to_vec()))
            .collect()
    }

    fn recorded_specs(&self) -> Vec<CommandSpec> {
        self.recorded.lock().expect("recorded lock").clone()
    }
}

impl CommandExecutor for MockExecutor {
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = Result<CommandHandle, IoError>> + Send {
        self.recorded.lock().expect("recorded lock").push(spec);
        let result = self
            .responses
            .lock()
            .expect("responses lock")
            .pop_front()
            .unwrap_or_else(|| Err(IoError::Process("mock executor exhausted".into())));
        ready(result)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn success_handle() -> CommandHandle {
    MockCommandHandleBuilder::new(CommandExit::new(true, Some(0))).build().0
}

fn failure_handle(code: i32) -> CommandHandle {
    MockCommandHandleBuilder::new(CommandExit::new(false, Some(code))).build().0
}

fn wait_error_handle(message: &str) -> CommandHandle {
    MockCommandHandleBuilder::new(CommandExit::new(true, Some(0))).wait_error(message).build().0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn successful_install_returns_installed_outcome() {
    let executor = MockExecutor::new(vec![Ok(success_handle()), Ok(success_handle())]);

    let outcome = run(&executor, |_| {}, |_| {}).await.expect("should succeed");

    assert_eq!(outcome, UpdateOutcome::Installed);
}

#[tokio::test]
async fn install_command_uses_force_flag_and_correct_package() {
    let executor = MockExecutor::new(vec![Ok(success_handle()), Ok(success_handle())]);

    run(&executor, |_| {}, |_| {}).await.expect("should succeed");

    let commands = executor.recorded_commands();
    assert_eq!(commands.len(), 2);

    let (cmd1, args1) = &commands[0];
    assert_eq!(cmd1, "cargo");
    assert!(args1.contains(&"install".to_owned()));
    assert!(args1.contains(&"--force".to_owned()));
    assert!(args1.contains(&"mcp-vector".to_owned()));
    assert!(args1.contains(&"--git".to_owned()));

    let (cmd2, args2) = &commands[1];
    assert_eq!(cmd2, "cargo");
    assert!(args2.contains(&"install".to_owned()));
    assert!(args2.contains(&"--force".to_owned()));
    assert!(args2.contains(&"vector-database".to_owned()));
    assert!(args2.contains(&"--git".to_owned()));
}

#[tokio::test]
async fn base_install_does_not_install_vector_rag() {
    let executor = MockExecutor::new(vec![Ok(success_handle()), Ok(success_handle())]);

    run(&executor, |_| {}, |_| {}).await.expect("should succeed");

    let commands = executor.recorded_commands();
    assert_eq!(commands.len(), 2);
    assert!(
        commands.iter().all(|(_, args)| !args.contains(&"vector-rag".to_owned())),
        "update-mcp-vector should leave RAG support to get-vector install rag"
    );
}

#[tokio::test]
async fn rag_install_command_installs_only_vector_rag() {
    let executor = MockExecutor::new(vec![Ok(success_handle())]);

    run_rag(&executor, |_| {}, |_| {}).await.expect("should succeed");

    let commands = executor.recorded_commands();
    assert_eq!(commands.len(), 1);

    let (cmd, args) = &commands[0];
    assert_eq!(cmd, "cargo");
    assert!(args.contains(&"install".to_owned()));
    assert!(args.contains(&"--force".to_owned()));
    assert!(args.contains(&"vector-rag".to_owned()));
    assert!(args.contains(&"--git".to_owned()));
    assert!(!args.contains(&"mcp-vector".to_owned()));
    assert!(!args.contains(&"vector-database".to_owned()));
}

#[tokio::test]
async fn spawn_failure_is_propagated_as_spawn_error() {
    let executor = MockExecutor::new(vec![Err(IoError::Process("cargo not found".into()))]);

    let result = run(&executor, |_| {}, |_| {}).await;

    assert!(matches!(result, Err(UpdateError::Spawn(_))));
}

#[tokio::test]
async fn non_zero_exit_is_propagated_as_install_failed() {
    let executor = MockExecutor::new(vec![Ok(failure_handle(1))]);

    let result = run(&executor, |_| {}, |_| {}).await;

    assert!(matches!(result, Err(UpdateError::InstallFailed { code: Some(1) })));
}

#[tokio::test]
async fn wait_failure_is_propagated_as_wait_error() {
    let executor = MockExecutor::new(vec![Ok(wait_error_handle("process disconnected"))]);

    let result = run(&executor, |_| {}, |_| {}).await;

    assert!(matches!(result, Err(UpdateError::Wait(_))));
}

#[tokio::test]
async fn cargo_output_is_forwarded_to_callbacks() {
    let handle1 = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout(b"installing binary\n".to_vec())
        .stderr(b"Compiling mcp-vector\n".to_vec())
        .build()
        .0;
    let handle2 = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout(b"installing cli\n".to_vec())
        .stderr(b"Compiling vector-database\n".to_vec())
        .build()
        .0;
    let executor = MockExecutor::new(vec![Ok(handle1), Ok(handle2)]);

    let mut captured_stdout = Vec::new();
    let mut captured_stderr = Vec::new();

    let outcome = run(
        &executor,
        |b| captured_stdout.extend_from_slice(b),
        |b| captured_stderr.extend_from_slice(b),
    )
    .await
    .expect("should succeed");

    assert_eq!(outcome, UpdateOutcome::Installed);
    assert_eq!(captured_stdout, b"installing binary\ninstalling cli\n");
    assert_eq!(captured_stderr, b"Compiling mcp-vector\nCompiling vector-database\n");
}

#[tokio::test]
async fn install_command_sets_cargo_term_progress_width_env() {
    let executor = MockExecutor::new(vec![Ok(success_handle()), Ok(success_handle())]);

    run(&executor, |_| {}, |_| {}).await.expect("should succeed");

    let specs = executor.recorded_specs();
    assert_eq!(specs.len(), 2);

    for spec in &specs {
        let envs = spec.env();
        let width_env = envs.iter().find(|(k, _)| k == "CARGO_TERM_PROGRESS_WIDTH").map(|(_, v)| v);

        assert!(
            width_env.is_some(),
            "CARGO_TERM_PROGRESS_WIDTH environment variable should be set"
        );
        let width_val = width_env.expect("checked is_some");
        assert!(
            width_val.parse::<usize>().is_ok(),
            "width value should be a valid integer, got: {width_val}"
        );
    }
}
