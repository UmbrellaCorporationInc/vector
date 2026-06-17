#![allow(clippy::expect_used)]

use std::{
    future::{Future, ready},
    sync::Mutex,
};

use rmcp::ServerHandler;
use runtime_io::{
    CommandExecutor, CommandExit, CommandHandle, CommandSpec, IoError, MockCommandHandleBuilder,
};

use super::{
    RagSearchParams, RagTools, build_search_command, execute_search_bridge, format_bridge_failure,
};

#[derive(Debug, Clone)]
struct RecordedCommand {
    command: String,
    args: Vec<String>,
    current_dir: Option<std::path::PathBuf>,
}

struct MockExecutor {
    response: Mutex<Option<Result<CommandHandle, IoError>>>,
    recorded: Mutex<Vec<RecordedCommand>>,
}

impl MockExecutor {
    fn new(response: Result<CommandHandle, IoError>) -> Self {
        Self { response: Mutex::new(Some(response)), recorded: Mutex::new(Vec::new()) }
    }

    fn recorded_commands(&self) -> Vec<RecordedCommand> {
        self.recorded.lock().expect("recorded lock").clone()
    }
}

impl CommandExecutor for MockExecutor {
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = Result<CommandHandle, IoError>> + Send {
        self.recorded.lock().expect("recorded lock").push(RecordedCommand {
            command: spec.command().to_owned(),
            args: spec.args().to_vec(),
            current_dir: spec.current_dir().map(std::path::Path::to_path_buf),
        });

        let result = self
            .response
            .lock()
            .expect("response lock")
            .take()
            .unwrap_or_else(|| Err(IoError::Process("mock executor exhausted".into())));
        ready(result)
    }
}

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
    assert!(search.output_schema.is_some(), "search must expose a structured output schema");
    assert!(
        tools.get_tool("rag.search").is_none(),
        "the current rmcp registry exposes the RAG tool by flattened name"
    );
}

#[test]
fn rag_search_builds_vector_database_bridge_command() {
    let temp = tempfile::tempdir().expect("tempdir");
    let params = RagSearchParams {
        query: "hybrid retrieval".to_owned(),
        limit: Some(3),
        package: Some("shared-docs".to_owned()),
        document: Some("rfc-00041-phase-9-canonical-result-for-retrieval-operation".to_owned()),
    };

    let spec = build_search_command(temp.path(), &params).expect("command should build");

    assert_eq!(spec.command(), "vector-database");
    assert_eq!(
        spec.args(),
        [
            "rag",
            "search",
            "hybrid retrieval",
            "--json",
            "--package",
            "shared-docs",
            "--document",
            "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
            "--limit",
            "3",
        ]
    );
    assert_eq!(spec.current_dir(), Some(temp.path()));
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

#[tokio::test]
async fn rag_search_bridge_parses_cli_retrieval_context() {
    let temp = tempfile::tempdir().expect("tempdir");
    let stdout = r#"{
  "query": "governed document context",
  "status": "empty",
  "limit": 8,
  "returned": 0,
  "sources": [],
  "chunks": [],
  "diagnostics": {
    "total_token_count": 0,
    "dropped_after_limit": 0,
    "retrieval_limit": 8
  }
}"#;
    let executor =
        MockExecutor::new(Ok(MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
            .stdout(stdout)
            .build()
            .0));
    let params = RagSearchParams {
        query: "governed document context".to_owned(),
        limit: None,
        package: None,
        document: None,
    };

    let context = execute_search_bridge(&executor, temp.path(), &params)
        .await
        .expect("bridge should parse retrieval context");

    assert_eq!(context.query, "governed document context");
    assert_eq!(context.returned, 0);
    assert_eq!(context.limit, 8);
    let commands = executor.recorded_commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].command, "vector-database");
    assert_eq!(commands[0].args, vec!["rag", "search", "governed document context", "--json"]);
    assert_eq!(commands[0].current_dir.as_deref(), Some(temp.path()));
}

#[tokio::test]
async fn rag_search_bridge_parses_non_empty_query_results() {
    let temp = tempfile::tempdir().expect("tempdir");
    let stdout = r#"{
  "query": "governed document context",
  "status": "has_results",
  "limit": 3,
  "returned": 1,
  "sources": [
    {
      "source_id": "src-1",
      "package": "shared-docs",
      "document_stem": "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
      "heading_path": ["Proposal", "Canonical Output Shape"],
      "citation_label": "shared-docs/rfc-00041-phase-9-canonical-result-for-retrieval-operation > Proposal > Canonical Output Shape"
    }
  ],
  "chunks": [
    {
      "context_id": "ctx-1",
      "source_id": "src-1",
      "package": "shared-docs",
      "document_stem": "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
      "heading_path": ["Proposal", "Canonical Output Shape"],
      "chunk_id": "chunk-1",
      "chunk_ordinal": 0,
      "text": "RetrievalContext separates metadata from evidence chunks.",
      "token_count": 8,
      "match_reason": "primary"
    }
  ],
  "diagnostics": {
    "total_token_count": 8,
    "dropped_after_limit": 0,
    "retrieval_limit": 3
  }
}"#;
    let executor =
        MockExecutor::new(Ok(MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
            .stdout(stdout)
            .build()
            .0));
    let params = RagSearchParams {
        query: "governed document context".to_owned(),
        limit: Some(3),
        package: Some("shared-docs".to_owned()),
        document: Some("rfc-00041-phase-9-canonical-result-for-retrieval-operation".to_owned()),
    };

    let context = execute_search_bridge(&executor, temp.path(), &params)
        .await
        .expect("bridge should parse non-empty retrieval context");

    assert_eq!(context.status, super::RetrievalContextStatus::HasResults);
    assert_eq!(context.returned, 1);
    assert_eq!(context.limit, 3);
    assert_eq!(context.sources.len(), 1);
    assert_eq!(context.chunks.len(), 1);
    assert_eq!(context.sources[0].package.as_deref(), Some("shared-docs"));
    assert_eq!(
        context.sources[0].document_stem,
        "rfc-00041-phase-9-canonical-result-for-retrieval-operation"
    );
    assert_eq!(context.chunks[0].match_reason, super::RetrievalMatchReason::Primary);
    let commands = executor.recorded_commands();
    assert_eq!(
        commands[0].args,
        vec![
            "rag",
            "search",
            "governed document context",
            "--json",
            "--package",
            "shared-docs",
            "--document",
            "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
            "--limit",
            "3",
        ]
    );
}

#[tokio::test]
async fn rag_search_bridge_returns_install_guidance_when_vector_database_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executor = MockExecutor::new(Err(IoError::Process("not found".to_owned())));
    let params = RagSearchParams {
        query: "retrieval".to_owned(),
        limit: None,
        package: None,
        document: None,
    };

    let error = execute_search_bridge(&executor, temp.path(), &params)
        .await
        .expect_err("spawn failure should fail");

    assert_eq!(
        error,
        "vector-database is not available on PATH. Install or expose the CLI bridge and try again."
    );
}

#[tokio::test]
async fn rag_search_bridge_surfaces_non_zero_cli_exit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executor =
        MockExecutor::new(Ok(MockCommandHandleBuilder::new(CommandExit::new(false, Some(2)))
            .stderr("missing RAG store")
            .build()
            .0));
    let params = RagSearchParams {
        query: "retrieval".to_owned(),
        limit: None,
        package: None,
        document: None,
    };

    let error = execute_search_bridge(&executor, temp.path(), &params)
        .await
        .expect_err("non-zero exit should fail");

    assert_eq!(error, "rag.search bridge command failed: missing RAG store");
}

#[tokio::test]
async fn rag_search_bridge_rejects_invalid_json() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executor =
        MockExecutor::new(Ok(MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
            .stdout("not json")
            .build()
            .0));
    let params = RagSearchParams {
        query: "retrieval".to_owned(),
        limit: None,
        package: None,
        document: None,
    };

    let error = execute_search_bridge(&executor, temp.path(), &params)
        .await
        .expect_err("invalid JSON should fail");

    assert!(error.contains("invalid retrieval JSON"), "bridge parse failures must be actionable");
}

#[test]
fn format_bridge_failure_strips_cli_help_text_noise() {
    let output = super::BridgeCommandOutput {
        stdout: Vec::new(),
        stderr: b"error: package_filter must not be empty\nvector-rag: Companion CLI for local RAG runtime execution.\n\nUsage:\n  vector-rag rag search <query>\n".to_vec(),
        exit: CommandExit::new(false, Some(1)),
    };

    let error = format_bridge_failure(&output);

    assert_eq!(
        error,
        "rag.search rejected an invalid package or document filter: package_filter must not be empty"
    );
}

#[test]
fn format_bridge_failure_classifies_missing_rag_store() {
    let output = super::BridgeCommandOutput {
        stdout: Vec::new(),
        stderr: b"error: RAG store is missing at '/tmp/.vector-database/rag/lancedb'; run 'vector-database rag init' or 'vector-database rag update-database' first\n".to_vec(),
        exit: CommandExit::new(false, Some(1)),
    };

    let error = format_bridge_failure(&output);

    assert_eq!(
        error,
        "rag.search requires an initialized local RAG store: RAG store is missing at '/tmp/.vector-database/rag/lancedb'; run 'vector-database rag init' or 'vector-database rag update-database' first"
    );
}

#[test]
fn format_bridge_failure_classifies_incompatible_embedding_metadata() {
    let output = super::BridgeCommandOutput {
        stdout: Vec::new(),
        stderr: b"error: LanceDB table 'chunks' is incompatible with embedding contract: expected model 'BGESmallENV15' and dimension 384, found model 'DifferentModel' and dimension 768\n".to_vec(),
        exit: CommandExit::new(false, Some(1)),
    };

    let error = format_bridge_failure(&output);

    assert_eq!(
        error,
        "rag.search found incompatible RAG embedding metadata: LanceDB table 'chunks' is incompatible with embedding contract: expected model 'BGESmallENV15' and dimension 384, found model 'DifferentModel' and dimension 768"
    );
}

#[test]
fn format_bridge_failure_classifies_corrupt_lancedb_schema() {
    let output = super::BridgeCommandOutput {
        stdout: Vec::new(),
        stderr: b"error: candidate query result is missing 'token_count'\n".to_vec(),
        exit: CommandExit::new(false, Some(1)),
    };

    let error = format_bridge_failure(&output);

    assert_eq!(
        error,
        "rag.search found a corrupt LanceDB table or schema: candidate query result is missing 'token_count'"
    );
}

#[test]
fn format_bridge_failure_classifies_query_embedding_failures() {
    let output = super::BridgeCommandOutput {
        stdout: Vec::new(),
        stderr: b"error: query embedding failed: backend offline\n".to_vec(),
        exit: CommandExit::new(false, Some(1)),
    };

    let error = format_bridge_failure(&output);

    assert_eq!(
        error,
        "rag.search failed to embed the query: query embedding failed: backend offline"
    );
}

#[test]
fn rag_search_mcp_output_remains_compatible_with_cli_json_contract() {
    let cli_json = r#"{
  "query": "governed document context",
  "status": "has_results",
  "limit": 2,
  "returned": 1,
  "sources": [
    {
      "source_id": "src-1",
      "package": "shared-docs",
      "document_stem": "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
      "heading_path": [
        "Proposal",
        "Canonical Output Shape"
      ],
      "citation_label": "shared-docs/rfc-00041-phase-9-canonical-result-for-retrieval-operation > Proposal > Canonical Output Shape"
    }
  ],
  "chunks": [
    {
      "context_id": "ctx-1",
      "source_id": "src-1",
      "package": "shared-docs",
      "document_stem": "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
      "heading_path": [
        "Proposal",
        "Canonical Output Shape"
      ],
      "chunk_id": "chunk-1",
      "chunk_ordinal": 0,
      "text": "RetrievalContext separates metadata from evidence chunks.",
      "token_count": 8,
      "match_reason": "primary"
    }
  ],
  "diagnostics": {
    "total_token_count": 8,
    "dropped_after_limit": 0,
    "retrieval_limit": 2
  }
}"#;

    let parsed: super::RetrievalContext =
        serde_json::from_str(cli_json).expect("MCP bridge must parse canonical CLI retrieval JSON");
    let reserialized = serde_json::to_string_pretty(&parsed)
        .expect("MCP output should preserve the same contract");
    let expected: serde_json::Value =
        serde_json::from_str(cli_json).expect("fixture JSON must remain valid");
    let actual: serde_json::Value =
        serde_json::from_str(&reserialized).expect("reserialized JSON must remain valid");

    assert_eq!(actual, expected);
}
