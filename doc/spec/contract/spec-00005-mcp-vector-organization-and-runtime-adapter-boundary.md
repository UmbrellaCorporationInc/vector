---
id: spec-00005-mcp-vector-organization-and-runtime-adapter-boundary
type: spec
code: "00005"
slug: mcp-vector-organization-and-runtime-adapter-boundary
title: MCP Vector Organization and Runtime Adapter Boundary
description: Defines the ownership, module organization, and runtime adapter boundary for the mcp/vector crate.
category: contract
created: 2026-05-06
updated: 2026-05-07
authors: []
tags:
  - mcp
  - vector
  - structure
  - boundary
  - runtime
related:
  - spec-00001-repository-directory-structure
  - spec-00002-runtime-core-crate
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
  - rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00005: MCP Vector Organization and Runtime Adapter Boundary"
---

# SPEC 00005: MCP Vector Organization and Runtime Adapter Boundary

## 1. Purpose

This spec defines how `mcp/vector/` must organize its MCP-facing code and how it must connect that code to reusable runtime capabilities.

This spec follows [[spec-00001-repository-directory-structure]], [[spec-00002-runtime-core-crate]], [[rfc-00001-thin-mcp-facade-over-runtime-libraries]], and [[rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary]].

## 2. Definition

`mcp/vector/` is the canonical MCP server crate for the repository. It owns the complete MCP protocol surface for the `vector` server.

For the current repository contract, `mcp/vector` is tool-only. MCP prompt surfaces are not part of the supported server boundary at this stage, even when runtime operations return resolved authoring text.

Owned by `mcp/vector`:

- MCP server bootstrap
- MCP transport setup
- MCP tool registration
- MCP request decoding
- MCP response encoding
- MCP-specific error mapping
- MCP-only validation driven by protocol shape
- module grouping for tools by capability domain

Not owned by `mcp/vector`:

- reusable document logic
- reusable project logic
- reusable plugin execution logic
- transport-agnostic runtime orchestration
- reusable business workflows
- reusable validation rules that are not MCP-specific
- `PluginOperation` definitions

Owned by `runtime/*` crates:

- `PluginOperation` definitions
- operation input and output types
- transport-agnostic reusable capability logic
- shared runtime error and result boundaries
- dispatcher-oriented execution building blocks below the MCP facade

### 2.1. Dependency direction

Allowed dependency direction:

- `mcp/vector` may depend on `runtime-core`
- `mcp/vector` may depend on higher-level `runtime/*` crates
- `mcp/vector` may depend on `rmcp`
- `runtime/*` crates must not depend on `mcp/vector`
- `runtime/*` crates must not depend on `rmcp`

### 2.2. Internal organization

`mcp/vector` should organize MCP-facing code by capability domain.

Recommended module layout:

```text
mcp/vector/
|-- Cargo.toml
`-- src/
    |-- lib.rs
    |-- server.rs
    |-- error.rs
    |-- tools/
    |   |-- mod.rs
    |   |-- doc.rs
    |   `-- project.rs
```

Meaning of those modules:

- `server.rs` composes the final MCP server surface from tool routers and transport bootstrap.
- `tools/<domain>.rs` defines one MCP-facing tool group struct such as `DocTools`.
- `error.rs` owns MCP-local error adaptation when the crate needs a dedicated facade error boundary.

### 2.3. Tool groups

Each capability domain may define one MCP tool group struct inside `mcp/vector`.

Examples:

- `DocTools`
- `ProjectTools`
- `VaultTools`

These structs exist to group MCP tool definitions and to keep tool registration maintainable as the MCP surface grows.

The grouping struct is an MCP adapter, not a reusable runtime abstraction.

### 2.4. Tool execution pattern

An MCP tool in `mcp/vector` must:

1. accept MCP parameters
2. perform MCP-local validation only when required by protocol shape
3. construct or select one accepted runtime `PluginOperation`
4. execute that operation through the dispatcher path
5. collect runtime outputs from the returned receiver
6. translate runtime success or failure into MCP response content

The tool must not absorb the reusable logic that belongs in the runtime operation.

If a runtime operation returns resolved authoring instructions as plain text, `mcp/vector` must expose that output through a tool result rather than through an MCP prompt surface.

### 2.5. Dispatcher bridge

`mcp/vector` uses the runtime dispatcher path as the execution bridge between MCP tools and reusable plugin operations.

At the architectural level:

1. the MCP adapter receives the request
2. the MCP adapter maps the request into runtime input
3. the MCP adapter selects one runtime operation
4. the dispatcher executes that operation
5. the MCP adapter reads runtime output
6. the MCP adapter maps the result into the MCP response

This keeps `mcp/vector` as a thin facade over runtime capabilities rather than a second home for reusable business logic.

## 3. Invariants

- `mcp/vector` must remain the only crate that owns `rmcp`-backed MCP adapters for the `vector` server.
- `runtime/*` crates must not define `#[tool]`, `#[tool_router]`, `#[prompt]`, or `#[prompt_router]` surfaces.
- `runtime/*` crates must not depend on `rmcp`.
- `rmcp` types must not appear in runtime contracts, runtime traits, or runtime operation signatures.
- Every MCP tool group struct such as `DocTools` must live in `mcp/vector`.
- `mcp/vector` must not expose MCP prompt surfaces in the current contract.
- `PluginOperation` definitions must live outside `mcp/vector`.
- MCP adapters may translate between protocol-specific shapes and runtime shapes, but they must not become the primary home of reusable domain rules.
- New capability domains must be added by introducing or extending runtime crates for reusable logic and then exposing that logic through MCP group structs in `mcp/vector`.
- Tool registration may be grouped by multiple structs, but the final MCP server surface must be composed centrally inside `mcp/vector`.

## 4. Examples

### 4.1. Example: runtime operation lives outside MCP

```rust
use runtime_core::plugin::PluginSender;
use runtime_core::result::RuntimeResult;

pub struct ValidateDocumentOp;

impl<S> runtime_core::plugin::PluginOperation<S> for ValidateDocumentOp
where
    S: PluginSender<String>,
{
    type Input = String;
    type Output = String;
}

impl<S> runtime_core::operation::FlowOperation<String, String, S> for ValidateDocumentOp
where
    S: PluginSender<String>,
{
    async fn run(&self, input: String, output: &mut S) -> RuntimeResult<()> {
        output.send(format!("validated:{input}")).await
    }
}
```

This operation is reusable outside MCP. It contains no `rmcp` types and no MCP metadata.

### 4.2. Example: `DocTools` lives in `mcp/vector`

```rust
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::{tool, tool_handler, tool_router};
use runtime_channel::PluginDispatcher;

pub struct DocTools;

#[tool_router]
impl DocTools {
    #[tool(description = "Validate one document input")]
    async fn validate_document(&self, input: String) -> Result<String, String> {
        let operation = runtime_doc::ValidateDocumentOp;

        let (_handler, mut receiver) = PluginDispatcher::new(operation)
            .input(input)
            .build()
            .map_err(|error| error.to_string())?;

        let mut result = String::new();
        while let Some(chunk) = receiver.recv().await {
            result.push_str(&chunk);
        }

        Ok(result)
    }
}

#[tool_handler]
impl rmcp::ServerHandler for DocTools {
    fn tool_router(&self) -> ToolRouter<Self> {
        Self::tool_router()
    }
}
```

This example keeps MCP ownership inside `mcp/vector` and uses the dispatcher bridge for runtime execution.

### 4.3. Example: final composition happens centrally in `mcp/vector`

```rust
use rmcp::handler::server::router::tool::ToolRouter;

use crate::tools::doc::DocTools;
use crate::tools::project::ProjectTools;

pub struct VectorServer {
    doc_tools: DocTools,
    project_tools: ProjectTools,
}

impl VectorServer {
    pub fn tool_router(&self) -> ToolRouter<Self> {
        DocTools::tool_router() + ProjectTools::tool_router()
    }
}
```

The final router composition is centralized in `mcp/vector`, even when tools are grouped by domain.

### 4.4. Invalid examples

Invalid:

- `runtime-doc` defines `DocTools`
- `runtime-doc` imports `rmcp`
- `mcp/vector` exposes MCP prompt surfaces before the repository contract allows them
- `mcp/vector` contains the reusable document validation algorithm itself
- `PluginOperation` input or output types include `rmcp` protocol types

## 5. Open Questions

- Should `mcp/vector` standardize one facade-local error enum before the first real tool is added?
- When prompt-capable MCP clients become a hard requirement, should prompt surfaces be introduced through a new RFC and a revision of this spec?
