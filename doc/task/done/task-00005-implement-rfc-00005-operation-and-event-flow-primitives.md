---
id: task-00005-implement-rfc-00005-operation-and-event-flow-primitives
type: task
code: "00005"
slug: implement-rfc-00005-operation-and-event-flow-primitives
title: Implement RFC 00005 operation and event flow primitives
description: Implement the runtime-core operation and event flow contracts from RFC 00005, including the runtime-channel event emitter implementation.
status: done
created: 2026-05-03
updated: 2026-05-03
tags:
  - runtime
  - async
  - operation
  - events
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
  - rfc-00005-runtime-core-operation-and-event-flow-primitives
supersedes: []
superseded_by: null
---

# Task 00005: Implement RFC 00005 operation and event flow primitives

## 1. Prime Directive

Establish one coherent async execution and event-flow boundary in `runtime-core`, then provide the standard runtime-owned event emitter implementation in `runtime-channel` so higher-level crates can depend on shared contracts without inventing incompatible operation or fan-out semantics.

## 2. Specs

- **Module:** `runtime/core/`, `runtime/channel/`
- **Dependencies:** Rust `std`, `thiserror`, `tokio`, `runtime-core`

## 3. Checklist

### 3.1. Phase A - Operation contracts in runtime-core

- [x] Define the accepted `Operation` contract shape for `1:1` execution in `runtime-core`
- [x] Define the accepted receiver-driven operation contract shape for `N:1` execution in `runtime-core`
- [x] Add tests covering async input-result behavior and contract substitution boundaries
- [x] Validation vector for Phase A completed
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Flow operation contracts in runtime-core

- [x] Define the accepted `FlowOperation` contract shape for `1:N` execution in `runtime-core`
- [x] Define the accepted receiver-driven flow operation contract shape for `N:N` execution in `runtime-core`
- [x] Ensure flow contracts use `Sender<Output>` and `Receiver<Input>` consistently with RFC 00003
- [x] Add tests covering sender-backed output behavior and receiver-driven flow boundaries
- [x] Validation vector for Phase B completed
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Event contracts in runtime-core

- [x] Define the accepted `EventEmitter<Event>` contract in `runtime-core`
- [x] Define the accepted `EventListener<Event>` contract in `runtime-core`
- [x] Ensure listener registration is part of the emitter contract without exposing concrete registry storage policy
- [x] Add tests covering contract-level event emission and listener registration boundaries
- [x] Validation vector for Phase C completed
- [x] execute section "4. Quality Gate"

### 3.4. Phase D - Event contract realignment in runtime-core

- [x] Make `EventEmitter<Event>` compatible with `Sender<Event>`
- [x] Provide `emit` as a default alias over `send` when the accepted Rust trait shape allows implementors to avoid duplicate publication logic
- [x] Replace the receiver-style event listener model with a dedicated `EventListener<Event>` contract designed for emitter-driven delivery
- [x] Update tests to cover sender substitution and dedicated listener behavior
- [x] Validation vector for Phase D completed
- [x] execute section "4. Quality Gate"

### 3.5. Phase E - Event emitter implementation in runtime-channel

- [x] Implement the standard concrete `EventEmitter` in `runtime-channel`
- [x] Use the accepted `runtime-core` event contracts without moving broadcaster policy into `runtime-core`
- [x] Implement fan-out over listener endpoints using the runtime-channel backend shape accepted by RFC 00005
- [x] If Tokio remains the selected runtime backend, keep the emitter implementation Tokio-backed
- [x] Add tests covering listener registration, event broadcast, closed-listener handling, and per-listener delivery ordering
- [x] Validation vector for Phase E completed
- [x] execute section "4. Quality Gate"

### 3.6. Phase F - Public API integration

- [x] Re-export the accepted operation and event primitives from `runtime-core`
- [x] Expose the standard event emitter implementation from `runtime-channel`
- [x] Update package README files to document the new contracts and the standard emitter implementation boundary
- [x] Validation vector for Phase F completed
- [x] execute section "4. Quality Gate"

### 3.7. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` (cargo fmt) passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [ ] `xtask quality-lint` passes
- [ ] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `Operation` remains limited to `1:1` and `N:1` execution shapes
- [x] `FlowOperation` remains limited to `1:N` and `N:N` execution shapes
- [x] `EventEmitter` and `EventListener` remain contract-only primitives in `runtime-core`
- [x] `EventEmitter` remains substitutable anywhere a `Sender<Event>` is accepted
- [x] `EventListener` remains a dedicated event contract rather than a `Receiver<Event>` specialization
- [x] The standard broadcaster implementation remains in `runtime-channel`
- [x] Listener registration does not force concrete registry storage policy into `runtime-core`
- [x] Event fan-out behavior is validated in `runtime-channel` tests rather than encoded as transport policy in `runtime-core`
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
