---
id: task-00042-implement-rfc-00023-markdown-editor-improvements-for-the-vector-extension
type: task
code: "00042"
slug: implement-rfc-00023-markdown-editor-improvements-for-the-vector-extension
title: Implement RFC 00023 Markdown Editor Improvements for the Vector Extension
description: Implement the markdown editor and container view changes defined by RFC 00023, including the governed validate-fix action flow.
status: done
created: 2026-05-18
updated: 2026-05-18
tags:
  - vscode
  - markdown
  - viewer
  - prompts
related:
  - rfc-00023-markdown-editor-improvements-for-the-vector-extension
supersedes: []
superseded_by: null
---

# Task 00042: Implement RFC 00023 Markdown Editor Improvements for the Vector Extension

## 1. Prime Directive

> [!Prime Directive]
> Remove friction in governed document editing by adding contextual inline actions, a prompt-enriched
> execution path, visible action affordances, and a container-level validate-fix entry point driven by
> repository governance config.

## 2. Specs

- **Module:** `extensions/vscode`, governed markdown viewer components, action wiring, and repository governance assets
- **Dependencies:** `.vector/document-types.yaml`, `prompts-00006-update-document`, new `prompts-00006` in `doc/prompts/actions/`, existing agent action rendering and execution pipeline

## 3. Checklist

### 3.1. Phase A - Governed Contracts and Prompt Assets

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00042
  phase: Phase A
  language: markdown, yaml
```

- [x] Add `doc-type.prompt-validate-fix` to `.vector/document-types.yaml` under the global `doc-type` block
- [x] Create `prompts-00006` in the `actions` category as the governed prompt for the container-level validate-fix flow
- [x] Keep `prompts-00006-update-document` as the header update action contract referenced by RFC 00023
- [x] Document any required input contract for `document-stem`, `prompt-message`, and validate-fix execution
- [x] Validate that prompt identifiers and categories resolve through the governed document system
- [x] Quality gates pass for governed document validation

Phase A note: by explicit user direction, `prompts-00006` is reserved for the container-level `validate-fix` flow. The RFC reference to `prompts-00006-update-document` therefore remains unresolved and must be reconciled before Phase B binds a header action to that identifier.

### 3.2. Phase B - Inline Markdown Header Actions

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00042
  phase: Phase B
  language: typescript, markdown
```

- [x] Add support for `vector-agent-inline-action` in the markdown viewer pipeline
- [x] Render an inline action for every markdown header
- [x] Bind the header action to `prompts-00006-update-document`
- [x] Pass `document-stem` and `profile=create-doc` through the action contract
- [x] Use a pencil-like affordance for the header action, with a bounded fallback if UTF rendering is unstable
- [x] Verify header action rendering across multiple heading levels and large documents
- [x] Quality gates pass for viewer rendering and command wiring

### 3.3. Phase C - Overlay Prompt Enrichment Flow

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00042
  phase: Phase C
  language: typescript
```

- [x] Open an overlay before executing `vector-agent-inline-action`
- [x] Provide a chat-style input for extra prompt content
- [x] Merge the submitted content into the action payload as `prompt-message`
- [x] Add an action information control inside the overlay that also triggers execution
- [x] Define bounded behavior for empty, trimmed, and cancelled input states
- [x] Verify keyboard navigation, focus handling, and close behavior
- [x] Quality gates pass for overlay interaction behavior

### 3.4. Phase D - Container View Actions and Visual Affordance

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00042
  phase: Phase D
  language: typescript, css
```

- [x] Add a container-level `validate-fix` action alongside search, reload, collapse, and add-doc-type
- [x] Resolve the validate-fix prompt from `.vector/document-types.yaml` using `doc-type.prompt-validate-fix`
- [x] Execute the validate-fix action through the agent with `profile=create-doc`
- [x] Add visible default, hover, and focus styles for existing `vector-agent-action` elements
- [x] Ensure missing or unresolved `doc-type.prompt-validate-fix` fails in a bounded and diagnosable way
- [x] Quality gates pass for container action visibility and execution behavior

### 3.5. Phase Z - Wrap Up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00042
  phase: Phase E
  language: markdown, typescript, yaml
```

- [x] Update related governed documentation if the final implementation changes the accepted contract
- [x] Add or update automated tests for prompt resolution, header action rendering, overlay behavior, and container validate-fix execution
- [x] Confirm no regressions in existing viewer actions and tree/container controls
- [x] Quality gates pass for all touched modules
