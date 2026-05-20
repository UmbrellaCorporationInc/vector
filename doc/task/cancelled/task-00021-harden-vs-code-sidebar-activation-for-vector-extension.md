---
id: task-00021-harden-vs-code-sidebar-activation-for-vector-extension
type: task
code: "00021"
slug: harden-vs-code-sidebar-activation-for-vector-extension
title: Harden VS Code Sidebar Activation for Vector Extension
description: Fix activation and packaging defects that prevent the Vector VS Code sidebar from loading reliably in real installations.
status: cancelled
created: 2026-05-08
updated: 2026-05-08
tags:
  - vscode
  - frontend
  - activation
  - packaging
related:
  - task-00020-implement-rfc-00014-vs-code-governed-documents-sidebar-extension
  - rfc-00014-vs-code-governed-documents-sidebar-extension
supersedes: []
superseded_by: null
---

# Task 00021: Harden VS Code Sidebar Activation for Vector Extension

## 1. Prime Directive

> Remove the failure paths that let the sidebar disappear in valid workspaces or crash after installation because runtime dependencies were not packaged into the published extension.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** VS Code extension runtime, `js-yaml` as a packaged runtime dependency

## 3. Checklist

### 3.1. Phase A - Activation Robustness

- [ ] Resolve the governed workspace root from any open workspace folder instead of assuming `workspaceFolders[0]`
- [ ] Add a fallback activation path so the extension can still initialize when `workspaceContains` does not trigger as expected
- [ ] Preserve safe behavior when no governed configuration exists in the current workspace
- [ ] Tests covering Phase A
- [ ] Validation vector for Phase A
- [ ] execute section "4. Quality Gate"

### 3.2. Phase B - Packaging Reliability

- [ ] Move every runtime-imported package out of `devDependencies` and into `dependencies`
- [ ] Remove packaging flags that exclude required runtime dependencies from the generated `.vsix`
- [ ] Update extension documentation to match the real packaging contract
- [ ] Tests covering Phase B
- [ ] Validation vector for Phase B
- [ ] execute section "4. Quality Gate"

### 3.3. Phase Z - Wrap-up

- [ ] Update README files on packages modified
- [ ] Confirm the installed extension activates without `ERR_MODULE_NOT_FOUND`
- [ ] `xtask quality-lint` passes where applicable
- [ ] `xtask quality-test` passes where applicable
- [ ] VS Code extension `pnpm run compile` passes
- [ ] VS Code extension `pnpm test` passes

## 4. Quality Gate

- [ ] VS Code extension `pnpm run compile` passes
- [ ] VS Code extension `pnpm test` passes

## 5. Validation Vector

- [ ] The sidebar becomes available in a governed workspace even when the governed root is not the first workspace folder
- [ ] The installed extension no longer throws `ERR_MODULE_NOT_FOUND` for `js-yaml`
- [ ] The generated `.vsix` contains required runtime dependencies
- [ ] All phase checkboxes completed
