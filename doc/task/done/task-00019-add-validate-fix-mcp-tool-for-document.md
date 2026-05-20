---
id: task-00019-add-validate-fix-mcp-tool-for-document
type: task
code: "00019"
slug: add-validate-fix-mcp-tool-for-document
title: Add Validate Fix MCP Tool For Document
description: Add a `validate_fix` MCP tool in the document tool group that calls the existing validate operation with `fix: true`, so callers can apply auto-fixes without passing an explicit flag.
status: done
created: 2026-05-07
updated: 2026-05-07
tags: []
related: []
supersedes: []
superseded_by: null
---

# Task 00019: Add Validate Fix MCP Tool For Document

## 1. Prime Directive

> There is no dedicated MCP tool to apply doc auto-fixes — callers must know to pass `fix: true` to `validate`. `validate_fix` exposes that intent explicitly, keeping `validate` as a read-only audit tool and giving fix mode a clear, discoverable entry point.

## 2. Specs

- **Module:** `mcp/vector/src/tools/document.rs`
- **Dependencies:** `runtime_doc::{ValidateInput, ValidateOp}` (already imported)

## 3. Checklist

### 3.1. Phase A — MCP Tool

- [x] Add `validate_fix` tool method to `DocumentTools` that calls `ValidateOp` with `fix: true`
- [x] Reuse `ValidateParams` struct (only `root_dir` needed — no `fix` field exposed)
- [x] Add unit tests in `mcp/vector/src/tools/document_test.rs` covering:
  - auto-fixes are applied when `validate_fix` is called
  - `validate` (fix: false) remains unaffected
- [x] execute section "4. Quality Gate"

### 3.2. Phase Z — Wrap-up

- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes (equivalent to `xtask quality-lint`)
- [x] `cargo test --workspace` passes (equivalent to `xtask quality-test`)
- [x] `cargo fmt --check` passes (equivalent to `xtask quality --format`)
- [x] Update README for `mcp/vector`

## 4. Quality Gate

- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [x] `cargo test --workspace` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `validate_fix` tool is callable via MCP and applies auto-fixes
- [x] `validate` tool behavior is unchanged
- [x] All quality gates pass
