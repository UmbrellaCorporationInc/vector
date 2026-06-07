---
id: rfc-00031-add-a-new-command-en-mcp-vector-and-vector-database-improvements
type: rfc
code: "00031"
slug: add-a-new-command-en-mcp-vector-and-vector-database-improvements
title: Add a new command in mcp-vector and vector-database improvements
description: Proposes extending the mcp-vector CLI to support project initialization commands and improving packages manifest parsing to handle missing manifests gracefully.
status: implemented
created: 2026-06-07
updated: 2026-06-07
authors: ["Antigravity"]
tags: ["cli", "mcp-vector", "packages"]
related: []
supersedes: []
superseded_by: null
aliases:
  - "RFC 00031: Add a new command in mcp-vector and vector-database improvements"
---

# RFC 00031: Add a new command in mcp-vector and vector-database improvements

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00031-add-a-new-command-en-mcp-vector-and-vector-database-improvements`
  document-type: task
  document-name: implement-rfc-00031-add-a-new-command-en-mcp-vector-and-vector-database-improvements
```

## 1. Problem

Currently, the `mcp-vector` CLI only supports the `--version` flag, defaulting to running the stdio MCP server for all other arguments. There is no CLI-native way to scaffold or bootstrap a governed vector project without invoking the MCP server via a client interface.

Additionally, the `vector-database` CLI commands fail when the packages manifest `.vector/packages.yaml` is missing. The underlying `runtime/packages` library's `load_manifest` operation raises a `RuntimeError` on read failures, which forces developers to manually create a manifest or causes errors in empty or fresh workspace contexts where no packages have been defined.

## 2. Proposal

### 2.1. mcp-vector CLI Subcommand

Extend the argument parsing in the `mcp-vector` executable to support a new command: `create-project`.
- The command will scaffold a new governed project skeleton in the current working directory using the `ProjectSetupOp` operation.
- Users can optionally provide a project name (e.g., `mcp-vector create-project my-project`). If omitted, the name is derived from the current working directory.
- The command prints execution progress messages to stdout.

### 2.2. Robust packages.yaml Manifest Loading

Improve the `load_manifest` operation in `runtime/packages` to check if the manifest file exists:
- If `.vector/packages.yaml` does not exist, return an empty `PackageManifest::default()` instead of failing with a `RuntimeError`.
- Update `add_package` in `runtime/packages` to remove redundant exists checks and directly load the manifest.
- Update tests in `manifest_test.rs` to verify that loading a non-existent manifest returns an empty manifest instead of an error.

## 3. Alternatives Considered

- **Alternative A:** Introduce a CLI parsing library (like `clap`) to handle subcommands.
  - *Discarded because:* Manual argument checking keeps dependencies minimal, compile times fast, and adheres to the thin-tooling-boundary design principle.
- **Alternative B:** Implement the manifest existence check directly inside the `vector-database` CLI.
  - *Discarded because:* Manifest resolution logic belongs in `runtime/packages`. Exposing raw path checks to the CLI duplicates domain details across crates.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Simpler project bootstrapping directly from terminal. | Minor increase in CLI surface area for the `mcp-vector` server binary. |
| Zero-configuration execution for packages command in new projects. | `load_manifest` no longer alerts developers to missing manifest files if they expected them. |

## 5. Acceptance Criteria

- [ ] `mcp-vector create-project` command executes `ProjectSetupOp` successfully.
- [ ] `mcp-vector create-project [name]` uses the specified project name.
- [ ] `vector-database package sync` succeeds with no actions in workspaces without `.vector/packages.yaml`.
- [ ] Cargo unit and integration tests compile and pass.

## 6. Open Questions

- Should we support an overwrite flag `--force` on the command line? For now, we default to `force: false`.
