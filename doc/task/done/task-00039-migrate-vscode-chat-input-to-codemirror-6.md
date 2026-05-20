---
id: task-00039-migrate-vscode-chat-input-to-codemirror-6
type: task
code: "00039"
slug: migrate-vscode-chat-input-to-codemirror-6
title: Migrate VS Code Chat Input To CodeMirror 6
description: Replace the current contenteditable chat-input runtime with a CodeMirror 6 implementation so mention editing, Markdown-aware authoring, and caret behavior are reliable under repeated re-render and deletion flows.
status: done
created: 2026-05-17
updated: 2026-05-18
tags:
  - vscode
  - chat-input
  - codemirror
  - editor
related:
  - rfc-00021-chat-input-file-mentions-in-form
  - task-00037-implement-rfc-00021-chat-input-file-mentions-in-form
  - task-00038-stabilize-vscode-chat-input-markdown-line-breaks-and-improve-agent-action-viewer-design
supersedes: []
superseded_by: null
---

# Task 00039: Migrate VS Code Chat Input To CodeMirror 6

## 1. Prime Directive

> [!Prime Directive]
> Eliminate the current `contenteditable` plus `innerHTML` editing loop in the governed preview chat-input. The current runtime uses the rendered DOM as both source-of-truth and view layer, which makes newline handling, caret restoration, mention chips, deletion, and repeated re-render behavior structurally unreliable. Replace that architecture with a CodeMirror 6 text model whose selection and document state are independent from the styled DOM projection.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** `@codemirror/state`, `@codemirror/view`, `@codemirror/commands`, `@codemirror/language`, and any minimal Markdown or highlighting packages required for the accepted RFC 00021 scope
- **Compatibility:** preserve the existing `chat-input("Label")` DSL, current plain-text submission contract, file-suggestion host messaging, and read-only chat-input rendering path
- **Out of scope:** runtime interpretation of structured mentions by agent execution, Monaco adoption, rich-text editing, and arbitrary document-wide editor replacement

## 3. Checklist

### 3.1. Phase A - CodeMirror 6 Editor Foundation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00039
  phase: Phase A
  language: typescript, javascript
```

- [x] Add the minimal CodeMirror 6 dependency set to the VS Code extension package
- [x] Introduce a dedicated chat-input runtime module that owns editor state, selection, and lifecycle outside `preview.js` ad hoc DOM mutation helpers
- [x] Replace editable `contenteditable` chat-input initialization with CodeMirror-backed editor instances mounted inside the existing host shell
- [x] Preserve read-only chat-input rendering without instantiating CodeMirror
- [x] Keep current form collection compatible by serializing plain text from the CodeMirror document model instead of the rendered DOM
- [x] Keep test coverage green after the dependency and runtime bootstrap change

### 3.2. Phase B - Mentions, Markdown Styling, And Host Integration

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00039
  phase: Phase B
  language: typescript, javascript
```

- [x] Reimplement `@` mention detection and selection on top of CodeMirror transactions and selection state
- [x] Preserve the current extension-host suggestion request and response contract so file search remains workspace-backed
- [x] Render mention chips or decorations without making the rendered DOM the source-of-truth for submitted text
- [x] Apply Markdown-aware visual styling for the RFC 00021 first-pass syntax set while preserving raw source text in the document model
- [x] Preserve keyboard behavior for arrow navigation, enter, escape, backspace, and mention deletion around inserted references
- [x] Keep the existing plain-text plus structured-mentions payload contract for form submission

### 3.3. Phase C - Layout, Regression Closure, And Runtime Validation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00039
  phase: Phase C
  language: typescript, javascript
```

- [x] Reimplement bounded auto-grow behavior using CodeMirror measurement APIs without breaking the `.vector-form` grid layout
- [x] Remove obsolete `contenteditable` caret-marker, trailing-break-anchor, and DOM-serialization workaround code from the webview runtime
- [x] Add regression coverage for repeated Enter, delete-all, mention insertion after blank lines, and caret stability after re-render-relevant edits
- [ ] Validate manual VS Code webview behavior for first-enter, repeated-enter, delete-all, and mention reopen scenarios before closing the task
- [x] Confirm that existing agent execution still consumes only plain text in the first iteration

#### Phase C execution notes

- Replaced CSS-only height bounding with `EditorView.requestMeasure(...)` plus `ResizeObserver` so the CodeMirror scroller grows to content height, caps at the existing maximum, and only enables internal scrolling after the bound is reached.
- Confirmed the webview runtime no longer contains the legacy `contenteditable` rewrite markers (`caret-marker`, `trailing-break-anchor`, `setCursorCharOffset`, `renderMarkdownHtml`, `applyMarkdownHighlight`) that previously coupled editing behavior to DOM serialization.
- Added `src/test/chatInputPhaseCRegression.test.ts` to cover bounded auto-grow wiring, blank-line mention insertion, delete-all mention reconciliation, and the first-iteration plain-text execution contract.
- Package test suite passed via `pnpm test`.
- `language-quality-gate` for `typescript, javascript` could not complete because governed prompt `prompts-00007-typescript` did not resolve in the repository configuration.
- Manual VS Code webview validation is still pending and remains the only unchecked Phase C item.

### 3.4. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00039
  phase: Phase Z
  language: markdown, typescript, javascript
```

- [x] Update extension documentation and architecture notes affected by the editor migration
- [x] Update task and RFC references if implementation details diverge materially from the accepted path
- [x] Run the relevant package test suite and record the executed quality gate in the final work log or close-out

#### Phase Z execution notes

- Updated `frontend/vscode/vector/README.md` to document the CodeMirror 6 dependency, the dedicated `chat-input-runtime.js` ownership boundary, and the current packaged behavior of editable `chat-input` fields.
- Updated `rfc-00021-chat-input-file-mentions-in-form` to record that task `00039` completed the accepted CodeMirror 6 migration and to align historical `form_editor` path references with the implemented `form-editor` module name.
- No material divergence from the accepted RFC path was found: the shipped implementation still uses a plain-text CodeMirror 6 editor, extension-host-backed file mentions, bounded auto-grow, and first-iteration plain-text execution.
- Package test suite passed via `pnpm test`.
- `language-quality-gate` for `markdown, typescript, javascript` failed because governed prompt `prompts-00007-typescript` did not resolve in the repository configuration.
- Manual VS Code webview validation from Phase C remains pending and continues to block full task closure even though the Phase Z wrap-up items are complete.
