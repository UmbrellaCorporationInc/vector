---
id: task-00011-bootstrap-mcp-vector-crate-and-approve-rmcp-dependency
type: task
code: "00011"
slug: bootstrap-mcp-vector-crate-and-approve-rmcp-dependency
title: Bootstrap mcp vector crate and approve rmcp dependency
description: Bootstrap the mcp/vector crate, approve rmcp for that crate only, and keep MCP tooling as a thin facade over runtime plugin operations.
status: done
created: 2026-05-04
updated: 2026-05-06
tags:
  - mcp
  - vector
  - dependency
  - architecture
related:
  - rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
  - rfc-00007-runtime-core-plugin-primitives
  - rfc-00008-runtime-channel-plugin-dispatcher-builder
  - spec-00005-mcp-vector-organization-and-runtime-adapter-boundary
  - project-0003-rust-dependencies
supersedes: []
superseded_by: null
---

# Task 00011: Bootstrap mcp vector crate and approve rmcp dependency

## 1. Prime Directive

Create the initial `mcp/vector` package and MCP server bootstrap skeleton without inventing tools, plugins, dispatch flows, or workflows that have not yet been specified.

## 2. Specs

- **Module:** `mcp/vector`
- **Dependencies:** Rust `std`, `rmcp`, selected async runtime backend, approved `runtime/*` crates

## 3. Checklist

### 3.1. Phase A - Dependency approval and governance alignment

- [x] Update Rust dependency governance to approve `rmcp` for `mcp/vector` only
- [x] Align dependency notes with [[rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary]]
- [x] Add tests or checks covering dependency-scope expectations when applicable
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Crate bootstrap and workspace integration

- [x] Create the `mcp/vector` crate and add it to the workspace
- [x] Define the initial public module layout only for crate bootstrap, server bootstrap, and protocol adapters required by the accepted boundary
- [x] Wire the crate manifest with the approved `rmcp` dependency and only the required runtime crates
- [x] Add tests covering public API visibility or crate bootstrap expectations — N/A: bare bootstrap has no testable surface yet
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Thin MCP server boundary

- [x] Define the thin MCP facade boundary through [[spec-00005-mcp-vector-organization-and-runtime-adapter-boundary]] so protocol concerns stay inside `mcp/vector`
- [x] Keep reusable execution, validation, and business logic outside `mcp/vector` through the ownership and invariant rules defined in [[spec-00005-mcp-vector-organization-and-runtime-adapter-boundary]]
- [x] Ensure the crate surface does not own repository-specific workflows, inferred tools, or inferred plugin behavior beyond MCP adaptation through [[spec-00005-mcp-vector-organization-and-runtime-adapter-boundary]]
- [x] Add tests covering boundary expectations that separate MCP adaptation from runtime logic — N/A at this phase: the boundary is defined by the spec and not yet materialized as executable MCP adapters
- [x] execute section "4. Quality Gate" — N/A for this documentation-only boundary definition phase

### 3.4. Phase D - Runtime project setup composition

- [x] Add a `ProjectSetupOp` runtime operation in `runtime-project` as the global project setup entrypoint
- [x] Add a `ProjectExtensionSetupOp` runtime operation in `runtime-doc` as the documentation-owned extension setup entrypoint for an already-created project
- [x] Keep `CreateProjectOp` focused on project bootstrap concerns and move cross-crate setup orchestration into `ProjectSetupOp`
- [x] Approve and document the `runtime-project` to `runtime-doc` dependency only as required for project setup composition
- [x] Make `ProjectSetupOp` execute `CreateProjectOp` first and then invoke `ProjectExtensionSetupOp`
- [x] Make `ProjectExtensionSetupOp` execute `CreateDocumentRuleOp` so a newly created project also leaves the documentation rule in the expected generated state
- [x] Keep the composed workflow transport-agnostic and reusable outside MCP
- [x] Add tests covering the composed runtime workflow chain `ProjectSetupOp -> ProjectExtensionSetupOp -> CreateDocumentRuleOp` without involving `rmcp` or MCP-facing adapters
- [x] execute section "4. Quality Gate"

### 3.5. Phase E - Runtime integration boundary preparation

- [x] Add the minimum internal bridge in `mcp/vector` for one real example path: `ProjectTools::CreateProject` executing `runtime-project::ProjectSetupOp` through the accepted dispatcher path without exposing MCP protocol types to runtime
- [x] Depend only on the accepted runtime execution boundary needed for that bridge, keeping dispatcher ownership and transport policy below the MCP facade
- [x] Define the runtime-facing adapter shape in `mcp/vector` for this path: selecting `ProjectSetupOp`, handing off runtime input, consuming the output receiver, and translating runtime results into MCP-facing tool results
- [x] Add the `ProjectTools` MCP adapter group in `mcp/vector` and keep `CreateProject` as the only concrete Phase E tool example
- [x] Avoid selecting, inventing, or wiring additional repository-specific tools, prompts, workflows, or plugin inventories beyond the accepted `CreateProject` example path
- [x] Keep the dispatcher path transport-agnostic below `mcp/vector` and avoid re-implementing dispatcher behavior inside the MCP crate
- [x] Add tests covering the `ProjectTools::CreateProject -> ProjectSetupOp` bridge shape and boundary expectations without assuming any broader production plugin inventory
- [x] execute section "4. Quality Gate"

### 3.6. Phase F - Server bootstrap skeleton

- [x] Add the initial MCP server bootstrap path using `rmcp`
- [x] Keep the result at server bootstrap skeleton level rather than a semantically complete MCP surface
- [x] Do not add concrete tools, prompts, resources, or plugin dispatch flows unless separately specified
- [x] Keep future tool definitions owned by `mcp/vector` rather than distributing MCP adapters into runtime crates
- [x] Add tests covering server bootstrap or handler bootstrap shape only
- [x] execute section "4. Quality Gate"

### 3.7. Phase G - Documentation and public integration

- [x] Add README documentation for `mcp/vector` ownership, dependency boundary, and runtime integration path
- [x] Verify the public API introduces no reusable domain logic, no runtime-to-MCP dependency inversion, and no protocol leakage into runtime contracts
- [x] Align package docs with [[rfc-00001-thin-mcp-facade-over-runtime-libraries]] and [[rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary]]
- [x] execute section "4. Quality Gate"

### 3.8. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `rmcp` is approved only for `mcp/vector`
- [x] `runtime/*` crates remain free of MCP SDK dependencies
- [x] `mcp/vector` stays a thin MCP facade over runtime capabilities
- [x] The task introduces no guessed tools, guessed plugins, or guessed workflows
- [x] Extensibility remains centered on segregated runtime or plugin crates through `PluginOperation`
- [x] The bootstrap prepares a future dispatcher bridge without prematurely choosing concrete plugin operations
- [x] The bootstrap stops at an MCP server skeleton and does not claim a complete tool surface
- [x] The public architecture introduces no reusable domain logic, retry policy, scheduling policy, or repository-specific workflow ownership into `mcp/vector`
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
