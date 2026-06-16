---
id: rfc-00041-phase-9-canonical-result-for-retrieval-operation
type: rfc
code: "00041"
slug: phase-9-canonical-result-for-retrieval-operation
title: Phase 9 Canonical Result For Retrieval Operation
description: Proposes the canonical context result contract produced by the retrieval operation for CLI and MCP consumers.
status: implemented
created: 2026-06-16
updated: 2026-06-16
authors: []
tags:
  - rag
  - retrieval
  - context
  - mcp
  - cli
related:
  - spec-00011-rag-plan-implementation
  - rfc-00040-phase-8-hybrid-search
  - task-00071-update-rag-cli-search-to-emit-retrieval-context
  - task-00072-implement-rfc-00041-phase-9-canonical-result-for-retrieval-operation
supersedes: []
superseded_by: null
aliases:
  - "RFC 00041: Phase 9 Canonical Result For Retrieval Operation"
---

# RFC 00041: Phase 9 Canonical Result For Retrieval Operation

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00041-phase-9-canonical-result-for-retrieval-operation`
  document-type: task
  document-name: implement-rfc-00041-phase-9-canonical-result-for-retrieval-operation
```

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: rfc-00041-phase-9-canonical-result-for-retrieval-operation
```

## 1. Problem

[[spec-00011-rag-plan-implementation]] defines Phase 9 as the point where Vector converts retrieved chunks into model-agnostic context results that MCP clients, CLI users, and agents can consume as evidence. [[rfc-00040-phase-8-hybrid-search]] defines the hybrid retrieval baseline and returns ranked retrieval hits with scoring details, but that output is still too close to the search algorithm to become the stable public context contract.

The current gap has several concrete parts:

- There is no canonical output shape for retrieved context.
- CLI JSON output and MCP output can drift if each adapter formats retrieval hits independently.
- Retrieval scoring fields are useful for debugging, but they should not be the primary contract consumed by agents.
- Empty retrieval results need a structured response instead of an ambiguous empty list.
- Source attribution must survive deduplication and adjacent chunk expansion from Phase 8.
- Token counts and final result limits must be enforced before output reaches adapters.

Without Phase 9, Vector can search the local corpus but cannot reliably hand the result to another tool as cited context.

## 2. Proposal

Introduce a canonical `RetrievalContext` result produced by the RAG runtime after Phase 8 retrieval completes. The context assembly layer receives ranked retrieval hits, applies the final output constraints, and returns a stable, model-agnostic evidence payload. Both CLI JSON output and MCP retrieval tools should delegate to this operation instead of formatting Phase 8 search hits directly.

### 2.1 Operation Boundary

Phase 9 should introduce a context assembly operation in the RAG runtime:

- **`AssembleRetrievalContextOp`**: converts ranked retrieval hits into canonical context output.
- **Input**: query metadata, effective retrieval configuration, and Phase 8 retrieval hits.
- **Output**: `RetrievalContext`.

The operation must not invoke an LLM, summarize text, rewrite chunk content, or reopen source files. It only normalizes, limits, attributes, and packages already retrieved evidence.

This keeps the boundary aligned with [[spec-00011-rag-plan-implementation]]: Vector returns context only, while answer generation belongs to the MCP client or consuming agent.

### 2.2 Canonical Output Shape

The canonical output should separate retrieval metadata from evidence chunks:

```text
RetrievalContext {
  query: String,
  status: RetrievalContextStatus,
  limit: usize,
  returned: usize,
  sources: Vec<RetrievalContextSource>,
  chunks: Vec<RetrievalContextChunk>,
  diagnostics: RetrievalContextDiagnostics,
}
```

`RetrievalContextStatus` should be explicit:

- `has_results`: at least one evidence chunk was returned.
- `empty`: the retrieval operation completed but found no matching evidence.

The canonical output must avoid embedding CLI-only presentation fields or MCP transport fields. Adapters can render or serialize this structure, but they should not change its meaning.

### 2.3 Evidence Chunk Contract

Each `RetrievalContextChunk` should include enough information for citation, display, and downstream filtering:

```text
RetrievalContextChunk {
  context_id: String,
  source_id: String,
  package: Option<String>,
  document_stem: String,
  heading_path: Vec<String>,
  chunk_id: String,
  chunk_ordinal: usize,
  text: String,
  token_count: usize,
  match_reason: RetrievalMatchReason,
}
```

`context_id` is a stable identifier within this response, such as `ctx-1`, `ctx-2`, and so on. It is intentionally response-local because the stable storage identity remains `chunk_id`.

`source_id` links the chunk to a normalized source entry in `sources`. Multiple chunks from the same document section may share the same source entry after adjacent expansion.

`match_reason` should preserve whether the chunk was a primary retrieval winner or an expansion added for continuity:

- `primary`: the chunk survived Phase 8 ranking and deduplication.
- `expanded`: the chunk was added by adjacent chunk expansion.

Detailed Phase 8 score information can remain available in diagnostics for debugging, but consumers should not need scoring internals to cite retrieved evidence.

### 2.4 Source Attribution Contract

Every returned chunk must have a matching `RetrievalContextSource`:

```text
RetrievalContextSource {
  source_id: String,
  package: Option<String>,
  document_stem: String,
  heading_path: Vec<String>,
  citation_label: String,
}
```

The source identity is the tuple already established by Phase 8:

- `package`
- `document_stem`
- `heading_path`

`citation_label` should be deterministic and human-readable. For workspace documents, it can use the governed document stem plus heading path. For package documents, it must include the package name so citations remain unambiguous across synchronized packages.

The context assembler must not depend on filesystem paths for attribution. Governed document stems are the stable citation unit for Vector documents.

### 2.5 Limit And Token Handling

The context assembler must enforce the configured final retrieval limit after deduplication and expansion. The limit applies to returned evidence chunks, not only to primary winners. If Phase 8 produces more primary and expanded chunks than the final limit allows, the assembler keeps the highest-priority chunks in retrieval order and drops the remainder.

The assembler should preserve per-chunk token counts and include an aggregate diagnostic count:

```text
RetrievalContextDiagnostics {
  total_token_count: usize,
  dropped_after_limit: usize,
  retrieval_limit: usize,
}
```

This does not introduce a model context-window budget. It only records the token counts already known from chunking so MCP clients and agents can make their own prompt-budget decisions.

### 2.6 Empty Result Contract

Empty retrieval must return a structured response:

```text
RetrievalContext {
  query: "...",
  status: empty,
  limit: 8,
  returned: 0,
  sources: [],
  chunks: [],
  diagnostics: RetrievalContextDiagnostics { ... },
}
```

The operation should not treat empty results as an error. An empty result is a successful retrieval with no evidence. Transport failures, missing indexes, incompatible embedding metadata, and malformed filters remain errors from earlier retrieval stages.

### 2.7 Adapter Usage

The CLI and MCP layers should share the same canonical result:

- CLI human-readable output renders `RetrievalContext` for terminal inspection.
- CLI `--json` serializes `RetrievalContext` directly or through a thin compatibility wrapper.
- MCP tools return `RetrievalContext` as structured tool output.

The existing `vector-database rag search <query>` command introduced in Phase 8 must be updated to call the Phase 9 context assembly operation before rendering or serializing results. The command may keep its existing query, limit, package, document, and JSON flags, but its output semantics must be based on `RetrievalContext` rather than raw Phase 8 retrieval hits.

Adapters may change presentation, but they must not create alternative source attribution, limit enforcement, or empty-result semantics.

## 3. Alternatives Considered

- **Expose Phase 8 retrieval hits directly to CLI and MCP consumers:** Discarded because search hits contain algorithm-specific scoring details and do not define a stable evidence contract for agents.
- **Let each adapter format results independently:** Discarded because this would create drift between CLI JSON and MCP output, especially around empty results, source attribution, and limit enforcement.
- **Return only plain text context blocks:** Discarded because plain text loses structured source attribution, token counts, package identity, and chunk identity.
- **Include generated summaries in Phase 9:** Discarded because [[spec-00011-rag-plan-implementation]] explicitly states that Vector returns context only and answer generation belongs outside Vector.
- **Use filesystem paths as the primary citation identity:** Discarded because Vector already resolves governed documents by package and document stem, and package-synchronized documents should not leak local storage paths into public output.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| A canonical context result keeps CLI JSON and MCP output consistent. | The shared contract must be versioned carefully once external clients rely on it. |
| Separating evidence chunks from sources makes citations explicit and compact. | Consumers must follow `source_id` references instead of reading all attribution from each chunk alone. |
| Keeping scores in diagnostics avoids coupling agents to ranking internals. | Debugging ranking quality may require an explicit verbose mode or diagnostic field inspection. |
| Structured empty results avoid ambiguity for tools and agents. | Some human CLI output still needs presentation logic to avoid feeling overly mechanical. |
| Enforcing limits in the runtime keeps adapters thin and consistent. | Runtime changes to limit semantics affect all callers at once. |

## 5. Acceptance Criteria

- [ ] The RAG runtime exposes an operation that converts Phase 8 retrieval hits into `RetrievalContext`.
- [ ] The operation returns selected chunks, package identity, document stems, heading paths, chunk identifiers, and token counts.
- [ ] Every returned chunk has source attribution through a normalized source entry.
- [ ] Source attribution survives Phase 8 deduplication and adjacent chunk expansion.
- [ ] The configured final retrieval limit is enforced after expansion and before adapter rendering.
- [ ] Empty retrieval results return a structured `empty` response with no chunks and no sources.
- [ ] The operation does not invoke an LLM, summarize retrieved text, or reopen source files.
- [ ] CLI JSON output uses the canonical context shape or a thin wrapper that preserves the same fields and semantics.
- [ ] The existing `vector-database rag search <query>` command renders and serializes Phase 9 `RetrievalContext` output instead of raw Phase 8 retrieval hits.
- [ ] MCP retrieval output uses the same canonical context shape.
- [ ] Tests cover primary chunks, expanded chunks, repeated sources, final limit truncation, empty results, package-qualified sources, and diagnostic token totals.

## 6. Open Questions

- Should `RetrievalContext` include a contract version field in Phase 9, or should versioning wait until the first MCP tool exposes this shape externally?
- Should diagnostics include full Phase 8 score details by default, or only when a caller explicitly requests debug output?
- Should `citation_label` include heading paths as plain text only, or should it also expose a structured governed-document reference that adapters can render as links later?

## 7. Staff Engineer Review

The right architectural move is to make Phase 9 a runtime contract, not an adapter formatting exercise. The main gap is contract stability: once MCP clients consume this shape, renaming fields becomes expensive. The implementation should therefore keep the first shape small, typed, and boring.

The most important flaw to avoid is leaking retrieval implementation details into the public evidence contract. Scores and ranks matter for diagnostics, but agents need reliable cited text, not ranking internals. The second flaw is treating empty retrieval as an error. Empty evidence is a valid answer from the retrieval layer and should be distinct from operational failure.

The tradeoff is clear: this RFC adds another internal operation instead of letting CLI and MCP format directly from Phase 8 hits. That costs a little code now, but it prevents duplicated semantics at the boundary where correctness matters most. I would keep the contract strict and defer richer diagnostics until real debugging workflows prove they need them.
