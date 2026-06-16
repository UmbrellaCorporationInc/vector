# `runtime-rag`

`runtime-rag` owns local retrieval-augmented generation defaults and
orchestration boundaries for Vector.

## Features

- **Phase 1 Defaults**: `RagDefaults::phase_one()` exposes fixed local RAG
  defaults for corpus roots, storage roots, embedding model identity, chunking
  limits, and retrieval limits.
- **Markdown Discovery Orchestration**:
  `RagDefaults::markdown_discovery_request(...)` translates package names into
  explicit `runtime-markdown` discovery roots.
- **Markdown Chunking Contract**: `MarkdownChunkDocument` adapts the normalized
  Phase 3 extraction record and source body text into the Phase 4 chunking
  boundary. `MarkdownChunkRecord` exposes package identity, document stem,
  document hash, chunk identity, heading path, text, token count, and
  neighboring chunk references before the embedding boundary.
- **Extraction-To-Chunking Pipeline**:
  `chunk_markdown_extraction(...)` wires normalized Markdown extraction outcomes
  into chunk batches before embedding without exposing Markdown parser internals
  to later embedding or storage phases.
- **Embedding Boundary**: `Embedder` exposes `model_id`, `dimension`, and
  `embed_batch(...)`. `embed_markdown_chunks(...)` returns embedded chunk
  records that carry the original chunk, embedding vector, `embedding_model`,
  and `embedding_dimension`.
- **Fastembed Baseline**: `FastembedBgeSmallEnV15Embedder` isolates fastembed
  initialization and model execution behind the `Embedder` boundary for the
  `BGESmallENV15` model.
- **LanceDB Store Lifecycle**: `ensure_lancedb_store(...)` owns creation and
  validation of the local Phase 6 LanceDB store under
  `.vector-database/rag/lancedb/`, including the primary table contract and the
  full-text index on persisted chunk text.
- **Phase 7 Incremental Indexing**: `IndexWorkspaceOp` is the single entry point
  for callers that need both store initialization and incremental indexing.
  `RagIndexerOp` owns the incremental pass: hash-based skip, chunk-level
  embedding reuse, stale row deletion, and per-document failure isolation.
  `IndexResult` summarizes skipped, re-indexed, deleted, and failed document
  counts so callers can render a run summary without inspecting internal storage.
- **Phase 8 Hybrid Retrieval**: `HybridSearchOp` owns query normalization,
  query embedding, semantic and lexical branch execution, deterministic
  Reciprocal Rank Fusion (RRF), section-level deduplication, same-section
  adjacent chunk expansion, and the machine-readable retrieval payload consumed
  by CLI adapters and future MCP callers.
- **Phase 9 Canonical Retrieval Context**: `AssembleRetrievalContextOp`
  converts Phase 8 retrieval output into the canonical, model-agnostic
  `RetrievalContext` contract consumed by CLI and MCP adapters. It normalizes
  source attribution, assigns response-local context identifiers, preserves
  package and chunk identity, enforces the final result limit, reports token
  diagnostics, and represents successful empty retrieval as `status: empty`.

## Phase 7 Incremental Indexing

`IndexWorkspaceOp` is the orchestrating operation that callers invoke for a full
incremental indexing run. It composes `InitRagStoreOp` (Phase 6) and
`RagIndexerOp` (Phase 7) by calling each in sequence using a `CapturingSender`.

`RagIndexerOp` owns the incremental indexing pass:

1. **Discovery**: discovers all governed Markdown files under the corpus roots.
2. **Stale deletion**: removes rows for source files absent from the current
   corpus, scoped to `(package, document_stem)`.
3. **Document-level skip**: compares the current `document_hash` against the
   stored value; unchanged documents are skipped entirely.
4. **Chunk-level reuse**: for changed documents, re-embeds only chunks whose
   `chunk_hash` differs from stored embeddings.
5. **Delete-before-write**: deletes existing rows for the document before writing
   replacement rows so a crash leaves an empty state rather than a mixed state.
6. **Failure isolation**: records per-document failures without aborting the run.

`IndexResult` carries `skipped_count`, `reindexed_count`, `deleted_count`, and
a `failures` list with per-document package identity, document stem, and error.
`has_failures()` returns `true` when the failures list is non-empty.

### Known Correctness Risks

Three residual risks are acknowledged but unresolved in Phase 7:

- **Delete-before-write recovery**: a crash between the delete and the
  subsequent write leaves the document with no indexed rows. The recovery path
  is a follow-up `update-database` run, which will re-index the document from
  scratch. No test covers mid-write interruption; recovery correctness relies on
  the invariant that an empty state for a document is indistinguishable from a
  new document on the next run.

- **Partial failure visibility**: `IndexResult` is ephemeral. If a document
  fails on every run, the failure is surfaced only through stderr output during
  `update-database`. There is no persisted run record. Operators must observe
  CLI output to detect persistent failures. Persisted run state is deferred to
  `rfc-00035-incremental-validation-index-phase-2`.

- **Hash algorithm consistency**: `document_hash` is stored as a hex string with
  no accompanying algorithm identifier. If the hashing algorithm changes between
  releases, all stored hashes become stale and a full re-index is required. The
  store does not detect this condition automatically.

## Phase 6 LanceDB Store Contract

Phase 6 persists one primary LanceDB table for retrieval chunks under
`.vector-database/rag/lancedb/`. The storage contract is denormalized around a
single chunk row so semantic, lexical, and metadata-driven retrieval can
operate on the same persisted unit.

The primary row contract stores these fields:

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

`chunk_id` is the stable upsert key. It is derived from package identity,
governed document stem, chunk ordinal, and chunk hash so unchanged chunks keep
their persisted identity across repeated indexing runs.

The Phase 6 store keeps raw chunk text in `text` for inspection and full-text
retrieval, and it keeps package, document stem, heading path, tags, and
selected frontmatter data filterable through persisted metadata columns.

Phase 8 also persists a synthetic `search_text` surface for lexical retrieval.
That field prepends the governed `document_stem`, a synthetic `.md` filename,
and the flattened heading path ahead of the raw chunk text so exact identifier
and filename queries can match even when those strings do not appear verbatim in
the prose body.

### Known Lexical Limitations

The LanceDB lexical branch remains a baseline, not a tuned search engine.
`search_text` closes the exact stem and filename gap, but it does not prove that
LanceDB ranking quality is good enough for identifier-heavy corpora, long error
messages, or code-shaped queries. Phase 8 intentionally keeps deterministic RRF
fusion instead of adding score tuning that could hide those shortcomings.

### Known Retrieval Correctness Risks

Phase 8 closes the retrieval contract gap, but several risks remain explicit:

- **Fusion determinism depends on branch ordering**: the fixed `k = 60` RRF
  constant and stable tie-break ordering are covered by tests, but any future
  LanceDB branch ordering drift could still change rankings if the underlying
  candidate lists move.
- **Section identity is intentionally coarse**: deduplication uses
  `(package, document_stem, heading_path)`, which prevents one heading from
  flooding results but can hide multiple useful chunk hits under a very broad
  section.
- **Expansion is intentionally narrow**: adjacent chunk expansion is restricted
  to direct neighbors in the same section and stops at the final retrieval
  limit. This preserves attribution boundaries, but it may still omit useful
  farther context when a section is long.
- **Lexical quality is still unproven**: exact `document_stem` and synthetic
  filename retrieval are covered, but LanceDB full-text ranking quality for
  realistic identifier-heavy corpora still needs Phase 11 benchmarking.

## Phase 8 Hybrid Retrieval

`HybridSearchOp` is the only supported Phase 8 retrieval boundary. It performs:

1. Query normalization and governed retrieval default resolution.
2. Query embedding through the active embedder contract.
3. Semantic vector search against the LanceDB vector index.
4. Lexical search against the `search_text` full-text surface.
5. Package and document filtering before fusion on both branches.
6. Reciprocal Rank Fusion with a fixed `k = 60` constant.
7. Section-level deduplication using `(package, document_stem, heading_path)`.
8. Same-section neighbor expansion within the final result limit.

Each `HybridSearchResult` includes package identity, document stem, heading
path, chunk identity, chunk ordinal, chunk text, token count, branch ranks,
RRF score, neighbor chunk identifiers, and expansion provenance. Adapters must
reuse this output instead of re-implementing ranking or result shaping.

## Phase 9 Canonical Retrieval Context

`AssembleRetrievalContextOp` is the runtime boundary that turns
`HybridSearchOutput` into a canonical `RetrievalContext`. The operation does
not invoke an LLM, summarize text, rewrite chunk text, or reopen source files.
It only packages the evidence already returned by Phase 8.

The canonical context separates normalized sources from evidence chunks:

- `RetrievalContext` records the query, status, final limit, returned chunk
  count, sources, chunks, and diagnostics.
- `RetrievalContextSource` is keyed by package identity, governed document
  stem, and heading path. Its deterministic `citation_label` includes the
  package name for package-qualified documents.
- `RetrievalContextChunk` preserves package identity, document stem, heading
  path, chunk id, chunk ordinal, chunk text, token count, and whether the chunk
  is `primary` or `expanded`.
- `RetrievalContextDiagnostics` records total returned token count, the final
  retrieval limit, and chunks dropped after limit enforcement.

Successful empty retrieval returns `status: empty` with no sources and no
chunks. This is distinct from operational failures such as missing stores,
incompatible embedding metadata, malformed filters, or query execution errors.

The runtime contract is ready for adapters. Updating
`vector-database rag search <query>` and future MCP retrieval output to emit
this shape is tracked separately by Task 00071.

## Phase 1 Defaults

The first local RAG implementation uses:

- workspace corpus root: `doc/`;
- package corpus roots: `.vector-database/packages/{package}/doc/`;
- RAG storage root: `.vector-database/rag/`;
- LanceDB storage path: `.vector-database/rag/lancedb/`;
- embedding model identifier: `BGESmallENV15`;
- embedding model code: `Xenova/bge-small-en-v1.5`;
- embedding dimension: `384`;
- chunk token target: `350`;
- chunk token maximum: `500`;
- semantic retrieval limit: `20`;
- lexical retrieval limit: `20`;
- final retrieval limit: `8`.

## Chunking Contract

`chunk_markdown_document(...)` receives a `MarkdownChunkDocument` derived from
`runtime-markdown::MarkdownExtractionRecord`, a `MarkdownChunkingConfig`, and a
`MarkdownTokenCounter`. The chunker does not read files or parse frontmatter;
those responsibilities remain in Markdown discovery and extraction.

The current implementation establishes the stable DTO and tokenizer boundary
with a deterministic whitespace token counter, parses heading-aware sections,
and splits oversized sections before embedding. Sections at or below the
configured maximum token count are emitted unchanged. Oversized sections are
split using token-aware checks at preferred Markdown block boundaries:
paragraphs, list items, table rows, fenced code blocks, and blank-line-separated
blocks. Fenced code blocks are never split internally.

When a table must be split, every emitted table fragment repeats the original
table header row and separator row. This keeps each table chunk valid and
self-describing when retrieved independently. Overlap is applied only between
chunks produced from the same oversized section; compact sections do not receive
overlap.

Chunk identifiers are derived from package identity, document stem, heading
slug, zero-based chunk ordinal, and chunk hash. Chunk hashes use normalized
chunk text plus structural metadata, so unrelated document edits outside those
inputs do not churn unchanged chunk identifiers. Adjacent chunks in the same
document are linked with `previous_chunk_id` and `next_chunk_id`.

## Pipeline Boundary

`chunk_markdown_extraction(...)` accepts a `runtime-markdown`
`MarkdownExtractionOutcome`, the corresponding source text, a
`MarkdownChunkingConfig`, and a `MarkdownTokenCounter`. Successful extraction
records become `MarkdownChunkBatch` values containing stable document identity
and ordered `MarkdownChunkRecord` values ready for embedding.

The pipeline returns file-scoped failures instead of aborting unrelated
documents. Malformed extraction output, unsupported Markdown structures
reported by extraction, and unsplittable oversized Markdown blocks are surfaced
as actionable `MarkdownChunkingPipelineError` variants with package identity,
document stem, document hash, and structured details when available.

`embed_markdown_extraction(...)` runs embedding immediately after governed
Markdown chunk generation. It passes the generated chunk text to the embedder as
one document-scoped batch, then returns `EmbeddedMarkdownChunkBatch` values with
the same package, document stem, document hash, and chunk order as the chunking
output.

Embedding and storage code should consume embedded chunk batches and diagnostics
from this boundary. They should not inspect frontmatter parsing, heading
extraction, or Markdown source spans except when reporting diagnostics.

## Embedding Boundary

`Embedder` is the stable backend boundary for local embedding generation. An
implementation must expose:

- `model_id()`, the stable model identifier stored with emitted vectors;
- `dimension()`, the required vector length for every emitted embedding;
- `embed_batch(...)`, a batch operation over chunk text inputs.

`embed_markdown_chunks(...)` validates the returned batch before any downstream
storage phase can write data. It rejects an embedder that returns a different
number of vectors than requested, and it rejects any vector whose length differs
from `Embedder::dimension()`.

Successful embedding produces `EmbeddedMarkdownChunkRecord` values. Each record
preserves the original `MarkdownChunkRecord` and adds:

- `embedding_model`;
- `embedding_dimension`;
- `embedding`.

The embedded batch types are intentionally storage-ready but storage-agnostic.
Later LanceDB phases can persist the model metadata and vector without calling a
concrete embedding backend directly.

## LanceDB Ownership Boundary

`runtime-rag` owns the Phase 6 LanceDB lifecycle and persistence rules. That
includes:

- resolving `.vector-database/rag/lancedb/` from the workspace root;
- creating or opening the primary chunk table;
- validating the active embedding model and dimension against store metadata;
- creating the full-text index on `text`;
- creating the vector index on `vector` after persisted rows exist;
- replacing stale document rows deterministically by package and
  `document_stem`.

Adapters such as `vector-database` must call the high-level `InitRagStoreOp`
operation instead of implementing table, schema, or index creation logic
directly. This keeps LanceDB-specific behavior inside the RAG domain boundary
and prevents CLI code from becoming a second owner of persistence invariants.

## Fastembed Model Metadata

The baseline local embedder is `FastembedBgeSmallEnV15Embedder`.

- model identifier: `BGESmallENV15`;
- model code: `Xenova/bge-small-en-v1.5`;
- embedding dimension: `384`;
- backend crate: `fastembed`.

The fastembed implementation validates the model code and dimension against
Vector's RAG defaults before runtime use. `try_new()` performs fastembed model
initialization and keeps model download, cache setup, and ONNX runtime behavior
isolated from indexing callers. Unit and pipeline tests use deterministic fake
embedders instead of downloading or executing the real model.

## Build Dependencies

`runtime-rag` now depends on `lancedb` for the Phase 6 local retrieval store.
The current LanceDB dependency graph requires the Protocol Buffers compiler
`protoc` at build time through `lance-encoding`.

Local development and CI must therefore provide `protoc` through one of these
paths before running `cargo build` or `cargo test` for crates that compile the
LanceDB dependency graph:

- expose `protoc` on `PATH`;
- or set the `PROTOC` environment variable to the `protoc` executable path.

On Windows, install `protoc` with:

```powershell
winget install protobuf
```

## Boundary Rules

This crate owns RAG defaults and orchestration. Markdown-specific discovery and
extraction behavior belongs to `runtime-markdown`, and filesystem traversal and
hashing belong to `runtime-io`.

`runtime-rag` may depend on `runtime-markdown`; `runtime-markdown` must not
depend on `runtime-rag`.

## License

MIT
