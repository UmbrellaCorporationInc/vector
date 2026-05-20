---
id: task-00002-implement-rfc-00003-channel-contracts-and-cancel-aware-factories
type: task
code: "00002"
slug: implement-rfc-00003-channel-contracts-and-cancel-aware-factories
title: Implement RFC 00003 channel contracts and cancel-aware factories
description: Implement the runtime-core channel contracts from RFC 00003, including Sender, Receiver, cancel-aware specializations, and standard channel factories.
status: done
created: 2026-05-03
updated: 2026-05-03
tags:
  - runtime
  - channels
  - async
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
supersedes: []
superseded_by: null
---

# Task 00002: Implement RFC 00003 channel contracts and cancel-aware factories

## 1. Prime Directive

Establish one coherent channel boundary in `runtime-core` so higher-level runtime components can depend on stable async sender, receiver, and cancel-aware contracts without inventing incompatible channel semantics.

## 2. Specs

- **Module:** `runtime/core/`
- **Dependencies:** Rust `std`, `thiserror`

## 3. Checklist

### 3.1. Phase A - Base channel contracts

- [x] Define the `Sender<T>` and `Receiver<T>` contracts in `runtime-core`
- [x] Implement the standard `channel<T>()` factory boundary
- [x] Add tests covering connected sender and receiver behavior
- [x] Validation vector for Phase A completed
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Cancel-aware channel contracts

- [x] Define `CancelHandler`, `CancelableSender<T>`, and `CancelableReceiver<T>`
- [x] Implement the standard `cancelable_channel<T>()` factory boundary
- [x] Add tests covering shared cancellation state across both endpoints
- [x] Validation vector for Phase B completed
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Contract integration

- [x] Ensure cancel-aware contracts remain compatible with base sender and receiver boundaries
- [x] Ensure the public API matches RFC 00003 ownership and non-goals
- [x] Add tests covering substitution and factory return semantics
- [x] Validation vector for Phase C completed
- [x] execute section "4. Quality Gate"

### 3.4. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] Base channel contracts remain single publisher and single subscriber only
- [x] Cancel-aware contracts do not force cancellation semantics into base sender and receiver contracts
- [x] `channel<T>()` and `cancelable_channel<T>()` preserve the accepted RFC 00003 boundary
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
