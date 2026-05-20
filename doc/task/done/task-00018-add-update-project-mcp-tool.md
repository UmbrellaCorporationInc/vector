---
id: task-00018-add-update-project-mcp-tool
type: task
code: "00018"
slug: add-update-project-mcp-tool
title: Add Update Project MCP Tool
description: Add an `update_project` tool that runs `ProjectSetupOp` with `force: false` so it provisions missing assets into an existing project without overwriting anything.
status: done
created: 2026-05-07
updated: 2026-05-07
tags: []
related: []
supersedes: []
superseded_by: null
---

# Task 00018: Add Update Project MCP Tool

## 1. Prime Directive

> There is no way to bring an existing project up to date with the latest governed assets. `create_project` skips existing files, but there is no dedicated tool to run that same operation on an already-initialized project. `update_project` fills that gap by calling `ProjectSetupOp` with `force: false`, adding any asset that is absent while leaving every existing file untouched.

## 2. Specs

- **Module:** `mcp/vector/src/tools/project.rs`
- **Dependencies:** `runtime_project::{ProjectSetupInput, ProjectSetupOp}` (already imported)

## 3. Checklist

### 3.1. Phase A — MCP Tool

- [x] Add `UpdateProjectParams` struct (`target_dir: String`, `project_name: String`) — no `force` field, always `false`
- [x] Add `update_project` tool method to `ProjectTools` that calls `ProjectSetupOp` with `force: false`
- [x] Add unit tests in `mcp/vector/src/tools/project_test.rs` covering:
  - existing files are not overwritten
  - missing files are created
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
- [x] `update_project` tool is callable via MCP and provisions missing assets
- [x] Existing files in the target directory remain unmodified after the call
- [x] All quality gates pass
