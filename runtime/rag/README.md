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

## Boundary Rules

This crate owns RAG defaults and orchestration. Markdown-specific discovery and
extraction behavior belongs to `runtime-markdown`, and filesystem traversal and
hashing belong to `runtime-io`.

`runtime-rag` may depend on `runtime-markdown`; `runtime-markdown` must not
depend on `runtime-rag`.

## License

MIT
