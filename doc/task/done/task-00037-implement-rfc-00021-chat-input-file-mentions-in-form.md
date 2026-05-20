---
id: task-00037-implement-rfc-00021-chat-input-file-mentions-in-form
type: task
code: "00037"
slug: implement-rfc-00021-chat-input-file-mentions-in-form
title: Implement RFC 00021 Chat Input File Mentions In Form
description: Replace the current textarea-backed `chat-input` with a document-viewer-scoped editor that supports file mentions, Markdown-aware styling, dynamic growth, and first-iteration string-only execution compatibility.
status: done
created: 2026-05-16
updated: 2026-05-17
tags:
  - vscode
  - viewer
  - forms
  - chat-input
  - editor
related:
  - rfc-00021-chat-input-file-mentions-in-form
  - rfc-00018-enhanced-markdown-code-blocks
  - task-00028-enhanced-markdown-code-blocks
supersedes: []
superseded_by: null
---

# Task 00037: Implement RFC 00021 Chat Input File Mentions In Form

## 1. Prime Directive

> [!Prime Directive]
> The current `chat-input` field in the governed document viewer is a fixed-height textarea rendered from `document-viewer/form_editor/`. It cannot reference files inline, does not provide Markdown-aware authoring cues, and mixes a growing editor concern into underscored folder names that are inconsistent with the surrounding kebab-case frontend module layout. This task standardizes the viewer module naming first, then introduces a document-viewer-scoped `chat-input` editor that preserves the current string-based execution path while preparing structured mention metadata for future use.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Primary areas:** `document-viewer`, form rendering, governed preview webview assets, extension/webview messaging, extension tests
- **Dependencies:** RFC 00021, existing `vector-form` rendering contract from RFC 00018, governed preview webview pipeline, current agent execution flow
- **Boundary:** keep the feature inside `document-viewer/`; do not introduce a top-level `chat-input/` module in this task
- **Compatibility rule:** first-iteration runtime execution consumes only plain `text` or `content`; structured `mentions` metadata is emitted but not interpreted by the execution path
- **Naming rule:** internal frontend folders should use kebab-case when they represent module boundaries, including the replacements for `form_editor` and `document_actions`

## 3. Checklist

### 3.1. Phase 0 - Normalize Viewer Module Folder Names

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00037
  phase: Phase 0
  language: typescript
```

- [x] Rename `document-viewer/form_editor/` to `document-viewer/form-editor/`
- [x] Rename `document-viewer/document_actions/` to `document-viewer/document-actions/`
- [x] Update all imports, exports, and test references to the renamed folders
- [x] Preserve behavior exactly during the rename with no functional drift
- [x] Quality gates pass for the rename-only phase

### 3.2. Phase A - Introduce the Document-Viewer Chat Input Module

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00037
  phase: Phase A
  language: typescript
```

- [x] Add a dedicated `document-viewer/chat-input/` module for the interactive editor implementation
- [x] Keep generic form rendering responsibilities in `document-viewer/form-editor/`
- [x] Define the browser-side editor contract for content, mentions, and view state
- [x] Add the host/webview messaging contract needed for mention suggestions
- [x] Keep read-only `chat-input` rendering outside the interactive editor path
- [x] Quality gates pass for the new module structure

### 3.3. Phase B - Replace Textarea Rendering with the Interactive Editor Shell

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00037
  phase: Phase B
  language: typescript
```

- [x] Replace editable `chat-input` textarea rendering with the new editor host element
- [x] Preserve the existing `chat-input("Label")` DSL and field collection behavior
- [x] Keep non-`chat-input` fields working exactly as before
- [x] Keep read-only pre-substituted `chat-input` values rendered without editor initialization
- [x] Ensure the viewer can still collect plain text values from all form blocks
- [x] Quality gates pass for rendering and collection changes

### 3.4. Phase C - Add File Mentions Through Extension-Backed Search

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00037
  phase: Phase C
  language: typescript
```

- [x] Detect `@` mention triggers inside editable `chat-input`
- [x] Resolve file suggestions through the extension host instead of a browser-local index
- [x] Insert readable inline file mentions at the current cursor position
- [x] Emit structured `mentions` metadata together with the plain text content
- [x] Keep the first-iteration execution path string-only by ignoring `mentions` at runtime
- [x] Surface bounded failure behavior when suggestion lookup fails
- [x] Quality gates pass for mention flows

### 3.5. Phase C.5 - Fix Mention Overlay Positioning and Visual Style

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00037
  phase: Phase C.5
  language: typescript
```

- [x] Anchor the file-mention suggestion overlay to the current cursor position instead of the bottom of the `chat-input` field
- [x] Apply a styled dropdown to the overlay that matches the existing `vector-form` code-block visual language (border, background, font, shadow)
- [x] Ensure the overlay does not overflow the webview viewport when triggered near the bottom edge (flip upward if needed)
- [x] Keep keyboard navigation (arrow keys, Enter, Escape) working correctly after the repositioning change
- [x] Quality gates pass for overlay positioning and visual consistency

### 3.6. Phase D - Add Markdown-Aware Styling and Dynamic Growth

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00037
  phase: Phase D
  language: typescript
```

- [x] Add Markdown-aware visual styling for common prompt syntax such as headings, emphasis, inline code, lists, and fenced code blocks
- [x] Preserve raw Markdown source text exactly as authored
- [x] Make editable `chat-input` grow dynamically with content up to a bounded maximum height
- [x] Ensure internal scrolling begins only after the maximum height is reached
- [x] Keep the existing `vector-form` CSS grid layout stable as the editor grows
- [x] Quality gates pass for styling and sizing behavior

### 3.7. Phase E - Integrate With Existing Execution and Viewer Workflows

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00037
  phase: Phase E
  language: typescript
```

- [x] Ensure current agent-action and agent-button flows consume only plain text `text` or `content`
- [x] Preserve compatibility with existing prompt substitution and temp-file execution flows
- [x] Ensure the new editor does not break multiple-form collection in a single document
- [x] Keep unresolved or unsupported mentions from corrupting prompt submission
- [x] Preserve governed preview behavior outside `vector-form` blocks
- [x] Quality gates pass for end-to-end integration changes

### 3.8. Phase F - Cover the Feature With Tests

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00037
  phase: Phase F
  language: typescript
```

- [x] Add tests for the folder rename fallout so import and export surfaces remain valid
- [x] Add tests for editable versus read-only `chat-input` rendering
- [x] Add tests for file mention trigger, suggestion selection, and inline insertion
- [x] Add tests for emitted payload shape containing plain text content and `mentions`
- [x] Add tests proving the first-iteration execution path ignores `mentions`
- [x] Add tests for Markdown-aware styling and dynamic height behavior
- [x] Add tests that the existing form collection contract remains intact
- [x] Quality gates pass for the extension test suite changes

### 3.9. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00037
  phase: Phase Z
  language: typescript
```

- [x] Mark all implemented checklist items complete
- [x] Update extension documentation or architecture notes affected by the new chat-input behavior and folder naming
- [x] Confirm the implementation remains scoped to `document-viewer/`
- [x] Run the final quality gates affected by this task

## 4. Quality Gate

- [x] Editable `chat-input` no longer depends on a plain textarea renderer
- [x] `@` file mentions work through extension-backed search and insert readable inline references
- [x] Agent execution still consumes only plain text prompt content in the first iteration
- [x] Markdown-aware styling improves authoring without changing stored source text
- [x] Dynamic growth works inside the existing `vector-form` CSS grid layout
- [x] Folder renames to `form-editor` and `document-actions` are complete and stable

## 5. Validation Vector

- [x] `document-viewer/form-editor/` replaces `document-viewer/form_editor/` everywhere
- [x] `document-viewer/document-actions/` replaces `document-viewer/document_actions/` everywhere
- [x] A dedicated `document-viewer/chat-input/` module owns the interactive editor behavior
- [x] Editable `chat-input` fields support inline file mentions triggered by `@`
- [x] The emitted chat-input payload includes plain text content and structured `mentions`
- [x] The first-iteration runtime path ignores `mentions` and submits plain text content only
- [x] Markdown syntax remains source-authored plain text while receiving visual authoring cues
- [x] The editor grows vertically without breaking the governed form layout
