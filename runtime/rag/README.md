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

## Boundary Rules

This crate owns RAG defaults and orchestration. Markdown-specific discovery
behavior belongs to `runtime-markdown`, and filesystem traversal and hashing
belong to `runtime-io`.

`runtime-rag` may depend on `runtime-markdown`; `runtime-markdown` must not
depend on `runtime-rag`.

## License

MIT
