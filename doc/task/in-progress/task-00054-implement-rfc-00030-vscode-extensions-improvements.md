---
id: task-00054-implement-rfc-00030-vscode-extensions-improvements
type: task
code: "00054"
slug: implement-rfc-00030-vscode-extensions-improvements
title: Implement RFC 00030 VS Code Extension Improvements
description: Implement package-qualified wikilink resolution, a sync action in the VS Code extension, and package-aware document lookup support in MCP vector tools.
status: in-progress
created: 2026-06-06
updated: 2026-06-06
tags:
  - vscode
  - mcp
  - packages
  - wikilinks
related:
  - rfc-00030-vscode-extensions-improvements
supersedes: []
superseded_by: null
---

# Task 00054: Implement RFC 00030 VS Code Extension Improvements

## 1. Prime Directive

> [!Prime Directive]
> Eliminate the mismatch between workspace-local document resolution and synchronized package document resolution so the editor and MCP tooling resolve governed document identifiers deterministically.

## 2. Specs

- **Module:** `frontend/vscode/vector`, `mcp/vector`, shared document lookup flow
- **Dependencies:** RFC [[rfc-00030-vscode-extensions-improvements]]

## 3. Checklist

### 3.1. Phase A - Align identifier parsing and lookup behavior

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00054
  phase: Phase A
  language: TypeScript, Rust
```

- [x] Centralize support for `doc-id` and `package/doc-id` parsing instead of duplicating parsing logic across entrypoints.
- [x] Preserve current workspace-local resolution for unqualified identifiers.
- [x] Add package-aware lookup support for package-qualified identifiers in MCP vector document lookup flows.
- [x] Return actionable errors for unknown packages, unsynchronized packages, and missing governed documents.
- [x] Add tests for identifier parsing, workspace-local lookup, package-qualified lookup, and failure cases.

### 3.2. Phase B - Update VS Code extension navigation and sync flow

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00054
  phase: Phase B
  language: TypeScript
```

- [ ] Support `[[doc-id]]` and `[[package/doc-id]]` wikilink activation in the VS Code extension.
- [ ] Resolve package-qualified wikilinks against synchronized package locations under the packages folder.
- [ ] Reuse the existing open-document behavior after resolution rather than creating a second navigation path.
- [ ] Add a `Sync` action to the tree view near the existing validate, search, refresh, and create doc type actions.
- [ ] Make the `Sync` action open a terminal and execute `vector-database package sync`.
- [ ] Add tests for local wikilinks, package-qualified wikilinks, and sync command invocation.

### 3.3. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00054
  phase: Phase Z
  language: TypeScript, Rust
```

- [ ] Validate that extension behavior and MCP lookup behavior remain aligned for both supported identifier forms.
- [ ] Confirm all RFC 00030 acceptance criteria are covered by implementation or tests.
- [ ] Update README files on packages modified.
