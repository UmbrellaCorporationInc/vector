---
id: task-00033-normalize-language-lookups-in-runtime-language-quality-gate-resolution
type: task
code: "00033"
slug: normalize-language-lookups-in-runtime-language-quality-gate-resolution
title: Normalize Language Lookups in Runtime Language Quality Gate Resolution
description: Make `runtime/language` lowercase each requested language before lookup in `.vector/language-rules.yaml` and skip languages with no matching configuration.
status: done
created: 2026-05-11
updated: 2026-05-11
tags:
  - runtime
  - language
  - config
related:
  - task-00030-implement-rfc-00019-language-quality-gate-plugin-operation-and-mcp-tool
  - prompts-00004-execute-task-phase
supersedes: []
superseded_by: null
---

# Task 00033: Normalize Language Lookups in Runtime Language Quality Gate Resolution

## 1. Prime Directive

> [!Prime Directive]
> The `runtime/language` quality-gate flow currently depends on exact input casing and fails when a requested language is absent from `.vector/language-rules.yaml`. This task removes that friction by normalizing every requested language to lowercase before lookup and discarding any requested language that has no configuration entry.

## 2. Specs

- **Crate touched:** `runtime/language`
- **Primary modules:** `runtime/language/src/operation.rs`, `runtime/language/src/operation_test.rs`
- **Dependencies:** `.vector/language-rules.yaml`, governed prompt documents under `doc/prompts/`
- **Boundary:** keep normalization and config filtering inside `runtime/language`; callers should not need to pre-normalize language names
- **Behavior change:** requested language tokens must be lowercased before config lookup, and missing config entries must be ignored instead of producing an unknown-language error
- **Scope constraint:** this task changes lookup and filtering behavior only; it does not change prompt resolution, frontmatter stripping, or MCP request shapes

## 3. Checklist

### 3.1. Phase A - Normalize and Filter Language Lookups

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00033
  phase: Phase A
  language: rust
```

- [x] Lowercase each requested language before resolving it against `.vector/language-rules.yaml`
- [x] Preserve deterministic output ordering based on the original request order
- [x] Skip requested languages that do not exist in the loaded config
- [x] Continue rejecting configured languages whose entry exists but lacks a usable `quality-gate` mapping
- [x] Confirm duplicate detection still behaves correctly after normalization
- [x] Quality gates pass for the runtime crate changes

### 3.2. Phase B - Align Runtime Tests with the New Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00033
  phase: Phase B
  language: rust
```

- [x] Update tests to cover mixed-case input such as `Rust` resolving through the lowercase config key
- [x] Add coverage for requests that include one or more languages absent from `.vector/language-rules.yaml`
- [x] Preserve coverage for missing `quality-gate` mappings on languages that are present in config
- [x] Preserve coverage for duplicate-language rejection using the final normalized values
- [x] Quality gates pass for the updated test suite

### 3.3. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00033
  phase: Phase Z
  language: rust
```

- [x] Mark all implemented checklist items complete
- [x] Update any crate documentation that still describes case-sensitive matching or unknown-language failure for missing config entries
- [x] Run the final repository quality gates affected by this task

## 4. Quality Gate

- [x] Runtime tests covering lowercase normalization and config filtering pass
- [x] Existing prompt-resolution behavior remains intact for configured languages
- [x] No caller-side normalization requirement leaks into MCP or other adapters

## 5. Validation Vector

- [x] `runtime/language` lowercases requested language names before config lookup
- [x] Requests containing configured mixed-case languages resolve successfully
- [x] Requests containing unconfigured languages return prompt output for configured languages only
- [x] Configured languages with missing `quality-gate` values still fail explicitly
- [x] Duplicate detection remains deterministic after normalization
