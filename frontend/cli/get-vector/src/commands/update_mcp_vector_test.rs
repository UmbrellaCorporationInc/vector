#![allow(clippy::expect_used)]

use std::{
    collections::VecDeque,
    future::{Future, ready},
    sync::Mutex,
};

use runtime_io::{
    CommandExecutor, CommandExit, CommandHandle, CommandSpec, IoError, MockCommandHandleBuilder,
};

use super::{UpdateError, UpdateOutcome, run};

// ---------------------------------------------------------------------------
// Mock executor
// ---------------------------------------------------------------------------

struct MockExecutor {
    responses: Mutex<VecDeque<Result<CommandHandle, IoError>>>,
    recorded: Mutex<Vec<(String, Vec<String>)>>,
}

impl MockExecutor {
    fn new(responses: Vec<Result<CommandHandle, IoError>>) -> Self {
        Self { responses: Mutex::new(VecDeque::from(responses)), recorded: Mutex::new(Vec::new()) }
    }

    fn recorded_commands(&self) -> Vec<(String, Vec<String>)> {
        self.recorded.lock().expect("recorded lock").clone()
    }
}

impl CommandExecutor for MockExecutor {
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = Result<CommandHandle, IoError>> + Send {
        self.recorded
            .lock()
            .expect("recorded lock")
            .push((spec.command().to_owned(), spec.args().to_vec()));
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn successful_install_returns_installed_outcome() {
    let executor = MockExecutor::new(vec![Ok(success_handle())]);

    let outcome = run(&executor).await.expect("should succeed");

    assert_eq!(outcome, UpdateOutcome::Installed);
}

#[tokio::test]
async fn install_command_uses_force_flag_and_correct_package() {
    let executor = MockExecutor::new(vec![Ok(success_handle())]);

    run(&executor).await.expect("should succeed");

    let commands = executor.recorded_commands();
    assert_eq!(commands.len(), 1);
    let (cmd, args) = &commands[0];
    assert_eq!(cmd, "cargo");
    assert!(args.contains(&"install".to_owned()));
    assert!(args.contains(&"--force".to_owned()));
    assert!(args.contains(&"mcp-vector".to_owned()));
    assert!(args.contains(&"--git".to_owned()));
}

#[tokio::test]
async fn spawn_failure_is_propagated_as_spawn_error() {
    let executor = MockExecutor::new(vec![Err(IoError::Process("cargo not found".into()))]);

    let result = run(&executor).await;

    assert!(matches!(result, Err(UpdateError::Spawn(_))));
}

#[tokio::test]
async fn non_zero_exit_is_propagated_as_install_failed() {
    let executor = MockExecutor::new(vec![Ok(failure_handle(1))]);

    let result = run(&executor).await;

    assert!(matches!(result, Err(UpdateError::InstallFailed { code: Some(1) })));
}
