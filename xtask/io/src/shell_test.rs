#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Read;

use super::*;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn drain(output: &mut Reader) -> String {
    let mut buf = String::new();
    output.read_to_string(&mut buf).unwrap();
    buf
}

fn cursor(bytes: &'static [u8]) -> Reader {
    Box::new(std::io::Cursor::new(bytes))
}

// ---------------------------------------------------------------------------
// Execution::new — static / mocked execution handle
// ---------------------------------------------------------------------------

#[test]
fn execution_new_exposes_static_output() {
    let mut exec = Execution::new(0, cursor(b"static_content"));
    assert_eq!(drain(&mut exec.output), "static_content");
}

#[tokio::test]
async fn execution_new_wait_returns_given_exit_code() {
    let exec = Execution::new(42, cursor(b""));
    assert_eq!(exec.wait().await.unwrap(), 42);
}

#[tokio::test]
async fn execution_new_wait_drains_unread_output_without_error() {
    // output is never manually read — wait() must drain it.
    let exec = Execution::new(7, cursor(b"unread bytes"));
    assert_eq!(exec.wait().await.unwrap(), 7);
}

// ---------------------------------------------------------------------------
// Environment-Based Mocking (ADR 0024)
// ---------------------------------------------------------------------------

#[test]
fn run_returns_mocked_output_if_env_var_is_set() {
    let _guard = stub_shell("__nonexistent__", 0, "env_mock_data");
    let mut exec = CommandBuilder::new("__nonexistent__", "__nonexistent__").run().unwrap();
    assert_eq!(drain(&mut exec.output), "env_mock_data");
}

// ---------------------------------------------------------------------------
// run() — real process execution
// ---------------------------------------------------------------------------

#[tokio::test]
async fn run_captures_stdout_in_output_reader() {
    let mut exec =
        CommandBuilder::shell_command("io-test-shell-capture", "echo forge_hello").run().unwrap();
    let out = drain(&mut exec.output);
    assert!(out.contains("forge_hello"), "stdout not captured: {out}");
}

#[tokio::test]
async fn run_merges_stderr_into_output_reader() {
    // Write exclusively to stderr — must still appear in exec.output.
    let cmd = if cfg!(windows) { "echo stderr_marker 1>&2" } else { "echo stderr_marker >&2" };
    let mut exec = CommandBuilder::shell_command("io-test-shell-stderr-merge", cmd).run().unwrap();
    let out = drain(&mut exec.output);
    assert!(out.contains("stderr_marker"), "stderr not merged: {out}");
}

#[tokio::test]
async fn run_invalid_binary_returns_spawn_error() {
    let result =
        CommandBuilder::new("io-test-invalid-binary", "__nonexistent_forge_binary_xyz__").run();
    let msg = result.err().expect("expected an error").to_string();
    assert!(msg.contains("Failed to spawn"), "unexpected error: {msg}");
}

#[tokio::test]
async fn run_invalid_workdir_returns_error() {
    let result = CommandBuilder::shell_command("io-test-invalid-workdir", "echo hi")
        .workdir("/path/that/does/not/exist/forge_test")
        .run();
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// wait() — exit code and deadlock resistance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wait_returns_zero_on_successful_command() {
    let exec = CommandBuilder::shell_command("io-test-wait-zero", "echo ok").run().unwrap();
    assert_eq!(exec.wait().await.unwrap(), 0);
}

#[tokio::test]
async fn wait_returns_nonzero_on_failed_command() {
    let exec = CommandBuilder::shell_command("io-test-wait-nonzero", "exit 1").run().unwrap();
    let code = exec.wait().await.unwrap();
    assert_ne!(code, 0);
}

#[tokio::test]
async fn wait_drains_output_automatically_preventing_deadlock() {
    // Never read exec.output manually — wait() must drain it.
    let exec = CommandBuilder::shell_command("io-test-wait-drains", "echo large_output_test")
        .run()
        .unwrap();
    assert_eq!(exec.wait().await.unwrap(), 0);
}

// ---------------------------------------------------------------------------
// workdir
// ---------------------------------------------------------------------------

#[tokio::test]
async fn workdir_valid_directory_succeeds() {
    let wd = std::env::temp_dir();
    let exec = CommandBuilder::shell_command("io-test-workdir-valid", "echo ok")
        .workdir(&wd)
        .run()
        .unwrap();
    assert_eq!(exec.wait().await.unwrap(), 0);
}

// ---------------------------------------------------------------------------
// env / envs / clear_env
// ---------------------------------------------------------------------------

#[tokio::test]
async fn env_injects_single_variable_into_child() {
    let cmd = if cfg!(windows) { "echo %FORGE_SINGLE_VAR%" } else { "echo $FORGE_SINGLE_VAR" };
    let mut exec = CommandBuilder::shell_command("io-test-env-single", cmd)
        .env("FORGE_SINGLE_VAR", "injected_42")
        .run()
        .unwrap();
    let out = drain(&mut exec.output);
    assert!(out.contains("injected_42"), "env var not visible: {out}");
}

#[tokio::test]
async fn envs_injects_multiple_variables_into_child() {
    // `set` is a cmd.exe builtin — use shell_command to wrap it.
    let cmd = if cfg!(windows) { "set" } else { "env" };
    let mut exec = CommandBuilder::shell_command("io-test-env-multi", cmd)
        .clear_env(true)
        .envs([("FORGE_X", "10"), ("FORGE_Y", "20")])
        .run()
        .unwrap();
    let out = drain(&mut exec.output);
    assert!(out.contains("FORGE_X=10"), "FORGE_X not found: {out}");
    assert!(out.contains("FORGE_Y=20"), "FORGE_Y not found: {out}");
}

#[tokio::test]
async fn clear_env_removes_host_variables_from_child() {
    // With clear_env, the injected var must be present and the PATH var should be absent.
    // `set` is a cmd.exe builtin — use shell_command to wrap it.
    let cmd = if cfg!(windows) { "set" } else { "env" };
    let mut exec = CommandBuilder::shell_command("io-test-env-clear", cmd)
        .clear_env(true)
        .env("FORGE_ISOLATED", "only_me")
        .run()
        .unwrap();
    let out = drain(&mut exec.output);
    assert!(out.contains("FORGE_ISOLATED=only_me"), "injected var missing: {out}");
    // PATH should not be visible (host was cleared).
    assert!(!out.to_lowercase().contains("path="), "PATH leaked into cleared env: {out}");
}

// ---------------------------------------------------------------------------
// stdin / InputSource
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stdin_null_default_does_not_block_non_interactive_commands() {
    // If stdin defaulted to Stdio::inherit() on a non-TTY parent, the child
    // might block waiting for input.  With Null it must complete immediately.
    let exec =
        CommandBuilder::shell_command("io-test-stdin-null", "echo stdin_null_ok").run().unwrap();
    assert_eq!(exec.wait().await.unwrap(), 0);
}

#[tokio::test]
async fn stdin_execution_pipes_previous_output_into_next_command() {
    // echo forge_pipe_marker | grep/findstr forge_pipe_marker
    let source = CommandBuilder::shell_command("io-test-pipe-source", "echo forge_pipe_marker")
        .run()
        .unwrap();

    let filter = if cfg!(windows) {
        CommandBuilder::new("io-test-findstr", "findstr").arg("forge_pipe_marker")
    } else {
        CommandBuilder::new("io-test-grep", "grep").arg("forge_pipe_marker")
    };

    let mut piped = filter.stdin(InputSource::Execution(Box::new(source))).run().unwrap();
    let out = drain(&mut piped.output);
    assert!(out.contains("forge_pipe_marker"), "piped output missing: {out}");
}

#[tokio::test]
async fn stdin_execution_pipe_exit_code_reflects_last_command() {
    let source = CommandBuilder::shell_command("io-test-pipe-exit-source", "echo forge_pipe_exit")
        .run()
        .unwrap();

    let filter = if cfg!(windows) {
        CommandBuilder::new("io-test-findstr", "findstr").arg("forge_pipe_exit")
    } else {
        CommandBuilder::new("io-test-grep", "grep").arg("forge_pipe_exit")
    };

    let piped = filter.stdin(InputSource::Execution(Box::new(source))).run().unwrap();
    assert_eq!(piped.wait().await.unwrap(), 0);
}

// ---------------------------------------------------------------------------
// shell_command — platform-aware wrapper
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shell_command_is_platform_aware() {
    let exec =
        CommandBuilder::shell_command("io-test-platform-aware", "echo platform_ok").run().unwrap();
    let mut exec = exec;
    let out = drain(&mut exec.output);
    assert!(out.contains("platform_ok"), "shell_command output: {out}");
}

// ---------------------------------------------------------------------------
// command! macro
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
#[tokio::test]
async fn command_macro_builds_builder_correctly() {
    let exec = command!("io-test-macro", "echo", "macro_marker").run().unwrap();
    assert_eq!(exec.wait().await.unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Drop / zombie prevention
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dropping_execution_before_wait_does_not_panic() {
    // A long-running command is dropped immediately.
    // kill_on_drop ensures the process is terminated — if it leaked,
    // the process table would accumulate zombies across test runs.
    let cmd = if cfg!(windows) { "ping -n 60 127.0.0.1" } else { "sleep 60" };
    let exec = CommandBuilder::shell_command("io-test-drop-exec", cmd).run().unwrap();
    drop(exec);
    // Reaching here without a timeout means the drop path is non-blocking.
}
