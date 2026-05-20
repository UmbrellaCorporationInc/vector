---
id: rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary
type: rfc
code: "00011"
slug: mcp-vector-rmcp-dependency-and-thin-tooling-boundary
title: MCP Vector RMCP Dependency and Thin Tooling Boundary
description: Approves rmcp for mcp/vector only and defines the thin MCP tooling boundary over runtime plugin operations.
status: implemented
created: 2026-05-04
updated: 2026-05-04
authors: []
tags:
  - mcp
  - dependency
  - architecture
  - runtime
related:
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
  - rfc-00007-runtime-core-plugin-primitives
  - rfc-00008-runtime-channel-plugin-dispatcher-builder
  - project-0003-rust-dependencies
supersedes: []
superseded_by: null
aliases:
  - "RFC 00011: MCP Vector RMCP Dependency and Thin Tooling Boundary"
---

# RFC 00011: MCP Vector RMCP Dependency and Thin Tooling Boundary

## 1. Problem

`mcp/vector` needs one Rust MCP integration layer, but the project has already accepted that reusable behavior must live outside the MCP facade in [[rfc-00001-thin-mcp-facade-over-runtime-libraries]].

Two design pressures now exist at the same time:

- `mcp/vector` needs one practical MCP server dependency rather than a protocol implementation built from scratch
- the project wants extensibility through segregated `PluginOperation` crates and runtime dispatch, not through MCP-specific logic spread across the repository

Without an explicit dependency and ownership decision, the implementation will drift in one of two bad directions:

- `mcp/vector` becomes a protocol-plus-business-logic package
- reusable runtime crates become coupled to MCP SDK types and transport concerns

The project therefore needs one explicit decision for:

- whether `rmcp` is approved
- where `rmcp` is allowed to live
- whether tools are defined in `mcp/vector` or distributed across runtime crates
- how MCP tools are expected to call reusable plugin execution flows

## 2. Proposal

Approve `rmcp` as the MCP SDK dependency for `mcp/vector` only.

After this RFC is accepted:

- `mcp/vector` may depend on `rmcp`
- `runtime/*` crates and plugin crates must not depend on `rmcp`
- MCP tools are defined inside `mcp/vector`
- reusable capability logic lives in runtime and plugin crates through `PluginOperation` and runtime dispatch
- `mcp/vector` adapts MCP requests into runtime requests and maps runtime results into MCP responses

This RFC follows [[rfc-00001-thin-mcp-facade-over-runtime-libraries]] and narrows its implementation direction for the MCP server package.

### Approved dependency

`rmcp` is approved as the MCP SDK for `mcp/vector`.

Approved scope:

- `mcp/vector` only

Not approved by this RFC:

- `rmcp` in `runtime-core`
- `rmcp` in `runtime-channel`
- `rmcp` in `runtime-io`
- `rmcp` in plugin crates that own reusable operations

Implementation intent:

- use the minimum `rmcp` feature set needed for server behavior
- avoid unnecessary client-side or optional protocol features until a concrete use case requires them
- keep `rmcp` types at the protocol edge instead of leaking them into runtime contracts

### Tool ownership boundary

`mcp/vector` owns:

- MCP server bootstrap
- MCP transport setup
- MCP tool registration
- MCP request decoding
- MCP response encoding
- MCP-specific error mapping
- tool-layer validation that exists only because of MCP protocol shape

`mcp/vector` does not own:

- reusable repository logic
- reusable file or path logic
- reusable plugin execution logic
- reusable dispatcher logic
- reusable business workflows
- reusable validation rules that are not MCP-specific

### Runtime and plugin ownership boundary

Runtime and plugin crates own:

- `PluginOperation` definitions
- transport-agnostic domain logic
- runtime dispatch flows
- execution orchestration reusable outside MCP
- shared result and error boundaries

Runtime and plugin crates do not own:

- MCP request or response schemas
- MCP tool metadata
- MCP transport lifecycle
- MCP SDK adapter code

### Extensibility model

The approved extensibility model is runtime-first, not MCP-first.

That means:

- new capabilities may be added in separate runtime or plugin crates
- those capabilities expose reusable operations through accepted runtime contracts
- `mcp/vector` composes those capabilities by defining MCP tools that call the dispatcher and selected plugin operations

This RFC explicitly rejects the idea that extensibility should happen primarily by distributing MCP tool definitions across separate crates.

The main point of extensibility is:

- separate crates for `PluginOperation` and runtime logic

The MCP facade remains centralized:

- `mcp/vector` defines the tool surface

### Dispatcher integration direction

`mcp/vector` should use the runtime channel plugin dispatcher path as the execution bridge between MCP tools and reusable plugin operations.

Illustrative direction:

1. MCP request enters `mcp/vector`
2. MCP adapter validates and translates protocol input
3. MCP tool selects one runtime capability or `PluginOperation`
4. `runtime-channel` dispatcher executes the selected operation
5. runtime result is translated back into MCP response content

This keeps MCP adaptation above the runtime execution boundary rather than inside it.

### Dependency governance effect

If this RFC is accepted, [[project-0003-rust-dependencies]] should be updated so that:

- `rmcp` becomes an approved dependency
- its scope is limited to `mcp/vector`

This RFC does not approve `rmcp` as a general-purpose dependency for all Rust crates in the repository.

## 3. Alternatives Considered

- **Build MCP support without `rmcp`:** Discarded because it would force the project to maintain its own protocol implementation and increases compatibility risk for little architectural benefit.
- **Distribute MCP tool definitions across multiple crates:** Discarded because it spreads MCP SDK coupling across reusable layers and weakens the thin-facade boundary accepted in [[rfc-00001-thin-mcp-facade-over-runtime-libraries]].
- **Put `rmcp` into runtime crates directly:** Discarded because it would invert the accepted dependency direction and make reusable runtime packages protocol-aware.
- **Use `rmcp` only in `mcp/vector`, but also let plugin crates publish MCP adapters:** Discarded for now because it creates two competing extensibility models and blurs ownership before the first MCP package exists.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Using the official Rust MCP SDK reduces protocol implementation risk. | `mcp/vector` takes one external dependency with its own release cadence and upgrade cost. |
| Keeping `rmcp` in `mcp/vector` preserves the thin-facade boundary over runtime crates. | MCP tool definitions remain centralized instead of being distributed across crates. |
| Runtime extensibility stays focused on `PluginOperation` and dispatcher composition rather than protocol adapters. | Adding a new capability still requires touching `mcp/vector` to register one new tool. |
| Runtime crates stay transport-agnostic and reusable from CLI or future editor frontends. | The MCP crate must do more translation work because protocol types stop at the facade boundary. |
| The dependency direction remains simple: MCP depends on runtime, never the inverse. | If future MCP composition needs become more dynamic, this RFC may later need a companion adapter pattern. |

## 5. Acceptance Criteria

- [x] `rmcp` is approved as a Rust dependency for `mcp/vector`.
- [x] `rmcp` is not approved for `runtime/*` crates through this RFC.
- [x] The project documents that MCP tools are defined in `mcp/vector`.
- [x] The project documents that reusable capability logic lives in runtime or plugin crates.
- [x] The project documents that `mcp/vector` uses runtime dispatch and `PluginOperation` as its execution bridge.
- [x] The accepted architecture keeps MCP SDK types out of runtime contracts.
- [x] The accepted architecture introduces no requirement for plugin crates to depend on `rmcp`.
- [x] [[project-0003-rust-dependencies]] can be updated consistently with this scope restriction.

## 6. Open Questions

- What exact `rmcp` feature set should v1 enable for `mcp/vector` beyond the minimum server path?
- Should `mcp/vector` define its tools directly in one crate module tree, or use one internal adapter submodule per capability for maintainability?
- What runtime input and output shape should one MCP tool use when bridging into `PluginOperation` through the dispatcher?
- Should `mcp/vector` expose prompts or resources in v1, or start with tools only?
