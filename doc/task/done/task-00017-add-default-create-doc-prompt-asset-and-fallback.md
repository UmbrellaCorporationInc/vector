---
id: task-00017-add-default-create-doc-prompt-asset-and-fallback
type: task
code: "00017"
slug: add-default-create-doc-prompt-asset-and-fallback
title: Add Default Create Doc Prompt Asset And Fallback
description: Add a default authoring prompt asset for create_doc, provision it in new projects, and use it when a document type has no explicit prompt configured.
status: done
created: 2026-05-07
updated: 2026-05-07
tags: []
related: []
supersedes: []
superseded_by: null
---

# Task 00017: Add Default Create Doc Prompt Asset And Fallback

## 1. Prime Directive

> Remove the current coupling where `create_doc` fails when a document type omits `prompt`, by shipping a governed default prompt asset and wiring the runtime to use it deterministically.

## 2. Specs

- **Module:** `runtime/project`, `runtime/doc`, `mcp/vector`
- **Dependencies:** none

## 3. Checklist

### 3.1. Phase A - Bootstrap Default Prompt Asset

- [x] Add a governed default prompt document under `runtime/project/assets/doc/prompts/`
- [x] Provision the asset from `CreateProjectOp`
- [x] Add tests proving `create_project` writes the new asset
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Runtime Fallback Behavior

- [x] Allow document type configs without an explicit `prompt`
- [x] Make `create_doc` resolve the default prompt when the type prompt is missing or empty
- [x] Add tests covering runtime and MCP fallback behavior
- [x] execute section "4. Quality Gate"

### 3.3. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `create_project` provisions the default prompt asset
- [x] `create_doc` succeeds when a document type omits `prompt`
- [x] All quality gates pass
- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
