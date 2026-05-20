---
id: task-00045-implement-rfc-00025-vector-site-frontend
type: task
code: "00045"
slug: implement-rfc-00025-vector-site-frontend
title: Implement RFC 00025 Vector Site Frontend
description: Bootstrap and integrate the separate Astro website and documentation packages defined by RFC 00025 without introducing monorepo orchestration.
status: in-progress
created: 2026-05-18
updated: 2026-05-18
tags:
  - website
  - astro
  - documentation
  - frontend
related:
  - rfc-00025-vector-site-frontend
  - spec-00006-typescript-quality-gate-contract-for-vs-code-extensions
supersedes: []
superseded_by: null
---

# Task 00045: Implement RFC 00025 Vector Site Frontend

## 1. Prime Directive

> [!Prime Directive]
> Eliminate the current absence of a coherent public product entry point by delivering two separate Astro packages, `frontend/website` and `frontend/docs`, each with explicit quality gates, clear ownership boundaries, and cross-navigation between product positioning and product usage content.

## 2. Specs

- **Module:** `frontend/website`, `frontend/docs`
- **Dependencies:** `rfc-00025-vector-site-frontend`, `spec-00006-typescript-quality-gate-contract-for-vs-code-extensions`, Astro package scaffolds, package-level quality scripts

## 3. Checklist

### 3.1. Phase A - Bootstrap Astro Packages

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00045
  phase: Phase A
  language: TypeScript, Markdown
```

- [ ] Create a new Astro project at `frontend/website`
- [ ] Create a new Astro project at `frontend/docs`
- [ ] Keep both packages repository-local without adding Turbo or equivalent monorepo orchestration
- [ ] Define package boundaries and baseline scripts at package level instead of relying on scaffold defaults

### 3.2. Phase B - Apply Quality Gate Baseline

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00045
  phase: Phase B
  language: TypeScript, JSON
```

- [ ] Map the quality gate contract from `spec-00006-typescript-quality-gate-contract-for-vs-code-extensions` onto both Astro packages
- [ ] Add explicit scripts and enforcement points for linting, formatting, type checking, and test or validation equivalents where applicable
- [ ] Document any contract clauses that do not cleanly map to Astro packages as implementation gaps or follow-up work

### 3.3. Phase C - Implement Website Structure

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00045
  phase: Phase C
  language: TypeScript, Markdown
```

- [ ] Implement the landing site to explain Vector, the MCP server, and the VS Code extension
- [ ] Establish cross-links from `frontend/website` into the documentation entry points in `frontend/docs`
- [ ] Preserve a marketing-oriented information architecture in `frontend/website`

### 3.4. Phase D - Implement Documentation Structure

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00045
  phase: Phase D
  language: TypeScript, Markdown
```

- [ ] Implement the documentation site with installation, configuration, workflows, and reference entry points
- [ ] Establish cross-links from `frontend/docs` back to the product and positioning content in `frontend/website`
- [ ] Preserve a documentation-oriented information architecture in `frontend/docs`

### 3.5. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00045
  phase: Phase Z
  language: Markdown, YAML
```

- [ ] Verify the implementation satisfies every RFC 00025 acceptance criterion
- [ ] Record gaps, flaws, and tradeoffs discovered during implementation
- [ ] Update README files on packages modified
