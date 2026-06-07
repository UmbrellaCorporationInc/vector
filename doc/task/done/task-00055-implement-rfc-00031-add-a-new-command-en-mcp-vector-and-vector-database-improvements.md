---
id: task-00055-implement-rfc-00031-add-a-new-command-en-mcp-vector-and-vector-database-improvements
type: task
code: "00055"
slug: implement-rfc-00031-add-a-new-command-en-mcp-vector-and-vector-database-improvements
title: "Implement RFC 00031: Add a new command in mcp-vector and vector-database improvements"
description: Extend mcp-vector CLI to support project creation and make packages manifest loading robust against missing files.
status: done
created: 2026-06-07
updated: 2026-06-07
tags:
  - planning
  - execution
related:
  - rfc-00031-add-a-new-command-en-mcp-vector-and-vector-database-improvements
supersedes: []
superseded_by: null
---

# Task 00055: Implement RFC 00031: Add a new command in mcp-vector and vector-database improvements

## 1. Prime Directive

> [!Prime Directive]
> Eliminate friction in bootstrapping new vector projects and prevent crashes in empty or fresh workspace contexts where no packages are yet defined.

## 2. Specs

- **Module:** `mcp-vector`, `runtime/packages`
- **Dependencies:** none

## 3. Checklist

### 3.1. Phase A — Implementation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00055
  phase: Phase A
  language: rust
```

- [x] Extend argument parsing in `mcp-vector` to support the `create-project` subcommand, using the current working directory as the default project name if none is provided.
- [x] Implement command execution using `ProjectSetupOp` and print progress messages to stdout.
- [x] Modify `load_manifest` in `runtime/packages` to return an empty `PackageManifest::default()` if `.vector/packages.yaml` is missing, instead of returning a `RuntimeError`.
- [x] Update `add_package` in `runtime/packages` to directly load the manifest without redundant existence checks.
- [x] Update unit tests in `manifest_test.rs` to verify behavior for non-existent manifest files.
- [x] Run Cargo unit and integration tests to verify that everything compiles and passes.

### 3.2. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00055
  phase: Phase Z
  language: rust
```

- [x] Update README files on packages modified
