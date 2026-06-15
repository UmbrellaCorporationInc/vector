---
id: rfc-00038-phase-6-lancedb-integration
type: rfc
code: "00038"
slug: phase-6-lancedb-integration
title: Phase 6 LanceDB Integration
description: Proposes the LanceDB storage schema and indexing contract for RAG chunks with vector and full-text retrieval support.
status: implemented
created: 2026-06-15
updated: 2026-06-15
authors: []
tags:
  - rag
  - lancedb
  - retrieval
  - indexing
related:
  - spec-00011-rag-plan-implementation
  - task-00068-implement-rfc-00038-phase-6-lancedb-integration
supersedes: []
superseded_by: null
aliases:
  - "RFC 00038: Phase 6 LanceDB Integration"
---

# RFC 00038: Phase 6 LanceDB Integration

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00038-phase-6-lancedb-integration`
  document-type: task
  document-name: implement-rfc-00038-phase-6-lancedb-integration
```

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: rfc-00038-phase-6-lancedb-integration
```

## 1. Problem

[[spec-00011-rag-plan-implementation]] defines Phase 6 as the point where Vector commits to a persisted LanceDB data model for RAG chunks. Phases 2 through 5 can already define discovery, extraction, chunking, and embedding contracts, but the retrieval pipeline remains incomplete until those outputs can be written into a stable local store.

The store must satisfy several requirements at the same time:

- It must live under the existing RAG storage root at `.vector-database/rag/lancedb/`.
- It must support semantic similarity search over chunk vectors.
- It must support full-text inverted-index search over persisted chunk text so exact identifiers, filenames, and error strings can be retrieved lexically.
- It must preserve package-aware governed document identity without depending on source file paths at query time.
- It must keep raw chunk text inspectable and metadata-filterable for later retrieval phases.
- It must prevent silent mixing of incompatible embedding models or dimensions.

Without a concrete Phase 6 storage contract, later phases would either hard-code storage assumptions into the indexer and retrieval code or force a migration after implementation has already spread across the pipeline.

## 2. Proposal

Adopt a LanceDB-first retrieval store with one primary chunk table stored at `.vector-database/rag/lancedb/`. The table should be denormalized around retrieval units so each row contains the chunk text, embedding, and enough metadata to support filters and provenance without reopening source files.

The baseline table should include the Phase 6 required fields from [[spec-00011-rag-plan-implementation]]:

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

The storage contract should interpret those fields as follows:

- `chunk_id`: stable string key derived from `package`, `document_stem`, `chunk_ordinal`, and `chunk_hash`.
- `package`: nullable package identity; `null` represents workspace-local documents.
- `document_stem`: governed document stem in `<doc-type>-<code>-<slug>` form.
- `document_hash`: document-level content hash used to skip unchanged documents and remove stale rows for changed documents.
- `chunk_hash`: chunk-level content hash used to avoid unnecessary re-embedding.
- `chunk_ordinal`: deterministic chunk order within the document.
- `heading_path`: ordered heading path persisted in a structured form that can also be rendered for filtering and debugging.
- `frontmatter`: normalized frontmatter object persisted in a queryable representation.
- `text`: raw chunk text used both for inspection and full-text indexing.
- `token_count`: persisted token count from chunking.
- `embedding_model`: model identifier stored with every row.
- `embedding_dimension`: expected vector dimension stored with every row.
- `vector`: fixed-size embedding vector.

After this RFC is accepted, Vector should implement the Phase 6 store with these constraints:

- The `rag` crate owns the database lifecycle operation for creating and updating the LanceDB store.
- The `vector-database` CLI must consume that `rag` operation rather than creating tables or indexes directly.
- Create the LanceDB database only under `.vector-database/rag/lancedb/`.
- Create one primary chunk table for RAG retrieval rows.
- Build a vector index on `vector`.
- Build a full-text inverted index on `text`.
- Keep metadata columns filterable by package, document stem, heading path, tags, and frontmatter-derived fields.
- Reject writes when `embedding_model` or `embedding_dimension` do not match the active index contract.
- Treat `chunk_id` as the upsert identity for stable chunk replacement.

The recommended direction is to keep the Phase 6 schema denormalized and retrieval-oriented rather than introducing separate document, chunk, and lexical tables now. Phase 8 already expects hybrid retrieval to merge semantic and lexical candidates over the same persisted chunk unit. A single retrieval table keeps that join-free and reduces migration pressure before the baseline is proven.

This RFC also assigns ownership clearly: LanceDB persistence is a RAG domain concern, so creation and schema-update behavior must live behind a high-level operation in `rag`. The CLI is an interface layer that triggers the operation and reports outcomes, but it must not become an alternate owner of schema logic.

## 3. Alternatives Considered

- **Use LanceDB for vectors and a separate lexical engine such as Tantivy from Phase 6:** Discarded for the baseline because it adds a second persistence system, dual write paths, synchronization failure modes, and more migration surface before Vector has validated whether LanceDB full-text search is sufficient for the first implementation.
- **Normalize the store into separate document and chunk tables:** Discarded for the baseline because retrieval operates on chunks, and early normalization would increase query complexity and coordination cost without yet proving a real storage bottleneck.
- **Persist source file paths as the canonical identity:** Discarded because Vector already resolves governed documents through package and document stem, and paths are more fragile under repository moves or package synchronization.
- **Store only vectors and hashes, reopening source files at retrieval time:** Discarded because Phase 6 explicitly requires inspectable raw text and later phases need lexical search over persisted content.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| One denormalized LanceDB table keeps semantic and lexical retrieval centered on the same chunk unit. | Denormalization duplicates document metadata across rows and increases storage size. |
| Native LanceDB vector and full-text indexes keep the baseline architecture simpler than a dual-store design. | LanceDB lexical quality may prove weaker than a dedicated search engine for some identifier-heavy queries. |
| Package and document-stem identity keeps retrieval stable across source path changes. | Any missing or malformed governed identity must fail early during indexing rather than being deferred. |
| Persisting model and dimension with every row prevents silent cross-model vector corruption. | The indexer and future query path must validate this metadata consistently, which adds contract strictness. |
| Persisting raw text and frontmatter makes debugging and metadata filtering straightforward. | Queryable frontmatter increases schema-shaping work and may require a narrower normalized representation than arbitrary raw frontmatter. |

## 5. Acceptance Criteria

- [ ] Vector stores the Phase 6 retrieval database under `.vector-database/rag/lancedb/`.
- [ ] Vector creates one primary LanceDB chunk table for RAG retrieval rows.
- [ ] The `rag` crate exposes a high-level operation that creates or updates the LanceDB store for Phase 6.
- [ ] The `vector-database` CLI uses the `rag` operation and does not implement separate schema-creation logic.
- [ ] Table creation is idempotent across repeated runs.
- [ ] Upserts are keyed by stable `chunk_id` derived from package, document stem, chunk ordinal, and chunk hash.
- [ ] The table stores all required Phase 6 fields from [[spec-00011-rag-plan-implementation]].
- [ ] A vector index exists on `vector`.
- [ ] A full-text inverted index exists on `text`.
- [ ] Raw chunk text is inspectable from the store without reopening source files.
- [ ] Metadata filters can be applied by package, document stem, heading path, tags, and selected frontmatter fields.
- [ ] Writes fail before commit when the active embedding model or dimension is incompatible with stored rows.
- [ ] Re-indexing a changed document can replace its prior chunk rows deterministically by package and document stem.
- [ ] Tests cover idempotent table creation, stable `chunk_id` generation, metadata filtering, and both vector and full-text indexing behavior.

## 6. Open Questions

- Should `frontmatter` remain as a structured JSON-like object inside the LanceDB row, or should Phase 6 materialize a constrained subset of frontmatter fields into dedicated columns for more predictable filtering?
- Is LanceDB full-text search good enough for exact-code and filename retrieval in Vector's real corpora, or will Phase 8 need a dedicated lexical engine after benchmarking?
- Should `heading_path` be stored only as structured segments, or also as a flattened display string to simplify debugging and filtering?

## 7. Staff Engineer Review

This RFC is directionally correct only if the team treats LanceDB full-text search as a baseline capability to validate, not as a proven long-term answer. The main gap is benchmark evidence: today there is still no corpus-backed proof that LanceDB lexical retrieval is good enough for identifiers, filenames, and exact error text. The main flaw to avoid is pretending arbitrary `frontmatter` can stay fully queryable without a bounded representation; that assumption usually creates schema drift and brittle filters. The key tradeoff is clear: a single LanceDB retrieval table minimizes implementation and operational complexity now, but it deliberately defers the option of a stronger dedicated lexical engine until Phase 8 or Phase 11 data justifies the migration cost.

The ownership decision is the right one: if the CLI owned database creation, Vector would push a domain invariant into an adapter layer and make future MCP or test entrypoints depend on CLI behavior. The remaining risk is different: the `rag` crate can become too coupled to LanceDB internals if the operation is not kept behind a narrow persistence boundary. That means the decision is correct, but the implementation still needs discipline.
