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

fn expanded_context() -> RetrievalContext {
    RetrievalContext::new(
        "expanded context".to_owned(),
        4,
        vec![
            RetrievalContextSource::new(
                "src-1".to_owned(),
                Some("shared-docs".to_owned()),
                "spec-00011-rag-plan-implementation".to_owned(),
                vec!["Phase 9".to_owned()],
                "shared-docs/spec-00011-rag-plan-implementation > Phase 9".to_owned(),
            ),
            RetrievalContextSource::new(
                "src-2".to_owned(),
                None,
                "rfc-00041-phase-9-canonical-result-for-retrieval-operation".to_owned(),
                vec!["Source Attribution Contract".to_owned()],
                "rfc-00041-phase-9-canonical-result-for-retrieval-operation > Source Attribution Contract".to_owned(),
            ),
        ],
        vec![
            RetrievalContextChunk::new(
                "ctx-1".to_owned(),
                "src-1".to_owned(),
                Some("shared-docs".to_owned()),
                "spec-00011-rag-plan-implementation".to_owned(),
                vec!["Phase 9".to_owned()],
                "chunk-package-primary".to_owned(),
                7,
                "Package primary evidence.".to_owned(),
                11,
                RetrievalMatchReason::Primary,
            ),
            RetrievalContextChunk::new(
                "ctx-2".to_owned(),
                "src-1".to_owned(),
                Some("shared-docs".to_owned()),
                "spec-00011-rag-plan-implementation".to_owned(),
                vec!["Phase 9".to_owned()],
                "chunk-package-expanded".to_owned(),
                8,
                "Package expanded evidence.".to_owned(),
                13,
                RetrievalMatchReason::Expanded,
            ),
            RetrievalContextChunk::new(
                "ctx-3".to_owned(),
                "src-2".to_owned(),
                None,
                "rfc-00041-phase-9-canonical-result-for-retrieval-operation".to_owned(),
                vec!["Source Attribution Contract".to_owned()],
                "chunk-workspace-primary".to_owned(),
                2,
                "Workspace primary evidence.".to_owned(),
                17,
                RetrievalMatchReason::Primary,
            ),
        ],
        RetrievalContextDiagnostics::new(41, 2, 4),
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
fn render_human_output_includes_primary_expanded_repeated_sources_packages_and_diagnostics() {
    let rendered = render_human_output(&expanded_context());

    assert!(rendered.contains("status=has_results query='expanded context' limit=4 returned=3"));
    assert!(rendered.contains(
        "- src-1 [shared-docs] spec-00011-rag-plan-implementation :: Phase 9 \
         (shared-docs/spec-00011-rag-plan-implementation > Phase 9)"
    ));
    assert!(rendered.contains(
        "- src-2 [<workspace>] rfc-00041-phase-9-canonical-result-for-retrieval-operation \
         :: Source Attribution Contract \
         (rfc-00041-phase-9-canonical-result-for-retrieval-operation > Source Attribution Contract)"
    ));
    assert!(
        rendered.contains("- ctx-1 [shared-docs] spec-00011-rag-plan-implementation :: Phase 9")
    );
    assert!(rendered.contains(
        "source=src-1 chunk=chunk-package-primary ordinal=7 tokens=11 match_reason=primary"
    ));
    assert!(rendered.contains(
        "source=src-1 chunk=chunk-package-expanded ordinal=8 tokens=13 match_reason=expanded"
    ));
    assert!(rendered.contains(
        "source=src-2 chunk=chunk-workspace-primary ordinal=2 tokens=17 match_reason=primary"
    ));
    assert!(rendered.contains("Package primary evidence."));
    assert!(rendered.contains("Package expanded evidence."));
    assert!(rendered.contains("Workspace primary evidence."));
    assert!(rendered.contains("total_token_count=41 dropped_after_limit=2 retrieval_limit=4"));
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
fn render_json_output_serializes_full_phase_nine_canonical_context_shape() {
    let rendered = render_json_output(&expanded_context()).expect("json rendering should succeed");
    let payload: Value = serde_json::from_str(&rendered).expect("json payload should parse");

    assert_eq!(payload["query"], "expanded context");
    assert_eq!(payload["status"], "has_results");
    assert_eq!(payload["limit"], 4);
    assert_eq!(payload["returned"], 3);
    assert_eq!(
        payload["sources"],
        serde_json::json!([
            {
                "source_id": "src-1",
                "package": "shared-docs",
                "document_stem": "spec-00011-rag-plan-implementation",
                "heading_path": ["Phase 9"],
                "citation_label": "shared-docs/spec-00011-rag-plan-implementation > Phase 9"
            },
            {
                "source_id": "src-2",
                "package": null,
                "document_stem": "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
                "heading_path": ["Source Attribution Contract"],
                "citation_label": "rfc-00041-phase-9-canonical-result-for-retrieval-operation > Source Attribution Contract"
            }
        ])
    );
    assert_eq!(
        payload["chunks"],
        serde_json::json!([
            {
                "context_id": "ctx-1",
                "source_id": "src-1",
                "package": "shared-docs",
                "document_stem": "spec-00011-rag-plan-implementation",
                "heading_path": ["Phase 9"],
                "chunk_id": "chunk-package-primary",
                "chunk_ordinal": 7,
                "text": "Package primary evidence.",
                "token_count": 11,
                "match_reason": "primary"
            },
            {
                "context_id": "ctx-2",
                "source_id": "src-1",
                "package": "shared-docs",
                "document_stem": "spec-00011-rag-plan-implementation",
                "heading_path": ["Phase 9"],
                "chunk_id": "chunk-package-expanded",
                "chunk_ordinal": 8,
                "text": "Package expanded evidence.",
                "token_count": 13,
                "match_reason": "expanded"
            },
            {
                "context_id": "ctx-3",
                "source_id": "src-2",
                "package": null,
                "document_stem": "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
                "heading_path": ["Source Attribution Contract"],
                "chunk_id": "chunk-workspace-primary",
                "chunk_ordinal": 2,
                "text": "Workspace primary evidence.",
                "token_count": 17,
                "match_reason": "primary"
            }
        ])
    );
    assert_eq!(
        payload["diagnostics"],
        serde_json::json!({
            "total_token_count": 41,
            "dropped_after_limit": 2,
            "retrieval_limit": 4
        })
    );
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
    assert_eq!(payload["diagnostics"]["total_token_count"], 0);
    assert_eq!(payload["diagnostics"]["dropped_after_limit"], 0);
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
