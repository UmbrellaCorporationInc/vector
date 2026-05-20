---
id: task-00044-implement-vscode-navigation-history-integration
type: task
code: "00044"
slug: implement-vscode-navigation-history-integration
title: Implement VSCode Navigation History Integration
description: Fix the Vector VSCode extension so that extension-driven file opens register in VSCode's navigation history, enabling Go Back / Go Forward to work correctly.
status: done
created: 2026-05-18
updated: 2026-05-18
tags:
  - vscode
  - extension
  - user-experience
related:
  - rfc-00024-vscode-navigation-history-integration
supersedes: []
superseded_by: null
---

# Task 00044: Implement VSCode Navigation History Integration

## 1. Prime Directive

> [!Prime Directive]
> The Vector VSCode extension opens files through a custom command that bypasses VSCode's native API surface, making every extension-driven navigation invisible to the history stack. The Go Back / Go Forward buttons stay greyed out after any extension navigation. Fix this by replacing the custom command's internal implementation with `openTextDocument` + `showTextDocument` while keeping the command ID stable so no external callers break.

## 2. Specs

- **Module:** VSCode extension (`vscode-extension` package)
- **RFC:** `rfc-00024-vscode-navigation-history-integration`
- **Strategy:** Keep the custom command registration; replace its implementation body with the option 2a mechanism from the RFC.
- **Dependencies:** none — VSCode API only (`vscode.workspace.openTextDocument`, `vscode.window.showTextDocument`)

## 3. Checklist

### 3.1. Phase A — Replace command implementation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00044
  phase: Phase A
  language: TypeScript
```

- [x] Locate the custom file-open command registration in the extension source
- [x] Replace the implementation body with `openTextDocument` + `showTextDocument`, mapping existing options (column, selection, preview flag) to `TextDocumentShowOptions`
- [x] Keep the command ID and registration intact — do not remove it
- [x] Ensure all `showTextDocument` calls are properly `await`-ed

### 3.2. Phase B — Smoke-test

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00044
  phase: Phase B
  language: TypeScript
```

- [x] Open a file via the extension UI (link / reference click)
- [x] Verify **Go Back** (`Alt+Left`) returns to the previous editor location
- [x] Verify the workspace navigation buttons (top-left) are no longer permanently disabled
- [x] Verify the correct file opens with the cursor at the correct position
- [x] Verify the correct editor column is preserved where applicable

### 3.3. Phase C — Migrate Remaining Internal Actions

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00044
  phase: Phase C
  language: TypeScript
```

- [x] Update `vector.validateFix` command to use `vscode.openWith`.
- [x] Verify that validation and fixing workflows still function correctly within the Custom Editor.
- [x] Audit `extension.ts` for any other orphaned calls to `previewController.openDocument`.

### 3.4. Phase D — Decommission Legacy Preview Controller

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00044
  phase: Phase D
  language: TypeScript
```

- [x] Remove `GovernedPreviewController` instantiation from `extension.ts`.
- [x] Delete `frontend/vscode/vector/src/document-viewer/governedPreviewController.ts`.
- [x] Remove any tests specifically targeting the legacy controller that are now redundant.
- [x] Ensure `GovernedDocumentEditorProvider` is the sole source of truth for document viewing.

### 3.5. Phase Z — Update README

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00044
  phase: Phase Z
  language: TypeScript
```

- [x] Update README files on packages modified
