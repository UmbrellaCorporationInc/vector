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

Neighbor population across multiple chunks is implemented in a later RFC 00034
phase without changing the public chunk record shape.

## Boundary Rules

This crate owns RAG defaults and orchestration. Markdown-specific discovery and
extraction behavior belongs to `runtime-markdown`, and filesystem traversal and
hashing belong to `runtime-io`.

`runtime-rag` may depend on `runtime-markdown`; `runtime-markdown` must not
depend on `runtime-rag`.

## License

MIT
