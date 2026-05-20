---
id: task-00020-implement-rfc-00014-vs-code-governed-documents-sidebar-extension
type: task
code: "00020"
slug: implement-rfc-00014-vs-code-governed-documents-sidebar-extension
title: Implement RFC 00014 VS Code Governed Documents Sidebar Extension
description: Implements the phased delivery of the VS Code governed documents sidebar extension defined by RFC 00014, starting with minimal plugin scaffolding only.
status: done
created: 2026-05-07
updated: 2026-05-08
tags:
  - vscode
  - frontend
  - sidebar
  - plugin
related:
  - rfc-00014-vs-code-governed-documents-sidebar-extension
  - adr-00001-adopt-an-extension-owned-governed-document-preview-for-vs-code
  - spec-00001-repository-directory-structure
  - rfc-00013-runtime-doc-validation-and-authoring-crate
supersedes: []
superseded_by: null
---

# Task 00020: Implement RFC 00014 VS Code Governed Documents Sidebar Extension

## 1. Prime Directive

> Establish a governed VS Code frontend package for documentation navigation without inventing behavior ahead of the accepted RFC, then incrementally add dynamic views, filtered trees, preview navigation, and wikilink resolution.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** VS Code extension runtime, governed document discovery adapter from `runtime/doc` or a dedicated frontend-safe integration boundary

## 3. Checklist

### 3.1. Phase A - Minimal Plugin Scaffold

- [x] Create the package directory `frontend/vscode/vector/` following the repository structure contract
- [x] Add only the minimum required VS Code extension files and metadata to make the package structurally valid
- [x] Define the extension identity, activation entrypoint, and empty baseline command or activation wiring required for later phases
- [x] Avoid introducing document views, tree providers, filters, refresh actions, Markdown Preview integration, or wikilink behavior in this phase
- [x] Add the smallest possible README or package notes required to explain the package purpose and local development entrypoints
- [x] Confirm the scaffold does not invent static document-type assumptions or duplicate governed document logic
- [x] Tests covering Phase A
- [x] Validation vector for Phase A
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Dynamic Governed View Container

- [x] Add the governed documents view container contribution
- [x] Load `.vector/document-types.yaml` and derive the set of document-type views dynamically
- [x] Scan `doc/` to establish the current governed document set for configured document types
- [x] Fail safely when configuration is missing, inconsistent, or temporarily unreadable
- [x] Tests covering Phase B
- [x] Validation vector for Phase B
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Per-Type Tree Rendering and View Actions

- [x] Implement one tree provider per dynamically discovered document type
- [x] Render tree items with code, title, and status or category when applicable
- [x] Implement `Search` per view using `InputBox` and resolve documents only inside the owning type
- [x] Accept codes without leading zeros in `Search` (e.g. `14` resolves the same as `00014`) by left-padding the raw input before lookup
- [x] When `Search` finds a document, open it in the editor in addition to revealing it in the tree
- [x] When `Search` finds no document, show an error message identifying the type and normalized code that was not found
- [x] Implement `List` per view using `QuickPick` with `All` plus supported status or category values
- [x] Implement `Refresh` per view and preserve valid active filter state across reloads
- [x] Keep sorting deterministic by numeric code and slug
- [x] Tests covering Phase C
- [x] Tests for short-code input (no leading zeros) in `Search`
- [x] Tests that a found document is opened in the editor
- [x] Tests that a missing document surfaces an error message with type and code
- [x] Validation vector for Phase C
- [x] execute section "4. Quality Gate"

### 3.4. Phase D - Clear Filters Action

- [x] Register a `vector.clearAllFilters` command that resets every per-type filter to `all`
- [x] Expose the command as a title-bar button on the governed documents view, visible only when at least one filter is active (`when` clause: `vector.hasActiveFilter`)
- [x] Set `vector.hasActiveFilter` context key to `true` whenever any filter is non-`all`, and to `false` after clearing
- [x] Update `treeView.description` immediately after clearing so the badge disappears without a manual refresh
- [x] After applying a filter via `List`, expand and reveal the filtered type's root node in the tree so the results are immediately visible
- [x] Tests for auto-expand after filter is applied
- [x] Tests covering Phase D
- [x] Validation vector for Phase D
- [x] execute section "4. Quality Gate"

### 3.5. Phase E - Markdown Preview and Wikilink Navigation

- [x] Open selected governed documents in Markdown Preview
- [x] Extend preview navigation so governed wikilinks without `.md` are clickable
- [x] Resolve wikilinks by parsing governed file name stems into document type and code
- [x] Resolve target paths within the correct governed document-type folder tree rather than by blind workspace-wide filename search
- [x] Reuse runtime document lookup behavior or introduce a narrow frontend-safe adapter if direct reuse is not yet possible
- [x] Tests covering Phase E
- [x] Validation vector for Phase E
- [x] execute section "4. Quality Gate"

#### Phase E - Bug Fix: ESM output breaks `extendMarkdownIt` (wikilinks render as plain text)

Root cause: `"type": "module"` in `package.json` combined with `module: node16` in tsconfig
emits ESM. VS Code's extension host only supports CommonJS — `extendMarkdownIt` is never
invoked, so all `[[wikilink]]` tokens pass through unmodified.

- [x] Change `tsconfig.json` `module` / `moduleResolution` from `node16` to `commonjs`
- [x] Remove `"type": "module"` from `package.json`
- [x] Remove `.js` extensions from all internal `import` statements (not needed under `moduleResolution: node`)
- [x] Update `package.json` test script: drop `--experimental-loader ./out/test/loader.js` (ESM loader no longer needed)
- [x] Confirm `pnpm run compile` and `pnpm test` pass (70/70)
- [x] Mark Phase E validation items in section 5

### 3.6. Phase F - Markdown Preview Script and Style Bridge

- [x] Replace command-URI wikilink rendering with inert preview anchors that use `href="#"` plus a `data-wikilink` attribute
- [x] Contribute `markdown.previewScripts` for temporary wikilink click handling inside Markdown Preview
- [x] Contribute `markdown.previewStyles` so governed wikilinks render as boxed white pills in preview
- [x] Add a temporary preview script that intercepts governed wikilink clicks and shows the raw wikilink value with `alert`
- [x] Keep the VS Code extension output in ESM; do not migrate this phase to CommonJS
- [x] Confirm the ESM-based Markdown Preview bridge still executes `extendMarkdownIt` correctly
- [x] Tests covering Phase F
- [x] Validation vector for Phase F
- [x] execute section "4. Quality Gate"

### 3.7. Phase Z - Wrap-up

- [x] Update README files on packages modified
- [x] Confirm the implemented behavior still matches RFC 00014 acceptance criteria
- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes

## 3.8. Follow-up Architecture Note

The governed-document preview strategy approved later in [[adr-00001-adopt-an-extension-owned-governed-document-preview-for-vs-code]] is intentionally out of scope for this task.

This task is closed against the original RFC 00014 implementation path that used native Markdown Preview plus governed wikilink extensions. Any extension-owned governed preview or editor must proceed as new work under its own RFC and task.

## 4. Quality Gate

- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes (equivalent to `xtask quality-lint`)
- [x] `cargo test --workspace` passes (equivalent to `xtask quality-test`)
- [x] VS Code extension `pnpm run compile` passes
- [x] VS Code extension `pnpm test` passes (40 tests)

## 5. Validation Vector

- [x] Phase A checkboxes completed
- [x] Phase B checkboxes completed
- [x] The package exists at `frontend/vscode/vector/` and remains structurally minimal after Phase A
- [x] Dynamic views come from `.vector/document-types.yaml` plus current `doc/` contents rather than a hardcoded registry
- [x] Per-view actions `Search`, `List`, and `Refresh` behave only within the owning document type
- [x] `vector.clearAllFilters` button is visible in the sidebar title bar only when at least one filter is active
- [x] Clearing filters resets all types to `all` and removes the description badge without requiring a manual refresh
- [x] Clicking a governed document opens Markdown Preview
- [x] Governed wikilinks resolve by type and code within the correct document-type tree
- [x] Governed wikilinks render with `href="#"` and `data-wikilink` in Markdown Preview output
- [x] Markdown Preview loads the contributed script and stylesheet for governed wikilinks
- [x] Phase F preserves ESM output and validates the preview bridge without a CommonJS migration
