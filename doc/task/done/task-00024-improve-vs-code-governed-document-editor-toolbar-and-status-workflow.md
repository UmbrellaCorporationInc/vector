---
id: task-00024-improve-vs-code-governed-document-editor-toolbar-and-status-workflow
type: task
code: "00024"
slug: improve-vs-code-governed-document-editor-toolbar-and-status-workflow
title: Improve VS Code Governed Document Editor Toolbar and Status Workflow
description: Add governed preview toolbar actions for table-of-contents navigation and direct VS Code markdown editing, plus status-aware frontmatter editing with safe document relocation.
status: done
created: 2026-05-08
updated: 2026-05-08
tags:
  - vscode
  - frontend
  - preview
  - editor
related:
  - rfc-00014-vs-code-governed-documents-sidebar-extension
  - task-00022-implement-rfc-00015-extension-owned-governed-document-preview-for-vs-code
  - task-00023-improve-vs-code-governed-documents-tree-lazy-hierarchy-and-accordion-behavior
supersedes: []
superseded_by: null
---

# Task 00024: Improve VS Code Governed Document Editor Toolbar and Status Workflow

## 1. Prime Directive

> Remove preview-only friction in the governed document viewer by adding direct navigation and edit actions, then make status-based frontmatter editable without leaving documents in an inconsistent folder layout.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** VS Code extension runtime, governed document discovery/config loading, preview webview shell, local filesystem rename semantics on the same workspace volume

## 3. Checklist

### 3.1. Phase A - Preview Toolbar and Heading Navigation

- [x] Add a preview toolbar in the governed webview shell
- [x] Extract document headings into a table of contents model with stable anchors
- [x] Add a toolbar action that opens or toggles a table of contents panel and navigates to headings inside the current document
- [x] Preserve governed wikilink and frontmatter-link behavior after the toolbar is added
- [x] Tests covering Phase A
- [x] Execute section "4. Quality Gate"

### 3.2. Phase B - Open in VS Code Markdown Editor

- [x] Add a toolbar action that opens the current governed file in the VS Code text editor for markdown editing
- [x] Preserve the current preview panel so the user can return to the governed render without reopening from the tree
- [x] Refresh the preview when the underlying text document changes on disk
- [x] Tests covering Phase B
- [x] Execute section "4. Quality Gate"

### 3.3. Phase C - Editable Status Frontmatter with Safe Relocation

- [x] Detect when the current governed document type uses `status` layout
- [x] Render the frontmatter `status` field as an editable control only for status-based document types
- [x] Restrict available status values to the configured document type contract
- [x] When the user changes status, update the frontmatter `status` value and relocate the document into the matching status folder
- [x] Ensure the relocation uses an atomic rename for the folder move on the current workspace volume and fails safely without silent data loss
- [x] Refresh tree and preview state after a successful status change
- [x] Tests covering Phase C
- [x] Execute section "4. Quality Gate"

### 3.4. Phase D - Native Editor Title Bar Actions

- [x] Register `vector.previewToggleToc` and `vector.previewOpenEditor` commands in `package.json` with codicon icons
- [x] Expose a `vector.governedPreviewActive` context key that is `true` when the governed preview panel is open and focused, `false` when closed
- [x] Contribute both commands to the `editor/title` menu gated on `vector.governedPreviewActive`
- [x] Wire `vector.previewToggleToc` to post the existing `toggle-toc` message to the webview
- [x] Wire `vector.previewOpenEditor` to invoke the existing `openCurrentDocumentInEditor` logic
- [x] Tests covering Phase D
- [x] Execute section "4. Quality Gate"

### 3.5. Phase E - Remove Internal Webview Toolbar Buttons and Auto-Close TOC

- [x] Remove the `<div class="vector-preview-toolbar">` block from `buildToolbarHtml` in `previewHtml.ts`
- [x] Remove the `vector-preview-toolbar` and `vector-toolbar-button` CSS rules from `preview.css`
- [x] Remove the `open-editor` message handler from `preview.js`; keep `toggle-toc` handler for the native button message bridge
- [x] Close the TOC panel when the user clicks anywhere outside it or on any item inside it
- [x] Verify that TOC and open-editor flows still work exclusively through the native title bar buttons
- [x] Tests covering Phase E
- [x] Execute section "4. Quality Gate"

### 3.6. Phase G - Native Refresh Button with Stem Re-Resolution

- [x] Register `vector.previewRefresh` command in `package.json` with `$(refresh)` icon
- [x] Contribute it to `editor/title` gated on `activeWebviewPanelId == vectorGovernedPreview`
- [x] Implement `refreshByStem` in `GovernedPreviewController`: re-resolve the document by type and code against the workspace (picks up folder moves), fall back to path-based read if stem is incomplete, show an error if the document cannot be found
- [x] Wire the command in `extension.ts`
- [x] Tests covering Phase G
- [x] Execute section "4. Quality Gate"

### 3.7. Phase F - TOC Button Scope and Floating TOC Panel

- [x] Restrict `vector.previewToggleToc` and `vector.previewOpenEditor` in `editor/title` to `activeWebviewPanelId == vectorGovernedPreview` so buttons only appear when the governed preview panel is active
- [x] Make the TOC panel position fixed so it stays visible during scroll
- [x] Tests covering Phase F
- [x] Execute section "4. Quality Gate"

### 3.7. Phase Z - Wrap-up

- [x] `pnpm run compile` passes
- [x] `pnpm test` passes
- [ ] `cargo xtask lint --markdown` passes
- [ ] `cargo xtask vault check --fix` passes
- [x] Update README files on packages modified when required

## 4. Quality Gate

- [x] `pnpm run compile` passes
- [x] `pnpm test` passes

## 5. Validation Vector

- [x] Toolbar renders inside the governed preview without breaking existing navigation
- [x] TOC navigation jumps to the selected heading in the current document
- [x] Open-in-editor action opens the same governed markdown file in the VS Code text editor
- [x] Status-based documents expose only configured status values
- [x] Changing status updates frontmatter and moves the file into the matching folder
- [x] Failed status changes do not silently lose the original file
- [ ] All phase checkboxes completed

## 6. Execution Notes

- The highest-risk area is the status mutation flow because content update and path move cannot be expressed as a single filesystem transaction with the extension APIs; the implementation must still guarantee an atomic folder rename step and safe failure behavior.
- TOC anchors must be deterministic across rerenders or the webview navigation button will degrade into unstable scroll targets.
- Validation note (2026-05-08): VS Code extension compile and test passed through direct `node.exe` invocation because the active `pnpm` shell path is affected by a broken PowerShell profile and missing `node` on `PATH`.
- Validation note (2026-05-08): `cargo xtask lint --markdown` and `cargo xtask vault check --fix` remain blocked in this session because `cargo` reports `no such command: xtask`.
