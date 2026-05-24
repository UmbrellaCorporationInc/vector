---
id: task-00050-custom-prompt-message-field-name
type: task
code: "00050"
slug: custom-prompt-message-field-name
title: Custom prompt-message field name
description: Allow inline-action authors to rename the extra-context field via a prompt-field key instead of hard-coding the prompt-message variable name.
status: in-progress
created: 2026-05-24
updated: 2026-05-24
tags: []
related: []
supersedes: []
superseded_by: null
---

# Task 00050: Custom prompt-message field name

## 1. Prime Directive

> [!Prime Directive]
> The inline-action overlay always injects extra user context under the fixed key `prompt-message`. Authors cannot change that key name, which forces downstream prompts to reference `prompt-message` even when a different name is semantically clearer. This task introduces `prompt-field` as an optional inline-action key that, when present, replaces `prompt-message` as the field name passed to the spawned agent.

## 2. Specs

- **Module:** VS Code extension — inline-action parser and agent-spawn overlay
- **Dependencies:** none

### Behaviour

| Condition | Key sent to agent |
|---|---|
| `prompt-field` absent | `prompt-message` (current default) |
| `prompt-field: <name>` present | `<name>` |

**Example inline-action block:**

```vector-agent-inline-action
label: Create a task
prompt-field: message
profile: create-doc
prompt: prompts-00005-create-document
input:
  document-name: derive it from the prompt
  document-type: task
```

When the user clicks the spawn button, the overlay collects the extra-context text and sends it as `message` (not `prompt-message`) together with all `input` fields.

## 3. Checklist

### 3.1. Phase A — Parse and propagate `prompt-field`

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00050
  phase: Phase A
  language: TypeScript
```

- [x] Read `prompt-field` from the inline-action YAML block; default to `prompt-message` when absent
- [x] Pass the resolved field name through to the overlay component that builds the agent call payload
- [x] Use the resolved field name as the key when appending the extra-context string to the input map
- [x] Unit tests cover both the default path (`prompt-field` absent) and the explicit path (`prompt-field: message`)
- [x] Quality gates pass

### 3.2. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00050
  phase: Phase Z
  language: TypeScript
```

- [ ] Update README files on packages modified
