---
id: task-00058-add-help-command-to-mcp-vector
type: task
code: "00058"
slug: add-help-command-to-mcp-vector
title: Add --help command to mcp-vector
description: Add a help command to mcp-vector and add version and help commands to get-vector CLI.
status: in-progress
created: 2026-06-07
updated: 2026-06-07
tags:
  - cli
  - help
  - mcp
related: []
supersedes: []
superseded_by: null
---

# Task 00058: Add --help command to mcp-vector

## 1. Prime Directive

> [!Prime Directive]
> Add a `--help` option to `mcp-vector` to provide information about the MCP usage. Additionally, add a `--version` option to the `get-vector` CLI (fetching the version from the cargo manifest) to verify alignment with the MCP version, and a `--help` option for the `get-vector` CLI that displays usage help along with an ASCII box showing the `cargo` command for updating the CLI.

## 2. Specs

- **Module:** `mcp-vector`, `get-vector`
- **Dependencies:** `cargo.toml` package metadata (for version)

## 3. Checklist

### 3.1. Phase A — Implement Help and Version Support

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00058
  phase: Phase A
  language: rust
```

- [x] Add support for `--help` in `mcp-vector` to print help info.
- [x] Add support for `--version` in `get-vector` CLI, deriving the version string from the cargo package version.
- [x] Add support for `--help` in `get-vector` CLI, displaying usage text and an ASCII box with the command:
  ```
  cargo install --git https://github.com/UmbrellaCorporationInc/vector get-vector
  ```
- [x] Implement command parsing logic to handle these options correctly.
- [x] Verify execution of new flags.

### 3.2. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00058
  phase: Phase Z
  language: rust
```

- [ ] Update README files on packages modified
