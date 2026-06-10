---
id: rfc-00032-markdown-discovery
type: rfc
code: "00032"
slug: markdown-discovery
title: Markdown Discovery Runtime Boundary
description: Proposes the runtime crate boundary for Phase 2 Markdown discovery in the local RAG implementation.
status: draft
created: 2026-06-10
updated: 2026-06-10
authors: []
tags:
  - markdown
  - rag
  - runtime
  - discovery
related:
  - spec-00011-rag-plan-implementation
  - project-0003-rust-dependencies
  - task-00059-improve-runtime-io-directory-traversal
supersedes: []
superseded_by: null
aliases:
  - "RFC 00032: Markdown Discovery Runtime Boundary"
---

# RFC 00032: Markdown Discovery Runtime Boundary

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00032-markdown-discovery`
  document-type: task
  document-name: implement-rfc-00032-markdown-discovery
```

## 1. Problem

Phase 2 of [[spec-00011-rag-plan-implementation]] requires deterministic discovery of local Markdown corpora for the RAG indexer. The current plan defines the behavior, but it does not yet define the runtime crate boundary that will own Markdown discovery or the dependency-governance steps needed before implementation.

The discovery implementation must support both workspace documents and synchronized package documents without coupling Markdown parsing and file walking directly to the future retrieval store. It also needs a clear ownership model for Phase 1 RAG defaults, because those defaults define the corpus roots and storage conventions that drive discovery.

## 2. Proposal

Create two runtime crates for the local RAG implementation:

- `runtime-rag`: owns Phase 1 RAG defaults, configuration loading, corpus-root resolution, and orchestration of the RAG pipeline.
- `runtime-markdown`: owns Markdown file discovery and later Markdown metadata/chunking capabilities.

`runtime-rag` will depend on `runtime-markdown`. `runtime-markdown` must not depend on `runtime-rag`, LanceDB, embedding code, or MCP-facing types. This keeps Markdown discovery reusable from CLI, tests, and future indexing entrypoints.

`runtime-markdown` must use `runtime-io` primitives for directory traversal and file reading. Filesystem access should enter through `runtime-io`; Markdown-specific logic should remain in `runtime-markdown`. This keeps IO behavior reusable and testable while preventing `runtime-io` from learning about Markdown extensions, package identity, governed document stems, or RAG indexing records.

For Phase 2, `runtime-markdown` will expose a discovery API that accepts explicit arguments from `runtime-rag`, including workspace document roots, package document roots, ignore behavior, and hashing options required by discovery. The RAG crate will translate its Phase 1 defaults into those arguments before invoking the Markdown crate. The discovery implementation will ask `runtime-io` to walk the provided roots and read candidate files, then apply Markdown filtering, governed stem validation, package attribution, and content hashing inside `runtime-markdown`.

The discovery output must include stable file records with:

- package identity, using `null` or an equivalent workspace marker for workspace-local documents;
- governed document stem;
- modified time;
- content hash;
- internal read path required by the indexer.

The implementation must include `.md` and `.markdown` files, respect ignored paths, and treat missing package `doc/` folders as package-structure errors instead of workspace discovery failures. Governed document stems must follow `<doc-type>-<code>-<slug>`.

Any third-party dependency needed by either crate must be approved in [[project-0003-rust-dependencies]] before it is added to `Cargo.toml`. Likely candidates include a recursive walking dependency for `runtime-io` and a stable content hashing dependency if the standard library is not sufficient for deterministic file hashes.

## 3. Alternatives Considered

- **Put Markdown discovery inside `runtime-rag`:** Discarded because it would make the RAG crate own low-level Markdown and filesystem behavior, making later metadata extraction and chunking harder to test and reuse independently.
- **Put RAG defaults inside `runtime-markdown`:** Discarded because corpus roots, retrieval limits, embedding defaults, and storage paths are RAG concerns. Markdown discovery should receive only the discovery inputs it needs.
- **Let `runtime-markdown` use filesystem APIs directly:** Discarded because traversal and file reading are reusable IO concerns. Keeping those primitives in `runtime-io` gives Markdown discovery a narrower, testable boundary.
- **Reuse `runtime-doc` for Markdown discovery:** Discarded for this RAG phase because `runtime-doc` owns governed document operations. RAG discovery needs package-aware corpus traversal, hashing, and indexer-oriented records, which should not broaden the existing document-authoring boundary without a separate justification.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Keeps Markdown discovery independent from RAG storage, embeddings, and MCP contracts. | Adds a new runtime crate and workspace dependency boundary. |
| Lets `runtime-rag` centralize Phase 1 defaults and orchestration. | Requires explicit translation from RAG configuration into Markdown discovery arguments. |
| Reuses `runtime-io` for traversal and file reading instead of duplicating filesystem behavior. | Requires `runtime-io` to grow a generic directory traversal boundary before Markdown discovery is implemented. |
| Makes Phase 3 metadata extraction and Phase 4 chunking natural extensions of the Markdown crate. | May duplicate some filesystem concepts already present in document-oriented crates unless the API is kept narrow. |
| Makes dependency approval explicit through [[project-0003-rust-dependencies]]. | Implementation cannot start with new third-party crates until dependency governance is updated. |

## 5. Acceptance Criteria

- [ ] A `runtime-markdown` crate exists and owns Markdown discovery APIs.
- [ ] A `runtime-rag` crate exists or is planned as the owner of Phase 1 RAG defaults and the caller of Markdown discovery.
- [ ] `runtime-rag` depends on `runtime-markdown`; `runtime-markdown` does not depend on `runtime-rag`.
- [ ] `runtime-markdown` uses `runtime-io` primitives for directory traversal.
- [ ] `runtime-markdown` uses `runtime-io` primitives for reading Markdown file contents.
- [ ] `runtime-io` remains free of Markdown, RAG, package, and governed-document semantics.
- [ ] Markdown discovery accepts the configuration it needs as function arguments instead of reading global RAG state.
- [ ] Discovery walks the workspace `doc/` folder and synchronized package `doc/` folders under `.vector-database/packages/{package}/doc/`.
- [ ] Discovery includes `.md` and `.markdown` files.
- [ ] Discovery emits deterministic records containing package, governed document stem, modified time, content hash, and internal read path.
- [ ] Discovery respects ignored paths.
- [ ] Missing package `doc/` folders are reported as package-structure errors.
- [ ] Governed document stems are validated against `<doc-type>-<code>-<slug>`.
- [ ] File hashes change only when file content changes.
- [ ] Any new third-party Rust dependency required by `runtime-rag` or `runtime-markdown` is registered in [[project-0003-rust-dependencies]] before implementation.
- [ ] Any new third-party Rust dependency required by `runtime-io` traversal is registered in [[project-0003-rust-dependencies]] before implementation.

## 6. Open Questions

- Which hashing crate should be approved for stable content hashing, if any?
- Should ignore behavior reuse existing project ignore semantics, Git ignore semantics, or a RAG-specific ignore contract?
- Should package discovery errors be accumulated into a partial discovery report or returned as a hard failure for the full package corpus?
