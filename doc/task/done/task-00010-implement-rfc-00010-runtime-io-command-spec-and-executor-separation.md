---
id: task-00010-implement-rfc-00010-runtime-io-command-spec-and-executor-separation
type: task
code: "00010"
slug: implement-rfc-00010-runtime-io-command-spec-and-executor-separation
title: Implement RFC 00010 runtime IO command spec and executor separation
description: Implement the command model split from RFC 00010 so runtime-io builds command specs separately from real or mock execution.
status: done
created: 2026-05-03
updated: 2026-05-04
tags:
  - runtime
  - io
  - shell
  - architecture
related:
  - rfc-00010-runtime-io-command-spec-and-executor-separation
  - rfc-00009-runtime-io-file-access-and-shell-command-execution
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
supersedes: []
superseded_by: null
---

# Task 00010: Implement RFC 00010 runtime IO command spec and executor separation

## 1. Prime Directive

Split command description from command side effects in `runtime-io` so command planning becomes data-only, execution becomes swappable, and higher-level crates can use real or mock executors without changing the command model.

## 2. Specs

- **Module:** `runtime/io`
- **Dependencies:** Rust `std`, `thiserror`, `runtime-core`, selected async runtime backend

## 3. Checklist

### 3.1. Phase A - Command API realignment

- [x] Replace builder-driven direct process execution with the accepted spec-first command model
- [x] Define the public command module layout for `CommandSpec`, `CommandBuilder`, `CommandExecutor`, and concrete executor implementations
- [x] Preserve the accepted running-command boundary around `CommandHandle`, `CommandOutput`, and `CommandInput`
- [x] Add tests covering public API visibility and command model separation
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - CommandSpec and builder implementation

- [x] Implement `CommandSpec` as the data-only command specification type
- [x] Update `CommandBuilder` to build `CommandSpec` instead of spawning processes directly
- [x] Preserve explicit command, ordered arguments, optional working directory, and environment configuration
- [x] Ensure `CommandBuilder::build` introduces no process side effects
- [x] Add tests covering spec construction, builder composition, and deterministic field preservation
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - CommandExecutor contract

- [x] Implement the accepted `CommandExecutor` execution boundary
- [x] Ensure `CommandExecutor` accepts `CommandSpec` and returns `CommandHandle`
- [x] Decide and document the v1 dispatch strategy used by the crate implementation
- [x] Add tests covering executor substitution through the accepted command model
- [x] execute section "4. Quality Gate"

### 3.4. Phase D - Real process executor

- [x] Implement the real operating-system-backed executor for `CommandSpec`
- [x] Preserve the existing command IO behavior through `CommandHandle`, `CommandOutput`, and `CommandInput`
- [x] Ensure executor-owned process creation remains outside `CommandBuilder`
- [x] Add tests covering real process spawn, stdout and stderr consumption, stdin publication, completion, and cleanup behavior
- [x] execute section "4. Quality Gate"

### 3.5. Phase E - Mock execution support

- [x] Provide one mock-friendly executor path compatible with the accepted `CommandExecutor` contract
- [x] Ensure tests can validate command planning without launching real processes
- [x] Keep fake behavior out of `CommandSpec`
- [x] Add tests covering deterministic mock execution and command-spec inspection
- [x] execute section "4. Quality Gate"

### 3.6. Phase F - RFC 00009 alignment

- [x] Update `runtime-io` command documentation to reflect `CommandBuilder::build` plus `CommandExecutor::spawn`
- [x] Remove or realign any builder-driven direct execution API that conflicts with RFC 00010
- [x] Verify file, memory, text, and path APIs remain unchanged by the command refactor
- [x] Add tests covering backward-boundary expectations that remain valid after the split
- [x] execute section "4. Quality Gate"

### 3.7. Phase G - Public API integration and docs

- [x] Re-export `CommandSpec`, `CommandBuilder`, `CommandExecutor`, and the real executor from the crate root
- [x] Update README documentation for the command planning and command execution split
- [x] Verify the command API introduces no shell parser, retry policy, scheduling policy, or repository-specific workflow logic
- [x] execute section "4. Quality Gate"

### 3.8. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `CommandBuilder` builds `CommandSpec` and performs no process side effects
- [x] `CommandExecutor` is the only accepted execution boundary for `CommandSpec`
- [x] The public architecture supports both real and mock command execution without changing the command model
- [x] `CommandHandle`, `CommandOutput`, and `CommandInput` remain the running-command boundary after execution starts
- [x] `CommandOutput` continues to satisfy `Receiver<Bytes>`
- [x] `CommandInput` continues to satisfy `Sender<Bytes>`
- [x] File, memory, text, and path APIs remain outside the scope of command execution changes
- [x] The command API introduces no shell-form parsing rules, retry policy, scheduling policy, or repository-specific workflow logic
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
