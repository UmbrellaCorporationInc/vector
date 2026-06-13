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
| `runtime-doc`     | Document capability operations: `ValidateOp`, `FindDocOp`, `CreateDocOp`, `CreateDocTypeOp`, `PatchDocOp`, `ReplaceDocOp`, and their corresponding input/output types |
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
| `find_doc`              | Document          | Locate a governed document by type and numeric code; returns `path`, `package` (if resolved from a package), and `content` |
| `create_doc_prompt`     | Document          | Create a governed document and return the resolved authoring prompt |
| `create_doc_type_prompt`| Document          | Create a governed document type and return the resolved authoring prompt |
| `patch_doc`             | Document          | Apply a patch to a governed document; recommended agent path is omitted-format `apply_patch` (no numeric hunk headers required); `format: "unified"` is still supported for source-control-native diffs; enforces target matching, returns format-specific diagnostics, rejects BOM-encoded output, and returns the final patched content |
| `replace_doc`           | Document          | Replace a governed document with complete content; resolves the target from `doc_type`, `code`, and optional `package`, validates governed front matter identity, rejects BOM content, and returns the resolved path and final content. Bootstrap companion to `create_doc_prompt`. |
| `language_quality_gate` | Language          | Resolve and concatenate governed quality-gate prompt bodies for a language list |
| `language_best_practices` | Language        | Resolve and concatenate governed best-practices prompt bodies for a language list |

### `find_doc` response contract

`find_doc` returns a JSON string response representing a serialized `FindDocResponse` object with the following fields:

- `path` (string): The absolute canonicalized path to the located document.
- `package` (string): The synchronized package name, or an empty string for workspace-local documents.
- `content` (string): The full raw content of the document.

Example JSON response:
```json
{
  "path": "/absolute/path/to/doc.md",
  "package": "",
  "content": "--- \nmetadata...\n---\n..."
}
```

The `package` input parameter is used for package-qualified document lookup. When set to a package name (e.g., `"my-pkg"`), the document is resolved against the synchronized package at `.vector-database/packages/{package}/` instead of the active workspace. If the package is resolved from a synchronized package location, the output `package` field echoes the package name; otherwise, it is empty. Callers must handle both workspace-local and package-qualified lookup results; see RFC 00030.

### `patch_doc` input contract

`patch_doc` resolves the writable document from `root_dir`, `doc_type`, `code`, and optional `package`; callers do not provide a write path. Send patch content in `patch` and select the syntax with `format`. Supported format values are `unified` and `apply_patch`. When `format` is omitted, `patch` is parsed as `apply_patch`. The deprecated `git_diff` field remains a transition alias for `format: "unified"`.

**Recommended for agent-authored edits:** omit `format` and send an `apply_patch`-style payload. This is the safer default because it does not require numeric hunk headers or line-count arithmetic. Use `format: "unified"` only when you already have a source-control-native diff.

Recommended agent path — omitted-format `apply_patch` example:

```json
{
  "root_dir": "/path/to/project",
  "doc_type": "rfc",
  "code": 37,
  "patch": "*** Begin Patch\n*** Update File: doc/rfc/draft/rfc-00037-extend-patch-doc-formats.md\n@@\n-old\n+new\n*** End Patch\n"
}
```

Explicit unified diff example (for source-control-native callers):

Unified diff hunk headers use 1-based line indices — the first document line is line 1, not line 0. Always use the full `@@ -start,count +start,count @@` form. The path in `---` and `+++` lines must be the document path resolved from `doc_type`, `code`, and optional `package`; call `find_doc` to obtain it.

```json
{
  "root_dir": "/path/to/project",
  "doc_type": "rfc",
  "code": 37,
  "format": "unified",
  "patch": "--- a/doc/rfc/implemented/rfc-00037-extend-patch-doc-formats.md\n+++ b/doc/rfc/implemented/rfc-00037-extend-patch-doc-formats.md\n@@ -1,1 +1,1 @@\n-old\n+new\n"
}
```

### `replace_doc` input contract

`replace_doc` is the bootstrap companion to `create_doc_prompt`. After `create_doc_prompt` creates the governed document skeleton, call `replace_doc` to write the fully authored content without generating a patch against the placeholder template.

`replace_doc` resolves the writable document from `root_dir`, `doc_type`, `code`, and optional `package`. The caller provides the complete replacement `content`; the document path is not caller-supplied. The `content` must be valid UTF-8 without a BOM and must preserve the governed front matter identity fields — `id`, `type`, `code`, and `slug` — of the resolved document.

Bootstrap example — replacing a newly created RFC document:

```json
{
  "root_dir": "/path/to/project",
  "doc_type": "rfc",
  "code": 37,
  "content": "---\nid: rfc-00037-extend-patch-doc-formats\ntype: rfc\ncode: \"00037\"\nslug: extend-patch-doc-formats\ntitle: Extend Document Patch and Replacement Operations\ndescription: Proposes extending patch_doc and adding replace_doc.\nstatus: draft\ncreated: 2026-06-11\nupdated: 2026-06-11\nauthors: []\ntags: []\nrelated: []\n---\n\n# RFC 00037: Extend Document Patch and Replacement Operations\n\n## 1. Problem\n\n...\n"
}
```

`replace_doc` returns `path` and the final document content after a successful write.

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

## 5. CLI Commands

The executable can be run directly from the command line:

- **Start MCP Server**: Run without arguments (or with unrecognized arguments) to start the stdio MCP server:
  ```bash
  mcp-vector
  ```
- **Create Project**: Scaffold a new governed vector project with vault and workspace:
  ```bash
  mcp-vector create-project [name]
  ```
- **Version**: Print the workspace version:
  ```bash
  mcp-vector --version
  # or
  mcp-vector -V
  ```
- **Help**: Print usage and help information:
  ```bash
  mcp-vector --help
  # or
  mcp-vector -h
  ```

## 6. Dependency Boundary

This crate enforces the thin-facade boundary defined in RFC-00011 and RFC-00001:

- `rmcp` is approved for this crate only — `runtime/*` crates must not depend on it
- `rmcp` types must not appear in runtime contracts, traits, or operation signatures
- All reusable capability logic must live in `runtime/*` crates accessed through the dispatcher bridge

## 7. Non-Goals and Future Work

- Prompts and resources are not yet registered; tool-only surface is intentional at this phase
- Additional tool groups (`VaultTools`) will be added as capability domains are specified in separate tasks
- Dynamic tool registration or plugin-contributed MCP adapters are out of scope for this crate
