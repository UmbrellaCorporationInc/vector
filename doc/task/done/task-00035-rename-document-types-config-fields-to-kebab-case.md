---
id: task-00035-rename-document-types-config-fields-to-kebab-case
type: task
code: "00035"
slug: rename-document-types-config-fields-to-kebab-case
title: Rename Document Types Config Fields to Kebab Case
description: Rename `code-width` to `code-width` and `initial-status` to `initial-status` across governed document type configuration, runtime code, MCP adapters, VS Code extension code, and repository config assets.
status: done
created: 2026-05-12
updated: 2026-05-12
tags:
  - config
  - runtime
  - mcp
  - vscode
related:
  - rfc-00013-runtime-doc-validation-and-authoring-crate
  - rfc-00016-add-directory-layout-for-document-types
supersedes: []
superseded_by: null
---

# Task 00035: Rename Document Types Config Fields to Kebab Case

## 1. Prime Directive

> [!Prime Directive]
> The governed document type contract currently mixes snake_case with kebab-case inside `.vector/document-types.yaml`. This task normalizes the YAML-facing contract by renaming `code-width` to `code-width` and `initial-status` to `initial-status`, then propagates that contract change through every crate, adapter, extension component, template asset, and repository configuration file that reads, writes, validates, scaffolds, or tests those fields.

## 2. Specs

- **Contract change:** `code_width` -> `code-width`
- **Contract change:** `initial_status` -> `initial-status`
- **Primary crates/components:** `runtime/doc`, `mcp/vector`, `frontend/vscode/vector`, `runtime/project`
- **Config files to migrate:** `.vector/document-types.yaml`, `runtime/project/assets/.vector/document-types.yaml`
- **Boundary:** rename the YAML contract and all consumer expectations without changing the semantic meaning of code padding or status-based initial placement
- **Compatibility decision:** this task should define whether snake_case remains temporarily supported or is rejected immediately; implementation and tests must match that decision consistently

## 3. Checklist

### 3.1. Phase A - Update `runtime/doc` for the New YAML Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00035
  phase: Phase A
  language: rust
```

- [x] Rename YAML deserialization and serialization expectations in `runtime/doc` from `code_width` to `code-width`
- [x] Rename YAML deserialization and serialization expectations in `runtime/doc` from `initial_status` to `initial-status`
- [x] Update bootstrap, create-doc, create-doc-type, find-doc, validate, and related `runtime/doc` operations to consume the renamed fields
- [x] Update `runtime/doc` tests and fixtures that embed `document-types.yaml` content
- [x] Decide and enforce whether legacy snake_case input is rejected or supported only as an explicit compatibility path
- [x] Quality gates pass for `runtime/doc`

### 3.2. Phase B - Update `mcp/vector` Adapters and Tests

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00035
  phase: Phase B
  language: rust
```

- [x] Update `mcp/vector` tests and fixtures that assert or embed `document-types.yaml` content with `code_width`
- [x] Align any MCP-facing schema, adapter assumptions, or test descriptions that refer to the old YAML field names
- [x] Keep MCP tool contracts stable unless a public request shape truly needs to change
- [x] Quality gates pass for `mcp/vector`

### 3.3. Phase C - Update the VS Code Extension Consumers

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00035
  phase: Phase C
  language: typescript
```

- [x] Rename `frontend/vscode/vector` config parsing expectations from `code_width` to `code-width`
- [x] Rename `frontend/vscode/vector` config parsing expectations from `initial_status` to `initial-status`
- [x] Update tree, dashboard, discovery, and prompt-related tests that currently use the old YAML field names
- [x] Preserve extension behavior for code padding and status grouping after the config rename
- [x] Quality gates pass for `frontend/vscode/vector`

### 3.4. Phase D - Update `runtime/project` Assets and Scaffolded Templates

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00035
  phase: Phase D
  language: rust
```

- [x] Update `runtime/project/assets/.vector/document-types.yaml` to use `code-width` and `initial-status`
- [x] Update any scaffolded template content or generated examples that still emit `code_width` or `initial_status`
- [x] Keep newly created projects aligned with the final accepted YAML contract
- [x] Quality gates pass for `runtime/project` and related scaffolding tests

### 3.5. Phase E - Migrate Repository `document-types.yaml` Files

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00035
  phase: Phase E
  language: yaml
```

- [x] Update `.vector/document-types.yaml` to use `code-width` and `initial-status`
- [x] Update `runtime/project/assets/.vector/document-types.yaml` to use `code-width` and `initial-status` wherever present
- [x] Confirm there are no remaining repository `document-types.yaml` files using the old snake_case fields
- [x] Keep the migrated YAML files parseable by the final implementation

### 3.6. Phase F - Update Documentation and Residual Fixtures

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00035
  phase: Phase F
  language: rust, typescript, markdown
```

- [x] Update governed docs, templates, and inline examples that still document `code_width` or `initial_status`
- [x] Update residual non-`document-types.yaml` test fixtures that intentionally show config snippets
- [x] Ensure the final documentation consistently describes the kebab-case contract
- [x] Quality gates pass for the touched docs and tests

### 3.7. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00035
  phase: Phase Z
  language: rust, typescript, yaml
```

- [x] Mark all implemented checklist items complete
- [x] Confirm all crates and components consume the same final field names
- [x] Run the final affected quality gates across runtime, MCP, extension, and project assets

## 4. Quality Gate

- [x] `runtime/doc` loads, validates, and writes the kebab-case field names correctly
- [x] `mcp/vector` tests stay green after the config rename
- [x] `frontend/vscode/vector` continues to parse and use document type config correctly
- [x] Repository `document-types.yaml` files use only `code-width` and `initial-status`
- [x] No remaining required code path depends on `code_width` or `initial_status` unless explicit compatibility support was intentionally added and tested

## 5. Validation Vector

- [x] YAML-facing governed config uses `code-width` instead of `code_width`
- [x] YAML-facing governed config uses `initial-status` instead of `initial_status`
- [x] `runtime/doc` behavior for code padding and initial status placement remains correct after the rename
- [x] MCP-backed document operations remain aligned with the updated config contract
- [x] VS Code extension discovery and tree/dashboard behavior remain aligned with the updated config contract
- [x] Both repository `document-types.yaml` files are migrated to the final field names
