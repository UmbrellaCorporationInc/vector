#![allow(clippy::panic, clippy::unwrap_used)]

use super::*;
use runtime_io::{IoPath, hash_file_content};
use runtime_markdown::{
    MarkdownDiscoveryRecord, MarkdownExtractionOutcome, MarkdownExtractionRecord,
    MarkdownSourceSpan, extract_markdown_source,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[tokio::test]
async fn test_chunk_document_from_extraction_preserves_phase_three_contract() {
    let fixture = fixture_short_section().await;

    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();

    assert_eq!(document.package, None);
    assert_eq!(document.document_stem, "spec-00011-rag-plan-implementation");
    assert_eq!(document.document_hash, fixture.extraction.document_hash);
    assert_eq!(document.body, "# Title\n\n## Short Section\n\nA concise section.\n");
    assert_eq!(
        document.headings.iter().map(|heading| heading.text.as_str()).collect::<Vec<_>>(),
        vec!["Title", "Short Section"]
    );
}

#[tokio::test]
async fn test_chunk_record_exposes_rfc_required_fields() {
    let fixture = fixture_short_section().await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();

    let chunks = chunk_markdown_document(
        &document,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    )
    .unwrap();

    assert_eq!(chunks.len(), 1);
    let chunk = &chunks[0];
    assert!(chunk.chunk_id.starts_with("workspace/spec-00011-rag-plan-implementation/title/0000/"));
    assert_eq!(chunk.package, None);
    assert_eq!(chunk.document_stem, "spec-00011-rag-plan-implementation");
    assert_eq!(chunk.document_hash, document.document_hash);
    assert!(!chunk.chunk_hash.is_empty());
    assert_eq!(chunk.chunk_ordinal, 0);
    assert_eq!(chunk.heading_path, vec!["Title".to_owned()]);
    assert_eq!(chunk.text, "# Title\n\n## Short Section\n\nA concise section.");
    assert_eq!(chunk.token_count, 8);
    assert_eq!(chunk.previous_chunk_id, None);
    assert_eq!(chunk.next_chunk_id, None);
}

#[test]
fn test_chunking_config_exposes_phase_four_defaults_and_tokenizer_adapter() {
    let config = MarkdownChunkingConfig::phase_four_defaults();

    assert_eq!(config.target_token_count(), 350);
    assert_eq!(config.maximum_token_count(), 500);
    assert_eq!(config.overlap_token_count(), 0);
    assert_eq!(WhitespaceMarkdownTokenCounter.count_tokens("one two\nthree"), 3);
}

#[tokio::test]
async fn test_chunk_output_is_deterministic_for_same_input_and_config() {
    let fixture = fixture_nested_headings().await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();
    let config = MarkdownChunkingConfig::new(350, 500, 0);

    let first =
        chunk_markdown_document(&document, config, &WhitespaceMarkdownTokenCounter).unwrap();
    let second =
        chunk_markdown_document(&document, config, &WhitespaceMarkdownTokenCounter).unwrap();

    assert_eq!(first, second);
}

#[tokio::test]
async fn test_chunk_input_rejects_invalid_extraction_body_span() {
    let mut fixture = fixture_short_section().await;
    fixture.extraction.body_span = MarkdownSourceSpan::new(0, fixture.source.len() + 1);

    let error = MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
        .unwrap_err();

    assert!(matches!(error, MarkdownChunkingError::InvalidBodySpan { .. }));
}

#[tokio::test]
async fn test_phase_a_fixtures_cover_required_markdown_shapes() {
    let fixtures = [
        fixture_short_section().await,
        fixture_nested_headings().await,
        fixture_duplicate_headings().await,
        fixture_oversized_section().await,
        fixture_fenced_code_block().await,
        fixture_list_section().await,
        fixture_table_section().await,
        fixture_package_document().await,
    ];

    for fixture in fixtures {
        let document =
            MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
                .unwrap();

        assert_eq!(document.package.as_deref(), fixture.expected_package);
        assert_eq!(document.document_stem, fixture.expected_document_stem);
        assert!(!document.body.trim().is_empty());
        assert!(
            document.body.contains(fixture.expected_body_marker),
            "fixture {} did not contain expected marker",
            fixture.name
        );
    }
}

struct ChunkFixture {
    name: &'static str,
    source: String,
    extraction: MarkdownExtractionRecord,
    expected_package: Option<&'static str>,
    expected_document_stem: &'static str,
    expected_body_marker: &'static str,
}

async fn fixture_short_section() -> ChunkFixture {
    fixture(
        "short-section",
        None,
        "spec-00011-rag-plan-implementation",
        "---\ntitle: Short\n---\n# Title\n\n## Short Section\n\nA concise section.\n",
        "A concise section.",
    )
    .await
}

async fn fixture_nested_headings() -> ChunkFixture {
    fixture(
        "nested-headings",
        None,
        "rfc-00034-markdown-chunking",
        "# RFC 00034\n\n## Proposal\n\n### Details\n\nNested content.\n",
        "Nested content.",
    )
    .await
}

async fn fixture_duplicate_headings() -> ChunkFixture {
    fixture(
        "duplicate-headings",
        None,
        "task-00063-implement-rfc-00034-markdown-chunking",
        "# Task\n\n## Phase\n\nFirst.\n\n## Phase\n\nSecond.\n",
        "Second.",
    )
    .await
}

async fn fixture_oversized_section() -> ChunkFixture {
    fixture(
        "oversized-section",
        None,
        "spec-00011-rag-plan-implementation",
        format!("# Title\n\n## Long Section\n\n{}", "word ".repeat(520)),
        "word word",
    )
    .await
}

async fn fixture_fenced_code_block() -> ChunkFixture {
    fixture(
        "fenced-code-block",
        None,
        "rfc-00034-markdown-chunking",
        "# Title\n\n```rust\nfn main() {}\n```\n",
        "fn main",
    )
    .await
}

async fn fixture_list_section() -> ChunkFixture {
    fixture(
        "list-section",
        None,
        "task-00063-implement-rfc-00034-markdown-chunking",
        "# Title\n\n- first item\n- second item\n",
        "- second item",
    )
    .await
}

async fn fixture_table_section() -> ChunkFixture {
    fixture(
        "table-section",
        None,
        "rfc-00034-markdown-chunking",
        "# Title\n\n| Field | Meaning |\n| --- | --- |\n| chunk_hash | Stable hash |\n",
        "| chunk_hash |",
    )
    .await
}

async fn fixture_package_document() -> ChunkFixture {
    fixture(
        "package-document",
        Some("shared-docs"),
        "spec-00011-rag-plan-implementation",
        "# Package Spec\n\nPackage synchronized content.\n",
        "Package synchronized content.",
    )
    .await
}

async fn fixture(
    name: &'static str,
    package: Option<&'static str>,
    document_stem: &'static str,
    source: impl Into<String>,
    expected_body_marker: &'static str,
) -> ChunkFixture {
    let source = source.into();
    let path = write_fixture_file(name, &source).await;
    let content_hash = hash_file_content(&path).await.unwrap();
    let record = MarkdownDiscoveryRecord::new(
        package.map(ToOwned::to_owned),
        document_stem.to_owned(),
        None,
        content_hash,
        path.clone(),
    );
    let extraction = match extract_markdown_source(&record, &source) {
        MarkdownExtractionOutcome::Extracted(extraction) => extraction,
        MarkdownExtractionOutcome::Failed(failure) => {
            panic!("fixture {name} extraction failed: {}", failure.error.message);
        }
        _ => panic!("fixture {name} returned an unsupported extraction outcome"),
    };
    let _ = fs::remove_file(path.as_path()).await;

    ChunkFixture {
        name,
        source,
        extraction,
        expected_package: package,
        expected_document_stem: document_stem,
        expected_body_marker,
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
    std::env::temp_dir().join(format!("vector-runtime-rag-chunking-{name}-{nanos}.md"))
}
