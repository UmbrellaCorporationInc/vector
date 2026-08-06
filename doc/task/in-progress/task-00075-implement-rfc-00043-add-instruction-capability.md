---
id: task-00075-implement-rfc-00043-add-instruction-capability
type: task
code: "00075"
slug: implement-rfc-00043-add-instruction-capability
title: Implement RFC 00043 Add Instruction Capability
description: Replace filesystem-path prompt handoff with the UUID-scoped instruction capability accepted in RFC 00043.
status: in-progress
created: 2026-08-05
updated: 2026-08-06
tags:
  - vscode
  - mcp
  - security
  - configuration
related:
  - rfc-00043-add-instruction-capability
supersedes: []
superseded_by: null
---

# Task 00075: Implement RFC 00043 Add Instruction Capability

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: task-00075-implement-rfc-00043-add-instruction-capability
```

## 1. Prime Directive

> [!Prime Directive]
> Eliminate arbitrary prompt-path handoff between the Vector VS Code extension and mcp-vector by implementing the UUID-scoped instruction capability defined in [[rfc-00043-add-instruction-capability]].

## 2. Specs

- **Modules:** Vector VS Code extension, `runtime-doc`, `mcp-vector`, repository and bootstrap agent configuration, user documentation
- **Dependencies:** Existing VS Code agent launch surfaces, MCP tool composition, YAML configuration boundaries, UUID parsing, filesystem abstractions, target-specific Windows APIs through `windows-sys`, and repository quality gates
- **Languages:** TypeScript, Rust, YAML, Markdown
- **Security boundary:** Callers provide only a canonical UUID; directory and filename construction remain implementation-controlled

## 3. Checklist

### 3.1. Phase A — Configuration Contract and Migration

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00075
  phase: Phase A
  language: TypeScript, Rust, YAML
```

- [x] Add the required `instructions-dir: "${system-temp}/vector/instructions"` property to repository and bootstrap configuration.
- [x] Replace bundled `<file>` usage with exactly one `<instruction>` placeholder per agent command and reject `<file>` and `<instruction-id>`.
- [x] Implement equivalent TypeScript and Rust validation for missing, empty, malformed, unsupported, repeated, rooted, traversing, or escaping instruction-directory values.
- [x] Add independent table-driven TypeScript and Rust tests covering the same observable configuration cases; do not introduce shared cross-language fixture infrastructure in this task.
- [x] Ensure configuration failures are actionable and prevent terminal creation.
- [x] Run focused configuration and bootstrap asset tests.

### 3.2. Phase B — VS Code Instruction Lifecycle

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00075
  phase: Phase B
  language: TypeScript
```

- [x] Resolve prompts and inputs before generating a canonical random UUID and exclusively creating the fixed UTF-8 instruction file.
- [x] Render the frontend-owned MCP directive and substitute it as one shell-safe `<instruction>` argument without a second interpolation pass.
- [x] Launch terminals with the active workspace root as explicit `cwd` for both agent actions and agent buttons.
- [x] Preserve the current single-governed-project contract: use the `workspaceRoot` selected during extension activation and do not add multi-project workspace mapping in this task.
- [x] Track instruction ownership by terminal and clean files on launch failure, terminal close, and extension deactivation.
- [x] On activation, remove only recognized regular instruction files older than 24 hours without following links, recursing, or deleting unrelated files.
- [x] Apply restrictive POSIX permissions where supported and report cleanup failures through a local VS Code diagnostic output channel without telemetry.
- [x] Add focused tests for UUID naming, exclusive creation, UTF-8 writing, directive rendering, cross-shell quoting, both launch surfaces, explicit `cwd`, cleanup paths, and stale-file selection.

### 3.3. Phase C — MCP Instruction Retrieval

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00075
  phase: Phase C
  language: Rust
```

- [x] Implement `GetInstructionOp` in `runtime-doc` with the standard operation input, output, dispatcher, and focused test structure.
- [x] Add `get_instruction` to the MCP document tool group as a thin `PluginDispatcher` adapter with no duplicated configuration or filesystem logic.
- [x] Accept only canonical lowercase hyphenated UUIDs and construct the controlled filename internally.
- [x] Enforce direct-child containment, regular-file and link protections, a fixed non-configurable 1 MiB limit before and during reads, and valid UTF-8.
- [x] Add target-specific `windows-sys` support to open without following reparse redirection, reject reparse targets, and validate the opened handle's final location; document any residual platform limitation.
- [x] Return exact instruction content without deleting it so repeated reads remain valid for the terminal lifetime.
- [x] Provide bounded actionable errors for invalid UUIDs, invalid configuration, missing or expired instructions, non-regular targets, oversized content, invalid UTF-8, and read failures.
- [x] Add tool-listing, routing, containment, validation, repeated-read, error, and platform-appropriate link tests using isolated temporary directories.

### 3.4. Phase D — Integration and Documentation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00075
  phase: Phase D
  language: TypeScript, Rust, Markdown
```

- [x] Verify the complete frontend-to-MCP flow while preserving the distinct responsibilities of `.agents/mcp_config.json` and `.vector/agents.yaml`.
- [x] Document the breaking migration, supported `${system-temp}` syntax, lifecycle ownership, retry behavior, size bound, and platform-specific filesystem guarantees.
- [x] Document the single-governed-project workspace scope, the absence of cleanup telemetry, and the deferral of shared cross-language fixture infrastructure.
- [x] Confirm existing MCP tools and unaffected VS Code agent behavior remain covered by regression tests.

### 3.5. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00075
  phase: Phase Z
  language: TypeScript, Rust, Markdown
```

- [ ] Run TypeScript compilation, lint, and focused tests.
- [ ] Run Rust formatting, lint, and focused tests.
- [ ] Run MCP tool-listing and routing tests plus project bootstrap asset tests.
- [ ] Run repository-prescribed quality gates and separate unrelated pre-existing failures from implementation failures.
- [ ] Update README files for modified packages and ensure every governed-document reference uses a stem wikilink.

## 4. Staff Engineer Assessment

This task is security-sensitive and should not be split into independently shippable frontend and MCP changes unless the intermediate state cannot launch agents. The UUID is only a bearer token; the actual boundary depends on controlled path construction, canonical validation, exclusive creation, opened-handle checks where available, bounded UTF-8 reads, and terminal-scoped cleanup working together.

The highest implementation risk is semantic drift between TypeScript and Rust, especially around `${system-temp}` expansion and Windows reparse behavior. Shared cross-language fixture infrastructure is deliberately deferred, so the independent test matrices must cover matching cases and be compared during review. Lexical containment plus pre-open metadata is insufficient to claim race-free link protection; Windows handle validation through `windows-sys` and an explicit account of residual platform guarantees are required completion evidence.

The intentional breaking migration is justified by [[rfc-00043-add-instruction-capability]], but it raises rollout risk. Actionable validation errors, updated bootstrap assets, and documentation are feature requirements, not optional follow-up work.
