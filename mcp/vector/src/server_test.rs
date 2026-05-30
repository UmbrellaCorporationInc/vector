#![allow(clippy::expect_used)]

use rmcp::{
    ClientHandler, ServerHandler, ServiceExt,
    model::{CallToolRequestParams, ClientInfo},
};
use serde_json::{Map, json};

use super::VectorServer;
use crate::release::version::workspace_version;

#[derive(Debug, Clone, Default)]
struct DummyClientHandler;

impl ClientHandler for DummyClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

fn json_object(value: &serde_json::Value) -> Map<String, serde_json::Value> {
    value.as_object().expect("tool arguments must be a JSON object").clone()
}

/// Verifies that `VectorServer::new` and `Default` construct a valid server instance.
#[test]
fn vector_server_constructors_produce_valid_instance() {
    let _from_new = VectorServer::new();
    let _from_default = VectorServer::default();
}

/// Verifies that the server declares tool capabilities in its `ServerInfo`.
#[test]
fn vector_server_get_info_declares_tool_capabilities() {
    let server = VectorServer::new();
    let info = server.get_info();
    let tools = info.capabilities.tools;
    assert!(tools.is_some(), "VectorServer must declare tool capabilities in its ServerInfo");
}

/// Verifies that `get_tool` resolves the `create_project` tool by name.
#[test]
fn vector_server_exposes_create_project_tool() {
    let server = VectorServer::new();
    let tool = server.get_tool("create_project");
    assert!(
        tool.is_some(),
        "VectorServer must expose the create_project tool registered by ProjectTools"
    );
}

/// Verifies that `get_tool` resolves the `validate` tool by name.
#[test]
fn vector_server_exposes_validate_tool() {
    let server = VectorServer::new();
    let tool = server.get_tool("validate");
    assert!(
        tool.is_some(),
        "VectorServer must expose the validate tool registered by DocumentTools"
    );
}

/// Verifies that `get_tool` resolves the `language_quality_gate` tool by name.
#[test]
fn vector_server_exposes_language_quality_gate_tool() {
    let server = VectorServer::new();
    let tool = server.get_tool("language_quality_gate");
    assert!(
        tool.is_some(),
        "VectorServer must expose the language_quality_gate tool registered by LanguageTools"
    );
}

/// Verifies that `get_tool` resolves the `get_version` tool by name.
#[test]
fn vector_server_exposes_get_version_tool() {
    let server = VectorServer::new();
    let tool = server.get_tool("get_version");
    assert!(
        tool.is_some(),
        "VectorServer must expose the get_version tool registered by VersionTools"
    );
}

/// Verifies that `get_tool` returns `None` for an unregistered name.
#[test]
fn vector_server_returns_none_for_unknown_tool() {
    let server = VectorServer::new();
    let tool = server.get_tool("nonexistent_tool");
    assert!(tool.is_none(), "VectorServer must return None for an unregistered tool name");
}

/// Verifies that the registered tool metadata exposed through `get_tool`
/// matches the current bootstrap surface of the server.
#[test]
fn vector_server_create_project_tool_metadata_is_stable() {
    let server = VectorServer::new();
    let tool = server
        .get_tool("create_project")
        .expect("VectorServer must expose the create_project tool");
    let description = tool.description.as_ref().expect("registered tool must expose a description");

    assert_eq!(tool.name, "create_project");
    assert!(
        description.contains("governed vector project"),
        "registered tool description must remain aligned with the bootstrap contract"
    );
    assert_eq!(
        tool.input_schema["type"], "object",
        "registered tool input schema must remain an object"
    );
    let required =
        tool.input_schema["required"].as_array().expect("required must be an array of field names");
    assert!(
        required.iter().any(|value| value == "target_dir"),
        "registered schema must require target_dir"
    );
    assert!(
        required.iter().any(|value| value == "project_name"),
        "registered schema must require project_name"
    );
}

/// Verifies that the `validate` tool metadata is stable and correct.
#[test]
fn vector_server_validate_tool_metadata_is_stable() {
    let server = VectorServer::new();
    let tool = server.get_tool("validate").expect("VectorServer must expose the validate tool");
    let description = tool.description.as_ref().expect("registered tool must expose a description");

    assert_eq!(tool.name, "validate");
    assert!(
        description.contains("document-types.yaml"),
        "validate tool description must reference document-types.yaml"
    );
    assert_eq!(tool.input_schema["type"], "object", "validate tool input schema must be an object");
    let required =
        tool.input_schema["required"].as_array().expect("required must be an array of field names");
    assert!(
        required.iter().any(|value| value == "root_dir"),
        "validate schema must require root_dir"
    );
}

/// Verifies that the `language_quality_gate` tool metadata is stable and correct.
#[test]
fn vector_server_language_quality_gate_tool_metadata_is_stable() {
    let server = VectorServer::new();
    let tool = server
        .get_tool("language_quality_gate")
        .expect("VectorServer must expose the language_quality_gate tool");
    let description = tool.description.as_ref().expect("registered tool must expose a description");

    assert_eq!(tool.name, "language_quality_gate");
    assert!(
        description.contains("quality-gate prompts"),
        "language_quality_gate description must remain aligned with the prompt resolution contract"
    );
    assert_eq!(
        tool.input_schema["type"], "object",
        "language_quality_gate input schema must be an object"
    );
    let required =
        tool.input_schema["required"].as_array().expect("required must be an array of field names");
    assert!(
        required.iter().any(|value| value == "root_dir"),
        "language_quality_gate schema must require root_dir"
    );
    assert!(
        required.iter().any(|value| value == "languages"),
        "language_quality_gate schema must require languages"
    );
}

/// Verifies that the `get_version` tool metadata is stable and read-only.
#[test]
fn vector_server_get_version_tool_metadata_is_stable() {
    let server = VectorServer::new();
    let tool =
        server.get_tool("get_version").expect("VectorServer must expose the get_version tool");
    let description = tool.description.as_ref().expect("registered tool must expose a description");

    assert_eq!(tool.name, "get_version");
    assert!(
        description.contains("workspace version"),
        "get_version description must remain aligned with the version introspection contract"
    );
    assert_eq!(tool.input_schema["type"], "object", "get_version input schema must be an object");
    let properties = tool
        .input_schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert!(properties.is_empty(), "get_version must remain a read-only zero-argument tool");
}

/// Verifies that `get_tool` resolves the `find_doc` tool by name.
#[test]
fn vector_server_exposes_find_doc_tool() {
    let server = VectorServer::new();
    let tool = server.get_tool("find_doc");
    assert!(
        tool.is_some(),
        "VectorServer must expose the find_doc tool registered by DocumentTools"
    );
}

/// Verifies that `get_tool` resolves the `create_doc_prompt` tool by name.
#[test]
fn vector_server_exposes_create_doc_prompt_tool() {
    let server = VectorServer::new();
    let tool = server.get_tool("create_doc_prompt");
    assert!(
        tool.is_some(),
        "VectorServer must expose the create_doc_prompt tool registered by DocumentTools"
    );
}

/// Verifies that `get_tool` resolves the `create_doc_type_prompt` tool by name.
#[test]
fn vector_server_exposes_create_doc_type_prompt_tool() {
    let server = VectorServer::new();
    let tool = server.get_tool("create_doc_type_prompt");
    assert!(
        tool.is_some(),
        "VectorServer must expose the create_doc_type_prompt tool registered by DocumentTools"
    );
}

/// Verifies that the `find_doc` tool schema requires `root_dir`, `doc_type`, and `code`,
/// exposes `package` as an optional field, and documents the enriched response contract.
#[test]
fn vector_server_find_doc_tool_schema_is_correct() {
    let server = VectorServer::new();
    let tool = server.get_tool("find_doc").expect("VectorServer must expose find_doc");
    let required =
        tool.input_schema["required"].as_array().expect("required must be an array of field names");
    assert!(required.iter().any(|v| v == "root_dir"), "find_doc schema must require root_dir");
    assert!(required.iter().any(|v| v == "doc_type"), "find_doc schema must require doc_type");
    assert!(required.iter().any(|v| v == "code"), "find_doc schema must require code");
    assert!(
        !required.iter().any(|v| v == "package"),
        "find_doc schema must not require package — it is a reserved optional field"
    );
    let properties = tool
        .input_schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .expect("find_doc schema must expose input properties");
    assert!(
        properties.contains_key("package"),
        "find_doc schema must expose package as an optional property"
    );
    let description =
        tool.description.as_ref().expect("find_doc tool must expose a description");
    assert!(
        description.contains("content"),
        "find_doc tool description must reference content in the enriched response"
    );
}

/// Verifies that the `create_doc_prompt` tool schema requires the mandatory authoring fields.
#[test]
fn vector_server_create_doc_prompt_tool_schema_is_correct() {
    let server = VectorServer::new();
    let tool =
        server.get_tool("create_doc_prompt").expect("VectorServer must expose create_doc_prompt");
    let required =
        tool.input_schema["required"].as_array().expect("required must be an array of field names");
    assert!(
        required.iter().any(|v| v == "root_dir"),
        "create_doc_prompt schema must require root_dir"
    );
    assert!(
        required.iter().any(|v| v == "doc_type"),
        "create_doc_prompt schema must require doc_type"
    );
    assert!(required.iter().any(|v| v == "slug"), "create_doc_prompt schema must require slug");
}

/// Verifies that the `create_doc_type_prompt` tool schema requires the mandatory scaffolding fields.
#[test]
fn vector_server_create_doc_type_prompt_tool_schema_is_correct() {
    let server = VectorServer::new();
    let tool = server
        .get_tool("create_doc_type_prompt")
        .expect("VectorServer must expose create_doc_type_prompt");
    let required =
        tool.input_schema["required"].as_array().expect("required must be an array of field names");
    assert!(
        required.iter().any(|v| v == "root_dir"),
        "create_doc_type_prompt schema must require root_dir"
    );
    assert!(
        required.iter().any(|v| v == "doc_type"),
        "create_doc_type_prompt schema must require doc_type"
    );
    assert!(
        required.iter().any(|v| v == "layout"),
        "create_doc_type_prompt schema must require layout"
    );
    assert!(
        required.iter().any(|v| v == "code-width"),
        "create_doc_type_prompt schema must require code-width"
    );
}

/// Verifies that `ProjectTools` continue to expose `create_project` alongside the document tools.
#[test]
fn vector_server_project_tool_group_remains_intact() {
    let server = VectorServer::new();
    assert!(
        server.get_tool("create_project").is_some(),
        "VectorServer must continue to expose create_project after document tools were added"
    );
    assert!(
        server.get_tool("validate").is_some(),
        "both tool groups must coexist: validate must be available alongside create_project"
    );
    assert!(
        server.get_tool("language_quality_gate").is_some(),
        "language tools must coexist alongside the project and document tool groups"
    );
    assert!(
        server.get_tool("get_version").is_some(),
        "version tools must coexist alongside the project, document, and language tool groups"
    );
}

#[tokio::test]
async fn vector_server_lists_tools_from_both_groups_over_transport() {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_handle = tokio::spawn(async move {
        VectorServer::new()
            .serve(server_transport)
            .await
            .expect("server must start")
            .waiting()
            .await
            .expect("server task must complete cleanly");
    });

    let client = DummyClientHandler.serve(client_transport).await.expect("client must connect");

    let tools = client.peer().list_tools(None).await.expect("list_tools must succeed");
    let tool_names: Vec<_> = tools.tools.iter().map(|tool| tool.name.as_ref()).collect();

    assert!(tool_names.contains(&"create_project"), "project tools must be listed");
    assert!(tool_names.contains(&"validate"), "document tools must be listed");
    assert!(tool_names.contains(&"find_doc"), "document lookup tool must be listed");
    assert!(tool_names.contains(&"language_quality_gate"), "language tools must be listed");
    assert!(tool_names.contains(&"get_version"), "version tools must be listed");

    client.cancel().await.expect("client shutdown must succeed");
    server_handle.abort();
}

#[tokio::test]
async fn vector_server_dispatches_version_tool_calls_over_transport() {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_handle = tokio::spawn(async move {
        VectorServer::new()
            .serve(server_transport)
            .await
            .expect("server must start")
            .waiting()
            .await
            .expect("server task must complete cleanly");
    });

    let client = DummyClientHandler.serve(client_transport).await.expect("client must connect");

    let result = client
        .call_tool(CallToolRequestParams::new("get_version").with_arguments(Map::new()))
        .await
        .expect("get_version tool call must succeed");

    let text = result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|text| text.text.as_str())
        .expect("tool result must contain text content");

    assert_eq!(
        text,
        workspace_version(),
        "get_version must return the canonical workspace version string"
    );

    client.cancel().await.expect("client shutdown must succeed");
    server_handle.abort();
}

#[tokio::test]
async fn vector_server_dispatches_language_tool_calls_over_transport() {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_handle = tokio::spawn(async move {
        VectorServer::new()
            .serve(server_transport)
            .await
            .expect("server must start")
            .waiting()
            .await
            .expect("server task must complete cleanly");
    });

    let client = DummyClientHandler.serve(client_transport).await.expect("client must connect");

    let root = tempfile::tempdir().expect("temp dir");
    let vector_dir = root.path().join(".vector");
    let prompt_dir = root.path().join("doc").join("prompts").join("quality-gate");
    std::fs::create_dir_all(&vector_dir).expect("create .vector dir");
    std::fs::create_dir_all(&prompt_dir).expect("create prompt dir");
    std::fs::write(
        vector_dir.join("language-rules.yaml"),
        "rust:\n  quality-gate: prompts-00005-rust\n",
    )
    .expect("write language config");
    std::fs::write(
        prompt_dir.join("prompts-00005-rust.md"),
        "---\nid: prompts-00005-rust\ntype: prompts\ncode: \"00005\"\nslug: rust\n---\n\n# Rust Quality Gate\nRun xtask quality-lint.\n",
    )
    .expect("write prompt");

    let result = client
        .call_tool(CallToolRequestParams::new("language_quality_gate").with_arguments(json_object(
            &json!({
                "root_dir": root.path().display().to_string(),
                "languages": ["rust"]
            }),
        )))
        .await
        .expect("language_quality_gate tool call must succeed");

    let text = result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|text| text.text.as_str())
        .expect("tool result must contain text content");

    assert!(
        text.contains("# Rust Quality Gate"),
        "language tool call must be routed to language tools and return the prompt body"
    );
    assert!(
        !text.contains("id: prompts-00005-rust"),
        "language tool result must not expose prompt frontmatter"
    );

    client.cancel().await.expect("client shutdown must succeed");
    server_handle.abort();
}

#[tokio::test]
async fn vector_server_dispatches_document_tool_calls_over_transport() {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_handle = tokio::spawn(async move {
        VectorServer::new()
            .serve(server_transport)
            .await
            .expect("server must start")
            .waiting()
            .await
            .expect("server task must complete cleanly");
    });

    let client = DummyClientHandler.serve(client_transport).await.expect("client must connect");

    let root = tempfile::tempdir().expect("temp dir");
    let result =
        client
            .call_tool(CallToolRequestParams::new("validate").with_arguments(json_object(
                &json!({ "root_dir": root.path().display().to_string() }),
            )))
            .await
            .expect("validate tool call must succeed");

    let text = result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|text| text.text.as_str())
        .expect("tool result must contain text content");

    assert!(
        text.contains("document-types.yaml"),
        "validate call must be routed to document tools and report the missing config"
    );

    client.cancel().await.expect("client shutdown must succeed");
    server_handle.abort();
}

#[tokio::test]
async fn vector_server_dispatches_project_tool_calls_over_transport() {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_handle = tokio::spawn(async move {
        VectorServer::new()
            .serve(server_transport)
            .await
            .expect("server must start")
            .waiting()
            .await
            .expect("server task must complete cleanly");
    });

    let client = DummyClientHandler.serve(client_transport).await.expect("client must connect");

    let root = tempfile::tempdir().expect("temp dir");
    let target_dir = root.path().join("project");
    let result = client
        .call_tool(CallToolRequestParams::new("create_project").with_arguments(json_object(
            &json!({
                "target_dir": target_dir.display().to_string(),
                "project_name": "transport-project"
            }),
        )))
        .await
        .expect("create_project tool call must succeed");

    let text = result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|text| text.text.as_str())
        .expect("tool result must contain text content");

    assert!(
        text.contains("transport-project"),
        "project tool result must include the requested project name"
    );
    assert!(
        target_dir.join(".vector").join("document-types.yaml").exists(),
        "project tool call must provision bootstrap assets through the server routing path"
    );

    client.cancel().await.expect("client shutdown must succeed");
    server_handle.abort();
}
