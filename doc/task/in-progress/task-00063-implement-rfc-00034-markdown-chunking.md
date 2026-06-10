---
id: task-00063-implement-rfc-00034-markdown-chunking
type: task
code: "00063"
slug: implement-rfc-00034-markdown-chunking
title: Implement RFC 00034 Markdown Chunking
description: Implement deterministic heading-aware Markdown chunking for the local RAG pipeline.
status: in-progress
created: 2026-06-10
updated: 2026-06-10
tags:
  - rag
  - markdown
  - chunking
related:
  - rfc-00034-markdown-chunking
  - spec-00011-rag-plan-implementation
supersedes: []
superseded_by: null
---

# Task 00063: Implement RFC 00034 Markdown Chunking

## 1. Prime Directive

> [!Prime Directive]
> Eliminate retrieval-quality and attribution gaps caused by naive Markdown splitting by implementing deterministic, heading-aware chunks that preserve Markdown structure, stable identity, token limits, and neighboring chunk metadata.

## 2. Specs

- **Module:** local RAG indexing pipeline, Markdown extraction output, chunking boundary before embedding
- **Dependencies:** [[rfc-00034-markdown-chunking]], [[spec-00011-rag-plan-implementation]]
- **Defaults:** target chunk size `350` tokens, maximum chunk size `500` tokens, overlap only for oversized sections

## 3. Checklist

### 3.1. Phase A - Define chunking contracts and fixtures

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00063
  phase: Phase A
  language: Rust, Markdown
```

- [x] Locate the Phase 3 normalized Markdown extraction output model and define the chunker input contract without duplicating extraction responsibilities.
- [x] Define the chunk record fields required by [[rfc-00034-markdown-chunking]]: package, document stem, document hash, chunk hash, ordinal, heading path, text, token count, and neighboring chunk references.
- [x] Add fixtures for short sections, nested headings, duplicate headings, oversized sections, fenced code blocks, lists, tables, and synchronized package documents.
- [x] Establish the authoritative tokenizer or a temporary adapter that can be replaced by the embedding boundary without changing the chunker contract.
- [x] Add regression tests for deterministic output from the same extracted document and chunking configuration.

### 3.2. Phase B - Implement heading-aware sectioning

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00063
  phase: Phase B
  language: Rust, Markdown
```

- [x] Parse Markdown into heading sections that include heading path and content until the next heading of the same or higher level.
- [x] Ensure emitted chunk text includes the relevant heading context.
- [x] Prevent chunks that contain only a heading.
- [x] Preserve deterministic ordering and zero-based chunk ordinals for each document.
- [x] Cover root-level content before the first heading if existing extracted documents can contain it.

### 3.3. Phase C - Split oversized sections safely

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00063
  phase: Phase C
  language: Rust, Markdown
```

- [ ] Emit sections at or below the maximum token limit without overlap.
- [ ] Split sections above the maximum token limit using token-aware limits and preferred Markdown block boundaries.
- [ ] Never split inside fenced code blocks.
- [ ] Preserve valid Markdown when splitting lists and tables.
- [ ] Apply overlap only between chunks created from the same oversized section.
- [ ] Decide and document table split behavior, including whether each split table chunk repeats the header row.

### 3.4. Phase D - Add stable identity and neighbor metadata

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00063
  phase: Phase D
  language: Rust, Markdown
```

- [ ] Derive `chunk_id` from package identity, document stem, heading path or heading slug, chunk ordinal, and chunk hash.
- [ ] Compute `chunk_hash` from normalized chunk text and structural metadata.
- [ ] Preserve package identity as `null` or equivalent for workspace documents and as package name for synchronized package documents.
- [ ] Populate `previous_chunk_id` and `next_chunk_id` for adjacent chunks in the same document.
- [ ] Add tests proving unchanged chunks keep stable identifiers when unrelated document content changes outside their identity inputs.

### 3.5. Phase E - Integrate chunking into the RAG pipeline

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00063
  phase: Phase E
  language: Rust, Markdown
```

- [ ] Wire the chunker after normalized Markdown metadata extraction and before embedding.
- [ ] Keep Phase 5 embedding and Phase 6 storage boundaries decoupled from Markdown parsing details.
- [ ] Return actionable errors for malformed extraction inputs, unsupported Markdown structures, and unsplittable oversized blocks.
- [ ] Confirm package documents and workspace documents follow the same chunking semantics.
- [ ] Update any README or developer documentation for the new chunking boundary if a modified package exposes it.

### 3.6. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00063
  phase: Phase Z
  language: Rust, Markdown
```

- [ ] Confirm every acceptance criterion in [[rfc-00034-markdown-chunking]] is covered by implementation or tests.
- [ ] Run the project quality gate for Rust and governed Markdown changes.
- [ ] Update README files on packages modified.
- [ ] Move [[rfc-00034-markdown-chunking]] to implemented status only after implementation and validation are complete.

## 4. Quality Gate

- [ ] Chunker unit tests pass.
- [ ] RAG pipeline integration tests pass.
- [ ] Governed document validation passes.
- [ ] Project formatting and linting pass.

## 5. Validation Vector

- [ ] Chunking is deterministic for the same extracted document and configuration.
- [ ] Each chunk includes heading path, chunk ordinal, token count, chunk hash, and neighboring chunk references.
- [ ] Each chunk includes package identity and governed document stem identity.
- [ ] Chunks never contain only a heading.
- [ ] Fenced code blocks are never split.
- [ ] Lists and tables remain valid Markdown after splitting.
- [ ] Sections at or below the maximum token limit are emitted without overlap.
- [ ] Sections above the maximum token limit are split with token-aware limits and local overlap.
- [ ] Chunk identifiers remain stable when unchanged content keeps the same package, document stem, heading path, ordinal, and chunk hash.
- [ ] Tests cover short sections, duplicate headings, nested headings, oversized sections, fenced code blocks, lists, tables, and package documents.

## 6. Gaps, Flaws, and Tradeoffs

- **Gap:** The authoritative tokenizer must be confirmed before implementation. A temporary tokenizer adapter reduces blocking but creates a replacement task if it diverges from embedding behavior.
- **Gap:** Table splitting behavior needs an explicit decision because preserving valid Markdown may require repeating the table header in each split chunk.
- **Flaw:** Heading-path-based identifiers are readable but can change when headings are renamed, even if body content remains equivalent.
- **Tradeoff:** Markdown-aware chunking adds parser complexity and test surface, but it preserves retrieval context and avoids invalid chunk text that a fixed-size splitter would produce.
