---
id: research-00003-local-rag
type: research
code: "00003"
slug: local-rag
title: Local Rag
description: Research on Rust tools and embedded components for building a local RAG system over Markdown files.
category: rag
created: 2026-06-10
updated: 2026-06-10
tags:
  - rag
  - rust
  - markdown
  - embedded
related: []
---

# Local RAG

## Context

The goal is to build a Retrieval-Augmented Generation system in Rust for local Markdown files. The system should be self-contained for ingestion, parsing, chunking, embedding generation, metadata storage, and vector search. Answer generation can be delegated to external coding assistants or language model providers such as Codex and Claude.

This research focuses on a pragmatic embedded architecture. "Embedded" means the application can run as a local binary or service with its persistence and retrieval stack packaged inside the process or deployed beside it without a separately managed database cluster.

## Scope

The system should support:

- Reading Markdown files from one or more local directories.
- Extracting document metadata from frontmatter, headings, links, and file paths.
- Splitting Markdown into retrieval-friendly chunks.
- Generating embeddings locally.
- Persisting documents, chunks, embeddings, and index metadata in an embedded database.
- Running hybrid retrieval across semantic vectors and lexical search.
- Supplying retrieved Markdown context to Codex, Claude, or another configured generation provider.

Out of scope for the first version:

- Multi-user authorization.
- Hosted vector databases.
- Distributed indexing.
- Real-time collaborative editing.
- Non-Markdown document formats.

## Candidate Rust Tools

### Markdown parsing

- `pulldown-cmark`: mature CommonMark parser with event-based processing. Good default for robust Markdown parsing.
- `markdown`: useful when an abstract syntax tree is preferred over a streaming event model.
- `gray_matter`: frontmatter extraction for YAML, TOML, and JSON metadata.
- `ignore`: directory traversal that respects `.gitignore`-style rules.
- `notify`: file watching for incremental re-indexing.

Recommendation: use `ignore` for traversal, `gray_matter` for frontmatter, and `pulldown-cmark` for Markdown structure.

### Text processing and chunking

- `regex`: simple rule-based splitting and cleanup.
- `unicode-segmentation`: safer boundary handling for user-facing text.
- `text-splitter`: token-aware or semantic-ish splitting when chunk size needs to align with model context limits.
- `tokenizers`: token counting and token-aware chunk sizing when using Hugging Face compatible models.

Recommendation: start with heading-aware chunking and token limits. Preserve heading hierarchy in chunk metadata because Markdown structure is often more valuable than raw proximity.

### Embedding generation

- `candle`: Rust-native machine learning framework from Hugging Face. Suitable for local embedding models without a Python runtime.
- `ort`: ONNX Runtime bindings for Rust. Good when using ONNX-exported embedding models.
- `fastembed`: high-level Rust embedding library designed for local embedding workflows.
- `llama-cpp-2` or `llama_cpp_rs`: useful if embeddings are generated through GGUF models supported by llama.cpp.

Recommendation: evaluate `fastembed` first for implementation speed. Use `candle` or `ort` when lower-level model control, model compatibility, or binary packaging constraints require it.

### Generation provider integration

- `async-openai`: useful for OpenAI-compatible provider APIs when the retrieval pipeline sends context to a remote model.
- Anthropic SDKs or HTTP clients: useful for Claude integration when there is no preferred Rust-native SDK in the project.
- Internal command adapters: useful when Codex or Claude are invoked through a CLI, agent runtime, or editor integration instead of direct HTTP.

Recommendation: keep generation behind a provider boundary so retrieval remains independent from Codex, Claude, or any future model backend. The RAG system should produce structured context packages; the provider layer should handle prompt construction, token limits, request execution, and source attribution.

## Embedded Database Options

### LanceDB

LanceDB is the strongest default candidate for the retrieval store because it is built for local vector search while also storing chunk text and metadata. It can run embedded against a local filesystem path and has a Rust SDK.

Useful Rust crates:

- `lancedb`: Rust SDK for embedded LanceDB usage.
- `arrow-array` and `arrow-schema`: table schema and record batch construction used by the Rust API.
- `datafusion`: useful indirectly because LanceDB builds on Arrow/DataFusion concepts and supports SQL-style querying.

Relevant LanceDB capabilities:

- Vector similarity search over embedding columns.
- Storage for chunk text, source metadata, and vectors in the same table.
- Metadata filtering for narrowing retrieval by path, tags, headings, or document attributes.
- Full-text and hybrid search support for combining semantic and lexical retrieval.
- Local embedded deployment without running a separate vector database service.

Weakness: the Rust API requires working with Arrow schemas and record batches, which is more complex than inserting rows into SQLite. Operational maturity, index behavior, and full-text search quality should be tested against the expected Markdown corpus.

Recommendation: use LanceDB as the primary embedded retrieval store for the first RAG implementation. It aligns directly with the need to store chunks, embeddings, metadata, and retrieval indexes locally.

### SQLite

SQLite remains a strong option for relational metadata, document state, application configuration, and audit data.

Useful Rust crates:

- `rusqlite`: synchronous SQLite API with broad extension support.
- `sqlx`: async SQL interface with compile-time checked queries.
- `diesel`: typed ORM if the project already favors Diesel-style modeling.

Relevant SQLite capabilities:

- FTS5 for lexical full-text search.
- JSON columns for flexible metadata.
- Transactional updates for incremental indexing.
- Single-file storage that is easy to back up and inspect.

Weakness: vector search is not built into stock SQLite. It requires an extension or an adjacent vector index.

Recommendation: use SQLite as an optional control-plane database if the application needs relational state outside LanceDB, or as a fallback if LanceDB packaging or API complexity becomes a blocker.

### SQLite vector extensions

Options include:

- `sqlite-vec`: vector search extension designed for SQLite.
- `sqlite-vss`: SQLite extension backed by FAISS.

Strength: keeps metadata, text, and vector search close to a single embedded database.

Weakness: extension packaging can complicate cross-platform distribution. Query performance and operational maturity must be tested with the expected corpus size.

### Tantivy

Tantivy is a Rust-native full-text search engine.

Strengths:

- Excellent lexical retrieval.
- Rust-native integration.
- Better search ranking controls than basic SQLite FTS.

Weaknesses:

- It is not a relational metadata store.
- It does not replace vector search.
- It introduces a second persistence/indexing component.

Recommendation: use Tantivy when lexical retrieval quality becomes important enough to justify a dedicated index. For a first embedded version, start with LanceDB hybrid search before adding a separate Tantivy index.

### Sled

Sled is an embedded key-value store.

Strengths:

- Simple embedded persistence.
- Good for append-oriented local state.

Weaknesses:

- No native SQL, joins, or full-text search.
- Additional work is required for metadata querying and migrations.

Recommendation: avoid Sled as the primary database for RAG metadata unless the data model is intentionally key-value oriented.

### RocksDB

RocksDB is an embedded LSM key-value database.

Strengths:

- High write throughput.
- Good for large local indexes with careful tuning.

Weaknesses:

- Operationally heavier than SQLite.
- Query modeling, migrations, and inspection are more complex.

Recommendation: reserve RocksDB for larger indexing workloads where SQLite write behavior is insufficient.

### Redb

Redb is an embedded Rust database.

Strengths:

- Pure Rust.
- Simple local deployment.

Weaknesses:

- Smaller ecosystem than SQLite.
- No built-in full-text or vector retrieval story.

Recommendation: promising for pure-Rust local persistence, but LanceDB is more pragmatic for a vector-first RAG system today.

## Recommended Baseline Architecture

Use a hybrid local stack:

- LanceDB for chunks, embeddings, metadata filters, vector search, and hybrid retrieval.
- Optional SQLite for application configuration, indexing audit state, or relational metadata that does not belong in the retrieval table.
- `ignore`, `gray_matter`, and `pulldown-cmark` for Markdown ingestion.
- `fastembed` for local embeddings in the first implementation.
- A generation provider boundary for sending retrieved context to Codex, Claude, or another configured assistant.

This architecture keeps retrieval vector-first while preserving a path to SQLite where relational state is genuinely useful.

## Data Model

Suggested retrieval table in LanceDB:

- `chunk_id`: stable identifier derived from document path, heading path, ordinal, and content hash.
- `document_path`: source Markdown file path.
- `document_hash`: hash of the full source file.
- `chunk_hash`: hash of the chunk text and structural metadata.
- `chunk_ordinal`: chunk order within the document.
- `heading_path`: Markdown heading hierarchy for source attribution.
- `frontmatter`: structured frontmatter metadata when supported by the chosen schema.
- `text`: retrievable chunk text.
- `token_count`: estimated token count for prompt budgeting.
- `embedding_model`: model used to generate the vector.
- `embedding_dimension`: vector dimension.
- `vector`: embedding vector.

Optional SQLite tables:

- `documents`: one row per Markdown file when relational document state is needed outside LanceDB.
- `index_runs`: audit table for indexing attempts, failures, durations, and model versions.
- `settings`: persisted source directories, model ids, and retrieval parameters.

Important constraints:

- Store the embedding model name and dimension with every embedding.
- Use content hashes to avoid unnecessary re-embedding.
- Treat chunk ids as stable only when the chunk content and heading path are unchanged.
- Keep raw chunk text in the database so retrieval can be inspected without reopening source files.

## Markdown Ingestion Pipeline

1. Walk configured directories with ignore rules.
2. Filter for `.md` and `.markdown` files.
3. Read file content and compute a content hash.
4. Skip files whose hash is already indexed.
5. Extract frontmatter.
6. Parse Markdown headings and body content.
7. Build heading-aware chunks.
8. Normalize whitespace while preserving code blocks and lists.
9. Generate embeddings for new or changed chunks.
10. Upsert chunk, embedding, and metadata records into LanceDB.
11. Write optional indexing audit state into SQLite if a separate control-plane database is used.

Chunking should prefer semantic boundaries:

- Keep headings with their section content.
- Avoid splitting code blocks.
- Avoid chunks that contain only a heading.
- Add overlap only when sections are long enough to justify it.
- Store neighboring chunk references for optional context expansion at retrieval time.

## Retrieval Strategy

Use hybrid retrieval:

- Semantic search over embeddings for conceptual matches.
- Lexical search through LanceDB hybrid search, SQLite FTS5, or Tantivy for exact terms, identifiers, filenames, and error messages.
- Metadata filters for path, tags, frontmatter fields, and modified time.
- Reciprocal rank fusion or weighted score merging to combine semantic and lexical results.

After initial retrieval:

- Deduplicate chunks from the same document section.
- Expand with adjacent chunks when the selected chunk starts mid-topic.
- Re-rank with a cross-encoder only if local latency and model packaging allow it.
- Build a context window with explicit source paths and heading paths.

## Rust Implementation Shape

Use clear boundaries:

- `ingest`: file discovery, hashing, Markdown parsing.
- `chunking`: section extraction and chunk sizing.
- `embedding`: embedding model abstraction and batch generation.
- `storage`: LanceDB table management, optional SQLite schema, migrations, and transactions.
- `retrieval`: semantic search, lexical search, score fusion, and context assembly.
- `generation`: context packaging, prompt construction, and provider integration.
- `cli` or `server`: user interface, configuration, and commands.

Core traits:

```rust
trait Embedder {
    fn model_id(&self) -> &str;
    fn dimension(&self) -> usize;
    fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>>;
}

trait Generator {
    fn generate(&self, request: GenerationRequest) -> anyhow::Result<GenerationResponse>;
}

trait VectorIndex {
    fn upsert(&self, chunk_id: &str, vector: &[f32]) -> anyhow::Result<()>;
    fn search(&self, vector: &[f32], limit: usize) -> anyhow::Result<Vec<VectorHit>>;
}
```

Keep these abstractions narrow. The first version should optimize for correctness, inspectability, and replaceable components rather than a broad plugin system.

## Configuration

Minimum configuration:

- Source directories.
- Include and exclude glob patterns.
- Embedding model id.
- Embedding dimension.
- Chunk token target and maximum.
- Retrieval limits for semantic, lexical, and final merged results.
- LanceDB path.
- Optional SQLite path for control-plane state.
- Generation provider, model id, and token budget.

The configuration should be persisted with indexing audit state so retrieval behavior can be reproduced.

## Testing Strategy

Use focused tests around deterministic components:

- Markdown frontmatter extraction.
- Heading-aware chunking.
- Content hashing and skip logic.
- LanceDB schema creation and upsert behavior.
- Schema migrations.
- FTS indexing.
- Score fusion.
- Context assembly with source attribution.

Use fixture-based integration tests with a small Markdown corpus that includes:

- Nested headings.
- Code blocks.
- Lists and tables.
- Duplicate headings.
- Frontmatter.
- Long sections requiring splits.
- Files with renamed paths but identical content.

Embedding and generation-provider tests should use deterministic fake implementations by default. Real provider tests should be opt-in because they require credentials, network access, and provider-specific quotas.

## Gaps

- Exact crate maturity and current maintenance status must be verified before implementation.
- LanceDB Rust SDK behavior, indexing configuration, and packaging must be tested on the target operating systems.
- SQLite vector extension packaging must be tested if the fallback path is used.
- Embedding model size, quality, and latency need benchmarking with the expected Markdown corpus.
- Generation quality depends on the selected provider, model, prompt format, and retrieved context quality.
- Multilingual Markdown content may require different embedding models or tokenization rules.

## Flaws And Risks

- A purely embedded system simplifies deployment but can make large corpora slower to index and search.
- LanceDB introduces Arrow schema and record batch complexity in Rust.
- SQLite plus vector extensions may be harder to package than a simple SQLite-only application if the fallback path is selected.
- Local embeddings can create large binary, model, and memory requirements.
- External generation providers introduce network, credential, privacy, quota, and latency constraints.
- Markdown chunking can lose important context if code blocks, headings, and lists are split poorly.
- Hybrid retrieval improves recall but adds ranking complexity that must be tested against real questions.

## Tradeoffs

LanceDB-first design:

- Gains a retrieval-native embedded store for chunks, vectors, metadata filtering, and hybrid search.
- Sacrifices some simplicity in the Rust write path because schemas and record batches are more complex than SQLite row inserts.

SQLite-first design:

- Gains simple persistence, inspection, transactions, and FTS5.
- Sacrifices native vector search unless an extension or adjacent index is added.

Fastembed-first embeddings:

- Gains a faster implementation path.
- Sacrifices some low-level model control.

External provider generation:

- Gains access to strong models such as Codex and Claude without packaging local inference.
- Sacrifices fully offline operation and requires provider credentials, token management, and privacy controls.

Provider boundary:

- Gains the ability to switch between Codex, Claude, or future providers.
- Sacrifices some simplicity because prompts, limits, errors, and source attribution must be normalized.

Tantivy lexical search:

- Gains stronger lexical retrieval.
- Sacrifices the simplicity of keeping all searchable state in SQLite.

## Recommendation

Build the first version with LanceDB, `fastembed`, heading-aware Markdown chunking, and a generation provider boundary for Codex and Claude. Use SQLite only for optional control-plane state or as a fallback if LanceDB does not meet packaging, indexing, or API requirements. Keep Tantivy as a second-stage improvement if LanceDB hybrid search or SQLite FTS5 ranking is not good enough.

The most important early decision is not the final model or vector store. It is preserving enough structured Markdown metadata during ingestion so retrieval can explain where each answer came from and the index can be rebuilt without ambiguity.
