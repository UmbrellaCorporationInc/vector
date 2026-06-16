#![allow(clippy::expect_used)]

use super::*;
use serde_json::json;

#[test]
fn retrieval_context_preserves_canonical_data_shape() {
    let source = RetrievalContextSource::new(
        "src-1".to_owned(),
        Some("shared-docs".to_owned()),
        "rfc-00041-phase-9-canonical-result-for-retrieval-operation".to_owned(),
        vec!["Proposal".to_owned(), "Evidence Chunk Contract".to_owned()],
        "shared-docs/rfc-00041-phase-9-canonical-result-for-retrieval-operation > Proposal > Evidence Chunk Contract".to_owned(),
    );
    let chunk = RetrievalContextChunk::new(
        "ctx-1".to_owned(),
        "src-1".to_owned(),
        Some("shared-docs".to_owned()),
        "rfc-00041-phase-9-canonical-result-for-retrieval-operation".to_owned(),
        vec!["Proposal".to_owned(), "Evidence Chunk Contract".to_owned()],
        "chunk-rfc-00041-000".to_owned(),
        0,
        "Canonical retrieval evidence.".to_owned(),
        3,
        RetrievalMatchReason::Primary,
    );
    let context = RetrievalContext::new(
        "canonical context".to_owned(),
        8,
        vec![source],
        vec![chunk],
        RetrievalContextDiagnostics::new(3, 0, 8),
    );

    assert_eq!(context.status, RetrievalContextStatus::HasResults);
    assert_eq!(context.returned, 1);
    assert_eq!(context.sources.len(), 1);
    assert_eq!(context.chunks.len(), 1);
    assert_eq!(context.chunks[0].source_id, context.sources[0].source_id);
    assert_eq!(context.chunks[0].package, context.sources[0].package);
    assert_eq!(context.chunks[0].document_stem, context.sources[0].document_stem);
    assert_eq!(context.chunks[0].heading_path, context.sources[0].heading_path);

    let serialized = serde_json::to_value(&context).expect("context should serialize");
    assert_eq!(
        serialized,
        json!({
            "query": "canonical context",
            "status": "has_results",
            "limit": 8,
            "returned": 1,
            "sources": [{
                "source_id": "src-1",
                "package": "shared-docs",
                "document_stem": "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
                "heading_path": ["Proposal", "Evidence Chunk Contract"],
                "citation_label": "shared-docs/rfc-00041-phase-9-canonical-result-for-retrieval-operation > Proposal > Evidence Chunk Contract"
            }],
            "chunks": [{
                "context_id": "ctx-1",
                "source_id": "src-1",
                "package": "shared-docs",
                "document_stem": "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
                "heading_path": ["Proposal", "Evidence Chunk Contract"],
                "chunk_id": "chunk-rfc-00041-000",
                "chunk_ordinal": 0,
                "text": "Canonical retrieval evidence.",
                "token_count": 3,
                "match_reason": "primary"
            }],
            "diagnostics": {
                "total_token_count": 3,
                "dropped_after_limit": 0,
                "retrieval_limit": 8
            }
        })
    );
}

#[test]
fn empty_retrieval_context_is_successful_without_sources_or_chunks() {
    let context = RetrievalContext::empty("no matches".to_owned(), 8);

    assert_eq!(context.query, "no matches");
    assert_eq!(context.status, RetrievalContextStatus::Empty);
    assert_eq!(context.limit, 8);
    assert_eq!(context.returned, 0);
    assert!(context.sources.is_empty());
    assert!(context.chunks.is_empty());
    assert_eq!(context.diagnostics.total_token_count, 0);
    assert_eq!(context.diagnostics.dropped_after_limit, 0);
    assert_eq!(context.diagnostics.retrieval_limit, 8);

    let serialized = serde_json::to_value(&context).expect("context should serialize");
    assert_eq!(serialized["status"], "empty");
    assert_eq!(serialized["sources"], json!([]));
    assert_eq!(serialized["chunks"], json!([]));
}
