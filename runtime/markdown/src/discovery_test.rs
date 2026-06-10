use super::*;

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
