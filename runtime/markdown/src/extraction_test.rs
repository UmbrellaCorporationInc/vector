#![allow(clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::MarkdownDiscoveryRecord;
use runtime_io::{IoPath, hash_file_content};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[tokio::test]
async fn test_extract_markdown_file_preserves_identity_and_extracts_metadata() {
    let fixture = ExtractionFixture::create("full").await;
    let path = fixture.path("doc/spec-00011-rag-plan-implementation.md");
    fixture
        .write_file(
            "doc/spec-00011-rag-plan-implementation.md",
            r#"---
id: spec-00011-rag-plan-implementation
type: spec
code: "00011"
slug: rag-plan-implementation
tags:
  - rag
  - local
---

# SPEC 00011: RAG Plan Implementation

See [[rfc-00033-markdown-extraction]] and [RFC](rfc-00033-markdown-extraction.md#proposal).
Autolink: <https://example.test/vector>.
Reference: [Research][local-rag].

## Phase 3

```markdown
# Ignored Heading
[[ignored-link]]
```

## Phase 3

[local-rag]: research-00003-local-rag.md
"#,
        )
        .await;
    let record = fixture.record(None, "spec-00011-rag-plan-implementation", &path).await;

    let outcome = extract_markdown_file(&record).await.unwrap();
    let MarkdownExtractionOutcome::Extracted(extraction) = outcome else {
        panic!("expected successful extraction");
    };

    assert_eq!(extraction.package, None);
    assert_eq!(extraction.document_stem, "spec-00011-rag-plan-implementation");
    assert_eq!(extraction.document_type.as_deref(), Some("spec"));
    assert_eq!(extraction.document_code.as_deref(), Some("00011"));
    assert_eq!(extraction.document_slug.as_deref(), Some("rag-plan-implementation"));
    assert_eq!(extraction.document_hash, record.content_hash().to_string());
    assert_eq!(extraction.frontmatter.as_ref().unwrap().format, MarkdownFrontmatterFormat::Yaml);
    assert!(matches!(
        &extraction.frontmatter.as_ref().unwrap().metadata,
        MarkdownMetadataValue::Mapping(metadata)
            if metadata.contains_key("id") && metadata.contains_key("tags")
    ));
    assert!(extraction.body_span.start > 0);
    assert_eq!(extraction.body_span.end, fixture.read_to_string(&path).await.len());

    assert_eq!(
        extraction.headings.iter().map(|heading| heading.text.as_str()).collect::<Vec<_>>(),
        vec!["SPEC 00011: RAG Plan Implementation", "Phase 3", "Phase 3"]
    );
    assert_eq!(
        extraction.headings[1].path,
        vec!["SPEC 00011: RAG Plan Implementation".to_owned(), "Phase 3".to_owned()]
    );
    assert!(extraction.diagnostics.iter().any(|diagnostic| diagnostic.kind == "duplicate_anchor"));

    assert_eq!(
        extraction.links.iter().map(|link| link.kind).collect::<Vec<_>>(),
        vec![
            MarkdownLinkKind::Wikilink,
            MarkdownLinkKind::Inline,
            MarkdownLinkKind::Autolink,
            MarkdownLinkKind::Reference,
        ]
    );
    assert!(extraction.links.iter().all(|link| !link.raw.contains("ignored-link")));
    assert!(extraction.links.iter().any(|link| {
        link.kind == MarkdownLinkKind::Wikilink && link.target == "rfc-00033-markdown-extraction"
    }));
    assert!(extraction.links.iter().any(|link| {
        link.kind == MarkdownLinkKind::Inline
            && link.target == "rfc-00033-markdown-extraction.md"
            && link.heading.as_deref() == Some("proposal")
    }));
    assert!(extraction.links.iter().any(|link| {
        link.kind == MarkdownLinkKind::Reference && link.target == "research-00003-local-rag.md"
    }));

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_extract_markdown_file_preserves_package_identity() {
    let fixture = ExtractionFixture::create("package").await;
    let path =
        fixture.path(".vector-database/packages/shared/doc/rfc-00033-markdown-extraction.md");
    fixture
        .write_file(
            ".vector-database/packages/shared/doc/rfc-00033-markdown-extraction.md",
            "# Title\n",
        )
        .await;
    let record = fixture.record(Some("shared"), "rfc-00033-markdown-extraction", &path).await;

    let outcome = extract_markdown_file(&record).await.unwrap();
    let MarkdownExtractionOutcome::Extracted(extraction) = outcome else {
        panic!("expected successful extraction");
    };

    assert_eq!(extraction.package.as_deref(), Some("shared"));
    assert_eq!(extraction.document_type.as_deref(), Some("rfc"));
    assert_eq!(extraction.document_code.as_deref(), Some("00033"));
    assert_eq!(extraction.document_slug.as_deref(), Some("markdown-extraction"));

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_extract_markdown_file_parses_toml_and_json_frontmatter() {
    let fixture = ExtractionFixture::create("formats").await;
    let toml_path = fixture.path("doc/task-00001-toml.md");
    fixture
        .write_file(
            "doc/task-00001-toml.md",
            "+++\ntitle = \"TOML Task\"\ndraft = false\ntags = [\"rag\", \"markdown\"]\n+++\n# Title\n",
        )
        .await;
    let json_path = fixture.path("doc/task-00002-json.md");
    fixture
        .write_file(
            "doc/task-00002-json.md",
            "---json\n{\"title\":\"JSON Task\",\"tags\":[\"rag\"]}\n---\n# Title\n",
        )
        .await;

    let toml_record = fixture.record(None, "task-00001-toml", &toml_path).await;
    let json_record = fixture.record(None, "task-00002-json", &json_path).await;

    let MarkdownExtractionOutcome::Extracted(toml) =
        extract_markdown_file(&toml_record).await.unwrap()
    else {
        panic!("expected toml extraction");
    };
    let MarkdownExtractionOutcome::Extracted(json) =
        extract_markdown_file(&json_record).await.unwrap()
    else {
        panic!("expected json extraction");
    };

    assert_eq!(toml.frontmatter.unwrap().format, MarkdownFrontmatterFormat::Toml);
    assert_eq!(json.frontmatter.unwrap().format, MarkdownFrontmatterFormat::Json);

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_extract_markdown_file_returns_file_scoped_error_for_malformed_frontmatter() {
    let fixture = ExtractionFixture::create("malformed").await;
    let path = fixture.path("doc/rfc-00020-broken.md");
    fixture.write_file("doc/rfc-00020-broken.md", "---\ntitle: [broken\n---\n# Title\n").await;
    let record = fixture.record(Some("shared-docs"), "rfc-00020-broken", &path).await;

    let outcome = extract_markdown_file(&record).await.unwrap();
    let MarkdownExtractionOutcome::Failed(failure) = outcome else {
        panic!("expected file-scoped extraction failure");
    };

    assert_eq!(failure.package.as_deref(), Some("shared-docs"));
    assert_eq!(failure.document_stem, "rfc-00020-broken");
    assert_eq!(failure.document_hash, record.content_hash().to_string());
    assert_eq!(failure.error.kind, "malformed_frontmatter");
    assert_eq!(failure.error.details.get("format").map(String::as_str), Some("yaml"));
    assert!(failure.error.source_span.end > failure.error.source_span.start);

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_extract_markdown_file_warns_for_unresolved_reference_links() {
    let fixture = ExtractionFixture::create("unresolved-reference").await;
    let path = fixture.path("doc/task-00003-links.md");
    fixture.write_file("doc/task-00003-links.md", "# Title\nSee [Missing][missing-ref].\n").await;
    let record = fixture.record(None, "task-00003-links", &path).await;

    let outcome = extract_markdown_file(&record).await.unwrap();
    let MarkdownExtractionOutcome::Extracted(extraction) = outcome else {
        panic!("expected successful extraction");
    };

    assert!(
        extraction
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == "unresolved_reference_link")
    );

    fixture.cleanup().await;
}

#[test]
fn test_runtime_markdown_extraction_uses_only_registered_serialization_dependencies() {
    let crate_manifest = include_str!("../Cargo.toml");
    let dependency_register =
        include_str!("../../../doc/project/project-0003-rust-dependencies.md");

    assert!(crate_manifest.contains("serde = { workspace = true }"));
    assert!(crate_manifest.contains("serde_yaml = { workspace = true }"));
    assert!(dependency_register.contains("`runtime-markdown`"));
    assert!(dependency_register.contains("Markdown extraction frontmatter metadata"));
}

#[test]
fn test_runtime_markdown_readme_documents_extraction_contract() {
    let readme = include_str!("../README.md");

    assert!(readme.contains("## Extraction Contract"));
    assert!(readme.contains("extract_markdown_file(&MarkdownDiscoveryRecord)"));
    assert!(readme.contains("MarkdownExtractionOutcome::Failed"));
    assert!(readme.contains("malformed_frontmatter"));
    assert!(readme.contains("parser_dependency_spike_deferred"));
}

struct ExtractionFixture {
    root: IoPath,
}

impl ExtractionFixture {
    async fn create(name: &str) -> Self {
        let root = IoPath::new(unique_fixture_path(name));
        fs::create_dir_all(root.as_path()).await.unwrap();
        Self { root }
    }

    fn path(&self, relative_path: &str) -> IoPath {
        self.root.join(relative_path)
    }

    async fn write_file(&self, relative_path: &str, content: &str) {
        let path = self.path(relative_path);
        if let Some(parent) = path.as_path().parent() {
            fs::create_dir_all(parent).await.unwrap();
        }
        fs::write(path.as_path(), content.as_bytes()).await.unwrap();
    }

    async fn read_to_string(&self, path: &IoPath) -> String {
        fs::read_to_string(path.as_path()).await.unwrap()
    }

    async fn record(
        &self,
        package: Option<&str>,
        governed_document_stem: &str,
        path: &IoPath,
    ) -> MarkdownDiscoveryRecord {
        MarkdownDiscoveryRecord::new(
            package.map(ToOwned::to_owned),
            governed_document_stem.to_owned(),
            None,
            hash_file_content(path).await.unwrap(),
            path.clone(),
        )
    }

    async fn cleanup(self) {
        let _ = fs::remove_dir_all(self.root.as_path()).await;
    }
}

fn unique_fixture_path(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("vector-runtime-markdown-extraction-{name}-{nanos}"))
}
