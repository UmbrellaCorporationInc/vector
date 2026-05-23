# `mcp-vector`

## 1. Objective

`mcp-vector` is the canonical MCP server crate for the vector system. It owns the complete MCP protocol surface — server bootstrap, transport setup, tool registration, request decoding, response encoding, and MCP-specific error mapping — while delegating all reusable capability logic to `runtime/*` crates. Its intended consumer is any MCP client that communicates with the vector system over the MCP protocol.

## 2. Boundaries

### In scope

- MCP server bootstrap and stdio transport lifecycle
- Tool registration and capability-domain grouping (`ProjectTools`, etc.)
- MCP request decoding and response encoding
- MCP-specific error adaptation (`VectorServerError`)
- Adapter code that translates between MCP protocol shapes and runtime input/output types
- MCP-only validation driven by protocol shape

### Out of scope

- Reusable project, document, or domain logic
- Transport-agnostic orchestration or business workflows
- `PluginOperation` definitions — those live in `runtime/*` crates
- Reusable validation rules shared with CLI or other frontends
- Dispatcher behavior — owned by `runtime-channel`

### Dependencies

| Dependency        | Role                                                               |
|-------------------|--------------------------------------------------------------------|
| `rmcp`            | MCP SDK for server bootstrap, tool routing, and protocol handling  |
| `runtime-core`    | Core plugin and channel contracts used at the dispatcher boundary  |
| `runtime-channel` | `PluginDispatcher` execution bridge between tools and operations   |
| `runtime-io`      | `IoPath` type used in MCP-to-runtime input translation             |
| `runtime-language`| `QualityGateOp`, `BestPracticesOp`, and their inputs for language prompt resolution |
| `runtime-project` | `ProjectSetupOp` and `ProjectSetupInput` for the project tool path |
| `serde`           | MCP parameter deserialization                                      |
| `thiserror`       | MCP-local error enum derivation                                    |

## 3. Public Interface

### Types

- `VectorServer` — central MCP server handler; composes all tool groups and owns the stdio transport lifecycle
- `VectorServerError` — MCP-local error enum for handshake and transport failures
- `ProjectTools` — MCP tool group for the project capability domain
- `DocumentTools` — MCP tool group for the document capability domain
- `LanguageTools` — MCP tool group for the language capability domain

### Key functions / constructors

- `VectorServer::new()` — construct the handler with all registered tool groups
- `VectorServer::serve_stdio()` — start the MCP server over stdio and run until the transport closes

### Registered MCP tools

| Tool                    | Capability domain | Description                                                        |
|-------------------------|-------------------|--------------------------------------------------------------------|
| `create_project`        | Project           | Scaffold a governed vector project with vault and workspace        |
| `update_project`        | Project           | Add missing governed assets to an existing project without overwriting existing files |
| `validate`              | Document          | Validate governed documentation against `document-types.yaml`      |
| `validate_fix`          | Document          | Validate governed documentation and apply auto-fixes for correctable issues |
| `find_doc`              | Document          | Locate a governed document by type and numeric code                |
| `create_doc_prompt`     | Document          | Create a governed document and return the resolved authoring prompt |
| `create_doc_type_prompt`| Document          | Create a governed document type and return the resolved authoring prompt |
| `language_quality_gate` | Language          | Resolve and concatenate governed quality-gate prompt bodies for a language list |
| `language_best_practices` | Language        | Resolve and concatenate governed best-practices prompt bodies for a language list |

## 4. Usage Example

```rust
use mcp_vector::server::VectorServer;

#[tokio::main]
async fn main() {
    VectorServer::new()
        .serve_stdio()
        .await
        .expect("MCP server failed");
}
```

## 5. Dependency Boundary

This crate enforces the thin-facade boundary defined in RFC-00011 and RFC-00001:

- `rmcp` is approved for this crate only — `runtime/*` crates must not depend on it
- `rmcp` types must not appear in runtime contracts, traits, or operation signatures
- All reusable capability logic must live in `runtime/*` crates accessed through the dispatcher bridge

## 6. Non-Goals and Future Work

- Prompts and resources are not yet registered; tool-only surface is intentional at this phase
- Additional tool groups (`VaultTools`) will be added as capability domains are specified in separate tasks
- Dynamic tool registration or plugin-contributed MCP adapters are out of scope for this crate
