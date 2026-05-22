---
id: rfc-00026-mcp-vector-usability-improvements
type: rfc
code: "00026"
slug: mcp-vector-usability-improvements
title: MCP Vector Usability Improvements
description: Proposes an MCP version introspection tool and a sibling CLI command to install or update the local mcp-vector binary.
status: accepted
created: 2026-05-21
updated: 2026-05-21
authors: []
tags:
  - mcp
  - cli
  - usability
  - release
related:
  - "spec-00001-repository-directory-structure"
  - "spec-00008-mcp-vector-release-process"
supersedes: []
superseded_by: null
aliases:
  - "RFC 00026: MCP Vector Usability Improvements"
---

# RFC 00026: MCP Vector Usability Improvements

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00026-mcp-vector-usability-improvements`
  document-type: task
  document-name: implement-rfc-00026-mcp-vector-usability-improvements
```

## 1. Problem

`mcp-vector` is distributed as a local binary installed through `cargo install --git`, but the repository does not provide a first-class way to answer two basic operator questions:

1. Which version of `mcp-vector` is currently running?
2. How should an operator update or install the local binary in a repeatable way?

That gap creates avoidable friction for both local development and editor integrations. Consumers can infer the intended version from the repository `Cargo.toml`, but the running MCP server cannot expose that version through its own tool surface. Separately, there is no sibling CLI entrypoint that can check the local installation and reconcile it with the latest repository release.

The release and installation contract for `mcp-vector` is defined by `SPEC 00008: mcp-vector Release Process`, which uses `cargo install --git` and Git tags such as `v0.2.0`.

## 2. Proposal

Introduce two usability surfaces backed by release-aware logic owned by `mcp/vector`:

1. Add a `get_version` MCP tool in `mcp/vector` that returns the version declared in the workspace `Cargo.toml`, which is the single source of truth defined by `SPEC 00008`.
2. Add a sibling CLI crate at `frontend/cli/get-vector/` with a command surface under `frontend/cli/get-vector/commands/`, including an `update-mcp-vector` command.

The `update-mcp-vector` command V1 behavior:

- Always run `cargo install --git <repo> --force mcp-vector` to pull and compile the latest HEAD from the repository.
- No version comparison is performed in V1. The command is always a full reinstall from git.
- Version-aware reconciliation (detect installed version, compare against latest release, skip when already current) is deferred to a future task once a runtime strategy for resolving the latest published version is defined.

This approach eliminates the chicken-and-egg problem where a compile-time `INTENDED_VERSION` constant would cause an old CLI binary to always report `mcp-vector` as current even after the workspace has been bumped.

`mcp/vector` is the only owner of the workspace version source of truth (derived from the root `Cargo.toml`). This RFC does not introduce a shared release crate or a second version-runtime owner elsewhere in the workspace.

Process execution in `get-vector` must use the `CommandExecutor` trait, `CommandBuilder`, and `ProcessCommandExecutor` from `runtime-io` rather than defining a custom execution abstraction. `runtime-io` already provides the complete shell command boundary (`CommandExecutor` for the execution contract, `ProcessCommandExecutor` as the OS-backed production implementation, and `MockCommandHandleBuilder` for deterministic test handles). Tests must use `MockCommandHandleBuilder` from `runtime-io`.

This RFC intentionally does not extend the MCP server with self-update behavior. The MCP tool surface should expose metadata, not mutate the host environment.

## 3. Alternatives Considered

- **Keep version discovery out of MCP and document the Cargo.toml lookup only:** Discarded because it forces every consumer to reimplement version introspection and prevents editor or automation surfaces from querying the running server directly.
- **Implement update behavior inside `mcp/vector`:** Discarded because `SPEC 00001` separates MCP packages from CLI surfaces, and environment mutation from inside the MCP server increases operational risk and protocol-surface ambiguity.
## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Gives operators and integrations a canonical way to inspect the running MCP version. | Adds a new CLI crate that the repository does not currently have. |
| Aligns installation and update behavior with the existing `mcp-vector` release contract in `SPEC 00008`. | Keeps the CLI intentionally dependent on `mcp/vector` as the only release-logic owner, so boundaries must stay explicit to avoid accidental duplication. |
| Keeps version ownership unambiguous by making the CLI consume `mcp-vector --version` instead of an internal library API. | The `mcp-vector --version` output becomes a compatibility contract that must remain stable for CLI reconciliation. |
| Keeps the MCP server thin and avoids host mutation over the MCP protocol. | Introduces more surface area to test across OS environments and installation states. |
| Establishes the correct repository location for future operator commands under `frontend/cli/get-vector/commands/`. | The CLI contract must stay aligned with `SPEC 00008` so installation semantics remain stable. |

## 5. Acceptance Criteria

- [ ] `mcp/vector` exposes a `get_version` tool that returns the workspace version defined in the root `Cargo.toml`.
- [ ] The version returned by `get_version` is derived from the same source of truth used by `SPEC 00008`.
- [ ] A new CLI crate exists at `frontend/cli/get-vector/`.
- [ ] The CLI command surface includes `frontend/cli/get-vector/commands/update-mcp-vector`.
- [ ] `update-mcp-vector` always runs `cargo install --git <repo> --force mcp-vector` to install or update the binary from the latest git HEAD.
- [ ] No version comparison or install-state detection is performed in V1.
- [ ] Installation behavior follows the `cargo install --git` distribution model from `SPEC 00008`.
- [ ] No update or installation mutation is added to the MCP protocol surface itself.
- [ ] No shared release or version crate is introduced for this workflow.
- [ ] `get-vector` depends on `runtime-io` and uses `CommandExecutor`, `CommandBuilder`, and `ProcessCommandExecutor` for command execution; no custom execution abstraction is introduced.
- [ ] Tests in `get-vector` use `MockCommandHandleBuilder` from `runtime-io` and cover the success and failure cases of the install command.
## 6. Open Questions

- How should `update-mcp-vector` resolve the latest published version at runtime (GitHub Releases API, git tags, or another source) to enable version-aware reconciliation in V2? This is the prerequisite for skipping reinstallation when already current.
