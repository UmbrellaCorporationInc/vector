---
id: spec-00011-rag-plan-implementation
type: spec
code: "00011"
slug: rag-plan-implementation
title: RAG Plan Implementation
description: Defines the phased implementation plan for adding local RAG capabilities to Vector.
category: plan
created: 2026-06-10
updated: 2026-06-10
authors: []
tags:
  - rag
  - implementation
  - local
related:
  - rfc-00038-phase-6-lancedb-integration
  - rfc-00039-phase-7-incremental-indexing
  - rfc-00040-phase-8-hybrid-search
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00011: RAG Plan Implementation"
---

# SPEC 00011: RAG Plan Implementation

## 1. Purpose

This document defines the implementation plan for a local Retrieval-Augmented Generation system in Vector.
It translates the recommendations from [[research-00003-local-rag]] into ordered, atomic phases that can be implemented, reviewed, and tested independently.

## 2. Definition

The implementation must follow a LanceDB-first, local-embedding architecture with Markdown-aware ingestion and model-agnostic context retrieval. LanceDB is the only persistence layer in the first implementation.

### Phase 1: Define RAG Defaults

Define the fixed local RAG defaults. The first implementation must not introduce a configurable source-directory contract because Vector already defines the documentation corpus through workspace and package document folders.

- Workspace corpus root: `doc/`.
- Package corpus roots: `.vector-database/packages/{package}/doc/` for every synchronized package.
- RAG storage root: `.vector-database/rag/`.
- LanceDB storage path: `.vector-database/rag/lancedb/`.
- Embedding model identifier: `BGESmallENV15`.
- Embedding model code: `Xenova/bge-small-en-v1.5`.
- Embedding dimension: `384`.
- Chunk token target: `350`.
- Chunk token maximum: `500`.
- Semantic retrieval limit: `20`.
- Lexical retrieval limit: `20`.
- Final retrieval limit: `8`.

Acceptance criteria:

- Workspace-local indexing always includes `doc/`.
- Package indexing includes only `doc/` folders inside synchronized packages under `.vector-database/packages/`.
- RAG persistence is isolated under `.vector-database/rag/`.
- The baseline embedding model assumes governed documents are written in English.
- Model, chunking, and retrieval parameters can be loaded deterministically from the existing project configuration mechanism.
- Defaults are explicit and covered by tests.

### Phase 2: Implement Markdown File Discovery

Implement file discovery for local Markdown corpora.
See [[rfc-00032-markdown-discovery]] for the proposed runtime crate boundary and dependency-governance decisions for this phase.

- Walk the workspace `doc/` folder.
- Walk each synchronized package `doc/` folder under `.vector-database/packages/{package}/doc/`.
- Keep package documents associated with their package.
- Include `.md` and `.markdown` files.
- Emit stable file records with package, governed document stem, modified time, content hash, and the internal read path needed by the indexer.

Acceptance criteria:

- Discovery respects ignored paths.
- Discovery is deterministic across repeated runs.
- Missing package `doc/` folders are reported as package-structure errors, not as workspace discovery failures.
- Governed document stems follow `<doc-type>-<code>-<slug>`.
- File hashing changes only when file content changes.

### Phase 3: Extract Markdown Metadata

Extract document-level metadata before chunking.
See [[rfc-00033-markdown-extraction]] for the proposed extraction boundary, normalized output shape, and failure handling for this phase.

- Parse YAML, TOML, or JSON frontmatter.
- Parse heading hierarchy.
- Extract outbound Markdown links.
- Preserve governed source identity through package and document stem.
- Keep malformed frontmatter failures isolated to the affected file.

Acceptance criteria:

- Frontmatter extraction is covered with fixtures.
- Heading extraction handles duplicate and nested headings.
- A malformed document produces a clear indexing error without aborting unrelated files.

### Phase 4: Build Heading-Aware Chunking

Implement chunking that respects Markdown structure.
See [[rfc-00034-markdown-chunking]] for the implemented heading-aware chunking contract, output shape, and acceptance criteria for this phase.

- Keep headings with their section content.
- Avoid splitting fenced code blocks.
- Avoid chunks that contain only a heading.
- Split long sections with token-aware limits.
- Add overlap only for sections that exceed the maximum chunk size.
- Store heading path, chunk ordinal, token count, neighboring chunk references, and chunk hash.

Acceptance criteria:

- Chunking is deterministic for the same input.
- Code blocks, lists, and tables remain valid after splitting.
- Chunk identifiers remain stable when unchanged content keeps the same heading path.

### Phase 5: Add Embedding Boundary

Introduce a narrow embedding abstraction and a first local implementation.
See [[rfc-00036-phase-5-embedder]] for the proposed embedder boundary, baseline model, and CPU-first implementation constraints for this phase.

- Define an `Embedder` boundary with model identifier, dimension, and batch embedding operations.
- Implement the first embedder with `fastembed`.
- Store embedding model and dimension with every embedded chunk.
- Use deterministic fake embedders in tests.

Acceptance criteria:

- Embedding generation can run in batches.
- Dimension mismatches fail before data is written.
- Unit tests do not require network access, credentials, or real model downloads.

### Phase 6: Define LanceDB Data Model And Retrieval Store

Create the primary retrieval table in LanceDB. The persisted document identity must be package-aware and stem-based because Vector can resolve governed documents without storing source paths.

See [[rfc-00038-phase-6-lancedb-integration]] for the proposed LanceDB storage schema, index strategy, and retrieval-store contract for this phase.

This phase defines the persisted data model that later retrieval phases depend on, but it does not define the retrieval algorithm itself. Hybrid retrieval is made explicit in `Phase 8`, where semantic vector search and lexical search are executed together. Phase 6 must therefore preserve the fields and indexable text required for both embedding-based similarity and lexical matching.

Required fields:

- `chunk_id`
- `package`
- `document_stem`
- `document_hash`
- `chunk_hash`
- `chunk_ordinal`
- `heading_path`
- `frontmatter`
- `text`
- `token_count`
- `embedding_model`
- `embedding_dimension`
- `vector`

Acceptance criteria:

- Table creation is idempotent.
- Upserts are keyed by stable `chunk_id` derived from package, document stem, chunk ordinal, and chunk hash.
- Raw chunk text is inspectable from the store without reopening source files.
- Metadata filters can be applied by package, document stem, heading, tags, or frontmatter fields.

### Phase 7: Implement Incremental Indexing

Build the indexing pipeline over discovery, parsing, chunking, embedding, and storage.
See [[rfc-00039-phase-7-incremental-indexing]] for the proposed operation boundary, two-level hash-based change detection, stale chunk removal contract, and failure isolation model for this phase.

- Skip unchanged documents by content hash.
- Re-index changed documents.
- Remove stale chunks for deleted or changed documents by package and document stem.
- Avoid re-embedding unchanged chunks by chunk hash.
- Record indexing failures with enough context to debug the source file.

Acceptance criteria:

- Re-running the indexer with no file changes performs no unnecessary writes.
- Changed chunks are re-embedded without reprocessing unrelated files.
- Deleted files remove their old chunks from retrieval results.

### Phase 8: Implement Hybrid Retrieval

Implement retrieval that combines semantic and lexical signals.

- Run semantic vector search over embeddings.
- Run lexical search through LanceDB hybrid search first.
- Merge semantic and lexical candidates with weighted scoring or reciprocal rank fusion.
- Deduplicate chunks from the same document section.
- Apply metadata filters.
- Expand adjacent chunks when a selected chunk starts mid-topic.

Acceptance criteria:

- Exact identifiers and filenames can be retrieved lexically.
- Conceptual questions can be retrieved semantically.
- Score fusion is deterministic and covered by tests.
- Retrieval results include package, document stem, heading path, score details, and chunk text.

### Phase 9: Assemble MCP Context Results

Create model-agnostic context results that MCP clients and agents can use as retrieved evidence.

- Include selected chunks.
- Include packages, document stems, and heading paths.
- Include token counts.
- Enforce the final retrieval limit.
- Preserve source attribution for every chunk.
- Return structured context without invoking any model.

Acceptance criteria:

- Context assembly never exceeds the configured final retrieval limit.
- Source attribution survives deduplication and expansion.
- Empty retrieval results return a useful structured response.

### Phase 10: Add MCP Or CLI Query Commands

Expose the RAG workflow through the existing user interface layer.

- Index the workspace `doc/` folder and synchronized package `doc/` folders.
- Query the local index.
- Return retrieved context.
- Show index status and recent indexing failures.

Acceptance criteria:

- Commands return non-zero exit codes on actionable failures.
- Query output can be consumed by a human and by another tool.
- Index status identifies corpus size, chunk count, model, and last run.

### Phase 11: Benchmark And Validate The Baseline

Validate the implementation against realistic Markdown fixtures and local corpora.

- Measure indexing latency.
- Measure query latency.
- Measure embedding model size and memory use.
- Compare retrieval quality for semantic questions, exact identifiers, filenames, and error messages.
- Document packaging issues for LanceDB and local embedding models.

Acceptance criteria:

- Baseline latency and memory numbers are recorded.
- Retrieval quality gaps are converted into follow-up work.
- Packaging blockers are resolved or documented with a fallback recommendation.

## 3. Invariants

- Document, chunk, and embedding records must be reproducible from source content, configuration, and embedding model metadata.
- Embedding model identifier and embedding dimension must be stored with every vector.
- Retrieval must not silently mix vectors from incompatible embedding models or dimensions.
- Chunk text must remain inspectable from local persistence.
- Markdown chunking must preserve source attribution through package, document stem, and heading path.
- Indexing failures in one document must not corrupt indexed data for unrelated documents.
- Vector must return context only; answer generation belongs to the MCP client or consuming agent.
- Tests must use deterministic fake embedders by default.

## 4. Examples

Example implementation order:

```
1. Use `doc/` and `.vector-database/packages/{package}/doc/` as the RAG corpus.
2. Store RAG data under `.vector-database/rag/`.
3. Discover governed Markdown documents and compute content hashes.
4. Extract frontmatter and headings.
5. Produce heading-aware chunks.
6. Generate local embeddings for new chunks.
7. Upsert chunks and vectors into LanceDB.
8. Query with semantic and lexical retrieval.
9. Return cited context through MCP or CLI output.
```

## 5. Open Questions

- Does LanceDB hybrid search provide sufficient lexical quality for identifiers, filenames, and exact error messages, or is Tantivy needed in a later phase?
- What command or API shape should expose RAG queries to Codex, Claude, and other assistants?
- What corpus size should define the first acceptable indexing and query latency benchmark?
