---
id: task-00070-implement-rfc-00040-phase-8-hybrid-search
type: task
code: "00070"
slug: implement-rfc-00040-phase-8-hybrid-search
title: Implement RFC 00040 Phase 8 Hybrid Search
description: Implement the Phase 8 hybrid retrieval operation, fusion rules, deduplication policy, adjacent chunk expansion, and CLI search command defined by RFC 00040.
status: in-progress
created: 2026-06-16
updated: 2026-06-16
tags:
  - rag
  - retrieval
  - hybrid-search
  - lancedb
  - cli
related:
  - rfc-00040-phase-8-hybrid-search
  - spec-00011-rag-plan-implementation
supersedes: []
superseded_by: null
---

# Task 00070: Implement RFC 00040 Phase 8 Hybrid Search

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: task-00070-implement-rfc-00040-phase-8-hybrid-search
```

## 1. Prime Directive

> [!Prime Directive]
> Eliminate the current gap between indexed RAG data and a usable retrieval interface by implementing a `runtime-rag` owned hybrid search operation with deterministic RRF fusion, section-level deduplication, constrained context expansion, and a thin `vector-database rag search` adapter.

## 2. Specs

- **Module:** `runtime-rag`; `vector-database`
- **Dependencies:** existing Phase 6 LanceDB storage contracts; existing Phase 7 indexing output shape; any new retrieval backend or ranking dependency requires explicit justification before use.
- **Source:** [[rfc-00040-phase-8-hybrid-search]]

## 3. Checklist

### 3.1. Phase A - Retrieval Operation Boundary And Contracts

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00070
  phase: Phase A
  language: Rust
```

- [x] Add a `runtime-rag` retrieval operation that owns Phase 8 query execution instead of placing retrieval logic in `vector-database`.
- [x] Follow the existing runtime operation pattern already used by `doc`, `project`, and earlier `rag` phases: own retrieval in `runtime-rag`, execute it through the standard dispatcher path, and keep `vector-database` as a thin adapter only.
- [x] Define explicit input and output contracts for query text, optional package and document filters, result limit, and machine-readable retrieval results.
- [x] Reuse governed RAG configuration defaults for retrieval settings instead of creating CLI-only defaults.
- [x] Keep retrieval contracts deterministic and adapter-safe so later MCP consumers can call the same operation without behavioral drift.

### 3.2. Phase B - Hybrid Search And Fusion

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00070
  phase: Phase B
  language: Rust
```

- [x] Embed the query with the active embedding model and execute semantic search against the LanceDB vector index.
- [x] Execute lexical search against the LanceDB full-text index on chunk text.
- [x] Apply package and document filters before fusion on both candidate branches.
- [x] Merge ranked semantic and lexical candidates with a fixed Reciprocal Rank Fusion constant owned by `runtime-rag`.
- [x] Keep missing branch ranks contributing zero instead of inventing normalized cross-mode score comparisons.

### 3.3. Phase C - Deduplication, Expansion, And Result Shape

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00070
  phase: Phase C
  language: Rust
```

- [x] Deduplicate fused hits by `package`, `document_stem`, and `heading_path` so repeated chunks from one section do not dominate the final ranking.
- [x] Keep the highest-ranked chunk as the primary section hit and preserve chunk-level identifiers for debugging and traceability.
- [x] Implement adjacent chunk expansion only within the same section and only when neighbor chunks exist.
- [x] Mark expanded chunks as contextual additions rather than independent winners.
- [x] Return structured results that include package, document stem, heading path, chunk identifier, text, semantic rank, lexical rank, RRF score, and expansion metadata.

### 3.4. Phase D - CLI Integration And Failure Semantics

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00070
  phase: Phase D
  language: Rust
```

- [x] Add `vector-database rag search <query>` as a thin adapter over the `runtime-rag` retrieval operation.
- [x] Support `--limit`, `--package`, `--document`, and `--json` without duplicating ranking or filtering logic in the CLI layer.
- [x] Print readable default output for live validation and a stable machine-readable payload for automation.
- [x] Exit with a non-zero code on actionable failures such as a missing store, incompatible embedding contract, or query embedding failure.

### 3.5. Phase E - Quality Gates And Retrieval Coverage

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00070
  phase: Phase E
  language: Rust, Markdown
```

- [ ] Add tests for semantic-only hits, lexical-only hits, mixed hits fused by RRF, filter application, section-level deduplication, adjacent chunk expansion, and CLI output behavior.
- [ ] Cover exact identifier and filename retrieval through lexical matching.
- [ ] Cover conceptual query retrieval through semantic matching.
- [ ] Lock the chosen RRF constant behind deterministic tests so future tuning cannot silently change ranking behavior.
- [ ] Document any LanceDB lexical limitations discovered during implementation instead of hiding them behind score tuning.

### 3.6. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00070
  phase: Phase Z
  language: Rust, Markdown
```

- [ ] Verify every acceptance criterion in [[rfc-00040-phase-8-hybrid-search]] has implementation or test coverage.
- [ ] Record any unresolved correctness risk around fusion determinism, section identity, expansion limits, or LanceDB lexical quality.
- [ ] Update README files for `runtime-rag` and `vector-database` if retrieval workflows or command behavior changed.

## 4. Staff Engineer Review

The correct baseline is to keep Phase 8 boring in architecture and strict in contracts. The biggest flaw to avoid is letting `vector-database` absorb retrieval policy because that would fork behavior the moment another caller needs the same search semantics. The main gap is benchmark evidence: RRF is a sound baseline, but it does not prove LanceDB lexical quality is sufficient for identifier-heavy corpora. The key tradeoff is intentional. This task should optimize for deterministic, testable retrieval behavior first and only then expose where ranking quality is weak enough to justify a later engine or scoring change.
