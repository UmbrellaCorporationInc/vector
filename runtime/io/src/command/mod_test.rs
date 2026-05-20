#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;
use crate::Bytes;
use runtime_core::{Receiver, Sender};
use std::{
    future::ready,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default, Clone)]
struct RecordingExecutor {
    seen_specs: Arc<Mutex<Vec<CommandSpec>>>,
}

impl RecordingExecutor {
    fn take_specs(&self) -> Vec<CommandSpec> {
        let mut specs = self.seen_specs.lock().unwrap();
        std::mem::take(&mut *specs)
    }
}

impl CommandExecutor for RecordingExecutor {
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl std::future::Future<Output = Result<CommandHandle, crate::IoError>> + Send {
        self.seen_specs.lock().unwrap().push(spec);
        ready(Err(crate::IoError::Process("mock executor".into())))
    }
}

#[test]
fn test_command_api_visibility() {
    let spec = CommandBuilder::new("echo")
        .arg("hello")
        .env("VECTOR_MODE", "test")
        .build()
        .expect("build failed");

    assert_eq!(spec.command(), "echo");
    assert_eq!(spec.args(), ["hello"]);
    assert_eq!(spec.env(), [("VECTOR_MODE".to_string(), "test".to_string())]);
    assert!(spec.current_dir().is_none());
}

#[test]
fn test_command_builder_builds_without_process_side_effects() {
    let spec = CommandBuilder::new("definitely-not-a-real-command")
        .arg("--still-builds")
        .build()
        .expect("build failed");

    assert_eq!(spec.command(), "definitely-not-a-real-command");
    assert_eq!(spec.args(), ["--still-builds"]);
}

#[tokio::test]
async fn test_running_command_boundary_still_satisfies_sender_and_receiver_contracts() {
    async fn read_from_receiver<R>(receiver: &mut R) -> Option<Bytes>
    where
        R: Receiver<Bytes>,
    {
        receiver.recv().await.ok().flatten()
    }

    async fn write_to_sender<S>(sender: &mut S, value: Bytes)
    where
        S: Sender<Bytes>,
    {
        sender.send(value).await.expect("send failed");
    }

    let spec = CommandBuilder::new(if cfg!(windows) { "findstr" } else { "cat" })
        .arg(if cfg!(windows) { "^" } else { "-" })
        .build()
        .expect("build failed");

    let executor = ProcessCommandExecutor;
    let mut handle = executor.spawn(spec).await.expect("spawn failed");

    write_to_sender(handle.stdin(), b"boundary_check".to_vec()).await;
    handle.stdin().close();

    let chunk = read_from_receiver(handle.stdout()).await.expect("stdout should produce one chunk");
    assert!(std::str::from_utf8(&chunk).unwrap().contains("boundary_check"));

    let exit = handle.wait().await.expect("wait failed");
    assert!(exit.success);
}

#[tokio::test]
async fn test_command_executor_substitution_keeps_spec_model() {
    let executor = RecordingExecutor::default();
    let spec = CommandBuilder::new("echo").arg("hello").build().expect("build failed");

    let error = executor.spawn(spec.clone()).await.expect_err("spawn should fail");

    assert!(matches!(error, crate::IoError::Process(_)));
    assert_eq!(executor.take_specs(), vec![spec]);
}

#[tokio::test]
async fn test_command_spawn_and_wait() {
    let spec = CommandBuilder::new(if cfg!(windows) { "cmd" } else { "sh" })
        .arg(if cfg!(windows) { "/C" } else { "-c" })
        .arg("exit 0")
        .build()
        .expect("build failed");

    let executor = ProcessCommandExecutor;
    let handle = executor.spawn(spec).await.expect("spawn failed");
    let exit = handle.wait().await.expect("wait failed");

    assert!(exit.success);
    assert_eq!(exit.code, Some(0));
}

#[tokio::test]
async fn test_command_stdout() {
    let spec = CommandBuilder::new(if cfg!(windows) { "cmd" } else { "sh" })
        .arg(if cfg!(windows) { "/C" } else { "-c" })
        .arg("echo hello vector")
        .build()
        .expect("build failed");

    let executor = ProcessCommandExecutor;
    let mut handle = executor.spawn(spec).await.expect("spawn failed");

    let mut output = String::new();
    while let Ok(Some(chunk)) = handle.stdout().recv().await {
        output.push_str(std::str::from_utf8(&chunk).unwrap());
    }

    assert!(output.contains("hello vector"));

    let exit = handle.wait().await.expect("wait failed");
    assert!(exit.success);
}

#[tokio::test]
async fn test_command_stdin() {
    let spec = CommandBuilder::new(if cfg!(windows) { "findstr" } else { "cat" })
        .arg(if cfg!(windows) { "^" } else { "-" })
        .build()
        .expect("build failed");

    let executor = ProcessCommandExecutor;
    let mut handle = executor.spawn(spec).await.expect("spawn failed");

    handle.stdin().send(b"test_input_123".to_vec()).await.expect("send failed");
    handle.stdin().close();

    let mut output = String::new();
    while let Ok(Some(chunk)) = handle.stdout().recv().await {
        output.push_str(std::str::from_utf8(&chunk).unwrap());
    }

    assert!(output.contains("test_input_123"));
    let _exit = handle.wait().await.expect("wait failed");
}
