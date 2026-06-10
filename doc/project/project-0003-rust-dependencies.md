---
id: project-0003-rust-dependencies
type: project
code: "0003"
slug: rust-dependencies
title: Rust Dependencies
description: Defines the approved Rust dependencies for VECTOR project crates.
created: 2026-05-02
updated: 2026-06-10
tags:
  - project
  - rust
  - dependencies
related:
  - spec-00003-project-documentation-folder
  - task-00001-bootstrap-runtime-core-crate-and-rust-dependency-governance
  - task-00004-extract-tokio-backed-runtime-channel-crate-and-remove-runtime-core-channel-implementation
  - task-00011-bootstrap-mcp-vector-crate-and-approve-rmcp-dependency
  - task-00059-improve-runtime-io-directory-traversal
---

# Dependencies

## 1. thiserror

Tags: #rust #error-handling
Scope: `runtime-core`, `runtime-channel`, `runtime-io`, `runtime-project`, `runtime-doc`, `runtime-language`, `runtime-packages`, `mcp-vector`, `get-vector`, `vector-database`
Description: Standard typed error derivation crate approved for crates that expose stable error boundaries.

## 2. tokio

Tags: #rust #async #runtime
Scope: `runtime-channel`, `runtime-io`, `mcp-vector`, `get-vector`, `vector-database`, test-only in `runtime-core`, `runtime-doc`, `runtime-project`, `runtime-language`, and `runtime-packages`
Description: Tokio async runtime approved as the executor and async I/O backend where required. `runtime-channel` uses Tokio for channel transport. `runtime-io` uses Tokio directly for filesystem and process-backed I/O. `mcp-vector` uses Tokio at the MCP facade boundary. CLI crates may use Tokio when they own the concrete async entrypoint. Additional test-only Tokio usage is approved in crates that need async test execution.

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
Scope: `runtime-doc`, `runtime-language`, `runtime-packages`, `runtime-markdown`, `mcp-vector`
Description: Serialization framework approved for structured document metadata, package and language metadata decoding, Markdown extraction records, and MCP-facing DTO handling.

## 6. serde_yaml

Tags: #rust #serialization #yaml
Scope: `runtime-doc`, `runtime-language`, `runtime-packages`, `runtime-markdown`
Description: YAML codec approved for parsing governed Markdown frontmatter, Markdown extraction frontmatter metadata, and related project, language, and package metadata.

## 7. tempfile

Tags: #rust #testing #filesystem
Scope: test-only in `runtime-doc`, `runtime-project`, `runtime-language`, `runtime-packages`, `mcp-vector`, and `vector-database`
Description: Temporary filesystem fixture crate approved for isolated tests that need ephemeral files or directories.

## 8. walkdir

Tags: #rust #filesystem
Scope: `runtime-doc`, `runtime-language`
Description: Recursive directory traversal crate approved for project document discovery, vault scanning, and language asset discovery.

## 9. regex

Tags: #rust #text-processing
Scope: `runtime-doc`
Description: Regular expression crate approved for governed document parsing and validation workflows in `runtime-doc`.

## 10. chrono

Tags: #rust #date-time
Scope: `runtime-doc`
Description: Date and time crate approved for document metadata handling and date-aware governed document workflows in `runtime-doc`.

## 11. serde_json

Tags: #rust #serialization #json
Scope: `mcp-vector`
Description: JSON codec approved for use in `mcp-vector` to serialize and deserialize MCP tool inputs, outputs, and parameters.

## 12. dunce

Tags: #rust #filesystem #windows
Scope: `runtime-doc`, test-only in `mcp-vector`
Description: Windows path normalization crate approved to strip the `\\?\` extended-length path prefix that `std::fs::canonicalize` emits on Windows. Used in `runtime-doc` (production) wherever governed document paths are returned to callers, and in `mcp-vector` tests (dev-only) to produce matching expected paths.

## 13. patcher

Tags: #rust #diff #patching
Scope: `runtime-doc`
Description: Git-style unified diff patch generation and application crate approved for document patching operations in `runtime-doc`. Used by the `patch_doc` authoring operation to apply governed document updates from git diffs. Approved by [[rfc-00028-authoring-document-operations]] and [[task-00052-implement-rfc-00028-authoring-document-operations]].

## 14. terminal_size

Tags: #rust #cli #terminal
Scope: `get-vector`
Description: Retrieve terminal size dynamically. Approved for use in `get-vector` to dynamically format outputs based on the active terminal columns. Approved by [[task-00056-update-get-vector-to-calculate-the-terminal-size]].

## 15. blake3

Tags: #rust #hashing #filesystem
Scope: `runtime-io`
Description: BLAKE3 content hashing crate approved for `runtime-io` file-byte hashing primitives. The approved use is limited to hashing file bytes through the generic IO boundary; callers must not include paths, modified times, package identity, Markdown metadata, or other domain data in the hash input.

## Workspace-local dependencies

These workspace crates are used as internal dependencies and are governed by the workspace architecture rather than third-party dependency approval:

- `runtime-core`
- `runtime-channel`
- `runtime-io`
- `runtime-doc`
- `runtime-language`
- `runtime-packages`
- `runtime-project`
- `mcp-vector`

Workspace members `get-vector` and `vector-database` are application crates, not shared library dependencies. They may depend on approved runtime crates without needing separate third-party approval entries.

### Approved inter-crate dependency: `runtime-project` -> `runtime-doc`

`runtime-project` is approved to depend on `runtime-doc` for the sole purpose of project setup composition. `ProjectSetupOp` in `runtime-project` invokes `ProjectExtensionSetupOp` from `runtime-doc` to complete the documentation-owned extension setup step as part of the composed project setup chain. This dependency is strictly one-directional: `runtime-doc` must not depend on `runtime-project`. Approved by [[task-00011-bootstrap-mcp-vector-crate-and-approve-rmcp-dependency]] Phase D.

## Governance notes

- `runtime-core` must remain transport-agnostic in production code and must not take a direct runtime dependency on any async executor or runtime backend. Its current Tokio usage is test-only.
- `runtime-channel` uses `tokio::sync::mpsc` (bounded) for message transport and `tokio::sync::watch` for cancellation wake-up.
- Channel capacity is configurable through a `runtime-channel` configuration value; the standard implementation must not default to unbounded channel transport.
- `runtime-io` is allowed to depend on Tokio because it owns the concrete async I/O boundary for filesystem and process-backed operations.
- `runtime-io` directory traversal is implemented with Tokio filesystem primitives. No traversal-specific third-party dependency is approved for `runtime-io` by [[task-00059-improve-runtime-io-directory-traversal]].
- Any additional Tokio-adjacent dependency beyond `tokio` itself must be justified separately in a future task or RFC if it is not strictly required by the current async runtime and I/O boundaries.
- `rmcp` is **not** approved for `runtime/*` crates or plugin crates. Only `mcp-vector` may take a direct dependency on `rmcp`. Runtime and plugin crates must remain MCP-SDK-agnostic so they stay reusable from CLI or future non-MCP frontends.
- `rmcp` types must not leak into runtime contracts. The MCP facade boundary stops at `mcp-vector`; all protocol types stay inside that crate.
- `runtime-doc` remains the main consumer of document-processing dependencies (`serde`, `serde_yaml`, `walkdir`, `regex`, `chrono`, `patcher`, `dunce`). New crates should reuse existing runtime boundaries instead of spreading parsing or patching dependencies across the workspace without explicit justification.
- `runtime-language` and `runtime-packages` are approved to use `serde`, `serde_yaml`, and adjacent filesystem helpers only for their own metadata-loading boundaries. They must not duplicate governed document parsing responsibilities that belong in `runtime-doc` without explicit justification.
- `runtime-project` depends on `runtime-doc` only for project setup composition through `ProjectExtensionSetupOp`. This is the only approved dependency from `runtime-project` to `runtime-doc`; any broader cross-crate composition must be justified separately.
- CLI crates (`get-vector`, `vector-database`) may depend on `runtime-io`, Tokio, and approved runtime crates because they own the executable boundary. They should remain thin entrypoints and must not become the primary home of reusable domain logic.
