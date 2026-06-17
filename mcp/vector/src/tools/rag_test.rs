#![allow(clippy::expect_used)]

use std::{
    collections::VecDeque,
    future::{Future, ready},
    sync::{Arc, Mutex},
};

use rmcp::{
    ServerHandler,
    model::{LoggingLevel, LoggingMessageNotificationParam},
};
use runtime_io::{
    CommandExecutor, CommandExit, CommandHandle, CommandSpec, IoError, MockCommandHandleBuilder,
};

use super::{
    RagIndexProgressEvent, RagSearchParams, RagTools, build_index_command, build_search_command,
    execute_index_bridge_with_progress, execute_search_bridge, format_bridge_failure,
    index_lifecycle_log, index_progress_event_log,
};

#[derive(Debug, Clone)]
struct RecordedCommand {
    command: String,
    args: Vec<String>,
    current_dir: Option<std::path::PathBuf>,
}

struct MockExecutor {
    responses: Mutex<VecDeque<Result<CommandHandle, IoError>>>,
    recorded: Mutex<Vec<RecordedCommand>>,
}

impl MockExecutor {
    fn new(response: Result<CommandHandle, IoError>) -> Self {
        Self::from_responses(vec![response])
    }

    fn from_responses(responses: Vec<Result<CommandHandle, IoError>>) -> Self {
        Self { responses: Mutex::new(responses.into()), recorded: Mutex::new(Vec::new()) }
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
            .responses
            .lock()
            .expect("responses lock")
            .pop_front()
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
fn rag_tools_exposes_index_metadata() {
    let tools = RagTools::new();
    let index = tools.get_tool("index").expect("RagTools must expose index");
    let description = index.description.as_ref().expect("index must expose a description");

    assert_eq!(index.name, "index");
    assert_eq!(
        description,
        "Initialize the local RAG store for this workspace and update the workspace RAG index."
    );
    assert!(
        tools.get_tool("rag.index").is_none(),
        "the current rmcp registry exposes the RAG tool by flattened name"
    );
    assert_eq!(index.input_schema["type"], "object", "index input schema must be an object");
    let required = index
        .input_schema
        .get("required")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(required.is_empty(), "index must not require caller-provided inputs");
    let properties = index
        .input_schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert!(
        !properties.contains_key("root_dir"),
        "index must resolve the workspace root from MCP runtime context"
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
fn rag_index_builds_init_bridge_command() {
    let temp = tempfile::tempdir().expect("tempdir");

    let spec = build_index_command(temp.path(), "init").expect("command should build");

    assert_eq!(spec.command(), "vector-database");
    assert_eq!(spec.args(), ["rag", "init"]);
    assert_eq!(spec.current_dir(), Some(temp.path()));
}

#[test]
fn rag_index_builds_update_database_bridge_command() {
    let temp = tempfile::tempdir().expect("tempdir");

    let spec = build_index_command(temp.path(), "update-database").expect("command should build");

    assert_eq!(spec.command(), "vector-database");
    assert_eq!(spec.args(), ["rag", "update-database", "--json"]);
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

#[tokio::test]
async fn rag_index_bridge_runs_init_before_update_database() {
    let temp = tempfile::tempdir().expect("tempdir");
    let init_handle = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout("init ok\n")
        .build()
        .0;
    let update_stdout = r#"{
  "progress": [
    {
      "label": "initializing-store",
      "package": null,
      "document_stem": null,
      "message": "Preparing the local RAG store."
    },
    {
      "label": "indexed",
      "package": null,
      "document_stem": "spec-00011-rag-plan-implementation",
      "message": null
    }
  ],
  "summary": {
    "skipped_count": 0,
    "reindexed_count": 1,
    "deleted_count": 0,
    "failures": []
  }
}"#;
    let update_handle = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout(update_stdout)
        .stderr("indexed 1 documents\n")
        .build()
        .0;
    let executor = MockExecutor::from_responses(vec![Ok(init_handle), Ok(update_handle)]);

    let output = execute_index_bridge_with_progress(&executor, temp.path(), |_| async {})
        .await
        .expect("index bridge should succeed");

    let commands = executor.recorded_commands();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].command, "vector-database");
    assert_eq!(commands[0].args, vec!["rag", "init"]);
    assert_eq!(commands[1].command, "vector-database");
    assert_eq!(commands[1].args, vec!["rag", "update-database", "--json"]);
    assert_eq!(commands[0].current_dir.as_deref(), Some(temp.path()));
    assert_eq!(commands[1].current_dir.as_deref(), Some(temp.path()));

    assert_eq!(output.init.command, "vector-database");
    assert_eq!(output.init.args, vec!["rag", "init"]);
    assert_eq!(output.init.exit_code, Some(0));
    assert_eq!(output.init.stdout, "init ok\n");
    assert_eq!(output.init.stderr, "");

    assert_eq!(output.update_database.command, "vector-database");
    assert_eq!(output.update_database.args, vec!["rag", "update-database", "--json"]);
    assert_eq!(output.update_database.exit_code, Some(0));
    assert_eq!(output.update_database.progress.len(), 2);
    assert_eq!(output.update_database.progress[1].label, "indexed");
    assert_eq!(
        output.update_database.progress[1].document_stem.as_deref(),
        Some("spec-00011-rag-plan-implementation")
    );
    assert_eq!(output.update_database.summary.reindexed_count, 1);
    assert_eq!(output.update_database.stderr, "indexed 1 documents\n");
}

#[tokio::test]
async fn rag_index_bridge_skips_update_database_when_init_fails() {
    let temp = tempfile::tempdir().expect("tempdir");
    let init_handle = MockCommandHandleBuilder::new(CommandExit::new(false, Some(1)))
        .stderr("init exploded")
        .build()
        .0;
    let executor = MockExecutor::from_responses(vec![Ok(init_handle)]);

    let error = execute_index_bridge_with_progress(&executor, temp.path(), |_| async {})
        .await
        .expect_err("init failure must stop the lifecycle");

    let commands = executor.recorded_commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].args, vec!["rag", "init"]);
    assert_eq!(
        error,
        "rag.index init command failed: rag.search bridge command failed: init exploded"
    );
}

#[tokio::test]
async fn rag_index_bridge_rejects_invalid_update_database_json() {
    let temp = tempfile::tempdir().expect("tempdir");
    let init_handle = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout("init ok\n")
        .build()
        .0;
    let update_handle =
        MockCommandHandleBuilder::new(CommandExit::new(true, Some(0))).stdout("not json").build().0;
    let executor = MockExecutor::from_responses(vec![Ok(init_handle), Ok(update_handle)]);

    let error = execute_index_bridge_with_progress(&executor, temp.path(), |_| async {})
        .await
        .expect_err("invalid update JSON must fail");

    assert!(
        error.contains("invalid indexing JSON"),
        "update-database parse failures must be actionable: {error}"
    );
}

#[tokio::test]
async fn rag_index_bridge_returns_install_guidance_when_vector_database_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executor = MockExecutor::new(Err(IoError::Process("not found".to_owned())));

    let error = execute_index_bridge_with_progress(&executor, temp.path(), |_| async {})
        .await
        .expect_err("spawn failure should fail");

    assert_eq!(
        error,
        "vector-database is not available on PATH. Install or expose the CLI bridge and try again."
    );
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

// Phase K: MCP progress notification tests.
//
// rmcp 1.6.0 exposes `Peer<RoleServer>::notify_logging_message` which sends
// `notifications/message` events independently from the tool-call response.
// `execute_index_bridge_with_progress` accepts a generic async notify callback so the
// notification path is testable without constructing a real `Peer<RoleServer>` (whose
// constructor is `pub(crate)` inside rmcp).  The tests below verify the formatted
// notification content and the end-to-end lifecycle order.

#[test]
fn index_lifecycle_log_produces_info_notification_with_tool_and_message() {
    let param = index_lifecycle_log("starting rag init");

    assert_eq!(param.level, LoggingLevel::Info);
    let obj = param.data.as_object().expect("data must be a JSON object");
    assert_eq!(obj["tool"].as_str().expect("tool"), "rag.index");
    assert_eq!(obj["message"].as_str().expect("message"), "starting rag init");
}

#[test]
fn index_lifecycle_log_init_complete_step_is_correctly_labelled() {
    let param = index_lifecycle_log("init complete, starting update-database");

    let obj = param.data.as_object().expect("data must be a JSON object");
    assert!(
        obj["message"].as_str().expect("message").contains("init complete"),
        "lifecycle log for the update-database step must mention init completion"
    );
}

#[test]
fn index_progress_event_log_includes_label_and_document_stem() {
    let event = RagIndexProgressEvent {
        label: "indexed".to_owned(),
        package: None,
        document_stem: Some("spec-00011-rag-plan-implementation".to_owned()),
        message: None,
    };

    let param = index_progress_event_log(&event);

    assert_eq!(param.level, LoggingLevel::Info);
    let obj = param.data.as_object().expect("data must be a JSON object");
    assert_eq!(obj["tool"].as_str().expect("tool"), "rag.index");
    assert_eq!(obj["label"].as_str().expect("label"), "indexed");
    assert_eq!(
        obj["document_stem"].as_str().expect("document_stem"),
        "spec-00011-rag-plan-implementation"
    );
    assert!(obj["package"].is_null(), "package must be null for workspace-local documents");
}

#[test]
fn index_progress_event_log_includes_package_when_present() {
    let event = RagIndexProgressEvent {
        label: "unchanged".to_owned(),
        package: Some("shared-docs".to_owned()),
        document_stem: Some(
            "rfc-00041-phase-9-canonical-result-for-retrieval-operation".to_owned(),
        ),
        message: None,
    };

    let param = index_progress_event_log(&event);

    let obj = param.data.as_object().expect("data must be a JSON object");
    assert_eq!(obj["package"].as_str().expect("package"), "shared-docs");
    assert_eq!(obj["label"].as_str().expect("label"), "unchanged");
}

#[test]
fn index_progress_event_log_includes_message_for_lifecycle_steps() {
    let event = RagIndexProgressEvent {
        label: "failed".to_owned(),
        package: None,
        document_stem: Some("rfc-00042-phase-10-mcp-search-tool".to_owned()),
        message: Some("embedding backend offline".to_owned()),
    };

    let param = index_progress_event_log(&event);

    let obj = param.data.as_object().expect("data must be a JSON object");
    assert_eq!(obj["label"].as_str().expect("label"), "failed");
    assert_eq!(obj["message"].as_str().expect("message"), "embedding backend offline");
}

#[tokio::test]
async fn rag_index_bridge_with_progress_emits_lifecycle_and_event_notifications_in_order() {
    let temp = tempfile::tempdir().expect("tempdir");
    let init_handle = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout("init ok\n")
        .build()
        .0;
    let update_stdout = r#"{
  "progress": [
    {
      "label": "initializing-store",
      "package": null,
      "document_stem": null,
      "message": "Preparing the local RAG store."
    },
    {
      "label": "indexed",
      "package": null,
      "document_stem": "spec-00011-rag-plan-implementation",
      "message": null
    },
    {
      "label": "unchanged",
      "package": "shared-docs",
      "document_stem": "rfc-00041-phase-9-canonical-result-for-retrieval-operation",
      "message": null
    }
  ],
  "summary": {
    "skipped_count": 1,
    "reindexed_count": 1,
    "deleted_count": 0,
    "failures": []
  }
}"#;
    let update_handle = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout(update_stdout)
        .build()
        .0;
    let executor = MockExecutor::from_responses(vec![Ok(init_handle), Ok(update_handle)]);

    let captured: Arc<Mutex<Vec<LoggingMessageNotificationParam>>> =
        Arc::new(Mutex::new(Vec::new()));
    let captured_ref = captured.clone();

    let output = execute_index_bridge_with_progress(&executor, temp.path(), move |param| {
        let sink = captured_ref.clone();
        async move {
            sink.lock().expect("lock").push(param);
        }
    })
    .await
    .expect("bridge with progress should succeed");

    let notifications = captured.lock().expect("lock");
    // Expected order: "starting rag init", "init complete, starting update-database",
    // then one notification per progress event (3 events).
    assert_eq!(notifications.len(), 5, "two lifecycle + three progress event notifications");

    // Lifecycle: starting init
    let data0 = notifications[0].data.as_object().expect("data[0] must be object");
    assert!(
        data0["message"].as_str().expect("message").contains("starting rag init"),
        "first notification must announce init start"
    );

    // Lifecycle: init complete
    let data1 = notifications[1].data.as_object().expect("data[1] must be object");
    assert!(
        data1["message"].as_str().expect("message").contains("init complete"),
        "second notification must announce init completion"
    );

    // Progress events from parsed JSON
    let data2 = notifications[2].data.as_object().expect("data[2] must be object");
    assert_eq!(data2["label"].as_str().expect("label"), "initializing-store");

    let data3 = notifications[3].data.as_object().expect("data[3] must be object");
    assert_eq!(data3["label"].as_str().expect("label"), "indexed");
    assert_eq!(
        data3["document_stem"].as_str().expect("document_stem"),
        "spec-00011-rag-plan-implementation"
    );

    let data4 = notifications[4].data.as_object().expect("data[4] must be object");
    assert_eq!(data4["label"].as_str().expect("label"), "unchanged");
    assert_eq!(data4["package"].as_str().expect("package"), "shared-docs");
    drop(notifications);

    // Final result must be deterministic regardless of notification emission.
    assert_eq!(output.update_database.summary.reindexed_count, 1);
    assert_eq!(output.update_database.summary.skipped_count, 1);
    assert_eq!(output.update_database.progress.len(), 3);
}

#[tokio::test]
async fn rag_index_bridge_with_progress_notification_failure_does_not_abort_index_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let init_handle =
        MockCommandHandleBuilder::new(CommandExit::new(true, Some(0))).stdout("init ok").build().0;
    let update_stdout = r#"{
  "progress": [],
  "summary": {
    "skipped_count": 0,
    "reindexed_count": 0,
    "deleted_count": 0,
    "failures": []
  }
}"#;
    let update_handle = MockCommandHandleBuilder::new(CommandExit::new(true, Some(0)))
        .stdout(update_stdout)
        .build()
        .0;
    let executor = MockExecutor::from_responses(vec![Ok(init_handle), Ok(update_handle)]);

    // Notify callback that drops notifications immediately (simulates a disconnected peer).
    let output = execute_index_bridge_with_progress(&executor, temp.path(), |_param| async {})
        .await
        .expect("index must succeed even when notifications are dropped");

    assert_eq!(output.update_database.summary.reindexed_count, 0);
}
