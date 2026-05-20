---
id: spec-00002-runtime-core-crate
type: spec
code: "00002"
slug: runtime-core-crate
title: Runtime Core Crate
description: Defines the purpose and boundaries of the runtime-core crate as the shared contracts and utilities foundation for runtime packages.
category: contract
created: 2026-05-01
updated: 2026-05-01
authors: []
tags:
  - runtime
  - crate
  - foundation
related:
  - spec-00001-repository-directory-structure
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00002: Runtime Core Crate"
---

# SPEC 00002: Runtime Core Crate

## 1. Purpose

This spec defines the contract for `runtime/core/` as the directory that contains the foundational `runtime-core` crate for shared contracts and utilities used by other runtime crates.

This spec follows [[spec-00001-repository-directory-structure]] and supports [[rfc-00001-thin-mcp-facade-over-runtime-libraries]].

## 2. Definition

`runtime/core/` contains the `runtime-core` crate, which is the lowest-level shared runtime crate.

`runtime-core` exists to provide stable foundational building blocks that other crates under `runtime/` can depend on without introducing feature-specific behavior.

Allowed ownership in `runtime-core`:

- shared traits
- shared domain-neutral interfaces
- common result and error helpers
- reusable utility types
- reusable helper functions with no feature-specific policy
- stable crate-wide primitives used by more specialized runtime crates

Forbidden ownership in `runtime-core`:

- repository scanning logic
- document parsing logic
- indexing logic
- ranking logic
- MCP protocol types
- CLI-specific command logic
- VS Code extension-specific logic
- business workflows
- feature-specific orchestration

Dependency direction:

- other crates under `runtime/` may depend on `runtime-core`
- `mcp/vector/` may depend on `runtime-core` directly or indirectly through higher-level runtime crates
- crates under `frontend/` may depend on `runtime-core` directly or indirectly through higher-level runtime crates
- `runtime-core` must not depend on crates under `mcp/`
- `runtime-core` must not depend on crates under `frontend/`

Feature ownership:

- concrete capabilities must be defined in separate crates under `runtime/`
- functionality beyond shared contracts and utilities must be proposed in dedicated RFCs

## 3. Invariants

- `runtime/core/` must contain the `runtime-core` crate and must represent one primary responsibility: foundational shared runtime contracts and utilities.
- `runtime-core` must remain transport-agnostic.
- `runtime-core` must not own protocol adapters or transport-specific schemas.
- `runtime-core` must not become a catch-all crate for unrelated shared code.
- Any logic in `runtime-core` must be reusable by more than one higher-level consumer or represent a foundational contract required by the runtime layer.
- Feature-specific behavior must live outside `runtime-core`.
- New runtime functionality must not be added to `runtime-core` merely to avoid creating a dedicated crate.

## 4. Examples

```text
runtime/
`-- core/
    |-- Cargo.toml
    `-- src/
        |-- error.rs
        |-- result.rs
        |-- traits.rs
        |-- types.rs
        `-- lib.rs
```

Valid examples of `runtime-core` contents:

- a shared trait for runtime services
- a transport-agnostic error type used by multiple runtime crates
- a small utility type used by indexing and parsing crates

Invalid examples of `runtime-core` contents:

- an MCP request handler
- a CLI command implementation
- a repository index builder
- a document ranking pipeline

## 5. Open Questions

- None.
