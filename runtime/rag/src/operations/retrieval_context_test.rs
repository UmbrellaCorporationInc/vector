#![allow(clippy::expect_used)]

use super::*;
use runtime_channel::PluginDispatcher;
use runtime_core::Receiver;
use serde_json::json;

struct SearchResultFixture {
    package: Option<&'static str>,
    document_stem: &'static str,
    heading_path: Vec<&'static str>,
    chunk_id: &'static str,
    chunk_ordinal: usize,
    text: &'static str,
    token_count: usize,
    was_expanded: bool,
}

impl Default for SearchResultFixture {
    fn default() -> Self {
        Self {
            package: None,
            document_stem: "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
            heading_path: vec!["Proposal"],
            chunk_id: "chunk-primary",
            chunk_ordinal: 0,
            text: "Primary evidence.",
            token_count: 1,
            was_expanded: false,
        }
    }
}

fn search_result(fixture: SearchResultFixture) -> HybridSearchResult {
    HybridSearchResult::new(
        fixture.package.map(str::to_owned),
        fixture.document_stem.to_owned(),
        fixture.heading_path.into_iter().map(str::to_owned).collect(),
        fixture.chunk_id.to_owned(),
        fixture.chunk_ordinal,
        fixture.text.to_owned(),
        fixture.token_count,
        Some(1),
        None,
        0.1,
        None,
        None,
        fixture.was_expanded,
        fixture.was_expanded.then(|| "primary-chunk".to_owned()),
    )
}

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

#[tokio::test]
async fn assemble_retrieval_context_converts_primary_and_expanded_chunks() {
    let input = HybridSearchOutput::new(
        "canonical context".to_owned(),
        None,
        None,
        8,
        vec![
            search_result(SearchResultFixture {
                chunk_ordinal: 3,
                token_count: 2,
                ..SearchResultFixture::default()
            }),
            search_result(SearchResultFixture {
                chunk_id: "chunk-expanded",
                chunk_ordinal: 4,
                text: "Expanded evidence.",
                token_count: 2,
                was_expanded: true,
                ..SearchResultFixture::default()
            }),
        ],
    );

    let (_cancel, mut receiver) = PluginDispatcher::new(AssembleRetrievalContextOp::new())
        .input(input)
        .build()
        .expect("context assembler should build");
    let context = receiver
        .recv()
        .await
        .expect("context assembler should run")
        .expect("context assembler should send output");

    assert_eq!(context.status, RetrievalContextStatus::HasResults);
    assert_eq!(context.returned, 2);
    assert_eq!(context.chunks[0].context_id, "ctx-1");
    assert_eq!(context.chunks[0].match_reason, RetrievalMatchReason::Primary);
    assert_eq!(context.chunks[1].context_id, "ctx-2");
    assert_eq!(context.chunks[1].match_reason, RetrievalMatchReason::Expanded);
    assert_eq!(context.chunks[0].text, "Primary evidence.");
    assert_eq!(context.chunks[1].text, "Expanded evidence.");
    assert_eq!(context.diagnostics.total_token_count, 4);
    assert_eq!(context.diagnostics.dropped_after_limit, 0);
}

#[test]
fn assemble_retrieval_context_reuses_sources_and_preserves_package_citations() {
    let context = assemble_retrieval_context_output(HybridSearchOutput::new(
        "package context".to_owned(),
        Some("shared-docs".to_owned()),
        None,
        8,
        vec![
            search_result(SearchResultFixture {
                package: Some("shared-docs"),
                document_stem: "spec-00011-rag-plan-implementation",
                heading_path: vec!["Phase 9"],
                chunk_id: "chunk-1",
                chunk_ordinal: 1,
                text: "First package evidence.",
                token_count: 3,
                ..SearchResultFixture::default()
            }),
            search_result(SearchResultFixture {
                package: Some("shared-docs"),
                document_stem: "spec-00011-rag-plan-implementation",
                heading_path: vec!["Phase 9"],
                chunk_id: "chunk-2",
                chunk_ordinal: 2,
                text: "Second package evidence.",
                token_count: 4,
                was_expanded: true,
            }),
            search_result(SearchResultFixture {
                heading_path: vec!["Proposal", "Source Attribution Contract"],
                chunk_id: "chunk-3",
                chunk_ordinal: 3,
                text: "Workspace evidence.",
                token_count: 5,
                ..SearchResultFixture::default()
            }),
        ],
    ));

    assert_eq!(context.sources.len(), 2);
    assert_eq!(context.sources[0].source_id, "src-1");
    assert_eq!(context.sources[0].package, Some("shared-docs".to_owned()));
    assert_eq!(
        context.sources[0].citation_label,
        "shared-docs/spec-00011-rag-plan-implementation > Phase 9"
    );
    assert_eq!(context.sources[1].source_id, "src-2");
    assert_eq!(
        context.sources[1].citation_label,
        "rfc-00041-phase-9-canonical-result-for-retrieval-operation > Proposal > Source Attribution Contract"
    );
    assert_eq!(context.chunks[0].source_id, "src-1");
    assert_eq!(context.chunks[1].source_id, "src-1");
    assert_eq!(context.chunks[2].source_id, "src-2");
}

#[test]
fn assemble_retrieval_context_enforces_limit_after_expansion_and_reports_diagnostics() {
    let context = assemble_retrieval_context_output(HybridSearchOutput::new(
        "limited context".to_owned(),
        None,
        None,
        2,
        vec![
            search_result(SearchResultFixture {
                document_stem: "doc-00001-one",
                heading_path: vec!["A"],
                chunk_id: "chunk-1",
                chunk_ordinal: 1,
                text: "One.",
                token_count: 7,
                ..SearchResultFixture::default()
            }),
            search_result(SearchResultFixture {
                document_stem: "doc-00001-one",
                heading_path: vec!["A"],
                chunk_id: "chunk-2",
                chunk_ordinal: 2,
                text: "Two.",
                token_count: 11,
                was_expanded: true,
                ..SearchResultFixture::default()
            }),
            search_result(SearchResultFixture {
                document_stem: "doc-00002-two",
                heading_path: vec!["B"],
                chunk_id: "chunk-3",
                chunk_ordinal: 3,
                text: "Three.",
                token_count: 13,
                ..SearchResultFixture::default()
            }),
        ],
    ));

    assert_eq!(context.limit, 2);
    assert_eq!(context.returned, 2);
    assert_eq!(
        context.chunks.iter().map(|chunk| chunk.chunk_id.as_str()).collect::<Vec<_>>(),
        vec!["chunk-1", "chunk-2"]
    );
    assert_eq!(context.diagnostics.total_token_count, 18);
    assert_eq!(context.diagnostics.dropped_after_limit, 1);
    assert_eq!(context.diagnostics.retrieval_limit, 2);
}
