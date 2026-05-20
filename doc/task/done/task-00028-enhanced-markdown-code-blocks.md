---
id: task-00028-enhanced-markdown-code-blocks
type: task
code: "00028"
slug: enhanced-markdown-code-blocks
title: Enhanced Markdown Code Blocks
description: Extend the markdown viewer with syntax highlighting for standard code blocks and custom component rendering for vector-form, vector-agent-button, vector-agent-action, and vector-open-doc.
status: done
created: 2026-05-11
updated: 2026-05-11
tags:
  - markdown
  - viewer
  - agents
  - components
related:
  - rfc-00018-enhanced-markdown-code-blocks
supersedes: []
superseded_by: null
---

# Task 00028: Enhanced Markdown Code Blocks

## 1. Prime Directive

> [!Prime Directive]
> The markdown viewer renders all code blocks as plain text and has no mechanism for interactive components. This task adds syntax highlighting for standard blocks and introduces two extension modules — `form_editor` and `document_actions` — that parse and render custom fence blocks, collect form state, and execute agent prompts via spawned VSCode terminals.

## 2. Specs

- **Module:** `form_editor/`, `document_actions/` (Vector VSCode extension)
- **Config:** `.vector/agents.yaml`
- **Dependencies:** syntax highlighting library (`highlight.js` or `shiki`), `js-yaml` for parsing action block bodies

## 3. Checklist

### 3.1. Phase A — Syntax Highlighting

- [x] Integrate a syntax highlighting library into the markdown renderer
- [x] Apply highlighting to fenced blocks that declare a language identifier
- [x] Leave plain ` ``` ` blocks unaffected
- [x] Execute quality gate

### 3.2. Phase B — `form_editor` Module

- [x] Implement the form DSL parser (`key = <type>(label)` grammar)
- [x] Render `input` fields (single-line text input)
- [x] Render `chat-input` fields (multi-line textarea)
- [x] Render pre-substituted `#{}` values as read-only fields
- [x] Expose a document-scoped API that returns all form field values merged in document order (later form overrides earlier for the same key)
- [x] Execute quality gate

### 3.3. Phase C — `document_actions` Module: `vector-open-doc`

- [x] Parse `vector-open-doc` YAML block (`label`, `doc`, `input`)
- [x] Resolve `doc` identifier to a file path using `find_doc` logic
- [x] On click: open the target document, substitute `#{}` placeholders with `input` values (view-only — no disk writes)
- [x] Execute quality gate

### 3.4. Phase D — `document_actions` Module: Agent Triggers

- [x] Parse `vector-agent-button` and `vector-agent-action` YAML blocks
- [x] Render button (prominent) and action (flat) with the configured `label`
- [x] Load and parse `.vector/agents.yaml`; surface a user-visible error on parse failure
- [x] Resolve profile to agent list; mark agents not found in `PATH` as disabled
- [x] On agent selected: collect all form field values from the document, merge with block `input` (form overrides static input), resolve the prompt file, substitute `#{}` variables
- [x] Write resolved prompt to a named temp file (`%TEMP%\vector-prompt-<uuid>.txt`)
- [x] Spawn a named VSCode terminal (`Vector: <agent> — <label>`) running `<command> < <tmp_file>`
- [x] Keep terminal open after agent's initial response
- [x] Delete temp file on terminal close or extension deactivation
- [x] Warn the user for any `#{}` variable with no resolved value
- [x] Execute quality gate

### 3.5. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompt-00003-create-doc
input:
  phase: Phase A
  language: typescript
```

- [x] Verify all RFC-00018 acceptance criteria are met
- [x] Update extension README / changelog
