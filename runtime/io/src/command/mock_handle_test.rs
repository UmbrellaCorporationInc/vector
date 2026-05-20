#![allow(clippy::expect_used)]

use super::MockCommandHandleBuilder;
use crate::command::CommandExit;
use runtime_core::{Receiver, Sender};

#[tokio::test]
async fn test_mock_handle_builder_replays_output_and_records_input() {
    let (mut handle, probe) =
        MockCommandHandleBuilder::new(CommandExit { success: true, code: Some(0) })
            .stdout(b"alpha".to_vec())
            .stdout(b"beta".to_vec())
            .stderr(b"warn".to_vec())
            .build();

    handle.stdin().send(b"input".to_vec()).await.expect("stdin send should succeed");

    let first_stdout = handle
        .stdout()
        .recv()
        .await
        .expect("recv should not fail")
        .expect("first stdout chunk should exist");
    let second_stdout = handle
        .stdout()
        .recv()
        .await
        .expect("recv should not fail")
        .expect("second stdout chunk should exist");
    let stderr = handle
        .stderr()
        .recv()
        .await
        .expect("recv should not fail")
        .expect("stderr chunk should exist");
    let exit = handle.wait().await.expect("wait should succeed");

    assert_eq!(first_stdout, b"alpha".to_vec());
    assert_eq!(second_stdout, b"beta".to_vec());
    assert_eq!(stderr, b"warn".to_vec());
    assert!(exit.success);
    assert_eq!(exit.code, Some(0));
    assert_eq!(probe.recorded_stdin().expect("stdin should be recorded"), vec![b"input".to_vec()]);
}

#[tokio::test]
async fn test_mock_handle_builder_can_fail_wait() {
    let (handle, _probe) =
        MockCommandHandleBuilder::new(CommandExit { success: true, code: Some(0) })
            .wait_error("planned wait failure")
            .build();

    let error = handle.wait().await.expect_err("wait should fail");

    assert!(matches!(error, crate::IoError::Process(_)));
}
