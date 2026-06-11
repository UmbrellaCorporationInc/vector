---
id: task-00065-improve-patch-doc-hunk-count-diagnostics
type: task
code: "00065"
slug: improve-patch-doc-hunk-count-diagnostics
title: Improve patch_doc Hunk Count Diagnostics
description: Make patch_doc failures for unified-diff hunk count mismatches actionable for agents.
status: in-progress
created: 2026-06-11
updated: 2026-06-11
tags:
  - mcp
  - patching
  - diagnostics
  - governance
related:
  - task-00062-fix-patch-doc-or-find-doc
supersedes: []
superseded_by: null
---

# Task 00065: Improve patch_doc Hunk Count Diagnostics

## 1. Prime Directive

> [!Prime Directive]
> Eliminate the agent friction where `patch_doc` returns a technically correct but under-explained parser error for malformed unified-diff hunk line counts.

The observed failure happened while updating `task 00064` through the MCP `patch_doc` tool.
The diff hunk header declared `@@ -37,10 +37,10 @@ input:`, but the actual hunk body contained only seven old-side and seven new-side lines after parsing the context and changed lines.
The `patcher` parser rejected the diff before `patch_doc` could validate the target document or apply the patch:

```text
patch is not a valid unified diff: Invalid patch format: Chunk line count mismatch: Header expected (-10, +10), Parsed content counts (-7, +7). Chunk Header: @@ -37,10 +37,10 @@ input:
```

This specific failure was not caused by `find_doc`, document line endings, trailing newline handling, or patch application drift.
It was caused by a malformed agent-generated hunk header.
The tool should still reject malformed diffs, but the rejection should tell the caller how to fix the hunk count and should be covered by a regression test.

## 2. Specs

- **Module:** `runtime-doc`, `mcp-vector`
- **Primary files:** `runtime/doc/src/operations/patch_doc.rs`, `runtime/doc/src/operations/patch_doc_test.rs`, `mcp/vector/src/tools/document.rs`, `mcp/vector/src/tools/document_test.rs`
- **Dependencies:** none
- **Related work:** [[task-00062-fix-patch-doc-or-find-doc]]

## 3. Checklist

### 3.1. Phase A — Reproduce Hunk Count Mismatch

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00065
  phase: Phase A
  language: Rust, Markdown
```

- [x] Add a focused regression test where `patch_doc` receives a unified diff whose hunk header declares more old/new lines than the hunk body contains.
- [x] Assert that the failure is produced during diff parsing, before target mismatch checks and before file writes.
- [x] Include the original failure shape in the test fixture or assertion message: declared hunk counts differ from parsed hunk counts.
- [x] Confirm the target document remains unchanged after the malformed diff is rejected.

### 3.2. Phase B — Improve Runtime Diagnostics

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00065
  phase: Phase B
  language: Rust
```

- [x] Add a small diagnostic adapter around `Patch::parse` errors in `runtime/doc/src/operations/patch_doc.rs`.
- [x] Detect hunk line-count mismatch messages and return an actionable `patch_doc` error that explains the likely fix: make the `@@ -a,b +c,d @@` counts match the number of old-side and new-side lines in the hunk body.
- [x] Preserve the original parser error text as supporting detail.
- [x] Keep all existing rejection behavior for create, delete, rename, target mismatch, BOM, and malformed non-diff inputs.

### 3.3. Phase C — Improve MCP Surface

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00065
  phase: Phase C
  language: Rust
```

- [x] Ensure the MCP `patch_doc` tool returns the improved diagnostic without losing the `patch_doc failed:` prefix used by callers.
- [x] Add or update MCP tool tests for malformed hunk count diagnostics.
- [x] Verify the improved diagnostic is short enough to be useful in agent feedback loops.

### 3.4. Phase D — Evaluate Safe Preflight Validation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00065
  phase: Phase D
  language: Rust
```

- [x] Decide whether `patch_doc` should preflight hunk headers before calling `Patch::parse`.
- [x] If preflight is added, keep it diagnostic-only unless there is an explicit product decision to auto-correct malformed hunk counts.
- [x] Avoid silently rewriting agent diffs because incorrect hunk counts may indicate missing context lines or truncated content.
- [x] Add tests proving valid multi-hunk diffs are not rejected by the preflight path.

### 3.5. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00065
  phase: Phase Z
  language: Rust, Markdown
```

- [ ] Run `cargo test -p runtime-doc patch_doc`.
- [ ] Run MCP document tool tests that cover `patch_doc`.
- [ ] Run the Rust quality gate.
- [ ] Run governed document validation.
- [ ] Update `runtime/doc/README.md` or `mcp/vector/README.md` only if the user-facing `patch_doc` behavior changes.
