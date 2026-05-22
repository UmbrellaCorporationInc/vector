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

The `update-mcp-vector` command should:

- detect whether `mcp-vector` is installed locally;
- resolve the currently installed version by invoking `mcp-vector --version` when present;
- resolve the latest intended repository version from the release contract in `SPEC 00008`;
- install when the binary is missing;
- update when the installed version differs from the latest release;
- perform no mutation when the local installation is already current.

`mcp/vector` is the only owner of the workspace version source of truth (derived from the root `Cargo.toml`). Install-state resolution logic — including the process-execution abstraction (`CommandRunner`), version-output parsing, and the `Missing`/`Outdated`/`Current` states — belongs entirely inside the `get-vector` CLI crate and must not be placed in `mcp/vector`. This RFC does not introduce a shared release crate or a second version-runtime owner elsewhere in the workspace. For installed-version inspection, the CLI must invoke `mcp-vector --version` and treat that process output as the stable contract.

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
- [ ] `update-mcp-vector` installs `mcp-vector` when the binary is not present locally.
- [ ] `update-mcp-vector` determines the installed version by invoking `mcp-vector --version`.
- [ ] `update-mcp-vector` updates the local installation when its version differs from the latest released version.
- [ ] `update-mcp-vector` exits without reinstalling when the local installation is already current.
- [ ] Installation and update behavior follow the `cargo install --git` distribution model from `SPEC 00008`.
- [ ] No update or installation mutation is added to the MCP protocol surface itself.
- [ ] No shared release or version crate is introduced for this workflow.
- [ ] `mcp/vector` owns only the workspace version source of truth; install-state resolution lives in `get-vector`.
- [ ] `get-vector` defines its own `CommandRunner` abstraction and install-state types internally.
- [ ] Tests in `get-vector` cover at least the missing-installation, outdated-installation, and already-current cases.
## 6. Open Questions

- Should `mcp/vector` resolve the latest version from Git tags, GitHub releases, or another repository-controlled metadata source?
- Should `update-mcp-vector` support explicit version pinning in addition to "latest" reconciliation?
