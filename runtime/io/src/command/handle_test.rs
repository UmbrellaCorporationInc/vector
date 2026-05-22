#![allow(clippy::expect_used)]

use crate::command::{
    CommandBuilder, CommandExecutor, CommandExit, MockCommandHandleBuilder, ProcessCommandExecutor,
};
use runtime_core::Receiver;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[tokio::test]
async fn test_wait_reports_exit_status() {
    let spec = CommandBuilder::new(if cfg!(windows) { "cmd" } else { "sh" })
        .arg(if cfg!(windows) { "/C" } else { "-c" })
        .arg("exit 7")
        .build()
        .expect("build failed");

    let executor = ProcessCommandExecutor;
    let handle = executor.spawn(spec).await.expect("spawn failed");
    let exit = handle.wait().await.expect("wait failed");

    assert!(!exit.success);
    assert_eq!(exit.code, Some(7));
}

#[tokio::test]
async fn test_stderr_reads_process_output() {
    let spec = CommandBuilder::new(if cfg!(windows) { "cmd" } else { "sh" })
        .arg(if cfg!(windows) { "/C" } else { "-c" })
        .arg(if cfg!(windows) { "echo vector-error 1>&2" } else { "printf 'vector-error\\n' >&2" })
        .build()
        .expect("build failed");

    let executor = ProcessCommandExecutor;
    let mut handle = executor.spawn(spec).await.expect("spawn failed");

    let mut output = String::new();
    while let Ok(Some(chunk)) = handle.stderr().recv().await {
        output.push_str(std::str::from_utf8(&chunk).expect("stderr should be utf-8"));
    }

    let exit = handle.wait().await.expect("wait failed");

    assert!(output.contains("vector-error"));
    assert!(exit.success);
}

#[tokio::test]
async fn test_stream_output_forwards_stdout_and_stderr() {
    let (mut handle, _probe) = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout(b"hello stdout\n".to_vec())
        .stderr(b"hello stderr\n".to_vec())
        .build();

    let mut captured_stdout = Vec::new();
    let mut captured_stderr = Vec::new();

    handle
        .stream_output(&mut |b: &[u8]| captured_stdout.extend_from_slice(b), &mut |b: &[u8]| {
            captured_stderr.extend_from_slice(b);
        })
        .await;

    let exit = handle.wait().await.expect("wait failed");

    assert!(exit.success);
    assert_eq!(captured_stdout, b"hello stdout\n");
    assert_eq!(captured_stderr, b"hello stderr\n");
}

#[tokio::test]
async fn test_stream_output_tolerates_empty_streams() {
    let (mut handle, _probe) =
        MockCommandHandleBuilder::new(CommandExit::new(true, Some(0))).build();

    let mut captured_stdout = Vec::new();
    let mut captured_stderr = Vec::new();

    handle
        .stream_output(&mut |b: &[u8]| captured_stdout.extend_from_slice(b), &mut |b: &[u8]| {
            captured_stderr.extend_from_slice(b);
        })
        .await;

    let exit = handle.wait().await.expect("wait failed");

    assert!(exit.success);
    assert!(captured_stdout.is_empty());
    assert!(captured_stderr.is_empty());
}

#[tokio::test]
async fn test_drop_cleans_up_long_running_process() {
    let temp_dir = create_temp_directory();
    let marker_name = "command-drop-marker.txt";
    let script = if cfg!(windows) {
        format!("timeout /T 2 /NOBREAK >NUL && echo done > {marker_name}")
    } else {
        format!("sleep 2 && printf done > {marker_name}")
    };

    let spec = CommandBuilder::new(if cfg!(windows) { "cmd" } else { "sh" })
        .arg(if cfg!(windows) { "/C" } else { "-c" })
        .arg(script)
        .current_dir(&temp_dir)
        .build()
        .expect("build failed");

    let executor = ProcessCommandExecutor;
    let handle = executor.spawn(spec).await.expect("spawn failed");

    drop(handle);
    thread::sleep(Duration::from_secs(3));

    assert!(!temp_dir.join(marker_name).exists());

    fs::remove_dir_all(&temp_dir).expect("temp directory cleanup failed");
}

fn create_temp_directory() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("vector-runtime-io-command-{unique}"));
    fs::create_dir_all(&path).expect("temp directory creation failed");
    path
}
