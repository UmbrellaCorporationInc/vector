---
id: task-00040-implement-rfc-00022-kebab-case-substitution-variables-and-vector-yaml-fields
type: task
code: "00040"
slug: implement-rfc-00022-kebab-case-substitution-variables-and-vector-yaml-fields
title: Implement RFC 00022 Kebab-Case Substitution Variables and Vector YAML Fields
description: Enforce kebab-case for substitution variables and `.vector` YAML schema fields across validation, runtime crates, the VS Code extension, and governed repository documents.
status: done
created: 2026-05-18
updated: 2026-05-18
tags:
  - validation
  - runtime
  - vscode
  - config
related:
  - rfc-00022-enforce-kebab-case-substitution-variables
  - rfc-00013-runtime-doc-validation-and-authoring-crate
  - rfc-00020-add-create-document-actions-to-vscode-treeview-doc-type-folders
supersedes: []
superseded_by: null
---

# Task 00040: Implement RFC 00022 Kebab-Case Substitution Variables and Vector YAML Fields

## 1. Prime Directive

> [!Prime Directive]
> The repository currently publishes two incompatible naming contracts: snake_case still appears in active hash-brace placeholder producers and consumers, and `.vector` YAML field validation is inconsistent across loaders. This task removes that ambiguity by enforcing kebab-case as the only accepted naming style for substitution variables and `.vector` YAML schema fields, then migrating the repository content and tests so validation fails fast and deterministically.

## 2. Specs

- **Primary crates/components:** `runtime/doc`, `runtime/language`, `mcp/vector`, `frontend/vscode/vector`, `runtime/project`
- **Primary contracts:** governed Markdown hash-brace substitution variables, `.vector/document-types.yaml`, `.vector/language-rules.yaml`, `.vector/agents.yaml`
- **Primary repository surfaces:** governed prompts, forms, RFCs, tasks, mirrored runtime project assets, YAML examples embedded in docs
- **Boundary:** enforce naming and migration without changing the semantic meaning of prompt resolution, document creation, language quality-gate lookup, or agent profile execution
- **Compatibility decision:** underscore-containing placeholder names and underscore-containing `.vector` YAML schema fields must fail once this task is complete; compatibility shims should not remain as an alternate accepted contract
- **Risk:** broad validation can force migration of historical RFCs and task documents that still contain old placeholder examples

## 3. Checklist

### 3.1. Phase A - Add Repository Validation for Kebab-Case Contracts

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00040
  phase: Phase A
  language: rust
```

- [x] Extend `runtime/doc` validation to detect hash-brace placeholders whose variable names contain `_`
- [x] Add repository validation for YAML schema field names in `.vector/*.yaml` and reject non-kebab field names
- [x] Ensure validation errors identify the exact file path and offending placeholder or field name
- [x] Decide and implement whether `.vector` YAML validation lives only in repository validation or is also duplicated defensively in direct loaders
- [x] Preserve all unrelated governed validation behavior
- [x] Quality gates pass for the new validation rules and tests

### 3.2. Phase B - Rename Rust Placeholder Producers and Tests

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00040
  phase: Phase B
  language: rust
```

- [x] Rename `runtime/doc` placeholder producers from `doc_type` to `doc-type`
- [x] Rename `runtime/doc` placeholder producers from `file_path` to `file-path`
- [x] Keep compliant placeholders such as `code`, `slug`, `layout`, and `types` unchanged
- [x] Update bootstrap and prompt-resolution tests in `runtime/doc` to assert kebab-case output only
- [x] Confirm no Rust-side prompt producer still emits underscore placeholder names
- [x] Quality gates pass for the Rust runtime changes

### 3.3. Phase C - Harden `.vector` YAML Loaders and VS Code Placeholder Parsing

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00040
  phase: Phase C
  language: rust, typescript
```

- [x] Tighten the VS Code substitution regex to reject underscore-containing placeholder names
- [x] Update unresolved-variable detection to match the same kebab-case-only contract
- [x] Rename the create-document flow input from `document_type` to `document-type`
- [x] Harden `.vector/agents.yaml` parsing so schema field names are validated consistently
- [x] Harden `.vector/language-rules.yaml` parsing or validation so schema field names are validated consistently
- [x] Keep user-facing error messages bounded and specific for invalid `.vector` YAML
- [x] Quality gates pass for extension and loader changes

### 3.4. Phase D - Migrate Active Docs, Assets, and YAML Examples

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00040
  phase: Phase D
  language: markdown, yaml
```

- [x] Migrate active governed prompts and forms in `doc/` from snake_case placeholders to kebab-case
- [x] Migrate mirrored prompt and template assets under `runtime/project/assets/doc/` to the same final contract
- [x] Migrate `.vector` YAML examples embedded in governed documentation to kebab-case field names
- [x] Confirm active repository `.vector` YAML files use only kebab-case schema fields
- [x] Keep mirrored assets and source docs aligned so new projects scaffold the final contract
- [x] Quality gates pass for docs, assets, and repository config changes

### 3.5. Phase E - Migrate Historical Governed Documents and Residual Fixtures

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00040
  phase: Phase E
  language: markdown, rust, typescript
```

- [x] Migrate historical RFCs and tasks that still contain snake_case placeholder examples if repository validation scans all governed Markdown bodies
- [x] Update residual test fixtures and inline examples that still encode underscore placeholder names or non-kebab `.vector` YAML fields
- [x] Rewrite tests that currently prove mixed-style support so they instead prove underscore rejection
- [x] Confirm no remaining governed document or test fixture reintroduces the deprecated naming contract
- [x] Quality gates pass for the migrated historical docs and fixtures

### 3.6. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00040
  phase: Phase Z
  language: rust, typescript, markdown, yaml
```

- [x] Mark all implemented checklist items complete
- [x] Update README files or contract docs affected by the naming rule change
- [x] Confirm repository validation is the authoritative failure path for both placeholder names and `.vector` YAML schema fields
- [x] Run the final affected quality gates across runtime crates, MCP adapters, extension code, docs, and project assets

## 4. Quality Gate

- [x] `runtime/doc` validation rejects underscore-containing substitution variable names
- [x] Repository validation rejects non-kebab `.vector` YAML schema fields
- [x] Rust prompt producers emit only kebab-case placeholder names
- [x] VS Code substitution and create-document flows accept only the final kebab-case contract
- [x] Active docs, mirrored assets, and repository `.vector` YAML files are fully migrated
- [x] Historical docs and residual fixtures no longer cause validation regressions

## 5. Validation Vector

- [x] `#{doc-type}` replaces the legacy `doc_type` placeholder everywhere in active producer and consumer paths
- [x] `#{file-path}` replaces the legacy `file_path` placeholder everywhere in active producer and consumer paths
- [x] `#{document-type}` replaces the legacy `document_type` placeholder in the VS Code create-document flow
- [x] `.vector/document-types.yaml` uses only kebab-case schema fields
- [x] `.vector/language-rules.yaml` uses only kebab-case schema fields
- [x] `.vector/agents.yaml` uses only kebab-case schema fields
- [x] Governed prompts, forms, RFCs, tasks, and mirrored assets no longer contain underscore placeholder names unless an explicit validator escape mechanism is later introduced
- [x] Rust and TypeScript tests cover rejection of deprecated underscore naming in both placeholders and `.vector` YAML schema fields
