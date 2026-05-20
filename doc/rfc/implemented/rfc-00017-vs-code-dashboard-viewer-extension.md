---
id: rfc-00017-vs-code-dashboard-viewer-extension
type: rfc
code: "00017"
slug: vs-code-dashboard-viewer-extension
title: VS Code Dashboard Viewer Extension
description: Adds dashboard definitions under `.vector/dashboards/`, surfaces them as root-level sidebar entries in the VS Code extension, and opens a dedicated dashboard viewer that renders per-section document widgets linked to governed document preview.
status: implemented
created: 2026-05-09
updated: 2026-05-09
authors: []
tags:
  - vscode
  - frontend
  - dashboard
  - sidebar
  - documentation
related:
  - rfc-00016-add-directory-layout-for-document-types
supersedes: []
superseded_by: null
aliases:
  - "RFC 00017: VS Code Dashboard Viewer Extension"
---

# RFC 00017: VS Code Dashboard Viewer Extension

## 1. Problem

The current VS Code extension exposes governed documents through the sidebar and governed document preview, but it has no summary surface for project-level document status.

That creates four concrete gaps:

- users cannot define reusable dashboards in project configuration
- the sidebar cannot show opinionated root-level navigation entries such as `Project Status`
- there is no dashboard-oriented viewer that aggregates governed documents by operational slices
- users must manually traverse document trees to answer common questions such as "which RFCs are pending?" or "which tasks are in progress?"

This repository already contains a real dashboard-like source file:

- `.vector/dashboards/project-status.yaml`

Example shape:

```yaml
label: Project Status
sections:
  todo-tasks:
    title: TODO tasks
    doc-type: task
    statuses: [todo]
```

Without an accepted extension contract for dashboards, this file is just inert configuration. The extension cannot turn it into a first-class navigation and reporting surface.

## 2. Proposal

Add dashboard support to the VS Code extension.

Dashboards are YAML files stored under:

- `.vector/dashboards/`

Each dashboard file becomes a root-level sidebar item. The displayed label is taken from the dashboard `label` field.

When a user selects a dashboard item, the extension opens a dedicated viewer named:

- `dashboard_viewer`

The implementation must live under:

- `frontend/vscode/vector/src/dashboard-viewer/`

This viewer is separate from the governed document viewer, but integrates with it by opening governed documents when dashboard table rows are selected.

### 2.1. Sidebar contract

The governed sidebar root must include dashboard entries discovered from `.vector/dashboards/`.

For each dashboard definition:

- the tree item label is the YAML `label`
- the tree item represents one dashboard file
- selecting the item opens `dashboard_viewer`

Dashboard items live at the root level of the sidebar, alongside other top-level extension navigation, not nested under a document type.

### 2.2. Dashboard file contract

Each dashboard YAML file defines:

- `label`
- `sections`

Each section entry defines:

- `title`
- `doc-type`

Section filters depend on the governed document layout of `doc-type`:

- `status` layout uses `statuses`
- `category` layout uses `categories`
- `directory` layout uses no grouping filter and includes all documents of that type

Example:

```yaml
label: Project Status
sections:
  todo-tasks:
    title: TODO tasks
    doc-type: task
    statuses: [todo]
  api-specs:
    title: API specs
    doc-type: spec
    categories: [api]
  project-definitions:
    title: Project Definitions
    doc-type: project
```

This RFC deliberately keeps the dashboard contract layout-aware rather than forcing one generic filter key for every document type. That avoids ambiguous configuration and keeps the file aligned with the repository layout model introduced by RFC 00016.

### 2.3. Dashboard viewer contract

Opening a dashboard shows a dedicated dashboard viewer surface.

The viewer renders one widget per dashboard section.

Each widget contains:

- the section title
- one table listing governed documents that match the section contract

Each widget is independent and must fail safely. If one section cannot be resolved, the rest of the dashboard should still render.

### 2.4. Section query semantics

Section document resolution uses the governed document type definition from `.vector/document-types.yaml` plus current `doc/` contents.

For `status`-based document types:

- `statuses` is required for a filtered section
- each status value maps to the folder name under `doc/<type>/`
- a document matches when its file exists inside one of those status folders

For `category`-based document types:

- `categories` is required for a filtered section
- each category value maps to the folder name under `doc/<type>/`
- a document matches when its file exists inside one of those category folders

For `directory`-based document types:

- the section includes every document directly under `doc/<type>/`
- `statuses` and `categories` must not be required

This means dashboard filtering is derived from the governed filesystem contract, not from ad hoc frontmatter-only matching.

### 2.5. Table contract

Each section table shows exactly two columns:

- `status`
- `slug`

Column semantics vary slightly by layout:

- for `status` layout, the `status` column shows the matched status folder name
- for `category` layout, the `status` column shows the matched category folder name even though the visual column label remains `status`
- for `directory` layout, the `status` column is empty or shows a neutral placeholder defined by the viewer contract

The `slug` column shows the governed document slug.

When the user selects the `slug` cell or row action, the extension opens the governed document in the existing document viewer.

This RFC keeps the visible table contract intentionally narrow. It optimizes for quick operational scanning rather than full metadata display.

### 2.6. Integration with governed document viewer

The dashboard viewer must reuse existing governed document resolution rules.

Clicking a dashboard document entry must:

1. resolve the governed document path
2. open the existing governed `document_viewer`
3. show the selected document there

The dashboard viewer must not duplicate governed document rendering logic.

### 2.7. Discovery and refresh behavior

The extension must discover dashboards dynamically from `.vector/dashboards/`.

Refresh behavior must:

- reread dashboard YAML files
- reread `.vector/document-types.yaml`
- rescan governed documents under `doc/`
- rebuild dashboard sidebar items and dashboard section tables from current workspace state

If a dashboard references an unknown `doc-type` or uses an invalid filter field for that layout, the extension must show a bounded error state for that section instead of crashing the viewer.

### 2.8. Scope boundaries

In scope:

- dashboard YAML discovery under `.vector/dashboards/`
- root-level sidebar items for dashboards
- a dedicated `dashboard_viewer`
- per-section widgets
- per-section document tables
- layout-aware resolution for `status`, `category`, and `directory`
- click-through integration with governed `document_viewer`

Out of scope:

- editing dashboard YAML from the UI
- drag-and-drop widget rearrangement
- dashboard write-back or persistence of UI state into YAML
- arbitrary charting, metrics, or non-document data sources
- custom dashboard filters beyond governed layout fields

## 3. Alternatives Considered

- **Render dashboards inside the existing document viewer:** Discarded because dashboards are an aggregate exploration surface, not a single-document reading surface, and mixing both concerns would overcomplicate the existing viewer.
- **Treat dashboards as another document type under `doc/`:** Discarded because dashboard definitions are extension configuration, not governed content documents, and belong under `.vector/`.
- **Use one generic `filters` field instead of `statuses` or `categories`:** Discarded because it weakens the contract, hides layout semantics, and would require runtime interpretation rules that are harder to validate.
- **Display dashboards under a nested sidebar node instead of the root:** Discarded because the request is to make dashboards first-class navigation items and root placement better reflects their cross-cutting role.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Gives the extension a project-level summary surface with low authoring cost. | Adds a second viewer surface that must be maintained alongside `document_viewer`. |
| Keeps dashboard configuration aligned with governed layout semantics. | Layout-aware parsing introduces more branching in dashboard resolution code. |
| Reuses existing governed document lookup and preview flows instead of duplicating preview rendering. | Viewer integration requires careful navigation wiring between dashboard and document experiences. |
| Root-level sidebar entries make dashboards discoverable and fast to access. | Too many dashboards could add visual noise at the sidebar root. |
| Supports `directory` layout early, keeping the extension aligned with RFC 00016. | The fixed two-column table is intentionally minimal and may be too sparse for future dashboard needs. |

## 5. Acceptance Criteria

- [ ] The VS Code extension discovers dashboard definitions from `.vector/dashboards/`.
- [ ] Each dashboard YAML file contributes one root-level sidebar item.
- [ ] Each sidebar item label is read from the dashboard `label` field.
- [ ] Selecting a dashboard item opens a dedicated `dashboard_viewer`.
- [ ] Viewer implementation lives under `frontend/vscode/vector/src/dashboard-viewer/`.
- [ ] A dashboard file supports `label` plus `sections`.
- [ ] Each section supports `title` and `doc-type`.
- [ ] Status-based sections support a `statuses` filter whose values map to folder names under `doc/<type>/`.
- [ ] Category-based sections support a `categories` filter whose values map to folder names under `doc/<type>/`.
- [ ] Directory-based sections support no grouping filter and include all documents directly under `doc/<type>/`.
- [ ] The dashboard viewer renders one widget per section.
- [ ] Each widget renders a table with exactly two visible columns: `status` and `slug`.
- [ ] Clicking a document entry in the dashboard opens the existing governed `document_viewer` for that document.
- [ ] Dashboard refresh rereads dashboard YAML, document type config, and governed document files from current workspace state.
- [ ] Invalid dashboard sections fail safely without preventing the rest of the dashboard from rendering.
- [ ] Automated tests cover dashboard discovery, YAML parsing, section filtering for `status`, `category`, and `directory`, and click-through opening into `document_viewer`.

## 6. Open Questions

- Should the `status` column for `directory` layout render as blank, `-`, or a layout-specific token such as `directory`?
- Should the `status` column for `category` layout keep the requested generic label `status`, or should the viewer adapt the header name dynamically to `category` when appropriate?
- Should dashboard root items live inside the existing governed documents tree provider, or should the extension introduce a parallel provider or container for dashboards to keep concerns cleaner?
