use super::*;

#[test]
fn test_crate_root_reexports_discovery_api() {
    let request = MarkdownDiscoveryRequest::new(["doc"], []);

    assert_eq!(request.hashing_mode(), MarkdownHashingMode::Content);
}
