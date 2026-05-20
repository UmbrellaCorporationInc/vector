#![allow(clippy::expect_used)]

use super::CommandExecutor;
use crate::command::{CommandBuilder, ProcessCommandExecutor};

#[test]
fn test_executor_trait_supports_static_dispatch() {
    fn assert_executor<E: CommandExecutor>(_executor: &E) {}

    assert_executor(&ProcessCommandExecutor);
}

#[tokio::test]
async fn test_executor_trait_supports_generic_spawn_dispatch() {
    async fn spawn_with_executor<E: CommandExecutor + Sync>(
        executor: &E,
        spec: crate::command::CommandSpec,
    ) -> Result<crate::command::CommandHandle, crate::IoError> {
        executor.spawn(spec).await
    }

    let executor = ProcessCommandExecutor;
    let spec = CommandBuilder::new(if cfg!(windows) { "cmd" } else { "sh" })
        .arg(if cfg!(windows) { "/C" } else { "-c" })
        .arg("exit 0")
        .build()
        .expect("build failed");

    let handle = spawn_with_executor(&executor, spec).await.expect("spawn failed");
    let exit = handle.wait().await.expect("wait failed");

    assert!(exit.success);
    assert_eq!(exit.code, Some(0));
}
