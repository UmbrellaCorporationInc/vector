---
id: task-00034-implement-rfc-00020-create-document-actions-in-vscode-treeview
type: task
code: "00034"
slug: implement-rfc-00020-create-document-actions-in-vscode-treeview
title: Implement RFC 00020 Create Document Actions in VS Code Treeview
description: Add per-doc-type and global create actions to the VS Code governed documents tree, backed by configured create-form documents and viewer-based opening flows.
status: done
created: 2026-05-12
updated: 2026-05-12T12:30:00Z
tags:
  - vscode
  - treeview
  - viewer
  - forms
related:
  - rfc-00020-add-create-document-actions-to-vscode-treeview-doc-type-folders
  - rfc-00018-enhanced-markdown-code-blocks
  - task-00028-enhanced-markdown-code-blocks
supersedes: []
superseded_by: null
---

# Task 00034: Implement RFC 00020 Create Document Actions in VS Code Treeview

## 1. Prime Directive

> [!Prime Directive]
> The VS Code governed documents tree currently lets users browse and open existing documents but provides no creation entry point at either the selected `document-types.<type>` level or the global `doc-type` level. This task adds both create flows, resolves their configured source documents from `.vector/document-types.yaml`, and opens them through the existing governed `document_viewer` without mutating governed source files.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Primary areas:** governed documents tree provider/items, document type config loading, governed preview controller, viewer opening helpers, extension commands, extension tests, `runtime/doc` validate operation
- **Dependencies:** `.vector/document-types.yaml`, existing governed `document_viewer`, RFC 00018 embedded form and action block behavior, `runtime/doc` validation contract
- **Boundary:** keep create-flow orchestration in the VS Code extension; do not move viewer rendering or markdown interaction logic out of the existing viewer pipeline
- **Behavior split:** per-doc-type `Create Document` instantiates a temp file and replaces `#{document-type}`; global `Create Document Type` opens the configured source document unchanged
- **Governance requirement:** `runtime/doc` validation must fail when `document-types.<type>.create-document-form` or `doc-type.create-document-type-form` is missing

## 3. Checklist

### 3.1. Phase A - Extend Config Contracts and Resolution Helpers

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00034
  phase: Phase A
  language: typescript
```

- [x] Extend the document-types config model to support optional `create-document-form` on `document-types.<type>`
- [x] Extend the global `doc-type` config model to support required `create-document-type-form`
- [x] Add or update config parsing and validation helpers needed by the extension create flows
- [x] Add a shared resolver that maps configured governed identifiers to exactly one source document path
- [x] Keep missing optional config values non-fatal so tree rendering still works
- [x] Quality gates pass for config and resolver changes

### 3.2. Phase B - Add Treeview Commands and Action Surfaces

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00034
  phase: Phase B
  language: typescript
```

- [x] Add a per-doc-type `Create Document` action to governed tree items when `create-document-form` is configured
- [x] Add a global `Create Document Type` action to the governed documents tree title or equivalent global action surface when `doc-type.create-document-type-form` is configured
- [x] Register dedicated extension commands for both flows
- [x] Ensure the per-doc-type action appears only on doc-type folder items, not on grouping nodes or leaf documents
- [x] Ensure the global action does not depend on a selected tree item
- [x] Surface bounded user-visible errors when configured source documents cannot be resolved
- [x] Quality gates pass for tree and command wiring changes

### 3.3. Phase C - Open Create Forms Through the Existing Viewer

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00034
  phase: Phase C
  language: typescript
```

- [x] Implement per-doc-type create-form instantiation into an extension-controlled temp markdown file
- [x] Replace `#{document-type}` in the temp content before opening the instantiated document
- [x] Preserve the original governed create-form source document unchanged on disk
- [x] Open the instantiated temp document in the existing governed `document_viewer`
- [x] Implement the global document-type create flow so it opens the configured source document directly in the existing governed `document_viewer`
- [x] Avoid placeholder replacement in the global `Create Document Type` flow
- [x] Keep embedded `vector-form`, `vector-agent-button`, and `vector-agent-action` behavior working through the existing viewer contract
- [x] Quality gates pass for viewer integration changes

### 3.4. Phase D - Enforce Create-Form Fields in `runtime/doc` Validation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00034
  phase: Phase D
  language: rust, typescript
```

- [x] Update the `runtime/doc` validate operation so it fails when any `document-types.<type>` entry omits `create-document-form`
- [x] Update the `runtime/doc` validate operation so it fails when `doc-type.create-document-type-form` is missing
- [x] Ensure validation errors clearly point to `.vector/document-types.yaml` and the missing required field
- [x] Preserve existing validation behavior for unrelated governed document contracts
- [x] Add or update runtime validation tests covering both missing-field failures
- [x] Quality gates pass for the `runtime/doc` validation changes

### 3.5. Phase E - Cover the Flows with Tests

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00034
  phase: Phase E
  language: typescript
```

- [x] Add tests for config parsing of `create-document-form` and `create-document-type-form`
- [x] Add tests for per-doc-type action visibility on eligible and ineligible doc-type folders
- [x] Add tests for global `Create Document Type` action visibility
- [x] Add tests that the per-doc-type flow resolves the configured document, writes a temp file, and substitutes `#{document-type}`
- [x] Add tests that the global flow resolves the configured document and opens it without substitution
- [x] Add tests for bounded error handling when configured create-form documents are missing or ambiguous
- [x] Quality gates pass for the extension test suite changes

### 3.6. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00034
  phase: Phase Z
  language: typescript
```

- [x] Mark all implemented checklist items complete
- [x] Update extension documentation or configuration references affected by the new create actions
- [x] Confirm the implementation still reuses the existing governed `document_viewer` rather than introducing a parallel create-form renderer
- [x] Run the final quality gates affected by this task

## 4. Quality Gate

- [x] VS Code extension tests covering both create flows pass
- [x] `runtime/doc` validation tests covering the required create-form fields pass
- [x] Per-doc-type create actions instantiate temp content and replace `#{document-type}` deterministically
- [x] Global document-type creation opens the configured form unchanged
- [x] Repository validation fails fast when required create-form config fields are missing
- [x] Broken create-form configuration fails locally without breaking the governed treeview

## 5. Validation Vector

- [x] `.vector/document-types.yaml` config parsing supports both `create-document-form` and `doc-type.create-document-type-form`
- [x] `runtime/doc` validation rejects missing `document-types.<type>.create-document-form` and missing `doc-type.create-document-type-form`
- [x] Eligible doc-type folders expose `Create Document` in the governed documents tree
- [x] The governed documents tree exposes a global `Create Document Type` action when configured
- [x] The per-doc-type flow resolves one governed source document, writes a temp markdown file, and substitutes `#{document-type}`
- [x] The global document-type flow resolves one governed source document and opens it without substitution
- [x] Both flows open through the existing governed `document_viewer`
- [x] Embedded forms and actions in the opened documents remain compatible with the RFC 00018 viewer contract
