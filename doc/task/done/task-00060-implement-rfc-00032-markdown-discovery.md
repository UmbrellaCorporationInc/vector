---
id: task-00060-implement-rfc-00032-markdown-discovery
type: task
code: "00060"
slug: implement-rfc-00032-markdown-discovery
title: Implement RFC 00032 Markdown Discovery
description: Implement the runtime crate boundary and deterministic Markdown discovery behavior proposed by RFC 00032.
status: done
created: 2026-06-10
updated: 2026-06-10
tags:
  - markdown
  - rag
  - runtime
  - discovery
related:
  - rfc-00032-markdown-discovery
  - spec-00011-rag-plan-implementation
  - project-0003-rust-dependencies
  - task-00059-improve-runtime-io-directory-traversal
supersedes: []
superseded_by: null
---

# Task 00060: Implement RFC 00032 Markdown Discovery

## 1. Prime Directive

> [!Prime Directive]
> Implement [[rfc-00032-markdown-discovery]] so local RAG indexing has a deterministic Markdown discovery boundary that separates RAG orchestration, Markdown semantics, and reusable filesystem IO.

## 2. Specs

- **Module:** `runtime-rag`, `runtime-markdown`, `runtime-io`
- **Dependencies:** Register any new Rust third-party dependency in [[project-0003-rust-dependencies]] before adding it to `Cargo.toml`.

## 3. Checklist

### 3.1. Phase A -- Runtime IO content hashing primitive

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00060
  phase: Phase A
  language: Rust, TOML, Markdown
```

- [x] Register `blake3` for `runtime-io` in [[project-0003-rust-dependencies]] before modifying workspace manifests.
- [x] Add a generic `runtime-io` API that accepts an `IoPath` and returns a typed file content hash, such as `hash_file_content(path: &IoPath) -> Result<FileContentHash, IoError>`.
- [x] Define the hash as BLAKE3 over file bytes only; exclude path, modified time, package identity, and Markdown metadata from the hash input.
- [x] Return a stable lowercase hex representation from the typed hash value.
- [x] Implement hashing through the `runtime-io` file boundary, preferably streaming file bytes instead of requiring callers to load whole files.
- [x] Keep the hashing API free of Markdown, RAG, package, and governed-document semantics.
- [x] Add tests proving same bytes produce the same hash across different paths, changed bytes change the hash, and modified time changes without content changes do not change the hash.

### 3.2. Phase B -- Remaining dependency governance

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00060
  phase: Phase B
  language: Rust, TOML, Markdown
```

- [x] Identify whether traversal requires a new third-party Rust dependency.
- [x] Register any required dependency not covered by Phase A in [[project-0003-rust-dependencies]] before modifying workspace manifests.
- [x] Keep implementation blocked on governance if a required dependency is not approved.

### 3.3. Phase C -- Runtime IO traversal boundary

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00060
  phase: Phase C
  language: Rust
```

- [x] Extend `runtime-io` with generic directory traversal primitives required by Markdown discovery.
- [x] Extend `runtime-io` with file-reading primitives required by Markdown discovery.
- [x] Keep `runtime-io` free of Markdown, RAG, package, and governed-document semantics.
- [x] Add focused tests for traversal, ignored-path handling, and file-reading behavior.

### 3.4. Phase D -- Runtime crate boundary

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00060
  phase: Phase D
  language: Rust, TOML
```

- [x] Create or update `runtime-markdown` as the owner of Markdown discovery APIs.
- [x] Create or update `runtime-rag` as the owner of Phase 1 RAG defaults and Markdown discovery orchestration.
- [x] Make `runtime-rag` depend on `runtime-markdown`.
- [x] Prevent `runtime-markdown` from depending on `runtime-rag`, LanceDB, embedding code, or MCP-facing types.

### 3.5. Phase E -- Markdown discovery API

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00060
  phase: Phase E
  language: Rust
```

- [x] Define a discovery API that accepts explicit workspace roots, package roots, ignore behavior, and hashing options from `runtime-rag`.
- [x] Discover Markdown files under workspace `doc/` and package `.vector-database/packages/{package}/doc/` roots.
- [x] Include `.md` and `.markdown` files.
- [x] Validate governed document stems against `<doc-type>-<code>-<slug>`.
- [x] Emit deterministic records containing package identity, governed document stem, modified time, content hash, and internal read path.
- [x] Use `null` or an equivalent workspace marker for workspace-local package identity.
- [x] Use the `runtime-io` file content hashing primitive for discovery record content hashes.
- [x] Ensure file hashes change only when file content changes.
- [x] Report missing package `doc/` folders as package-structure errors, not workspace discovery failures.
- [x] Add tests for workspace discovery, package discovery, extension filtering, invalid stems, ignored paths, missing package `doc/`, and content-hash stability.

### 3.6. Phase Z -- Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00060
  phase: Phase Z
  language: Rust, Markdown
```

- [x] Verify all acceptance criteria from [[rfc-00032-markdown-discovery]] are either implemented or explicitly deferred in a follow-up governed document.
- [x] Run the relevant Rust tests and project quality gates.
- [x] Update README files for any package whose public behavior or setup changes.
- [x] Keep the task status aligned with implementation progress.
