---
id: rfc-00006-runtime-core-control-observability-and-encoding-primitives
type: rfc
code: "00006"
slug: runtime-core-control-observability-and-encoding-primitives
title: Runtime Core Control, Observability, and Encoding Primitives
description: Defines the v1 contracts for ControlEvent, ObservabilityEvent, and Encoding in runtime-core.
status: implemented
created: 2026-05-03
updated: 2026-05-03
authors: []
tags:
  - runtime
  - async
  - events
  - encoding
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00005-runtime-core-operation-and-event-flow-primitives
supersedes: []
superseded_by: null
aliases:
  - "RFC 00006: Runtime Core Control, Observability, and Encoding Primitives"
---

# RFC 00006: Runtime Core Control, Observability, and Encoding Primitives

## 1. Problem

`rfc-00002-runtime-core-v1-boundary-and-async-first-contracts` accepts `ControlEvent`, `ObservabilityEvent`, and `Encoding` into the `runtime-core` boundary but intentionally leaves their concrete contracts undefined. `rfc-00005-runtime-core-operation-and-event-flow-primitives` defines the `EventEmitter<Event>` and `EventListener<Event>` contracts and explicitly defers concrete event taxonomy definitions to later RFCs.

Without a dedicated RFC for these three primitives, the first implementation will likely drift in one of two bad directions:

- `ControlEvent` and `ObservabilityEvent` grow as ad hoc enums with product-specific cases added without boundary discipline, turning them into feature buckets embedded in `runtime-core`
- `Encoding` is improvised per crate, producing inconsistent UTF-8 enforcement across the shared runtime foundation

The project needs one minimal shared event vocabulary for runtime control and observability flows, and one canonical UTF-8 boundary for shared text primitives, before higher-level runtime crates begin wiring event emission and text conversion against `runtime-core`.

This RFC follows [[rfc-00002-runtime-core-v1-boundary-and-async-first-contracts]] and depends on the event contracts defined in [[rfc-00005-runtime-core-operation-and-event-flow-primitives]].

## 2. Proposal

Define three contract families in `runtime-core` v1:

- `ControlEvent`
- `ObservabilityEvent`
- `Encoding`

### ControlEvent

`ControlEvent` is the canonical shared event vocabulary for runtime control flows.

- `ControlEvent` is a narrow enum with cases that cover cross-cutting runtime control signals only
- In v1, `ControlEvent` must include at least one cancellation-oriented case (`Cancel`) to support `CancelHandler` wiring and cancel-aware channel coordination
- `ControlEvent` must not encode product-specific workflow steps, command behaviors, repository policy, or transport-specific signals
- `ControlEvent` cases must remain minimal in v1 — new cases require an RFC amendment or a successor RFC
- `ControlEvent` is the intended event type for `EventEmitter<ControlEvent>` and `EventListener<ControlEvent>` in cancellation and control flows

### ObservabilityEvent

`ObservabilityEvent<P>` is the canonical shared event vocabulary for runtime observability flows, parameterized over a caller-supplied payload type.

- `ObservabilityEvent<P>` is a narrow generic enum with cases that cover cross-cutting runtime observability signals only
- The payload type parameter `P` must satisfy `P: Debug + Clone + Send + 'static` — this is the minimum bound required for use in async observability contexts without introducing third-party dependencies
- In v1, `ObservabilityEvent<P>` must include at least cases for operation start, operation completion, and message emission to support basic async execution and flow observability
- `OperationStarted` and `OperationCompleted` carry no payload — they identify the operation by `operation_id` only
- `MessageSent` carries an `operation_id` and a `payload: P`, allowing plugin execution and flow operations to attach domain-specific output to the observability boundary without `ObservabilityEvent` knowing the concrete type
- `ObservabilityEvent<P>` must not encode product-specific telemetry schemas, feature-specific metrics, command execution details, or transport-specific observability data
- `ObservabilityEvent<P>` cases must remain minimal in v1 — new cases require an RFC amendment or a successor RFC
- `ObservabilityEvent<P>` is the intended event type for `EventEmitter<ObservabilityEvent<P>>` and `EventListener<ObservabilityEvent<P>>` in observability wiring, including `PluginDispatcher` attachment
- Each `ObservabilityEvent<P>` case must carry only the minimum data needed to express the observability signal at the `runtime-core` boundary — full telemetry schemas belong in higher-level crates

### Encoding

`Encoding` is the canonical shared boundary for validated conversion between `String` and UTF-8 bytes in shared runtime flows.

- `Encoding` enforces UTF-8 as the exclusive canonical text encoding for `runtime-core`
- `Encoding` must expose at least:
  - a function that converts a `String` to `Vec<u8>` under UTF-8 encoding
  - a function that validates and converts a `Vec<u8>` or `&[u8]` to `String` under UTF-8, returning a `RuntimeResult<String>` on invalid input
- `Encoding` must not introduce codec abstractions, configurable encoding variants, or alternative text encoding support
- `Encoding` must remain stateless in v1 — instance-based or configurable encoding behavior is a non-goal
- `Encoding` may be implemented as a unit struct, a module-level namespace, or a stateless enum variant, provided that the public API surface remains consistent with the accepted boundary
- All text primitives in `runtime-core` that convert between bytes and text must use `Encoding` as the canonical boundary rather than ad hoc UTF-8 handling

### Ownership boundary

- This RFC defines contracts only
- This RFC does not define domain-specific event routing policy, telemetry schemas, or transport adapters
- This RFC does not define plugin-specific events — plugin execution events should be defined in a separate RFC that references `ObservabilityEvent` as a foundation
- This RFC does not introduce third-party encoding dependencies — `Encoding` relies on Rust `std` UTF-8 guarantees only
- This RFC does not define multi-encoding support or codec abstraction

### Compatibility rules

- `ControlEvent` and `ObservabilityEvent` must be usable as the event type parameter of `EventEmitter<Event>` and `EventListener<Event>` without modification to those contracts
- `Encoding` must not depend on any crate outside of Rust `std` and `thiserror`
- Both event types and `Encoding` must satisfy the `runtime-core` inclusion rule: they are required across async runtime crates without feature-specific policy

### Design rules

- `ControlEvent` and `ObservabilityEvent` should be kept so narrow that adding a case in v1 requires justification and RFC coverage
- `ObservabilityEvent` payloads should carry only stable identifiers, not mutable state or complex nested types
- `Encoding` should feel like a zero-cost UTF-8 boundary rather than a general-purpose codec — if the conversion is trivially expressible with `std`, the struct should wrap it minimally

### Proposed contract shape

The following shape is proposed as the v1 direction. It is illustrative of the intended boundary and may be tightened during implementation without changing the accepted ownership model.

```rust
use crate::{RuntimeError, RuntimeResult};
use std::fmt::Debug;

#[non_exhaustive]
pub enum ControlEvent {
    Cancel,
}

#[non_exhaustive]
pub enum ObservabilityEvent<P>
where
    P: Debug + Clone + Send + 'static,
{
    OperationStarted { operation_id: String },
    OperationCompleted { operation_id: String },
    MessageSent { operation_id: String, payload: P },
}

pub struct Encoding;

impl Encoding {
    pub fn encode(text: &str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }

    pub fn decode(bytes: &[u8]) -> RuntimeResult<String> {
        std::str::from_utf8(bytes)
            .map(|s| s.to_owned())
            .map_err(|e| RuntimeError::encoding(e))
    }
}
```

Shape intent:

- `ControlEvent` is `#[non_exhaustive]` so that new control cases can be added without breaking downstream match arms — callers must handle the unknown-case arm from the start
- `ObservabilityEvent<P>` is `#[non_exhaustive]` for the same reason
- `OperationStarted` and `OperationCompleted` carry no payload — they identify the operation by `operation_id` only, keeping lifecycle signals independent of domain types
- `MessageSent` carries `payload: P` to let plugin execution and flow operations attach domain-specific output to the observability boundary without requiring `ObservabilityEvent` to know the concrete type
- The `P: Debug + Clone + Send + 'static` bound is the minimum required for use in async observability contexts — `Debug` for diagnostics, `Clone` for fan-out to multiple listeners, `Send + 'static` for safe transfer across async task boundaries — without introducing any third-party dependency
- `ObservabilityEvent<P>` payloads use `String` for operation identity in v1 — this keeps the type dependency minimal and defers stable identifier types to later RFCs
- `Encoding::encode` is infallible because `String` in Rust is always valid UTF-8
- `Encoding::decode` returns `RuntimeResult<String>` because byte slices may not be valid UTF-8
- `RuntimeError::encoding` must be added to `RuntimeError` as a variant for UTF-8 decode failures
- Both event types should implement `Clone`, `Debug`, and `PartialEq` as minimum derived traits — `PartialEq` on `ObservabilityEvent<P>` requires `P: PartialEq`, so the derived impl is conditional on that bound

## 3. Alternatives Considered

- **`ControlEvent` and `ObservabilityEvent` as open trait objects instead of enums:** Discarded because trait objects would weaken the minimal shared vocabulary contract and make match-based dispatch impossible at the `runtime-core` level.
- **A single shared `RuntimeEvent` enum combining control and observability cases:** Discarded because control signals and observability signals are fundamentally different routing concerns and conflating them would force every event consumer to handle irrelevant cases.
- **`ObservabilityEvent` without a type parameter, using `Box<dyn Any>` for payload:** Discarded because it erases the payload type at the contract boundary, forces heap allocation, and makes listener-side payload inspection unsafe and error-prone.
- **`ObservabilityEvent<P>` with `P` bound to `P: Serialize` for telemetry forwarding:** Discarded because `Serialize` introduces a third-party dependency (`serde`) that violates the v1 dependency rule — serialization concerns belong in higher-level crates.
- **Making `OperationStarted` and `OperationCompleted` also carry `P`:** Discarded because lifecycle signals are independent of domain output — coupling them to the payload type would force callers to supply a payload value for events that semantically have none.
- **`ObservabilityEvent` with rich telemetry payloads in v1:** Discarded because rich telemetry schemas are feature-specific and would turn `ObservabilityEvent` into a product-specific data model embedded in the shared runtime foundation.
- **`Encoding` as a configurable instance with runtime-selected encoding:** Discarded because `runtime-core` enforces UTF-8 as the only canonical encoding and alternative encoding support is an explicit non-goal for v1.
- **`Encoding` as a free-standing module instead of a named type:** Discarded because a named type provides a cleaner import boundary for the canonical UTF-8 contract and makes the exclusion of alternative encodings explicit in the type system.
- **UTF-8 enforcement handled ad hoc inside each crate:** Discarded because inconsistent UTF-8 handling across runtime crates is exactly the problem `runtime-core` is meant to prevent at the shared boundary level.
- **`ControlEvent::Cancel` carrying a cancellation reason payload in v1:** Discarded because payloads introduce schema questions that belong in a later RFC — the minimal v1 case establishes the signal with no payload.
- **Deriving `Hash` on event types in v1:** Not included because the generic `P` parameter and future `#[non_exhaustive]` cases make `Hash` stability harder to guarantee across crate versions; this can be added later if needed.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| `#[non_exhaustive]` on both event types lets the vocabulary grow in later RFCs without a semver break. | Callers must always provide a wildcard match arm, which adds a small boilerplate cost to every event consumer. |
| Keeping `ControlEvent` to a single `Cancel` case in v1 prevents premature expansion into workflow policy. | A single-case enum may feel underweight until more control signals are justified and accepted via RFC. |
| `ObservabilityEvent<P>` with `OperationStarted`, `OperationCompleted`, and `MessageSent` gives plugin dispatch and flow execution one minimal observability vocabulary with typed payload support from the start. | The generic parameter propagates into every `EventEmitter<ObservabilityEvent<P>>` instantiation, increasing type annotation burden at call sites. |
| `P: Debug + Clone + Send + 'static` lets listeners clone and inspect payloads in async contexts with no third-party dependency. | The `Clone` bound means every `MessageSent` payload must be cloneable — callers with non-clonable domain types must wrap before emitting. |
| Keeping `OperationStarted` and `OperationCompleted` free of `P` preserves clean lifecycle semantics independent of domain output types. | Callers that want a unified match over all variants must still handle the type parameter even for the payload-free cases. |
| `Encoding` as a stateless struct enforces UTF-8 at the contract boundary without any runtime overhead. | `Encoding` cannot accommodate non-UTF-8 integrations at the `runtime-core` level — adapters must live outside the crate. |
| Infallible `Encoding::encode` and fallible `Encoding::decode` match the actual guarantee profile of Rust `String` vs `&[u8]`. | `Encoding::decode` failure requires `RuntimeError` to carry a UTF-8 error variant, slightly expanding the error surface. |
| `PartialEq` on `ObservabilityEvent<P>` is conditional on `P: PartialEq`, so the derived bound is honest about what the impl requires. | Downstream code that tests observability events must ensure their payload types also implement `PartialEq`. |
| Separating `ControlEvent` from `ObservabilityEvent<P>` keeps routing concerns clean and lets consumers subscribe to only the relevant vocabulary. | Two separate event types mean two separate `EventEmitter` and `EventListener` instantiations in components that care about both. |

## 5. Acceptance Criteria

- [ ] `runtime-core` exposes `ControlEvent` as a `#[non_exhaustive]` enum.
- [ ] `ControlEvent` includes at least a `Cancel` variant in v1.
- [ ] `ControlEvent` does not include product-specific workflow, command, transport, or policy cases.
- [ ] `runtime-core` exposes `ObservabilityEvent<P>` as a `#[non_exhaustive]` generic enum.
- [ ] `ObservabilityEvent<P>` requires `P: Debug + Clone + Send + 'static`.
- [ ] `ObservabilityEvent<P>` includes `OperationStarted { operation_id: String }` in v1.
- [ ] `ObservabilityEvent<P>` includes `OperationCompleted { operation_id: String }` in v1.
- [ ] `ObservabilityEvent<P>` includes `MessageSent { operation_id: String, payload: P }` in v1.
- [ ] `OperationStarted` and `OperationCompleted` carry no payload — operation identity only.
- [ ] `ObservabilityEvent<P>` does not include feature-specific telemetry schemas, product metrics, or transport-specific observability data.
- [ ] `ControlEvent` derives `Clone`, `Debug`, and `PartialEq`.
- [ ] `ObservabilityEvent<P>` derives `Clone` and `Debug`; derives `PartialEq` conditionally on `P: PartialEq`.
- [ ] `ControlEvent` is usable as the event type parameter of `EventEmitter<ControlEvent>` and `EventListener<ControlEvent>` without modification to those contracts.
- [ ] `ObservabilityEvent<P>` is usable as the event type parameter of `EventEmitter<ObservabilityEvent<P>>` and `EventListener<ObservabilityEvent<P>>` without modification to those contracts.
- [ ] `runtime-core` exposes `Encoding` as the canonical shared boundary for validated UTF-8 conversion.
- [ ] `Encoding::encode` converts a `&str` to `Vec<u8>` and is infallible.
- [ ] `Encoding::decode` converts a `&[u8]` to `String`, returns `RuntimeResult<String>`, and fails with a typed `RuntimeError` variant on invalid UTF-8.
- [ ] `Encoding` introduces no configurable encoding variants, alternative text encodings, or codec abstractions.
- [ ] `Encoding` is stateless in v1.
- [ ] `RuntimeError` includes a variant for UTF-8 decode failure used by `Encoding::decode`.
- [ ] All text primitives in `runtime-core` that convert between bytes and text use `Encoding` as the canonical boundary.
- [ ] `Encoding` depends only on Rust `std` and `thiserror` — no additional third-party crates.
- [ ] No accepted type in this RFC encodes VECTOR feature policy, document schema policy, protocol adapter behavior, or workflow-specific decisions.
- [ ] Plugin-specific observability events are not defined in this RFC.

## 6. Open Questions

- Should `ControlEvent::Cancel` carry a cancellation token or reason string in a follow-up RFC, or should the minimal no-payload case remain the permanent v1 shape?
- Should `ObservabilityEvent<P>` include an `OperationFailed { operation_id: String, ... }` variant in v1, or should failure observability wait for a follow-up RFC once the error surface is more stable?
- Should `ObservabilityEvent<P>` use a stable `OperationId` type rather than a plain `String` once the project has an accepted identifier contract?
- Should `Encoding` expose a `decode_lossy` method in v1 for cases where callers explicitly want replacement-character behavior, or is lossless-only the correct v1 constraint?
- Should `ControlEvent` and `ObservabilityEvent<P>` require `Send + Sync` bounds explicitly in the contract, or should the compiler infer those bounds from concrete instantiations?
- Should additional `ControlEvent` cases — such as `Pause` or `Resume` — be reserved as named non-exhaustive placeholders in v1, or should the enum start with a single `Cancel` case and grow strictly via RFC amendment?
- Should `P` in `ObservabilityEvent<P>` carry a default type parameter (e.g., `P = ()`) so that callers who only emit lifecycle events can omit the type annotation entirely?
