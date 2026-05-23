---
id: task-00049-add-new-language-operation
type: task
code: "00049"
slug: add-new-language-operation
title: Add BestPractices Language Operation
description: Add a BestPractices operation to the language crate and expose it as an MCP tool, mirroring the existing QualityGate pattern.
status: todo
created: 2026-05-23
updated: 2026-05-23
tags:
  - language
  - mcp
  - operations
related: []
supersedes: []
superseded_by: null
---

# Task 00049: Add BestPractices Language Operation

## 1. Prime Directive

> [!Prime Directive]
> The `language` crate exposes `QualityGate` as a typed operation that reads a `quality-gate` field from each language definition. There is no equivalent for best-practice guidance. This task adds a `BestPractices` operation that reads a `best-practices` field and surfaces it through a new MCP tool `language_best_practices`.

## 2. Specs

- **Module:** `crate/language`, `crate/mcp`
- **New operation:** `BestPractices` — reads the `best-practices` field from language definition files, following the same structure as `QualityGate`
- **New MCP tool:** `language_best_practices` — wraps the `BestPractices` operation, following the same pattern as the existing `language_quality_gate` tool
- **Dependencies:** none

## 3. Checklist

### 3.1. Phase A — Implement BestPractices operation in the language crate

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00049
  phase: Phase A
  language: Rust
```

- [x] Add `best-practices` field to the language definition schema (mirror `quality-gate`)
- [x] Implement `BestPractices` struct/operation in `crate/language`, following the `QualityGate` implementation
- [x] Add unit tests for the new operation
- [x] Quality gates pass

### 3.2. Phase B — Expose BestPractices as an MCP tool

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00049
  phase: Phase B
  language: Rust
```

- [ ] Register `language_best_practices` tool in the MCP crate, following the `language_quality_gate` pattern
- [ ] Wire `BestPractices` operation as the handler for the new tool
- [ ] Add integration test or manual verification via MCP client
- [ ] Quality gates pass

### 3.3. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00049
  phase: Phase Z
  language: Rust
```

- [ ] Update README files on packages modified
