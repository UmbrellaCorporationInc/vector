---
id: task-00041-improve-rust-code-quality-from-review-findings
type: task
code: "00041"
slug: improve-rust-code-quality-from-review-findings
title: Improve Rust Code Quality from Review Findings
description: Address the highest-value maintainability and correctness findings in `runtime/doc` by removing duplicated helpers, eliminating repeated regex compilation, tightening error contracts, and reducing avoidable cloning in validation flows.
status: done
created: 2026-05-18
updated: 2026-05-18
tags:
  - rust
  - runtime
  - maintenance
  - validation
related:
  - prompts-00004-execute-task-phase
supersedes: []
superseded_by: null
---

# Task 00041: Improve Rust Code Quality from Review Findings

## 1. Prime Directive

> [!Prime Directive]
> `runtime/doc` has accumulated duplicated helper logic, per-call regex compilation, fragile string-based error checks, and avoidable full-string clones inside validation and fix-up flows. This task removes the highest-signal sources of maintenance friction and correctness risk without turning into a broad, low-discipline cleanup across the entire repository.

## 2. Specs

- **Crates touched:** `runtime/doc`, potentially `runtime/io` only if a narrow helper extraction requires it
- **Primary modules:** `runtime/doc/src/operations/bootstrap_doc.rs`, `runtime/doc/src/operations/bootstrap_doc_type.rs`, `runtime/doc/src/operations/validate.rs`, `runtime/doc/src/operations/validate_fix.rs`, `runtime/doc/src/internal/slug.rs`
- **Dependencies:** existing Rust crates already present in the workspace; add no new dependency unless the chosen regex cache mechanism is already standard in the repository
- **Boundary:** focus on `runtime/doc` validation, fix, and bootstrap flows
- **Scope constraint:** do not attempt a repository-wide `unwrap()`/`expect()` purge in this task
- **Scope constraint:** do not redesign `IoPath` or other cross-crate abstractions here unless a localized fix is strictly necessary
- **Target outcome:** reduce duplicated logic, remove obvious repeated allocations and regex recompilation, and make validation/fix behavior less dependent on fragile string comparisons

## 3. Gaps, Flaws, and Tradeoffs

- **Gap:** the review identifies many broad issues, but only a subset can be improved safely in one task without mixing unrelated risk profiles
- **Flaw:** duplicated slug and code-format helpers increase drift risk when validation rules evolve
- **Flaw:** regex compilation inside hot validation paths wastes work and obscures performance regressions
- **Flaw:** stringly typed error handling in validation-fix behavior is brittle and can silently break if messages change
- **Tradeoff:** keeping this task scoped to `runtime/doc` leaves some findings unresolved, but it preserves reviewability and testability
- **Tradeoff:** extracting shared helpers may touch multiple modules now, but it lowers future change cost and reduces rule drift

## 4. Checklist

### 4.1. Phase A - Remove Duplicated Validation and Formatting Logic

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00041
  phase: Phase A
  language: rust
```

- [x] Consolidate duplicated `format_code()` logic into a single shared helper with a clear ownership boundary
- [x] Reuse the existing slug validation contract instead of maintaining a second near-identical validation implementation
- [x] Remove duplicated mock or discard sender implementations only where they are structurally identical and can be shared without worsening test readability
- [x] Preserve current document bootstrap behavior and file naming semantics
- [x] Add or update tests that prove the shared helpers preserve current accepted and rejected inputs

### 4.2. Phase B - Harden Validation and Fix Error Contracts

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00041
  phase: Phase B
  language: rust
```

- [x] Replace fragile `String` or `&'static str` validation error flows in the targeted modules with typed errors or an equivalent structured contract
- [x] Remove string comparison as the mechanism for deciding whether BOM fixes should run
- [x] Preserve or improve error context where filesystem operations currently discard the source failure
- [x] Add tests for the affected error paths, not only success cases
- [x] Confirm public operation behavior remains compatible for current callers

### 4.3. Phase C - Remove Obvious Performance Waste in Validation Paths

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00041
  phase: Phase C
  language: rust
```

- [x] Cache regexes used by placeholder validation and wikilink fixes instead of compiling them on every call
- [x] Eliminate the full `content.clone()` in `validate_fix.rs` if the same behavior can be preserved with a lower-allocation approach
- [x] Reduce duplicated filesystem traversal only if the extraction keeps module boundaries readable and testable
- [x] Add or update tests that cover the affected validation and fix flows after the refactor
- [x] Run crate-level quality gates for the touched runtime modules

### 4.4. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task-00041
  phase: Phase Z
  language: rust
```

- [x] Mark implemented checklist items complete
- [x] Document any deferred findings that should become follow-up tasks, especially the repository-wide `unwrap()`/`expect()` count and the `IoPath` design concern
- [x] Run final validation for the governed document and affected Rust quality gates

#### Deferred Findings

- Repository-wide `unwrap()` and `expect()` usage remains intentionally out of scope for this task. The review baseline in `rust-improvements.md` still justifies a focused follow-up task to separate production-library removals from acceptable test-only usage.
- `runtime-io` `IoPath` design remains intentionally unchanged in this task. The current wrapper contract and its broader ergonomics concern should be handled in a dedicated cross-crate follow-up task or RFC rather than folded into `runtime/doc` cleanup work.

## 5. Quality Gate

- [x] No duplicated `format_code()` or slug validation logic remains in the targeted `runtime/doc` paths
- [x] Targeted validation and fix flows do not rely on string comparison for typed behavior decisions
- [x] Regexes in the touched validation paths are not recompiled on every call
- [x] Tests cover both shared-helper behavior and the new error-handling branches
- [x] The task remains scoped to high-value `runtime/doc` improvements rather than a repository-wide cleanup sweep

## 6. Validation Vector

- [x] Bootstrap and validation flows still produce the same governed file paths for valid inputs
- [x] Invalid slug and document-type names fail through one consistent validation contract
- [x] BOM fix behavior is triggered by structured error semantics rather than matching literal text
- [x] Placeholder and wikilink validation paths avoid repeated regex construction
- [x] Validation-fix content rewriting avoids unnecessary whole-file cloning where feasible
