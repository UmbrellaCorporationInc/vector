---
id: task-00030-implement-rfc-00019-language-quality-gate-plugin-operation-and-mcp-tool
type: task
code: "00030"
slug: implement-rfc-00019-language-quality-gate-plugin-operation-and-mcp-tool
title: Implement RFC 00019 Language Quality Gate Plugin Operation and MCP Tool
description: Add the reusable runtime language quality-gate operation and expose it through the MCP server as the `language-quality-gate` tool.
status: done
created: 2026-05-11
updated: 2026-05-11
tags:
  - runtime
  - mcp
  - language
  - prompts
related:
  - rfc-00019-implement-language-quality-gate-plugin-operation-and-mcp-tool
  - spec-00004-language-integration-components-for-mcp
  - prompts-00004-execute-task-phase
supersedes: []
superseded_by: null
---

# Task 00030: Implement RFC 00019 Language Quality Gate Plugin Operation and MCP Tool

## 1. Prime Directive

> [!Prime Directive]
> VECTOR already stores language-specific quality-gate prompt references in `.vector/language-rules.yaml`, but there is no runtime or MCP capability that resolves those references into executable prompt text. This task closes that gap by introducing a reusable `runtime/language` crate, a `QualityGate` plugin operation, and an MCP tool named `language-quality-gate` that returns the combined prompt bodies for a requested language list without frontmatter.

## 2. Specs

- **Crates touched:** `runtime/language`, `mcp/vector`
- **Primary modules:** `runtime/language/src/`, `mcp/vector/src/tools/`, `mcp/vector/src/server.rs`
- **Dependencies:** `runtime-core`, `runtime-channel`, `runtime-io`, governed prompt documents under `doc/prompts/`, `.vector/language-rules.yaml`
- **Boundary:** keep `mcp/vector` as a thin adapter; YAML loading, prompt lookup, frontmatter stripping, and concatenation stay in `runtime/language`
- **Reference RFC:** [[rfc-00019-implement-language-quality-gate-plugin-operation-and-mcp-tool]]
- **Scope constraint:** this task implements prompt-resolution only; it does not execute language-native tests, linters, or formatters

## 3. Checklist

### 3.1. Phase A - Implement the `runtime/language` quality-gate operation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00030
  phase: Phase A
  language: rust
```

- [x] Create the `runtime/language` crate and wire it into the workspace
- [x] Define the `QualityGateInput` and `QualityGateOutput` contracts
- [x] Implement the `QualityGate` plugin operation using the standard runtime plugin pattern
- [x] Load `.vector/language-rules.yaml` from the provided `root_dir`
- [x] Validate that the `languages` input is non-empty
- [x] Reject unknown languages and language entries missing `quality-gate`
- [x] Resolve each configured quality-gate reference to exactly one governed `prompts` document
- [x] Load the resolved prompt files and strip YAML frontmatter before returning content
- [x] Concatenate prompt bodies in the same order as the requested language list
- [x] Reject duplicate languages instead of silently duplicating prompt content
- [x] Add runtime tests covering config loading, unknown language handling, missing prompt mapping, frontmatter stripping, duplicate-language rejection, and deterministic concatenation
- [x] Quality gates pass for the runtime crate changes

### 3.2. Phase B - Expose `language-quality-gate` through MCP

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00030
  phase: Phase B
  language: rust
```

- [x] Add a new MCP tool group named `Language`
- [x] Define MCP-facing params for `language-quality-gate` with `root_dir` and `languages`
- [x] Dispatch the runtime `QualityGate` operation through `PluginDispatcher`
- [x] Return the concatenated prompt string directly from the MCP adapter
- [x] Keep the tool read-only and avoid any prompt-resolution logic duplication in `mcp/vector`
- [x] Register the `Language` tool group in the MCP server surface
- [x] Add MCP tests covering tool registration, schema correctness, successful execution, and error propagation
- [x] Quality gates pass for the MCP adapter changes

### 3.3. Phase C - Align governed configuration and prompt consumers

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00030
  phase: Phase C
  language: rust
```

- [x] Normalize `.vector/language-rules.yaml` to the final accepted prompt identifier contract or add explicit compatibility behavior if legacy identifiers remain supported
- [x] Update prompt and documentation references that depend on `language-quality-gate`
- [x] Confirm [[prompts-00004-execute-task-phase]] matches the final MCP tool name and language-list input shape
- [x] Add or update tests for any config compatibility behavior introduced in this phase
- [x] Quality gates pass for the configuration and documentation alignment changes

### 3.4. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00030
  phase: Phase Z
  language: rust
```

- [x] Mark all implemented checklist items complete
- [x] Ensure package and tool documentation reflect the final behavior
- [x] Confirm the new tool remains a thin MCP facade over reusable runtime logic
- [x] Run the final repository quality gates affected by this task

## 4. Quality Gate

- [x] Runtime tests covering `QualityGate` behavior pass
- [x] MCP tests covering `language-quality-gate` registration and execution pass
- [x] Repository quality gates for the touched Rust crates pass

## 5. Validation Vector

- [x] `runtime/language` exists and owns the reusable quality-gate prompt resolution logic
- [x] `QualityGate` loads `.vector/language-rules.yaml` and resolves requested languages deterministically
- [x] Resolved prompt output excludes YAML frontmatter
- [x] Multi-language requests return one concatenated string in input order
- [x] Duplicate, unknown, or unmapped language inputs fail with explicit errors
- [x] `mcp/vector` exposes the `language-quality-gate` tool through a `Language` tool group
- [x] MCP callers can retrieve combined quality-gate prompt content without reimplementing YAML parsing or prompt lookup
