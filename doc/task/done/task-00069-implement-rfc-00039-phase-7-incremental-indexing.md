---
id: task-00069-implement-rfc-00039-phase-7-incremental-indexing
type: task
code: "00069"
slug: implement-rfc-00039-phase-7-incremental-indexing
title: Implement RFC 00039 Phase 7 Incremental Indexing
description: Implement the incremental indexing pipeline, orchestration boundary, change detection, stale row deletion, and failure reporting defined by RFC 00039.
status: done
created: 2026-06-15
updated: 2026-06-15
tags:
  - rag
  - indexing
  - incremental
  - lancedb
related:
  - rfc-00039-phase-7-incremental-indexing
  - spec-00011-rag-plan-implementation
supersedes: []
superseded_by: null
---

# Task 00069: Implement RFC 00039 Phase 7 Incremental Indexing

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: task-00069-implement-rfc-00039-phase-7-incremental-indexing
```

## 1. Prime Directive

> [!Prime Directive]
> Eliminate the current gap between RAG component availability and a usable indexing workflow by introducing an incremental, failure-isolated indexing operation in `runtime-rag` and wiring `vector-database` to invoke the RAG-owned orchestration boundary instead of coordinating the pipeline in the CLI adapter.

## 2. Specs

- **Module:** `runtime-rag`; `vector-database`
- **Dependencies:** existing Phase 2 through Phase 6 contracts; any new persistence or orchestration dependency requires explicit justification before use.
- **Source:** [[rfc-00039-phase-7-incremental-indexing]]

## 3. Checklist

### 3.1. Phase A - Operation Boundary And Contracts

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00069
  phase: Phase A
  language: Rust
```

- [x] Add `RagIndexerOp` as the operation that owns the incremental indexing pass.
- [x] Add `IndexWorkspaceOp` as the orchestrating operation that composes Phase 6 store initialization with Phase 7 incremental indexing.
- [x] Define explicit input and output contracts for both operations, including an `IndexResult` summary that callers can render without inspecting internal storage state.
- [x] Keep CLI code out of orchestration ownership so future callers can reuse the same RAG entry point.

### 3.2. Phase B - Incremental Change Detection

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00069
  phase: Phase B
  language: Rust
```

- [x] Reuse the approved `FileContentHash` and `hash_file_content` contracts for document-level change detection instead of reimplementing hashing.
- [x] Skip unchanged documents before parsing, chunking, and embedding when the persisted `document_hash` matches the current file hash.
- [x] Recompute chunk output only for changed documents and re-embed only chunks whose `chunk_hash` changed.
- [x] Preserve deterministic row identity so repeated runs remain idempotent.

### 3.3. Phase C - Store Reconciliation And Deletion

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00069
  phase: Phase C
  language: Rust
```

- [x] Delete stale rows for a changed document, scoped strictly to `(package, document_stem)`, before inserting replacement rows.
- [x] Delete all rows for source documents removed from the corpus during the next indexing run.
- [x] Prevent cross-document deletion by keeping reconciliation keyed to package and governed document identity.
- [x] Keep deletion and rewrite behavior deterministic so a follow-up run can recover cleanly after an interrupted write.

### 3.4. Phase D - Failure Isolation And Reporting

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00069
  phase: Phase D
  language: Rust
```

- [x] Isolate parsing, chunking, embedding, and persistence failures per document so one malformed file does not abort the corpus run.
- [x] Record package, document stem, and actionable error details for each failed document.
- [x] Return counts for skipped, re-indexed, deleted, and failed documents in `IndexResult`.
- [x] Ensure callers can distinguish a successful partial run from a fully clean run without inspecting logs.

### 3.5. Phase E - CLI Integration And Quality Gates

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00069
  phase: Phase E
  language: Rust, Markdown
```

- [x] Update `vector-database update-database` to call `IndexWorkspaceOp` as its single RAG entry point.
- [x] Exit with a non-zero code when `IndexResult` reports any document failures.
- [x] Add or update tests for unchanged-document skips, changed-chunk re-embedding, stale-row deletion on file change, stale-row deletion on file removal, and per-document failure isolation.
- [x] Use deterministic fake embedders in tests and avoid network-backed model downloads.

### 3.6. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00069
  phase: Phase Z
  language: Rust, Markdown
```

- [x] Verify every acceptance criterion in [[rfc-00039-phase-7-incremental-indexing]] has implementation or test coverage.
- [x] Document any unresolved correctness risk around delete-before-write recovery, partial failure visibility, or hash persistence assumptions.
- [x] Update README files for `runtime-rag` and `vector-database` if command behavior or operational expectations changed.

## 4. Staff Engineer Review

The RFC direction is sound, but the implementation will fail structurally if Phase 7 is treated as a thin adapter patch over Phase 6. The highest-risk flaw is state reconciliation: delete-before-write is operationally simple, but it creates a real temporary data-loss window for a document and must therefore be covered by explicit rerun recovery tests. The main gap is observability. `IndexResult` counts alone are not enough unless the CLI exposes failures clearly and tests prove that partial failure exits non-zero while preserving successful work for unaffected documents. The key tradeoff is clear: keeping orchestration inside `runtime-rag` preserves architecture integrity and reuse, but it forces stronger contract discipline between discovery, chunking, embedding, and LanceDB persistence. If those boundaries blur during implementation, the project will accumulate exactly the adapter leakage this RFC is meant to prevent.
