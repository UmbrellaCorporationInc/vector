---
id: rfc-00029-vector-packages
type: rfc
code: "00029"
slug: vector-packages
title: Vector Packages
description: Define vector packages as governed vector repositories, including manifest operations, sync planning, CLI execution, local storage, and bootstrap ignore guarantees.
status: implemented
created: 2026-06-06
updated: 2026-06-06
authors:
  - Codex
tags:
  - packages
  - bootstrap
  - cli
related: []
supersedes: []
superseded_by: null
aliases:
  - "RFC 00029: Vector Packages"
---

# RFC 00029: Vector Packages

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00029-vector-packages`
  document-type: task
  document-name: implement-rfc-00029-vector-packages
```

## 1. Problem

The workspace has no governed way to declare reusable external packages that should be fetched into a local cache for project use.

Today, package sources, versions, and local storage behavior are implicit. That creates several problems:

- Teams cannot declare package dependencies in a single project-owned manifest.
- The runtime has no dedicated package boundary for evaluating package state and producing sync actions.
- The CLI has no contract for resolving, planning, and executing package synchronization.
- The system has no formal way to consume governed documentation from one vector repository inside another vector repository.
- Downloaded package content risks leaking into source control if bootstrap ignore rules do not exclude the local package cache.
- Different workspaces may implement ad hoc conventions, which increases operational drift and makes support harder.

## 2. Proposal

Introduce vector packages as source-backed dependencies declared in `.vector/packages.yaml`.

Package behavior belongs to a new dedicated crate, `runtime/packages`. Neither `runtime/doc` nor `runtime/project` should parse or evaluate `packages.yaml` directly.

A vector package is a governed vector repository that contains, at minimum:

- A `doc/` directory with governed documentation content.
- A `.vector/` directory with vector repository configuration.

The purpose of packages is not only distribution. It is repository-to-repository knowledge reuse: one vector repository can declare and access documentation hosted in other vector repositories through a governed package mechanism.

The manifest format is:

```yaml
<package-name-1>:
  type: git
  url: <url>
  tag: <git-tag-or-branch-ref>
<package-name-2>:
  type: file
  url: <file-path-or-file-url>
```

For `git` packages, `tag` is required. The `tag` field accepts either:

- a Git tag value
- `branch:main`
- `branch:<name>`

The `branch:*` forms explicitly mean "resolve the current HEAD of that branch" and are the only supported mutable Git references in the initial contract.

The runtime crate must expose two operations:

1. `sync-packages`
2. `add-package`

### 2.1. `sync-packages`

`sync-packages` evaluates `.vector/packages.yaml` and the current state of `.vector-database/packages`.

For each declared package, the operation must validate the package entry and return one sync instruction in a list with:

- the package name
- a command type
- an agent-facing execution description

The command type must be one of:

- `clone`
- `fetch`
- `copy`

The decision rules are:

- `git` package not present in `.vector-database/packages/<package-name>` -> `clone`
- `git` package already present in `.vector-database/packages/<package-name>` -> `fetch`
- `file` package -> `copy`

The agent-facing descriptions must follow these contracts:

- `fetch`: `haz un git fetch y actualiza el package que esta en .vector-database/packages/<package-name>`
- `copy`: copy data from the file source into `.vector-database/packages/<package-name>`
- `clone`: clone the Git source into `.vector-database/packages/<package-name>`

`sync-packages` is a planning and validation operation. It returns the actions to execute; it does not invoke shell commands itself.

### 2.2. `add-package`

`add-package` receives:

- package name
- package type
- package URL
- package tag when required

`add-package` must:

- validate that the package name is not already present in `.vector/packages.yaml`
- require `tag` for `git`
- treat `tag` as optional for `file`
- append or update the manifest in governed YAML form
- reject duplicate package names before any write

### 2.3. CLI Contract

The execution CLI for package flows is `vector-database`, not `get-vector`.

It must expose:

- `vector-database package sync`
- `vector-database package add`

`vector-database package sync` must call `sync-packages`, then execute the returned actions in the terminal. Before each command it must print a pre-message describing the action, for example:

- cloning package `<name>` from url `<url>`
- fetching package `<name>` from url `<url>`
- copying package `<name>` from url `<url>`

The CLI must stream the output of each executed command.

`vector-database package add` must call `add-package` and accept the package type, URL, and tag input required by the runtime contract.

After this RFC is accepted:

- A project may define zero or more named packages in `.vector/packages.yaml`.
- Each package must resolve to a repository or file source that represents a valid vector repository package.
- A valid vector repository package must contain `doc/` and `.vector/`.
- Each package entry must define a `type` and a `url`.
- `type` must be one of `git` or `file`.
- When `type` is `git`, `url` must identify a Git repository source and `tag` must be present.
- For `git`, `tag` may be a Git tag or `branch:<name>` to mean the current HEAD of a branch.
- When `type` is `file`, `url` must identify a file-based package source and `tag` must be optional.
- `runtime/packages` owns manifest loading, package validation, sync planning, and manifest mutation operations.
- `sync-packages` must inspect package state and return one command plan per package using `clone`, `fetch`, or `copy`.
- `add-package` must update `.vector/packages.yaml` and reject duplicate package names.
- The CLI must provide package commands that resolve the manifest and synchronize packages into `.vector-database/packages`.
- The CLI must make the downloaded package content available for access from the consuming vector repository.
- Package download behavior must be deterministic for a given manifest.
- Project bootstrap logic must ensure `.vector-database/packages` is excluded from Git through the bootstrap `.gitignore` template.
- This workspace must also ensure `.vector-database/packages` is ignored so local package material is never tracked by accident.
- The implementation documents failure behavior for unsupported source types, missing repositories or files, invalid package structure, missing required tags, duplicate package names, and local cache refreshes.

This RFC intentionally defines the package declaration, planning operations, and local storage contract first. It does not define package publishing, dependency graphs between packages, cross-package indexing semantics, or the full validation matrix for every remote transport.

## 3. Alternatives Considered

- **Commit packages into the repository:** Discarded because vendoring external sources would increase repository size, create noisy diffs, and blur ownership boundaries.
- **Disallow branch-based Git references entirely:** Discarded because branch HEAD sync is an explicit operational need. The accepted contract keeps branch-based references explicit under `tag: branch:<name>` rather than allowing arbitrary ref semantics.
- **Treat packages as arbitrary archives instead of vector repositories:** Discarded because the feature goal is governed documentation reuse across vector repositories, not generic file fetching.
- **Store packages under `.vector/` instead of `.vector-database/`:** Discarded because downloaded artifacts are runtime cache material, not source configuration, and should stay separate from governed configuration files.
- **Let `runtime/doc` or `runtime/project` read `packages.yaml`:** Discarded because package parsing and sync planning need a dedicated ownership boundary to avoid spreading package rules across unrelated crates.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Package dependencies become explicit and reviewable in a single manifest. | The CLI must now own source resolution, cache layout, and per-type failure handling. |
| A dedicated `runtime/packages` crate centralizes manifest and sync rules in one boundary. | The workspace gains another runtime crate and an additional integration surface to maintain. |
| Packages can serve as a governed documentation federation mechanism across vector repositories. | The implementation must validate repository structure, not only source reachability. |
| Supporting both `git` and `file` broadens package ingestion without changing the manifest location. | The implementation must validate different semantics for each source type. |
| Explicit tags and `branch:<name>` references keep Git intent visible in the manifest. | Branch-based references are mutable, so branch consumers sacrifice some reproducibility compared with immutable tags. |
| Keeping packages in `.vector-database/packages` separates generated cache data from source configuration. | Bootstrap and workspace ignore rules must stay aligned or developers may accidentally track cache files. |
| `sync-packages` separates planning from shell execution, which keeps runtime logic testable. | The CLI must faithfully interpret the returned command plans or runtime and execution behavior will drift. |
| A standard contract reduces workspace-by-workspace drift. | The first version is intentionally narrow and may require follow-up RFCs for auth, updates, and dependency resolution. |

## 5. Acceptance Criteria

- [x] The repository defines the vector package manifest as `.vector/packages.yaml`.
- [x] The RFC defines a vector package as a vector repository containing `doc/` and `.vector/`.
- [x] The implementation introduces a dedicated `runtime/packages` crate.
- [x] The manifest schema supports named package entries with required `type` and `url` fields.
- [x] The manifest accepts `type: git` and `type: file` only.
- [x] The manifest requires `tag` for `git` packages.
- [x] The manifest accepts `tag: branch:<name>` for `git` packages to mean branch HEAD resolution.
- [x] The manifest treats `tag` as optional for `file` packages.
- [x] `sync-packages` evaluates `.vector/packages.yaml` without delegating manifest parsing to `runtime/doc` or `runtime/project`.
- [x] `sync-packages` returns one planned action per package with package name, command type, and execution description.
- [x] `sync-packages` returns `clone` for missing Git packages, `fetch` for existing Git packages, and `copy` for file packages.
- [x] `add-package` validates duplicate names and updates `.vector/packages.yaml`.
- [x] The `vector-database package sync` CLI command executes planned actions and streams command output.
- [x] The `vector-database package add` CLI command accepts package type, URL, and tag inputs and delegates manifest mutation to `add-package`.
- [x] The implementation validates that downloaded packages have the minimum required vector repository structure.
- [x] The implementation defines how a consuming vector repository accesses documentation from downloaded packages.
- [x] Re-running the command with the same manifest produces the same resolved package contents for supported source types.
- [x] The bootstrap `.gitignore` template excludes `.vector-database/packages`.
- [x] This workspace ignores `.vector-database/packages`.
- [x] The implementation documents failure behavior for unsupported source types, missing repositories or files, invalid package structure, missing required tags, duplicate package names, and local cache refreshes.

## 6. Open Questions

- Should package directories be named only by package key, or should they include the resolved tag to support side-by-side versions?
- Should the initial implementation allow private repositories, and if so, which credential sources are supported?
- Should `file` sources be limited to workspace-local paths, or also allow absolute paths and `file://` URLs?
- Should package access expose all governed documents from `doc/`, or only a filtered subset declared by package metadata?
- Should future versions validate that Git tags are immutable references before accepting them?
