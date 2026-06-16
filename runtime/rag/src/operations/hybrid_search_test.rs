#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::{
    LanceDbChunkWriteRequest, LanceDbStoreRequest, MarkdownChunkingConfig,
    WhitespaceMarkdownTokenCounter, embed_markdown_extraction, ensure_lancedb_store,
    persist_embedded_markdown_chunks,
};
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_io::{IoPath, hash_file_content};
use runtime_markdown::{MarkdownDiscoveryRecord, extract_markdown_source};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

fn unique_fixture_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let root = std::env::temp_dir().join(format!("vector-rag-search-op-test-{label}-{nanos}"));
    std::fs::create_dir_all(root.join(".vector")).unwrap();
    root
}

async fn write_fixture_file(name: &str, source: &str) -> IoPath {
    let path = IoPath::new(std::env::temp_dir().join(format!(
        "vector-rag-search-{name}-{}.md",
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos())
    )));
    if let Some(parent) = path.as_path().parent() {
        fs::create_dir_all(parent).await.unwrap();
    }
    fs::write(path.as_path(), source.as_bytes()).await.unwrap();
    path
}

#[derive(Debug, Default)]
struct MappingEmbedder {
    query_vectors: HashMap<String, Vec<f32>>,
    chunk_vectors: HashMap<String, Vec<f32>>,
}

impl MappingEmbedder {
    fn with_query_vector(mut self, query: &str, vector: Vec<f32>) -> Self {
        self.query_vectors.insert(query.to_owned(), vector);
        self
    }

    fn with_chunk_vector(mut self, marker: &str, vector: Vec<f32>) -> Self {
        self.chunk_vectors.insert(marker.to_owned(), vector);
        self
    }
}

fn padded_vector(x: f32, y: f32, z: f32) -> Vec<f32> {
    let mut vector = vec![0.0; crate::defaults::EMBEDDING_DIMENSION];
    vector[0] = x;
    vector[1] = y;
    vector[2] = z;
    vector
}

impl Embedder for MappingEmbedder {
    fn model_id(&self) -> &'static str {
        "BGESmallENV15"
    }

    fn dimension(&self) -> usize {
        crate::defaults::EMBEDDING_DIMENSION
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        inputs
            .iter()
            .map(|input| {
                if let Some(vector) = self.query_vectors.get(*input) {
                    return Ok(vector.clone());
                }
                for (marker, vector) in &self.chunk_vectors {
                    if input.contains(marker) {
                        return Ok(vector.clone());
                    }
                }
                Ok(padded_vector(1000.0, 1000.0, 1000.0))
            })
            .collect()
    }
}

async fn insert_fixture_document(
    root_dir: &Path,
    package: Option<&str>,
    document_stem: &str,
    source: &str,
    embedder: &(impl Embedder + Sync),
) {
    ensure_lancedb_store(&LanceDbStoreRequest {
        root_dir: root_dir.to_path_buf(),
        embedding_model: embedder.model_id().to_owned(),
        embedding_dimension: embedder.dimension(),
    })
    .await
    .unwrap();

    let path = write_fixture_file(document_stem, source).await;
    let hash = hash_file_content(&path).await.unwrap();
    let record = MarkdownDiscoveryRecord::new(
        package.map(str::to_owned),
        document_stem.to_owned(),
        None,
        hash,
        path,
    );
    let extraction = extract_markdown_source(&record, source);
    let outcome = embed_markdown_extraction(
        &extraction,
        source,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
        embedder,
    );
    let batch = match outcome {
        crate::MarkdownEmbeddingPipelineOutcome::Embedded(batch) => *batch,
        crate::MarkdownEmbeddingPipelineOutcome::Failed(failure) => {
            unreachable!("fixture embed failed: {failure:?}")
        }
    };
    persist_embedded_markdown_chunks(&LanceDbChunkWriteRequest {
        root_dir: root_dir.to_path_buf(),
        batch,
    })
    .await
    .unwrap();
}

fn assert_f32_eq(actual: f32, expected: f32) {
    let delta = (actual - expected).abs();
    assert!(delta <= 1e-6, "expected {expected}, got {actual}, delta {delta}");
}

fn doc_source(title: &str, body: &str) -> String {
    format!("---\ntitle: {title}\n---\n\n# Title\n\n{body}\n")
}

fn long_paragraph(marker: &str, token_count: usize, needle: Option<&str>) -> String {
    let mut tokens = Vec::with_capacity(token_count.max(2));
    tokens.push(marker.to_owned());
    if let Some(needle) = needle {
        tokens.push(needle.to_owned());
    }
    while tokens.len() < token_count {
        tokens.push(format!("{marker}-{}", tokens.len()));
    }
    tokens.join(" ")
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
    assert!(output.results.is_empty());
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

#[tokio::test]
async fn run_hybrid_search_returns_semantic_only_hit_with_zero_lexical_contribution() {
    let root = unique_fixture_root("semantic-only");
    let embedder = MappingEmbedder::default()
        .with_query_vector("concept query", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("SEMANTIC_TARGET", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("DISTRACTOR", padded_vector(10.0, 10.0, 10.0));

    insert_fixture_document(
        &root,
        None,
        "task-00001-semantic-target",
        &doc_source("Semantic", "SEMANTIC_TARGET conceptual retrieval content."),
        &embedder,
    )
    .await;
    insert_fixture_document(
        &root,
        None,
        "task-00002-distractor",
        &doc_source("Distractor", "DISTRACTOR unrelated text."),
        &embedder,
    )
    .await;

    let output = run_hybrid_search(
        &HybridSearchInput::new(
            root,
            RagDefaults::phase_one(),
            "concept query".to_owned(),
            None,
            None,
            Some(2),
        ),
        &embedder,
    )
    .await
    .unwrap();

    let best = &output.results[0];
    assert_eq!(best.document_stem, "task-00001-semantic-target");
    assert_eq!(best.semantic_rank, Some(1));
    assert_eq!(best.lexical_rank, None);
    assert_f32_eq(best.rrf_score, 1.0 / 61.0);
}

#[tokio::test]
async fn run_hybrid_search_returns_lexical_only_hit_when_semantic_branch_limit_excludes_it() {
    let root = unique_fixture_root("lexical-only");
    let mut embedder =
        MappingEmbedder::default().with_query_vector("needle", padded_vector(0.0, 0.0, 0.0));

    for index in 0..20 {
        let stem = format!("task-{:05}-semantic-{index}", index + 1);
        let marker = format!("SEM{index}");
        let rank_distance = u16::try_from(index).unwrap();
        embedder =
            embedder.with_chunk_vector(&marker, padded_vector(f32::from(rank_distance), 0.0, 0.0));
        insert_fixture_document(
            &root,
            None,
            &stem,
            &doc_source("Semantic", &format!("{marker} vector-only content.")),
            &embedder,
        )
        .await;
    }

    embedder = embedder.with_chunk_vector("LEXICAL_ONLY", padded_vector(999.0, 999.0, 999.0));
    insert_fixture_document(
        &root,
        None,
        "task-99999-lexical-only",
        &doc_source("Lexical", "LEXICAL_ONLY needle exact-match retrieval."),
        &embedder,
    )
    .await;

    let output = run_hybrid_search(
        &HybridSearchInput::new(
            root,
            RagDefaults::phase_one(),
            "needle".to_owned(),
            None,
            None,
            Some(8),
        ),
        &embedder,
    )
    .await
    .unwrap();

    let lexical_only = output
        .results
        .iter()
        .find(|result| result.document_stem == "task-99999-lexical-only")
        .expect("lexical-only document must appear in fused results");
    assert_eq!(lexical_only.semantic_rank, None);
    assert_eq!(lexical_only.lexical_rank, Some(1));
    assert_f32_eq(lexical_only.rrf_score, 1.0 / 61.0);
}

#[tokio::test]
async fn run_hybrid_search_fuses_semantic_and_lexical_ranks_with_fixed_rrf_constant() {
    let root = unique_fixture_root("rrf-fusion");
    let embedder = MappingEmbedder::default()
        .with_query_vector("needle", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("MIXED_ALPHA", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("MIXED_BETA", padded_vector(0.1, 0.0, 0.0))
        .with_chunk_vector("MIXED_GAMMA", padded_vector(0.2, 0.0, 0.0));

    insert_fixture_document(
        &root,
        None,
        "task-00010-mixed-alpha",
        &doc_source("Alpha", "MIXED_ALPHA filler needle"),
        &embedder,
    )
    .await;
    insert_fixture_document(
        &root,
        None,
        "task-00011-mixed-beta",
        &doc_source("Beta", "MIXED_BETA needle needle"),
        &embedder,
    )
    .await;
    insert_fixture_document(
        &root,
        None,
        "task-00012-mixed-gamma",
        &doc_source("Gamma", "MIXED_GAMMA needle"),
        &embedder,
    )
    .await;

    let output = run_hybrid_search(
        &HybridSearchInput::new(
            root,
            RagDefaults::phase_one(),
            "needle".to_owned(),
            None,
            None,
            Some(3),
        ),
        &embedder,
    )
    .await
    .unwrap();

    assert_eq!(output.results.len(), 3);
    let alpha = output
        .results
        .iter()
        .find(|result| result.document_stem == "task-00010-mixed-alpha")
        .unwrap();
    let beta = output
        .results
        .iter()
        .find(|result| result.document_stem == "task-00011-mixed-beta")
        .unwrap();
    assert_eq!(alpha.semantic_rank, Some(1));
    assert_eq!(alpha.lexical_rank, Some(3));
    assert_f32_eq(alpha.rrf_score, (1.0 / 61.0) + (1.0 / 63.0));
    assert_eq!(beta.semantic_rank, Some(2));
    assert_eq!(beta.lexical_rank, Some(1));
    assert_f32_eq(beta.rrf_score, (1.0 / 62.0) + (1.0 / 61.0));
    assert!(beta.rrf_score > alpha.rrf_score, "beta must outrank alpha after RRF fusion");
}

#[tokio::test]
async fn run_hybrid_search_applies_package_and_document_filters_before_fusion() {
    let root = unique_fixture_root("filters");
    let embedder = MappingEmbedder::default()
        .with_query_vector("filter me", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("FILTER_MATCH", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("FILTER_OTHER", padded_vector(0.1, 0.0, 0.0));

    insert_fixture_document(
        &root,
        Some("shared-docs"),
        "task-00020-filter-match",
        &doc_source("Match", "FILTER_MATCH filter me"),
        &embedder,
    )
    .await;
    insert_fixture_document(
        &root,
        Some("other-package"),
        "task-00021-filter-other",
        &doc_source("Other", "FILTER_OTHER filter me"),
        &embedder,
    )
    .await;

    let output = run_hybrid_search(
        &HybridSearchInput::new(
            root,
            RagDefaults::phase_one(),
            "filter me".to_owned(),
            Some("shared-docs".to_owned()),
            Some("task-00020-filter-match".to_owned()),
            Some(5),
        ),
        &embedder,
    )
    .await
    .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].package.as_deref(), Some("shared-docs"));
    assert_eq!(output.results[0].document_stem, "task-00020-filter-match");
}

#[tokio::test]
async fn run_hybrid_search_deduplicates_multiple_chunks_from_one_section_before_limit() {
    let root = unique_fixture_root("section-dedup");
    let embedder = MappingEmbedder::default()
        .with_query_vector("needle", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("ALPHA", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("BETA", padded_vector(0.1, 0.0, 0.0))
        .with_chunk_vector("GAMMA", padded_vector(0.2, 0.0, 0.0))
        .with_chunk_vector("DELTA", padded_vector(0.3, 0.0, 0.0));

    let dominant_section = [
        long_paragraph("ALPHA", 300, Some("needle")),
        long_paragraph("BETA", 300, Some("needle")),
        long_paragraph("GAMMA", 300, Some("needle")),
    ]
    .join("\n\n");
    insert_fixture_document(
        &root,
        None,
        "task-00030-dominant-section",
        &doc_source("Dominant", &dominant_section),
        &embedder,
    )
    .await;
    insert_fixture_document(
        &root,
        None,
        "task-00031-secondary-section",
        &doc_source("Secondary", &long_paragraph("DELTA", 300, Some("needle"))),
        &embedder,
    )
    .await;

    let output = run_hybrid_search(
        &HybridSearchInput::new(
            root,
            RagDefaults::phase_one(),
            "needle".to_owned(),
            None,
            None,
            Some(2),
        ),
        &embedder,
    )
    .await
    .unwrap();

    assert_eq!(output.results.len(), 2);
    assert_eq!(
        output
            .results
            .iter()
            .filter(|result| result.document_stem == "task-00030-dominant-section")
            .count(),
        1
    );
    assert!(
        output.results.iter().any(|result| result.document_stem == "task-00031-secondary-section"),
        "a second section should survive after section-level deduplication"
    );
    assert!(output.results.iter().all(|result| !result.was_expanded));
}

#[tokio::test]
async fn run_hybrid_search_expands_adjacent_chunks_only_within_same_section() {
    let root = unique_fixture_root("section-expansion");
    let embedder = MappingEmbedder::default()
        .with_query_vector("needle", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("ALPHA", padded_vector(0.2, 0.0, 0.0))
        .with_chunk_vector("TARGET", padded_vector(0.0, 0.0, 0.0))
        .with_chunk_vector("OMEGA", padded_vector(0.3, 0.0, 0.0))
        .with_chunk_vector("OUTSIDE", padded_vector(0.4, 0.0, 0.0));

    let expanded_doc = format!(
        "# Title\n\n## Expanded Section\n\n{}\n\n{}\n\n{}\n\n## Other Section\n\n{}",
        long_paragraph("ALPHA", 300, None),
        long_paragraph("TARGET", 300, Some("needle")),
        long_paragraph("OMEGA", 300, None),
        long_paragraph("OUTSIDE", 300, Some("outside"))
    );
    insert_fixture_document(
        &root,
        None,
        "task-00032-expanded-section",
        &format!("---\ntitle: Expanded\n---\n\n{expanded_doc}\n"),
        &embedder,
    )
    .await;

    let output = run_hybrid_search(
        &HybridSearchInput::new(
            root,
            RagDefaults::phase_one(),
            "needle".to_owned(),
            None,
            None,
            Some(4),
        ),
        &embedder,
    )
    .await
    .unwrap();

    let primary = output
        .results
        .iter()
        .find(|result| !result.was_expanded && result.text.contains("TARGET"))
        .expect("primary hit must exist");
    assert!(primary.text.contains("TARGET"));
    assert!(primary.previous_chunk_id.is_some());
    assert!(primary.next_chunk_id.is_some());

    let expanded = output
        .results
        .iter()
        .filter(|result| {
            result.was_expanded
                && result.expanded_from_chunk_id.as_deref() == Some(primary.chunk_id.as_str())
        })
        .collect::<Vec<_>>();
    assert_eq!(expanded.len(), 2);
    assert!(expanded.iter().all(|result| result.heading_path == primary.heading_path));
    assert!(expanded.iter().all(|result| !result.text.contains("OUTSIDE")));
    assert!(expanded.iter().all(|result| result.semantic_rank.is_none()));
    assert!(expanded.iter().all(|result| result.lexical_rank.is_none()));
}
