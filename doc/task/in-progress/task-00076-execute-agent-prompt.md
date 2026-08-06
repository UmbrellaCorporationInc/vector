---
id: task-00076-execute-agent-prompt
type: task
code: "00076"
slug: execute-agent-prompt
title: Execute Agent Prompt
description: Make VS Code agent launches use a configurable root prompt with an instruction identifier placeholder and a safe built-in fallback.
status: in-progress
created: 2026-08-06
updated: 2026-08-06
tags:
  - vscode
  - agents
  - configuration
related: []
supersedes: []
superseded_by: null
---

# Task 00076: Execute Agent Prompt

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: task-00076-execute-agent-prompt
```

## 1. Prime Directive

> [!Prime Directive]
> Remove the hard-coded agent launch message from the VS Code frontend while preserving a deterministic fallback. Agent profiles must be able to define the launch prompt at the root of `agents.yaml`, and the resolved prompt must receive the current instruction identifier.

## 2. Specs

- **Modules:** VS Code frontend agent launcher and `agents.yaml` configuration parser
- **Dependencies:** none
- **Configuration contract:** Add an optional root-level `prompt` string to `agents.yaml`.
- **Supported placeholder:** Replace every `<instruction-id>` token in the selected prompt with the instruction identifier generated for the launch.
- **Fallback behavior:** When the root-level `prompt` field is absent, use the built-in default prompt.
- **Default prompt:** `Using Vector MCP, call get_instruction with id "<instruction-id>" and execute the returned instructions.`
- **Wording constraint:** The built-in default prompt must not contain the word `server`.
- **Compatibility:** Existing `agents.yaml` files without `prompt` must continue to launch agents with the built-in default.
- **Validation:** Reject a configured `prompt` whose value is not a string using the existing configuration error mechanism.

## 3. Checklist

### 3.1. Phase A — Configuration and Prompt Resolution

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00076
  phase: Phase A
  language: typescript, yaml
```

- [x] Extend the root `agents.yaml` schema and typed configuration model with the optional `prompt` field.
- [x] Centralize the built-in default prompt in one named constant.
- [x] Resolve the configured prompt when present; otherwise resolve the built-in default.
- [x] Substitute `<instruction-id>` with the launch instruction identifier immediately before starting the agent.
- [x] Ensure the default prompt says `Vector MCP` and does not contain the word `server`.
- [x] Preserve configured prompt text exactly apart from the documented placeholder substitution.

### 3.2. Phase B — Verification

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00076
  phase: Phase B
  language: typescript, yaml
```

- [ ] Add a test proving the built-in default is used when `prompt` is absent.
- [ ] Add a test proving a configured root-level `prompt` overrides the built-in default.
- [ ] Add a test proving all `<instruction-id>` occurrences are substituted with the generated identifier.
- [ ] Add a test for a configured prompt without the placeholder.
- [ ] Add a test proving an invalid non-string `prompt` produces a configuration error.
- [ ] Add or update an `agents.yaml` example documenting the field and placeholder.
- [ ] Run the affected unit, integration, lint, and type-check quality gates.

### 3.3. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00076
  phase: Phase Z
  language: typescript, yaml
```

- [ ] Update README files for modified packages when they document agent configuration or launch behavior.
- [ ] Confirm existing agent profiles without a root-level `prompt` remain compatible.
- [ ] Record the final quality-gate results in the implementation handoff.

## 4. Acceptance Criteria

- A root-level `prompt` string in `agents.yaml` controls the message used to start an agent.
- The `<instruction-id>` placeholder resolves to the identifier associated with that launch.
- Omitting `prompt` uses the specified built-in default prompt.
- The built-in default references `Vector MCP` and never uses the word `server`.
- Invalid prompt types fail through the established configuration validation path.
- Automated tests cover override, fallback, substitution, no-placeholder, and invalid-type behavior.

## 5. Staff Engineer Assessment

The requested root-level field is a small, backward-compatible extension and does not justify a broader configuration redesign. The main risks are duplicating prompt resolution across launch paths, silently coercing invalid YAML values, and applying placeholder substitution before the final instruction identifier exists. Keep resolution centralized, validate the field strictly, and test the exact emitted launch message. Supporting prompts without the placeholder is intentional: it preserves user control, while the documentation should make clear that such prompts cannot reference the generated instruction.
