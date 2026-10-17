---
id: task-00079-make-action-profiles-optional-and-document-runtime-inputs
type: task
code: "00079"
slug: make-action-profiles-optional-and-document-runtime-inputs
title: Make Action Profiles Optional and Document Runtime Inputs
description: Allow configured actions to inherit a runtime agent profile and document the values supplied by the extension.
status: in-progress
created: 2026-10-17
updated: 2026-10-17
tags: [extension, actions, documentation]
related: [task-00078-implement-rfc-00044-vector-actions-setting]
supersedes: []
superseded_by: null
---

# Task 00079: Make Action Profiles Optional and Document Runtime Inputs

## 1. Prime Directive

> [!Prime Directive]
> A configured action currently requires a literal `profile` in `.vector/actions.yaml`, preventing a `vector-action` block from retaining its runtime agent choice. Make the override optional and explain the runtime input contract for both supported actions.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** `task-00078-implement-rfc-00044-vector-actions-setting`

## 3. Checklist

### 3.1. Phase A — Optional Profile Resolution

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00079
  phase: Phase A
  language: TypeScript
```

- [x] Accept an omitted `profile` in each `.vector/actions.yaml` action; continue to reject a present empty or non-string value.
- [x] Resolve the effective profile from the configured action when present, otherwise from the action's runtime profile. Keep the configured `prompt` behavior unchanged.
- [x] For `vector-action`, use the profile carried by the clicked Markdown action when YAML omits `profile`. A configured profile must continue to override that runtime profile.
- [x] Fail with an actionable error when neither `.vector/actions.yaml` nor the runtime action supplies a profile. This includes `create-doc`, whose current tree command supplies `doc-type` but no runtime profile. Do not silently choose an agent or profile.
- [x] Preserve the existing fallback when `.vector/actions.yaml` or the action entry is absent, and preserve errors for an invalid configured action.
- [x] Cover configured override, runtime fallback, unknown profile, unavailable agents, and missing runtime profile in focused tests for both action paths.

### 3.2. Phase B — Document Action YAML Inputs

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00079
  phase: Phase B
  language: Markdown, YAML
```

- [x] Document the `.vector/actions.yaml` schema and profile precedence with examples for `create-doc` and `vector-action` in the extension's user-facing documentation.
- [x] Document that configured `create-doc` receives extension-generated `doc-type`, set to the selected document type. Distinguish this key from `document-type`, used by the legacy create-form template.
- [x] Document that configured `vector-action` receives extension-generated `file-path` (the source document's path relative to `doc/`, or a synchronized package's `doc/`) and `file-name` (the source document's basename).
- [x] Explain input precedence: YAML `input` defaults, then Markdown action or form values, then extension-generated values. Document the inner order for `vector-action`: static input, action context, then form values. State that merging is shallow and that `profile` and `prompt` are not prompt inputs.
- [x] Verify the documented keys and precedence against the implementation and update examples if the runtime contract changes during Phase A.

### 3.3. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00079
  phase: Phase Z
  language: TypeScript, Markdown
```

- [ ] Run relevant extension tests and documentation validation.
- [ ] Update the extension package README if another document is chosen as the primary reference.
