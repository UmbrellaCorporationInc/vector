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

- [ ] Add an input shape with required non-empty `query` plus optional `limit`, `package`, and `document` fields.
- [ ] Reject blank queries before invoking retrieval.
- [ ] Apply `limit` as an override of the configured final retrieval limit when present.
- [ ] Apply `package` and `document` filters through the shared retrieval path or bridge command.
- [ ] Delegate to the shared RAG runtime search operation when available.
- [ ] If a direct runtime call is not available for this phase, delegate to `vector-database rag search --json` as the bridge command.

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

- [ ] Return the Phase 9 `RetrievalContext` shape defined by [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]].
- [ ] Preserve query, retrieval status, effective limit, source entries, evidence chunks, package identity, governed document stems, heading paths, token counts, and diagnostics.
- [ ] Keep empty results as successful structured responses with `status: empty`.
- [ ] Do not generate answers, summarize evidence chunks, rewrite retrieved text, or decide evidence sufficiency inside the MCP tool.
- [ ] Keep CLI JSON output and MCP output contract-compatible for the same query.

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

- [ ] Return actionable MCP errors for missing RAG stores.
- [ ] Return actionable MCP errors for missing or incompatible embedding model metadata.
- [ ] Return actionable MCP errors for corrupt LanceDB tables.
- [ ] Return actionable MCP errors for invalid package or document filters.
- [ ] Return actionable MCP errors for query embedding failures.
- [ ] If using the bridge command, return actionable MCP errors for `vector-database` invocation failures.
- [ ] If using the bridge command, return actionable MCP errors for non-zero CLI exits.
- [ ] If using the bridge command, return actionable MCP errors for invalid or incompatible JSON output.
- [ ] Do not collapse operational failures into empty retrieval responses.

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

- [ ] Add or update tests for valid query results.
- [ ] Add or update tests for empty result responses.
- [ ] Add or update tests for `limit` overrides.
- [ ] Add or update tests for package filters.
- [ ] Add or update tests for document filters.
- [ ] Add or update tests for invalid filters.
- [ ] Add or update tests for missing index failures.
- [ ] If using the bridge command, add or update tests for `vector-database` execution failures.
- [ ] If using the bridge command, add or update tests for non-zero CLI exits.
- [ ] If using the bridge command, add or update tests for bridge JSON parse failures.
- [ ] Add or update a compatibility test that proves CLI JSON output and MCP output use the same retrieval context contract.

### 3.6. Phase Z: Wrap-Up

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
