---
id: task-00043-sync-doc-assets-with-doc-source-of-truth
type: task
code: "00043"
slug: sync-doc-assets-with-doc-source-of-truth
title: Sync doc assets with doc source of truth
description: Bring runtime/project/assets in line with the doc/ source of truth for templates, prompts, forms, ai-rules, and config files.
status: done
created: 2026-05-18
updated: 2026-05-18
tags:
  - assets
  - sync
  - governance
related:
  - spec-00007-assets-sync-process-from-doc-source-of-truth
supersedes: []
superseded_by: null
---

# Task 00043: Sync doc assets with doc source of truth

## 1. Prime Directive

> [!Prime Directive]
> `runtime/project/assets/` has drifted from `doc/`. The assets must reflect what lives in `doc/` — including file names — so the bootstrapper distributes the correct governed defaults. `doc/` is the single source of truth; assets are a read-only mirror.

## 2. Specs

- **Module:** `runtime/project/assets/`
- **Dependencies:** none
- **Reference:** [[spec-00007-assets-sync-process-from-doc-source-of-truth]]

> [!WARNING]
> The `doc/` folder is **read-only** in this process. No file inside `doc/` is created, modified, or deleted. Only `runtime/project/assets/` is changed.

### Sync rules

- `doc/` is the source of truth. Assets never override doc content.
- File names in assets must match file names in `doc/` exactly.
- If an ai-rule does not exist in `doc/ai-rule/active/`, do **not** add it to assets.
- Folders synced: `template`, `prompts`, `form`.
- Agent config and project config files are updated separately (see Phase C).

## 3. Checklist

### 3.1. Phase A — Sync doc folder mirrors (template, prompts, form)

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00043
  phase: Phase A
  language: file system
```

- [x] Copy `doc/template/` → `assets/doc/template/` (overwrite changed files, preserve file names)
- [x] Copy `doc/prompts/` → `assets/doc/prompts/` (overwrite changed files, preserve file names)
- [x] Copy `doc/form/` → `assets/doc/form/` (create folder if missing, copy all files)
- [x] Verify no files remain in assets that no longer exist in `doc/` for these three folders

### 3.2. Phase B — Sync ai-rules

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00043
  phase: Phase B
  language: file system
```

- [x] For each ai-rule present in **both** `doc/ai-rule/active/` and `assets/doc/ai-rule/`, overwrite the assets copy with the `doc/` content
- [x] Do **not** copy ai-rules from `doc/ai-rule/active/` to assets if they do not already exist in assets
- [x] Remove ai-rules from `assets/doc/ai-rule/` that no longer exist in `doc/ai-rule/active/`

### 3.3. Phase C — Sync config files

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00043
  phase: Phase C
  language: YAML, JSON, TOML
```

- [x] Sync `.vector/agents.yaml` → `assets/.vector/agents.yaml` (create if missing)
- [x] Sync `.vector/language-rules.yaml` → `assets/.vector/language-rules.yaml` (create if missing)
- [x] Sync `.vector/dashboards/project-status.yaml` → assets (update format to match doc)
- [x] Sync `.vector/document-types.yaml` → assets (**update-only**: update values for types already present in assets; do **not** promote new types to assets)
- [x] Sync `.gemini/settings.json` → assets (add missing `"trust": true`)
- [x] Sync `.codex/config.toml` → assets (add missing `default_tools_approval_mode`)
- [x] Sync `AGENTS.md` → assets (add `Project: vector` line)
- [x] Copy `.claude/settings.local.json` → `assets/.claude/settings.local.json` (create if missing)

### 3.4. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00043
  phase: Phase Z
  language: file system
```

- [ ] Re-run a diff between `doc/` and `assets/doc/` for the synced folders to confirm no remaining gaps
- [ ] Confirm `gaps.md` findings are resolved or documented as intentional exclusions
