---
id: task-00036-improve-vector-form-ui-grid-alignment-and-button-styling
type: task
code: "00036"
slug: improve-vector-form-ui-grid-alignment-and-button-styling
title: Improve vector-form UI grid alignment and button styling
description: Improve the `vector-form` presentation by aligning fields with a CSS grid, adding row spacing, setting `chat-input` to 10 visible rows, and refining action button styling.
status: done
created: 2026-05-15
updated: 2026-05-15
tags: [ui, vscode, forms]
related: []
supersedes: []
superseded_by: null
---

# Task 00036: Improve vector-form UI grid alignment and button styling

## 1. Prime Directive

> [!Prime Directive]
> Eliminate layout inconsistency and weak visual hierarchy in `vector-form` by introducing a structured grid layout, clearer vertical rhythm, a taller `chat-input`, and stronger button affordances.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** none

## 3. Checklist

### 3.1. Phase A - Grid layout and form spacing

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00036
  phase: Phase A
  language: typescript, css
```

- [x] Update the `vector-form` layout to use a CSS grid that aligns labels, inputs, and action surfaces consistently.
- [x] Add row gaps across form items so stacked fields have clear vertical separation.
- [x] Preserve responsive behavior so the layout remains usable in narrow preview widths.
- [x] Quality gates pass for layout changes.

### 3.2. Phase B - Chat input sizing and button visual design

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00036
  phase: Phase B
  language: typescript, css
```

- [x] Set the `chat-input` field to render with 10 visible rows.
- [x] Improve button styling so primary actions feel intentional, readable, and clearly interactive.
- [x] Keep the updated button treatment visually compatible with the existing document viewer UI.
- [x] Quality gates pass for control styling changes.

### 3.3. Phase Z - Wrap-up and validation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00036
  phase: Phase Z
  language: typescript, css
```

- [x] Review the final `vector-form` experience in the viewer for spacing, alignment, and button consistency.
- [x] Update any governed documentation or inline examples if the UI contract or screenshots need to change.
- [x] Quality gates pass for the affected package.
