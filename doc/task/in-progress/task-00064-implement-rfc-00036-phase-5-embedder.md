---
id: task-00064-implement-rfc-00036-phase-5-embedder
type: task
code: "00064"
slug: implement-rfc-00036-phase-5-embedder
title: Implement RFC 00036 Phase 5 Embedder
description: Implement the local embedding boundary and first fastembed-backed embedder for the RAG pipeline.
status: in-progress
created: 2026-06-10
updated: 2026-06-10
tags:
  - rag
  - embeddings
  - fastembed
related:
  - rfc-00036-phase-5-embedder
  - spec-00011-rag-plan-implementation
supersedes: []
superseded_by: null
---

# Task 00064: Implement RFC 00036 Phase 5 Embedder

## 1. Prime Directive

> [!Prime Directive]
> Eliminate the gap between chunk generation and semantic retrieval by adding a stable local embedding boundary for RAG indexing.

## 2. Specs

- **Module:** `runtime-rag`
- **Dependencies:** `fastembed`
- **Source:** [[rfc-00036-phase-5-embedder]]

## 3. Checklist

### 3.1. Phase A — Embedder Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00064
  phase: Phase A
  language: Rust
```

- [x] Define an `Embedder` boundary that exposes `model_id`, `dimension`, and batch embedding operations.
- [x] Define embedded chunk output that carries chunk content plus `embedding_model` and `embedding_dimension`.
- [x] Add dimension validation that rejects vectors whose length differs from the embedder dimension before downstream writes can occur.
- [x] Cover empty batches, single input chunks, multiple input chunks, and dimension mismatch failures with tests.

### 3.2. Phase B — Fastembed Implementation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00064
  phase: Phase B
  language: Rust
```

- [x] Add a `fastembed`-backed implementation for `BGESmallENV15`.
- [x] Configure the implementation to use model code `Xenova/bge-small-en-v1.5`.
- [x] Treat the expected embedding dimension `384` as part of the implementation contract.
- [x] Keep model download and runtime initialization behavior isolated from the indexing pipeline.

### 3.3. Phase C — Pipeline Integration

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00064
  phase: Phase C
  language: Rust
```

- [x] Integrate embedding after governed Markdown chunk generation in the RAG pipeline.
- [x] Ensure embedding accepts batches of chunk text rather than requiring per-chunk calls.
- [x] Add a deterministic fake embedder for unit tests and pipeline tests.
- [x] Ensure unit and pipeline tests do not require network access, credentials, or real model downloads.

### 3.4. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00064
  phase: Phase Z
  language: Rust
```

- [ ] Update `runtime/rag/README.md` with the embedder boundary and model metadata.
- [ ] Run the relevant Rust quality gates for `runtime-rag`.
- [ ] Confirm every acceptance criterion in [[rfc-00036-phase-5-embedder]] has implementation or test coverage.
