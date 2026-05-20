---
id: task-00032-highlight-completed-task-checkboxes-in-vs-code-preview
type: task
code: "00032"
slug: highlight-completed-task-checkboxes-in-vs-code-preview
title: Highlight completed task checkboxes in VS Code preview
description: Improve the governed VS Code preview so completed markdown checklist items render with a green highlighted x.
status: done
created: 2026-05-11
updated: 2026-05-11
tags: []
related: []
supersedes: []
superseded_by: null
---

# Task 00032: Highlight completed task checkboxes in VS Code preview

## 1. Prime Directive

> [!Prime Directive]
> Completed checklist items blend into surrounding markdown text in the governed preview, making finished work harder to scan than necessary.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** none

## 3. Checklist

### 3.1. Phase A - Task list marker rendering

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00032
  phase: Phase A
  language: TypeScript, CSS
```

- [x] Detect checklist markers at the start of markdown list items in the governed preview renderer
- [x] Render completed markers with a green highlighted x while preserving unchecked markers
- [x] Cover checked, unchecked, and non-list text cases with tests

### 3.2. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00032
  phase: Phase Z
  language: TypeScript, CSS
```

- [x] Run extension quality gates affected by the preview change
