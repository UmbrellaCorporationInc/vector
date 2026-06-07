---
id: task-00056-update-get-vector-to-calculate-the-terminal-size
type: task
code: "00056"
slug: update-get-vector-to-calculate-the-terminal-size
title: Update get-vector to calculate the terminal size
description: Retrieve the terminal width dynamically using the terminal_size crate, falling back to 80, and register the dependency in project-0003-rust-dependencies.md.
status: in-progress
created: 2026-06-07
updated: 2026-06-07
tags:
  - cli
  - dependencies
related:
  - project-0003-rust-dependencies
supersedes: []
superseded_by: null
---

# Task 00056: Update get-vector to calculate the terminal size

## 1. Prime Directive

> [!Prime Directive]
> Eliminate static terminal width assumptions in get-vector formatting by dynamically querying the terminal columns using the `terminal_size` crate, falling back to a default value of 80 if the width query fails or terminal size is not available.

## 2. Specs

- **Module:** `get-vector`
- **Dependencies:** `terminal_size` (scoped to `get-vector` only)

## 3. Checklist

### 3.1. Phase A — Dependency Registration

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00056
  phase: Phase A
  language: markdown
```

- [ ] Add `terminal_size` as approved dependency in [project-0003-rust-dependencies.md](file:///C:/Users/ferna/OneDrive/Obsidian/vector/doc/project/project-0003-rust-dependencies.md).
- [ ] Specify that the scope of `terminal_size` is restricted to the `get-vector` package.

### 3.2. Phase B — Crate Integration and Implementation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00056
  phase: Phase B
  language: rust
```

- [ ] Add `terminal_size` dependency to `get-vector/Cargo.toml`.
- [ ] Implement the `terminal_size` detection logic in `get-vector` to replace the hardcoded fallback values.
- [ ] Verify functionality via manual execution or cargo tests.

### 3.3. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00056
  phase: Phase Z
  language: markdown
```

- [ ] Update README files on packages modified.
- [ ] Run `validate_fix` to ensure all vector documentation constraints are met.
