---
id: rfc-00008-runtime-channel-plugin-dispatcher-builder
type: rfc
code: "00008"
slug: runtime-channel-plugin-dispatcher-builder
title: Runtime Channel Plugin Dispatcher Builder
description: Defines the builder contract for preparing plugin operation execution in runtime-channel with input, observability listeners, cancellation, and connected plugin output channels.
status: implemented
created: 2026-05-03
updated: 2026-05-03
authors: []
tags:
  - runtime
  - channel
  - plugin
  - dispatch
related:
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
  - rfc-00005-runtime-core-operation-and-event-flow-primitives
  - rfc-00006-runtime-core-control-observability-and-encoding-primitives
  - rfc-00007-runtime-core-plugin-primitives
supersedes: []
superseded_by: null
aliases:
  - "RFC 00008: Runtime Channel Plugin Dispatcher Builder"
---

# RFC 00008: Runtime Channel Plugin Dispatcher Builder

## 1. Problem

`runtime-core` now owns only plugin-oriented execution primitives: `PluginSender<T>`, `PluginReceiver<T>`, and `PluginOperation`. That keeps the core boundary small, but it leaves one concrete gap in the standard runtime layer: there is no accepted builder for preparing plugin execution using the Tokio-backed channel implementation in `runtime-channel`.

Without a standard dispatcher builder, each higher-level crate will need to reinvent the same wiring:

- create one connected cancel-aware plugin output channel
- expose the connected `PluginReceiver<T>` to the caller
- pass the connected `PluginSender<T>` into the selected `PluginOperation`
- accept one input value for the operation
- optionally attach observability listeners
- return a `CancelHandler` for the prepared execution flow

If every crate does that independently, plugin execution preparation will drift in cancellation behavior, event attachment shape, and channel ownership boundaries.

The project needs one runtime-owned builder in `runtime-channel` that prepares plugin execution against the accepted `runtime-core` contracts without moving orchestration policy back into `runtime-core`.

## 2. Proposal

Define `PluginDispatcher` in `runtime-channel` as a concrete builder for preparing one plugin operation execution.

Accepted builder responsibilities:

- accept one `PluginOperation`
- accept one input value
- allow registration of observability listeners before build
- create one connected cancel-aware plugin output channel using the standard runtime-owned channel backend
- return `(CancelHandler, PluginReceiver<T>)` from `build`

Accepted non-responsibilities:

- no plugin identity contract
- no plugin lookup registry
- no plugin discovery
- no plugin loading
- no retry policy
- no scheduling policy beyond preparing the execution flow
- no lifecycle supervision
- no dependency resolution

### Builder shape

The builder is concrete and runtime-owned.

Illustrative shape:

```rust
pub struct PluginDispatcher<Op, Input, Output>
where
    Op: PluginOperation<Input = Input, Output = Output>,
{
    /* private fields */
}

impl<Op, Input, Output> PluginDispatcher<Op, Input, Output>
where
    Op: PluginOperation<Input = Input, Output = Output>,
{
    pub fn new(operation: Op) -> Self { ... }

    pub fn input(self, input: Input) -> Self { ... }

    pub fn observe(
        self,
        listener: impl EventListener<ObservabilityEvent<Output>> + 'static,
    ) -> Self { ... }

    pub fn build(self) -> RuntimeResult<(impl CancelHandler, impl PluginReceiver<Output>)> { ... }
}
```

The exact generic shape may be tightened during implementation, but the accepted boundary is:

- operation selected at builder construction time
- input supplied before build
- observability listeners attached before build
- build returns cancellation and connected output receiver

### Execution wiring

`build` prepares one connected execution pair:

- the builder creates one `cancelable_channel<Output>()`
- the returned `PluginReceiver<Output>` is the receiver endpoint of that connected cancel-aware channel
- the `PluginSender<Output>` passed into the selected `PluginOperation` is backed by the sender endpoint of the same connected cancel-aware channel
- the returned `CancelHandler` is connected to both output channel endpoints
- the builder is responsible for attaching any registered observability listeners to its internal event emitter before execution starts

This RFC accepts preparation only. The runtime may start execution immediately during `build` or may return a prepared pair backed by a spawned task, but the caller-visible contract is the returned cancellation handle plus connected output receiver.

### Observability attachment

Observability listeners are optional.

- listener registration belongs to the dispatcher builder, not to `PluginOperation`
- listeners observe `ObservabilityEvent<Output>`
- listener-side processing failure after event dispatch does not retroactively fail plugin output publication
- observability attachment must not change the plugin operation contract itself

When observability is configured, the dispatcher instruments the sender side of the connected output channel by wrapping the concrete sender endpoint before passing it into the selected `PluginOperation`.

- the wrapper must still satisfy the accepted `PluginSender<Output>` boundary
- the wrapper may emit `ObservabilityEvent::MessageSent` after successful publication
- operation-level lifecycle events such as start and completion belong to dispatcher-owned execution wiring rather than to `PluginOperation` itself

### Ownership boundary

- `runtime-core` continues to own the abstract contracts only
- `runtime-channel` owns the standard Tokio-backed plugin dispatcher builder
- plugin identity, plugin registry, and operation lookup remain outside this RFC
- this RFC prepares execution for one already-selected operation

## 3. Alternatives Considered

- **Put `PluginDispatcher` in `runtime-core`:** Discarded because channel construction, listener attachment, and concrete cancel-aware output wiring are runtime-owned implementation concerns, not core execution contracts.
- **Let every higher-level crate wire plugin execution manually:** Discarded because cancellation, output channel ownership, and observability attachment would drift across crates.
- **Make the dispatcher own plugin lookup by name:** Discarded because plugin identity and lookup are higher-level concerns that do not belong in the standard channel builder boundary.
- **Make the dispatcher execute only after a separate `start()` call:** Discarded for v1 because it adds lifecycle surface without proving a concrete need beyond preparation plus returned handles.
- **Return only `PluginReceiver<T>` and hide cancellation:** Discarded because plugin flows already rely on cancel-aware channels and the caller needs explicit control over cancellation intent.
- **Emit observability directly from every `PluginOperation`:** Discarded because it pushes runtime policy into the operation contract and duplicates instrumentation logic across operations.
- **Observe output publication only outside the sender boundary:** Discarded because it cannot reliably intercept each `send(...)` call without wrapping the sender endpoint passed into the operation.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| One standard dispatcher builder gives higher-level crates one accepted way to prepare plugin execution over the runtime channel backend. | The builder adds a concrete public surface in `runtime-channel` that must be maintained carefully. |
| Returning `(CancelHandler, PluginReceiver<T>)` makes cancellation and output consumption explicit at the call site. | The builder contract must stay disciplined or it will grow into a general orchestrator. |
| Observability listeners can be attached without changing `PluginOperation` or `runtime-core`. | The event attachment surface adds generic complexity around `ObservabilityEvent<T>` and sender instrumentation. |
| Keeping the dispatcher in `runtime-channel` preserves the separation between core contracts and runtime-backed wiring. | Higher-level crates still need to own plugin identity and operation selection outside this RFC. |

## 5. Acceptance Criteria

- [ ] `runtime-channel` exposes a concrete `PluginDispatcher` builder.
- [ ] `PluginDispatcher` accepts one selected `PluginOperation`.
- [ ] `PluginDispatcher` accepts one input value before build.
- [ ] `PluginDispatcher` allows observability listener registration before build.
- [ ] `PluginDispatcher` uses the standard runtime-owned `cancelable_channel<T>()` backend to create one connected cancel-aware plugin output pair.
- [ ] `PluginDispatcher::build` returns `(CancelHandler, PluginReceiver<T>)`.
- [ ] The returned `PluginReceiver<T>` is connected to the `PluginSender<T>` passed into the selected `PluginOperation`.
- [ ] The returned `CancelHandler` controls the connected output channel pair used by the dispatcher.
- [ ] Observability listeners receive events through `EventListener<ObservabilityEvent<T>>`.
- [ ] Listener attachment remains optional.
- [ ] When observability is configured, the dispatcher wraps the sender endpoint with an instrumented sender that still satisfies `PluginSender<T>`.
- [ ] Sender instrumentation can emit output publication observability without changing the `PluginOperation` contract.
- [ ] `PluginDispatcher` introduces no plugin identity, plugin registry, plugin discovery, plugin loading, dependency resolution, retry, or scheduling policy concerns.
- [ ] `PluginDispatcher` is documented as a runtime-owned builder for one already-selected operation.

## 6. Open Questions

- Should `PluginDispatcher::build` spawn execution immediately, or should it return a prepared flow that starts only when the runtime consumes another step?
- Should duplicate observability listener registrations be allowed without restriction in v1?
- Should dispatcher build failure reuse existing `RuntimeError` cases in v1, or should a dedicated dispatcher-specific error variant be added later?
- Should the builder accept the input value only through `input(...)`, or is `new(operation, input)` a better construction shape?
- Should `ObservabilityEvent::MessageSent` carry the full output payload in v1, or should sender instrumentation support a lower-cost metadata-only mode later?
