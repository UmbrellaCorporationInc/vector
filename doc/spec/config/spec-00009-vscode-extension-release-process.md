---
id: spec-00009-vscode-extension-release-process
type: spec
code: "00009"
slug: vscode-extension-release-process
title: VS Code Extension Release Process
description: Defines the exact steps to publish a new version of the Vector VS Code extension to the Marketplace.
category: config
created: 2026-05-19
updated: 2026-05-19
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

Defines the steps to publish a new version of the Vector VS Code extension
to the Visual Studio Code Marketplace. The extension is versioned and released
independently from the Rust workspace (`mcp-vector`).

## 2. Definition

### 2.1 Preconditions

- The `main` branch CI is green (`vscode-extension` job passing).
- `VSCE_PAT` secret is set in the repository's GitHub Actions settings with
  scope **Marketplace → Manage** under publisher `fernandojerez`.
- The desired version does not already exist in the Marketplace under
  `fernandojerez.vector`.

### 2.2 Version format

The extension version follows [Semantic Versioning](https://semver.org/) and
is declared in `frontend/vscode/vector/package.json` under the `"version"`
field. It is **independent** from the Rust workspace version in `Cargo.toml`.

### 2.3 Release steps

| Step | Action | Who |
|------|--------|-----|
| 1 | Decide the next version (`MAJOR.MINOR.PATCH`). | Operator |
| 2 | Update `"version"` in `frontend/vscode/vector/package.json`. | Operator |
| 3 | Open a PR with the version bump, merge to `main`. | Operator |
| 4 | Create and publish a GitHub Release with tag `ext-v<VERSION>`. | Operator |
| 5 | `publish-extension.yml` packages and publishes to the Marketplace. | Automated |

### 2.4 Bumping the version

Edit `frontend/vscode/vector/package.json`:

```json
{
  "version": "1.4.0"
}
```

Commit and merge via PR before creating the release tag.

### 2.5 Creating the GitHub Release

Using the `gh` CLI:

```bash
gh release create ext-v<VERSION> \
  --title "Extension v<VERSION>" \
  --notes "<Release notes>" \
  --target main
```

The `ext-v` prefix distinguishes extension releases from Rust releases (`v*`),
allowing independent release cadences.

### 2.6 Workflow responsibilities

**`publish-extension.yml`**

- Triggered by: `release: published` or `workflow_dispatch`.
- Checks out `main`.
- Installs dependencies via `pnpm install --frozen-lockfile`.
- Runs `vsce publish --no-dependencies` using `VSCE_PAT`.

### 2.7 Manual trigger

To re-run a failed publish without creating a new release:

```bash
gh workflow run publish-extension.yml
```

Or via the GitHub UI: **Actions → Publish Extension → Run workflow**.

## 3. Invariants

- The version in `package.json` at the time the release tag is created must
  match the intended published version. `vsce` reads it directly from
  `package.json` — there is no override at publish time.
- The `ext-v` tag prefix must be used for extension releases. Using a bare
  `v*` tag would also trigger `release.yml` (the Rust bump workflow).
- Only the `fernandojerez` publisher account can publish. The `VSCE_PAT`
  must belong to that publisher.
- The extension and Rust workspace are versioned independently. Do not
  conflate their version numbers.

## 4. Examples

```bash
# Bump version in package.json, open and merge a PR, then:
gh release create ext-v1.4.0 \
  --title "Extension v1.4.0" \
  --notes "Add dashboard refresh command." \
  --target main

# publish-extension.yml runs automatically and publishes fernandojerez.vector@1.4.0
```

## 5. Open Questions

- Should the `publish-extension.yml` trigger be restricted to `ext-v*` tags
  only, to avoid accidental triggers from Rust releases?
