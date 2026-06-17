#![allow(clippy::expect_used)]

use rmcp::ServerHandler;

use super::RagTools;

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
