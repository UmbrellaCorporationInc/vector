#![allow(clippy::expect_used)]

use super::ProcessCommandExecutor;
use crate::command::{CommandBuilder, CommandExecutor};

#[tokio::test]
async fn test_process_executor_reports_spawn_errors() {
    let spec = CommandBuilder::new("definitely-not-a-real-command").build().expect("build failed");

    let executor = ProcessCommandExecutor;
    let error = executor.spawn(spec).await.expect_err("spawn should fail");

    assert!(matches!(error, crate::IoError::Process(_)));
}
