---
id: task-00059-improve-runtime-io-directory-traversal
type: task
code: "00059"
slug: improve-runtime-io-directory-traversal
title: Improve Runtime IO Directory Traversal
description: Adds reusable directory traversal and file-reading primitives needed by Markdown discovery.
status: in-progress
created: 2026-06-10
updated: 2026-06-10
tags:
  - rust
  - runtime-io
  - filesystem
  - markdown
related:
  - rfc-00032-markdown-discovery
  - spec-00011-rag-plan-implementation
  - project-0003-rust-dependencies
supersedes: []
superseded_by: null
---

# Task 00059: Improve Runtime IO Directory Traversal

## 1. Prime Directive

`runtime-markdown` needs deterministic filesystem traversal and file reading for Markdown discovery, but `runtime-io` currently exposes file, text, path, memory, and command primitives without directory listing or recursive traversal. Add generic directory primitives to `runtime-io` so Markdown discovery can depend on IO boundaries instead of using filesystem APIs directly.

## 2. Specs

- **Module:** `runtime-io`
- **Dependencies:** `tokio`; any new third-party traversal dependency must be registered in [[project-0003-rust-dependencies]] before use.

## 3. Checklist

### 3.1. Phase A — Directory Traversal Boundary

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00059
  phase: Phase A
  language: rust
```

- [ ] Add a generic directory entry type that exposes path, file type, and modified time when available.
- [ ] Add non-recursive directory listing support.
- [ ] Add deterministic recursive traversal support over `IoPath` roots.
- [ ] Ensure traversal output ordering is stable across repeated runs.
- [ ] Ensure missing directories and unreadable entries return typed `IoError` values.
- [ ] Keep `runtime-io` free of Markdown, RAG, package, and governed-document semantics.

### 3.2. Phase B — File Read Integration

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00059
  phase: Phase B
  language: rust
```

- [ ] Confirm existing byte and text readers can be used by callers that receive paths from traversal.
- [ ] Add helper APIs only if existing `read_file_bytes` and text adapters are insufficient for Markdown discovery.
- [ ] Cover traversal and file-reading composition with deterministic tests.
- [ ] Document how higher-level crates should combine traversal with file readers.

### 3.3. Phase C — Dependency Governance

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00059
  phase: Phase C
  language: markdown, toml, rust
```

- [ ] If a new traversal dependency is introduced, update [[project-0003-rust-dependencies]] before adding it to `Cargo.toml`.
- [ ] Scope any new traversal dependency to `runtime-io` unless another crate has a documented need.
- [ ] Update workspace dependencies and `runtime/io/Cargo.toml` consistently.

### 3.4. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00059
  phase: Phase Z
  language: markdown, rust
```

- [ ] Update `runtime/io/README.md` with the directory traversal contract.
- [ ] Run focused `runtime-io` tests.
- [ ] Run relevant workspace checks for affected crates.
