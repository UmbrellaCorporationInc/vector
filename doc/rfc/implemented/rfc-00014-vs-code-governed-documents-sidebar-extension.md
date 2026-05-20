---
id: rfc-00014-vs-code-governed-documents-sidebar-extension
type: rfc
code: "00014"
slug: vs-code-governed-documents-sidebar-extension
title: VS Code Governed Documents Sidebar Extension
description: Defines a VS Code extension under frontend/vscode/vector that exposes governed project documents through dynamically computed per-type sidebar views, filter commands, refresh actions, search-by-code, and wikilink-aware Markdown preview navigation.
status: implemented
created: 2026-05-07
updated: 2026-05-07
authors: []
tags:
  - vscode
  - frontend
  - documentation
  - sidebar
  - markdown
related:
  - spec-00001-repository-directory-structure
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
  - rfc-00013-runtime-doc-validation-and-authoring-crate
supersedes: []
superseded_by: null
aliases:
  - "RFC 00014: VS Code Governed Documents Sidebar Extension"
---

# RFC 00014: VS Code Governed Documents Sidebar Extension

## 1. Problem

VECTOR already defines governed project documents under `doc/` and intends to support VS Code extensions as a first-class frontend surface, but there is no editor-native document explorer for that governed documentation model.

That leaves six concrete gaps:

- governed documents cannot be browsed from a dedicated VS Code sidebar organized by document type
- users cannot filter document lists by supported status or category values without manually walking the filesystem
- users cannot jump directly to a governed document by code from the editor UI
- clicking a document does not open a documentation-focused reading flow in Markdown Preview
- governed wikilinks such as `[[rfc-00013-runtime-doc-validation-and-authoring-crate]]` are not resolvable inside VS Code Markdown Preview
- the repository has no accepted extension contract for how a VS Code frontend should discover and navigate governed documents without duplicating documentation logic ad hoc

If this remains unsolved, the documentation workflow stays split between filesystem navigation, manual search, and editor features that do not understand the governed document contract.

## 2. Proposal

Create a VS Code extension package at:

- `frontend/vscode/vector/`

This extension exposes one view container dedicated to governed documents. The container behaves as a documentation sidebar similar to a project explorer, but each governed document type is represented as its own child view.

Initial scope is read and navigation only. This RFC does not propose document editing, authoring wizards, or inline metadata mutation.

### 2.1. Sidebar structure

The extension contributes:

- one `viewsContainer` for governed documents
- one dynamically computed `TreeView` per governed document type declared in `.vector/document-types.yaml`

The set of views must be derived at runtime from two sources:

- `.vector/document-types.yaml` as the source of document type definitions and layout metadata
- the `doc/` folder tree as the source of actual governed document instances

The extension must not hardcode a fixed document-type whitelist as its primary sidebar model.

Each view title shows the document type it owns, for example:

- `RFC`
- `SPEC`
- `TASK`

Each view renders only documents of its own type.

### 2.2. View actions

Every document-type view contributes three title-bar actions:

- `Search`
- `List`
- `Refresh`

#### `Search`

The `Search` action opens a VS Code `InputBox`.

The input accepts a document code for the owning type view. Example:

- `00014` inside the `RFC` view

When the user confirms the input, the extension resolves the matching governed document for that type and code and selects it in the tree when found.

If no document exists for that type and code, the extension must show a bounded error message and keep the current tree state unchanged.

#### `List`

The `List` action opens a VS Code `QuickPick`.

The entries are driven by the document-type layout contract:

- status values for status-based types
- category values for category-based types
- `All` as a mandatory top-level option

Selecting one entry refreshes the owning tree view:

- `All` shows every document of that type
- any status or category value shows only documents of that type matching the selected value

The active filter must be visible in the view description or equivalent native view metadata so the user can tell when a filtered tree is being shown.

#### `Refresh`

The `Refresh` action is represented by a refresh icon in the view title.

When invoked, it reloads the owning view from the current workspace state.

The refresh operation must:

- reread `.vector/document-types.yaml`
- rescan the relevant governed files under `doc/`
- rebuild the tree items for the owning document type
- preserve the active filter when that filter is still valid for the current type definition

If the document type definition was removed or changed in a way that invalidates the current view, the extension must fail safely and refresh the container state accordingly.

### 2.3. Tree content

Each view contains a tree of governed documents for its document type.

Tree content must be computed from the current governed document set found under `doc/`, constrained by the layout and type definitions loaded from `.vector/document-types.yaml`.

Each tree item must display enough information to identify the document without opening it:

- code
- title
- current status or category when applicable

Sorting rules:

- primary sort by numeric code ascending
- secondary sort by slug for deterministic ordering when needed

The tree model must be derived from governed document metadata rather than inferred from arbitrary file names alone.

### 2.4. Document open behavior

When the user selects a document in the tree, the extension must:

1. open the underlying Markdown document in VS Code
2. show the document in Markdown Preview

The preview is the primary reading surface for this extension.

The tree item action must not route to a custom WebView when the native Markdown Preview can satisfy the requirement.

### 2.5. Wikilink support in Markdown Preview

Governed documents use wikilinks that reference target file names without the `.md` extension.

Example:

- `[[rfc-00013-runtime-doc-validation-and-authoring-crate]]`

The extension must extend Markdown Preview so those wikilinks become navigable inside VS Code.

Accepted behavior:

- when Markdown Preview renders a governed wikilink, clicking it resolves the target governed document and opens that document in Markdown Preview

Resolution contract:

1. parse the wikilink target as a governed file name stem
2. extract the document type prefix and numeric code from that stem
3. resolve the target document within the folder tree owned by that document type
4. open the resolved Markdown file

This resolution must respect the governed repository layout rather than performing a blind workspace-wide filename match.

### 2.6. Document discovery and resolution

The extension must not hardcode folder rules independently from the accepted documentation contract when reusable runtime behavior already exists.

The preferred integration direction is:

- reuse `runtime/doc` document-discovery behavior for document lookup by type and code
- keep VS Code-specific UI wiring inside `frontend/vscode/vector`

At minimum, the extension needs two reusable capabilities:

- list governed documents grouped by document type with their metadata
- resolve one governed document path from document type and code

The listing capability must support dynamic view computation from current configuration plus current `doc/` content rather than from a static compile-time registry.

If those capabilities are not yet exposed in a form consumable by the extension, follow-up work must introduce a frontend-safe adapter rather than embedding duplicate governance rules into the VS Code package.

### 2.7. Scope boundaries

In scope:

- view container
- per-type views
- filter quick pick
- search by code
- tree rendering
- open in Markdown Preview
- wikilink navigation from Markdown Preview

Out of scope:

- authoring new documents
- editing frontmatter from the sidebar
- drag-and-drop document moves
- non-governed markdown discovery
- Obsidian-specific UI behavior

## 3. Alternatives Considered

- **Single unified tree view:** Discarded because mixing all document types into one tree weakens the governed-type model and makes per-type actions, filters, and mental ownership less clear.
- **Custom WebView reader instead of Markdown Preview:** Discarded because it duplicates Markdown rendering behavior, increases maintenance cost, and bypasses native editor affordances.
- **Workspace-wide filename search for wikilinks:** Discarded because it ignores the governed layout contract and can resolve the wrong file if names ever collide outside the governed tree.
- **Full document scanning logic implemented only inside the extension:** Discarded because it duplicates governance knowledge that should remain reusable across MCP, CLI, and VS Code frontends.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Uses native VS Code views and Markdown Preview instead of building a custom reader UI. | Native Markdown Preview extension points may constrain how rich wikilink behavior can become. |
| Keeps the sidebar aligned with governed document types and their layout semantics. | One view per type can create vertical UI noise if the number of document types grows significantly. |
| Search by code matches the repository naming contract and is efficient for governed docs. | Code-only search is less forgiving than full-text search and depends on users knowing the document code. |
| Reusing runtime document lookup reduces duplicated rules across frontends. | It may require additional adapter work before the VS Code package can consume the runtime capability cleanly. |
| Opening documents in Markdown Preview creates a reading-first workflow with minimal custom UI. | Some users may still expect side-by-side source and preview synchronization that needs extra command wiring. |
| Dynamic view computation keeps the sidebar consistent with real repository configuration and document state. | Refresh and dynamic recomputation add state-management complexity when configuration changes during an active session. |

## 5. Acceptance Criteria

- [ ] A VS Code extension package exists at `frontend/vscode/vector/`.
- [ ] The extension contributes one governed-documents view container.
- [ ] The container renders one child view per document type declared in `.vector/document-types.yaml`.
- [ ] The extension computes views dynamically from `.vector/document-types.yaml` and the current `doc/` folder contents.
- [ ] Each view exposes a `Search` action that opens an `InputBox`.
- [ ] Entering a numeric code in `Search` resolves a document only within the owning document type.
- [ ] Each view exposes a `List` action that opens a `QuickPick`.
- [ ] Each view exposes a refresh icon action that reloads its document list from current configuration and filesystem state.
- [ ] The `QuickPick` includes `All` plus every supported status or category for that document type.
- [ ] Choosing a `QuickPick` item refreshes the tree so only matching documents are shown.
- [ ] Each tree item displays document code and title, plus status or category when applicable.
- [ ] Selecting a tree item opens the Markdown file and shows it in Markdown Preview.
- [ ] Markdown Preview supports navigation for governed wikilinks that omit the `.md` extension.
- [ ] Wikilink resolution derives document type and code from the governed file name stem and resolves the target within the correct document-type folder tree.
- [ ] The extension does not rely on blind workspace-wide file search as its primary governed document resolution strategy.
- [ ] Shared document lookup behavior is reused from runtime code or exposed through a dedicated adapter rather than reimplemented ad hoc in the VS Code package.

## 6. Open Questions

- Should clicking a tree item always replace the active preview, or should it open beside the current editor by default?
- Should the extension support reverse navigation from preview back to the tree selection state on every wikilink click?
- What is the thinnest integration boundary that lets `frontend/vscode/vector` consume governed document discovery from `runtime/doc` without introducing a frontend-specific dependency leak?
