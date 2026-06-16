---
id: task-00072-implement-rfc-00041-phase-9-canonical-result-for-retrieval-operation
type: task
code: "00072"
slug: implement-rfc-00041-phase-9-canonical-result-for-retrieval-operation
title: Implement RFC 00041 Phase 9 Canonical Result For Retrieval Operation
description: Implement the canonical retrieval context result contract shared by the RAG runtime, CLI output, and MCP output.
status: in-progress
created: 2026-06-16
updated: 2026-06-16
tags:
  - rag
  - retrieval
  - context
  - cli
  - mcp
related:
  - rfc-00041-phase-9-canonical-result-for-retrieval-operation
supersedes: []
superseded_by: null
---

# Task 00072: Implement RFC 00041 Phase 9 Canonical Result For Retrieval Operation

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: task-00072-implement-rfc-00041-phase-9-canonical-result-for-retrieval-operation
```

## 1. Prime Directive

> [!Prime Directive]
> Implement [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]] so retrieval evidence has one canonical runtime contract before it reaches CLI or MCP adapters.
> This eliminates adapter-level drift in result shape, source attribution, empty-result handling, token diagnostics, and final limit enforcement.

## 2. Specs

- **Module:** RAG runtime, CLI RAG search command, MCP retrieval surface
- **Dependencies:** [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]], [[rfc-00040-phase-8-hybrid-search]], [[spec-00011-rag-plan-implementation]]

## 3. Checklist

### 3.1. Phase A - Runtime Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00072
  phase: Phase A
  language: rust
```

- [x] Define typed runtime structures for `RetrievalContext`, `RetrievalContextStatus`, `RetrievalContextSource`, `RetrievalContextChunk`, `RetrievalMatchReason`, and `RetrievalContextDiagnostics`.
- [x] Keep the contract model-agnostic and free of CLI-only, MCP-only, or LLM-generated fields.
- [x] Preserve package identity, governed document stem, heading path, chunk id, chunk ordinal, text, and token count for every returned chunk.
- [x] Represent empty retrieval as a successful `empty` result with no sources and no chunks.
- [x] Add focused unit tests for the canonical data shape and empty-result status.

### 3.2. Phase B - Context Assembly Operation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00072
  phase: Phase B
  language: rust
```

- [x] Introduce the runtime operation that converts Phase 8 retrieval hits into `RetrievalContext`.
- [x] Ensure the operation does not invoke an LLM, summarize text, rewrite chunk text, or reopen source files.
- [x] Normalize repeated source attribution into shared `RetrievalContextSource` entries.
- [x] Assign response-local context ids such as `ctx-1`, `ctx-2`, and stable source ids.
- [x] Preserve whether each chunk is `primary` or `expanded`.
- [x] Enforce the configured final retrieval limit after deduplication and adjacent expansion.
- [x] Report diagnostic totals for total token count, retrieval limit, and chunks dropped after limit enforcement.
- [x] Add tests for primary chunks, expanded chunks, repeated sources, limit truncation, package-qualified sources, and diagnostic token totals.

### 3.3. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00072
  phase: Phase Z
  language: rust
```

- [x] Update README files for any modified package whose documented RAG behavior changes.
- [x] Run formatting, linting, and the relevant Rust test suite.
- [x] Verify `validate_fix` passes for governed documentation.
- [x] Confirm acceptance criteria from [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]] are either implemented or explicitly deferred in a follow-up task.

### 3.4. Phase Z Acceptance Review

- Implemented in this task: runtime `RetrievalContext` contract, source attribution, selected chunk fields, final limit enforcement, empty-result semantics, no LLM or source-file reopening in context assembly, and tests for primary chunks, expanded chunks, repeated sources, final limit truncation, empty results, package-qualified sources, and diagnostic token totals.
- Deferred to [[task-00071-update-rag-cli-search-to-emit-retrieval-context]]: CLI JSON output, `vector-database rag search <query>` human rendering, and MCP retrieval output adoption of the canonical context shape.

## 4. Staff Engineer Review

The implementation should treat [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]] as a contract boundary, not a display cleanup. The main risk is allowing CLI or MCP convenience code to shape the result independently, because that recreates the drift this task is supposed to remove.

The strongest implementation path is a small typed runtime operation with tests around semantics instead of presentation. Keep diagnostics useful but restrained. If score internals or verbose ranking details are exposed too early, external consumers may couple to fields that should remain debugging aids.

The important tradeoff is that adapters become thinner and less flexible. That is acceptable here because the canonical result is the product boundary. Presentation can vary, but attribution, limit handling, empty results, and token diagnostics should not.
