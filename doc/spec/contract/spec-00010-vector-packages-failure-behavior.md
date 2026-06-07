---
id: spec-00010-vector-packages-failure-behavior
type: spec
code: "00010"
slug: vector-packages-failure-behavior
title: Vector Packages Failure Behavior
description: Specification of error handling and failure behavior for vector package synchronization, parsing, validation, and CLI commands.
category: contract
created: 2026-06-06
updated: 2026-06-06
authors:
  - Antigravity
tags:
  - packages
  - spec
  - errors
related:
  - rfc-00029-vector-packages
  - task-00053-implement-rfc-00029-vector-packages
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00010: Vector Packages Failure Behavior"
---

# SPEC 00010: Vector Packages Failure Behavior

## 1. Purpose

This specification defines the error handling and failure behavior of the vector packaging system. It details how the manifest parser, the sync planner, the CLI execution surface, and cache management operations handle invalid configurations, missing resources, and environmental errors.

## 2. Failure Scenarios and Behavior

### 2.1. Unsupported Source Types

- **Trigger:** A package entry in `.vector/packages.yaml` specifies a `type` other than the supported `git` or `file` values.
- **Handling:**
  - The parser or operation immediately rejects the manifest with an `UnsupportedType` error.
  - No sync planning or package execution can proceed.
  - Example error message: `unsupported package source type: <type>` or `has unsupported source type '<type>'`.

### 2.2. Missing Required Tags (Git)

- **Trigger:** A `git` package entry in `.vector/packages.yaml` is missing the `tag` field, or the `tag` field is empty/malformed.
- **Handling:**
  - If the `tag` field is completely missing, the parser fails with a `MissingTagForGit` error.
  - If the `tag` uses a branch tracker syntax (`branch:<name>`) but `<name>` is empty or only contains whitespace (e.g. `branch:`, `branch:  `), the parser fails with an `InvalidBranchFormat` error.
  - No sync operations or modifications can be performed when the manifest contains such an entry.

### 2.3. Duplicate Package Names

- **Trigger:** An attempt is made (via the CLI or `add-package` operation) to add a package whose name already exists in `.vector/packages.yaml`.
- **Handling:**
  - The `add-package` operation performs validation before writing to the filesystem.
  - If a duplicate name is detected, the operation immediately aborts and returns an error: `package '<name>' is already present in manifest`.
  - The manifest file on disk remains unchanged.

### 2.4. Missing Repositories or Files

- **Trigger:** During a sync operation, a Git package URL is inaccessible or invalid, or a file package source path does not exist on the filesystem.
- **Handling:**
  - The subprocess command (e.g. `git clone` or `xcopy`/`cp`) fails and returns a non-zero exit status.
  - The CLI aborts synchronization for that package.
  - To prevent a partially cloned or corrupted cache state, the CLI cleans up by deleting the target directory under `.vector-database/packages/<package-name>` if the sync was a new clone.

### 2.5. Invalid Package Structure (Contract Violation)

- **Trigger:** A synchronized package is successfully fetched or copied into `.vector-database/packages/<package-name>`, but it does not represent a valid vector repository.
- **Handling:**
  - Post-sync validation checks for the existence of two directories under the synchronized package:
    - `doc/` (containing documentation)
    - `.vector/` (containing repository configuration)
  - If either directory is missing or is not a directory, validation fails.
  - The CLI logs a contract violation error: `package '<name>' does not satisfy the minimum repository contract: missing 'doc/' or '.vector/' directory`.
  - The CLI deletes the entire synchronized directory `.vector-database/packages/<package-name>` to avoid exposing an invalid cache package to the workspace.
  - The sync process for that package is marked as failed.

### 2.6. Cache Refreshes and Network/IO Failures

- **Trigger:** A Git package already exists in `.vector-database/packages/<package-name>` and the CLI runs a sync command, but the network is down or the branch has been deleted.
- **Handling:**
  - The sync planner plans a `fetch` command.
  - The CLI executes `git fetch` followed by `git checkout` and potentially `git reset --hard`.
  - If any of these Git commands fail, the CLI aborts the sync for that package and streams the Git error output.
  - Unlike a clone failure, the existing cache directory `.vector-database/packages/<package-name>` is *not* deleted upon fetch failure. This preserves the last successfully fetched state for offline use, though the sync operation reports a failure.

## 3. Invariants

1. **Transactional Manifest Modifications:** No invalid package entries can be written to `.vector/packages.yaml`. All validations must pass before any file writes occur.
2. **Deterministic Planning:** Repeated planning runs on the same manifest and cache state must produce identical planned actions in alphabetical order of package names.
3. **No Partial State for New Clones:** A new package clone or copy that fails execution or fails post-sync repository contract validation must be completely deleted from `.vector-database/packages/`.
4. **Cache Isolation:** The packages cache under `.vector-database/packages/` must remain separated from the governed `.vector/` configuration, and must be ignored by Git to avoid accidental inclusion in source control.

## 4. Examples

### 4.1. Invalid Type Error
```
error: unsupported package source type: hg
```

### 4.2. Missing Tag Error
```
error: tag is required for git packages
```

### 4.3. Duplicate Name Error
```
error: package 'pkg1' is already present in manifest
```

### 4.4. Contract Violation Error
```
error: package 'pkg_git' does not satisfy the minimum repository contract: missing 'doc/' or '.vector/' directory
```

## 5. Open Questions

- Should we support automated retries for network-related failures during `git fetch` or `git clone`?
- Should the CLI support a `--force` flag to skip the minimum repository contract validation?
