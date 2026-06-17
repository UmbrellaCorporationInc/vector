---
id: task-00074-implement-rfc-00042-phase-10-mcp-search-tool
type: task
code: "00074"
slug: implement-rfc-00042-phase-10-mcp-search-tool
title: Implement RFC 00042 Phase 10 MCP Search Tool
description: Implement the RAG-scoped MCP search tool proposed by RFC 00042 and keep its output aligned with the Phase 9 retrieval context contract.
status: in-progress
created: 2026-06-17
updated: 2026-06-17
tags:
  - rag
  - mcp
  - search
  - retrieval
related:
  - rfc-00042-phase-10-mcp-search-tool
  - rfc-00041-phase-9-canonical-result-for-retrieval-operation
  - spec-00011-rag-plan-implementation
supersedes: []
superseded_by: null
---

# Task 00074: Implement RFC 00042 Phase 10 MCP Search Tool

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: task-00074-implement-rfc-00042-phase-10-mcp-search-tool
```

## 1. Prime Directive

> [!Prime Directive]
> Expose the local RAG retrieval operation through MCP as a focused `rag.search` tool that returns attributed governed document context without generating answers or inventing an MCP-only result contract.

## 2. Specs

- **Module:** `mcp-vector`, `vector-database`, RAG runtime search operation
- **Dependencies:** [[rfc-00042-phase-10-mcp-search-tool]], [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]], [[spec-00011-rag-plan-implementation]]

## 3. Checklist

### 3.1. Phase A: Locate the MCP Tool Boundary

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase A
  language: rust
```

- [x] Identify the existing MCP tool registration pattern and add the RAG category tool named `search` in the same style.
- [x] Ensure the user-facing tool description states that it queries the local RAG index and returns relevant governed document context.
- [x] Resolve the workspace root from MCP runtime context instead of accepting a caller-provided path.
- [x] Confirm whether the MCP registry exposes the tool as `rag.search` or flattens category and name into a single identifier.

### 3.2. Phase B: Implement the Input and Execution Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase B
  language: rust
```

- [x] Add an input shape with required non-empty `query` plus optional `limit`, `package`, and `document` fields.
- [x] Reject blank queries before invoking retrieval.
- [x] Apply `limit` as an override of the configured final retrieval limit when present.
- [x] Apply `package` and `document` filters through the shared retrieval path or bridge command.
- [x] Delegate to the shared RAG runtime search operation when available.
- [x] If a direct runtime call is not available for this phase, delegate to `vector-database rag search --json` as the bridge command.

### 3.3. Phase C: Reuse the Canonical Retrieval Context

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase C
  language: rust
```

- [x] Return the Phase 9 `RetrievalContext` shape defined by [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]].
- [x] Preserve query, retrieval status, effective limit, source entries, evidence chunks, package identity, governed document stems, heading paths, token counts, and diagnostics.
- [x] Keep empty results as successful structured responses with `status: empty`.
- [x] Do not generate answers, summarize evidence chunks, rewrite retrieved text, or decide evidence sufficiency inside the MCP tool.
- [x] Keep CLI JSON output and MCP output contract-compatible for the same query.

### 3.4. Phase D: Map Operational Failures

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase D
  language: rust
```

- [x] Return actionable MCP errors for missing RAG stores.
- [x] Return actionable MCP errors for missing or incompatible embedding model metadata.
- [x] Return actionable MCP errors for corrupt LanceDB tables.
- [x] Return actionable MCP errors for invalid package or document filters.
- [x] Return actionable MCP errors for query embedding failures.
- [x] If using the bridge command, return actionable MCP errors for `vector-database` invocation failures.
- [x] If using the bridge command, return actionable MCP errors for non-zero CLI exits.
- [x] If using the bridge command, return actionable MCP errors for invalid or incompatible JSON output.
- [x] Do not collapse operational failures into empty retrieval responses.

Phase D validated on 2026-06-17 with the MCP RAG bridge test suite and Rust quality gates.

### 3.5. Phase E: Test the MCP Search Tool

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase E
  language: rust
```

- [x] Add or update tests for valid query results.
- [x] Add or update tests for empty result responses.
- [x] Add or update tests for `limit` overrides.
- [x] Add or update tests for package filters.
- [x] Add or update tests for document filters.
- [x] Add or update tests for invalid filters.
- [x] Add or update tests for missing index failures.
- [x] If using the bridge command, add or update tests for `vector-database` execution failures.
- [x] If using the bridge command, add or update tests for non-zero CLI exits.
- [x] If using the bridge command, add or update tests for bridge JSON parse failures.
- [x] Add or update a compatibility test that proves CLI JSON output and MCP output use the same retrieval context contract.

Phase E validated on 2026-06-17 with the MCP `rag.search` bridge test suite. The focused Rust tests passed. The broader Rust quality-gate commands remain blocked in this environment because `protoc` is not installed and an agent-generated workspace `.cargo-target/` directory is being scanned by the repository lint rules.

### 3.6. Phase F: Locate the MCP Index Tool Boundary

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase F
  language: rust
```

- [x] Add the RAG category tool named `index` in the same MCP tool group and registration style as `search`.
- [x] Ensure the user-facing tool description states that it initializes the local RAG store and updates the workspace RAG index.
- [x] Resolve the workspace root from MCP runtime context instead of accepting a caller-provided path.
- [x] Confirm whether the MCP registry exposes the tool as `rag.index` or flattens category and name into a single identifier.
- [x] Keep the `index` tool focused on index lifecycle work and separate from query retrieval behavior.

Phase F validated on 2026-06-17 with focused `mcp-vector` Rust tests covering the new RAG `index` tool metadata, flattened MCP registration, and transport listing. The Rust quality gate prompt was resolved for `rust`. `xtask quality-lint -p mcp-vector` still fails because repository lint rules scan the agent-generated workspace `.cargo-target/` directory and report third-party generated files outside this task's scope.

### 3.7. Phase G: Implement the Index Execution Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase G
  language: rust
```

- [x] Run `vector-database rag init` from the resolved workspace root before updating the index.
- [x] Run `vector-database rag update-database` from the resolved workspace root after a successful init.
- [x] Return a structured MCP success response that includes both command outcomes.
- [x] Do not accept caller-provided shell commands or arbitrary command arguments.
- [x] Do not query the RAG index or return retrieval context from the `index` tool.

Phase G validated on 2026-06-17 with focused `mcp-vector` Rust tests covering the `rag.index` bridge command construction, ordered `init` then `update-database` execution, install-guidance failures, and skip-on-init-failure behavior. The Rust quality gate prompt was resolved for `rust`. Failure classification for `rag.index` remains intentionally generic until Phase L defines the actionable error mapping contract.

### 3.8. Phase H: Index Workspace and Package Documents

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase H
  language: rust
```

- [x] Update `vector-rag rag update-database` so the indexing pass includes the workspace `doc/` corpus and every synchronized package `doc/` corpus under `.vector-database/packages/{package}/doc/`.
- [x] Derive package document roots from the synchronized package directory layout instead of requiring caller-provided package paths.
- [x] Preserve package identity on every indexed chunk so retrieval, filtering, and citations remain package-aware.
- [x] Treat missing package `doc/` folders as package-structure issues, not as workspace discovery failures.
- [x] Reconcile deleted documents separately for workspace-local and package-qualified document identities.
- [x] Add or update tests that prove package documents are discovered, indexed, skipped when unchanged, and deleted when removed.
- [x] Add or update tests that prove workspace and package documents with the same governed document stem do not overwrite each other.

Phase H validated on 2026-06-17 with focused `runtime-rag` package-indexing tests plus full `cargo test -p runtime-rag` coverage. The Rust quality gate prompt was resolved for `rust`. `xtask quality-lint` remains blocked by repository-wide rule violations under the generated workspace `.cargo-target/` directory, while `xtask quality-test` and `cargo fmt --all` passed.

### 3.9. Phase I: Stream Index Progress in the CLI

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase I
  language: rust
```

- [ ] Change `vector-rag rag update-database` so indexing progress is emitted while the operation is running instead of only after the final operation output is received.
- [ ] Preserve the existing final summary with re-indexed, skipped, and deleted document counts.
- [ ] Emit useful progress for long-running steps such as store initialization, document discovery, document indexing, skipped unchanged documents, deleted stale chunks, and document-level failures when that information is available.
- [ ] Emit one progress line for each newly indexed or re-indexed document, including package identity when present and governed document stem.
- [ ] Emit one progress line for each unchanged document that was already indexed and skipped, including package identity when present and governed document stem.
- [ ] Emit one progress line for each document-level indexing error, including package identity when present, governed document stem, and the actionable error message.
- [ ] Use stable progress labels such as `indexed`, `unchanged`, and `failed` so CLI users and MCP consumers can parse or scan progress consistently.
- [ ] Flush CLI progress output so users can tell the command is still running.
- [ ] Confirm that `vector-database rag update-database` forwards `vector-rag` progress incrementally through the existing passthrough streaming path.
- [ ] Do not make CLI progress output part of the `RetrievalContext` search contract.

### 3.10. Phase J: Decide the Agent-Facing Index Output Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase J
  language: rust
```

- [ ] Evaluate whether agent-facing index output should remain final-response `--json` or introduce a streaming NDJSON event contract.
- [ ] Compare final-response `--json` against NDJSON streaming for MCP consumption, CLI automation, partial progress visibility, parser stability, and backwards compatibility.
- [ ] If NDJSON is selected, define stable event names and fields for `started`, `indexed`, `unchanged`, `failed`, `deleted`, and `summary` events.
- [ ] If final-response `--json` is retained, document how MCP `index` exposes progress without requiring agents to parse human CLI text.
- [ ] Keep human-oriented CLI output separate from the machine-readable agent contract.
- [ ] Preserve compatibility for existing JSON consumers or document the migration path explicitly.
- [ ] Record the chosen contract in the task notes, README, or a follow-up RFC if the decision changes a public CLI/MCP contract.

### 3.11. Phase K: Define MCP Index Progress Behavior

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase K
  language: rust
```

- [ ] Verify whether the current `rmcp` server and supported MCP clients can surface tool progress or logging notifications during a long-running tool call.
- [ ] If MCP progress notifications are supported, emit index lifecycle progress from the `index` tool while `vector-database rag init` and `vector-database rag update-database` run.
- [ ] If MCP progress notifications are not supported by the current stack, return a structured final response that includes captured command output and document the limitation in the task notes or implementation comments.
- [ ] Keep the final `index` tool result deterministic even when progress streaming is enabled.
- [ ] Do not fake streaming by delaying the final MCP response with accumulated logs only.

### 3.12. Phase L: Map Index Operational Failures and Tests

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase L
  language: rust
```

- [ ] Return actionable MCP errors when `vector-database rag init` cannot be invoked.
- [ ] Return actionable MCP errors when `vector-database rag init` exits non-zero.
- [ ] Return actionable MCP errors when `vector-database rag update-database` cannot be invoked.
- [ ] Return actionable MCP errors when `vector-database rag update-database` exits non-zero.
- [ ] Add or update tests that prove `index` runs init before update-database.
- [ ] Add or update tests that prove update-database is skipped when init fails.
- [ ] Add or update tests for successful command output mapping.
- [ ] Add or update tests for invocation failures and non-zero exits for both commands.
- [ ] Add or update tests that prove CLI progress is emitted before indexing completes.
- [ ] Add or update tests that prove CLI progress reports indexed documents, unchanged skipped documents, and document-level failures separately.
- [ ] Add or update tests that prove `vector-database rag update-database` forwards `vector-rag` output incrementally.
- [ ] Add or update tests for the chosen agent-facing output contract, including final-response `--json` or streaming NDJSON behavior.
- [ ] Add or update MCP progress tests when progress notifications are supported, or a test that proves the final MCP response includes captured command output when they are not supported.

### 3.13. Phase Z: Wrap-Up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00074
  phase: Phase Z
  language: rust
```

- [ ] Run the relevant Rust formatting, linting, and test commands for the modified crates.
- [ ] Run governed document validation after code and document edits.
- [ ] Update README files for packages whose public MCP or RAG behavior changed.
- [ ] Confirm [[rfc-00042-phase-10-mcp-search-tool]] acceptance criteria are fully covered or explicitly note any deferred item.

## 4. Staff Engineer Review

This task is scoped correctly around one MCP primitive: search should return retrieval context, not an answer. The highest-risk part is the bridge boundary. Shelling out to `vector-database rag search --json` is acceptable for Phase 10 only if errors remain typed and actionable; otherwise the MCP layer will turn domain failures into vague process or JSON failures.

The implementation should prefer a direct shared runtime operation as soon as the boundary is available. That keeps CLI and MCP as thin adapters over one retrieval contract. If the bridge is used first, tests must make the bridge behavior explicit so it does not become an undocumented permanent architecture.
