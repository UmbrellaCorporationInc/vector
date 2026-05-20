---
id: rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
type: rfc
code: "00003"
slug: runtime-core-channel-contracts-and-cancel-aware-specializations
title: Runtime Core Channel Contracts and Cancel-Aware Specializations
description: Defines the v1 contracts for Sender, Receiver, CancelableSender, and CancelableReceiver in runtime-core.
status: implemented
created: 2026-05-03
updated: 2026-05-03
authors: []
tags:
  - runtime
  - async
  - channels
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
supersedes: []
superseded_by: null
aliases:
  - "RFC 00003: Runtime Core Channel Contracts and Cancel-Aware Specializations"
---

# RFC 00003: Runtime Core Channel Contracts and Cancel-Aware Specializations

## 1. Problem

`rfc-00002-runtime-core-v1-boundary-and-async-first-contracts` accepts `Sender`, `Receiver`, `CancelableSender`, and `CancelableReceiver` into the `runtime-core` boundary, but it intentionally does not define their concrete contracts.

Without a dedicated channel RFC, the first implementation will likely drift in one of two directions:

- channels become concrete implementation details rather than stable runtime contracts
- cancellation semantics leak into every sender and receiver whether needed or not

The project needs one minimal async message boundary that higher-level runtime crates can depend on before `FlowOperation`, plugin dispatch, or event routing can be implemented coherently.

This RFC follows [[spec-00002-runtime-core-crate]] and refines [[rfc-00002-runtime-core-v1-boundary-and-async-first-contracts]].

## 2. Proposal

Define four channel contracts in `runtime-core` v1:

- `Sender<T>`
- `Receiver<T>`
- `CancelableSender<T>`
- `CancelableReceiver<T>`
- `CancelHandler`

Contract boundary:

- `Sender<T>` is the canonical async contract for publishing values of type `T`
- `Receiver<T>` is the canonical async contract for consuming values of type `T`
- the base channel model is one publisher and one subscriber
- the contracts are transport-agnostic and must not assume a concrete runtime or channel implementation

Cancellation boundary:

- `CancelableSender<T>` is a cancel-aware specialization compatible with `Sender<T>`
- `CancelableReceiver<T>` is a cancel-aware specialization compatible with `Receiver<T>`
- `CancelHandler` is the shared cancellation control trait boundary for connected cancel-aware endpoints
- `CancelableSender<T>` exposes `is_cancelled`
- `CancelableReceiver<T>` exposes `is_cancelled`
- cancellation awareness is optional and must not be forced into every sender or receiver contract

Ownership boundary:

- this RFC defines channel contracts, not a concrete queue, buffer, or executor
- this RFC defines `CancelHandler` as a trait boundary, not a concrete implementation type
- this RFC does not define backpressure policy
- this RFC does not define retry policy
- this RFC does not define ordering guarantees beyond what a concrete implementation may later document
- this RFC does not define lifecycle supervision or scheduling
- this RFC does not define multi-producer or multi-consumer semantics

Compatibility rule:

- every `CancelableSender<T>` must remain usable anywhere a `Sender<T>` is accepted
- every `CancelableReceiver<T>` must remain usable anywhere a `Receiver<T>` is accepted

Design rule:

- channel contracts should be trait-based unless a later RFC proves a concrete-only boundary is necessary
- the contracts should stay minimal enough to support multiple implementations without leaking runtime-specific policy into `runtime-core`

Implementation strategy:

- v1 should target modern stable Rust async features
- `Sender<T>` and `Receiver<T>` may use `async fn` in trait contracts
- `runtime-core` should expose the contracts required by standard channel factories without owning the standard concrete factory implementation
- a runtime-owned implementation crate may provide a standard `channel<T>()` factory for constructing one connected sender and one connected receiver
- a runtime-owned implementation crate may provide a standard `cancelable_channel<T>()` factory for constructing one connected cancel-aware sender, one connected cancel-aware receiver, and one shared cancel handler
- the preferred v1 factory shape is `channel<T>() -> (impl Sender<T>, impl Receiver<T>)`
- the preferred v1 cancel-aware factory shape is `cancelable_channel<T>() -> (impl CancelHandler, impl CancelableSender<T>, impl CancelableReceiver<T>)`
- the standard factory should hide its concrete connected endpoint types behind `impl Trait`
- the cancel handler returned by the standard cancel-aware factory should be able to cancel both connected endpoints
- `CancelableSender<T>::is_cancelled` and `CancelableReceiver<T>::is_cancelled` should reflect the shared cancellation state controlled by the returned handler
- the standard factory must preserve the one publisher and one subscriber boundary accepted by this RFC
- if public async trait bounds require refinement during implementation, the exact signature may be tightened without changing the accepted boundary of one connected sender and one connected receiver returned by the standard factory

Implementation staging note:

- the detailed method set for each contract may evolve during implementation, but the accepted boundary for v1 is one sender contract, one receiver contract, and one cancel-aware specialization for each

## 3. Alternatives Considered

- **Concrete channel types only:** Discarded because it binds `runtime-core` to one implementation strategy too early and weakens its role as a shared contract crate.
- **Cancellation as part of every sender and receiver contract:** Discarded because plain message flow should remain available without requiring every use case to carry cancellation semantics.
- **Separate incompatible cancelable contracts:** Discarded because it would break substitution and complicate `FlowOperation`, plugin dispatch, and higher-level runtime composition.
- **Cancelable endpoints without a shared cancel handler contract:** Discarded because it weakens external cancellation control and makes coordinated cancellation of both endpoints less explicit.
- **Multi-producer or multi-consumer semantics in v1:** Discarded because the first boundary should stay minimal and single-owner on both ends until real reuse pressure justifies broader semantics.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| One stable channel contract gives higher-level runtime crates a shared async message boundary. | A minimal contract may feel underspecified until more derived RFCs are accepted. |
| Cancel-aware specializations preserve plain sender and receiver contracts without forcing cancellation everywhere. | Four channel-facing contracts are more surface area than a single unified abstraction. |
| Compatibility between base and cancel-aware contracts preserves substitution in higher-level APIs. | Compatibility requirements reduce freedom in future implementations. |
| A trait-first boundary keeps `runtime-core` transport-agnostic. | Trait design mistakes are harder to unwind once multiple crates depend on them. |
| A standard `channel<T>()` factory shape gives runtime crates one standard way to create connected endpoints without exposing concrete types. | Public async trait design may still force careful bounds decisions even with modern stable Rust support. |
| A shared `CancelHandler` trait makes coordinated cancellation of connected cancel-aware endpoints explicit without locking `runtime-core` to one implementation. | A shared cancel control boundary must stay minimal or it will drift into lifecycle orchestration. |

## 5. Acceptance Criteria

- [ ] `runtime-core` documents `Sender<T>` as the canonical async publishing contract.
- [ ] `runtime-core` documents `Receiver<T>` as the canonical async consuming contract.
- [ ] `runtime-core` defines the base channel boundary as one publisher and one subscriber.
- [ ] `runtime-core` does not own a standard concrete `channel<T>()` factory implementation.
- [ ] The accepted standard `channel<T>()` boundary hides concrete endpoint types behind `impl Trait` or an equivalent opaque contract shape.
- [ ] `runtime-core` documents `CancelHandler` as the shared cancellation control trait boundary for cancel-aware connected endpoints.
- [ ] `runtime-core` documents `CancelableSender<T>` as a cancel-aware specialization compatible with `Sender<T>`.
- [ ] `runtime-core` documents `CancelableReceiver<T>` as a cancel-aware specialization compatible with `Receiver<T>`.
- [ ] `CancelableSender<T>` exposes `is_cancelled`.
- [ ] `CancelableReceiver<T>` exposes `is_cancelled`.
- [ ] A runtime-owned implementation crate defines a standard `cancelable_channel<T>()` factory that returns `(cancel handler, cancel-aware sender, cancel-aware receiver)`.
- [ ] The `CancelHandler` returned by the standard cancel-aware factory can cancel both connected endpoints.
- [ ] Cancel-aware endpoint cancellation state reflects the shared handler-controlled state.
- [ ] No accepted channel contract forces cancellation semantics into every sender or receiver.
- [ ] No accepted channel contract commits `runtime-core` to a concrete queue, buffer, executor, or transport implementation.
- [ ] No accepted channel contract introduces multi-producer or multi-consumer semantics in v1.
- [ ] Higher-level RFCs that depend on these channel contracts reference this RFC rather than redefining the boundary.

## 6. Open Questions

- Which exact async method names should `Sender<T>` and `Receiver<T>` expose in v1?
- Should `channel<T>()` use return-position `impl Trait` directly, or should an equivalent opaque endpoint strategy be used if trait bounds become too restrictive?
- Should `is_cancelled` be a synchronous query only, or should cancellation observation also include a separate notification contract?
- Should `CancelHandler` expose only cancellation intent, or should it also surface completion state in a later RFC?
- Should `CancelableSender<T>` and `CancelableReceiver<T>` be expressed as subtraits, wrappers, or another refinement mechanism?
- What minimum ordering expectation, if any, should be stated for single publisher and single subscriber implementations?
