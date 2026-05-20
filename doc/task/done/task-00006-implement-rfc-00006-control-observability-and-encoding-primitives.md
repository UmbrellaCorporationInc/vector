---
id: task-00006-implement-rfc-00006-control-observability-and-encoding-primitives
type: task
code: "00006"
slug: implement-rfc-00006-control-observability-and-encoding-primitives
title: Implement RFC 00006 control, observability, and encoding primitives
description: Implement the ControlEvent, ObservabilityEvent, and Encoding contracts in runtime-core as defined in RFC 00006.
status: done
created: 2026-05-03
updated: 2026-05-03
tags:
  - runtime
  - async
  - events
  - encoding
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00005-runtime-core-operation-and-event-flow-primitives
  - rfc-00006-runtime-core-control-observability-and-encoding-primitives
supersedes: []
superseded_by: null
---

# Task 00006: Implement RFC 00006 control, observability, and encoding primitives

## 1. Prime Directive

Establish the three remaining shared vocabulary primitives in `runtime-core` — `ControlEvent`, `ObservabilityEvent<P>`, and `Encoding` — so that higher-level crates can wire cancellation control, typed observability signals, and UTF-8 text conversion against stable core contracts without improvising incompatible local solutions.

## 2. Specs

- **Module:** `runtime/core/`
- **Dependencies:** Rust `std`, `thiserror`, `runtime-core`

## 3. Checklist

### 3.1. Phase A — ControlEvent

- [x] Define `ControlEvent` as a `#[non_exhaustive]` enum with a `Cancel` variant
- [x] Derive `Clone`, `Debug`, and `PartialEq` on `ControlEvent`
- [x] Verify `ControlEvent` is usable as the event type parameter of `EventEmitter<ControlEvent>` and `EventListener<ControlEvent>`
- [x] Add tests covering match exhaustiveness with the wildcard arm and basic clone and equality behavior
- [x] Validation vector for Phase A completed
- [x] execute section "4. Quality Gate"

### 3.2. Phase B — ObservabilityEvent

- [x] Define `ObservabilityEvent<P>` as a `#[non_exhaustive]` generic enum with bound `P: Debug + Clone + Send + 'static`
- [x] Add `OperationStarted { operation_id: String }` variant — no payload
- [x] Add `OperationCompleted { operation_id: String }` variant — no payload
- [x] Add `MessageSent { operation_id: String, payload: P }` variant
- [x] Derive `Clone` and `Debug` unconditionally; derive `PartialEq` conditionally on `P: PartialEq`
- [x] Verify `ObservabilityEvent<P>` is usable as the event type parameter of `EventEmitter<ObservabilityEvent<P>>` and `EventListener<ObservabilityEvent<P>>`
- [x] Add tests covering all three variants, wildcard arm, clone behavior, and conditional `PartialEq`
- [x] Validation vector for Phase B completed
- [x] execute section "4. Quality Gate"

### 3.3. Phase C — Encoding

- [x] Define `Encoding` as a stateless unit struct
- [x] Implement `Encoding::encode(text: &str) -> Vec<u8>` as an infallible UTF-8 encoder
- [x] Implement `Encoding::decode(bytes: &[u8]) -> RuntimeResult<String>` as a fallible UTF-8 decoder
- [x] Add a UTF-8 decode failure variant to `RuntimeError` used by `Encoding::decode`
- [x] Add tests covering round-trip encode/decode, invalid UTF-8 rejection, and typed error propagation
- [x] Validation vector for Phase C completed
- [x] execute section "4. Quality Gate"

### 3.4. Phase D — Public API integration

- [x] Re-export `ControlEvent`, `ObservabilityEvent`, and `Encoding` from the `runtime-core` crate root
- [x] Update the `runtime-core` README to document the three new primitives and their accepted contracts
- [x] Validation vector for Phase D completed
- [x] execute section "4. Quality Gate"

### 3.5. Phase Z — Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `ControlEvent` is `#[non_exhaustive]` with at least `Cancel`
- [x] `ControlEvent` introduces no product-specific workflow, command, transport, or policy cases
- [x] `ObservabilityEvent<P>` is `#[non_exhaustive]` with `OperationStarted`, `OperationCompleted`, and `MessageSent`
- [x] `OperationStarted` and `OperationCompleted` carry no payload
- [x] `MessageSent` carries `payload: P` bound to `P: Debug + Clone + Send + 'static`
- [x] `ObservabilityEvent<P>` introduces no feature-specific telemetry schemas or transport-specific data
- [x] `Encoding` is stateless and enforces UTF-8 exclusively
- [x] `Encoding::encode` is infallible
- [x] `Encoding::decode` returns `RuntimeResult<String>` and fails with a typed `RuntimeError` variant
- [x] `RuntimeError` carries a UTF-8 decode failure variant
- [x] No new third-party dependencies introduced beyond `std` and `thiserror`
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
