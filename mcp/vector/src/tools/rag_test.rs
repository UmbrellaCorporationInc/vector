#![allow(clippy::expect_used)]

use rmcp::ServerHandler;

use super::{RagSearchParams, RagTools};

#[test]
fn rag_tools_exposes_search_metadata() {
    let tools = RagTools::new();
    let search = tools.get_tool("search").expect("RagTools must expose search");
    let description = search.description.as_ref().expect("search must expose a description");

    assert_eq!(search.name, "search");
    assert_eq!(
        description,
        "Query the local RAG index for this workspace and return relevant governed document \
         context."
    );
    assert!(
        tools.get_tool("rag.search").is_none(),
        "the current rmcp registry exposes the RAG tool by flattened name"
    );
}

#[test]
fn rag_search_params_deserializes_query_only() {
    let json = r#"{"query": "what is a retrieval context"}"#;
    let params: RagSearchParams =
        serde_json::from_str(json).expect("must deserialize with query only");
    assert_eq!(params.query, "what is a retrieval context");
    assert!(params.limit.is_none());
    assert!(params.package.is_none());
    assert!(params.document.is_none());
}

#[test]
fn rag_search_params_deserializes_all_fields() {
    let json = r#"{"query": "retrieval", "limit": 5, "package": "my-pkg", "document": "rfc-00041-phase-9"}"#;
    let params: RagSearchParams = serde_json::from_str(json).expect("must deserialize all fields");
    assert_eq!(params.query, "retrieval");
    assert_eq!(params.limit, Some(5));
    assert_eq!(params.package.as_deref(), Some("my-pkg"));
    assert_eq!(params.document.as_deref(), Some("rfc-00041-phase-9"));
}

#[test]
fn rag_search_params_query_is_required() {
    let json = r#"{"limit": 3}"#;
    let result: Result<RagSearchParams, _> = serde_json::from_str(json);
    assert!(result.is_err(), "query is required and must be present");
}
