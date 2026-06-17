---
id: task-00073-split-rag-cli-installation
type: task
code: "00073"
slug: split-rag-cli-installation
title: Split RAG CLI Installation from Base Vector Tools
description: Separate the heavy RAG runtime into a dedicated CLI so base Vector installation and MCP updates do not always compile LanceDB, DataFusion, and embedding dependencies.
status: in-progress
created: 2026-06-17
updated: 2026-06-17
tags:
  - cli
  - rag
  - install
  - performance
related: []
supersedes: []
superseded_by: null
---

# Task 00073: Split RAG CLI Installation from Base Vector Tools

## 1. Prime Directive

> [!Prime Directive]
> Base Vector installs and MCP updates currently pay the full compile and binary-size cost of `runtime-rag`, including LanceDB, Lance, DataFusion, FastEmbed, ONNX Runtime, and tokenizer dependencies. Split RAG execution into a dedicated `vector-rag` CLI so users only pay that cost when they explicitly install RAG support.

## 2. Specs

- **Module:** `frontend/cli/vector-rag`, `frontend/cli/vector-database`, `frontend/cli/get-vector`, `runtime/rag`
- **Dependencies:** `runtime-rag`, `lancedb`, `fastembed`
- **Install contract:** `get-vector update-mcp-vector` installs or updates `mcp-vector` and `vector-database` only.
- **RAG install contract:** `get-vector install rag` installs `vector-rag`.
- **Compatibility contract:** `vector-database rag ...` either delegates to `vector-rag` or returns an actionable error telling the user to run `get-vector install rag`.
- **Initial path contract:** tool discovery may depend on the user's `PATH` for this version.

## 3. Checklist

### 3.1. Phase A — Extract RAG CLI

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00073
  phase: Phase A
  language: rust, toml
```

- [ ] Add a new `frontend/cli/vector-rag` workspace crate with a `vector-rag` binary.
- [ ] Move `rag init`, `rag search`, and `rag update-database` command ownership into `vector-rag`.
- [ ] Keep `runtime-rag` as the only crate that directly owns LanceDB and embedding runtime integration.
- [ ] Add focused CLI tests for `vector-rag` command parsing and command dispatch.

### 3.2. Phase B — Slim Base CLI

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00073
  phase: Phase B
  language: rust, toml
```

- [ ] Remove the direct `runtime-rag` dependency from `frontend/cli/vector-database`.
- [ ] Replace `vector-database rag ...` implementations with delegation to `vector-rag` through process execution.
- [ ] Return a clear install guidance error when `vector-rag` is not available on `PATH`.
- [ ] Verify `cargo tree -p vector-database` no longer includes `lancedb`, `fastembed`, `datafusion`, or `ort`.

### 3.3. Phase C — Update Installer Flow

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00073
  phase: Phase C
  language: rust
```

- [ ] Update `get-vector update-mcp-vector` so it installs or updates only `mcp-vector` and `vector-database`.
- [ ] Add `get-vector install rag` to install `vector-rag`.
- [ ] Ensure installer output explains when RAG is not installed and how to add it.
- [ ] Add tests for the new installer command routing.

### 3.4. Phase D — Quality Gates

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00073
  phase: Phase D
  language: rust, toml
```

- [ ] Run formatting checks.
- [ ] Run lint checks.
- [ ] Run targeted CLI tests for `get-vector`, `vector-database`, and `vector-rag`.
- [ ] Compare install dependency trees before and after the split.
- [ ] Document the RAG install command in any README or help output touched by the change.
