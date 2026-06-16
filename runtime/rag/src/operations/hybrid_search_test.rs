#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_fixture_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    std::env::temp_dir().join(format!("vector-rag-search-op-test-{label}-{nanos}"))
}

#[tokio::test]
async fn hybrid_search_op_resolves_governed_default_limit_through_dispatcher() {
    let input = HybridSearchInput::new(
        unique_fixture_root("default-limit"),
        RagDefaults::phase_one(),
        "  hybrid retrieval  ".to_owned(),
        None,
        None,
        None,
    );

    let (_cancel, mut receiver) = PluginDispatcher::new(HybridSearchOp::new())
        .input(input)
        .build()
        .expect("dispatcher build failed");

    let output = receiver.recv().await.expect("channel error").expect("missing output");

    assert_eq!(output.query_text, "hybrid retrieval");
    assert_eq!(output.result_limit, RagDefaults::phase_one().final_retrieval_limit());
    assert!(output.results.is_empty(), "Phase A only defines the boundary contract");
}

#[tokio::test]
async fn hybrid_search_op_preserves_explicit_filters_and_limit() {
    let input = HybridSearchInput::new(
        unique_fixture_root("explicit-filters"),
        RagDefaults::phase_one(),
        "query".to_owned(),
        Some("shared-docs".to_owned()),
        Some("rfc-00040-phase-8-hybrid-search".to_owned()),
        Some(3),
    );

    let (_cancel, mut receiver) = PluginDispatcher::new(HybridSearchOp::new())
        .input(input)
        .build()
        .expect("dispatcher build failed");

    let output = receiver.recv().await.expect("channel error").expect("missing output");

    assert_eq!(output.package_filter.as_deref(), Some("shared-docs"));
    assert_eq!(output.document_filter.as_deref(), Some("rfc-00040-phase-8-hybrid-search"));
    assert_eq!(output.result_limit, 3);
}

#[tokio::test]
async fn hybrid_search_op_rejects_blank_query_text() {
    let input = HybridSearchInput::new(
        unique_fixture_root("blank-query"),
        RagDefaults::phase_one(),
        "   ".to_owned(),
        None,
        None,
        None,
    );

    let (_cancel, mut receiver) =
        PluginDispatcher::new(HybridSearchOp::new()).input(input).build().unwrap();

    let result = receiver.recv().await;
    assert!(result.is_err(), "expected validation failure for blank query text");
}

#[tokio::test]
async fn hybrid_search_op_rejects_blank_filters_and_zero_limit() {
    let input = HybridSearchInput::new(
        unique_fixture_root("invalid-filters"),
        RagDefaults::phase_one(),
        "query".to_owned(),
        Some(" ".to_owned()),
        None,
        Some(0),
    );

    let (_cancel, mut receiver) =
        PluginDispatcher::new(HybridSearchOp::new()).input(input).build().unwrap();

    let result = receiver.recv().await;
    assert!(result.is_err(), "expected validation failure for invalid filter or limit");
}

#[tokio::test]
async fn hybrid_search_op_receiver_is_none_after_single_output() {
    let input = HybridSearchInput::new(
        unique_fixture_root("single-output"),
        RagDefaults::phase_one(),
        "query".to_owned(),
        None,
        None,
        Some(2),
    );

    let (_cancel, mut receiver) =
        PluginDispatcher::new(HybridSearchOp::new()).input(input).build().unwrap();

    let first = receiver.recv().await;
    let second = receiver.recv().await;

    assert!(first.is_ok());
    assert!(first.unwrap().is_some(), "expected one output");
    assert!(second.is_ok(), "channel error on second recv");
    assert!(second.unwrap().is_none(), "channel must be closed after single output");
}
