---
id: rfc-00042-phase-10-mcp-search-tool
type: rfc
code: "00042"
slug: phase-10-mcp-search-tool
title: Phase 10 MCP Search Tool
description: Proposes a RAG-scoped MCP search tool that queries the local RAG index and returns relevant governed document context.
status: accepted
created: 2026-06-17
updated: 2026-06-17
authors: []
tags:
  - rag
  - mcp
  - search
  - retrieval
related:
  - spec-00011-rag-plan-implementation
  - rfc-00041-phase-9-canonical-result-for-retrieval-operation
  - task-00074-implement-rfc-00042-phase-10-mcp-search-tool
supersedes: []
superseded_by: null
aliases:
  - "RFC 00042: Phase 10 MCP Search Tool"
---

# RFC 00042: Phase 10 MCP Search Tool

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00042-phase-10-mcp-search-tool`
  document-type: task
  document-name: implement-rfc-00042-phase-10-mcp-search-tool
```

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: rfc-00042-phase-10-mcp-search-tool
```

## 1. Problem

[[spec-00011-rag-plan-implementation]] defines Phase 10 as the point where the RAG workflow becomes available through user-facing commands or MCP tools. Earlier phases establish indexing, hybrid retrieval, and canonical context assembly, especially [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]]. What is still missing is the MCP tool that lets an agent query the local RAG index and receive relevant Vector documents as structured context.

The current gap has several concrete parts:

- Agents can only benefit from the local RAG index if a tool exposes the retrieval operation through MCP.
- The MCP surface needs a concise, discoverable tool in the `rag` category for searching governed documentation.
- Search results must preserve package, document stem, heading path, and chunk attribution instead of returning unstructured text.
- The tool needs predictable error behavior for missing indexes, stale indexes, embedding metadata mismatches, and empty results.
- The MCP contract should stay aligned with CLI JSON output instead of defining an independent result shape.

Without this tool, Phase 10 would leave the RAG system usable from a terminal but not from the MCP context where agents actually need retrieval.

## 2. Proposal

Add an MCP tool in the `rag` category named `search`. The tool queries the local RAG index for the active workspace and returns the relevant files, sections, and chunks as structured retrieval context.

The user-facing description should be:

> Query the local RAG index for this workspace and return relevant governed document context.

The tool should be exposed as a RAG tool rather than a generic document search tool because it depends on the RAG index, embedding configuration, retrieval defaults, and Phase 9 context contract.

### 2.1 MCP Tool Contract

The MCP tool should accept a small input shape:

```text
RagSearchToolInput {
  query: String,
  limit: Option<usize>,
  package: Option<String>,
  document: Option<String>,
}
```

Required behavior:

- `query` is required and must be non-empty after trimming.
- `limit` overrides the configured final retrieval limit when present.
- `package` filters results to one synchronized package when present.
- `document` filters results to one governed document stem when present.
- The tool resolves the workspace root from the MCP runtime context, not from caller-provided paths.

The output should reuse the canonical retrieval context shape from [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]] rather than inventing an MCP-only payload:

```text
RagSearchToolOutput {
  context: RetrievalContext,
}
```

This keeps MCP output, CLI JSON output, and runtime tests aligned around one stable contract.

### 2.2 Execution Boundary

The preferred implementation is for the MCP adapter to call the same runtime retrieval operation used by the CLI. For the Phase 10 bridge implementation, the MCP adapter should delegate to `vector-database rag search --json` because `vector-database` is distributed with `mcp-vector` and can own the RAG search logic, error mapping, and canonical JSON output.

The long-term boundary should be:

```text
MCP rag.search tool -> RAG runtime search operation -> RetrievalContext
```

The bridge boundary, if needed, should be:

```text
MCP rag.search tool -> vector-database rag search --json -> RetrievalContext parser
```

The bridge is acceptable only if it preserves the canonical context fields exactly and maps CLI failures into structured MCP tool errors. If `vector-database` cannot be executed, returns a non-zero exit code, or emits invalid JSON, the MCP tool must fail with an actionable tool error rather than returning an empty retrieval result.

`vector-rag search --json` may remain a compatible external CLI, but it is not the Phase 10 MCP baseline because it may not be installed in the MCP process `PATH`. The MCP adapter should not depend on a binary that is outside the `mcp-vector` distribution unless that binary is resolved explicitly and failures are reported with the same semantics as `vector-database` execution failures.

### 2.3 Result Semantics

The tool returns retrieved context only. It must not generate answers, summarize evidence, rewrite chunk text, or decide whether the retrieved evidence is sufficient. That responsibility belongs to the agent or MCP client consuming the tool output.

For successful searches, the result must include:

- The original query.
- Retrieval status.
- The effective result limit.
- Returned source entries.
- Returned evidence chunks.
- Package identity when the result comes from a synchronized package.
- Governed document stems.
- Heading paths.
- Token counts.
- Diagnostics needed to understand truncation or empty results.

Empty results are successful retrieval responses with `status: empty`, not transport errors.

### 2.4 Error Semantics

The tool should return actionable MCP errors for operational failures:

- Missing RAG store.
- Missing or incompatible embedding model metadata.
- Corrupt LanceDB table.
- Invalid package or document filter.
- Query embedding failure.
- `vector-database` invocation failure when using the bridge implementation.
- JSON parse failure when using the bridge implementation.

These failures should not be collapsed into an empty result, because an empty result means the search ran successfully and found no evidence.

### 2.5 Index Status Relationship

[[spec-00011-rag-plan-implementation]] also asks Phase 10 to show index status and recent indexing failures. This RFC intentionally limits itself to the MCP `rag.search` tool. Index status should remain a separate tool or command, such as `rag.status`, because status inspection has different inputs, output semantics, and failure modes from query execution.

Keeping search and status separate avoids overloading `rag.search` with lifecycle diagnostics that agents do not need on every query.

## 3. Alternatives Considered

- **Expose only a CLI command and skip MCP:** Discarded because Phase 10 explicitly needs the workflow available through the user interface layer, and agents consume local context through MCP tools.
- **Name the tool simply `search` outside the RAG category:** Discarded because this would imply a broader project-wide search capability. The tool depends specifically on the RAG index and should be discoverable under `rag`.
- **Return raw file paths and text snippets:** Discarded because Vector's governed document identity is package-aware and stem-based. Raw paths lose package attribution and are weaker as citations.
- **Let the MCP tool return plain Markdown context:** Discarded because plain text loses source structure, token counts, empty-result status, and diagnostics already defined by [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]].
- **Implement MCP by shelling out to `vector-rag search --json`:** Discarded as the Phase 10 baseline because `vector-rag` may not be installed in the MCP process `PATH`. If supported later, it must fail with the same actionable semantics as a `vector-database` execution failure.
- **Implement MCP by shelling out to `vector-database rag search --json` permanently:** Accepted as the pragmatic bridge for Phase 10 because `vector-database` is distributed with `mcp-vector` and can own the RAG search behavior. It is still a process bridge, so direct runtime calls remain the cleaner long-term architecture.
- **Combine search and index status in one MCP tool:** Discarded because query retrieval and index health are different operations. A caller that wants status should ask for status explicitly.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| A RAG-scoped MCP `search` tool gives agents direct access to the local governed-document index. | The MCP surface becomes part of the public contract and must remain stable once agents depend on it. |
| Reusing `RetrievalContext` keeps CLI JSON and MCP output aligned. | Any future change to the canonical retrieval contract affects both CLI and MCP consumers. |
| A minimal input shape keeps the tool easy to call and test. | Advanced retrieval tuning is deferred until benchmarks justify additional parameters. |
| Treating empty results as successful structured responses avoids ambiguity. | Callers must distinguish empty retrieval from operational errors explicitly. |
| Using `vector-database rag search --json` lets the MCP adapter rely on the CLI distributed with `mcp-vector`. | A bridge adds process overhead and can hide domain errors behind serialization failures if not handled carefully. |
| Keeping status as a separate tool keeps search focused. | Users need to call a second tool when they want index health information. |

## 5. Acceptance Criteria

- [ ] The MCP server exposes a RAG category tool named `search`.
- [ ] The tool description says that it queries the local RAG index and returns relevant governed document context.
- [ ] The tool accepts `query`, optional `limit`, optional `package`, and optional `document` inputs.
- [ ] The tool resolves the workspace root from MCP runtime context rather than accepting arbitrary caller paths.
- [ ] The tool delegates to the shared RAG runtime search operation, or to `vector-database rag search --json` as the Phase 10 CLI JSON bridge.
- [ ] The tool output reuses the Phase 9 `RetrievalContext` shape.
- [ ] Empty results return a successful structured `empty` retrieval context.
- [ ] Missing stores, incompatible embedding metadata, invalid filters, query embedding failures, `vector-database` invocation failures, non-zero CLI exits, and JSON parse failures return actionable MCP errors.
- [ ] The tool does not generate answers, summarize chunks, or rewrite retrieved text.
- [ ] Tests cover valid queries, empty results, limit overrides, package filters, document filters, invalid filters, missing index failures, `vector-database` execution failures, non-zero CLI exits, and bridge JSON parse failures if the bridge implementation is used.
- [ ] CLI JSON output and MCP output remain contract-compatible for the same query.

## 6. Open Questions

- Should `vector-database rag search --json` be the only supported MCP bridge command, or should `vector-rag search --json` be supported later through explicit binary resolution?
- Should the MCP tool name be surfaced to clients as `rag.search`, or does the current MCP tool registry flatten category and name into a single identifier?
- Should `limit` be capped by a governed maximum to prevent very large MCP responses?
- Should the tool expose a debug flag for score diagnostics, or should diagnostics remain fixed in the canonical Phase 9 context output?

## 7. Staff Engineer Review

The proposed tool is the right product shape: agents need a focused MCP search primitive that returns attributed context, not a generated answer. The strongest architectural point is reusing [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]] so the project does not create separate CLI and MCP retrieval contracts.

The main concern is the idea of implementing MCP by shelling out to a CLI that is not guaranteed to exist in the MCP runtime environment. `vector-rag search --json` is therefore the wrong baseline if it may not be installed in `PATH`. `vector-database rag search --json` is the better bridge because `vector-database` is distributed with `mcp-vector` and can own the command contract. A process boundary still makes error typing weaker, adds avoidable overhead, and risks turning CLI JSON into an accidental internal API. The better long-term design is one runtime operation with thin CLI and MCP adapters.

There is also a naming risk. Introducing `vector-rag search --json` into the MCP baseline would fragment the interface when `vector-database rag search --json` is already the command shape aligned with `mcp-vector`. My recommendation is to keep the MCP tool as `rag.search`, use `vector-database rag search --json` for the Phase 10 bridge, keep status as a separate operation, and make both adapters consume the same runtime result contract directly wherever possible.
