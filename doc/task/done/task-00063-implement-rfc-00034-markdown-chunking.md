---
id: task-00063-implement-rfc-00034-markdown-chunking
type: task
code: "00063"
slug: implement-rfc-00034-markdown-chunking
title: Implement RFC 00034 Markdown Chunking
description: Implement deterministic heading-aware Markdown chunking for the local RAG pipeline.
status: done
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

- [x] Emit sections at or below the maximum token limit without overlap.
- [x] Split sections above the maximum token limit using token-aware limits and preferred Markdown block boundaries.
- [x] Never split inside fenced code blocks.
- [x] Preserve valid Markdown when splitting lists and tables.
- [x] Apply overlap only between chunks created from the same oversized section.
- [x] Decide and document table split behavior, including whether each split table chunk repeats the header row.

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

- [x] Derive `chunk_id` from package identity, document stem, heading path or heading slug, chunk ordinal, and chunk hash.
- [x] Compute `chunk_hash` from normalized chunk text and structural metadata.
- [x] Preserve package identity as `null` or equivalent for workspace documents and as package name for synchronized package documents.
- [x] Populate `previous_chunk_id` and `next_chunk_id` for adjacent chunks in the same document.
- [x] Add tests proving unchanged chunks keep stable identifiers when unrelated document content changes outside their identity inputs.

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

- [x] Wire the chunker after normalized Markdown metadata extraction and before embedding.
- [x] Keep Phase 5 embedding and Phase 6 storage boundaries decoupled from Markdown parsing details.
- [x] Return actionable errors for malformed extraction inputs, unsupported Markdown structures, and unsplittable oversized blocks.
- [x] Confirm package documents and workspace documents follow the same chunking semantics.
- [x] Update any README or developer documentation for the new chunking boundary if a modified package exposes it.

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

- [x] Confirm every acceptance criterion in [[rfc-00034-markdown-chunking]] is covered by implementation or tests.
- [x] Run the project quality gate for Rust and governed Markdown changes.
- [x] Update README files on packages modified.
- [x] Move [[rfc-00034-markdown-chunking]] to implemented status only after implementation and validation are complete.

## 4. Quality Gate

- [x] Chunker unit tests pass.
- [x] RAG pipeline integration tests pass.
- [x] Governed document validation passes.
- [x] Project formatting and linting pass.

## 5. Validation Vector

- [x] Chunking is deterministic for the same extracted document and configuration.
- [x] Each chunk includes heading path, chunk ordinal, token count, chunk hash, and neighboring chunk references.
- [x] Each chunk includes package identity and governed document stem identity.
- [x] Chunks never contain only a heading.
- [x] Fenced code blocks are never split.
- [x] Lists and tables remain valid Markdown after splitting.
- [x] Sections at or below the maximum token limit are emitted without overlap.
- [x] Sections above the maximum token limit are split with token-aware limits and local overlap.
- [x] Chunk identifiers remain stable when unchanged content keeps the same package, document stem, heading path, ordinal, and chunk hash.
- [x] Tests cover short sections, duplicate headings, nested headings, oversized sections, fenced code blocks, lists, tables, and package documents.

## 6. Gaps, Flaws, and Tradeoffs

- **Gap:** The authoritative tokenizer still belongs to the future embedding boundary. The current deterministic whitespace adapter keeps the chunker contract stable, but it must be replaced if embedding tokenization diverges materially.
- **Resolved:** Split table chunks repeat the table header row and separator row so every emitted table fragment remains valid Markdown.
- **Flaw:** Heading-path-based identifiers are readable but can change when headings are renamed, even if body content remains equivalent.
- **Tradeoff:** Markdown-aware chunking adds parser complexity and test surface, but it preserves retrieval context and avoids invalid chunk text that a fixed-size splitter would produce.
