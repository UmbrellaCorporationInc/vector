---
id: task-00026-implement-rfc-00016-add-directory-layout-for-document-types
type: task
code: "00026"
slug: implement-rfc-00016-add-directory-layout-for-document-types
title: Implement RFC 00016 Add Directory Layout for Document Types
description: Implements the new `directory` document-type layout across runtime configuration, document bootstrap and validation, MCP-facing contracts, and the VS Code governed documents sidebar.
status: done
created: 2026-05-09
updated: 2026-05-09
tags:
  - runtime
  - vscode
  - mcp
  - layout
related:
  - rfc-00016-add-directory-layout-for-document-types
  - rfc-00017-vs-code-dashboard-viewer-extension
supersedes: []
superseded_by: null
---

# Task 00026: Implement RFC 00016 Add Directory Layout for Document Types

## 1. Prime Directive

> Remove the current structural assumption that every governed document type must be grouped by `status` or `category`, and make `directory` a first-class layout across runtime, MCP, and VS Code without regressing existing governed behavior.

## 2. Specs

- **Module:** `runtime/doc`, `mcp/vector`, `frontend/vscode/vector`
- **Dependencies:** RFC 00016 accepted contract, existing governed document discovery and validation flows

## 3. Checklist

### 3.1. Phase A - Runtime Layout Contract

- [x] Extend document-type config loading so `layout: directory` is accepted as a valid governed layout
- [x] Add explicit runtime helpers or equivalent logic to distinguish `status`, `category`, and `directory`
- [x] Keep `statuses` required only for `status` layouts
- [x] Keep `initial-status` meaningful only for `status` layouts
- [x] Add tests covering config loading and layout discrimination for `directory`
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Bootstrap and Lookup Behavior

- [x] Update document bootstrap path derivation so `directory` documents are created directly under `doc/<type>/`
- [x] Ensure `create_doc` and `create_doc_prompt` work for `directory` document types without category input
- [x] Ensure governed lookup by stem or by `{type, code}` continues to resolve `directory` documents correctly
- [x] Add tests covering bootstrap path derivation and lookup behavior for `directory`
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Validation and Auto-Fix

- [x] Update validation so `directory` documents do not require `status` or `category`
- [x] Validate that `directory` documents live directly under `doc/<type>/`
- [x] Update `validate_fix` so misplaced `directory` documents can be flattened back to `doc/<type>/` when the correction is unambiguous
- [x] Preserve existing validation behavior for `status` and `category`
- [x] Add tests covering valid placement, invalid nested placement, and safe auto-fix behavior for `directory`
- [x] execute section "4. Quality Gate"

### 3.4. Phase D - MCP Contract Updates

- [x] Update MCP-facing layout descriptions and schemas so `directory` is a supported layout value wherever layout is described
- [x] Ensure document-type creation flows accept `directory` without requiring `statuses`
- [x] Add tests covering MCP adapter behavior for `directory`
- [x] execute section "4. Quality Gate"

### 3.5. Phase E - VS Code Governed Sidebar Support

- [x] Update governed document discovery in the VS Code extension to scan files directly under `doc/<type>/` for `directory` layouts
- [x] Render `directory` document-type roots as flat document lists with no intermediate group nodes
- [x] Render `directory` document items without status or category badges
- [x] Keep search by code working for `directory` document types
- [x] Keep `List` behavior safe for `directory` document types without inventing synthetic filter values beyond `All`
- [x] Add tests covering discovery, tree rendering, filtering behavior, and search for `directory`
- [x] execute section "4. Quality Gate"

### 3.6. Phase Z - Wrap-up

- [x] Update README files on packages modified
- [x] Confirm the implementation still matches RFC 00016 acceptance criteria
- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] `.vector/document-types.yaml` accepts `layout: directory`
- [x] `create_doc_type` and bootstrap flows accept `directory` without `statuses`
- [x] `directory` documents are created at `doc/<type>/<type>-<code>-<slug>.md`
- [x] Validation no longer requires `status` or `category` for `directory` documents
- [x] `validate_fix` safely repairs misplaced `directory` documents when the target path is unambiguous
- [x] Existing `status` and `category` behavior remains unchanged
- [x] MCP-facing layout contracts include `directory`
- [x] VS Code discovery lists `directory` documents directly under `doc/<type>/`
- [x] VS Code tree roots for `directory` layouts expand directly to document items with no synthetic groups
- [x] VS Code search and list interactions remain safe and deterministic for `directory` document types
- [x] All phase checkboxes completed
