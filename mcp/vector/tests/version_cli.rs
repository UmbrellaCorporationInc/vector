//! Integration tests for the `mcp-vector` process-level CLI contract.

#![allow(clippy::expect_used)]

use mcp_vector::release::version::workspace_version;
use runtime_core::Receiver;
use runtime_io::{CommandBuilder, CommandExecutor, CommandOutput, ProcessCommandExecutor};

async fn read_output(output: &mut CommandOutput) -> String {
    let mut text = String::new();
    while let Some(chunk) = output.recv().await.expect("command output read must succeed") {
        text.push_str(std::str::from_utf8(&chunk).expect("command output must be valid UTF-8"));
    }
    text
}

/// Verifies that `mcp-vector --version` prints only the canonical version string.
#[tokio::test]
async fn version_flag_prints_only_workspace_version() {
    let spec = CommandBuilder::new(env!("CARGO_BIN_EXE_mcp-vector"))
        .arg("--version")
        .build()
        .expect("command spec must build");
    let executor = ProcessCommandExecutor::default();
    let mut handle = executor.spawn(spec).await.expect("mcp-vector binary must spawn successfully");
    let stdout = read_output(handle.stdout()).await;
    let stderr = read_output(handle.stderr()).await;
    let exit = handle.wait().await.expect("mcp-vector binary must exit successfully");

    assert!(exit.success, "mcp-vector --version must exit successfully: {exit:?}");
    assert_eq!(
        stdout,
        format!("{}\n", workspace_version()),
        "--version must print only the canonical workspace version followed by a newline"
    );
    assert!(stderr.is_empty(), "--version must not emit stderr output");
}
