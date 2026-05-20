---
id: rfc-00001-thin-mcp-facade-over-runtime-libraries
type: rfc
code: "00001"
slug: thin-mcp-facade-over-runtime-libraries
title: Thin MCP Facade Over Runtime Libraries
description: Proposes a thin MCP server that exposes tools backed by reusable runtime libraries shared with CLI and editor frontends.
status: implemented
created: 2026-05-01
updated: 2026-05-01
authors: []
tags:
  - mcp
  - architecture
  - runtime
related:
  - spec-00001-repository-directory-structure
supersedes: []
superseded_by: null
aliases:
  - "RFC 00001: Thin MCP Facade Over Runtime Libraries"
---

# RFC 00001: Thin MCP Facade Over Runtime Libraries

## 1. Problem

The repository needs an MCP server at `mcp/vector/`, but the same core capabilities must also be reusable from CLI frontends and future editor extensions. If the MCP server owns business logic directly, the project will duplicate behavior across `mcp/`, `frontend/cli/`, and `frontend/vscode/`, or force those frontends to depend on protocol-specific code. That creates avoidable coupling, inconsistent behavior, and higher maintenance cost.

This RFC follows [[spec-00001-repository-directory-structure]].

## 2. Proposal

Create `mcp/vector/` as a thin MCP facade over reusable runtime libraries under `runtime/`.

After this RFC is accepted:

- `runtime/` will own the reusable implementation of tool behavior, execution flows, domain contracts, and shared error boundaries.
- `mcp/vector/` will own MCP protocol integration only.
- `mcp/vector/` will register tools, translate MCP requests into runtime calls, and map runtime results into MCP responses.
- `frontend/cli/` packages will call the same `runtime/` libraries rather than reimplementing behavior.
- Future `frontend/vscode/` extensions will consume the same `runtime/` libraries directly or through stable frontend adapters.

Architectural boundary:

- `runtime/` must remain transport-agnostic.
- `mcp/vector/` must not become the home of reusable domain logic.
- MCP-specific schemas, server bootstrap, and protocol adapters belong in `mcp/vector/`.
- Validation, execution, orchestration, and shared contracts belong in `runtime/` when they are reusable beyond MCP.

Allowed ownership in `mcp/vector/`:

- MCP server bootstrap
- tool registration
- MCP request and response schemas
- protocol-specific error mapping
- protocol-specific logging and tracing metadata
- adapter code from MCP input and output types to runtime input and output types

Forbidden ownership in `mcp/vector/`:

- repository scanning logic
- document parsing logic
- indexing logic
- ranking logic
- validation rules shared with CLI or editor frontends
- business workflows reusable outside MCP

Allowed ownership in `runtime/`:

- repository scanning logic
- document parsing logic
- indexing logic
- ranking logic
- validation rules shared across MCP, CLI, or editor frontends
- business workflows reusable outside a single transport
- transport-agnostic domain errors and result types
- orchestration logic that can be called by more than one frontend

Initial package direction:

- `runtime/` may contain libraries such as execution, tool contracts, and shared domain modules.
- `mcp/vector/` should depend on `runtime/` packages, not the inverse.
- Frontends under `frontend/` should depend on `runtime/` packages, not on `mcp/vector/`.

## 3. Alternatives Considered

- **Self-contained MCP server:** Discarded because it centralizes logic inside `mcp/vector/` and makes CLI or VS Code reuse either duplicative or dependent on MCP internals.
- **Frontend-driven architecture with MCP calling CLI commands:** Discarded because process-boundary reuse is slower, harder to test, and weaker as a library contract than direct runtime dependencies.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Shared behavior can be reused by MCP, CLI, and VS Code frontends. | More upfront design work is required to define runtime boundaries. |
| Transport-agnostic runtime libraries are easier to test than protocol-bound handlers. | Some functionality must be split across packages instead of implemented in one place. |
| MCP remains small and focused on protocol adaptation. | Poor boundary discipline could still allow runtime leakage into the MCP layer. |
| Future frontends can be added without extracting MCP-owned logic later. | Initial delivery may be slower than a single-package prototype. |

## 5. Acceptance Criteria

- [ ] The repository contains `mcp/vector/` as the canonical MCP server package.
- [ ] The repository contains one or more `runtime/` packages that implement reusable tool behavior.
- [ ] `mcp/vector/` depends on `runtime/` packages for tool execution logic.
- [ ] `runtime/` packages do not depend on MCP protocol crates or MCP transport-specific modules.
- [ ] At least one CLI package under `frontend/cli/` can invoke the same runtime capability exposed by the MCP server.
- [ ] The project documents the ownership boundary between `runtime/` and `mcp/vector/`.
- [ ] Tests cover runtime behavior independently from MCP transport integration.

## 6. Open Questions

- Which first runtime package should be extracted before implementing the initial MCP tool set?
