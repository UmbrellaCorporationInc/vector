use super::*;

#[test]
fn test_phase_one_defaults_match_rag_plan() {
    let defaults = RagDefaults::phase_one();

    assert_eq!(defaults.workspace_corpus_root(), "doc");
    assert_eq!(defaults.package_storage_root(), ".vector-database/packages");
    assert_eq!(defaults.package_document_dir(), "doc");
    assert_eq!(defaults.rag_storage_root(), ".vector-database/rag");
    assert_eq!(defaults.lancedb_storage_path(), ".vector-database/rag/lancedb");
    assert_eq!(defaults.embedding_model_identifier(), "BGESmallENV15");
    assert_eq!(defaults.embedding_model_code(), "Xenova/bge-small-en-v1.5");
    assert_eq!(defaults.embedding_dimension(), 384);
    assert_eq!(defaults.chunk_token_target(), 350);
    assert_eq!(defaults.chunk_token_maximum(), 500);
    assert_eq!(defaults.semantic_retrieval_limit(), 20);
    assert_eq!(defaults.lexical_retrieval_limit(), 20);
    assert_eq!(defaults.final_retrieval_limit(), 8);
}

#[test]
fn test_markdown_discovery_request_uses_workspace_and_package_doc_roots() {
    let request = RagDefaults::phase_one().markdown_discovery_request(["package-b", "package-a"]);

    assert_eq!(request.workspace_doc_roots()[0].as_path(), std::path::Path::new("doc"));
    assert_eq!(request.package_doc_roots()[0].package(), "package-a");
    assert_eq!(
        request.package_doc_roots()[0].doc_root().as_path(),
        std::path::Path::new(".vector-database/packages/package-a/doc")
    );
    assert_eq!(request.package_doc_roots()[1].package(), "package-b");
    assert_eq!(
        request.package_doc_roots()[1].doc_root().as_path(),
        std::path::Path::new(".vector-database/packages/package-b/doc")
    );
}
