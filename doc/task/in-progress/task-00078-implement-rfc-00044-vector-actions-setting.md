---
id: task-00078-implement-rfc-00044-vector-actions-setting
type: task
code: "00078"
slug: implement-rfc-00044-vector-actions-setting
title: Implement RFC 00044 Vector Actions Settings
description: Add governed action overrides for the VS Code extension's document creation and document action entry points.
status: in-progress
created: 2026-10-17
updated: 2026-10-17
tags:
  - agents
  - configuration
  - vscode
related:
  - rfc-00044-vector-actions-setting
supersedes: []
superseded_by: null
---

# Task 00078: Implement RFC 00044 Vector Actions Settings

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: task-00078-implement-rfc-00044-vector-actions-setting
```

## 1. Prime Directive

> [!Prime Directive]
> Implement the accepted action override contract in [[rfc-00044-vector-actions-setting]] so a project can configure agent profile, prompt, and default inputs for document creation and document actions while preserving existing behavior for missing overrides.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Configuration:** Optional `.vector/actions.yaml`; existing profiles remain in `.vector/agents.yaml`.
- **Dependencies:** Existing agent profile resolution, governed prompt lookup, interpolation, and document action execution.
- **Contract:** `create-doc` applies to the sidebar form; `vector-action` applies to both block and inline document actions. Implement the decisions and acceptance criteria in [[rfc-00044-vector-actions-setting]].

## 3. Checklist

### 3.1. Phase A — Configuration and resolution

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00078
  phase: Phase A
  language: TypeScript
```

- [x] Load optional `.vector/actions.yaml` and validate the root `actions` map, non-empty action IDs, required `profile` and `prompt`, and optional `input` map.
- [x] Validate configured entries independently at load time and report every invalid entry with the action ID, invalid field, and configuration file; keep valid entries usable.
- [x] Resolve configured profiles through `.vector/agents.yaml` and configured governed prompts through the existing prompt mechanism.
- [x] Preserve built-in behavior when the file or action ID is absent. Fail an invoked configured ID when its entry is invalid, without falling back.
- [x] Merge configured `input` defaults, action or inline form values, and extension-generated values in that order. Keep `profile` and `prompt` outside the merged variables.
- [x] Interpolate `#{variable}` placeholders in one pass and fail with an actionable error when a placeholder remains unresolved. Avoid logging rendered prompt contents or runtime values by default.

### 3.2. Phase B — Entry point integration

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00078
  phase: Phase B
  language: TypeScript
```

- [x] Route sidebar document creation through `create-doc` when configured and expose the selected governed type as `doc-type`.
- [x] Route `vector-agent-action` and `vector-agent-inline-action` through `vector-action` when configured, using the complete parsed action YAML as runtime context.
- [x] Collect inline form values before resolving the configured action and include those values in runtime context.
- [x] Generate `file-path` relative to the `doc` directory and `file-name` as the document basename. Ensure generated values override configured defaults and action-supplied values.
- [x] Present the agents associated with the resolved profile and launch the selected agent with the rendered instruction.
- [x] Retain each entry point's current behavior when no matching override is configured.

### 3.3. Phase C — Verification and documentation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00078
  phase: Phase C
  language: TypeScript
```

- [ ] Test absent files, unknown IDs, invalid profiles and prompts, partially invalid configuration, malformed YAML, and failure of an invoked invalid entry.
- [ ] Test variable precedence, duplicate keys, single-pass interpolation, unresolved placeholders, and extension-generated path metadata, including path normalization across supported platforms.
- [ ] Test sidebar `doc-type`, block action YAML context, inline form values, agent selection, and built-in fallback for all three entry points.
- [ ] Document the schema, recognized action IDs, variable precedence, runtime variables, diagnostics, and an end-to-end example.
- [ ] Run the extension's relevant automated checks and address failures.

### 3.4. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00078
  phase: Phase Z
  language: TypeScript
```

- [ ] Update README files for modified packages.
- [ ] Confirm every acceptance criterion in [[rfc-00044-vector-actions-setting]] is met and link evidence in the implementation summary.

## 4. Staff Engineering Assessment

The highest implementation risk is bypassing existing execution behavior while adding the override path. Keep configuration parsing, per-entry validation, variable merging, interpolation, and agent launch separate so each failure can be tested without an agent terminal. Preserve the accepted precedence and invalid-entry behavior exactly; a silent fallback after an invalid configured entry would hide configuration errors. The RFC does not define how non-scalar YAML values become interpolation text or prescribe a canonical separator for `file-path`; settle those contracts explicitly before encoding them in tests or documentation.
