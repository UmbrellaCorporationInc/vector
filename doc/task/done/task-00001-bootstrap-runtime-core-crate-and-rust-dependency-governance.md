---
id: task-00001-bootstrap-runtime-core-crate-and-rust-dependency-governance
type: task
code: "00001"
slug: bootstrap-runtime-core-crate-and-rust-dependency-governance
title: Bootstrap runtime-core crate and Rust dependency governance
description: Create the runtime-core crate bootstrap, add Rust dependency governance for thiserror, and define the initial RuntimeError and RuntimeResult boundary.
status: done
created: 2026-05-02
updated: 2026-05-03
tags:
  - runtime
  - crate
  - governance
related:
  - spec-00002-runtime-core-crate
  - spec-00003-project-documentation-folder
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
supersedes: []
superseded_by: null
---

# Task 00001: Bootstrap runtime-core crate and Rust dependency governance

## 1. Prime Directive

Establish the governed runtime-core bootstrap so later runtime work starts from a real crate boundary with one approved dependency policy and one canonical error boundary.

## 2. Specs

- **Module:** `runtime/core/`
- **Dependencies:** Rust `std`, `thiserror`

## 3. Checklist

### 3.1. Phase A - Governance bootstrap

- [x] Create `doc/project/project-0003-rust-dependencies.md`
- [x] Record `thiserror` as the approved Rust dependency for runtime-core bootstrap
- [x] Validation vector for Phase A completed

### 3.2. Phase B - Crate bootstrap

- [x] Create the `runtime-core` crate under `runtime/core/`
- [x] Add the crate to the workspace and declare the `thiserror` dependency
- [x] Define `RuntimeError` and `RuntimeResult`
- [x] Validation vector for Phase B completed

### 3.3. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] Task status updated after implementation outcome is known

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All completed work maps to an accepted phase item
- [x] No runtime-core code beyond bootstrap error and result boundary was added
- [x] Dependency governance matches the workspace dependency boundary
- [x] All quality gates pass
