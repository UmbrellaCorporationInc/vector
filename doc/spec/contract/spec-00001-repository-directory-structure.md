---
id: spec-00001-repository-directory-structure
type: spec
code: "00001"
slug: repository-directory-structure
title: Repository Directory Structure
description: Defines the repository directory structure contract for frontend packages, runtime libraries, and MCP packages.
category: contract
created: 2026-05-01
updated: 2026-05-01
authors: []
tags:
  - repository
  - structure
  - layout
related: []
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00001: Repository Directory Structure"
---

# SPEC 00001: Repository Directory Structure

## 1. Purpose

This spec defines the top-level repository directory contract for frontend packages, shared runtime libraries, and MCP packages.

## 2. Definition

The repository must organize production source code under three top-level capability roots:

- `frontend/`
- `runtime/`
- `mcp/`

Directory responsibilities:

- `frontend/` contains user-facing delivery packages.
- `frontend/cli/` contains executable command-line tools.
- `frontend/cli/<tool-name>/` contains one CLI tool with its own entrypoints, command modules, tests, and local assets.
- `frontend/cli/<tool-name>/commands/` contains the CLI command surface owned by that tool.
- `frontend/vscode/` contains Visual Studio Code extensions.
- `frontend/vscode/<extension-name>/` contains one VS Code extension package.
- `runtime/` contains shared libraries that can be imported by one or more frontend packages or MCP packages.
- `runtime/<library-name>/` contains one reusable runtime library with a narrowly defined responsibility.
- `mcp/` contains Model Context Protocol server packages.
- `mcp/vector/` is the canonical package for the `vector` MCP server.
- Package names under `frontend/`, `runtime/`, and `mcp/` do not follow a required global naming convention.

Allowed supporting top-level directories outside this contract include governance, automation, and repository metadata roots such as `doc/`, `.github/`, `.cargo/`, and build output directories.

## 3. Invariants

- Production code must not be introduced at the repository root.
- Every executable CLI surface must live under `frontend/cli/<tool-name>/commands/`.
- Every VS Code extension must live under `frontend/vscode/<extension-name>/`.
- Shared code used by more than one frontend package or by both a frontend package and an MCP package must live under `runtime/`.
- `runtime/` libraries must not depend on frontend-specific command or extension modules.
- MCP servers must live under `mcp/<server-name>/`.
- The `vector` MCP implementation must live under `mcp/vector/`.
- A directory under `frontend/`, `runtime/`, or `mcp/` must represent one package with one primary responsibility.
- Cross-package dependencies must point inward toward shared runtime libraries, not sideways into another package's private internals.

## 4. Examples

```text
.
|-- .github/
|-- doc/
|-- frontend/
|   |-- cli/
|   |   `-- forge/
|   |       |-- Cargo.toml
|   |       |-- commands/
|   |       `-- src/
|   |   `-- heph/
|   |       |-- Cargo.toml
|   |       |-- commands/
|   |       `-- src/
|   `-- vscode/
|       `-- vector-tools/
|           |-- package.json
|           `-- src/
|-- mcp/
|   `-- vector/
|       |-- Cargo.toml
|       `-- src/
|-- runtime/
|   |-- protocol/
|   |   |-- Cargo.toml
|   |   `-- src/
|   `-- execution/
|       |-- Cargo.toml
|       `-- src/
```

## 5. Open Questions

- None.
