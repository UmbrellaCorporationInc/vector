//! MCP tool group for RAG capability domain.

use std::{future::Future, path::PathBuf};

use rmcp::{
    RoleServer, ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::{LoggingLevel, LoggingMessageNotificationParam},
    schemars, tool, tool_handler, tool_router,
};
use runtime_io::{CommandBuilder, CommandExecutor, CommandExit, CommandHandle, CommandSpec};
use serde::{Deserialize, Serialize};

const VECTOR_DATABASE_BINARY: &str = "vector-database";
const INSTALL_GUIDANCE: &str = "vector-database is not available on PATH. \
Install or expose the CLI bridge and try again.";
const VECTOR_RAG_HELP_BANNER: &str = "vector-rag: Companion CLI for local RAG runtime execution.";

/// MCP-facing parameters for the `search` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; serde deserialization requires public fields)
#[non_exhaustive]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RagSearchParams {
    /// Free-text query sent to the local RAG retrieval operation.
    pub query: String,
    /// Optional final retrieval limit override applied before context assembly.
    pub limit: Option<usize>,
    /// Optional package filter applied before ranking and fusion.
    pub package: Option<String>,
    /// Optional governed document stem filter applied before ranking and fusion.
    pub document: Option<String>,
}

/// MCP-facing parameters for the `index` tool.
///
/// # DTO(MCP protocol input mapped at the adapter boundary; the index lifecycle is rooted in runtime context, so no caller-provided fields are accepted)
#[non_exhaustive]
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct RagIndexParams {}

/// Structured MCP output for the `index` bridge.
///
/// # DTO(machine-readable command results emitted after the init and update-database lifecycle commands succeed)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RagIndexOutput {
    /// Result of `vector-database rag init`.
    pub init: RagIndexCommandOutcome,
    /// Structured result of `vector-database rag update-database --json`.
    pub update_database: RagIndexUpdateDatabaseOutcome,
}

/// One command outcome captured by the `index` bridge.
///
/// # DTO(machine-readable command execution summary for one bridge subprocess)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RagIndexCommandOutcome {
    /// Executed bridge command name.
    pub command: String,
    /// Executed bridge command arguments.
    pub args: Vec<String>,
    /// Process exit code, when available.
    pub exit_code: Option<i32>,
    /// Captured stdout text.
    pub stdout: String,
    /// Captured stderr text.
    pub stderr: String,
}

/// Structured update-database result captured by the `index` bridge.
///
/// # DTO(machine-readable indexing contract consumed by MCP without parsing human CLI text)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RagIndexUpdateDatabaseOutcome {
    /// Executed bridge command name.
    pub command: String,
    /// Executed bridge command arguments.
    pub args: Vec<String>,
    /// Process exit code, when available.
    pub exit_code: Option<i32>,
    /// Incremental index progress captured from the final JSON contract.
    pub progress: Vec<RagIndexProgressEvent>,
    /// Final indexing summary.
    pub summary: RagIndexSummary,
    /// Captured stderr text.
    pub stderr: String,
}

/// One stable indexing progress event returned to MCP consumers.
///
/// # DTO(MCP tool output is serialized through serde, so these fields remain public at the adapter boundary)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RagIndexProgressEvent {
    /// Stable progress label such as `indexed`, `unchanged`, or `failed`.
    pub label: String,
    /// Package identity when the progress event applies to a synchronized package document.
    pub package: Option<String>,
    /// Governed document stem when the progress event applies to one document.
    pub document_stem: Option<String>,
    /// Human-readable detail for lifecycle steps or actionable failures.
    pub message: Option<String>,
}

/// Final indexing summary returned to MCP consumers.
///
/// # DTO(MCP tool output is serialized through serde, so these fields remain public at the adapter boundary)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RagIndexSummary {
    /// Number of documents skipped because the indexed content was unchanged.
    pub skipped_count: usize,
    /// Number of documents successfully indexed or re-indexed.
    pub reindexed_count: usize,
    /// Number of stale indexed documents deleted from the store.
    pub deleted_count: usize,
    /// Per-document failures recorded during the indexing run.
    pub failures: Vec<RagIndexFailureRecord>,
}

/// One per-document indexing failure returned in the final summary.
///
/// # DTO(MCP tool output is serialized through serde, so these fields remain public at the adapter boundary)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RagIndexFailureRecord {
    /// Package identity, or `None` for a workspace-local document.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Actionable document-level failure message.
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RagIndexCliJsonOutput {
    progress: Vec<RagIndexProgressEvent>,
    summary: RagIndexSummary,
}

/// Canonical retrieval context returned by the MCP `search` bridge.
///
/// # DTO(machine-readable retrieval context parsed from the `vector-database` CLI bridge and emitted as structured MCP output)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RetrievalContext {
    /// Query text used to retrieve the evidence.
    pub query: String,
    /// Successful retrieval status.
    pub status: RetrievalContextStatus,
    /// Final retrieval limit applied to the context result.
    pub limit: usize,
    /// Number of evidence chunks returned.
    pub returned: usize,
    /// Normalized source entries referenced by returned chunks.
    pub sources: Vec<RetrievalContextSource>,
    /// Evidence chunks returned for the query.
    pub chunks: Vec<RetrievalContextChunk>,
    /// Runtime diagnostics for the context result.
    pub diagnostics: RetrievalContextDiagnostics,
}

/// Successful retrieval status for the canonical context result.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalContextStatus {
    /// At least one evidence chunk was returned.
    HasResults,
    /// Retrieval completed successfully with no evidence chunks.
    Empty,
}

/// Normalized source attribution for one or more evidence chunks.
///
/// # DTO(machine-readable retrieval source parsed from the `vector-database` CLI bridge and emitted as structured MCP output)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RetrievalContextSource {
    /// Response-local source identifier referenced by chunks.
    pub source_id: String,
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Heading path for the source section.
    pub heading_path: Vec<String>,
    /// Deterministic human-readable citation label.
    pub citation_label: String,
}

/// One retrieved evidence chunk in the canonical context result.
///
/// # DTO(machine-readable retrieval chunk parsed from the `vector-database` CLI bridge and emitted as structured MCP output)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RetrievalContextChunk {
    /// Response-local context identifier, such as `ctx-1`.
    pub context_id: String,
    /// Response-local source identifier matching an entry in `sources`.
    pub source_id: String,
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Heading path for the chunk section.
    pub heading_path: Vec<String>,
    /// Stable persisted chunk identifier.
    pub chunk_id: String,
    /// Zero-based chunk ordinal within the governed document.
    pub chunk_ordinal: usize,
    /// Retrieved chunk text.
    pub text: String,
    /// Token count emitted by chunking for this stored row.
    pub token_count: usize,
    /// Reason this chunk appears in the context result.
    pub match_reason: RetrievalMatchReason,
}

/// Reason an evidence chunk was selected for the context result.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMatchReason {
    /// The chunk survived ranking and deduplication.
    Primary,
    /// The chunk was added by adjacent chunk expansion.
    Expanded,
}

/// Diagnostics for the canonical context result.
///
/// # DTO(machine-readable retrieval diagnostics parsed from the `vector-database` CLI bridge and emitted as structured MCP output)
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RetrievalContextDiagnostics {
    /// Aggregate token count across returned evidence chunks.
    pub total_token_count: usize,
    /// Number of chunks dropped after final limit enforcement.
    pub dropped_after_limit: usize,
    /// Final retrieval limit applied to the context result.
    pub retrieval_limit: usize,
}

/// Builds a log notification for a single lifecycle step of the `rag.index` bridge.
fn index_lifecycle_log(step: &str) -> LoggingMessageNotificationParam {
    LoggingMessageNotificationParam::new(
        LoggingLevel::Info,
        serde_json::json!({ "tool": "rag.index", "message": step }),
    )
}

/// Builds a log notification from one structured progress event emitted by the `rag.index` bridge.
fn index_progress_event_log(event: &RagIndexProgressEvent) -> LoggingMessageNotificationParam {
    LoggingMessageNotificationParam::new(
        LoggingLevel::Info,
        serde_json::json!({
            "tool": "rag.index",
            "label": event.label,
            "package": event.package,
            "document_stem": event.document_stem,
            "message": event.message,
        }),
    )
}

/// MCP tool group for RAG operations.
///
/// Owns MCP tool definitions for the RAG capability domain.
/// Reusable retrieval logic lives outside `mcp-vector`; this struct is
/// only the MCP adapter boundary.
pub struct RagTools {
    tool_router: ToolRouter<Self>,
}

impl RagTools {
    /// Construct a new `RagTools` adapter.
    #[must_use]
    pub fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }
}

impl Default for RagTools {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BridgeCommandOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit: CommandExit,
}

fn resolve_workspace_root_from_runtime_context(
    tool_name: &str,
    _context: &rmcp::service::RequestContext<RoleServer>,
) -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|error| {
        format!("{tool_name} failed to resolve the MCP runtime workspace root: {error}")
    })
}

fn build_search_command(
    workspace_root: &std::path::Path,
    params: &RagSearchParams,
) -> Result<CommandSpec, String> {
    let query = params.query.trim();
    let mut builder = CommandBuilder::new(VECTOR_DATABASE_BINARY)
        .arg("rag")
        .arg("search")
        .arg(query)
        .arg("--json")
        .current_dir(workspace_root);

    if let Some(package) = params.package.as_deref() {
        builder = builder.arg("--package").arg(package);
    }
    if let Some(document) = params.document.as_deref() {
        builder = builder.arg("--document").arg(document);
    }
    if let Some(limit) = params.limit {
        builder = builder.arg("--limit").arg(limit.to_string());
    }

    builder.build().map_err(|error| format!("rag.search failed to prepare bridge command: {error}"))
}

fn build_index_command(
    workspace_root: &std::path::Path,
    subcommand: &'static str,
) -> Result<CommandSpec, String> {
    CommandBuilder::new(VECTOR_DATABASE_BINARY)
        .arg("rag")
        .arg(subcommand)
        .args((subcommand == "update-database").then_some("--json"))
        .current_dir(workspace_root)
        .build()
        .map_err(|error| {
            format!("rag.index failed to prepare `{subcommand}` bridge command: {error}")
        })
}

async fn execute_search_bridge<E>(
    executor: &E,
    workspace_root: &std::path::Path,
    params: &RagSearchParams,
) -> Result<RetrievalContext, String>
where
    E: CommandExecutor + Sync,
{
    let spec = build_search_command(workspace_root, params)?;
    let handle = executor.spawn(spec).await.map_err(|_| INSTALL_GUIDANCE.to_owned())?;
    let output = collect_command_output(handle).await?;

    if !output.exit.success {
        return Err(format_bridge_failure(&output));
    }

    serde_json::from_slice::<RetrievalContext>(&output.stdout).map_err(|error| {
        format!("rag.search received invalid retrieval JSON from vector-database: {error}")
    })
}

/// Runs the `rag init` then `rag update-database` lifecycle, calling `notify` at each step.
///
/// `notify` receives a [`LoggingMessageNotificationParam`] before each command, and once per
/// progress event from the parsed `update-database --json` output. Notification failures are
/// silently ignored so a disconnected client cannot abort an in-progress index run.
///
/// # Limitation
/// Per-line subprocess output is not available because [`CommandHandle::stream_output`] only
/// accepts synchronous callbacks. Notifications are therefore emitted at lifecycle boundaries
/// (before init, after init/before update-database) and per structured progress event from the
/// final parsed JSON, not as raw subprocess lines arrive.
async fn execute_index_bridge_with_progress<E, N, Fut>(
    executor: &E,
    workspace_root: &std::path::Path,
    notify: N,
) -> Result<RagIndexOutput, String>
where
    E: CommandExecutor + Sync,
    N: Fn(LoggingMessageNotificationParam) -> Fut,
    Fut: Future<Output = ()>,
{
    notify(index_lifecycle_log("starting rag init")).await;

    let init_spec = build_index_command(workspace_root, "init")?;
    let init_output = execute_index_command(executor, init_spec).await?;

    if !init_output.output.exit.success {
        return Err(format!(
            "rag.index init command failed: {}",
            format_bridge_failure(&init_output.output)
        ));
    }

    notify(index_lifecycle_log("init complete, starting update-database")).await;

    let update_spec = build_index_command(workspace_root, "update-database")?;
    let update_output = execute_index_command(executor, update_spec).await?;

    if !update_output.output.exit.success {
        return Err(format!(
            "rag.index update-database command failed: {}",
            format_bridge_failure(&update_output.output)
        ));
    }

    let index_output = RagIndexOutput {
        init: into_index_command_outcome(&init_output.spec, &init_output.output),
        update_database: parse_update_database_outcome(&update_output.spec, &update_output.output)?,
    };

    for event in &index_output.update_database.progress {
        notify(index_progress_event_log(event)).await;
    }

    Ok(index_output)
}

struct ExecutedBridgeCommand {
    spec: CommandSpec,
    output: BridgeCommandOutput,
}

async fn execute_index_command<E>(
    executor: &E,
    spec: CommandSpec,
) -> Result<ExecutedBridgeCommand, String>
where
    E: CommandExecutor + Sync,
{
    let handle = executor.spawn(spec.clone()).await.map_err(|_| INSTALL_GUIDANCE.to_owned())?;
    let output = collect_command_output(handle).await?;
    Ok(ExecutedBridgeCommand { spec, output })
}

fn into_index_command_outcome(
    spec: &CommandSpec,
    output: &BridgeCommandOutput,
) -> RagIndexCommandOutcome {
    RagIndexCommandOutcome {
        command: spec.command().to_owned(),
        args: spec.args().to_vec(),
        exit_code: output.exit.code,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn parse_update_database_outcome(
    spec: &CommandSpec,
    output: &BridgeCommandOutput,
) -> Result<RagIndexUpdateDatabaseOutcome, String> {
    let parsed =
        serde_json::from_slice::<RagIndexCliJsonOutput>(&output.stdout).map_err(|error| {
            format!("rag.index received invalid indexing JSON from vector-database: {error}")
        })?;
    Ok(RagIndexUpdateDatabaseOutcome {
        command: spec.command().to_owned(),
        args: spec.args().to_vec(),
        exit_code: output.exit.code,
        progress: parsed.progress,
        summary: parsed.summary,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

async fn collect_command_output(mut handle: CommandHandle) -> Result<BridgeCommandOutput, String> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    handle
        .stream_output(&mut |bytes| stdout.extend_from_slice(bytes), &mut |bytes| {
            stderr.extend_from_slice(bytes);
        })
        .await;
    let exit = handle
        .wait()
        .await
        .map_err(|error| format!("rag.search failed waiting for vector-database: {error}"))?;
    Ok(BridgeCommandOutput { stdout, stderr, exit })
}

fn format_bridge_failure(output: &BridgeCommandOutput) -> String {
    let stderr = sanitize_bridge_stream(&output.stderr);
    let stdout = sanitize_bridge_stream(&output.stdout);
    if !stderr.is_empty() {
        classify_bridge_failure(&stderr)
    } else if !stdout.is_empty() {
        classify_bridge_failure(&stdout)
    } else if let Some(code) = output.exit.code {
        format!("vector-database exited with code {code}")
    } else {
        "vector-database exited without an exit code".to_owned()
    }
}

fn sanitize_bridge_stream(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let without_help =
        trimmed.split_once(VECTOR_RAG_HELP_BANNER).map_or(trimmed, |(before, _)| before.trim_end());
    without_help.strip_prefix("error: ").unwrap_or(without_help).trim().to_owned()
}

fn classify_bridge_failure(detail: &str) -> String {
    if detail.contains("RAG store is missing at") {
        return format!("rag.search requires an initialized local RAG store: {detail}");
    }
    if detail.contains("incompatible with embedding contract")
        || detail.contains("embedding_model")
        || detail.contains("embedding_dimension")
    {
        return format!("rag.search found incompatible RAG embedding metadata: {detail}");
    }
    if detail.contains("failed to open LanceDB table")
        || detail.contains("failed to connect LanceDB database")
        || detail.contains("invalid vector column")
        || detail.contains("candidate query result is missing")
        || detail.contains("heading_path column")
        || detail.contains("chunk_ordinal column")
        || detail.contains("token_count column")
    {
        return format!("rag.search found a corrupt LanceDB table or schema: {detail}");
    }
    if detail.contains("package_filter must not be empty")
        || detail.contains("document_filter must not be empty")
    {
        return format!("rag.search rejected an invalid package or document filter: {detail}");
    }
    if detail.contains("query embedding failed")
        || detail.contains("query embedding returned no vectors")
    {
        return format!("rag.search failed to embed the query: {detail}");
    }

    format!("rag.search bridge command failed: {detail}")
}

#[tool_router]
impl RagTools {
    /// Search the local RAG index for governed document context.
    #[tool(
        name = "search",
        description = "Query the local RAG index for this workspace and return relevant governed document context."
    )]
    async fn search(
        &self,
        context: rmcp::service::RequestContext<RoleServer>,
        Parameters(params): Parameters<RagSearchParams>,
    ) -> Result<Json<RetrievalContext>, String> {
        let workspace_root = resolve_workspace_root_from_runtime_context("rag.search", &context)?;

        if params.query.trim().is_empty() {
            return Err("rag.search requires a non-empty query".to_owned());
        }

        let executor = runtime_io::ProcessCommandExecutor::default();
        let context_result = execute_search_bridge(&executor, &workspace_root, &params).await?;
        Ok(Json(context_result))
    }

    /// Initialize and update the local RAG index for this workspace.
    #[tool(
        name = "index",
        description = "Initialize the local RAG store for this workspace and update the workspace RAG index."
    )]
    async fn index(
        &self,
        context: rmcp::service::RequestContext<RoleServer>,
        Parameters(_params): Parameters<RagIndexParams>,
    ) -> Result<Json<RagIndexOutput>, String> {
        let workspace_root = resolve_workspace_root_from_runtime_context("rag.index", &context)?;
        let executor = runtime_io::ProcessCommandExecutor::default();
        let peer = context.peer.clone();
        let index_result =
            execute_index_bridge_with_progress(&executor, &workspace_root, move |param| {
                let peer = peer.clone();
                async move {
                    let _ = peer.notify_logging_message(param).await;
                }
            })
            .await?;
        Ok(Json(index_result))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for RagTools {}

#[cfg(test)]
#[path = "rag_test.rs"]
mod tests;
