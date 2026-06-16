#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_fixture_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    std::env::temp_dir().join(format!("vector-index-workspace-test-{label}-{nanos}"))
}

#[tokio::test]
async fn index_workspace_op_initializes_store_and_delivers_index_result() {
    let root_dir = unique_fixture_root("basic");
    let input = IndexWorkspaceInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut receiver) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(input)
        .build()
        .expect("dispatcher build failed");

    let output = receiver.recv().await.expect("channel error").expect("no output from operation");

    assert_eq!(output.result.skipped_count, 0);
    assert_eq!(output.result.reindexed_count, 0);
    assert_eq!(output.result.deleted_count, 0);
    assert!(output.result.failures.is_empty());
    assert!(!output.result.has_failures());
}

#[tokio::test]
async fn index_workspace_op_is_idempotent_on_repeated_runs() {
    let root_dir = unique_fixture_root("idempotent");
    let make_input = || IndexWorkspaceInput::new(root_dir.clone(), RagDefaults::phase_one());

    let (_cancel, mut rx1) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(make_input())
        .build()
        .expect("first dispatcher build failed");
    let first = rx1.recv().await.unwrap().unwrap();

    let (_cancel, mut rx2) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(make_input())
        .build()
        .expect("second dispatcher build failed");
    let second = rx2.recv().await.unwrap().unwrap();

    assert!(!first.result.has_failures());
    assert!(!second.result.has_failures());
}

#[tokio::test]
async fn index_workspace_op_receiver_is_none_after_single_output() {
    let root_dir = unique_fixture_root("single-output");
    let input = IndexWorkspaceInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut rx) =
        PluginDispatcher::new(IndexWorkspaceOp::new()).input(input).build().unwrap();

    let first = rx.recv().await;
    let second = rx.recv().await;

    assert!(first.is_ok());
    assert!(first.unwrap().is_some(), "expected one output");
    assert!(second.is_ok(), "channel error on second recv");
    assert!(second.unwrap().is_none(), "channel must be closed after single output");
}
