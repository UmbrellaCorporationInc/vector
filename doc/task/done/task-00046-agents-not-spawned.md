---
id: task-00046-agents-not-spawned
type: task
code: "00046"
slug: agents-not-spawned
title: Agents not spawned
description: Fix bug where agent-action blocks with codex or opencode profiles do not open a terminal and execute the command.
status: done
created: 2026-05-21
updated: 2026-05-21
tags:
  - bug
  - agent-action
  - profiles
related: []
supersedes: []
superseded_by: null
---

# Task 00046: Agents not spawned

## 1. Prime Directive

> [!Prime Directive]
> When a `vector-agent-action` block is executed and the selected profile is `codex` or `opencode`, no terminal is opened and the command is never executed. Profiles using `code` (Claude Code) work correctly. The dispatch path for non-Claude Code profiles is broken.

## 2. Specs

- **Module:** agent-action spawner / profile dispatcher
- **Dependencies:** none

## 3. Checklist

### 3.1. Phase A — Investigate Profile Dispatch Failure

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00046
  phase: Phase A
  language: Rust, TypeScript
```

**Goal:** Produce a written analysis of the failure. Do not fix anything in this phase.

- [x] Reproduce the bug: select `codex` or `opencode` profile on any agent-action block and confirm no terminal spawns
- [x] Locate the profile dispatch entry point (likely TypeScript extension code)
- [x] Identify the exact branch or condition that gates terminal creation — confirm it only fires for the `code` profile
- [x] Determine whether `codex` and `opencode` reach the dispatch at all, or are dropped before it
- [x] Document findings: file paths, line numbers, and the root-cause condition for each broken profile
- [x] Record findings as detail under section 3.1 (edit this task) and ensure Phase B and Phase C below are scoped correctly

#### Phase A Findings

**Dispatch entry point:**

`frontend/vscode/vector/src/document-viewer/governedDocumentEditorProvider.ts`
- `onDidReceiveMessage` handler (line 103) routes `"vector.runAgent"` messages to `_handleRunAgent` (line 243).
- `_handleRunAgent` resolves the profile at line 256, filters to available agents at line 264, and builds the quick pick at lines 280–301.

**What the user sees:**

The `code` profile lists `[antigravity, claude, codex, opencode]`. The quick pick includes **all** agents — both available and unavailable. `codex` and `opencode` appear in the list (with `(not installed)` in their description when the availability check fails). The user can see and select them.

**Root-cause condition — silent return:**

`frontend/vscode/vector/src/document-viewer/governedDocumentEditorProvider.ts` lines 294–299:

```typescript
const picked = await vscode.window.showQuickPick(items, {
    placeHolder: `Select agent for "${msg.label}"`,
});
if (!picked || !picked.agent.available) {
    return;   // silent return — no error, no terminal
}
```

When the user selects an agent whose `available` flag is `false`, the guard fires a **silent return**. No terminal is opened, no error message is shown. The user receives zero feedback.

**Why `codex` and `opencode` are marked `available: false`:**

`agentsConfig.ts` → `isCommandInPath` (line 131–138) runs `execSync('where <exe>')` (Windows) / `execSync('which <exe>')` (Unix) from the VS Code **extension host process**. The extension host inherits a PATH snapshot taken at VS Code launch time, which typically excludes paths added by user-scoped package managers (npm global, mise, nvm, winget) that are only present in interactive shell sessions. `codex` and `opencode` are commonly installed this way. Even when the user can run them from an integrated terminal, `execSync('where codex')` in the extension host throws → `available: false`.

**Why `code` (Claude Code / `claude`) works:**

`claude` is typically installed system-wide via a platform installer or Homebrew, making it visible in the extension host's restricted PATH. It passes the `isCommandInPath` check → `available: true` → selected automatically or chosen from the pick without issue.

**Two contributing defects:**

1. **Silent rejection (primary UX bug):** Selecting an unavailable agent from the quick pick causes a silent return. The user gets no feedback explaining why nothing happened.
2. **PATH resolution mismatch (root cause of wrong availability):** `isCommandInPath` uses `execSync` from the extension host, not the terminal's shell PATH. Tools installed via user-scoped package managers are incorrectly classified as unavailable.

**Fix scope for Phase B:**

1. Replace the silent `return` with an informative `showErrorMessage` when the user picks an unavailable agent, naming the agent and telling them it was not found in PATH.
2. Fix `isCommandInPath` to resolve availability using the VS Code terminal environment (spawn a shell subprocess with the user's full PATH) so tools installed via user-scoped package managers are detected correctly.

### 3.2. Phase B — Fix Profile Dispatch

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00046
  phase: Phase B
  language: Rust, TypeScript
```

**Scope (confirmed by Phase A):**

Two changes are required:

1. **`governedDocumentEditorProvider.ts` — replace silent return** with a user-facing error when the selected agent is unavailable, so the user understands why nothing happened.

2. **`agentsConfig.ts` — fix `isCommandInPath`** to spawn a login shell subprocess (e.g. `sh -lc 'which <exe>'` on Unix, `pwsh -Command 'where <exe>'` on Windows) instead of calling `execSync` directly, so the check uses the user's full PATH rather than the extension host's restricted snapshot.

- [x] In `governedDocumentEditorProvider.ts` → `_handleRunAgent`: replace the silent `return` on `!picked.agent.available` with `showErrorMessage` naming the agent and explaining it was not found in PATH
- [x] In `agentsConfig.ts` → `isCommandInPath`: use a login-shell subprocess so user-scoped package manager installs are detected correctly
- [x] Add unit tests for the updated availability error path
- [x] Add integration test: selecting an unavailable agent from the quick pick shows an error message
- [x] Verify `codex` and `opencode` spawn correctly when they are available in PATH
- [x] Confirm `code` (Claude Code) profile still works as before (no regression)
- [x] Quality gates pass

### 3.3. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00046
  phase: Phase Z
  language: Rust, TypeScript
```

- [x] Update README files on packages modified
