#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use runtime_rag::{
    HybridSearchOutput, HybridSearchResult, RetrievalContext, RetrievalContextChunk,
    RetrievalContextDiagnostics, RetrievalContextSource, RetrievalMatchReason,
};
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

fn sample_context() -> RetrievalContext {
    RetrievalContext::new(
        "hybrid retrieval".to_owned(),
        3,
        vec![RetrievalContextSource::new(
            "src-1".to_owned(),
            Some("shared-docs".to_owned()),
            "task-00070-implement-rfc-00040-phase-8-hybrid-search".to_owned(),
            vec!["Phase D".to_owned()],
            "shared-docs/task-00070-implement-rfc-00040-phase-8-hybrid-search > Phase D".to_owned(),
        )],
        vec![RetrievalContextChunk::new(
            "ctx-1".to_owned(),
            "src-1".to_owned(),
            Some("shared-docs".to_owned()),
            "task-00070-implement-rfc-00040-phase-8-hybrid-search".to_owned(),
            vec!["Phase D".to_owned()],
            "chunk-001".to_owned(),
            1,
            "Hybrid retrieval result text.".to_owned(),
            42,
            RetrievalMatchReason::Primary,
        )],
        RetrievalContextDiagnostics::new(42, 0, 3),
    )
}

fn empty_context() -> RetrievalContext {
    RetrievalContext::empty("no matches".to_owned(), 3)
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
fn render_human_output_includes_context_identity_and_text() {
    let rendered = render_human_output(&sample_context());

    assert!(rendered.contains("Retrieval Context"));
    assert!(rendered.contains("status=has_results query='hybrid retrieval' limit=3 returned=1"));
    assert!(rendered.contains("Sources:"));
    assert!(rendered.contains("- src-1 [shared-docs]"));
    assert!(
        rendered.contains("[shared-docs] task-00070-implement-rfc-00040-phase-8-hybrid-search")
    );
    assert!(rendered.contains("Chunks:"));
    assert!(rendered.contains("- ctx-1 [shared-docs]"));
    assert!(rendered.contains("source=src-1 chunk=chunk-001 ordinal=1 tokens=42"));
    assert!(rendered.contains("match_reason=primary"));
    assert!(rendered.contains("Hybrid retrieval result text."));
    assert!(rendered.contains("Diagnostics:"));
    assert!(rendered.contains("total_token_count=42 dropped_after_limit=0 retrieval_limit=3"));
}

#[test]
fn render_human_output_preserves_empty_retrieval_as_successful_context() {
    let rendered = render_human_output(&empty_context());

    assert!(rendered.contains("status=empty query='no matches' limit=3 returned=0"));
    assert!(rendered.contains("Sources:\n\n- none"));
    assert!(rendered.contains("Chunks:\n\n- none"));
    assert!(rendered.contains("total_token_count=0 dropped_after_limit=0 retrieval_limit=3"));
}

#[test]
fn render_json_output_serializes_retrieval_context_payload() {
    let rendered = render_json_output(&sample_context()).expect("json rendering should succeed");
    let payload: Value = serde_json::from_str(&rendered).expect("json payload should parse");

    assert_eq!(payload["query"], "hybrid retrieval");
    assert_eq!(payload["status"], "has_results");
    assert_eq!(payload["limit"], 3);
    assert_eq!(payload["returned"], 1);
    assert_eq!(payload["sources"][0]["package"], "shared-docs");
    assert_eq!(payload["sources"][0]["source_id"], "src-1");
    assert_eq!(
        payload["sources"][0]["citation_label"],
        "shared-docs/task-00070-implement-rfc-00040-phase-8-hybrid-search > Phase D"
    );
    assert_eq!(payload["chunks"][0]["context_id"], "ctx-1");
    assert_eq!(payload["chunks"][0]["source_id"], "src-1");
    assert_eq!(payload["chunks"][0]["chunk_id"], "chunk-001");
    assert_eq!(payload["chunks"][0]["token_count"], 42);
    assert_eq!(payload["chunks"][0]["match_reason"], "primary");
    assert_eq!(payload["diagnostics"]["total_token_count"], 42);
    assert_eq!(payload["diagnostics"]["dropped_after_limit"], 0);
    assert_eq!(payload["diagnostics"]["retrieval_limit"], 3);
}

#[test]
fn render_json_output_preserves_empty_retrieval_context() {
    let rendered = render_json_output(&empty_context()).expect("json rendering should succeed");
    let payload: Value = serde_json::from_str(&rendered).expect("json payload should parse");

    assert_eq!(payload["query"], "no matches");
    assert_eq!(payload["status"], "empty");
    assert_eq!(payload["limit"], 3);
    assert_eq!(payload["returned"], 0);
    assert_eq!(payload["sources"], serde_json::json!([]));
    assert_eq!(payload["chunks"], serde_json::json!([]));
    assert_eq!(payload["diagnostics"]["retrieval_limit"], 3);
}

#[tokio::test]
async fn assemble_retrieval_context_converts_hybrid_search_output_before_rendering() {
    let context =
        assemble_retrieval_context(sample_output()).await.expect("context assembly should succeed");

    assert_eq!(context.query, "hybrid retrieval");
    assert_eq!(context.limit, 3);
    assert_eq!(context.returned, 1);
    assert_eq!(context.sources[0].source_id, "src-1");
    assert_eq!(context.chunks[0].context_id, "ctx-1");
    assert_eq!(context.chunks[0].chunk_id, "chunk-001");
    assert_eq!(context.chunks[0].match_reason, RetrievalMatchReason::Primary);
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
