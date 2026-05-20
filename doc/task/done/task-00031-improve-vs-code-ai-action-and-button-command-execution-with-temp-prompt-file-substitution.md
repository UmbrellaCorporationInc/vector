---
id: task-00031-improve-vs-code-ai-action-and-button-command-execution-with-temp-prompt-file-substitution
type: task
code: "00031"
slug: improve-vs-code-ai-action-and-button-command-execution-with-temp-prompt-file-substitution
title: Improve VS Code AI Action and Button Command Execution with Temp Prompt File Substitution
description: Update the VS Code extension so AI actions and buttons replace the `<file>` placeholder in `agents.yaml` commands with the generated temp prompt file path and execute the resulting command in the terminal.
status: done
created: 2026-05-11
updated: 2026-05-11
tags:
  - vscode
  - frontend
  - agents
  - terminal
related:
  - task-00027-implement-rfc-00017-vs-code-dashboard-viewer-extension
  - prompts-00004-execute-task-phase
supersedes: []
superseded_by: null
---

# Task 00031: Improve VS Code AI Action and Button Command Execution with Temp Prompt File Substitution

## 1. Prime Directive

> [!Prime Directive]
> The VS Code extension currently writes the resolved AI prompt to a temp file and executes agent commands by piping that file into stdin with shell redirection. That behavior does not match the configured command contract in `.vector/agents.yaml`, which already embeds a `<file>` placeholder inside each agent command. This task aligns the extension with the config contract by replacing `<file>` with the generated temp file path and executing the final resolved command string in the VS Code terminal.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** existing governed preview action flow, `.vector/agents.yaml`, AI action/button rendering, VS Code terminal API
- **Primary modules:** `src/document-viewer/governedPreviewController.ts`, `src/document-viewer/document_actions/agentExecutor.ts`, `src/document-viewer/document_actions/agentsConfig.ts`, `src/test/extension.test.ts`
- **Current behavior:** `spawnAgentTerminal` sends `${command} < "${tempFilePath}"` to the terminal after writing the resolved prompt to a temp file
- **Target behavior:** after writing the temp file, replace every `<file>` token in the configured command with the temp file path and send the fully resolved command to the terminal
- **Scope constraint:** keep temp file lifecycle and cleanup behavior intact; this task changes command resolution and execution semantics, not prompt authoring or agent profile discovery

## 3. Checklist

### 3.1. Phase A - Formalize agent command placeholder resolution

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00031
  phase: Phase A
  language: typescript
```

- [x] Define the command-resolution rule for `.vector/agents.yaml` agent commands using the `<file>` placeholder
- [x] Keep temp prompt file generation unchanged as the source for the substituted file path
- [x] Add a dedicated helper or equivalent boundary that resolves a configured command string with the generated temp file path
- [x] Ensure the substituted file path is quoted safely when inserted into the command
- [x] Decide and document the failure behavior when an agent command does not contain `<file>`
- [x] Add or update unit tests covering placeholder replacement and invalid command cases
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Update AI action and button terminal execution

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00031
  phase: Phase B
  language: typescript
```

- [x] Update the governed preview action flow so resolved prompts are still written to a temp file before command execution
- [x] Replace the current stdin redirection execution path with resolved-command execution in the VS Code terminal
- [x] Ensure both `vector-agent-action` and `vector-agent-button` flows use the same command-resolution behavior
- [x] Preserve the current terminal naming behavior and cleanup on terminal close
- [x] Avoid duplicating prompt-resolution logic outside the existing governed preview controller flow
- [x] Add or update tests covering terminal command execution for AI actions and buttons
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Validate `agents.yaml` contract and user-facing errors

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00031
  phase: Phase C
  language: typescript
```

- [x] Tighten validation or runtime checks so misconfigured agent commands surface a clear error message
- [x] Ensure `.vector/agents.yaml` parsing and profile selection continue to work unchanged for valid configurations
- [x] Add coverage for missing `<file>`, malformed commands, and happy-path command substitution
- [x] Confirm the extension still reports missing `agents.yaml`, unknown profiles, and unavailable agents correctly
- [x] execute section "4. Quality Gate"

### 3.4. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00031
  phase: Phase Z
  language: typescript
```

- [x] Mark all implemented checklist items complete
- [x] Update README or extension documentation if the agent command contract is described there
- [x] Confirm `.vector/agents.yaml` examples match the final `<file>` substitution behavior
- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `pnpm run compile` passes
- [x] `pnpm run test` passes
- [x] `pnpm run lint` passes

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `pnpm run compile` passes
- [x] `pnpm run test` passes
- [x] `pnpm run lint` passes

## 5. Validation Vector

- [x] AI actions still write resolved prompts to a uniquely named temp file
- [x] Agent commands from `.vector/agents.yaml` are executed by replacing `<file>` with the temp file path instead of using stdin redirection
- [x] The resolved command path works for both `vector-agent-action` and `vector-agent-button`
- [x] Temp file cleanup on terminal close and extension deactivation remains intact
- [x] Misconfigured agent commands surface a clear user-facing error
- [x] Existing agent profile loading and availability checks keep working
- [x] All phase checkboxes completed
