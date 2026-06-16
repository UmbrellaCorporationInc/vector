#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_fixture_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    std::env::temp_dir().join(format!("vector-rag-indexer-test-{label}-{nanos}"))
}

#[tokio::test]
async fn rag_indexer_op_delivers_empty_index_result_through_dispatcher() {
    let root_dir = unique_fixture_root("empty-result");
    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut receiver) = PluginDispatcher::new(RagIndexerOp::new())
        .input(input)
        .build()
        .expect("dispatcher build failed");

    let result = receiver.recv().await.expect("channel error").expect("no output from operation");

    assert_eq!(result.skipped_count, 0);
    assert_eq!(result.reindexed_count, 0);
    assert_eq!(result.deleted_count, 0);
    assert!(result.failures.is_empty());
    assert!(!result.has_failures());
}

#[tokio::test]
async fn rag_indexer_op_receiver_is_none_after_single_output() {
    let root_dir = unique_fixture_root("single-output");
    let input = RagIndexerInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut rx) =
        PluginDispatcher::new(RagIndexerOp::new()).input(input).build().unwrap();

    let first = rx.recv().await;
    let second = rx.recv().await;

    assert!(first.is_ok());
    assert!(first.unwrap().is_some(), "expected one output");
    assert!(second.is_ok(), "channel error on second recv");
    assert!(second.unwrap().is_none(), "channel must be closed after single output");
}

#[test]
fn index_result_has_failures_returns_false_for_empty_failures() {
    let result = IndexResult::default();
    assert!(!result.has_failures());
}

#[test]
fn index_result_has_failures_returns_true_when_failures_present() {
    let result = IndexResult {
        failures: vec![IndexFailureRecord {
            package: None,
            document_stem: "task-00001-example".to_owned(),
            error: "extraction failed".to_owned(),
        }],
        ..IndexResult::default()
    };
    assert!(result.has_failures());
}

#[test]
fn index_failure_record_carries_package_document_stem_and_error() {
    let record = IndexFailureRecord {
        package: Some("my-pkg".to_owned()),
        document_stem: "rfc-00001-my-rfc".to_owned(),
        error: "chunking failed: oversized block".to_owned(),
    };
    assert_eq!(record.package.as_deref(), Some("my-pkg"));
    assert_eq!(record.document_stem, "rfc-00001-my-rfc");
    assert!(record.error.contains("oversized"));
}
