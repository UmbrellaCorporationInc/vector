---
id: task-00047-implement-rfc-00026-mcp-vector-usability-improvements
type: task
code: "00047"
slug: implement-rfc-00026-mcp-vector-usability-improvements
title: Implement RFC 00026 MCP Vector Usability Improvements
description: Implement MCP version introspection and a CLI-driven install or update workflow for the local mcp-vector binary.
status: done
created: 2026-05-21
updated: 2026-05-21
tags:
  - mcp
  - cli
  - usability
  - release
related:
  - "rfc-00026-mcp-vector-usability-improvements"
supersedes: []
superseded_by: null
---

# Task 00047: Implement RFC 00026 MCP Vector Usability Improvements

## 1. Prime Directive

> [!Prime Directive]
> Eliminate operator friction around `mcp-vector` by adding a canonical read-only version surface in MCP and a separate CLI command that can install or update the local binary according to the repository release contract.

## 2. Specs

- **Module:** `mcp/vector`, `frontend/cli/get-vector`, `frontend/cli/get-vector/commands/update-mcp-vector`
- **Dependencies:** root `Cargo.toml` workspace version metadata, `spec-00008-mcp-vector-release-process`, `runtime-io` (for `CommandExecutor`, `CommandBuilder`, `ProcessCommandExecutor`, `MockCommandHandleBuilder`)

## 3. Execution Notes

- **Gap:** The repository currently lacks a first-class operator surface for version introspection and binary reconciliation.
- **Flaw to avoid:** Do not add self-update behavior to the MCP protocol surface. Host mutation belongs in the CLI flow only.
- **Tradeoff:** Centralizing release and version logic in `mcp/vector` preserves a single owner, but it requires deliberate CLI integration boundaries so the CLI does not recreate that logic elsewhere.
- **Minimum test bar:** Cover missing installation, outdated installation, and already-current installation states.

## 4. Checklist

### 4.1. Phase A - MCP-owned release and version runtime

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00047
  phase: Phase A
  language: rust
```

- [x] Make `mcp/vector` the canonical owner of repository version truth derived from the root `Cargo.toml`
- [x] Keep release and version-resolution logic inside `mcp/vector` instead of introducing a shared release crate or second runtime owner

### 4.2. Phase B - MCP version introspection

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00047
  phase: Phase B
  language: rust
```

- [x] Decide the version introspection contract for both protocol clients and the sibling CLI
- [x] Add a read-only `get_version` tool to `mcp/vector` for MCP consumers
- [x] Expose a process-level `--version` argument on the `mcp-vector` binary so the sibling CLI can inspect the installed binary by process invocation without starting an MCP session
- [x] Return the version from `get_version` and `--version` from the same `mcp/vector` source of truth
- [x] Treat the `mcp-vector --version` output format as a stable contract for the sibling CLI
- [x] Keep the MCP surface read-only with no installation or update mutation

### 4.3. Phase C - CLI package and update command

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00047
  phase: Phase C
  language: rust
```

- [x] Create the CLI crate at `frontend/cli/get-vector/`
- [x] Add the command surface at `frontend/cli/get-vector/commands/update-mcp-vector`
- [x] Add `runtime-io` as a workspace dependency of `get-vector` to obtain `CommandExecutor`, `CommandBuilder`, `ProcessCommandExecutor`, and `MockCommandHandleBuilder`
- [x] Use `CommandBuilder` to construct command specs and `CommandExecutor` (with `ProcessCommandExecutor`) as the execution boundary — do not introduce a custom execution abstraction
- [x] V1: always run `cargo install --git <repo> --force mcp-vector` — no version comparison or install-state detection; version-aware reconciliation is deferred to a future task once runtime version resolution is defined

### 4.4. Phase D - Quality gates and documentation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00047
  phase: Phase D
  language: rust, markdown
```

- [x] Add tests using `MockCommandHandleBuilder` from `runtime-io` covering the install success and install failure cases
- [x] Verify the CLI behavior remains aligned with `spec-00008-mcp-vector-release-process`
- [x] Update package or operator documentation affected by the new CLI surface
- [x] Quality gates pass for all modified packages
