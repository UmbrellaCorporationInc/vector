---
id: rfc-00005-runtime-core-operation-and-event-flow-primitives
type: rfc
code: "00005"
slug: runtime-core-operation-and-event-flow-primitives
title: Runtime Core Operation and Event Flow Primitives
description: Defines the v1 contracts for Operation, FlowOperation, EventEmitter, and EventListener in runtime-core.
status: implemented
created: 2026-05-03
updated: 2026-05-03
authors: []
tags:
  - runtime
  - async
  - operation
  - events
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
supersedes: []
superseded_by: null
aliases:
  - "RFC 00005: Runtime Core Operation and Event Flow Primitives"
---

# RFC 00005: Runtime Core Operation and Event Flow Primitives

## 1. Problem

`rfc-00002-runtime-core-v1-boundary-and-async-first-contracts` accepts `Operation`, `FlowOperation`, `EventEmitter`, and `EventListener` into the `runtime-core` boundary, but it intentionally leaves their concrete contracts undefined.

Without a dedicated RFC for these primitives, the first implementation will likely drift in one of two directions:

- async work remains expressed only as ad hoc function shapes, which weakens shared composition across runtime crates
- event fan-out grows around a transport-specific or product-specific subscription model instead of a stable core contract

The project needs one minimal execution and event-flow boundary before plugin execution, plugin dispatch, and higher-level orchestration can be implemented coherently against `runtime-core`.

This RFC follows [[spec-00002-runtime-core-crate]], refines [[rfc-00002-runtime-core-v1-boundary-and-async-first-contracts]], and depends on the channel contracts defined in [[rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations]].

## 2. Proposal

Define four contract families in `runtime-core` v1:

- `Operation<Input, Output>`
- `FlowOperation<Input, Output>`
- `EventEmitter<Event>`
- `EventListener<Event>`

Execution boundary:

- `Operation<Input, Output>` is the canonical async input-result contract for ordinary asynchronous computation
- `Operation<Input, Output>` receives one input value and resolves to one output value wrapped in `RuntimeResult<Output>`
- `Operation<Input, Output>` is intended to cover `1:1` and `N:1` execution shapes only
- when `Operation<Input, Output>` receives a plain input value it is `1:1`
- when `Operation<Input, Output>` receives a `Receiver<Input>`-driven input boundary it is `N:1`
- `Operation<Input, Output>` must not encode scheduling, retries, cancellation, backpressure, supervision, ordering guarantees, lifecycle ownership, or transport policy

Flow boundary:

- `FlowOperation<Input, Output>` is the canonical async input-output-flow contract for asynchronous dataflow
- `FlowOperation<Input, Output>` receives an input boundary and an output boundary
- the output boundary is `Sender<Output>`
- the input boundary may be either a plain input value or a `Receiver<Input>`-driven input flow
- `FlowOperation<Input, Output>` is intended to cover `1:N` and `N:N` execution shapes only
- when `FlowOperation<Input, Output>` receives a plain input value and writes through `Sender<Output>` it is `1:N`
- when `FlowOperation<Input, Output>` receives a `Receiver<Input>`-driven input flow and writes through `Sender<Output>` it is `N:N`
- `FlowOperation<Input, Output>` resolves to `RuntimeResult<()>`
- `FlowOperation<Input, Output>` must not encode scheduling, retries, cancellation, backpressure, supervision, ordering guarantees, lifecycle ownership, or transport policy

Event boundary:

- `EventEmitter<Event>` is the canonical transport-agnostic event-emission contract for publishing shared runtime events into a fan-out boundary
- `EventEmitter<Event>` must remain compatible with the base `Sender<Event>` boundary
- `EventEmitter<Event>` must allow higher-level crates to publish one event without knowing the concrete listener implementation
- `EventEmitter<Event>` must expose listener registration as part of the core contract boundary
- `EventEmitter<Event>` is designed for registered-listener event emission rather than high-guarantee delivery semantics
- `EventEmitter<Event>` in `runtime-core` defines contract only and does not own a concrete listener registry or broadcast backend
- `EventListener<Event>` is the canonical transport-agnostic event-observation contract for consuming events emitted from an `EventEmitter<Event>` boundary
- `EventListener<Event>` is a dedicated event-listener concept rather than a semantic refinement over `Receiver<Event>`
- `EventListener<Event>` must allow higher-level crates to observe emitted events without binding `runtime-core` to receiver semantics, callback registration frameworks, streams, or a concrete transport
- `EventEmitter<Event>` does not wait for a listener-side processing acknowledgment after dispatching an event
- listener-side failure while consuming or handling an already-dispatched event is outside the responsibility of `EventEmitter<Event>`
- `runtime-channel` may provide the standard concrete event fan-out implementation for these contracts, including a Tokio-backed implementation when Tokio remains the selected runtime backend

Ownership boundary:

- this RFC defines contracts only
- this RFC does not define a concrete executor, queue, buffer, stream type, callback registry, or runtime backend
- this RFC does not define a concrete event broadcaster or listener registry in `runtime-core`
- this RFC does not define concrete event taxonomies such as `ControlEvent` or `ObservabilityEvent`
- this RFC does not define plugin-specific execution policy
- this RFC does not define end-to-end delivery guarantees, listener processing acknowledgments, or durable event delivery
- this RFC does not define transport adapters or frontend subscription policy

Compatibility rule:

- `FlowOperation<Input, Output>` must be expressible in terms of `Sender<Output>` from `runtime-core`
- `EventEmitter<Event>` must remain usable anywhere a `Sender<Event>` is accepted
- event contracts must remain transport-agnostic and must not require `runtime-channel` as an implementation detail of the contract shape
- the standard concrete event fan-out implementation may live in `runtime-channel` without changing the accepted `runtime-core` contract boundary
- the accepted boundary must support multiple future implementations without requiring changes to higher-level runtime crates

Design rule:

- `Operation<Input, Output>` and `FlowOperation<Input, Output>` should be trait-based contracts unless a later RFC proves that function-based wrappers are the only viable stable boundary
- `EventEmitter<Event>` and `EventListener<Event>` should stay minimal enough to avoid committing `runtime-core` to a specific fan-out model too early
- `EventEmitter<Event>` should not be implemented in `runtime-core` as a concrete broadcaster that owns a collection of `Sender<Event>` instances
- `EventEmitter<Event>` should inherit sender semantics directly rather than defining an unrelated publication primitive
- `EventEmitter<Event>` should remain a lightweight emission primitive rather than evolving into a guaranteed-delivery or listener-supervision mechanism
- the operation and event contracts should stay small enough that plugin, control, and observability RFCs can specialize them later instead of redefining them

Implementation strategy:

- v1 should target modern stable Rust async features
- `Operation<Input, Output>` and `FlowOperation<Input, Output>` may use `async fn` in trait contracts
- `EventEmitter<Event>` and `EventListener<Event>` may use `async fn` in trait contracts if that remains consistent with the accepted transport-agnostic boundary
- the first implementation should prefer method names that reflect intent directly rather than generic verbs that blur execution and event semantics
- the standard event implementation should be provided outside `runtime-core`, in `runtime-channel`, if the project wants a Tokio-backed broadcaster over channel primitives
- if public async trait bounds require refinement during implementation, exact signatures may be tightened without changing the accepted boundary of async input-result execution, async sender-backed dataflow, async event publication, and async event observation

Staging note:

- `Operation<Input, Output>` should be implemented before `FlowOperation<Input, Output>` because flow execution depends directly on the accepted sender boundary
- `EventEmitter<Event>` and `EventListener<Event>` should be implemented as minimal core contracts before `ControlEvent` and `ObservabilityEvent` are accepted in later RFCs
- plugin-oriented contracts should depend on these primitives rather than redefining operation or event semantics privately

Proposed contract shape:

The following shape is proposed as the v1 direction for the `runtime-core` contracts. It is illustrative of the intended boundary and may be tightened during implementation without changing the accepted ownership model of this RFC.

```rust
use runtime_core::{Receiver, RuntimeResult, Sender};

pub trait Operation<Input, Output>: Send {
    fn run(&self, input: Input) -> impl Future<Output = RuntimeResult<Output>> + Send;
}

pub trait ReceiverOperation<Input, Output>: Send {
    fn run(
        &self,
        input: &mut impl Receiver<Input>,
    ) -> impl Future<Output = RuntimeResult<Output>> + Send;
}

pub trait FlowOperation<Input, Output>: Send {
    fn run(
        &self,
        input: Input,
        output: &mut impl Sender<Output>,
    ) -> impl Future<Output = RuntimeResult<()>> + Send;
}

pub trait ReceiverFlowOperation<Input, Output>: Send {
    fn run(
        &self,
        input: &mut impl Receiver<Input>,
        output: &mut impl Sender<Output>,
    ) -> impl Future<Output = RuntimeResult<()>> + Send;
}

pub trait EventEmitter<Event>: Sender<Event> {
    fn register_listener(
        &mut self,
        listener: impl EventListener<Event> + 'static,
    ) -> RuntimeResult<()>;

    /// Dispatches an event through sender semantics without waiting for listener-side processing.
    fn emit(&mut self, event: Event) -> impl Future<Output = RuntimeResult<()>> + Send {
        self.send(event)
    }
}

pub trait EventListener<Event>: Send {
    /// Handles a dispatched event after emitter-side publication has already succeeded.
    fn on_event(&mut self, event: Event) -> impl Future<Output = RuntimeResult<()>> + Send;
}
```

Shape intent:

- `Operation<Input, Output>` expresses the `1:1` contract directly through `run(input) -> RuntimeResult<Output>`
- `ReceiverOperation<Input, Output>` expresses the `N:1` contract directly through a receiver-driven input boundary
- `FlowOperation<Input, Output>` expresses the `1:N` contract directly through `run(input, output)`
- `ReceiverFlowOperation<Input, Output>` expresses the `N:N` contract directly through receiver-driven input and sender-backed output
- `EventEmitter<Event>` is a sender-compatible event publication boundary in `runtime-core`
- `EventEmitter<Event>` owns the listener-registration boundary in the contract, but not the concrete registry implementation
- `emit` may remain a default alias over `send` so implementors only need to satisfy the sender contract plus listener registration
- `EventListener<Event>` is a dedicated event callback-style contract rather than a receiver specialization
- emitter-side success means the event was accepted for dispatch, not that every listener finished handling it successfully
- listener-side processing failure after dispatch does not retroactively fail emitter-side publication
- listener association, broadcast fan-out, and registry ownership remain outside `runtime-core`
- the contract should not expose heap allocation or boxing requirements to the caller
- a concrete broadcaster in `runtime-channel` may internally box, pin, clone, wrap, or otherwise store listener state as needed by its implementation
- if Tokio remains the selected runtime backend, the standard broadcaster may be implemented in `runtime-channel` on top of Tokio-backed channel primitives

## 3. Alternatives Considered

- **Plain async functions with no shared operation trait:** Discarded because it leaves higher-level crates without one named boundary for async computation and weakens contract reuse across runtime packages.
- **A single operation abstraction for both input-result execution and sender-backed flow execution:** Discarded because `Operation` and `FlowOperation` have materially different contracts and different output boundaries.
- **Allowing `Operation<Input, Output>` to cover `1:N` or `N:N` shapes:** Discarded because multi-output execution belongs to sender-backed flow semantics rather than result-returning operation semantics.
- **Allowing `FlowOperation<Input, Output>` to cover `1:1` or `N:1` shapes as a primary contract:** Discarded because result-returning execution belongs to `Operation<Input, Output>` and should not be blurred with sender-backed fan-out semantics.
- **Event fan-out defined as callback registration in `runtime-core`:** Discarded because callback-oriented registration commits the crate to one listener ownership model too early.
- **Event fan-out defined as a stream-only boundary:** Discarded because it would bias `runtime-core` toward one async observation shape before the project validates that stream semantics are the right universal contract.
- **An `EventEmitter<Event>` contract unrelated to `Sender<Event>`:** Discarded because event emission is still sender semantics and higher-level crates should be able to substitute an emitter anywhere a sender is accepted.
- **An `EventListener<Event>` contract modeled as a receiver specialization:** Discarded because listener behavior is tied to emitter-driven callback delivery rather than general-purpose receiver consumption.
- **Emitter-side acknowledgment coupled to listener-side handling completion:** Discarded because this primitive is intended for lightweight event emission to registered listeners rather than high-guarantee end-to-end delivery.
- **A concrete broadcaster in `runtime-core` that owns multiple `Sender<Event>` listeners directly:** Discarded because listener registry ownership, broadcast failure policy, and runtime-backed fan-out are implementation concerns that belong in `runtime-channel`.
- **Concrete event bus types in `runtime-core`:** Discarded because concrete transport and fan-out implementation belong in a dedicated runtime-owned implementation crate rather than the shared contract crate.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| A named `Operation<Input, Output>` contract gives runtime crates one shared boundary for async computation. | If kept too thin, `Operation<Input, Output>` can collapse into little more than a naming layer over plain async callables. |
| Separating `Operation<Input, Output>` from `FlowOperation<Input, Output>` preserves a clear distinction between return-value execution and sender-backed flow execution. | Two operation contracts increase surface area and require discipline to avoid overlap. |
| Sender-backed `FlowOperation<Input, Output>` keeps dataflow aligned with the channel boundary already accepted in `runtime-core`. | The sender dependency constrains future flow shapes unless later RFCs extend the model carefully. |
| Sender-compatible `EventEmitter<Event>` and dedicated `EventListener<Event>` contracts let runtime crates share event vocabulary without choosing one transport implementation. | Default aliasing and listener registration add trait-shape complexity that must remain compatible with stable Rust trait limitations. |
| Fire-and-forget listener dispatch keeps the primitive lightweight and aligned with ordinary event emission. | Callers that need end-to-end delivery guarantees or listener processing confirmation must use a different primitive. |
| Keeping event contracts transport-agnostic reduces the chance that observability or control wiring hardcodes one backend into `runtime-core`. | Delaying the concrete fan-out model pushes some implementation decisions into later RFCs. |

## 5. Acceptance Criteria

- [ ] `runtime-core` documents `Operation<Input, Output>` as the canonical async input-result execution contract.
- [ ] `Operation<Input, Output>` accepts one input value and resolves to `RuntimeResult<Output>`.
- [ ] `Operation<Input, Output>` is explicitly limited to `1:1` and `N:1` execution shapes.
- [ ] `Operation<Input, Output>` is `1:1` for plain input values and `N:1` for `Receiver<Input>`-driven input.
- [ ] `Operation<Input, Output>` introduces no scheduling, retry, cancellation, supervision, backpressure, ordering, transport, or lifecycle policy.
- [ ] `runtime-core` documents `FlowOperation<Input, Output>` as the canonical async sender-backed dataflow contract.
- [ ] `FlowOperation<Input, Output>` uses `Sender<Output>` as its output boundary.
- [ ] `FlowOperation<Input, Output>` resolves to `RuntimeResult<()>`.
- [ ] `FlowOperation<Input, Output>` supports either plain input values or `Receiver<Input>`-driven input flow.
- [ ] `FlowOperation<Input, Output>` is explicitly limited to `1:N` and `N:N` execution shapes.
- [ ] `FlowOperation<Input, Output>` is `1:N` for plain input values and `N:N` for `Receiver<Input>`-driven input flow.
- [ ] `FlowOperation<Input, Output>` introduces no scheduling, retry, cancellation, supervision, backpressure, ordering, transport, or lifecycle policy.
- [ ] `runtime-core` documents `EventEmitter<Event>` as the canonical transport-agnostic event publication contract.
- [ ] `runtime-core` documents `EventListener<Event>` as the canonical transport-agnostic event observation contract.
- [ ] `EventEmitter<Event>` remains compatible with `Sender<Event>`.
- [ ] `EventEmitter<Event>` may provide `emit` as a default alias over `send` so implementors do not need separate publication logic.
- [ ] `EventEmitter<Event>` exposes listener registration as part of the accepted core contract boundary.
- [ ] `EventListener<Event>` is a dedicated contract and is not modeled as a `Receiver<Event>` specialization.
- [ ] `EventEmitter<Event>` does not require listener-side processing acknowledgment to complete emitter-side publication.
- [ ] Listener-side processing failure after dispatch is outside the responsibility of `EventEmitter<Event>`.
- [ ] `runtime-core` keeps `EventEmitter<Event>` and `EventListener<Event>` at the contract level and does not own a concrete broadcaster implementation.
- [ ] A runtime-owned implementation crate may provide the standard concrete event fan-out implementation for these contracts.
- [ ] If Tokio remains the selected runtime backend, `runtime-channel` may provide the standard Tokio-backed event fan-out implementation.
- [ ] This RFC includes a proposed v1 Rust contract shape for `Operation`, `FlowOperation`, `EventEmitter`, and `EventListener`.
- [ ] Event contracts introduce no product-specific routing policy, transport adapter behavior, callback registry policy, or frontend subscription semantics.
- [ ] No accepted contract requires a concrete executor, queue, event bus, stream type, or runtime backend in `runtime-core`.
- [ ] Higher-level RFCs for plugin execution, control events, or observability events reference this RFC rather than redefining operation or event-flow semantics.

## 6. Open Questions

- Should `Operation<Input, Output>` and `FlowOperation<Input, Output>` keep `run` as the shared method name in v1, or should operation and flow execution use distinct verbs?
- Should `1:1` and `N:1` operation shapes remain separate traits, or should they be unified through a different input-boundary abstraction?
- Should `1:N` and `N:N` flow shapes remain separate traits, or should they be unified through a different input-boundary abstraction?
- Should `EventEmitter<Event>` own listener registration directly, or should later RFCs split registration into a separate builder or registry boundary while preserving compatibility?
- Should `EventListener<Event>` remain callback-oriented in v1, or should a later RFC add a separate pull-based event observation boundary alongside it?
- Should registration failure and dispatch failure stay the only emitter-visible error cases in v1, or is a narrower error boundary needed?
- What minimum ordering expectation, if any, should be stated for event delivery once a concrete implementation crate exists?
