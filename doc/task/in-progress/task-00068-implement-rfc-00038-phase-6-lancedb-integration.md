---
id: task-00068-implement-rfc-00038-phase-6-lancedb-integration
type: task
code: "00068"
slug: implement-rfc-00038-phase-6-lancedb-integration
title: Implement RFC 00038 Phase 6 LanceDB Integration
description: Implement the LanceDB retrieval store, the RAG-owned database lifecycle operation, and CLI integration for Phase 6.
status: in-progress
created: 2026-06-15
updated: 2026-06-15
tags:
  - rag
  - lancedb
  - retrieval
  - cli
related:
  - rfc-00038-phase-6-lancedb-integration
  - spec-00011-rag-plan-implementation
supersedes: []
superseded_by: null
---

# Task 00068: Implement RFC 00038 Phase 6 LanceDB Integration

## 1. Prime Directive

> [!Prime Directive]
> Eliminate the persistence gap in the RAG pipeline by adding the Phase 6 LanceDB store in `runtime-rag` and wiring `vector-database` to invoke the RAG-owned database lifecycle operation instead of owning schema logic itself.

## 2. Specs

- **Module:** `runtime-rag`; `vector-database`
- **Dependencies:** `lancedb`; any additional storage or schema dependency must be justified before use.
- **Source:** [[rfc-00038-phase-6-lancedb-integration]]

## 3. Checklist

### 3.1. Phase A - LanceDB Schema Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00068
  phase: Phase A
  language: Rust
```

- [x] Define the Phase 6 row shape for `chunk_id`, package identity, governed document identity, document and chunk hashes, heading path, frontmatter, raw text, token count, embedding metadata, and vector payload.
- [x] Define stable `chunk_id` generation from package, document stem, chunk ordinal, and chunk hash.
- [x] Ensure the schema design preserves raw chunk text for inspection and full-text indexing.
- [x] Define the metadata representation needed for filters over package, document stem, heading path, tags, and selected frontmatter fields.

### 3.2. Phase B - RAG-Owned Store Lifecycle

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00068
  phase: Phase B
  language: Rust
```

- [x] Add a high-level operation in `runtime-rag` that creates or updates the LanceDB store under `.vector-database/rag/lancedb/`.
- [x] Keep LanceDB-specific schema and index creation behind the RAG persistence boundary instead of exposing it through CLI code.
- [x] Make table creation idempotent across repeated runs.
- [x] Create the full-text inverted index on `text` during lifecycle initialization.

### 3.3. Phase C - Indexing Integration

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00068
  phase: Phase C
  language: Rust
```

- [x] Wire the Phase 6 store to accept chunking and embedding outputs from the RAG pipeline.
- [x] Implement deterministic upserts keyed by `chunk_id`.
- [x] Replace stale document rows deterministically by package and document stem when a document changes or is deleted.
- [x] Keep raw text and metadata inspectable without reopening source files.
- [x] Create the vector index on `vector` once persisted rows exist so index creation does not depend on unsupported empty-table behavior.
- [x] Fail writes before commit when `embedding_model` or `embedding_dimension` are incompatible with the active store contract.

### 3.4. Phase D - Runtime Operation Boundary

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00068
  phase: Phase D
  language: Rust
```

- [x] Add a `runtime-rag` operation boundary for LanceDB lifecycle work that matches the project runtime operation pattern used by `doc` and `project`.
- [x] Route store initialization and compatibility validation through the standard dispatcher/channel execution path instead of calling lifecycle functions directly from adapters.
- [x] Keep LanceDB-specific request and error mapping owned by `runtime-rag` so higher-level callers depend on operation contracts rather than raw persistence functions.
- [x] Add or update tests that verify the new `runtime-rag` operation executes correctly through the standard dispatcher path.

### 3.5. Phase E - CLI Integration

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00068
  phase: Phase E
  language: Rust
```

- [ ] Update `vector-database` to invoke the RAG-owned database lifecycle operation.
- [ ] Ensure the CLI does not implement separate schema-creation or index-creation logic.
- [ ] Return actionable errors when store initialization or compatibility validation fails.
- [ ] Document the command behavior for creating or updating the local RAG store.

### 3.6. Phase Y - Tooling And CI

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00068
  phase: Phase Y
  language: Rust, YAML, Markdown
```

- [ ] Update the GitHub Actions Rust workflow to provide `protoc` before building crates that compile the LanceDB dependency graph.
- [x] Document the `protoc` build dependency for local development and CI in the affected README files.

### 3.7. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00068
  phase: Phase Z
  language: Markdown, Rust
```

- [ ] Update the affected README files with the Phase 6 LanceDB store contract and CLI ownership boundary.
- [ ] Run the relevant Rust quality gates for `runtime-rag` and `vector-database`.
- [ ] Confirm every acceptance criterion in [[rfc-00038-phase-6-lancedb-integration]] has implementation or test coverage.
