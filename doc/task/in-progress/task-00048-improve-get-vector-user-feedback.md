---
id: task-00048-improve-get-vector-user-feedback
type: task
code: "00048"
slug: improve-get-vector-user-feedback
title: Improve `get-version` User Feedback During MCP Updates
description: Surface live update progress while `get-version` refreshes the MCP so the command no longer appears blocked.
status: done
created: 2026-05-21
updated: 2026-05-21
tags:
  - cli
  - mcp
  - ux
related: []
supersedes: []
superseded_by: null
---

# Task 00048: Improve `get-version` User Feedback During MCP Updates

## 1. Prime Directive

> [!Prime Directive]
> Remove the false impression that `get-version` is hung while it updates the MCP by exposing the underlying Cargo progress to the user.

## 2. Specs

- **Module:** `get-version` command and MCP update execution path
- **Dependencies:** existing Cargo execution and output pipeline

## 3. Checklist

### 3.1. Phase A - Expose Update Progress

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00048
  phase: Phase A
  language: Rust, Markdown
```

- [x] Identify where `get-version` triggers the MCP update flow
- [x] Pass through Cargo stdout and stderr while the update is running
- [x] Keep the command output readable and aligned with existing CLI behavior
- [x] Verify the command still reports final success and failure states correctly
- [x] Quality gates passes

### 3.2. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00048
  phase: Phase Z
  language: Rust, Markdown
```

- [x] Update README files on packages modified
- [x] Document any remaining UX limitations or follow-up tasks
