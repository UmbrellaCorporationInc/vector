---
id: task-00004-extract-tokio-backed-runtime-channel-crate-and-remove-runtime-core-channel-implementation
type: task
code: "00004"
slug: extract-tokio-backed-runtime-channel-crate-and-remove-runtime-core-channel-implementation
title: Extract Tokio-Backed Runtime Channel Crate And Remove Runtime-Core Channel Implementation
description: Remove the concrete channel implementation from runtime-core, initialize the runtime-channel crate, and document Tokio as an approved dependency for that crate.
status: done
created: 2026-05-03
updated: 2026-05-03
tags:
  - runtime
  - channels
  - tokio
  - governance
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
  - rfc-00004-tokio-backed-runtime-channel-implementation-crate
  - project-0003-rust-dependencies
supersedes: []
superseded_by: null
---

# Task 00004: Extract Tokio-Backed Runtime Channel Crate And Remove Runtime-Core Channel Implementation

## 1. Prime Directive

Separate channel contract ownership from channel backend ownership so `runtime-core` remains a transport-agnostic contract crate while the standard runtime-owned implementation moves into a dedicated Tokio-backed crate.

## 2. Specs

- **Module:** `runtime/core/`, `runtime/channel/`
- **Dependencies:** Rust `std`, `thiserror`, `tokio`

## 3. Checklist

### 3.1. Phase A - Runtime-core channel ownership cleanup

- [x] Remove the concrete connected-endpoint implementation from `runtime-core`
- [x] Keep `Sender<T>`, `Receiver<T>`, `CancelableSender<T>`, `CancelableReceiver<T>`, the `CancelHandler` trait, `RuntimeError`, and `RuntimeResult` in `runtime-core`
- [x] Remove `runtime-core` factory ownership for the standard connected channel constructors
- [x] Remove channel-behavior tests from `runtime-core`
- [x] Update `runtime-core` crate documentation to reflect contract-only ownership with no channel implementation logic
- [x] Validation vector for Phase A completed
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Runtime-channel crate bootstrap

- [x] Create the `runtime-channel` crate under `runtime/channel/`
- [x] Add `runtime-channel` to the workspace and declare its dependency on `runtime-core`
- [x] Add `tokio` as the backend dependency required by the crate
- [x] Define a `runtime-channel` configuration value that supplies bounded channel capacity
- [x] Define the concrete `CancelHandler` implementation in `runtime-channel`
- [x] Define the standard Tokio-backed base channel implementation in `runtime-channel`
- [x] Define the standard Tokio-backed cancel-aware channel implementation in `runtime-channel`
- [x] Ensure the standard implementation uses bounded `tokio::sync::mpsc` rather than unbounded transport
- [x] Ensure cancellation state uses a shared flag plus `tokio::sync::watch` for await wake-up
- [x] Add channel-oriented utilities that are specific to channel configuration or cancellation behavior only when they belong to the crate boundary
- [x] Validation vector for Phase B completed
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Factory and API boundary alignment

- [x] Expose `runtime-channel::channel<T>() -> (impl Sender<T>, impl Receiver<T>)`
- [x] Expose `runtime-channel::cancelable_channel<T>() -> (impl CancelHandler, impl CancelableSender<T>, impl CancelableReceiver<T>)`
- [x] Hide concrete endpoint types behind `impl Trait` wherever the accepted public boundary permits it
- [x] Route the standard factory construction path through the configured bounded channel capacity
- [x] Preserve one-publisher and one-subscriber semantics across both factories
- [x] Preserve cancel-aware substitution against the base sender and receiver contracts
- [x] Ensure pending awaits can be released when cancellation is signalled
- [x] Add tests proving the new crate preserves the accepted async-first and cancellation semantics
- [x] Validation vector for Phase C completed
- [x] execute section "4. Quality Gate"

### 3.4. Phase D - Dependency governance and documentation

- [x] Record `tokio` in `doc/project/project-0003-rust-dependencies.md` as approved for `runtime-channel`
- [x] State explicitly that `tokio` is not approved as a dependency of `runtime-core` by this task
- [x] Document that the standard Tokio-backed channel uses bounded `mpsc` and that capacity is configured through a `runtime-channel` configuration value
- [x] Document that cancellation uses a shared flag for state and `tokio::sync::watch` for wake-up of pending awaits
- [x] Update crate README files affected by the extraction
- [x] Record any RFC 00003 wording that must move factory ownership out of `runtime-core`
  - RFC 00003 §2 "Implementation strategy" currently places standard factory ownership inside `runtime-core`. RFC 00004 acceptance criterion #20 records the follow-up obligation: RFC 00003 (or an equivalent channel contract document) must be updated so factory ownership is no longer described as living in `runtime-core`. That update is out of scope for Phase D but must be tracked as a follow-up before RFC 00003 is accepted.
- [x] Validation vector for Phase D completed
- [x] execute section "4. Quality Gate"

### 3.5. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update task status after implementation outcome is known

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `runtime-core` remains transport-agnostic after the extraction
- [x] `runtime-core` no longer owns the standard concrete channel implementation, channel behavior logic, or channel behavior tests
- [x] `runtime-channel` becomes the standard runtime-owned implementation crate for the accepted channel contracts
- [x] Public factories use `impl Trait` wherever the accepted contract boundary allows it
- [x] The standard Tokio-backed implementation uses bounded `tokio::sync::mpsc` with capacity supplied by `runtime-channel` configuration
- [x] `CancelHandler` is a trait in `runtime-core` and its standard implementation resides in `runtime-channel`
- [x] The new implementation preserves async-first behavior with no blocking work before the returned future is polled or awaited
- [x] Cancellation uses a shared flag for observable state and `tokio::sync::watch` to release pending awaits
- [x] Cancellation remains distinguishable from ordinary channel closure through the accepted `is_cancelled()` observation rule
- [x] Dependency governance documents `tokio` for `runtime-channel` without widening the `runtime-core` boundary
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
