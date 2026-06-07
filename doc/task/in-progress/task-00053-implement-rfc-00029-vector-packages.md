---
id: task-00053-implement-rfc-00029-vector-packages
type: task
code: "00053"
slug: implement-rfc-00029-vector-packages
title: Implement RFC 00029 - Vector Packages
description: Add the new runtime packages crate, manifest mutation and sync-planning operations, package execution CLI, repository structure validation, and ignore rules required to consume vector repository packages locally.
status: in-progress
created: 2026-06-06
updated: 2026-06-06
tags:
  - packages
  - cli
  - bootstrap
  - governance
related:
  - rfc-00029-vector-packages
supersedes: []
superseded_by: null
---

# Task 00053: Implement RFC 00029 - Vector Packages

## 1. Prime Directive

> [!Prime Directive]
> The workspace has no dedicated package boundary for declaring, validating, planning, and executing reusable vector repository dependencies. This task must introduce `runtime/packages`, make package dependencies explicit in `.vector/packages.yaml`, plan sync actions against `.vector-database/packages`, reject invalid package structures, and ensure cached package content never leaks into source control.

## 2. Specs

- **Module:** `runtime/packages`, `frontend/cli/vector-database`, bootstrap assets for project ignore rules
- **Dependencies:** none unless a new fetch or archive dependency is explicitly approved in dependency governance before merge

## 3. Checklist

### 3.1. Phase A - Crate Bootstrap and Manifest Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00053
  phase: Phase A
  language: Rust, YAML, Markdown
```

- [x] Create the new `runtime/packages` crate and wire it into the workspace.
- [x] Keep `packages.yaml` parsing and validation inside `runtime/packages`; `runtime/doc` and `runtime/project` must not own this responsibility.
- [x] Define the `.vector/packages.yaml` manifest contract for named package entries with required `type` and `url` fields.
- [x] Accept `git` and `file` as the only supported source types.
- [x] Enforce `tag` as required for `git` packages and optional for `file` packages.
- [x] Accept `tag: branch:main` and `tag: branch:<name>` as the manifest form for tracking branch HEAD in Git packages.
- [x] Add manifest parsing and validation in `runtime/packages` with explicit errors for unsupported source types, missing required fields, and malformed entries.

### 3.2. Phase B - Implement `sync-packages`

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00053
  phase: Phase B
  language: Rust
```

- [x] Implement the `sync-packages` operation in `runtime/packages`.
- [x] Make `sync-packages` evaluate the manifest and the current state of `.vector-database/packages`.
- [x] Return one list entry per package containing package name, command type, and execution description.
- [x] Return `clone` when a Git package does not yet exist in `.vector-database/packages/<package-name>`.
- [x] Return `fetch` when a Git package already exists in `.vector-database/packages/<package-name>`.
- [x] Return `copy` for `file` packages targeting `.vector-database/packages/<package-name>`.
- [x] Use the required `fetch` description contract: `haz un git fetch y actualiza el package que esta en .vector-database/packages/<package-name>`.
- [x] Make the sync plan deterministic for repeated runs against the same manifest and package state.

### 3.3. Phase C - Implement `add-package`

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00053
  phase: Phase C
  language: Rust, YAML
```

- [x] Implement the `add-package` operation in `runtime/packages`.
- [x] Accept package name, type, URL, and tag input according to the manifest contract.
- [x] Reject any package name that is already present in `.vector/packages.yaml`.
- [x] Require `tag` for `git` packages and keep it optional for `file` packages.
- [x] Update `.vector/packages.yaml` in governed YAML form after validation succeeds.
- [x] Return a clear validation error before any write when a duplicate package name is requested.

### 3.4. Phase D - CLI Execution Surface

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00053
  phase: Phase D
  language: Rust
```

- [ ] Create the package execution CLI as `frontend/cli/vector-database`.
- [ ] Add the `vector-database package sync` subcommand.
- [ ] Make `vector-database package sync` call `sync-packages`, execute each returned action in the terminal, and stream command output.
- [ ] Print a pre-message before each command, such as cloning, fetching, or copying package `<name>` from URL `<url>`.
- [ ] Add the `vector-database package add` subcommand.
- [ ] Make `vector-database package add` delegate manifest mutation to `add-package` and accept package type, URL, and tag input.

### 3.5. Phase E - Package Validation and Repository Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00053
  phase: Phase E
  language: Rust
```

- [ ] Validate that synchronized packages are vector repositories containing both `doc/` and `.vector/`.
- [ ] Reject packages that resolve but do not satisfy the minimum repository contract.
- [ ] Define how the consuming workspace accesses governed documentation from downloaded packages without weakening existing repository boundaries.
- [ ] Keep package access behavior scoped to documentation reuse; do not invent package publishing, transitive dependency resolution, or cross-package indexing semantics in this task.

### 3.6. Phase F - Bootstrap and Ignore Guarantees

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00053
  phase: Phase F
  language: Rust, Markdown
```

- [ ] Update bootstrap assets so newly created projects ignore `.vector-database/packages` in the generated `.gitignore`.
- [ ] Ensure this repository also ignores `.vector-database/packages`.
- [ ] Keep cache storage separate from governed configuration under `.vector/`.
- [ ] Add regression coverage proving repeated bootstrap or setup flows preserve the ignore contract.

### 3.7. Phase G - Tests and Documentation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00053
  phase: Phase G
  language: Rust, Markdown
```

- [ ] Add tests for manifest validation, deterministic sync planning, missing `tag` on `git`, `branch:<name>` parsing, duplicate package names, invalid repository structure, and refresh behavior.
- [ ] Add tests for both `git` and `file` package sources using stable fixtures.
- [ ] Add CLI tests proving `package sync` executes `clone`, `fetch`, and `copy` plans with the expected pre-messages.
- [ ] Document failure behavior for unsupported source types, missing repositories or files, invalid package structure, missing required tags, duplicate package names, and cache refreshes.
- [ ] Run and pass the relevant Rust quality gates for the touched crates and CLI package.

### 3.8. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00053
  phase: Phase Z
  language: Rust, Markdown
```

- [ ] Update README files for every modified crate or package.
- [ ] Mark RFC 00029 status as `implemented` only after all acceptance criteria are satisfied end to end.
