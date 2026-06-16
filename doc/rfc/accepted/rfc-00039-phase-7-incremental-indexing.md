---
id: rfc-00039-phase-7-incremental-indexing
type: rfc
code: "00039"
slug: phase-7-incremental-indexing
title: Phase 7 Incremental Indexing
description: Proposes the incremental indexing pipeline that wires together discovery, parsing, chunking, embedding, and LanceDB storage with hash-based change detection.
status: accepted
created: 2026-06-15
updated: 2026-06-15
authors: []
tags:
  - rag
  - indexing
  - incremental
  - pipeline
related:
  - spec-00011-rag-plan-implementation
  - rfc-00038-phase-6-lancedb-integration
  - task-00069-implement-rfc-00039-phase-7-incremental-indexing
supersedes: []
superseded_by: null
aliases:
  - "RFC 00039: Phase 7 Incremental Indexing"
---

# RFC 00039: Phase 7 Incremental Indexing

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00039-phase-7-incremental-indexing`
  document-type: task
  document-name: implement-rfc-00039-phase-7-incremental-indexing
```

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: rfc-00039-phase-7-incremental-indexing
```

## 1. Problem

[[spec-00011-rag-plan-implementation]] defines Phase 7 as the point where Vector wires together all prior RAG pipeline phases into a working incremental indexer. Phases 2 through 6 have established the individual contracts: file discovery, metadata extraction, heading-aware chunking, local embedding, and the LanceDB retrieval store introduced in [[rfc-00038-phase-6-lancedb-integration]]. However, no component yet orchestrates those phases into a single coherent run.

The following gaps remain unresolved after Phase 6:

- There is no driver that walks the corpus and coordinates discovery, parsing, chunking, embedding, and storage in one execution.
- There is no change-detection mechanism. Every run re-processes every file regardless of whether the content changed, making repeated indexing prohibitively expensive on a growing corpus.
- There is no mechanism to remove stale chunks from the LanceDB store when source documents are deleted or significantly restructured.
- There is no per-document failure isolation. A single malformed file can currently abort the rest of the pipeline.
- The `vector-database` CLI `update-database` command does not yet trigger indexing at all; it only initializes the LanceDB store.

Without Phase 7, the LanceDB store from Phase 6 remains empty and the RAG pipeline produces no retrievable results.

## 2. Proposal

Introduce a dedicated `RagIndexer` operation in the `rag` crate that owns the incremental indexing pipeline. This operation is separate from the Phase 6 store-management operation but composed with it inside the `rag` crate so that the `vector-database` CLI calls a single entry point.

### 2.1 Operation Boundary

Phase 7 introduces two new operations in the `rag` crate, following the same `FlowOperation` pattern used by `InitRagStoreOp` (Phase 6), `CreateDocOp` (doc), and `CreateProjectOp` (project):

- **`RagIndexerOp`** — owns the incremental indexing pass: change detection, stale chunk removal, embedding, and upsert. Input: `RagIndexerInput { root_dir, config }`. Output: `IndexResult`.
- **`IndexWorkspaceOp`** — the orchestrating operation. Composes `InitRagStoreOp` (Phase 6) and `RagIndexerOp` (Phase 7) by calling each in sequence using a `CapturingSender`, then forwards the `IndexResult`. Input: `IndexWorkspaceInput { root_dir, config }`. Output: `IndexWorkspaceOutput { result: IndexResult }`.

The CLI `update-database` command calls only `IndexWorkspaceOp::new().run(input, &mut sender)`. It does not instantiate `InitRagStoreOp` or `RagIndexerOp` directly.

This follows the ownership rule established in [[rfc-00038-phase-6-lancedb-integration]]: the CLI is an interface layer that triggers a domain operation, not an orchestrator of schema or storage concerns. The orchestrating operation owns the composition so that any future caller — CLI, MCP tool, or test harness — gets the same sequencing guarantee.

### 2.2 Incremental Change Detection

The indexer applies two-level hash-based change detection. Both levels use BLAKE3, the algorithm already approved in [[project-0003-rust-dependencies]] and established by prior phases:

**Document level:** `document_hash` is a BLAKE3 digest over raw file bytes, produced by `hash_file_content` from the `runtime-io` crate, which returns a `FileContentHash`. This function and type are the single approved hashing primitive in the project; `RagIndexerOp` must consume `FileContentHash` directly and must not reimplement file hashing. The hash input is file bytes only; paths, modified times, package identity, and Markdown metadata are excluded. This contract is established by the Phase 2 discovery layer (see [[rfc-00032-markdown-discovery]]). The indexer compares the current `document_hash` against the value stored in the LanceDB row for that document stem. If they match, the entire document is skipped: no re-parsing, no re-chunking, no re-embedding.

**Chunk level:** `chunk_hash` is a BLAKE3 digest over the normalized chunk text and structural metadata, produced by the Phase 4 chunker (see [[rfc-00034-markdown-chunking]]). For documents whose `document_hash` has changed, the indexer parses and chunks the new content. For each resulting chunk, it compares the new `chunk_hash` against the stored value. Chunks whose hash is unchanged are not re-embedded; only genuinely new or modified chunks trigger embedding calls.

This two-level strategy means:

- No unnecessary writes on repeated runs when the corpus is unchanged.
- Only modified chunks consume embedding compute.
- Documents with changed content but mostly stable sections re-embed only the altered chunks.

### 2.3 Stale Chunk Removal

When a document is re-indexed, the indexer first deletes all stored rows for that `(package, document_stem)` pair that carry a different `document_hash` than the new content hash. This avoids accumulating orphaned chunks from prior document versions.

When a source file is removed from the corpus, the discovery pass detects its absence and the indexer deletes all stored rows for that `(package, document_stem)` pair entirely.

Deletion must be scoped to `(package, document_stem)` to avoid touching rows from other documents. Deletion happens before new rows are written so that a crash between delete and write leaves the store in an empty state for that document rather than in a mixed state.

### 2.4 Failure Isolation

The indexer processes each document inside an isolated error boundary. A failure during parsing, chunking, embedding, or writing for one document is recorded with enough context to identify the source file (package, document stem, and error message), but it does not abort the indexing run for other documents.

The `IndexResult` returned by `RagIndexerOp` and forwarded by `IndexWorkspaceOp` includes:

- Count of documents skipped (unchanged).
- Count of documents re-indexed successfully.
- Count of documents deleted from the store.
- List of per-document failures with source identity and error.

The CLI renders this summary and exits with a non-zero code if any document failures were recorded.

### 2.5 Integration with `update-database`

After this RFC is accepted:

- The `update-database` CLI command calls `IndexWorkspaceOp::new().run(input, &mut sender)`.
- `IndexWorkspaceOp` ensures the LanceDB store is initialized (via `InitRagStoreOp`) then runs the incremental pass (via `RagIndexerOp`).
- No indexing or store-initialization logic lives in the CLI adapter.

The same `IndexWorkspaceOp` can later be invoked from an MCP tool without duplicating orchestration.

## 3. Alternatives Considered

- **Extend the Phase 6 store operation to also run indexing:** Discarded because the Phase 6 operation is a schema-management and storage-lifecycle concern. Adding full pipeline orchestration (discovery, parsing, chunking, embedding) to it would violate single responsibility and make the store operation untestable in isolation. Keeping the two operations separate and composing them in `rag` achieves the same CLI simplicity without conflating their concerns.

- **Own the indexing orchestration in the CLI `update-database` command:** Discarded because [[rfc-00038-phase-6-lancedb-integration]] already established that the CLI is an interface adapter and must not own domain logic. Indexing orchestration is a domain concern in `rag`, not a CLI concern. A second CLI entry point (e.g., a future `rag query` MCP tool) would need the same orchestration, confirming that it belongs in `rag`.

- **Run a full re-index on every `update-database` invocation:** Discarded because re-embedding the entire corpus on every run is prohibitively expensive as the document set grows. The two-level hash strategy makes repeated runs cheap and removes the main reason to avoid frequent indexing.

- **Implement stale chunk removal lazily at query time:** Discarded because stale chunks from deleted or restructured documents would pollute retrieval results until they happen to be filtered. Eager deletion during indexing keeps the store consistent with the source corpus.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Separate `RagIndexer` keeps store-management and pipeline orchestration independently testable. | Two internal components require clear interface boundaries to avoid coupling back to LanceDB internals. |
| Two-level hash detection avoids re-embedding unchanged chunks, which is the dominant cost in a local RAG run. | Hash state must be persisted and remain consistent; a corrupted or missing hash forces a full re-index for the affected document. |
| Composing initialization and indexing in `rag` keeps the CLI adapter thin and reusable from other entry points. | The `rag` crate becomes the single coordination point; a bug in composition logic affects all callers. |
| Per-document failure isolation prevents one bad file from blocking the corpus. | Partial failure is silent unless the caller inspects `IndexResult`; the CLI must surface failures explicitly. |
| Eager stale chunk deletion keeps the store consistent with the corpus after every run. | Delete-before-write creates a window where a document has no indexed rows; a crash in that window requires re-indexing to recover. |

## 5. Acceptance Criteria

- [ ] The `rag` crate exposes `IndexWorkspaceOp` as an orchestrating operation that composes `InitRagStoreOp` (Phase 6) and `RagIndexerOp` (Phase 7) using a `CapturingSender`.
- [ ] `index_workspace` is idempotent: running it twice with no file changes produces no new writes.
- [ ] Documents whose content hash has not changed are skipped entirely without re-parsing, re-chunking, or re-embedding.
- [ ] Documents whose content hash has changed are re-indexed; only chunks with a new `chunk_hash` trigger embedding calls.
- [ ] Changed documents have their prior stale rows removed from the store before new rows are written, scoped to `(package, document_stem)`.
- [ ] Deleted source files have all their rows removed from the store during the next indexing run.
- [ ] A per-document indexing failure is recorded in `IndexResult` without aborting the rest of the run.
- [ ] `IndexResult` includes counts of skipped, re-indexed, deleted, and failed documents.
- [ ] The `update-database` CLI command calls `IndexWorkspaceOp` and reports the `IndexResult` summary.
- [ ] The CLI exits with a non-zero code when `IndexResult` contains any per-document failures.
- [ ] No indexing or schema logic resides in the CLI adapter layer.
- [ ] Unit tests cover: skip-on-unchanged hash, re-embed on changed chunk hash, stale row deletion on document change, stale row deletion on document removal, and per-document failure isolation.
- [ ] Tests use deterministic fake embedders and do not require network access or real model downloads.

## 6. Open Questions

- Should `rag::index_workspace` also accept a list of explicit file paths for partial re-indexing, or is full-corpus incremental sufficient for the Phase 7 baseline?
- How should the indexer handle a document whose `document_hash` changed but whose re-parse or re-chunking fails? Delete the old rows immediately (safe but produces a brief gap) or keep old rows until the new ones are committed (safe on crash but risks serving stale content)?
- Persisting `IndexResult` to a local file and exposing an `index-status` command is out of scope for Phase 7. This is deferred to [[rfc-00035-incremental-validation-index-phase-2]] as part of the broader indexing infrastructure and run-result tracking work.
- Watch-mode (re-index on file-change events) is explicitly out of scope for Phase 7 and is not planned for a subsequent phase at this time.
