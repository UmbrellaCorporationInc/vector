#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_fixture_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    std::env::temp_dir().join(format!("vector-rag-op-test-{label}-{nanos}"))
}

#[tokio::test]
async fn init_rag_store_op_delivers_output_through_dispatcher() {
    let root_dir = unique_fixture_root("dispatcher-basic");
    let input = InitRagStoreInput::new(root_dir, "BGESmallENV15".to_owned(), 384);

    let (_cancel, mut receiver) = PluginDispatcher::new(InitRagStoreOp::new())
        .input(input)
        .build()
        .expect("dispatcher build failed");

    let output = receiver.recv().await.expect("channel error").expect("no output from operation");

    assert!(!output.database_dir.as_os_str().is_empty());
    assert!(!output.table_name.is_empty());
    assert!(output.created_table);
    assert!(output.created_text_index);
}

#[tokio::test]
async fn init_rag_store_op_is_idempotent_through_dispatcher() {
    let root_dir = unique_fixture_root("dispatcher-idempotent");
    let make_input = || InitRagStoreInput::new(root_dir.clone(), "BGESmallENV15".to_owned(), 384);

    let (_cancel, mut rx1) = PluginDispatcher::new(InitRagStoreOp::new())
        .input(make_input())
        .build()
        .expect("first dispatcher build failed");
    let first = rx1.recv().await.unwrap().unwrap();

    let (_cancel, mut rx2) = PluginDispatcher::new(InitRagStoreOp::new())
        .input(make_input())
        .build()
        .expect("second dispatcher build failed");
    let second = rx2.recv().await.unwrap().unwrap();

    assert!(first.created_table);
    assert!(!second.created_table);
    assert_eq!(first.database_dir, second.database_dir);
    assert_eq!(first.table_name, second.table_name);
}

#[tokio::test]
async fn init_rag_store_op_surfaces_error_for_incompatible_contract_through_dispatcher() {
    let root_dir = unique_fixture_root("dispatcher-incompatible");

    let baseline = InitRagStoreInput::new(root_dir.clone(), "BGESmallENV15".to_owned(), 384);
    let (_cancel, mut rx) =
        PluginDispatcher::new(InitRagStoreOp::new()).input(baseline).build().unwrap();
    rx.recv().await.unwrap().unwrap();

    let incompatible = InitRagStoreInput::new(root_dir, "DifferentModel".to_owned(), 768);
    let (_cancel, mut rx2) =
        PluginDispatcher::new(InitRagStoreOp::new()).input(incompatible).build().unwrap();
    let result = rx2.recv().await;

    assert!(result.is_err(), "expected error for incompatible embedding contract; got: {result:?}");
    let message = result.unwrap_err().to_string();
    assert!(
        message.contains("incompatible") || message.contains("DifferentModel"),
        "error must describe the incompatible contract; got: {message}"
    );
}

#[tokio::test]
async fn init_rag_store_op_surfaces_error_for_invalid_input_through_dispatcher() {
    let input = InitRagStoreInput::new(
        std::env::temp_dir().join("vector-rag-op-invalid"),
        String::new(),
        0,
    );

    let (_cancel, mut rx) =
        PluginDispatcher::new(InitRagStoreOp::new()).input(input).build().unwrap();

    let result = rx.recv().await;
    assert!(result.is_err(), "expected error for empty embedding_model; got: {result:?}");
}

#[tokio::test]
async fn init_rag_store_op_receiver_is_none_after_single_output() {
    let root_dir = unique_fixture_root("dispatcher-single-output");
    let input = InitRagStoreInput::new(root_dir, "BGESmallENV15".to_owned(), 384);

    let (_cancel, mut rx) =
        PluginDispatcher::new(InitRagStoreOp::new()).input(input).build().unwrap();

    let first = rx.recv().await;
    let second = rx.recv().await;

    assert!(first.is_ok());
    assert!(first.unwrap().is_some(), "expected one output");
    assert!(second.is_ok(), "channel error on second recv");
    assert!(second.unwrap().is_none(), "channel must be closed after single output");
}
