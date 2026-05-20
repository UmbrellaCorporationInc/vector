---
id: spec-00008-mcp-vector-release-process
type: spec
code: "00008"
slug: mcp-vector-release-process
title: mcp-vector Release Process
description: Defines the exact steps to publish a new version of mcp-vector to crates.io via the GitHub Actions release pipeline.
category: config
created: 2026-05-19
updated: 2026-05-19
authors: []
tags:
  - release
  - ci
  - mcp
related:
  - "SPEC 00005: mcp-vector Organization and Runtime Adapter Boundary"
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00008: mcp-vector Release Process"
---

# SPEC 00008: mcp-vector Release Process

## 1. Purpose

Defines the exact steps an operator must follow to publish a new version of
`mcp-vector` to crates.io. The process is implemented by two GitHub Actions
workflows — `release.yml` and `publish.yml` — and is triggered by creating a
GitHub Release.

## 2. Definition

### 2.1 Preconditions

Before starting a release, all of the following must be true:

- The `main` branch CI is green (both `rust` and `vscode-extension` jobs).
- All changes intended for the release have been merged to `main`.
- The `CARGO_REGISTRY_TOKEN` secret is set in the repository's GitHub Actions
  settings and has publish rights for `mcp-vector` on crates.io.

### 2.2 Version format

Versions follow [Semantic Versioning](https://semver.org/). The release tag
must match the pattern `v<MAJOR>.<MINOR>.<PATCH>` (e.g. `v0.2.0`).

The tag version controls the workspace version: `[workspace.package].version`
in the root `Cargo.toml` is the single source of truth for all crates,
including `mcp-vector`.

### 2.3 Release steps

| Step | Action | Who |
|------|--------|-----|
| 1 | Decide the next version (`MAJOR.MINOR.PATCH`). | Operator |
| 2 | Create and publish a GitHub Release with tag `v<VERSION>`. | Operator |
| 3 | `release.yml` opens a PR: `chore: bump workspace version to <VERSION>`. | Automated |
| 4 | Review and merge the bump PR into `main`. | Operator |
| 5 | `publish.yml` detects the `Cargo.toml` change, confirms the tag exists, and runs `cargo publish -p mcp-vector`. | Automated |

### 2.4 Creating the GitHub Release

Using the `gh` CLI:

```bash
gh release create v<VERSION> \
  --title "v<VERSION>" \
  --notes "<Release notes>"
```

Or via the GitHub web UI: **Releases → Draft a new release → Choose a tag →
Create new tag → Publish release**.

### 2.5 Workflow responsibilities

**`release.yml`**

- Triggered by: `release: published`
- Extracts `VERSION` from the tag by stripping the `v` prefix.
- Updates `version = "..."` under `[workspace.package]` in `Cargo.toml` using
  `sed`.
- Opens a pull request on branch `release/bump-v<VERSION>` with the label
  `release`.

**`publish.yml`**

- Triggered by: `push` to `main` when `Cargo.toml` changes.
- Guard: fetches all tags and checks that `v<VERSION>` exists. If the tag is
  absent, the job exits without publishing — this prevents accidental publishes
  from unrelated `Cargo.toml` edits.
- Publishes with: `cargo publish -p mcp-vector` and `CARGO_TARGET_DIR: target`
  to avoid the Windows-path issue in `.cargo/config.toml`.

## 3. Invariants

- The release tag **must** be created before or as part of the GitHub Release
  event. `publish.yml` will fail the tag-check guard otherwise.
- The bump PR **must** be merged before `publish.yml` can run, since the
  workflow triggers on push to `main`.
- The version in `Cargo.toml` at the time of publish **must** match the
  release tag. A mismatch will cause `cargo publish` to fail because crates.io
  validates the version in the manifest.
- Only `mcp-vector` is published. The remaining workspace crates
  (`runtime-*`) are internal and must not be published independently.

## 4. Examples

```bash
# Step 1 — create and publish the GitHub Release
gh release create v0.2.0 --title "v0.2.0" --notes "Add YAML tool support."

# Step 2 — release.yml opens the bump PR automatically
# Step 3 — review and merge the PR on GitHub

# After merge, publish.yml runs automatically and publishes mcp-vector 0.2.0
```

## 5. Open Questions

- Should the bump PR be auto-merged after CI passes to reduce manual steps?
- Should `release.yml` also update a `CHANGELOG.md` from the GitHub Release notes?
