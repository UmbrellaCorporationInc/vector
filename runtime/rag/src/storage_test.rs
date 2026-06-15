#![allow(clippy::unwrap_used)]

use super::*;
use crate::MarkdownChunkRecord;
use runtime_io::{IoPath, hash_file_content};
use runtime_markdown::{
    MarkdownDiscoveryRecord, MarkdownExtractionOutcome, extract_markdown_source,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[test]
fn test_stable_chunk_id_uses_package_stem_ordinal_and_chunk_hash_only() {
    let chunk_hash = "abc123deadbeef00";

    let workspace_id = stable_chunk_id(None, "spec-00011-rag-plan-implementation", 7, chunk_hash);
    let package_id =
        stable_chunk_id(Some("shared-docs"), "spec-00011-rag-plan-implementation", 7, chunk_hash);

    assert_eq!(workspace_id, "workspace/spec-00011-rag-plan-implementation/0007/abc123deadbeef00");
    assert_eq!(package_id, "shared-docs/spec-00011-rag-plan-implementation/0007/abc123deadbeef00");
}

#[tokio::test]
async fn test_lancedb_chunk_row_preserves_required_phase_six_fields() {
    let extraction = extraction_record().await;
    let row = lancedb_chunk_row(&extraction, &embedded_chunk());

    assert_eq!(row.chunk_id, "workspace/spec-00011-rag-plan-implementation/0000/chunk-hash-0");
    assert_eq!(row.package, None);
    assert_eq!(row.document_stem, "spec-00011-rag-plan-implementation");
    assert_eq!(row.document_hash, "document-hash");
    assert_eq!(row.chunk_hash, "chunk-hash-0");
    assert_eq!(row.chunk_ordinal, 0);
    assert_eq!(row.heading_path, vec!["Title".to_owned(), "Phase 6".to_owned()]);
    assert_eq!(
        row.frontmatter,
        Some(MarkdownMetadataValue::Mapping(BTreeMap::from([
            (
                "id".to_owned(),
                MarkdownMetadataValue::String("spec-00011-rag-plan-implementation".to_owned())
            ),
            ("title".to_owned(), MarkdownMetadataValue::String("RAG Plan".to_owned())),
            (
                "tags".to_owned(),
                MarkdownMetadataValue::Sequence(vec![
                    MarkdownMetadataValue::String("rag".to_owned()),
                    MarkdownMetadataValue::String("lancedb".to_owned()),
                ]),
            ),
            ("draft".to_owned(), MarkdownMetadataValue::Bool(false)),
            ("priority".to_owned(), MarkdownMetadataValue::Number("2".to_owned())),
            (
                "owners".to_owned(),
                MarkdownMetadataValue::Sequence(vec![
                    MarkdownMetadataValue::String("runtime".to_owned()),
                    MarkdownMetadataValue::String("search".to_owned()),
                ]),
            ),
            (
                "nested".to_owned(),
                MarkdownMetadataValue::Mapping(BTreeMap::from([(
                    "ignored".to_owned(),
                    MarkdownMetadataValue::String("value".to_owned()),
                )])),
            ),
        ])))
    );
    assert_eq!(row.text, "## Phase 6\n\nPersist raw chunk text.");
    assert_eq!(row.token_count, 7);
    assert_eq!(row.embedding_model, "BGESmallENV15");
    assert_eq!(row.embedding_dimension, 3);
    assert_eq!(row.vector, vec![1.0, 2.0, 3.0]);
}

#[tokio::test]
async fn test_lancedb_chunk_metadata_materializes_filterable_fields() {
    let extraction = extraction_record().await;
    let row = lancedb_chunk_row(&extraction, &embedded_chunk());

    assert_eq!(row.governed_document.package, None);
    assert_eq!(row.governed_document.document_stem, "spec-00011-rag-plan-implementation");
    assert_eq!(row.governed_document.document_type.as_deref(), Some("spec"));
    assert_eq!(row.governed_document.document_code.as_deref(), Some("00011"));
    assert_eq!(row.governed_document.document_slug.as_deref(), Some("rag-plan-implementation"));
    assert_eq!(row.metadata.package, None);
    assert_eq!(row.metadata.document_stem, "spec-00011-rag-plan-implementation");
    assert_eq!(row.metadata.heading_path, vec!["Title".to_owned(), "Phase 6".to_owned()]);
    assert_eq!(row.metadata.heading_path_text, "Title / Phase 6");
    assert_eq!(row.metadata.tags, vec!["rag".to_owned(), "lancedb".to_owned()]);
    assert_eq!(
        row.metadata.frontmatter_fields,
        BTreeMap::from([
            ("draft".to_owned(), LanceDbFilterValue::Scalar("false".to_owned()),),
            (
                "id".to_owned(),
                LanceDbFilterValue::Scalar("spec-00011-rag-plan-implementation".to_owned()),
            ),
            (
                "owners".to_owned(),
                LanceDbFilterValue::StringList(vec!["runtime".to_owned(), "search".to_owned(),]),
            ),
            ("priority".to_owned(), LanceDbFilterValue::Scalar("2".to_owned()),),
            (
                "tags".to_owned(),
                LanceDbFilterValue::StringList(vec!["rag".to_owned(), "lancedb".to_owned(),]),
            ),
            ("title".to_owned(), LanceDbFilterValue::Scalar("RAG Plan".to_owned()),),
        ])
    );
}

async fn extraction_record() -> runtime_markdown::MarkdownExtractionRecord {
    let source = r"---
id: spec-00011-rag-plan-implementation
title: RAG Plan
tags:
  - rag
  - lancedb
draft: false
priority: 2
owners:
  - runtime
  - search
nested:
  ignored: value
---
## Phase 6

Persist raw chunk text.
";
    let path = write_fixture_file("storage-contract", source).await;
    let content_hash = hash_file_content(&path).await.unwrap();
    let record = MarkdownDiscoveryRecord::new(
        None,
        "spec-00011-rag-plan-implementation".to_owned(),
        None,
        content_hash,
        path.clone(),
    );
    let extraction = match extract_markdown_source(&record, source) {
        MarkdownExtractionOutcome::Extracted(extraction) => extraction,
        MarkdownExtractionOutcome::Failed(failure) => {
            unreachable!("storage fixture extraction failed: {}", failure.error.message);
        }
        _ => unreachable!("storage fixture returned an unsupported extraction outcome"),
    };
    let _ = fs::remove_file(path.as_path()).await;
    extraction
}

fn embedded_chunk() -> EmbeddedMarkdownChunkRecord {
    EmbeddedMarkdownChunkRecord {
        chunk: MarkdownChunkRecord {
            chunk_id: "workspace/spec-00011-rag-plan-implementation/0000/chunk-hash-0".to_owned(),
            package: None,
            document_stem: "spec-00011-rag-plan-implementation".to_owned(),
            document_hash: "document-hash".to_owned(),
            chunk_hash: "chunk-hash-0".to_owned(),
            chunk_ordinal: 0,
            heading_path: vec!["Title".to_owned(), "Phase 6".to_owned()],
            text: "## Phase 6\n\nPersist raw chunk text.".to_owned(),
            token_count: 7,
            previous_chunk_id: None,
            next_chunk_id: None,
        },
        embedding_model: "BGESmallENV15".to_owned(),
        embedding_dimension: 3,
        embedding: vec![1.0, 2.0, 3.0],
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
    std::env::temp_dir().join(format!("vector-runtime-rag-storage-{name}-{nanos}.md"))
}
