---
id: task-00038-stabilize-vscode-chat-input-markdown-line-breaks-and-improve-agent-action-viewer-design
type: task
code: "00038"
slug: stabilize-vscode-chat-input-markdown-line-breaks-and-improve-agent-action-viewer-design
title: Stabilize VS Code Chat Input Markdown Line Breaks And Improve Agent Action Viewer Design
description: Fix the governed viewer `chat-input` so Markdown headings and paragraph breaks remain stable while editing, and upgrade `vector-agent-action` rendering from a flat control into a deliberate viewer-native action surface.
status: cancelled
created: 2026-05-17
updated: 2026-05-17
tags:
  - vscode
  - viewer
  - chat-input
  - markdown
  - agents
  - ui
related:
  - rfc-00018-enhanced-markdown-code-blocks
  - rfc-00021-chat-input-file-mentions-in-form
  - task-00028-enhanced-markdown-code-blocks
  - task-00036-improve-vector-form-ui-grid-alignment-and-button-styling
  - task-00037-implement-rfc-00021-chat-input-file-mentions-in-form
supersedes: []
superseded_by: null
---

# Task 00038: Stabilize VS Code Chat Input Markdown Line Breaks And Improve Agent Action Viewer Design

## 1. Prime Directive

> [!Prime Directive]
> The current governed viewer `chat-input` rewrites `contenteditable` HTML from `innerText` during deferred Markdown highlighting. That keeps lightweight styling, but it also creates a structural editing bug: when a user writes a heading such as `# Title` and presses Enter, the trailing empty line can be collapsed during the re-render pass, the cursor can snap back into the heading, and paragraph authoring becomes timing-dependent. In the same viewer, `vector-agent-action` still renders as a flat inline control inherited from the original markdown-components task, which no longer matches the richer visual weight of the surrounding viewer UI. This task restores stable authoring semantics first and then raises the action surface design without changing the execution contract.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Primary areas:** `media/preview.js`, `media/preview.css`, `src/document-viewer/document-actions/agentBlockRenderer.ts`, governed preview tests
- **Dependencies:** existing `chat-input` Markdown cue pipeline, mention insertion flow, RFC 00018 viewer component contract, RFC 00021 `chat-input` editor contract
- **Constraint:** preserve the current extension-host execution flow for `vector-agent-action` and `vector-agent-button`; this task is about editing stability and presentation, not command behavior
- **Tradeoff guardrail:** do not introduce a heavyweight editor framework to solve the newline bug unless the current architecture proves structurally unsalvageable

## 3. Checklist

### 3.1. Phase A - Reproduce and Isolate the Chat Input Line-Break Regression

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00038
  phase: Phase A
  language: typescript, javascript
```

- [x] Document the exact reproduction path for heading-plus-Enter authoring in editable `chat-input`
- [x] Isolate which re-render path collapses the trailing empty line or restores the cursor to the wrong logical position
- [x] Confirm whether the failure is caused by trailing-line trimming, selection mapping, DOM normalization, or an interaction among those behaviors
- [x] Add or update a failing test that proves the bug before implementation begins
- [x] Quality gates pass for the reproduction and regression harness changes

#### Phase A execution notes

- Reproduction path: in an editable `chat-input`, type `# Title`, press `Enter`, then wait for the deferred Markdown highlight pass in `media/preview.js`; the trailing empty logical line is removed during the re-render and the caret can snap back into the heading instead of staying on the new paragraph line.
- Isolated runtime path: `renderMarkdownHtml(rawText, mentions)` trims a trailing empty string after `rawText.split("\n")`, then `applyMarkdownHighlight(hostEl, editorEl)` rewrites `editorEl.innerHTML` and restores the caret through `setCursorCharOffset(editorEl, cursorOffset)`.
- Root cause assessment: the bug is primarily caused by trailing-line trimming, with the visible caret jump amplified by text-offset selection restoration after DOM normalization. The failure is therefore an interaction among trimming, selection mapping, and `contenteditable` DOM rewrite behavior rather than a single mention-specific issue.

### 3.2. Phase B - Preserve Newlines and Cursor Stability During Markdown Highlighting

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00038
  phase: Phase B
  language: typescript, javascript
```

- [x] Update the `chat-input` highlight pipeline so pressing Enter after Markdown headings, lists, or plain lines preserves the intended blank line in the authored source
- [x] Keep caret restoration stable after deferred highlight passes, including newly created empty lines and end-of-block cursor positions
- [x] Preserve source-authored Markdown text exactly as entered, including paragraph boundaries, while still applying visual cues
- [x] Avoid regressions in dynamic height growth and placeholder state updates during multiline editing
- [x] Quality gates pass for newline preservation and caret stability changes

### 3.3. Phase C - Harden Mention and Markdown Interop Around Multiline Editing

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00038
  phase: Phase C
  language: typescript, javascript
```

- [ ] Verify that `@` mention detection, insertion, and chip rendering still work correctly before and after line breaks
- [ ] Ensure multiline edits do not corrupt stored mention metadata when the editor re-renders highlighted HTML
- [ ] Confirm Enter continues to select a suggestion only when the mention dropdown is active and otherwise inserts a newline normally
- [ ] Keep plain-text form collection and current execution payload behavior unchanged
- [ ] Quality gates pass for multiline mention interop

### 3.4. Phase D - Redesign the Agent Action Surface in the Viewer

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00038
  phase: Phase D
  language: typescript, css
```

- [ ] Replace the current flat `vector-agent-action` treatment with a viewer-native design that has clearer hierarchy, spacing, and affordance
- [ ] Keep `vector-agent-action` visually distinct from `vector-agent-button` while making both controls feel part of the same system
- [ ] Improve hover, focus-visible, and disabled/error-adjacent states so actions remain readable and intentional in VS Code themes
- [ ] Preserve the current renderer and data-attribute execution contract unless a minimal markup change is required for styling
- [ ] Quality gates pass for the visual redesign

### 3.5. Phase E - Cover Editing Stability and Action Styling With Tests

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00038
  phase: Phase E
  language: typescript, css, javascript
```

- [ ] Add regression coverage for heading-plus-Enter paragraph authoring in editable `chat-input`
- [ ] Add coverage for trailing empty-line preservation across deferred highlight passes
- [ ] Add coverage that Enter inserts a newline when no mention dropdown is active and selects a suggestion only when the dropdown is open
- [ ] Add renderer or DOM-level coverage for the updated `vector-agent-action` visual structure when needed
- [ ] Quality gates pass for the affected VS Code extension test suite

### 3.6. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00038
  phase: Phase Z
  language: typescript, css, javascript
```

- [ ] Mark every implemented checklist item complete
- [ ] Confirm the final fix does not widen the feature scope into a new editor framework or agent execution redesign
- [ ] Update any governed documentation or viewer notes that describe `chat-input` authoring behavior or agent action presentation
- [ ] Run the final quality gates affected by this task

## 4. Quality Gate

- [ ] Editable `chat-input` preserves authored paragraph breaks after headings and other Markdown-prefixed lines
- [ ] Caret position remains stable through deferred Markdown highlight passes
- [ ] Mention interactions keep working in multiline authoring flows without stealing normal Enter behavior
- [ ] `vector-agent-action` no longer reads as a flat legacy control and instead matches the viewer's current design quality
- [ ] Existing agent execution semantics and form payload collection remain unchanged

## 5. Validation Vector

- [ ] Pressing Enter after `# Heading` in editable `chat-input` produces a durable next line instead of collapsing back into the heading
- [ ] Trailing empty lines survive the markdown-highlight re-render path used by `media/preview.js`
- [ ] Cursor restoration is based on a logical text position that remains valid after multiline DOM rewrites
- [ ] Mention dropdown Enter handling stays scoped to the active suggestion state only
- [ ] `vector-agent-action` styling has stronger hierarchy, spacing, and focus affordance without breaking the click contract
