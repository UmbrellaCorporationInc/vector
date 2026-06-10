---
id: rfc-00034-markdown-chunking
type: rfc
code: "00034"
slug: markdown-chunking
title: Markdown Chunking
description: Proposes the heading-aware Markdown chunking contract for Phase 4 of the local RAG implementation.
status: accepted
created: 2026-06-10
updated: 2026-06-10
authors: []
tags:
  - rag
  - markdown
  - chunking
related: []
supersedes: []
superseded_by: null
aliases:
  - "RFC 00034: Markdown Chunking"
---

# RFC 00034: Markdown Chunking

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00034-markdown-chunking`
  document-type: task
  document-name: implement-rfc-00034-markdown-chunking
```

## 1. Problem

Phase 4 of [[spec-00011-rag-plan-implementation]] requires Markdown chunks that preserve document structure while staying small enough for local embedding and retrieval.

A naive character or line splitter would produce unstable and low-quality retrieval units:

- Headings can be separated from the content they describe.
- Fenced code blocks can be split into invalid Markdown fragments.
- Lists and tables can be broken in ways that remove useful context.
- Stable chunk identifiers become difficult when unrelated edits shift line offsets.
- Adjacent chunks cannot be expanded safely unless the chunker records neighborhood metadata.

The RAG pipeline needs a deterministic chunking contract that can be implemented after metadata extraction and before embedding. This contract must produce chunks that are useful for retrieval, reproducible across repeated indexing runs, and attributable to a governed document section.

## 2. Proposal

Implement a heading-aware Markdown chunker that receives the normalized Markdown extraction output from Phase 3 and emits ordered chunk records for Phase 5 embedding and Phase 6 storage.

The chunker should treat each heading section as the primary chunking unit. A section includes its heading path and all Markdown content until the next heading of the same or higher level. The chunk text should include the relevant heading context so each chunk remains understandable when retrieved without the full document.

The chunker must enforce the defaults defined by [[spec-00011-rag-plan-implementation]]:

- Target chunk size: `350` tokens.
- Maximum chunk size: `500` tokens.
- Overlap: applied only when a section exceeds the maximum chunk size.

Each emitted chunk must contain:

- `chunk_id`: stable identifier derived from package, document stem, heading path, chunk ordinal, and chunk hash.
- `package`: package name for synchronized package documents, or `null` for the workspace document.
- `document_stem`: governed document stem such as `spec-00011-rag-plan-implementation`.
- `document_hash`: source document content hash from discovery.
- `chunk_hash`: hash of the normalized chunk text and structural metadata.
- `chunk_ordinal`: zero-based ordinal within the document.
- `heading_path`: ordered heading labels from the document root to the chunk section.
- `text`: Markdown text to embed and store.
- `token_count`: token count used for limit enforcement.
- `previous_chunk_id`: previous chunk in the same document, or `null`.
- `next_chunk_id`: next chunk in the same document, or `null`.

Chunking rules:

- Keep each section heading with its section content.
- Never emit a chunk that contains only a heading.
- Never split inside a fenced code block.
- Prefer split points between paragraphs, list items, table rows, or block boundaries.
- Preserve valid Markdown for code blocks, lists, and tables after splitting.
- Split long sections into multiple chunks only when token count exceeds the configured maximum.
- Add overlap only between chunks created from the same oversized section.
- Preserve deterministic output for the same extracted document and configuration.

### Output Example: Small Section

Input section:

```markdown
## 2. Proposal

Implement a heading-aware Markdown chunker that receives extracted Markdown metadata
and emits ordered chunk records.

The chunker treats each heading section as the primary chunking unit.
```

Expected chunk:

```json
{
  "chunk_id": "workspace/spec-00011-rag-plan-implementation/2-proposal/0000/9f57c2b7",
  "package": null,
  "document_stem": "spec-00011-rag-plan-implementation",
  "document_hash": "4d2b8f0c1d6e",
  "chunk_hash": "9f57c2b7",
  "chunk_ordinal": 0,
  "heading_path": ["SPEC 00011: RAG Plan Implementation", "2. Proposal"],
  "text": "## 2. Proposal\n\nImplement a heading-aware Markdown chunker that receives extracted Markdown metadata\nand emits ordered chunk records.\n\nThe chunker treats each heading section as the primary chunking unit.",
  "token_count": 31,
  "previous_chunk_id": null,
  "next_chunk_id": null
}
```

### Output Example: Oversized Section

Input section:

```markdown
### Phase 4: Build Heading-Aware Chunking

Implement chunking that respects Markdown structure.

- Keep headings with their section content.
- Avoid splitting fenced code blocks.
- Avoid chunks that contain only a heading.
- Split long sections with token-aware limits.
- Add overlap only for sections that exceed the maximum chunk size.
- Store heading path, chunk ordinal, token count, neighboring chunk references, and chunk hash.

Additional paragraphs continue until the section exceeds the maximum token count.
```

Expected chunks:

```json
[
  {
    "chunk_id": "workspace/spec-00011-rag-plan-implementation/phase-4-build-heading-aware-chunking/0002/0e13c65a",
    "package": null,
    "document_stem": "spec-00011-rag-plan-implementation",
    "document_hash": "4d2b8f0c1d6e",
    "chunk_hash": "0e13c65a",
    "chunk_ordinal": 2,
    "heading_path": ["SPEC 00011: RAG Plan Implementation", "2. Definition", "Phase 4: Build Heading-Aware Chunking"],
    "text": "### Phase 4: Build Heading-Aware Chunking\n\nImplement chunking that respects Markdown structure.\n\n- Keep headings with their section content.\n- Avoid splitting fenced code blocks.\n- Avoid chunks that contain only a heading.",
    "token_count": 46,
    "previous_chunk_id": "workspace/spec-00011-rag-plan-implementation/phase-3-extract-markdown-metadata/0001/a86f92d1",
    "next_chunk_id": "workspace/spec-00011-rag-plan-implementation/phase-4-build-heading-aware-chunking/0003/77a8f9c4"
  },
  {
    "chunk_id": "workspace/spec-00011-rag-plan-implementation/phase-4-build-heading-aware-chunking/0003/77a8f9c4",
    "package": null,
    "document_stem": "spec-00011-rag-plan-implementation",
    "document_hash": "4d2b8f0c1d6e",
    "chunk_hash": "77a8f9c4",
    "chunk_ordinal": 3,
    "heading_path": ["SPEC 00011: RAG Plan Implementation", "2. Definition", "Phase 4: Build Heading-Aware Chunking"],
    "text": "### Phase 4: Build Heading-Aware Chunking\n\n- Split long sections with token-aware limits.\n- Add overlap only for sections that exceed the maximum chunk size.\n- Store heading path, chunk ordinal, token count, neighboring chunk references, and chunk hash.\n\nAdditional paragraphs continue until the section exceeds the maximum token count.",
    "token_count": 58,
    "previous_chunk_id": "workspace/spec-00011-rag-plan-implementation/phase-4-build-heading-aware-chunking/0002/0e13c65a",
    "next_chunk_id": "workspace/spec-00011-rag-plan-implementation/phase-5-add-embedding-boundary/0004/62c71b8e"
  }
]
```

The example token counts are illustrative. Tests should assert counts using the project tokenizer rather than these exact numbers.

### Output Example: Fenced Code Block

Input section:

````markdown
## 4. Example

The chunker must keep this block intact:

```clojure
(defn chunk-document [document config]
  (-> document
      extract-sections
      (split-sections config)))
```

The next paragraph can be split if the section is too large.
````

Expected chunk text:

````markdown
## 4. Example

The chunker must keep this block intact:

```clojure
(defn chunk-document [document config]
  (-> document
      extract-sections
      (split-sections config)))
```
````

The chunker may split before or after the fenced block, but never inside it.

## 3. Alternatives Considered

- **Line-based chunking:** Discarded because it can detach headings from content and split Markdown blocks into invalid fragments.
- **Fixed-token chunking without Markdown parsing:** Discarded because it maximizes implementation simplicity at the cost of retrieval quality and source attribution.
- **One chunk per heading section with no splitting:** Discarded because large sections can exceed embedding limits and reduce retrieval precision.
- **Always overlapping chunks:** Discarded because overlap increases storage and embedding cost for sections that already fit within the maximum token size.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Preserves heading context for retrieved chunks. | Requires a Markdown-aware block model instead of a simple string splitter. |
| Keeps code blocks, lists, and tables valid after splitting. | Token-aware splitting must coordinate Markdown structure with tokenizer behavior. |
| Produces stable chunk metadata for incremental indexing and retrieval expansion. | Chunk identifiers can still change when a heading path changes. |
| Limits overlap to oversized sections, reducing unnecessary embedding and storage cost. | Retrieved chunks from compact sections may have less nearby context unless retrieval expansion is used. |

## 5. Acceptance Criteria

- [ ] Chunking is deterministic for the same extracted document and chunking configuration.
- [ ] Each chunk includes heading path, chunk ordinal, token count, chunk hash, and neighboring chunk references.
- [ ] Each chunk includes package and governed document stem identity.
- [ ] Chunks never contain only a heading.
- [ ] Fenced code blocks are never split.
- [ ] Lists and tables remain valid Markdown after splitting.
- [ ] Sections at or below the maximum token limit are emitted without overlap.
- [ ] Sections above the maximum token limit are split with token-aware limits and local overlap.
- [ ] Chunk identifiers remain stable when unchanged content keeps the same package, document stem, heading path, ordinal, and chunk hash.
- [ ] Tests cover short sections, duplicate headings, nested headings, oversized sections, fenced code blocks, lists, tables, and package documents.

## 6. Open Questions

- Should `chunk_id` include a shortened heading-path slug, or should it use only package, document stem, ordinal, and chunk hash?
- Which tokenizer should define the authoritative token count before the embedding boundary is implemented?
- What overlap size should be used for oversized sections if the default plan only defines target and maximum token counts?
- Should table splitting preserve the header row in every split table chunk?
