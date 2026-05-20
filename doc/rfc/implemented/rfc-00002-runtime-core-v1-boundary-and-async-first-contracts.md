---
id: rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
type: rfc
code: "00002"
slug: runtime-core-v1-boundary-and-async-first-contracts
title: Runtime Core V1 Boundary and Async-First Contracts
description: Proposes the initial contract surface and ownership boundary for the runtime-core crate.
status: implemented
created: 2026-05-01
updated: 2026-05-02
authors: []
tags:
  - runtime
  - architecture
  - async
related:
  - spec-00002-runtime-core-crate
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
supersedes: []
superseded_by: null
aliases:
- "RFC 00002: Runtime Core V1 Boundary and Async-First Contracts"
---

# RFC 00002: Runtime Core V1 Boundary and Async-First Contracts

## 1. Problem

`runtime-core` is defined as the foundational crate for shared runtime contracts, but the crate does not exist yet and `spec-00002-runtime-core-crate` intentionally leaves the initial surface area open. Without a tighter v1 boundary, the first implementation will drift in one of two bad directions: either it becomes too thin to be useful, forcing duplication across runtime crates, or it becomes a catch-all for unrelated helpers and feature-specific policy.

The project also needs a stable async-first execution foundation before building more specialized runtime crates. Message flow, event fan-out, and text conversion are cross-cutting concerns that will appear in multiple runtime packages. If those concerns are not standardized early, higher-level crates will encode incompatible conventions that are harder to unwind later.

This RFC follows [[spec-00002-runtime-core-crate]], supports [[rfc-00001-thin-mcp-facade-over-runtime-libraries]].

## 2. Proposal

Create a new `runtime-core` crate and define v1 as a small async-first crate that owns only stable transport-agnostic primitives needed by multiple independent runtime crates.

Initial v1 surface:

- `Sender`
- `CancelableSender`
- `Receiver`
- `CancelableReceiver`
- `RuntimeError`
- `RuntimeResult`
- `Operation`
- `FlowOperation`
- `PluginSender`
- `PluginOperation`
- `PluginReceiver`
- `CancelHandler`
- `EventEmitter`
- `EventListener`
- `ControlEvent`
- `ObservabilityEvent`
- `Encoding`

The crate must be designed around async execution as the default model. Types in `runtime-core` must support asynchronous orchestration first and must not assume synchronous-first execution semantics that later need adapters layered on top.

This RFC intentionally defines crate-level ownership and primitive boundaries, not the full contract of every higher-level runtime component. Component-specific message contracts that depend on `runtime-core` primitives should be defined in separate RFCs.

The components accepted into this boundary are intentionally not fully defined here. `Sender`, `CancelableSender`, `Receiver`, `CancelableReceiver`, `RuntimeError`, `RuntimeResult`, `Operation`, `FlowOperation`, `PluginSender`, `PluginOperation`, `PluginReceiver`, `CancelHandler`, `EventEmitter`, `EventListener`, `ControlEvent`, `ObservabilityEvent`, and `Encoding` must each be refined by independent RFCs before implementation commits their detailed semantics.

Channel primitives belong in `runtime-core` when they define the smallest reusable message-passing boundary that multiple runtime crates can share without binding to a concrete transport, runtime feature, or frontend integration. The initial channel boundary for v1 is single-producer single-consumer.

`CancelableSender` belongs in `runtime-core` as a cancel-aware specialization of `Sender` that can be cancelled and can expose cancellation state through `is_cancelled`. It must remain compatible with `Sender` rather than forming an unrelated parallel contract.

`CancelableReceiver` belongs in `runtime-core` as a cancel-aware specialization of `Receiver` that can notify the consumer that cancellation has been observed and can expose cancellation state through `is_cancelled`. It must remain compatible with `Receiver` rather than forming an unrelated parallel contract.

`RuntimeError` belongs in `runtime-core` as the root execution error boundary shared by runtime-core primitives.

`RuntimeResult` belongs in `runtime-core` as the canonical result alias over `RuntimeError`.

`Operation` belongs in `runtime-core` as the minimal async-first execution primitive for ordinary asynchronous computation. It receives one input value and resolves to a result value.

`FlowOperation` belongs in `runtime-core` as the minimal async-first dataflow primitive. It receives an input boundary and an output boundary, where the output boundary is a `Sender`, and resolves to `Result<(), Error>`.

`Operation` must not imply scheduling, backpressure, supervision, retries, cancellation, ordering guarantees, lifecycle management, or any other control semantics beyond its input-result contract.

`FlowOperation` must not imply scheduling, backpressure, supervision, retries, cancellation, ordering guarantees, lifecycle management, or any other control semantics beyond its input-output-flow contract.

When `FlowOperation` receives a plain input value and writes to a `Sender`, it is `1:N`. When its input is a `Receiver`, it becomes `N:N`. The input side remains open to either plain values or receiver-driven flows.

`PluginSender<T>` belongs in `runtime-core` as a type alias for a cancel-aware sender used by plugin-oriented flow execution.

`PluginOperation` belongs in `runtime-core` as a named specialization of `FlowOperation` for plugin execution. It is `1:N` and writes through a cancel-aware plugin sender boundary.

`PluginReceiver` belongs in `runtime-core` as the canonical plugin-oriented receiver boundary for values emitted by plugin execution.

`CancelHandler` belongs in `runtime-core` as the minimal cancellation control boundary for prepared runtime flows. It is limited to sending cancellation intent and may be reused by channel and plugin execution primitives.

Event primitives belong in `runtime-core` when they provide transport-agnostic fan-out over stable runtime events and listener registration without embedding feature-specific workflow rules. They must not encode domain event taxonomies, transport adapters, or product-specific subscription policy.

Common event types belong in `runtime-core` only when they provide a minimal shared vocabulary for cross-runtime coordination or observability. They must be explicitly named and narrowly scoped rather than collected into a generic bucket of common types.

Error modeling in `runtime-core` must use `thiserror` as the library boundary for typed library errors.

Dependency boundary for v1:

- `runtime-core` depends on Rust `std`
- `runtime-core` uses `thiserror` for typed error modeling
- no other third-party libraries belong to the accepted v1 dependency boundary unless approved by a later RFC

For textual data, `runtime-core` must enforce UTF-8 as the canonical encoding. Text primitives in the crate must assume UTF-8, validate UTF-8 at boundaries where bytes become text, and avoid introducing alternative text encodings into the shared runtime foundation.

Inclusion rule:

A type belongs in `runtime-core` only if it satisfies at least one of these conditions:

1. It is required by multiple independent runtime crates.
2. It defines a cross-cutting execution concern in async runtime message flow, cancellation observation, operation flow, event flow, or text conversion.
3. It is a stable primitive needed to keep runtime crates transport-agnostic.
4. It is a stable operation, flow-operation, plugin-extension, event-flow, cancellation, error, or text-boundary primitive required across async runtime crates without feature-specific policy.

Exclusion rule:

A type does not belong in `runtime-core` if it encodes a VECTOR feature, repository policy, document schema, protocol adapter, command behavior, or workflow-specific decision.

Structural rule:

- do not add `utils.rs`
- do not add `helpers.rs`
- do not add `common.rs`
- do not add `misc.rs`

Those files concentrate unrelated behavior and weaken crate boundaries over time.

Expected ownership of the proposed v1 types:

- `Sender` and `Receiver` define the minimal transport-agnostic async message channel boundary for one publisher and one subscriber.
- `CancelableSender` defines a cancel-aware sender boundary that remains compatible with `Sender` while reporting cancellation state through `is_cancelled`.
- `CancelableReceiver` defines a cancel-aware receiver boundary that remains compatible with `Receiver` while reporting observed cancellation through notification semantics and `is_cancelled`.
- `RuntimeError` defines the canonical typed execution error boundary shared by the crate.
- `RuntimeResult` defines the canonical result alias over `RuntimeError`.
- `Operation` defines the minimal async input-result execution primitive for ordinary asynchronous computation.
- `FlowOperation` defines the minimal async dataflow execution primitive. It receives an input boundary and writes to a `Sender` output boundary.
- `PluginSender<T>` defines the canonical plugin-oriented sender alias over a cancel-aware sender boundary.
- `PluginOperation` defines a named plugin-oriented specialization of `FlowOperation` with `1:N` cardinality and a cancel-aware sender output.
- `PluginReceiver` defines the canonical plugin-oriented receiver boundary for outputs produced by plugin execution.
- `CancelHandler` defines the minimal boundary for sending cancellation intent to a prepared runtime flow, including cancel-aware channels and plugin execution.
- `EventEmitter` and `EventListener` define a higher-level transport-agnostic event fan-out boundary built on stable runtime event flow rather than feature-specific subscription rules.
- `ControlEvent` defines a minimal shared event vocabulary for runtime control flows such as cancellation.
- `ObservabilityEvent` defines a minimal shared event vocabulary for runtime observability flows.
- `Encoding` defines the canonical UTF-8 text boundary for validated conversion between `String` and UTF-8 bytes in shared runtime flows.

Non-goals for v1:

- feature-specific services
- repository scanning logic
- document schema definitions
- CLI command behavior
- MCP protocol types
- VS Code extension integration
- workflow policy
- transport adapters
- concrete repository file workflows
- feature-specific codecs or serialization formats
- alternative text encoding support in shared runtime primitives
- multi-producer or multi-consumer semantics unless accepted in a later RFC
- domain-specific event routing policy
- control semantics embedded into `Operation`
- control semantics embedded into `FlowOperation`
- plugin identity concerns
- plugin registry concerns
- plugin discovery concerns
- plugin loading concerns
- plugin lifecycle concerns
- plugin versioning concerns
- plugin dependency resolution concerns
- plugin execution policy concerns
- plugin lookup concerns
- plugin introspection concerns
- plugin dispatch or orchestration concerns
- retry concerns
- scheduling concerns
- completion or lifecycle supervision embedded into `CancelHandler`
- cancellation as an obligation of every `Sender`
- cancellation as an obligation of every `Receiver`
- opaque catch-all error boundaries as the primary crate contract
- string-only error contracts
- a generic bucket of common runtime types
- feature-specific event taxonomies
- generic helper buckets with mixed responsibilities
- additional third-party dependencies in v1 without explicit RFC approval

## 3. Alternatives Considered

- **Channel-only core with no operation, flow operation, cancellation-aware receiver, event, or encoding primitive:** Discarded because it keeps the crate mechanically small but pushes shared execution, dataflow, cancellation observation, event fan-out, and UTF-8 boundary semantics into higher-level crates too early, where incompatible abstractions would likely emerge.
- **Channel-only core with no operation, flow operation, cancellation-aware sender, cancellation-aware receiver, event, or encoding primitive:** Discarded because it keeps the crate mechanically small but pushes shared execution, dataflow, cancellation observation, event fan-out, and UTF-8 boundary semantics into higher-level crates too early, where incompatible abstractions would likely emerge.
- **Cancellation as an obligation of every `Sender`:** Discarded because it enlarges the base sender contract and forces cancellation semantics onto cases that only need plain message publication.
- **Cancellation as an obligation of every `Receiver`:** Discarded because it enlarges the base receiver contract and forces cancellation semantics onto cases that only need plain message consumption.
- **Per-component unrelated error types with no shared root error:** Discarded because it fragments crate-level error handling even though runtime-core primitives need one consistent error boundary.
- **Opaque application-style error handling as the primary library contract:** Discarded because a foundational runtime crate benefits from typed error boundaries rather than unstructured dynamic error propagation.
- **One operation abstraction for both ordinary async computation and async dataflow:** Discarded because it collapses two materially different contracts into one name even though dataflow writes to an output boundary while ordinary computation returns a value.
- **Plugin-oriented operation support outside `runtime-core`:** Discarded because other runtime crates need one shared operation contract and cancel-aware sender boundary for plugin execution.
- **Full plugin system in `runtime-core`:** Discarded because identity, registry, discovery, loading, lifecycle, versioning, dispatch, and dependency policy are higher-level concerns that would overload the core boundary too early.
- **Unstructured common event types bucket:** Discarded because it weakens crate ownership and invites unrelated enums and structs under the label of common use cases.
- **Broad shared crate with generic helpers and early feature abstractions:** Discarded because it encourages `runtime-core` to become a dumping ground for convenience code, creates unclear ownership, and makes later extraction harder.
- **Synchronous-first core with async adapters later:** Discarded because async orchestration is a primary runtime concern for the project, and synchronous-first contracts would bias APIs in the wrong direction from the start.

## 4. Tradeoffs

| Pro                                                                                                                                          | Con                                                                                                           |
|----------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------|
| `CancelableSender` keeps sender-side cancellation explicit without forcing every sender to carry the same semantics.                        | A cancel-aware sender specialization increases the surface area and requires boundary discipline to avoid variants. |
| `CancelableReceiver` keeps cancellation observation explicit without forcing every receiver to carry the same semantics.                    | A cancel-aware receiver specialization increases the surface area and requires boundary discipline to avoid variants. |
| `RuntimeError` and `RuntimeResult` give the crate one typed execution error boundary from the start.                                        | If the error surface grows without discipline, `RuntimeError` can become a generic bucket enum.              |
| Shared runtime crates get one stable channel and event-flow layer before feature-specific orchestration appears.                             | Event fan-out adds semantic weight earlier than a strictly minimal core would.                                |
| `Operation` preserves a simple async computation contract instead of duplicating what plain functions already express.                       | If kept too thin, `Operation` can collapse into a naming layer over ordinary async functions.                 |
| `FlowOperation` gives dataflow work a dedicated async contract instead of overloading `Operation` with sender-based output semantics.       | `FlowOperation` adds another core primitive that must be kept narrowly scoped to avoid conceptual overlap.    |
| `PluginOperation` gives runtime crates one shared plugin-oriented execution primitive using only `runtime-core`.                            | Higher-level crates must still define plugin identity, selection, and orchestration explicitly outside core.  |
| `ControlEvent` and `ObservabilityEvent` give runtime crates a minimal shared event vocabulary from the start.                               | If expanded too early, shared event enums can become a dumping ground for feature-specific event cases.       |
| `Encoding` makes the UTF-8 boundary explicit instead of letting each crate improvise string and byte conversion rules.                      | If scoped poorly, `Encoding` can drift into a generic codec abstraction with weak runtime ownership.          |
| Enforcing UTF-8 gives all runtime crates one predictable text boundary.                                                                      | Integrations with non-UTF-8 external systems will need explicit adapters outside `runtime-core`.              |
| Async-first contracts reduce rework when higher-level runtime crates begin orchestrating concurrent operations.                              | Some simple synchronous use cases may need lightweight adapters instead of direct synchronous APIs.           |
| A strict inclusion rule reduces the risk that `runtime-core` becomes a feature bucket.                                                       | Teams must create new crates sooner instead of hiding feature logic inside the core crate.                    |
| Banning generic helper files preserves ownership clarity at the file and module level.                                                       | The crate may initially feel more fragmented because concerns are split into narrowly scoped modules.         |

## 5. Acceptance Criteria

- [ ] `runtime-core` is created as a new crate rather than described as a future placeholder boundary.
- [ ] `runtime-core` defines a v1 module layout that maps directly to the accepted surface instead of using generic helper buckets.
- [ ] `runtime-core` exposes `Sender` and `Receiver` as the canonical async one-publisher one-subscriber channel primitives.
- [ ] `runtime-core` exposes `CancelableSender` as a cancel-aware specialization compatible with `Sender`.
- [ ] `runtime-core` exposes `CancelableReceiver` as a cancel-aware specialization compatible with `Receiver`.
- [ ] `runtime-core` exposes `RuntimeError` as the canonical typed execution error boundary.
- [ ] `runtime-core` exposes `RuntimeResult` as the canonical result alias over `RuntimeError`.
- [ ] `runtime-core` uses `thiserror` for typed library error modeling.
- [ ] `runtime-core` v1 dependency boundary is limited to Rust `std` and `thiserror`.
- [ ] `runtime-core` exposes `Operation` as the canonical async input-result execution primitive for ordinary asynchronous computation.
- [ ] `runtime-core` exposes `FlowOperation` as the canonical async dataflow execution primitive with `Sender` as its output boundary.
- [ ] `FlowOperation` accepts either a plain input value or a `Receiver`-driven input boundary.
- [ ] `Operation` encodes no control semantics beyond its input-result contract.
- [ ] `FlowOperation` encodes no control semantics beyond its input-output-flow contract.
- [ ] `runtime-core` exposes `PluginSender<T>` as the canonical plugin-oriented alias over a cancel-aware sender boundary.
- [ ] `runtime-core` exposes `PluginOperation` as a named specialization of `FlowOperation`.
- [ ] `PluginOperation` is `1:N` and writes through a cancel-aware plugin sender boundary.
- [ ] `runtime-core` exposes `PluginReceiver` as the canonical plugin-oriented output receiver boundary.
- [ ] `runtime-core` exposes `CancelHandler` as the minimal boundary for sending cancellation intent to a prepared runtime flow.
- [ ] `runtime-core` exposes `EventEmitter` and `EventListener` as transport-agnostic event-flow primitives with no feature-specific routing policy.
- [ ] `runtime-core` exposes `ControlEvent` as a minimal shared control event enum that includes cancellation-oriented control flow.
- [ ] `runtime-core` exposes `ObservabilityEvent` as a minimal shared observability event enum.
- [ ] `runtime-core` exposes `Encoding` as the canonical shared boundary for validated conversion between `String` and UTF-8 bytes.
- [ ] `runtime-core` enforces UTF-8 as the canonical text encoding for shared runtime text primitives.
- [ ] `runtime-core` contains no `utils.rs`, `helpers.rs`, `common.rs`, or `misc.rs`.
- [ ] No accepted v1 type encodes VECTOR feature policy, document schema policy, protocol adapter behavior, or workflow-specific decisions.
- [ ] The crate documentation states that async-first design is a foundational constraint of `runtime-core`.
- [ ] Higher-level component contracts that depend on `runtime-core` are documented outside this RFC.

## 6. Open Questions

- Should `Sender` and `Receiver` be trait-based contracts, concrete channel types, or thin wrappers over a selected async primitive?
- Should `CancelableSender` extend `Sender`, wrap `Sender`, or stand as a sibling contract with shared semantics but separate ownership?
- Should `CancelableReceiver` extend `Receiver`, wrap `Receiver`, or stand as a sibling contract with shared semantics but separate ownership?
- Should `RuntimeError` remain one flat enum in v1, or should some areas define typed sub-errors that are wrapped into the root error from the start?
- Should `Operation` be trait-based, function-based, or a thin wrapper over callable async handlers?
- Should `FlowOperation` be trait-based, function-based, or a thin wrapper over callable async handlers that write to a `Sender`?
- Should `PluginOperation` extend `FlowOperation`, wrap `FlowOperation`, or stand as a named sibling contract with equivalent flow semantics?
- Should `PluginReceiver` be a type alias over `Receiver`, a dedicated receiver type, or a cancel-aware specialization?
- Should `CancelHandler` be limited to cancellation signaling only, or should it later expose completion state outside v1?
- Should `EventEmitter` own listener registration directly or publish into a separate registration boundary owned by another primitive?
- Should `EventListener` be modeled as a passive receiver, an async stream-like primitive, or a narrower callback-oriented contract?
- How small should `ControlEvent` remain in v1 beyond a cancellation case such as `Cancel`?
- How small should `ObservabilityEvent` remain in v1 without becoming a telemetry schema bucket?
- Should `Encoding` be a stateless utility struct, a namespace-like type, or an instance-based boundary with configurable behavior kept internal?
