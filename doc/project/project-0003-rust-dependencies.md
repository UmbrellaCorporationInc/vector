---
id: project-0003-rust-dependencies
type: project
code: "0003"
slug: rust-dependencies
title: Rust Dependencies
description: Defines the approved Rust dependencies for VECTOR project crates.
created: 2026-05-02
updated: 2026-05-06
tags:
  - project
  - rust
  - dependencies
related:
  - spec-00003-project-documentation-folder
  - task-00001-bootstrap-runtime-core-crate-and-rust-dependency-governance
  - task-00004-extract-tokio-backed-runtime-channel-crate-and-remove-runtime-core-channel-implementation
  - task-00011-bootstrap-mcp-vector-crate-and-approve-rmcp-dependency
---

# Dependencies

## 1. thiserror

Tags: #rust #error-handling
Scope: `runtime-core`, `runtime-channel`, `runtime-io`, `runtime-project`, `mcp-vector`
Description: Standard typed error derivation crate approved for library-style crates that expose stable error boundaries.

## 2. tokio

Tags: #rust #async #runtime
Scope: `runtime-channel`, `runtime-io`, test-only in `runtime-core`, `runtime-doc`, and `runtime-project`
Description: Tokio async runtime approved as the executor and async I/O backend where required. `runtime-channel` uses Tokio for channel transport. `runtime-io` uses Tokio directly for filesystem and process-backed I/O. Additional test-only Tokio usage is approved in crates that need async test execution.

## 3. paste

Tags: #rust #macros
Scope: `runtime-core`
Description: Macros for concatenating identifiers. Approved for use in `runtime-core` to enable ergonomic plugin declaration and token generation in `declare_plugin!`.

## 4. rmcp

Tags: #rust #mcp #protocol
Scope: `mcp-vector`
Description: Official Rust MCP SDK approved as the server integration dependency for the `mcp/vector` crate only. Approved by [[rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary]] and [[task-00011-bootstrap-mcp-vector-crate-and-approve-rmcp-dependency]]. Use the minimum feature set required for server behavior; avoid leaking `rmcp` types into runtime contracts.

## 5. serde

Tags: #rust #serialization
Scope: `runtime-doc`
Description: Serialization framework approved for structured document metadata and frontmatter decoding in `runtime-doc`.

## 6. serde_yaml

Tags: #rust #serialization #yaml
Scope: `runtime-doc`
Description: YAML codec approved for parsing governed Markdown frontmatter and related project document metadata in `runtime-doc`.

## 7. tempfile

Tags: #rust #testing #filesystem
Scope: test-only in `runtime-doc`
Description: Temporary filesystem fixture crate approved for isolated tests that need ephemeral files or directories.

## 8. walkdir

Tags: #rust #filesystem
Scope: `runtime-doc`
Description: Recursive directory traversal crate approved for project document discovery and vault scanning in `runtime-doc`.

## 9. regex

Tags: #rust #text-processing
Scope: `runtime-doc`
Description: Regular expression crate approved for governed document parsing and validation workflows in `runtime-doc`.

## 10. chrono

Tags: #rust #date-time
Scope: `runtime-doc`
Description: Date and time crate approved for document metadata handling and date-aware project document workflows in `runtime-doc`.

## 11. serde_json

Tags: #rust #serialization #json
Scope: test-only in `mcp-vector`
Description: JSON codec approved for test-only use in `mcp-vector` to deserialize MCP tool parameters in unit tests.

## 12. dunce

Tags: #rust #filesystem #windows
Scope: `runtime-doc`, test-only in `mcp-vector`
Description: Windows path normalization crate approved to strip the `\\?\` extended-length path prefix that `std::fs::canonicalize` emits on Windows. Used in `runtime-doc` (production) wherever governed document paths are returned to callers, and in `mcp-vector` tests (dev-only) to produce matching expected paths.

## Workspace-local dependencies

These workspace crates are used as internal dependencies and are governed by the workspace architecture rather than third-party dependency approval:

- `runtime-core`
- `runtime-io`
- `runtime-doc`
- `mcp-vector`

### Approved inter-crate dependency: `runtime-project` → `runtime-doc`

`runtime-project` is approved to depend on `runtime-doc` for the sole purpose of project setup composition. `ProjectSetupOp` in `runtime-project` invokes `ProjectExtensionSetupOp` from `runtime-doc` to complete the documentation-owned extension setup step as part of the composed project setup chain. This dependency is strictly one-directional: `runtime-doc` must not depend on `runtime-project`. Approved by [[task-00011-bootstrap-mcp-vector-crate-and-approve-rmcp-dependency]] Phase D.

## Governance notes

- `runtime-core` must remain transport-agnostic in production code and must not take a direct runtime dependency on any async executor or runtime backend. Its current Tokio usage is test-only.
- `runtime-channel` uses `tokio::sync::mpsc` (bounded) for message transport and `tokio::sync::watch` for cancellation wake-up.
- Channel capacity is configurable through a `runtime-channel` configuration value; the standard implementation must not default to unbounded channel transport.
- `runtime-io` is allowed to depend on Tokio because it owns the concrete async I/O boundary for filesystem and process-backed operations.
- Any additional Tokio-adjacent dependency beyond `tokio` itself must be justified separately in a future task or RFC if it is not strictly required by the current async runtime and I/O boundaries.
- `rmcp` is **not** approved for `runtime/*` crates or plugin crates. Only `mcp-vector` may take a direct dependency on `rmcp`. Runtime and plugin crates must remain MCP-SDK-agnostic so they stay reusable from CLI or future non-MCP frontends.
- `rmcp` types must not leak into runtime contracts. The MCP facade boundary stops at `mcp-vector`; all protocol types stay inside that crate.
- `runtime-doc` is currently the main consumer of document-processing dependencies (`serde`, `serde_yaml`, `walkdir`, `regex`, `chrono`). New document-stack crates should reuse that boundary instead of spreading parsing dependencies across the workspace without explicit justification.
- `runtime-project` depends on `runtime-doc` only for project setup composition through `ProjectExtensionSetupOp`. This is the only approved inter-crate dependency between these two runtime crates; all other cross-crate composition must be justified separately.
