---
id: rfc-00040-phase-8-hybrid-search
type: rfc
code: "00040"
slug: phase-8-hybrid-search
title: Phase 8 Hybrid Search
description: Proposes the hybrid retrieval algorithm and CLI query command for Phase 8 of the local RAG plan.
status: implemented
created: 2026-06-16
updated: 2026-06-16
authors: []
tags:
  - rag
  - retrieval
  - hybrid-search
  - cli
related:
  - spec-00011-rag-plan-implementation
  - rfc-00038-phase-6-lancedb-integration
  - rfc-00039-phase-7-incremental-indexing
  - task-00070-implement-rfc-00040-phase-8-hybrid-search
supersedes: []
superseded_by: null
aliases:
  - "RFC 00040: Phase 8 Hybrid Search"
---

# RFC 00040: Phase 8 Hybrid Search

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00040-phase-8-hybrid-search`
  document-type: task
  document-name: implement-rfc-00040-phase-8-hybrid-search
```

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: rfc-00040-phase-8-hybrid-search
```

Implementation task: [[task-00070-implement-rfc-00040-phase-8-hybrid-search]]

## 1. Problem

[[spec-00011-rag-plan-implementation]] defines Phase 8 as the point where Vector turns the indexed LanceDB corpus into a usable retrieval system by combining semantic and lexical search. After Phase 7, Vector can discover documents, chunk them, embed them, and persist them incrementally. What is still missing is the retrieval contract that decides how a query is executed and how results are returned.

The current gap has several concrete parts:

- There is no defined hybrid retrieval algorithm for combining vector similarity and lexical matches.
- There is no deterministic rule for deduplicating overlapping hits that point to the same document section.
- There is no contract for adjacent chunk expansion when the best hit starts in the middle of a topic.
- There is no stable result shape that exposes enough evidence for debugging and downstream MCP consumption.
- There is no CLI query command for validating retrieval quality live against the local corpus.

Without Phase 8, the store from [[rfc-00038-phase-6-lancedb-integration]] and the indexer from [[rfc-00039-phase-7-incremental-indexing]] remain operational but not practically testable as a retrieval system.

## 2. Proposal

Adopt a hybrid retrieval baseline in `runtime-rag` that executes semantic vector search and lexical full-text search over the same LanceDB chunk table, then merges both ranked lists with Reciprocal Rank Fusion (RRF). Expose that operation through a new `vector-database rag search` CLI command for live validation.

### 2.1 Retrieval Flow

The Phase 8 retrieval operation should execute the following sequence:

1. Normalize the input query and resolve retrieval defaults from the governed RAG configuration.
2. Embed the query with the active embedding model.
3. Run semantic vector search against the LanceDB `vector` index.
4. Run lexical search against the LanceDB full-text index on `text`.
5. Apply metadata filters independently to both candidate sets before fusion.
6. Merge ranked candidates with Reciprocal Rank Fusion.
7. Deduplicate hits that resolve to the same `package`, `document_stem`, and `heading_path`.
8. Expand adjacent chunks only when the winning chunk starts mid-topic and neighboring chunks belong to the same document section.
9. Return the top final results in a structured retrieval payload.

This keeps Phase 8 aligned with the Phase 6 denormalized retrieval unit: the persisted chunk row is both the lexical and semantic candidate unit, so fusion does not require cross-table joins or source-file reopening.

### 2.2 Fusion Strategy

Phase 8 should use Reciprocal Rank Fusion as the baseline merge strategy.

For each candidate chunk, compute:

`rrf_score = 1 / (k + semantic_rank) + 1 / (k + lexical_rank)`

Where:

- `semantic_rank` is the 1-based rank position in the semantic result list.
- `lexical_rank` is the 1-based rank position in the lexical result list.
- Missing ranks contribute `0` for that branch.
- `k` is a fixed constant owned by `runtime-rag` and covered by tests.

RRF is the recommended baseline because it avoids pretending LanceDB vector scores and full-text scores are directly comparable. Weighted score merging would require score normalization rules that the current stack has not yet validated. Phase 8 should prefer a ranking method that is deterministic, explainable, and stable before the team invests in tuning.

### 2.3 Candidate Identity And Deduplication

The fusion stage should treat the following tuple as the section-level identity for result deduplication:

- `package`
- `document_stem`
- `heading_path`

If multiple chunks from the same section survive fusion, keep the highest-ranked chunk as the primary hit and use adjacent chunk expansion to recover surrounding context only when needed. This prevents result lists from being dominated by repeated chunks from one long section while preserving section-level provenance.

Chunk-level identifiers such as `chunk_id` remain important for storage and debugging, but they are too granular to serve as the user-facing retrieval identity.

### 2.4 Adjacent Chunk Expansion

Adjacent chunk expansion is allowed only under a narrow rule:

- Expand only when the selected chunk has either a `previous_chunk_id` or `next_chunk_id` inside the same section.
- Expand only within the same `package`, `document_stem`, and `heading_path`.
- Stop expansion when the final retrieval limit would be exceeded.
- Preserve the primary hit score and annotate expanded chunks as contextual additions rather than independent winners.

This rule keeps expansions useful for incomplete mid-section hits without turning the retrieval layer into a broad summarization mechanism.

### 2.5 Retrieval Output Contract

The retrieval operation should return structured results that later Phase 9 context assembly can consume directly.

Each selected result must include:

- `package`
- `document_stem`
- `heading_path`
- `chunk_id`
- `text`
- `semantic_rank`
- `lexical_rank`
- `rrf_score`
- `was_expanded`

The operation may also include optional debug metadata such as `token_count`, `chunk_ordinal`, and neighbor chunk identifiers, but the fields above are the minimum contract for testability and traceability.

### 2.6 CLI Command

Add a new CLI command under the existing RAG namespace:

`vector-database rag search <query>`

The command should:

- Delegate execution to a retrieval operation owned by `runtime-rag`.
- Reuse the same governed RAG defaults used by indexing.
- Print human-readable results by default for live validation.
- Support a machine-readable output mode for future automation.
- Exit with a non-zero code on actionable failures such as a missing store, incompatible embedding contract, or query embedding failure.

The baseline CLI surface should support these arguments:

- `<query>`: required query string.
- `--limit <n>`: optional override for final result count.
- `--package <name>`: optional package filter.
- `--document <stem>`: optional document filter.
- `--json`: optional machine-readable output.

The command belongs under `rag` instead of becoming a top-level `search` command because the current CLI already groups RAG lifecycle operations under that namespace. Moving retrieval outside `rag` would make the command surface less coherent without solving a real problem.

## 3. Alternatives Considered

- **Use weighted score merging between semantic and lexical results:** Discarded for the baseline because it requires stable score normalization across two retrieval modes that have different score semantics and no benchmark-backed calibration yet.
- **Use only semantic search and skip lexical retrieval:** Discarded because [[spec-00011-rag-plan-implementation]] explicitly requires exact identifiers and filenames to remain retrievable, which purely semantic ranking does not guarantee.
- **Use only lexical retrieval through LanceDB full-text search:** Discarded because conceptual queries and paraphrased questions need embedding-based recall.
- **Introduce a dedicated lexical engine such as Tantivy in Phase 8:** Discarded for the baseline because it adds a second retrieval backend, more synchronization cost, and more failure modes before LanceDB lexical quality has been benchmarked against real corpus needs.
- **Expose the command as `vector-database search` instead of `vector-database rag search`:** Discarded because retrieval is a RAG-specific capability in the current CLI and should stay grouped with the existing `rag init` and `rag update-database` commands.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| RRF gives a deterministic hybrid ranking method without pretending semantic and lexical scores are directly comparable. | RRF sacrifices fine-grained tuning that a carefully calibrated weighted merge could eventually provide. |
| One retrieval operation over the existing LanceDB chunk table keeps the baseline architecture simple. | The baseline stays dependent on LanceDB lexical quality, which may later prove insufficient for identifier-heavy corpora. |
| Section-level deduplication reduces noisy repeated hits from long headings. | Section-level deduplication can hide multiple useful chunk-level matches inside the same heading if the heading is very broad. |
| Narrow adjacent chunk expansion improves context continuity without reopening files. | Expansion policy adds result-shaping complexity and must be constrained carefully to avoid crowding out diverse hits. |
| `vector-database rag search` makes live retrieval testing possible from the terminal. | The CLI surface grows again, so the adapter must remain thin and resist accumulating retrieval logic. |

## 5. Acceptance Criteria

- [ ] `runtime-rag` exposes a retrieval operation that runs semantic and lexical search over the Phase 6 LanceDB store.
- [ ] The retrieval operation merges both ranked candidate lists with Reciprocal Rank Fusion.
- [ ] Fusion is deterministic and covered by tests.
- [ ] Exact identifiers and filenames can be retrieved through lexical matching.
- [ ] Conceptual questions can be retrieved through semantic matching.
- [ ] Metadata filters can be applied by package and document stem before fusion.
- [ ] Section-level deduplication uses `package`, `document_stem`, and `heading_path` as the winning identity.
- [ ] Adjacent chunk expansion never crosses section boundaries.
- [ ] Retrieval results include package, document stem, heading path, chunk identifier, text, and score details.
- [ ] The final result count respects the configured retrieval limit or the explicit CLI override.
- [ ] `vector-database rag search <query>` executes the retrieval operation without implementing ranking logic in the CLI adapter.
- [ ] The CLI supports `--limit`, `--package`, `--document`, and `--json`.
- [ ] The CLI exits with a non-zero code when the store is missing, incompatible, or the query operation fails.
- [ ] Tests cover semantic-only hits, lexical-only hits, mixed hits fused by RRF, deduplication, filter application, adjacent chunk expansion, and CLI output behavior.

## 6. Open Questions

- What fixed `k` value should the baseline RRF implementation use, and does that value need to be configurable later or remain a governed constant?
- Is LanceDB lexical retrieval quality sufficient for exact-code, filename, and error-message lookups in realistic Vector corpora, or will a dedicated lexical engine be required after Phase 11 benchmarking?
- Should the first `--json` output shape mirror the future Phase 9 MCP context contract exactly, or should it remain a retrieval-specific debug payload?

## 7. Staff Engineer Review

This RFC makes the right baseline decision by choosing RRF now instead of inventing score normalization without evidence. The main gap is still benchmark data: no one should confuse "deterministic" with "good enough." If LanceDB lexical quality is weak on real identifiers, the ranking strategy will not save the system. The main flaw to avoid in implementation is letting the CLI accumulate retrieval behavior that belongs in `runtime-rag`; once ranking, filtering, or formatting logic leaks into the adapter, every future caller inherits inconsistent behavior.

The strongest tradeoff in this RFC is intentional: it prefers architectural discipline and testability over early ranking sophistication. That is the correct staff-level choice for Phase 8. The cost is that the first retrieval quality ceiling may be lower than a tuned multi-signal system. That is acceptable only if the team treats this RFC as a baseline to measure critically, not as proof that hybrid retrieval is solved.
