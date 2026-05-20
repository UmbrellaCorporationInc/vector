---
id: spec-00008-mcp-vector-release-process
type: spec
code: "00008"
slug: mcp-vector-release-process
title: mcp-vector Release Process
description: Defines the exact steps to release a new version of mcp-vector via the unified bump workflow and cargo install --git distribution.
category: config
created: 2026-05-19
updated: 2026-05-20
authors: []
tags:
  - release
  - ci
  - mcp
related:
  - "SPEC 00005: mcp-vector Organization and Runtime Adapter Boundary"
  - "SPEC 00009: VS Code Extension Release Process"
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00008: mcp-vector Release Process"
---

# SPEC 00008: mcp-vector Release Process

## 1. Purpose

Defines the exact steps to release a new version of `mcp-vector`. The process
is driven by `bump.yml` (version bump PR) and `auto-release.yml` (automatic
GitHub Release on merge). Distribution is via `cargo install --git` — the
crate is not published to crates.io because it depends on internal
`runtime-*` path crates.

## 2. Definition

### 2.1 Preconditions

- The `main` branch CI is green (both `rust` and `vscode-extension` jobs).
- All changes intended for the release have been merged to `main`.

### 2.2 Version format

Versions follow [Semantic Versioning](https://semver.org/). The release tag
matches the pattern `v<MAJOR>.<MINOR>.<PATCH>` (e.g. `v0.2.0`).

`[workspace.package].version` in the root `Cargo.toml` is the single source
of truth for all crates, including `mcp-vector`.

### 2.3 Release steps

| Step | Action | Who |
|------|--------|-----|
| 1 | Decide the next version (`MAJOR.MINOR.PATCH`). | Operator |
| 2 | Trigger `bump.yml` with `artifact=mcp` and the new version. | Operator |
| 3 | `bump.yml` updates `Cargo.toml` and opens a PR labeled `release`. | Automated |
| 4 | Review and merge the bump PR into `main`. | Operator |
| 5 | `auto-release.yml` detects the merge and creates the GitHub Release `v<VERSION>`. | Automated |
| 6 | `publish.yml` verifies `cargo install --git` succeeds at the new tag. | Automated |

### 2.4 Triggering the bump

```bash
gh workflow run bump.yml --field artifact=mcp --field version=0.2.0
```

Or via the GitHub UI: **Actions → Bump Version → Run workflow**.

`bump.yml` opens a PR on branch `release/mcp-v<VERSION>` with the label
`release` and the commit `chore: bump mcp version to <VERSION>`.

### 2.5 Workflow responsibilities

**`bump.yml`**

- Triggered by: `workflow_dispatch` with inputs `artifact=mcp` and `version`.
- Updates `version = "..."` under `[workspace.package]` in `Cargo.toml`.
- Opens a pull request labeled `release` on branch `release/mcp-v<VERSION>`.

**`auto-release.yml`**

- Triggered by: `push` to `main`.
- Finds the merged PR by commit SHA and label `release`.
- Derives the tag from the branch name: `release/mcp-v0.2.0` → `v0.2.0`.
- Creates the GitHub Release targeting `main`.

**`publish.yml`**

- Triggered by: `release: published` for tags matching `v*`.
- Verifies that `cargo install --git` succeeds at the release tag as a
  smoke test.

### 2.6 Distribution

`mcp-vector` is distributed via `cargo install --git`:

```bash
# Latest release
cargo install --git https://github.com/UmbrellaCorporationInc/vector mcp-vector

# Specific version
cargo install --git https://github.com/UmbrellaCorporationInc/vector --tag v0.2.0 mcp-vector
```

The crate is not published to crates.io because the `runtime-*` workspace
crates are internal path dependencies that cannot be resolved by crates.io.

## 3. Invariants

- The bump PR branch **must** follow the pattern `release/mcp-v<VERSION>` and
  carry the `release` label. `auto-release.yml` relies on both to create the
  correct tag.
- The version in `Cargo.toml` at merge time **must** match the intended tag.
  `auto-release.yml` derives the tag from the branch name, not from
  `Cargo.toml` — a mismatch would create a misaligned release.
- Only `mcp-vector` is distributed. The `runtime-*` crates are internal and
  must never be published or distributed independently.
- The `v*` tag prefix is reserved for Rust releases. Using it for other
  artifacts would trigger `publish.yml` incorrectly.

## 4. Examples

```bash
# Trigger the bump
gh workflow run bump.yml --field artifact=mcp --field version=0.2.0

# bump.yml opens PR: "chore: bump mcp version to 0.2.0"
# Merge the PR on GitHub

# auto-release.yml creates release v0.2.0 automatically
# publish.yml verifies: cargo install --git ... --tag v0.2.0 mcp-vector
```

## 5. Open Questions

- Should the bump PR be auto-merged after CI passes to remove the manual merge step?
