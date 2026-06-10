#![allow(clippy::panic, clippy::unwrap_used)]

use crate::{MarkdownChunkingConfig, WhitespaceMarkdownTokenCounter};
use runtime_io::{IoPath, hash_file_content};
use runtime_markdown::{
    MarkdownDiscoveryRecord, MarkdownExtractionOutcome, MarkdownSourceSpan, extract_markdown_source,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

use super::{
    MarkdownChunkingPipelineError, MarkdownChunkingPipelineOutcome, chunk_markdown_extraction,
};

#[tokio::test]
async fn test_chunk_markdown_extraction_wires_extraction_to_chunking_for_workspace_documents() {
    let fixture = fixture(
        "workspace-pipeline",
        None,
        "spec-00011-rag-plan-implementation",
        "# Title\n\n## Chunking Boundary\n\nBody for embedding.\n",
    )
    .await;

    let outcome = chunk_markdown_extraction(
        &fixture.extraction,
        &fixture.source,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    );

    let MarkdownChunkingPipelineOutcome::Chunked(batch) = outcome else {
        panic!("expected extracted workspace document to chunk");
    };
    assert_eq!(batch.package, None);
    assert_eq!(batch.document_stem, "spec-00011-rag-plan-implementation");
    assert_eq!(batch.document_hash, fixture.document_hash);
    assert_eq!(batch.chunks.len(), 1);
    assert_eq!(batch.chunks[0].heading_path, vec!["Title", "Chunking Boundary"]);
    assert_eq!(batch.chunks[0].text, "## Chunking Boundary\n\nBody for embedding.");
}

#[tokio::test]
async fn test_chunk_markdown_extraction_uses_same_semantics_for_package_documents() {
    let fixture = fixture(
        "package-pipeline",
        Some("shared-docs"),
        "rfc-00034-markdown-chunking",
        "# Package RFC\n\nPackage content.\n",
    )
    .await;

    let outcome = chunk_markdown_extraction(
        &fixture.extraction,
        &fixture.source,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    );

    let MarkdownChunkingPipelineOutcome::Chunked(batch) = outcome else {
        panic!("expected extracted package document to chunk");
    };
    assert_eq!(batch.package.as_deref(), Some("shared-docs"));
    assert_eq!(batch.document_stem, "rfc-00034-markdown-chunking");
    assert_eq!(batch.chunks.len(), 1);
    assert_eq!(batch.chunks[0].package.as_deref(), Some("shared-docs"));
    assert_eq!(batch.chunks[0].heading_path, vec!["Package RFC"]);
    assert!(batch.chunks[0].chunk_id.starts_with("shared-docs/rfc-00034-markdown-chunking/"));
}

#[tokio::test]
async fn test_chunk_markdown_extraction_returns_malformed_extraction_errors() {
    let fixture = fixture(
        "malformed-frontmatter",
        None,
        "task-00063-implement-rfc-00034-markdown-chunking",
        "---\ntitle: [broken\n---\n# Title\n",
    )
    .await;

    let outcome = chunk_markdown_extraction(
        &fixture.extraction,
        &fixture.source,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    );

    let MarkdownChunkingPipelineOutcome::Failed(failure) = outcome else {
        panic!("expected malformed extraction input to fail");
    };
    assert_eq!(failure.document_stem, "task-00063-implement-rfc-00034-markdown-chunking");
    assert!(matches!(
        failure.error,
        MarkdownChunkingPipelineError::MalformedExtractionInput { ref kind, .. }
            if kind == "malformed_frontmatter"
    ));
}

#[tokio::test]
async fn test_chunk_markdown_extraction_returns_unsupported_structure_errors() {
    let fixture = fixture(
        "unsupported-structure",
        Some("shared-docs"),
        "rfc-00034-markdown-chunking",
        "---\ntitle: [broken\n---\n# Title\n",
    )
    .await;
    let MarkdownExtractionOutcome::Failed(mut failure) = fixture.extraction else {
        panic!("expected fixture extraction to fail");
    };
    failure.error.kind = "unsupported_markdown_structure".to_owned();
    failure.error.message =
        "HTML blocks are not supported by the current Markdown extraction boundary.".to_owned();
    failure.error.source_span = MarkdownSourceSpan::new(10, 42);
    failure.error.details = BTreeMap::from([("structure".to_owned(), "html_block".to_owned())]);
    let extraction = MarkdownExtractionOutcome::Failed(failure);

    let outcome = chunk_markdown_extraction(
        &extraction,
        "",
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    );

    let MarkdownChunkingPipelineOutcome::Failed(failure) = outcome else {
        panic!("expected unsupported Markdown structure to fail");
    };
    assert_eq!(failure.package.as_deref(), Some("shared-docs"));
    assert!(matches!(
        failure.error,
        MarkdownChunkingPipelineError::UnsupportedMarkdownStructure { ref kind, .. }
            if kind == "unsupported_markdown_structure"
    ));
}

#[tokio::test]
async fn test_chunk_markdown_extraction_returns_unsplittable_oversized_block_errors() {
    let fixture = fixture(
        "oversized-code-block",
        None,
        "rfc-00034-markdown-chunking",
        "# Title\n\n## Example\n\n```rust\none two three four five six seven eight\n```\n",
    )
    .await;

    let outcome = chunk_markdown_extraction(
        &fixture.extraction,
        &fixture.source,
        MarkdownChunkingConfig::new(4, 6, 0),
        &WhitespaceMarkdownTokenCounter,
    );

    let MarkdownChunkingPipelineOutcome::Failed(failure) = outcome else {
        panic!("expected oversized code block to fail");
    };
    assert_eq!(failure.document_stem, "rfc-00034-markdown-chunking");
    assert!(matches!(
        failure.error,
        MarkdownChunkingPipelineError::UnsplittableOversizedBlock {
            token_count,
            maximum_token_count: 6,
            ..
        } if token_count > 6
    ));
}

struct PipelineFixture {
    source: String,
    document_hash: String,
    extraction: MarkdownExtractionOutcome,
}

async fn fixture(
    name: &str,
    package: Option<&str>,
    document_stem: &str,
    source: &str,
) -> PipelineFixture {
    let path = write_fixture_file(name, source).await;
    let content_hash = hash_file_content(&path).await.unwrap();
    let record = MarkdownDiscoveryRecord::new(
        package.map(ToOwned::to_owned),
        document_stem.to_owned(),
        None,
        content_hash.clone(),
        path.clone(),
    );
    let extraction = extract_markdown_source(&record, source);
    let _ = fs::remove_file(path.as_path()).await;

    PipelineFixture {
        source: source.to_owned(),
        document_hash: content_hash.to_string(),
        extraction,
    }
}

async fn write_fixture_file(name: &str, source: &str) -> IoPath {
    let path = IoPath::new(unique_fixture_path(name));
    if let Some(parent) = path.as_path().parent() {
        fs::create_dir_all(parent).await.unwrap();
    }
    fs::write(path.as_path(), source.as_bytes()).await.unwrap();
    path
}

fn unique_fixture_path(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("vector-runtime-rag-pipeline-{name}-{nanos}.md"))
}
