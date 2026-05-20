---
id: task-00027-implement-rfc-00017-vs-code-dashboard-viewer-extension
type: task
code: "00027"
slug: implement-rfc-00017-vs-code-dashboard-viewer-extension
title: Implement RFC 00017 VS Code Dashboard Viewer Extension
description: Implements dashboard discovery, root-level sidebar items, and a dedicated dashboard viewer in the VS Code extension with governed document click-through and layout-aware section filtering.
status: done
created: 2026-05-09
updated: 2026-05-09
tags:
  - vscode
  - frontend
  - dashboard
  - sidebar
related:
  - rfc-00017-vs-code-dashboard-viewer-extension
  - rfc-00016-add-directory-layout-for-document-types
supersedes: []
superseded_by: null
---

# Task 00027: Implement RFC 00017 VS Code Dashboard Viewer Extension

## 1. Prime Directive

> Turn dashboard YAML files under `.vector/dashboards/` into first-class VS Code navigation and reporting surfaces without duplicating governed document resolution logic or regressing the existing `document_viewer`.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** RFC 00017 accepted contract, RFC 00016 `directory` layout support, existing governed sidebar discovery and `document_viewer` navigation flows

## 3. Checklist

### 3.1. Phase A - Dashboard Contract and Discovery

- [x] Add dashboard model and parsing support for YAML files under `.vector/dashboards/`
- [x] Validate the dashboard contract for `label`, `sections`, `title`, and `doc_type`
- [x] Distinguish allowed layout-aware filters so `statuses` is used for `status`, `categories` for `category`, and no grouping filter is required for `directory`
- [x] Keep invalid or unknown section definitions representable as bounded section errors instead of fatal extension failures
- [x] Add tests covering dashboard discovery, YAML parsing, and invalid contract handling
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Sidebar Integration

- [x] Extend the governed sidebar root so each dashboard file contributes one root-level item
- [x] Read the dashboard tree item label from the YAML `label` field
- [x] Keep dashboard entries alongside existing top-level navigation instead of nesting them under document types
- [x] Ensure refresh rebuilds dashboard root items from current workspace state
- [x] Add tests covering root-level dashboard items and refresh behavior
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Dashboard Section Resolution

- [x] Resolve dashboard sections against `.vector/document-types.yaml` plus current `doc/` contents
- [x] Implement `status` layout section filtering by status folder names under `doc/<type>/`
- [x] Implement `category` layout section filtering by category folder names under `doc/<type>/`
- [x] Implement `directory` layout section resolution for all documents directly under `doc/<type>/`
- [x] Preserve deterministic row shaping so tables expose only `status` and `slug` with safe handling for `directory` rows
- [x] Add tests covering section filtering for `status`, `category`, and `directory`
- [x] execute section "4. Quality Gate"

### 3.4. Phase D - Dashboard Viewer and Navigation

- [x] Create the dedicated `dashboard_viewer` implementation under `frontend/vscode/vector/src/dashboard-viewer/`
- [x] Render one widget per dashboard section with isolated failure boundaries per section
- [x] Render each widget as a two-column table with exactly `status` and `slug`
- [x] Reuse governed document lookup rules so clicking a dashboard row opens the existing `document_viewer`
- [x] Avoid duplicating governed document rendering logic inside the dashboard viewer
- [x] Add tests covering viewer rendering, section-level error containment, and click-through navigation into `document_viewer`
- [x] execute section "4. Quality Gate"

### 3.5. Phase Z - Wrap-up

- [x] Update README files on packages modified
- [x] Confirm the implementation still matches RFC 00017 acceptance criteria
- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `pnpm run compile` passes
- [x] `pnpm run test` passes
- [x] `pnpm run lint` passes
## 5. Validation Vector

- [x] The VS Code extension discovers dashboards from `.vector/dashboards/`
- [x] Each dashboard file contributes one root-level sidebar item labeled from YAML `label`
- [x] Selecting a dashboard item opens the dedicated `dashboard_viewer`
- [x] The viewer implementation lives under `frontend/vscode/vector/src/dashboard-viewer/`
- [x] Dashboard sections support `title` and `doc_type` with layout-aware `statuses` or `categories` filters where applicable
- [x] `directory` layout sections include all governed documents directly under `doc/<type>/`
- [x] Each section renders a table with exactly `status` and `slug`
- [x] Clicking a dashboard document entry opens the existing governed `document_viewer`
- [x] Refresh rereads dashboard YAML, document type config, and governed documents from current workspace state
- [x] Invalid sections fail safely without blocking the rest of the dashboard
- [x] All phase checkboxes completed

