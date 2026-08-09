---
id: task-00077-remove-rag-capability
type: task
code: "00077"
slug: remove-rag-capability
title: Remove RAG Capability
description: Remove the complete RAG runtime, CLI, MCP, dependency, installation, test, documentation, and local-artifact surface after version 0.5.0.
status: done
created: 2026-08-08
updated: 2026-08-09
tags:
  - rag
  - mcp
  - cli
  - dependencies
  - removal
related:
  - adr-00002-remove-rag-functionality-in-vector-mcp
supersedes: []
superseded_by: null
---

# Task 00077: Remove RAG Capability

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: task-00077-remove-rag-capability
```

## 1. Prime Directive

> [!Prime Directive]
> Eliminate the RAG capability accepted for removal in [[adr-00002-remove-rag-functionality-in-vector-mcp]] after the 0.5.0 compatibility boundary. Remove its public interfaces, executable paths, runtime implementation, exclusive dependencies, tests, installation flows, and active documentation without deleting user-owned local indexes or damaging the independent package synchronization data under `.vector-database/packages/`.

## 2. Specs

- **Modules:** `runtime/rag`, `frontend/cli/vector-rag`, `frontend/cli/vector-database`, `frontend/cli/get-vector`, `mcp/vector`, workspace manifests, release automation, and active documentation
- **Public MCP surface:** `search` and `index`
- **CLI surface:** `vector-rag rag init`, `vector-rag rag search`, `vector-rag rag update-database`, `vector-database rag ...`, and `get-vector install rag`
- **Persisted artifacts:** `.vector-database/rag/`, including the LanceDB store; removal must leave existing user data untouched and document manual cleanup
- **Direct RAG dependencies:** `runtime-rag`, `arrow-array`, `arrow-schema`, `fastembed`, `futures`, `lancedb`, and `runtime-markdown`
- **Dependency rule:** Remove a workspace dependency only after proving that no retained crate uses it; retain `runtime-markdown` if it has a non-RAG consumer
- **Out of scope:** Implementing an inverted index, changing package synchronization under `.vector-database/packages/`, and deleting local RAG data automatically
- **Compatibility boundary:** Version 0.5.0 is the final release that exposes RAG behavior

## 3. Checklist

### 3.1. Phase A — Freeze the Removal Contract and Inventory Consumers

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00077
  phase: Phase A
  language: Rust, Markdown
```

- [x] Record the exact 0.5.0 contracts for the MCP `search` and `index` tools, their structured inputs and outputs, and their error behavior so release notes can identify every removed interface.
- [x] Inventory all repository references to `runtime-rag`, `vector-rag`, `vector-database rag`, `get-vector install rag`, `.vector-database/rag/`, LanceDB, FastEmbed, embeddings, hybrid search, and RAG-specific environment or configuration values.
- [x] Check release scripts, CI workflows, packaging metadata, examples, shell snippets, and external installation instructions for assumptions that build, install, invoke, or distribute `vector-rag`.
- [x] Identify known consumers of the MCP tools and CLI commands; document migration to repository-native search where applicable.
- [x] Define release-note language stating that 0.5.0 is the last RAG-capable version and that consumers requiring RAG must remain on 0.5.0 or migrate.
- [x] Define manual cleanup guidance for `.vector-database/rag/`; do not add automatic deletion or reuse the directory for another feature.
- [x] Prove that `.vector-database/packages/` and any validation index are independent storage domains and remain operational.
- [x] Run the existing quality gates before removal to establish a clean baseline.

### 3.2. Phase B — Remove the MCP RAG Surface

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00077
  phase: Phase B
  language: Rust
```

- [x] Remove the RAG tool module and its tests from `mcp/vector/src/tools/rag.rs` and `mcp/vector/src/tools/rag_test.rs`.
- [x] Remove `RagTools` construction, storage, listing, lookup, and dispatch from the MCP tool module and `VectorServer`.
- [x] Remove the public MCP `search` and `index` schemas, handlers, subprocess delegation, logging notifications, and RAG-only error paths.
- [x] Update MCP server tests to assert that `search` and `index` are absent while all retained document, language, project, and version tools remain discoverable and callable.
- [x] Add a regression test proving that the MCP server starts and serves retained tools when `vector-rag`, RAG configuration, model files, and `.vector-database/rag/` are absent.
- [x] Remove RAG tool-group and contract documentation from `mcp/vector/README.md`.

### 3.3. Phase C — Remove RAG CLI and Installation Paths

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00077
  phase: Phase C
  language: Rust, Markdown
```

- [x] Remove the `frontend/cli/vector-rag` crate, including command parsing, `rag init`, `rag search`, `rag update-database`, JSON output contracts, tests, and README.
- [x] Remove the `rag` command group, usage text, dispatch, subprocess passthrough, installation guidance, and passthrough tests from `vector-database`.
- [x] Remove `get-vector install rag`, `run_rag`, `RAG_PACKAGE_NAME`, RAG installation messages, and their parsing and command-construction tests.
- [x] Preserve the base `get-vector update-mcp-vector` behavior for `mcp-vector` and `vector-database`.
- [x] Update packaging and release automation so no build or install step references the removed `vector-rag` package.
- [x] Add CLI regression tests proving retained `vector-database package add` and `package sync` behavior remains intact and the removed RAG command group is rejected as unknown.

### 3.4. Phase D — Remove Runtime Code and Exclusive Dependencies

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00077
  phase: Phase D
  language: Rust
```

- [x] Remove the `runtime/rag` crate and its chunking, embedding, storage, lifecycle, indexing, retrieval, context assembly, defaults, pipeline, operations, tests, and README.
- [x] Remove `runtime/rag` and `frontend/cli/vector-rag` from the workspace members and remove the `runtime-rag` workspace dependency.
- [x] Remove `arrow-array`, `arrow-schema`, `fastembed`, `futures`, and `lancedb` from workspace dependencies after confirming no retained crate consumes them.
- [x] Determine whether `runtime/markdown` exists solely for RAG. Remove the crate and `runtime-markdown` workspace dependency if exclusive; otherwise retain it and document its remaining owner.
- [x] Regenerate `Cargo.lock` and verify that transitive model-runtime, ONNX, Arrow, LanceDB, and embedding dependencies no longer remain unless a retained crate independently requires them.
- [x] Confirm no retained source module imports RAG types or depends transitively on removed operations, defaults, schemas, or storage abstractions.
- [x] Run formatting, compilation, linting, tests, and dependency checks for the complete Rust workspace.

### 3.5. Phase E — Reconcile Documentation and Release Guidance

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00077
  phase: Phase E
  language: Markdown
```

- [x] Update the root README, crate READMEs, dependency registry, release documentation, examples, and help text that describe RAG as an available capability.
- [x] Add release notes listing removed MCP tools, CLI commands, crate names, dependencies, and the 0.5.0 compatibility boundary.
- [x] Add migration guidance for direct repository search and manual cleanup guidance for existing `.vector-database/rag/` data.
- [x] Preserve accepted and implemented governed documents as historical records unless the documentation policy explicitly requires another lifecycle transition; add references to [[adr-00002-remove-rag-functionality-in-vector-mcp]] where readers could otherwise mistake an old RAG design for active functionality.
- [x] Validate that no active documentation instructs users to install, configure, initialize, index, or query the removed RAG subsystem.
- [x] Run governed-document validation and fix all correctable issues.

### 3.6. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00077
  phase: Phase Z
  language: Rust, Markdown
```

- [x] Run a repository-wide RAG reference audit and classify every remaining match as historical governance, compatibility/release guidance, or an unrelated use of the word `index`.
- [x] Verify from a clean build that neither `runtime-rag` nor `vector-rag` is produced and that no RAG-exclusive dependency remains in `Cargo.lock`.
- [x] Verify the MCP server starts and exposes only retained tools without local RAG artifacts or model downloads.
- [x] Verify `vector-database package add` and `package sync` still work with synchronized packages under `.vector-database/packages/`.
- [x] Update README files for every modified package.
- [x] Record the final list of deleted public interfaces, crates, dependencies, tests, documents, and retained historical references in the completion notes.
- [x] Run all project quality gates successfully.

## 4. Acceptance Criteria

- [x] The workspace contains no `runtime-rag` or `vector-rag` crate and builds without their source directories.
- [x] MCP tool discovery and invocation no longer expose `search` or `index`.
- [x] `vector-database` has no `rag` command group or companion-process dependency.
- [x] `get-vector` has no RAG installation command or RAG-specific installation messaging.
- [x] RAG-exclusive direct and transitive dependencies are absent from manifests and `Cargo.lock`.
- [x] Existing `.vector-database/rag/` data is not deleted by installation, update, startup, or migration code.
- [x] Package synchronization and all retained MCP and CLI capabilities pass regression tests.
- [x] User-facing documentation identifies 0.5.0 as the final RAG-capable version and provides migration and manual cleanup guidance.
- [x] Governed-document validation, formatting, compilation, linting, and the full test suite pass.

## 5. Staff Engineering Assessment

The removal scope is appropriately broad because retaining any one of the MCP facade, CLI passthrough, companion binary, runtime crate, or installation path would leave a misleading or broken compatibility surface. The highest-risk gap is consumer evidence: repository references can prove internal reachability, but they cannot prove that no external client calls the MCP tools or CLI commands. Release notes and an explicit 0.5.0 boundary are therefore mandatory, not optional cleanup.

Dependency removal should be evidence-driven. The RAG-exclusive machine-learning and vector-store dependencies are strong removal candidates, but `runtime-markdown` must not be deleted merely because RAG currently consumes it; its ownership must be checked across retained features. Likewise, `.vector-database/` is not wholly RAG-owned. Only the `rag/` subtree is in scope, while package synchronization and future validation storage must remain untouched.

The task should be considered incomplete if it only makes RAG unreachable. A clean removal requires eliminating dead packages, installation affordances, transitive dependencies, active documentation, and tests that encode obsolete contracts while preserving historical decision records and user-owned local data.
