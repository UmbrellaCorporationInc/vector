---
id: rfc-00036-phase-5-embedder
type: rfc
code: "00036"
slug: phase-5-embedder
title: Phase 5 Embedder
description: Proposes the local embedding boundary and first CPU-efficient fastembed implementation for the RAG pipeline.
status: accepted
created: 2026-06-10
updated: 2026-06-10
authors: []
tags:
  - rag
  - embeddings
  - fastembed
  - local
related:
  - spec-00011-rag-plan-implementation
  - task-00064-implement-rfc-00036-phase-5-embedder
supersedes: []
superseded_by: null
aliases:
  - "RFC 00036: Phase 5 Embedder"
---

# RFC 00036: Phase 5 Embedder

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00036-phase-5-embedder`
  document-type: task
  document-name: implement-rfc-00036-phase-5-embedder
```

## 1. Problem

[[spec-00011-rag-plan-implementation]] defines Phase 5 as the point where Vector adds local embedding generation to the RAG pipeline. Phases 2 through 4 can discover, parse, and chunk governed Markdown documents, but those chunks cannot be persisted for semantic retrieval until Vector has a stable embedding boundary.

The embedding layer needs to satisfy several constraints at once:

- It must run locally without requiring network access during normal indexing.
- It must be efficient enough on CPU to support local developer workstations.
- It must preserve model identity and vector dimension so later LanceDB writes cannot mix incompatible embeddings.
- It must be testable without downloading real models or depending on nondeterministic model output.

## 2. Proposal

Introduce an `Embedder` boundary for Phase 5 of the RAG implementation. The boundary exposes model metadata and batch embedding operations while hiding the concrete embedding backend from the rest of the indexing pipeline.

The first implementation should use `fastembed` with the default embedding model already defined by [[spec-00011-rag-plan-implementation]]:

- Model identifier: `BGESmallENV15`.
- Model code: `Xenova/bge-small-en-v1.5`.
- Embedding dimension: `384`.

This model is a good baseline for the first local implementation because it keeps vectors small, supports English governed documents, and is practical for CPU-first indexing. The implementation should treat the model identifier and expected dimension as part of the embedder contract, not as incidental runtime details.

After this RFC is accepted, Vector should have:

- An `Embedder` abstraction that returns `model_id`, `dimension`, and batch embeddings for chunk text.
- A `fastembed`-backed implementation for `BGESmallENV15`.
- Dimension validation that fails before any embedded chunk can be written downstream.
- Embedded chunk output that carries `embedding_model` and `embedding_dimension`.
- A deterministic fake embedder for unit tests and pipeline tests.

The embedder should accept batches of chunk text rather than individual chunks. Batch size can remain an internal implementation detail for this phase, as long as callers can request embeddings for multiple chunks in one operation.

## 3. Alternatives Considered

- **Call `fastembed` directly from the indexer:** Discarded because it would couple indexing orchestration to one embedding backend and make fake embedders harder to use in tests.
- **Use a larger local embedding model first:** Discarded for the baseline because Phase 1 explicitly chooses `BGESmallENV15` and a `384`-dimension vector, which keeps storage and CPU cost lower for the first implementation.
- **Add remote embedding providers now:** Discarded because the plan calls for local embeddings first, and remote providers would add credentials, network failure modes, and model compatibility concerns before the local pipeline is stable.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| A narrow `Embedder` boundary keeps indexing independent from the concrete embedding backend. | The boundary adds an interface that must be maintained as retrieval needs evolve. |
| `BGESmallENV15` keeps CPU and storage requirements modest for the baseline. | Retrieval quality may be lower than larger embedding models for some technical queries. |
| Storing model and dimension with each embedded chunk prevents silent vector incompatibility. | Every downstream storage and retrieval path must carry and validate this metadata. |
| Deterministic fake embedders make tests fast and offline. | Fake vectors cannot validate real semantic retrieval quality. |

## 5. Acceptance Criteria

- [ ] Vector defines an `Embedder` boundary with model identifier, embedding dimension, and batch embedding operations.
- [ ] Vector provides a `fastembed` implementation for `BGESmallENV15` using `Xenova/bge-small-en-v1.5`.
- [ ] The embedder validates that generated vectors have dimension `384`.
- [ ] Dimension mismatches fail before any downstream storage write is attempted.
- [ ] Embedded chunk records include `embedding_model` and `embedding_dimension`.
- [ ] Unit tests use deterministic fake embedders and do not require network access, credentials, or real model downloads.
- [ ] Batch embedding behavior is covered by tests, including empty batches and multiple input chunks.

## 6. Open Questions

- What default internal batch size should the `fastembed` implementation use for local CPU indexing?
- Should model download or cache preparation be exposed as an explicit command in a later phase, or should the embedder lazily initialize on first use?
- Which benchmark corpus size should Phase 11 use to decide whether `BGESmallENV15` remains the baseline model?
