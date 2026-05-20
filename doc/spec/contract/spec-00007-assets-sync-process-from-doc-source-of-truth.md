---
id: spec-00007-assets-sync-process-from-doc-source-of-truth
type: spec
code: "00007"
slug: assets-sync-process-from-doc-source-of-truth
title: Assets Sync Process from doc Source of Truth
description: Defines the contract for keeping runtime/project/assets/ in sync with doc/ as the authoritative source of truth.
category: contract
created: 2026-05-18
updated: 2026-05-18
authors: []
tags:
  - assets
  - sync
  - governance
related:
  - task-00043-sync-doc-assets-with-doc-source-of-truth
  - spec-00003-project-documentation-folder
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00007: Assets Sync Process from doc Source of Truth"
---

# SPEC 00007: Assets Sync Process from doc Source of Truth

## 1. Purpose

Defines the directional contract between `doc/` (the source of truth) and `runtime/project/assets/` (the distributed mirror). Every sync operation must follow this contract so the bootstrapper ships the correct governed defaults to new projects.

## 2. Definition

### 2.1 Source of truth

`doc/` is the single authoritative source. `runtime/project/assets/` is a read-only mirror and must never diverge from it. When a conflict exists, `doc/` always wins.

### 2.2 Synced folders

The following `doc/` sub-folders are mirrored 1-to-1 into `assets/doc/`:

| Source (`doc/`) | Destination (`assets/doc/`) | Notes |
|---|---|---|
| `template/` | `template/` | All sub-folders and files |
| `prompts/` | `prompts/` | All sub-folders and files |
| `form/` | `form/` | All sub-folders and files |
| `ai-rule/active/` | `ai-rule/` | Update-only: only rules already present in `assets/doc/ai-rule/` are synced; new rules added to `doc/ai-rule/active/` are **not** automatically promoted to assets |

> **Special case — ai-rule:** Adding a new rule to `doc/ai-rule/active/` does not migrate it to assets. Promotion to assets is a deliberate, manual step. This is the only folder in this table where new files are not synced automatically.

Folders **not** synced into assets: `task/`, `rfc/`, `adr/`, `spec/`, `design/`, `research/`, `snippet/`, `project/`.

### 2.3 Config files

These project-level config files live outside `doc/` but are also mirrored into assets:

| Source (project root) | Destination (`assets/`) |
|---|---|
| `.vector/agents.yaml` | `.vector/agents.yaml` |
| `.vector/dashboards/project-status.yaml` | `.vector/dashboards/project-status.yaml` |
| `.vector/document-types.yaml` | `.vector/document-types.yaml` | Update-only: only types already present in `assets/.vector/document-types.yaml` are synced; new types added to the source are **not** automatically promoted to assets |
| `.gemini/settings.json` | `.gemini/settings.json` |
| `.codex/config.toml` | `.codex/config.toml` |
| `.claude/settings.local.json` | `.claude/settings.local.json` |
| `AGENTS.md` | `AGENTS.md` |

> **Special case — document-types.yaml:** When syncing `.vector/document-types.yaml`, only document types that are already present in `assets/.vector/document-types.yaml` are updated. New types added to the source file are **not** automatically promoted to assets. Promoting a new type to assets is a deliberate, manual step.

### 2.4 File naming

File names in assets must match file names in `doc/` exactly. Renaming in `doc/` requires renaming in assets. No aliasing or remapping allowed.

### 2.5 ai-rule exclusion rule

An ai-rule file must exist in `doc/ai-rule/active/` to be included in assets. If a rule is absent from `doc/ai-rule/active/` (e.g. it was never written, is draft, or was removed), it must **not** appear in assets. This prevents the bootstrapper from distributing stale or incomplete rules.

## 3. Invariants

- Assets never contain a file that does not exist in the corresponding `doc/` location or config source.
- Assets never contain an ai-rule that is absent from `doc/ai-rule/active/`.
- File content in assets equals file content in `doc/` at sync time. No post-processing or transformation is applied.
- File names match exactly between source and destination.
- The sync is one-directional: `doc/` → `assets/`. Edits made directly in `assets/` are overwritten on the next sync.

## 4. Sync Procedure

Run this procedure whenever `doc/` changes and the bootstrapper must distribute updated defaults.

### Step 1 — Mirror doc sub-folders

For each synced folder (`template/`, `prompts/`, `form/`):

```
copy doc/<folder>/ → assets/doc/<folder>/
```

- Overwrite changed files.
- Delete files in assets that no longer exist in `doc/`.
- Preserve the exact directory structure and file names.

### Step 2 — Mirror ai-rules

```
for each file in doc/ai-rule/active/:
    copy → assets/doc/ai-rule/<filename>
```

- Overwrite changed files.
- Do not add files that are not in `doc/ai-rule/active/`.
- Remove files from assets that have been removed from `doc/ai-rule/active/`.

### Step 3 — Mirror config files

For each entry in the config table (section 2.3):

```
copy <source> → assets/<destination>
```

- Overwrite if content differs.
- **Exception — document-types.yaml:** Do not copy the file verbatim. Merge only: update the values of types that already exist in `assets/.vector/document-types.yaml`. Do not add types that are absent from assets.

### Step 4 — Verify

Run a diff between each source and its asset mirror. No differences should remain. Document any intentional exclusions.

## 5. Open Questions

- Should the sync procedure be automated via a CI step or a task command to prevent future drift?
