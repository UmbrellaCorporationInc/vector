---
id: task-00016-implement-mcp-doc-validation-document-lookup-and-authoring-prompts
type: task
code: "00016"
slug: implement-mcp-doc-validation-document-lookup-and-authoring-prompts
title: Implement MCP Doc Validation, Document Lookup, and Authoring Prompts
description: Expose runtime-doc validation, document lookup, and authoring flows through MCP tools only.
status: done
created: 2026-05-06
updated: 2026-05-07
tags:
  - mcp
  - documentation
  - prompt
  - runtime
related:
  - task-00013-implement-rfc-00013-runtime-doc-validation-and-authoring-crate
  - rfc-00013-runtime-doc-validation-and-authoring-crate
  - rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary
supersedes: []
superseded_by: null
---

# Task 00016: Implement MCP Doc Validation, Document Lookup, and Authoring Prompts

## 1. Prime Directive

`runtime-doc` already owns transport-agnostic operations such as `validate`, `find_doc`,
`create_doc`, and `create_doc_type`, but the MCP facade does not yet expose the missing
document-governance tools consistently. This task closes that adapter gap so MCP callers can
validate the vault, locate governed documents, create governed documents, and create governed
document types through the existing `runtime-doc` operations without reimplementing prompt or
authoring logic in the MCP layer.

MCP prompt surfaces are out of scope for now. Even when a runtime operation returns resolved
authoring text, the MCP adapter must expose that flow as a tool result because current client
compatibility is tool-first and some agents do not support MCP prompts.

## 2. Specs

- **Crates touched:** `mcp/vector`, `runtime/doc`
- **Primary modules:** `mcp/vector/src/tools/`, `mcp/vector/src/server.rs`
- **Dependencies:** existing `runtime-doc` operations, `PluginDispatcher`, `rmcp`
- **Boundary:** keep `mcp/vector` as a thin adapter; business logic stays in `runtime-doc`
- **Scope constraint:** expose MCP tools only; do not add or preserve MCP prompt surfaces in this task

## 3. Checklist

### 3.1. Phase A - Expose `validate` as an MCP tool

- [x] Add a document-oriented MCP tool group under `mcp/vector/src/tools/`
- [x] Define MCP input params for `validate`:
  - `root_dir: String`
  - `fix: bool` with default `false`
- [x] Dispatch `ValidateOp` through the standard plugin dispatcher path
- [x] Keep the adapter transport-only; do not duplicate validation logic in `mcp/vector`
- [x] Add tests covering the MCP tool wiring and result mapping
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass

### 3.2. Phase B - Relax `template` id validation in `runtime-doc`

- [x] Update the `validate` operation in `runtime/doc` so the frontmatter `id` field accepts a `template`-specific exception
- [x] Keep the current governed `id` validation unchanged for every doc type except `template`
- [x] Only when `type: template`, allow the `id` field to use any of these shapes:
  - `template-<code>-<slug>`
  - `template-<valid-doc-type>-<code>-<slug>`
  - `template-<valid-slug>`
- [x] Treat `<valid-doc-type>` as any configured document type identifier accepted by the loaded document-types configuration
- [x] Treat `<slug>` as a value that satisfies the existing slug validator
- [x] Do not require the template `id` to equal the file stem; this exception applies only to the frontmatter `id`
- [x] During validation of `type: template`, treat the frontmatter `category` field as optional
- [x] Keep the existing `category` requirement unchanged for every non-template document type that uses category layout
- [x] Keep filename validation unchanged for template files
- [x] Add or update validation tests covering:
  - accepted `template-<code>-<slug>` ids
  - accepted `template-<valid-doc-type>-<code>-<slug>` ids
  - accepted `template-<valid-slug>` ids
  - accepted template documents without a `category` field
  - rejection when the embedded doc type is not a configured valid doc type
  - rejection when the suffix contains an invalid slug
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass

### 3.3. Phase C - Expose `find_doc` as an MCP tool

- [x] Define MCP input params for `find_doc`:
  - `root_dir: String`
  - `doc_type: String`
  - `code: u32` — numeric type, matching the `FindDocInput` contract in `runtime-doc`
- [x] Dispatch `FindDocOp` through the standard plugin dispatcher path
- [x] Ensure the adapter preserves the `runtime-doc` contract:
  - no file content loading
  - no naming-convention reimplementation inside the MCP layer
- [x] Add tests covering successful lookup and not-found behavior
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass

### 3.4. Phase D - Expose `create_doc` as an MCP tool

- [x] Define MCP input params for `create_doc` matching the existing `CreateDocInput` contract in `runtime-doc`
- [x] Dispatch `CreateDocOp` through the standard plugin dispatcher path
- [x] Return the existing operation output to the MCP caller as a tool result, including:
  - created file path
  - assigned code
  - resolved prompt returned by `CreateDocOp`
- [x] Do not create or mutate governed prompt assets in this task
- [x] Do not duplicate placeholder substitution or authoring logic inside `mcp/vector`
- [x] Remove any MCP prompt surface that exposes `create_doc` as a prompt so the flow is tool-only
- [x] Add tests covering MCP tool wiring and result mapping for `create_doc`
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass

### 3.5. Phase E - Improve `create_doc` error diagnostics before adding more authoring surfaces

- [x] Extend `runtime_core::RuntimeError::Operation` so runtime operations can carry a caller-meaningful message instead of only the current generic failure sentinel
- [x] Update impacted crates and tests that currently construct or match `RuntimeError::Operation` without payload:
  - `runtime/core`
  - `runtime/channel`
  - `runtime/io`
  - `runtime/project`
  - `runtime/doc`
  - any MCP adapter tests that assert generic fallback behavior
- [x] Add a consistent construction pattern for operation failures (`RuntimeError::operation(msg)`) so call sites do not hand-roll message formatting inconsistently
- [x] Audit the end-to-end `create_doc` failure path from `runtime-doc` through the MCP adapter and identify where specific causes are collapsed into generic operation failures
- [x] Replace generic `create_doc` runtime failures with caller-meaningful error messages for at least these cases:
  - invalid slug
  - unknown document type
  - missing `prompt` configuration on the target document type
  - missing prompt document on disk
  - prompt document unreadable
- [x] Keep the adapter thin:
  - error classification stays in `runtime-doc`
  - `mcp/vector` forwards the message without reimplementing business rules
- [x] Keep non-user-facing generic operation failures acceptable where the specific underlying cause is either unavailable or not worth surfacing, but make those sites explicit and intentional
- [x] Add or update tests covering runtime and MCP-level error reporting for the documented failure modes
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass
- **Known limitation:** The MCP `create_doc` tool cannot yet forward the `RuntimeError` message to the caller. `Receiver::recv()` returns `Option<T>` — `None` is indistinguishable from a normal empty completion. The error message is discarded by the dispatcher before reaching the adapter. This is resolved in Phase F.

### 3.6. Phase F - Refactor `Receiver::recv()` to surface operation errors through the channel

The dispatcher currently discards operation errors with `let _ = operation.run(...).await`. The receiver contract returns `Option<T>`, so `None` is indistinguishable from normal completion with no output. This phase refactors the channel contract so errors flow through the same channel, making them available to MCP adapters and other callers without any shared state or side channels.

- [x] Change `Receiver<T>::recv()` return type from `Option<T>` to `Result<Option<T>, RuntimeError>`:
  - `Ok(Some(value))` — a value was received
  - `Ok(None)` — channel closed normally (operation completed)
  - `Err(e)` — operation failed with a caller-meaningful error
- [x] Update the dispatcher to send the error through the channel instead of discarding it:
  - replace `let _ = operation.run(...).await` with error capture and forwarding
- [x] Update all `Receiver<T>` implementations in `runtime/channel`, `runtime/io`:
  - `TokioReceiver<T>`
  - `TokioCancelableReceiver<T>`
  - `FileReader`
  - `MemReader`
  - `CommandOutput`
  - `TextReader<R>`
- [x] Update `CancelableReceiver<T>` and `PluginReceiver<T>` accordingly
- [x] Update all `.recv()` call sites across production and test code to handle `Result<Option<T>, RuntimeError>`:
  - `mcp/vector/src/tools/document.rs`
  - `mcp/vector/src/tools/project.rs`
  - `runtime/channel/src/event.rs`
  - `runtime/io/src/file.rs`
  - `runtime/io/src/text.rs`
  - all test files with `.recv()` calls
- [x] Update the MCP `create_doc` tool to forward the `RuntimeError` message from `Err(e)` to the caller, replacing the current `"create_doc failed: see runtime logs..."` fallback
- [x] Update the `Reader<T>` alias in `runtime/io/src/alias.rs`
- [x] Add or update tests covering:
  - receiver yields `Err(e)` when the operation fails
  - MCP tool returns the specific error message from a failed operation
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass

### 3.7. Phase G - Expose `create_doc_type` as an MCP tool

- [x] Define MCP input params for `create_doc_type` matching the existing `CreateDocTypeInput` contract in `runtime-doc`
- [x] Dispatch `CreateDocTypeOp` through the standard plugin dispatcher path
- [x] Return the existing operation output to the MCP caller as a tool result, including:
  - created document type
  - selected layout
  - resolved prompt returned by `CreateDocTypeOp`
- [x] Do not create or mutate governed prompt assets in this task
- [x] Do not duplicate placeholder substitution or authoring logic inside `mcp/vector`
- [x] Add tests covering MCP tool wiring and result mapping for `create_doc_type`
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass

### 3.8. Phase G - Wire document tools into the MCP server surface

- [x] Register document tools in the MCP server so all document surfaces are discoverable by clients
- [x] Add integration tests proving the server exposes:
  - `validate`
  - `find_doc`
  - `create_doc`
  - `create_doc_type`
- [x] Confirm the project tool group continues to work unchanged
- [x] Confirm the server scope remains tool-only for now
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass

### 3.9. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified
- [x] Ensure MCP tool docs describe the same workflow the runtime actually implements

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] All quality gates pass
- [x] MCP clients can call `validate` and receive validation results without bypassing `runtime-doc`
- [x] MCP clients can call `find_doc` and receive the governed document absolute path without parsing file content
- [x] `runtime-doc` validation accepts the template-only `id` exception and keeps all non-template `id` validation unchanged
- [x] MCP clients can call `create_doc` as a tool and receive the resolved authoring payload already produced by `CreateDocOp`
- [x] MCP clients can call `create_doc_type` as a tool and receive the resolved authoring payload already produced by `CreateDocTypeOp`
- [x] The MCP adapter remains tool-only for now
- [x] `mcp/vector` does not create prompt assets or reimplement prompt-resolution logic already owned by `runtime-doc`
