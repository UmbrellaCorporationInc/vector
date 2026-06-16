#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use runtime_rag::{HybridSearchOutput, HybridSearchResult};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_result() -> HybridSearchResult {
    HybridSearchResult::new(
        Some("shared-docs".to_owned()),
        "task-00070-implement-rfc-00040-phase-8-hybrid-search".to_owned(),
        vec!["Phase D".to_owned()],
        "chunk-001".to_owned(),
        1,
        "Hybrid retrieval result text.".to_owned(),
        42,
        Some(2),
        Some(1),
        0.0325,
        Some("chunk-000".to_owned()),
        Some("chunk-002".to_owned()),
        false,
        None,
    )
}

fn sample_output() -> HybridSearchOutput {
    HybridSearchOutput::new(
        "hybrid retrieval".to_owned(),
        Some("shared-docs".to_owned()),
        Some("task-00070-implement-rfc-00040-phase-8-hybrid-search".to_owned()),
        3,
        vec![sample_result()],
    )
}

fn unique_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let root = std::env::temp_dir().join(format!("vector-cli-rag-search-{label}-{nanos}"));
    std::fs::create_dir_all(root.join(".vector")).expect("failed to create .vector root");
    root
}

#[test]
fn parse_args_accepts_query_filters_limit_and_json_flag() {
    let args = vec![
        "hybrid".to_owned(),
        "retrieval".to_owned(),
        "--limit".to_owned(),
        "5".to_owned(),
        "--package".to_owned(),
        "shared-docs".to_owned(),
        "--document".to_owned(),
        "task-00070-implement-rfc-00040-phase-8-hybrid-search".to_owned(),
        "--json".to_owned(),
    ];

    let parsed = parse_args(&args).expect("rag search args should parse");

    assert_eq!(parsed.query_text, "hybrid retrieval");
    assert_eq!(parsed.result_limit, Some(5));
    assert_eq!(parsed.package_filter.as_deref(), Some("shared-docs"));
    assert_eq!(
        parsed.document_filter.as_deref(),
        Some("task-00070-implement-rfc-00040-phase-8-hybrid-search")
    );
    assert!(parsed.json_output);
}

#[test]
fn parse_args_rejects_missing_query_and_zero_limit() {
    let no_query_error = parse_args(&["--json".to_owned()]).unwrap_err();
    assert!(no_query_error.contains("missing search query"));

    let zero_limit_error =
        parse_args(&["query".to_owned(), "--limit".to_owned(), "0".to_owned()]).unwrap_err();
    assert!(zero_limit_error.contains("--limit"));
}

#[test]
fn render_human_output_includes_identity_score_and_text() {
    let rendered = render_human_output(&sample_output());

    assert!(rendered.contains("Retrieved 1 result(s) for 'hybrid retrieval'"));
    assert!(
        rendered.contains("[shared-docs] task-00070-implement-rfc-00040-phase-8-hybrid-search")
    );
    assert!(rendered.contains("score=0.032500"));
    assert!(rendered.contains("Hybrid retrieval result text."));
}

#[test]
fn render_json_output_preserves_machine_readable_payload() {
    let rendered = render_json_output(&sample_output()).expect("json rendering should succeed");
    let payload: Value = serde_json::from_str(&rendered).expect("json payload should parse");

    assert_eq!(payload["query_text"], "hybrid retrieval");
    assert_eq!(payload["package_filter"], "shared-docs");
    assert_eq!(payload["results"][0]["chunk_id"], "chunk-001");
    assert_eq!(payload["results"][0]["lexical_rank"], 1);
}

#[tokio::test]
async fn rag_search_returns_actionable_error_when_store_is_missing() {
    let root = unique_root("missing-store");
    let error = run(
        &root,
        RagSearchArgs {
            query_text: "hybrid retrieval".to_owned(),
            package_filter: None,
            document_filter: None,
            result_limit: Some(3),
            json_output: false,
        },
    )
    .await
    .unwrap_err();

    assert!(error.contains("RAG store is missing"), "unexpected error: {error}");
}
