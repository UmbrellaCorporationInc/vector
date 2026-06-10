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
    assert_eq!(document.body_start, 21);
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
    assert!(
        chunk
            .chunk_id
            .starts_with("workspace/spec-00011-rag-plan-implementation/short-section/0000/")
    );
    assert_eq!(chunk.package, None);
    assert_eq!(chunk.document_stem, "spec-00011-rag-plan-implementation");
    assert_eq!(chunk.document_hash, document.document_hash);
    assert!(!chunk.chunk_hash.is_empty());
    assert_eq!(chunk.chunk_ordinal, 0);
    assert_eq!(chunk.heading_path, vec!["Title".to_owned(), "Short Section".to_owned()]);
    assert_eq!(chunk.text, "## Short Section\n\nA concise section.");
    assert_eq!(chunk.token_count, 6);
    assert_eq!(chunk.previous_chunk_id, None);
    assert_eq!(chunk.next_chunk_id, None);
}

#[tokio::test]
async fn test_chunking_emits_heading_sections_with_paths_in_source_order() {
    let fixture = fixture_nested_headings().await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();

    let chunks = chunk_markdown_document(
        &document,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    )
    .unwrap();

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chunk_ordinal, 0);
    assert_eq!(chunks[0].heading_path, vec!["RFC 00034".to_owned(), "Proposal".to_owned()]);
    assert_eq!(
        chunks[0].text,
        "## Proposal\n\nProposal introduction.\n\n### Details\n\nNested content."
    );
    assert_eq!(chunks[1].chunk_ordinal, 1);
    assert_eq!(
        chunks[1].heading_path,
        vec!["RFC 00034".to_owned(), "Proposal".to_owned(), "Details".to_owned()]
    );
    assert_eq!(chunks[1].text, "### Details\n\nNested content.");
}

#[tokio::test]
async fn test_chunking_skips_heading_only_sections() {
    let fixture = fixture(
        "heading-only",
        None,
        "task-00063-implement-rfc-00034-markdown-chunking",
        "# Title\n\n## Empty\n\n## Full\n\nBody.\n",
        "Body.",
    )
    .await;
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
    assert_eq!(chunks[0].heading_path, vec!["Title".to_owned(), "Full".to_owned()]);
    assert_eq!(chunks[0].text, "## Full\n\nBody.");
}

#[tokio::test]
async fn test_chunking_preserves_root_level_content_before_first_heading() {
    let fixture = fixture(
        "root-preface",
        None,
        "spec-00011-rag-plan-implementation",
        "Opening paragraph before headings.\n\n# Title\n\nBody.\n",
        "Opening paragraph",
    )
    .await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();

    let chunks = chunk_markdown_document(
        &document,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    )
    .unwrap();

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chunk_ordinal, 0);
    assert_eq!(chunks[0].heading_path, Vec::<String>::new());
    assert_eq!(chunks[0].text, "Opening paragraph before headings.");
    assert_eq!(chunks[1].chunk_ordinal, 1);
    assert_eq!(chunks[1].heading_path, vec!["Title".to_owned()]);
    assert_eq!(chunks[1].text, "# Title\n\nBody.");
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
async fn test_chunking_populates_neighbor_chunk_ids_within_document_order() {
    let fixture = fixture_nested_headings().await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();

    let chunks = chunk_markdown_document(
        &document,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    )
    .unwrap();

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].previous_chunk_id, None);
    assert_eq!(chunks[0].next_chunk_id, Some(chunks[1].chunk_id.clone()));
    assert_eq!(chunks[1].previous_chunk_id, Some(chunks[0].chunk_id.clone()));
    assert_eq!(chunks[1].next_chunk_id, None);
}

#[tokio::test]
async fn test_chunking_preserves_package_identity_for_synchronized_documents() {
    let fixture = fixture_package_document().await;
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
    assert_eq!(chunks[0].package.as_deref(), Some("shared-docs"));
    assert!(
        chunks[0]
            .chunk_id
            .starts_with("shared-docs/spec-00011-rag-plan-implementation/package-spec/0000/")
    );
}

#[tokio::test]
async fn test_unchanged_chunks_keep_ids_when_unrelated_document_content_changes() {
    let first_fixture = fixture(
        "stable-identity-original",
        None,
        "rfc-00034-markdown-chunking",
        "# Title\n\n## Stable\n\nKeep this body.\n\n## Changed\n\nOriginal content.\n",
        "Keep this body.",
    )
    .await;
    let second_fixture = fixture(
        "stable-identity-edited",
        None,
        "rfc-00034-markdown-chunking",
        "# Title\n\n## Stable\n\nKeep this body.\n\n## Changed\n\nEdited unrelated content.\n",
        "Keep this body.",
    )
    .await;
    let first_document = MarkdownChunkDocument::from_extraction_record(
        &first_fixture.extraction,
        &first_fixture.source,
    )
    .unwrap();
    let second_document = MarkdownChunkDocument::from_extraction_record(
        &second_fixture.extraction,
        &second_fixture.source,
    )
    .unwrap();

    let first_chunks = chunk_markdown_document(
        &first_document,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    )
    .unwrap();
    let second_chunks = chunk_markdown_document(
        &second_document,
        MarkdownChunkingConfig::phase_four_defaults(),
        &WhitespaceMarkdownTokenCounter,
    )
    .unwrap();
    let first_stable = first_chunks
        .iter()
        .find(|chunk| chunk.heading_path.last().is_some_and(|heading| heading == "Stable"))
        .unwrap();
    let second_stable = second_chunks
        .iter()
        .find(|chunk| chunk.heading_path.last().is_some_and(|heading| heading == "Stable"))
        .unwrap();

    assert_ne!(first_stable.document_hash, second_stable.document_hash);
    assert_eq!(first_stable.text, second_stable.text);
    assert_eq!(first_stable.chunk_ordinal, second_stable.chunk_ordinal);
    assert_eq!(first_stable.heading_path, second_stable.heading_path);
    assert_eq!(first_stable.chunk_hash, second_stable.chunk_hash);
    assert_eq!(first_stable.chunk_id, second_stable.chunk_id);
}

#[test]
fn test_chunk_hash_uses_normalized_chunk_text() {
    let heading_path = vec!["Title".to_owned()];
    let lf_hash = stable_chunk_hash(
        None,
        "spec-00011-rag-plan-implementation",
        0,
        &heading_path,
        "# Title\n\nBody.",
    );
    let crlf_hash = stable_chunk_hash(
        None,
        "spec-00011-rag-plan-implementation",
        0,
        &heading_path,
        "# Title\r\n\r\nBody.",
    );

    assert_eq!(lf_hash, crlf_hash);
}

#[tokio::test]
async fn test_sections_at_or_below_maximum_are_emitted_without_overlap() {
    let fixture = fixture_short_section().await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();
    let config = MarkdownChunkingConfig::new(3, 6, 3);

    let chunks =
        chunk_markdown_document(&document, config, &WhitespaceMarkdownTokenCounter).unwrap();

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].text, "## Short Section\n\nA concise section.");
    assert_eq!(chunks[0].token_count, config.maximum_token_count());
}

#[tokio::test]
async fn test_oversized_sections_split_on_token_aware_block_boundaries() {
    let fixture = fixture(
        "oversized-paragraph",
        None,
        "rfc-00034-markdown-chunking",
        format!(
            "# Title\n\n## Long Section\n\n{}\n\n{}\n\n{}",
            numbered_words("alpha", 8),
            numbered_words("beta", 8),
            numbered_words("gamma", 8)
        ),
        "gamma-7",
    )
    .await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();
    let config = MarkdownChunkingConfig::new(10, 12, 0);

    let chunks =
        chunk_markdown_document(&document, config, &WhitespaceMarkdownTokenCounter).unwrap();

    assert!(chunks.len() > 1);
    assert!(
        chunks.iter().all(|chunk| chunk.token_count <= config.maximum_token_count()),
        "all chunks must stay within the configured maximum: {chunks:#?}"
    );
    assert!(chunks.iter().all(|chunk| chunk.text.starts_with("## Long Section\n\n")));
    assert!(chunks.iter().all(|chunk| !chunk.text.contains("alpha-7\n\nbeta-0")));
}

#[tokio::test]
async fn test_oversized_section_overlap_is_local_to_split_section() {
    let fixture = fixture(
        "local-overlap",
        None,
        "task-00063-implement-rfc-00034-markdown-chunking",
        "# Title\n\n## Before\n\nBefore body.\n\n## Long List\n\n- first item words\n- second item words\n- third item words\n- fourth item words\n\n## After\n\nAfter body.\n",
        "fourth item",
    )
    .await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();
    let config = MarkdownChunkingConfig::new(8, 14, 8);

    let chunks =
        chunk_markdown_document(&document, config, &WhitespaceMarkdownTokenCounter).unwrap();
    let long_list_chunks = chunks
        .iter()
        .filter(|chunk| chunk.heading_path.last().is_some_and(|heading| heading == "Long List"))
        .collect::<Vec<_>>();

    assert!(long_list_chunks.len() > 1);
    assert!(long_list_chunks[0].text.contains("- second item words"));
    assert!(long_list_chunks[1].text.contains("- second item words"));
    assert!(!long_list_chunks[0].text.contains("Before body."));
    assert!(!long_list_chunks.last().unwrap().text.contains("After body."));
}

#[tokio::test]
async fn test_oversized_sections_never_split_inside_fenced_code_blocks() {
    let fixture = fixture(
        "fenced-code-split",
        None,
        "rfc-00034-markdown-chunking",
        "# Title\n\n## Example\n\nintro one two three four five six\n\n```rust\nfn main() {}\n```\n\ntail one two three four five six\n",
        "fn main",
    )
    .await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();
    let config = MarkdownChunkingConfig::new(7, 12, 0);

    let chunks =
        chunk_markdown_document(&document, config, &WhitespaceMarkdownTokenCounter).unwrap();
    let code_chunks =
        chunks.iter().filter(|chunk| chunk.text.contains("```rust")).collect::<Vec<_>>();

    assert_eq!(code_chunks.len(), 1);
    assert!(code_chunks[0].text.contains("fn main() {}"));
    assert!(code_chunks[0].text.contains("\n```\n") || code_chunks[0].text.ends_with("\n```"));
    assert!(
        chunks.iter().all(|chunk| chunk.token_count <= config.maximum_token_count()),
        "all chunks must stay within the configured maximum: {chunks:#?}"
    );
}

#[tokio::test]
async fn test_oversized_sections_preserve_valid_list_chunks() {
    let fixture = fixture(
        "list-split",
        None,
        "task-00063-implement-rfc-00034-markdown-chunking",
        "# Title\n\n## Steps\n\n- first item words\n- second item words\n- third item words\n- fourth item words\n",
        "fourth item",
    )
    .await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();
    let config = MarkdownChunkingConfig::new(8, 12, 0);

    let chunks =
        chunk_markdown_document(&document, config, &WhitespaceMarkdownTokenCounter).unwrap();

    assert!(chunks.len() > 1);
    for chunk in &chunks {
        let body_lines = chunk.text.lines().skip_while(|line| !line.trim().is_empty()).skip(1);
        for line in body_lines.filter(|line| !line.trim().is_empty()) {
            assert!(line.starts_with("- "), "list chunk contains a broken item line: {chunk:#?}");
        }
    }
}

#[tokio::test]
async fn test_oversized_sections_preserve_valid_table_chunks_with_repeated_headers() {
    let fixture = fixture(
        "table-split",
        None,
        "rfc-00034-markdown-chunking",
        "# Title\n\n## Table\n\n| Field | Meaning |\n| --- | --- |\n| alpha | first value |\n| beta | second value |\n| gamma | third value |\n",
        "| gamma |",
    )
    .await;
    let document =
        MarkdownChunkDocument::from_extraction_record(&fixture.extraction, &fixture.source)
            .unwrap();
    let config = MarkdownChunkingConfig::new(16, 18, 0);

    let chunks =
        chunk_markdown_document(&document, config, &WhitespaceMarkdownTokenCounter).unwrap();

    assert!(chunks.len() > 1);
    for chunk in chunks.iter().filter(|chunk| chunk.text.contains('|')) {
        assert!(chunk.text.contains("| Field | Meaning |"));
        assert!(chunk.text.contains("| --- | --- |"));
        assert!(
            chunk.token_count <= config.maximum_token_count(),
            "table chunk exceeds maximum: {chunk:#?}"
        );
    }
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
        "# RFC 00034\n\n## Proposal\n\nProposal introduction.\n\n### Details\n\nNested content.\n",
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

fn numbered_words(prefix: &str, count: usize) -> String {
    (0..count).map(|index| format!("{prefix}-{index}")).collect::<Vec<_>>().join(" ")
}
