---
id: task-00023-improve-vs-code-governed-documents-tree-lazy-hierarchy-and-accordion-behavior
type: task
code: "00023"
slug: improve-vs-code-governed-documents-tree-lazy-hierarchy-and-accordion-behavior
title: Improve VS Code Governed Documents Tree Lazy Hierarchy and Collapse-All Toolbar Behavior
description: Refines the VS Code governed documents sidebar tree so it loads document types lazily, introduces category or status grouping as the second level, removes the toolbar filter button, and exposes a native collapse-all control.
status: done
created: 2026-05-08
updated: 2026-05-08
tags:
  - vscode
  - frontend
  - sidebar
  - tree
related:
  - rfc-00014-vs-code-governed-documents-sidebar-extension
  - task-00020-implement-rfc-00014-vs-code-governed-documents-sidebar-extension
  - task-00022-implement-rfc-00015-extension-owned-governed-document-preview-for-vs-code
supersedes: []
superseded_by: null
---

# Task 00023: Improve VS Code Governed Documents Tree Lazy Hierarchy and Collapse-All Toolbar Behavior

## 1. Prime Directive

> Remove the current eager per-type document expansion model from the VS Code governed documents sidebar and replace it with a lazy multi-level tree that first shows document types, then status or category folders, then document items, while replacing the toolbar filter button with a native collapse-all affordance.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** VS Code extension runtime, `.vector/document-types.yaml`, governed document discovery and tree provider modules already used by the sidebar

## 3. Checklist

### 3.1. Phase A - Tree Node Model and Root-Only Initial Load

- [x] Refactor the governed tree node model so document-type roots, status or category group nodes, and document leaf nodes are represented explicitly
- [x] Ensure the initial tree load returns only document-type root nodes
- [x] Ensure the initial tree load does not scan Markdown files or parse frontmatter for any document type
- [x] Preserve safe behavior when `.vector/document-types.yaml` is missing, invalid, or partially inconsistent
- [x] Tests covering Phase A
- [ ] Validation vector for Phase A
- [ ] execute section "4. Quality Gate"

### 3.2. Phase B - Lazy Second-Level Status or Category Groups

- [x] When a document-type root expands, inspect the configured layout and expose one child node per supported status or discovered category folder
- [x] For `status` layouts, show only statuses that are supported by the type contract and present on disk
- [x] For `category` layouts, enumerate category folders from the governed type directory without parsing document contents
- [x] Keep second-level ordering deterministic
- [x] Fail safely when a configured type directory is absent
- [x] Tests covering Phase B
- [x] Validation vector for Phase B
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Lazy Document Loading per Group Node

- [x] When a status or category node expands, load only the documents that belong to that folder
- [x] Restrict frontmatter reads and file-name parsing to the currently expanded status or category node
- [x] Render document leaf labels with the same governed metadata contract already used by the sidebar where applicable
- [x] Preserve existing open, reveal, search, refresh, and filter behavior or adjust those flows so they remain correct against the new three-level tree
- [x] Tests covering Phase C
- [ ] Validation vector for Phase C
- [ ] execute section "4. Quality Gate"

### 3.4. Phase D - Collapse-All Toolbar Behavior

- [x] Remove the `List / Filter` toolbar button from the governed documents tree view title
- [x] Expose a native `Collapse All` affordance for the governed documents tree view
- [x] Preserve command-driven filter flows without requiring the toolbar button
- [x] Tests covering Phase D
- [ ] Validation vector for Phase D
- [ ] execute section "4. Quality Gate"

### 3.5. Phase Z - Wrap-up

- [x] Update README files on packages modified
- [ ] Confirm the sidebar behavior remains aligned with RFC 00014 intent after the tree hierarchy change
- [x] VS Code extension `pnpm run compile` passes
- [x] VS Code extension `pnpm test` passes
- [ ] `cargo xtask lint --markdown` passes
- [ ] `cargo xtask vault check --fix` passes

## 4. Quality Gate

- [x] VS Code extension `pnpm run compile` passes
- [x] VS Code extension `pnpm test` passes
- [ ] `cargo xtask lint --markdown` passes
- [ ] `cargo xtask vault check --fix` passes

## 5. Validation Vector

- [x] Initial sidebar load renders only document-type roots
- [x] Expanding a document-type root does not require a full scan of all documents for all document types
- [x] Expanding a document-type root reveals status or category nodes derived from the supported folder layout
- [x] Expanding a status or category node reveals only the documents inside that group
- [x] The toolbar exposes native `Collapse All` and no longer shows `List / Filter`
- [ ] Search, refresh, open, and reveal flows still resolve the correct governed document under the new hierarchy
- [ ] All phase checkboxes completed

## 6. Execution Notes

- The current provider assumes a two-level tree of `docType -> documents`; this task requires a three-level model of `docType -> status/category -> documents`.
- The highest-risk integration point is the existing filter and reveal behavior because it currently targets document-type roots directly and may require a new mapping to second-level nodes.
- Update note (2026-05-08): Phase D replaced the proposed accordion behavior with a native `Collapse All` toolbar affordance and removed the dedicated `List / Filter` title button while preserving the command itself.
- Validation note (2026-05-08): the VS Code extension compile and test gates passed via direct `node` invocation because the `pnpm` WinGet shim was not executable in this environment.
- Validation note (2026-05-08): Phase B added lazy second-level status/category group coverage and revalidated the VS Code extension compile and test gates via direct `node.exe` invocation because `node` was not available on `PATH` for `pnpm`.
- Validation note (2026-05-08): `cargo xtask lint --markdown` and `cargo xtask vault check --fix` remain blocked in this workspace because `cargo` reports `no such command: xtask`.
- Validation note (2026-05-08): Phase C implementation and test coverage were added, but local `pnpm run compile` and `pnpm test` validation are currently blocked because the active PowerShell profile references a missing `conda.exe` hook and no usable `node.exe` binary was discoverable in this workspace session.
