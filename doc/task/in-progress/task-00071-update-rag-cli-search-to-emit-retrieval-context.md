---
id: task-00071-update-rag-cli-search-to-emit-retrieval-context
type: task
code: "00071"
slug: update-rag-cli-search-to-emit-retrieval-context
title: Update RAG CLI Search To Emit RetrievalContext
description: Update the existing RAG search CLI command to render and serialize the Phase 9 canonical RetrievalContext contract.
status: in-progress
created: 2026-06-16
updated: 2026-06-16
tags:
  - rag
  - cli
  - retrieval
  - context
related:
  - rfc-00041-phase-9-canonical-result-for-retrieval-operation
  - rfc-00040-phase-8-hybrid-search
  - spec-00011-rag-plan-implementation
supersedes: []
superseded_by: null
---

# Task 00071: Update RAG CLI Search To Emit RetrievalContext

## 1. Prime Directive

> [!Prime Directive]
> Eliminate CLI output drift by making the existing `vector-database rag search <query>` command consume the Phase 9 context assembly operation and emit the canonical `RetrievalContext` semantics instead of raw Phase 8 retrieval hits.

## 2. Specs

- **Module:** `runtime-rag`, CLI RAG command adapter
- **Dependencies:** [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]], [[rfc-00040-phase-8-hybrid-search]], [[spec-00011-rag-plan-implementation]]

## 3. Checklist

### 3.1. Phase A — Runtime Integration

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00071
  phase: Phase A
  language: rust
```

- [ ] Route the CLI search execution path through the Phase 9 context assembly operation after Phase 8 retrieval completes.
- [ ] Preserve the existing query, limit, package, document, and JSON CLI inputs unless the RFC requires a contract-level change.
- [ ] Ensure the command receives `RetrievalContext` before human rendering or JSON serialization.
- [ ] Keep retrieval ranking, deduplication, limit enforcement, source attribution, and empty-result semantics out of the CLI adapter.

### 3.2. Phase B — Output Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00071
  phase: Phase B
  language: rust
```

- [ ] Make `--json` serialize `RetrievalContext` directly or through a thin compatibility wrapper that preserves all canonical fields and semantics.
- [ ] Update human-readable CLI output to render `RetrievalContext` sources, chunks, empty status, and diagnostics clearly.
- [ ] Preserve structured empty retrieval responses as successful command output.
- [ ] Preserve non-zero exit behavior for actionable failures such as missing indexes, incompatible embedding metadata, malformed filters, or query execution failures.

### 3.3. Phase C — Tests And Validation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00071
  phase: Phase C
  language: rust
```

- [ ] Add or update CLI JSON tests to assert the Phase 9 canonical context shape or approved compatibility wrapper.
- [ ] Add or update CLI human-output tests for primary chunks, expanded chunks, repeated sources, package-qualified sources, and diagnostics.
- [ ] Add or update empty-result tests to assert `status: empty`, zero returned chunks, no sources, and successful command completion.
- [ ] Run the relevant quality gates for the modified Rust crates.

### 3.4. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00071
  phase: Phase Z
  language: rust
```

- [ ] Update any CLI usage documentation affected by the JSON or human-readable output shape.
- [ ] Confirm [[rfc-00041-phase-9-canonical-result-for-retrieval-operation]] acceptance criteria remain satisfied by the implementation.
