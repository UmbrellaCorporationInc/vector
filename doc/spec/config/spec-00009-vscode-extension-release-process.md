---
id: spec-00009-vscode-extension-release-process
type: spec
code: "00009"
slug: vscode-extension-release-process
title: VS Code Extension Release Process
description: Defines the exact steps to publish a new version of the Vector VS Code extension to the VS Code Marketplace, Open VSX Registry, and GitHub Releases.
category: config
created: 2026-05-19
updated: 2026-05-20
authors: []
tags:
  - release
  - ci
  - vscode
related:
  - "SPEC 00008: mcp-vector Release Process"
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00009: VS Code Extension Release Process"
---

# SPEC 00009: VS Code Extension Release Process

## 1. Purpose

Defines the steps to publish a new version of the Vector VS Code extension.
Each release publishes to three targets simultaneously: the VS Code Marketplace,
the Open VSX Registry, and GitHub Releases (as a downloadable `.vsix`). The
extension is versioned and released independently from the Rust workspace.

## 2. Definition

### 2.1 Preconditions

- The `main` branch CI is green (`vscode-extension` job passing).
- `VSCE_PAT` secret is configured with scope **Marketplace → Manage** under
  publisher `fernandojerez`.
- `OVSX_PAT` secret is configured for the Open VSX Registry.
- The desired version does not already exist in either registry.

### 2.2 Version format

The extension version follows [Semantic Versioning](https://semver.org/) and
is declared in `frontend/vscode/vector/package.json` under the `"version"`
field. It is **independent** from the Rust workspace version in `Cargo.toml`.

### 2.3 Release steps

| Step | Action | Who |
|------|--------|-----|
| 1 | Decide the next version (`MAJOR.MINOR.PATCH`). | Operator |
| 2 | Trigger `bump.yml` with `artifact=extension` and the new version. | Operator |
| 3 | `bump.yml` updates `package.json` and opens a PR labeled `release`. | Automated |
| 4 | Review and merge the bump PR into `main`. | Operator |
| 5 | `auto-release.yml` detects the merge and creates the GitHub Release `ext-v<VERSION>`. | Automated |
| 6 | `publish-extension.yml` builds the `.vsix` and publishes to all three targets. | Automated |

### 2.4 Triggering the bump

```bash
gh workflow run bump.yml --field artifact=extension --field version=1.4.2
```

Or via the GitHub UI: **Actions → Bump Version → Run workflow**.

`bump.yml` opens a PR on branch `release/extension-v<VERSION>` with the label
`release` and the commit `chore: bump extension version to <VERSION>`.

### 2.5 Workflow responsibilities

**`bump.yml`**

- Triggered by: `workflow_dispatch` with inputs `artifact=extension` and `version`.
- Updates `"version"` in `frontend/vscode/vector/package.json`.
- Opens a pull request labeled `release` on branch `release/extension-v<VERSION>`.

**`auto-release.yml`**

- Triggered by: `push` to `main`.
- Finds the merged PR by commit SHA and label `release`.
- Derives the tag from the branch name: `release/extension-v1.4.2` → `ext-v1.4.2`.
- Creates the GitHub Release targeting `main`.

**`publish-extension.yml`**

- Triggered by: `release: published` for tags starting with `ext-`.
- Checks out `main` and installs dependencies via `pnpm install --frozen-lockfile`.
- Builds the package once: `vsce package --out vector.vsix`.
- Uploads `vector.vsix` as a GitHub Release asset.
- Publishes to the VS Code Marketplace via `vsce publish --packagePath vector.vsix`.
- Publishes to Open VSX via `ovsx publish vector.vsix`.

### 2.6 Distribution targets

| Target | Tool | Secret |
|--------|------|--------|
| VS Code Marketplace | `vsce` | `VSCE_PAT` |
| Open VSX Registry | `ovsx` | `OVSX_PAT` |
| GitHub Releases | `gh release upload` | `GITHUB_TOKEN` |

The `.vsix` is built once and reused for all three targets, guaranteeing
identical artifacts across registries.

### 2.7 Manual re-run

To re-run a failed publish without creating a new release:

```bash
gh workflow run publish-extension.yml
```

Or via the GitHub UI: **Actions → Publish Extension → Run workflow**.

## 3. Invariants

- The version in `package.json` at merge time **must** match the intended
  published version. `vsce` reads it directly from `package.json`.
- `@types/vscode` must not exceed `engines.vscode`. Both are currently set
  to `^1.90.0`. `vsce` enforces this at package time.
- The `ext-v` tag prefix is reserved for extension releases. Tags matching
  `v*` trigger the Rust publish workflow instead.
- Only the `fernandojerez` publisher account can publish. Both `VSCE_PAT`
  and `OVSX_PAT` must belong to that account.
- The extension and Rust workspace are versioned independently. Do not
  conflate their version numbers.
- The `.vsix` is built once and shared across all publish steps. Never
  build separately per registry.

## 4. Examples

```bash
# Trigger the bump
gh workflow run bump.yml --field artifact=extension --field version=1.4.2

# bump.yml opens PR: "chore: bump extension version to 1.4.2"
# Merge the PR on GitHub

# auto-release.yml creates release ext-v1.4.2 automatically
# publish-extension.yml runs and publishes fernandojerez.vector@1.4.2
# to VS Code Marketplace, Open VSX, and GitHub Releases
```

## 5. Open Questions

- Should the bump PR be auto-merged after CI passes to remove the manual merge step?
