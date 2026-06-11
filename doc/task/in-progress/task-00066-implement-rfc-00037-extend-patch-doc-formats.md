---
id: task-00066-implement-rfc-00037-extend-patch-doc-formats
type: task
code: "00066"
slug: implement-rfc-00037-extend-patch-doc-formats
title: Implement RFC 00037 Document Patch and Replacement Operations
description: Extend patch_doc to support explicit patch formats and add replace_doc for full governed document replacement.
status: in-progress
created: 2026-06-11
updated: 2026-06-11
tags:
  - mcp
  - patching
  - documents
related:
  - rfc-00037-extend-patch-doc-formats
supersedes: []
superseded_by: null
---

# Task 00066: Implement RFC 00037 Document Patch and Replacement Operations

## 1. Prime Directive

> [!Prime Directive]
> Eliminate malformed unified-diff friction and bootstrap full-document replacement friction in governed document edits.

This task implements [[rfc-00037-extend-patch-doc-formats]].

## 2. Specs

- **Primary files:** `runtime/doc/src/operations/patch_doc.rs`, `runtime/doc/src/operations/patch_doc_test.rs`, `runtime/doc/src/operations/replace_doc.rs`, `runtime/doc/src/operations/replace_doc_test.rs`, `mcp/vector/src/tools/document.rs`, `mcp/vector/src/tools/document_test.rs`
- **Documentation:** `runtime/doc/README.md`, `mcp/vector/README.md`
- **Dependencies:** none expected

## 3. Checklist

### 3.1. Phase A - Define the Patch Format Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00066
  phase: Phase A
  language: rust
```

- [x] Add a typed patch format contract for `patch_doc` with supported values `unified` and the omitted default `apply_patch`.
- [x] Rename the runtime payload concept from `git_diff` to `patch` internally where practical without breaking existing call sites unnecessarily.
- [x] Decide whether the MCP input should keep `git_diff` as a deprecated alias for explicit `format: "unified"` during the transition.
- [x] Reject unknown format values with an actionable error that lists supported values.
- [x] Preserve target document resolution from `doc_type`, `code`, and optional `package` as the authority for all formats.

### 3.2. Phase B - Preserve Explicit Unified Diff Behavior

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00066
  phase: Phase B
  language: rust
```

- [x] Route `format: "unified"` through the existing unified-diff parser and application path.
- [x] Keep existing hunk-count, newline normalization, BOM rejection, and target-mismatch protections for unified diffs.
- [x] Update unified-diff error messages so they identify `format: "unified"` and keep the existing actionable diagnostics.

### 3.3. Phase C - Add apply_patch-Style Updates

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00066
  phase: Phase C
  language: rust
```

- [x] Parse omitted `format` payloads as `apply_patch`-style patches with `*** Begin Patch` and `*** End Patch` boundaries.
- [x] Support `*** Update File:` only when the target path matches the governed document resolved by the tool inputs.
- [x] Apply update hunks without requiring numeric hunk ranges.
- [x] Reject `Add File`, `Delete File`, and `Move to` operations with format-specific errors.
- [x] Report missing boundaries, unsupported operations, ambiguous context, and target mismatches with actionable diagnostics.

### 3.4. Phase D - Expose the MCP Tool Shape and Documentation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00066
  phase: Phase D
  language: rust
```

- [ ] Update `PatchDocParams` so MCP callers can send `format` and `patch`.
- [ ] Keep the MCP adapter thin; parsing, validation, and patch application must stay in `runtime-doc`.
- [ ] Update runtime and MCP README documentation with one valid `format: "unified"` example and one valid omitted-format `apply_patch` example.
- [ ] Ensure generated tool documentation reflects the default format and supported values.

### 3.5. Phase E - Add replace_doc Runtime Operation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00066
  phase: Phase E
  language: rust
```

- [ ] Add a `replace_doc` operation in `runtime-doc`.
- [ ] Accept the same authoritative targeting fields as `patch_doc`: `root_dir`, `doc_type`, `code`, and optional `package`.
- [ ] Accept `content` as the complete replacement Markdown/plain-text document, not a patch format.
- [ ] Resolve the writable path from governed document metadata rather than accepting a caller-provided path.
- [ ] Reject replacement content with a UTF-8 BOM.
- [ ] Reject replacement content whose governed front matter identity does not match the resolved document, including `id`, `type`, `code`, and `slug`.
- [ ] Return the resolved path and final content after a successful write.

### 3.6. Phase F - Expose replace_doc MCP Tool and Documentation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00066
  phase: Phase F
  language: rust
```

- [ ] Add a thin MCP `replace_doc` tool that delegates to the runtime operation.
- [ ] Add MCP parameter handling for `root_dir`, `doc_type`, `code`, optional `package`, and `content`.
- [ ] Document `replace_doc` as the bootstrap companion to `create_doc_prompt`.
- [ ] Include one working runtime or MCP README example that replaces a newly created governed document.

### 3.7. Phase G - Test the Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00066
  phase: Phase G
  language: rust
```

- [ ] Add runtime tests for explicit unified diffs, omitted-format `apply_patch` updates, unknown formats, unsupported `apply_patch` operations, and target-file mismatch rejection.
- [ ] Add MCP tool tests for parameter deserialization, explicit unified diffs, omitted-format `apply_patch`, unknown formats, and target-file mismatch rejection.
- [ ] Add runtime `replace_doc` tests for successful replacement, missing document, mismatched identity, BOM rejection, and returned content.
- [ ] Add MCP `replace_doc` tests for parameter deserialization, successful replacement, runtime error propagation, and returned content.
- [ ] Keep regression coverage for malformed unified-diff hunk counts.
- [ ] Run `cargo test -p runtime-doc patch_doc`.
- [ ] Run `cargo test -p runtime-doc replace_doc`.
- [ ] Run MCP document tool tests that cover `patch_doc`.
- [ ] Run MCP document tool tests that cover `replace_doc`.

### 3.8. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00066
  phase: Phase Z
  language: rust
```

- [ ] Run `cargo fmt`.
- [ ] Run the repository Rust quality gate relevant to the touched crates.
- [ ] Update README files on packages modified.
- [ ] Move this task to `done` only after every acceptance criterion in [[rfc-00037-extend-patch-doc-formats]] is satisfied.

## 4. Gaps, Flaws, and Tradeoffs

- **Gap:** RFC 00037 leaves the `git_diff` alias transition open; Phase A must resolve it before changing the MCP schema.
- **Flaw:** Defaulting omitted `format` to `apply_patch` can break callers that currently omit format while sending unified diffs.
- **Tradeoff:** Supporting two patch formats improves agent reliability but increases parser surface area, test requirements, and error-message maintenance.
- **Gap:** `replace_doc` must define whether it runs full document validation before writing or only enforces governed identity and relies on `validate_fix` afterward.
- **Flaw:** Full-document replacement can overwrite unrelated sections if the caller built content from stale context.
- **Tradeoff:** `replace_doc` makes bootstrap authoring simpler and more deterministic, but it needs stricter identity checks because the write surface is larger than a patch.
