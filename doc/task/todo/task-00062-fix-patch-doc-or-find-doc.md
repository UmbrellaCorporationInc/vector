---
id: task-00062-fix-patch-doc-or-find-doc
type: task
code: "00062"
slug: fix-patch-doc-or-find-doc
title: Fix patch_doc Rejections for Governed Documents
description: Make governed document patching reliable when agents edit content returned by find_doc.
status: todo
created: 2026-06-10
updated: 2026-06-10
tags:
  - mcp
  - patching
  - governance
related: []
supersedes: []
superseded_by: null
---

# Task 00062: Fix patch_doc Rejections for Governed Documents

## 1. Prime Directive

> [!Prime Directive]
> Agents cannot reliably update governed documents through `patch_doc` because valid-looking patches can be rejected, likely due to line-ending, trailing-newline, or context-offset mismatches between the content returned by `find_doc` and the content consumed by the patch engine.

The structural friction to eliminate is the mismatch between the document content agents inspect and the document content `patch_doc` validates. The fix should make the patch path deterministic, observable, and covered by regression tests.

## 2. Specs

- **Module:** vector MCP document lookup and patching tools
- **Dependencies:** none

### 2.1. Decision Context

The proposed idea is to change `find_doc` so it returns only the package and path, requiring the agent to read the file directly before calling `patch_doc`.

That change may reduce one class of content mismatch when the target document is local and readable from the agent environment. It also weakens the `find_doc` contract, removes useful context from a lookup tool, and may break package-qualified or remote-like document flows where the caller cannot safely assume direct filesystem access.

The recommended path is to keep `find_doc` content-compatible and fix the authoritative mutation path in `patch_doc`.

### 2.2. Target Behavior

- `find_doc` continues returning path, package, and content for lookup workflows.
- `patch_doc` applies patches against the same canonical document bytes or normalized text representation used to produce lookup content.
- `patch_doc` accepts patches that differ only by safe newline details, including final newline presence, CRLF versus LF normalization, and common unified-diff formatting differences.
- `patch_doc` rejects genuinely invalid patches with diagnostics that identify the failing hunk, expected context, observed context, and newline mode.
- Tests cover the failure mode before implementation and the accepted behavior after implementation.

Use [[prompts-00004-execute-task-phase]] for phase execution.

## 3. Checklist

### 3.1. Phase A - Reproduce and Localize

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase A
  language: Rust, Markdown
```

- [ ] Add or identify a failing regression case where `patch_doc` rejects a patch generated from `find_doc` content.
- [ ] Confirm whether the rejection is caused by trailing newline handling, CRLF normalization, hunk context mismatch, offset drift, or another parser issue.
- [ ] Document the exact failing path in the test name or assertion message.

### 3.2. Phase B - Stabilize patch_doc

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase B
  language: Rust, Markdown
```

- [ ] Make `patch_doc` patch against a canonical representation that preserves enough information to write the final file correctly.
- [ ] Normalize only newline differences that are safe and reversible for governed Markdown documents.
- [ ] Preserve existing frontmatter and validation semantics.
- [ ] Improve rejection diagnostics for failed hunks.
- [ ] Keep `find_doc` backward-compatible unless implementation evidence proves the contract itself is the root cause.

### 3.3. Phase C - Evaluate Optional API Refinement

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase C
  language: Rust, Markdown
```

- [ ] Decide whether `find_doc` should add an optional path-only or metadata-only mode instead of removing content from the default response.
- [ ] If a new mode is added, keep the default response compatible for existing agents.
- [ ] Add tests for both default lookup and any new optional lookup mode.

### 3.4. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase Z
  language: Rust, Markdown
```

- [ ] Run the relevant unit and integration tests.
- [ ] Run governed document validation.
- [ ] Update README files only if user-facing tool behavior changes.
