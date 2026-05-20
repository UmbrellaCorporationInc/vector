---
id: task-<code>-<slug>
type: task
code: "<code>"
slug: <slug>
title: <Title>
description: <One sentence stating the concrete problem this task solves.>
status: todo
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
tags: []
related: []
supersedes: []
superseded_by: null
---

# Task <code>: <Title>

## 1. Prime Directive

> [!Prime Directive]
> What structural friction is being eliminated? Zero fluff, strict facts.

## 2. Specs

- **Module:** `crate, package, etc`
- **Dependencies:** none *(or list them)*

## 3. Checklist

### 3.1. Phase A — <name>

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task <code>
  phase: Phase A
  language: <lang>, <lang>
```

- [ ] Implementation item
- [ ] Quality gates passes

### 3.2. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task <code>
  phase: Phase Z
  language: <lang>, <lang>
```

- [ ] Update README files on packages modified
