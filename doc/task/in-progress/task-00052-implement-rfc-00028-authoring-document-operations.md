---
id: task-00052-implement-rfc-00028-authoring-document-operations
type: task
code: "00052"
slug: implement-rfc-00028-authoring-document-operations
title: Implement RFC 00028 — Authoring Document Operations
description: Add a document-scoped patch operation in crate doc and expose it as an MCP tool so agents can update governed documents from git diffs without gaining broader filesystem write access.
status: in-progress
created: 2026-05-30
updated: 2026-05-30
tags:
  - governance
  - authoring
  - documents
  - mcp
related:
  - rfc-00028-authoring-document-operations
supersedes: []
superseded_by: null
---

# Task 00052: Implement RFC 00028 — Authoring Document Operations

## 1. Prime Directive

> [!Prime Directive]
> The current authoring flow has no narrow operation for patching a governed document from a git diff. That gap lets agents write outside `doc/`, leaves diff behavior underspecified, and allows BOM-encoded writes to break downstream tooling. This task closes that gap by adding a scoped `patch_doc` operation in crate doc and wiring it to an MCP tool of the same name.

## 2. Specs

- **Module:** `crate doc`, `crate mcp`
- **Dependencies:** `patcher` (register in `project-0003-rust-dependencies.md` before merge)

## 3. Checklist

### 3.1. Phase A — Register `patcher` Dependency

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00052
  phase: Phase A
  language: Rust, TOML, Markdown
```

- [x] Add `patcher` to `Cargo.toml` of crate doc (and workspace if needed).
- [x] Register `patcher` in `doc/project/project-0003-rust-dependencies.md` with scope limited to crate doc.
- [x] Confirm no other crate gains the dependency by accident.

### 3.2. Phase B — Implement `patch_doc` Operation in Crate Doc

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00052
  phase: Phase B
  language: Rust
```

- [x] Define the operation signature accepting `doc_type`, `code`, and `git_diff`.
- [x] Resolve the governed target document from `doc_type` and `code` using existing lookup logic.
- [x] Enforce scope: reject any patch that would touch a path outside the repository `doc/` directory.
- [x] Normalize agent-produced patch wrappers to raw unified diff before invoking `patcher`.
- [x] Reject unsupported patch shapes (non-text, rename-only, delete, or target mismatch).
- [x] Apply the normalized unified diff to the resolved document using `patcher`.
- [x] Verify the resulting content is UTF-8 without BOM; if BOM is detected, return an explicit remediation error without writing anything.
- [x] On success, return the final document content.

### 3.3. Phase C — Expose `patch_doc` MCP Tool

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00052
  phase: Phase C
  language: Rust
```

- [x] Add `patch_doc` tool registration in the MCP server layer.
- [x] Accept `doc_type`, `code`, and `git_diff` as MCP input parameters.
- [x] Delegate entirely to the crate doc operation; do not reimplement patching, path authorization, or encoding logic in the MCP adapter.
- [x] Return the final patched content or a structured validation error.

### 3.4. Phase D — Tests

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00052
  phase: Phase D
  language: Rust
```

- [x] Valid patch application on a governed document succeeds and returns final content.
- [x] Path outside `doc/` is rejected.
- [x] Missing document returns a clear error.
- [x] Malformed or unsupported diff shape is rejected before `patcher` is invoked.
- [x] Content with BOM is rejected without writing; error instructs the caller to remove the BOM.
- [x] Quality gates pass (`cargo test`, `cargo clippy`, `cargo fmt --check`).

### 3.5. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00052
  phase: Phase Z
  language: Rust, Markdown
```

- [x] Update README files for crate doc and crate mcp.
- [x] Mark RFC 00028 status as `accepted` once all acceptance criteria are met.
