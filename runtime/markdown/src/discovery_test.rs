#![allow(clippy::unwrap_used)]

use super::*;
use runtime_io::hash_file_content;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[test]
fn test_markdown_discovery_request_keeps_explicit_roots() {
    let request = MarkdownDiscoveryRequest::new(
        ["doc"],
        [PackageMarkdownRoot::new("package-a", ".vector-database/packages/package-a/doc")],
    );

    assert_eq!(request.workspace_doc_roots(), &[IoPath::new("doc")]);
    assert_eq!(request.package_doc_roots()[0].package(), "package-a");
    assert_eq!(
        request.package_doc_roots()[0].doc_root(),
        &IoPath::new(".vector-database/packages/package-a/doc")
    );
}

#[test]
fn test_markdown_discovery_request_defaults_to_content_hashing() {
    let request = MarkdownDiscoveryRequest::new(["doc"], []);

    assert_eq!(request.hashing_mode(), MarkdownHashingMode::Content);
    assert!(request.traversal_options().ignored_paths().is_empty());
}

#[test]
fn test_markdown_discovery_request_accepts_runtime_io_traversal_options() {
    let traversal_options =
        DirectoryTraversalOptions::new().with_ignored_path(IoPath::new("doc/ignored"));

    let request =
        MarkdownDiscoveryRequest::new(["doc"], []).with_traversal_options(traversal_options);

    assert_eq!(request.traversal_options().ignored_paths(), &[IoPath::new("doc/ignored")]);
}

#[tokio::test]
async fn test_discover_markdown_files_discovers_workspace_markdown() {
    let fixture = MarkdownFixture::create("workspace").await;
    fixture.write_file("doc/task-00001-first-task.md", "alpha").await;
    fixture.write_file("doc/rfc-00032-markdown-discovery.markdown", "bravo").await;

    let request = MarkdownDiscoveryRequest::new([fixture.path("doc")], []);
    let report = discover_markdown_files(&request).await.unwrap();

    assert!(report.issues().is_empty());
    assert_eq!(
        document_stems(report.records()),
        vec!["rfc-00032-markdown-discovery", "task-00001-first-task",]
    );
    assert_eq!(report.records()[0].package(), None);
    assert!(report.records()[0].modified_time().is_some());
    assert_eq!(
        report.records()[0].internal_read_path().as_path(),
        fixture.path("doc/rfc-00032-markdown-discovery.markdown").as_path()
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_discover_markdown_files_discovers_package_markdown() {
    let fixture = MarkdownFixture::create("package").await;
    fixture
        .write_file(".vector-database/packages/package-a/doc/spec-00011-rag-plan.md", "alpha")
        .await;

    let request = MarkdownDiscoveryRequest::new(
        Vec::<IoPath>::new(),
        [PackageMarkdownRoot::new(
            "package-a",
            fixture.path(".vector-database/packages/package-a/doc"),
        )],
    );
    let report = discover_markdown_files(&request).await.unwrap();

    assert!(report.issues().is_empty());
    assert_eq!(report.records().len(), 1);
    assert_eq!(report.records()[0].package(), Some("package-a"));
    assert_eq!(report.records()[0].governed_document_stem(), "spec-00011-rag-plan");

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_discover_markdown_files_filters_non_markdown_extensions() {
    let fixture = MarkdownFixture::create("extensions").await;
    fixture.write_file("doc/task-00001-first-task.md", "alpha").await;
    fixture.write_file("doc/task-00002-second-task.markdown", "bravo").await;
    fixture.write_file("doc/task-00003-third-task.txt", "charlie").await;

    let request = MarkdownDiscoveryRequest::new([fixture.path("doc")], []);
    let report = discover_markdown_files(&request).await.unwrap();

    assert!(report.issues().is_empty());
    assert_eq!(
        document_stems(report.records()),
        vec!["task-00001-first-task", "task-00002-second-task",]
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_discover_markdown_files_reports_invalid_governed_stems() {
    let fixture = MarkdownFixture::create("invalid-stem").await;
    fixture.write_file("doc/not-governed.md", "alpha").await;
    fixture.write_file("doc/task-00001-valid-task.md", "bravo").await;

    let request = MarkdownDiscoveryRequest::new([fixture.path("doc")], []);
    let report = discover_markdown_files(&request).await.unwrap();

    assert_eq!(document_stems(report.records()), vec!["task-00001-valid-task"]);
    assert_eq!(report.issues().len(), 1);
    assert!(matches!(
        &report.issues()[0],
        MarkdownDiscoveryIssue::InvalidGovernedDocumentStem { package: None, stem, .. }
            if stem == "not-governed"
    ));

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_discover_markdown_files_respects_ignored_paths() {
    let fixture = MarkdownFixture::create("ignored").await;
    fixture.write_file("doc/keep/task-00001-keep.md", "alpha").await;
    fixture.write_file("doc/skip/task-00002-skip.md", "bravo").await;
    let options = DirectoryTraversalOptions::new().with_ignored_path(fixture.path("doc/skip"));

    let request =
        MarkdownDiscoveryRequest::new([fixture.path("doc")], []).with_traversal_options(options);
    let report = discover_markdown_files(&request).await.unwrap();

    assert!(report.issues().is_empty());
    assert_eq!(document_stems(report.records()), vec!["task-00001-keep"]);

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_discover_markdown_files_reports_missing_package_doc_as_package_structure_issue() {
    let fixture = MarkdownFixture::create("missing-package-doc").await;
    fixture.write_file("doc/task-00001-workspace.md", "alpha").await;

    let request = MarkdownDiscoveryRequest::new(
        [fixture.path("doc")],
        [PackageMarkdownRoot::new(
            "package-a",
            fixture.path(".vector-database/packages/package-a/doc"),
        )],
    );
    let report = discover_markdown_files(&request).await.unwrap();

    assert_eq!(document_stems(report.records()), vec!["task-00001-workspace"]);
    assert_eq!(report.issues().len(), 1);
    assert!(matches!(
        &report.issues()[0],
        MarkdownDiscoveryIssue::PackageStructure { package, .. } if package == "package-a"
    ));

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_discover_markdown_files_content_hash_is_stable_across_modified_time_changes() {
    let fixture = MarkdownFixture::create("hash-stability").await;
    let path = fixture.path("doc/task-00001-stable-hash.md");
    fixture.write_file("doc/task-00001-stable-hash.md", "alpha").await;

    let request = MarkdownDiscoveryRequest::new([fixture.path("doc")], []);
    let first = discover_markdown_files(&request).await.unwrap();
    let direct_hash = hash_file_content(&path).await.unwrap();
    assert_eq!(first.records()[0].content_hash(), &direct_hash);

    fixture.touch("doc/task-00001-stable-hash.md").await;
    let second = discover_markdown_files(&request).await.unwrap();
    assert_eq!(first.records()[0].content_hash(), second.records()[0].content_hash());

    fixture.write_file("doc/task-00001-stable-hash.md", "bravo").await;
    let third = discover_markdown_files(&request).await.unwrap();
    assert_ne!(second.records()[0].content_hash(), third.records()[0].content_hash());

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_discover_markdown_files_fails_for_missing_workspace_doc() {
    let fixture = MarkdownFixture::create("missing-workspace-doc").await;
    let request = MarkdownDiscoveryRequest::new([fixture.path("doc")], []);

    let error = discover_markdown_files(&request).await.unwrap_err();

    assert!(matches!(error, MarkdownDiscoveryFailure::WorkspaceDiscovery { .. }));

    fixture.cleanup().await;
}

fn document_stems(records: &[MarkdownDiscoveryRecord]) -> Vec<&str> {
    records.iter().map(MarkdownDiscoveryRecord::governed_document_stem).collect()
}

struct MarkdownFixture {
    root: IoPath,
}

impl MarkdownFixture {
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

    async fn touch(&self, relative_path: &str) {
        let path = self.path(relative_path);
        let content = fs::read(path.as_path()).await.unwrap();
        fs::write(path.as_path(), content).await.unwrap();
    }

    async fn cleanup(self) {
        let _ = fs::remove_dir_all(self.root.as_path()).await;
    }
}

fn unique_fixture_path(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("vector-runtime-markdown-{name}-{nanos}"))
}

#[test]
fn test_runtime_markdown_uses_only_approved_discovery_dependencies() {
    let crate_manifest = include_str!("../Cargo.toml");
    let dependency_register =
        include_str!("../../../doc/project/project-0003-rust-dependencies.md");

    assert!(crate_manifest.contains("runtime-io = { workspace = true }"));
    assert!(
        crate_manifest.contains("tokio = { workspace = true, features = [\"rt\", \"macros\"] }")
    );
    assert!(
        dependency_register.contains("runtime-markdown` must use `runtime-io` primitives")
            || dependency_register.contains("Additional test-only Tokio usage is approved"),
        "runtime-markdown discovery should remain on approved runtime-io and test runtime boundaries"
    );
}
