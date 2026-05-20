---
id: rfc-00007-runtime-core-plugin-primitives
type: rfc
code: "00007"
slug: runtime-core-plugin-primitives
title: Runtime Core Plugin Primitives
description: Defines the v1 contracts for PluginSender, PluginOperation, and PluginReceiver in runtime-core, and refines FlowOperation to make the sender boundary a type parameter.
status: implemented
created: 2026-05-03
updated: 2026-05-03
authors: []
tags:
  - runtime
  - async
  - plugin
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
  - rfc-00005-runtime-core-operation-and-event-flow-primitives
supersedes: []
superseded_by: null
aliases:
  - "RFC 00007: Runtime Core Plugin Primitives"
---

# RFC 00007: Runtime Core Plugin Primitives

## 1. Problem

[[rfc-00002-runtime-core-v1-boundary-and-async-first-contracts]] accepts `PluginSender<T>`, `PluginOperation`, and `PluginReceiver<T>` into the `runtime-core` boundary but leaves their concrete contracts undefined.

[[rfc-00005-runtime-core-operation-and-event-flow-primitives]] defines `FlowOperation<Input, Output>` with `Sender<Output>` assumed as the output boundary. That assumption is correct for plain flow execution but is too narrow for plugin execution, where the output boundary is a cancel-aware sender specialization. If the sender type is not a parameter of `FlowOperation`, `PluginOperation` cannot be expressed as a sub-trait of `FlowOperation` without either breaking the cancel-aware sender contract or duplicating the entire flow operation shape.

The project also validated that plugin identity, plugin introspection, and operation-selection tokens do not belong in `runtime-core` v1. Those concerns are useful, but they are not core execution primitives. They add naming, lookup, and orchestration policy to a crate that should stay focused on transport-agnostic async execution contracts.

The project therefore needs a smaller plugin boundary:

- `runtime-core` should define plugin-oriented sender and receiver aliases
- `runtime-core` should define the operation contract for plugin execution
- identity, introspection, dispatch, lookup, and orchestration should stay outside `runtime-core`

This RFC follows [[rfc-00002-runtime-core-v1-boundary-and-async-first-contracts]], depends on [[rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations]], and refines [[rfc-00005-runtime-core-operation-and-event-flow-primitives]] by making the sender boundary a type parameter of `FlowOperation`.

## 2. Proposal

### FlowOperation refinement

[[rfc-00005-runtime-core-operation-and-event-flow-primitives]] proposed `FlowOperation<Input, Output>` with the output boundary assumed to be `Sender<Output>`. This RFC refines that shape by making the sender type an explicit type parameter bounded by `Sender<Output>`.

```rust
pub trait FlowOperation<Input, Output, S: Sender<Output>>: Send {
    fn run(
        &self,
        input: Input,
        output: &mut S,
    ) -> impl Future<Output = RuntimeResult<()>> + Send;
}
```

This refinement preserves the semantic commitments of [[rfc-00005-runtime-core-operation-and-event-flow-primitives]]:

- `FlowOperation` remains the canonical async dataflow contract
- the output boundary is still a `Sender<Output>`
- `FlowOperation` still resolves to `RuntimeResult<()>`
- `FlowOperation` still encodes no scheduling, retry, cancellation, supervision, backpressure, or lifecycle policy

The receiver-driven flow shape is extended in parallel:

```rust
pub trait ReceiverFlowOperation<Input, Output, S: Sender<Output>>: Send {
    fn run(
        &self,
        input: &mut impl Receiver<Input>,
        output: &mut S,
    ) -> impl Future<Output = RuntimeResult<()>> + Send;
}
```

### PluginSender\<T\>

`PluginSender<T>` is the canonical plugin-oriented sender alias over a cancel-aware sender boundary.

- `PluginSender<T>` is a named alias over `CancelableSender<T>`
- it inherits `is_cancelled` from `CancelableSender<T>`
- it remains compatible with `Sender<T>`
- it adds naming clarity at the plugin operation output boundary without introducing new semantics

```rust
pub trait PluginSender<T>: CancelableSender<T> {}
```

### PluginReceiver\<T\>

`PluginReceiver<T>` is the canonical plugin-oriented receiver alias over a cancel-aware receiver boundary.

- `PluginReceiver<T>` is a named alias over `CancelableReceiver<T>`
- it inherits `is_cancelled` from `CancelableReceiver<T>`
- it remains compatible with `Receiver<T>`
- it adds naming clarity at plugin output boundaries without introducing new semantics

```rust
pub trait PluginReceiver<T>: CancelableReceiver<T> {}
```

### PluginOperation

`PluginOperation` is the canonical named plugin-oriented specialization of `FlowOperation` that writes through a cancel-aware plugin sender boundary.

- `PluginOperation` is a sub-trait of `FlowOperation<Self::Input, Self::Output, Self::Sender>`
- `PluginOperation` exposes `name` as a stable operation identity string for documentation and higher-level dispatch layers
- `Input` and `Output` are associated types on the trait
- `PluginOperation` is `1:N`: one input value and multiple output values written through the cancel-aware sender
- `PluginOperation` must not encode scheduling, retry, cancellation control, supervision, ordering guarantees, lifecycle management, or execution policy

```rust
pub trait PluginOperation:
    FlowOperation<Self::Input, Self::Output, Self::Sender>
{
    type Input: Send + 'static;
    type Output: Send + 'static;
    type Sender: PluginSender<Self::Output>;

    fn name(&self) -> &str;
}
```

The `name` method returns the operation name as a string slice. It is stable metadata for the operation, but it is not by itself a core lookup or dispatch boundary.

### Macro-assisted operation authoring

This RFC accepts macro-assisted operation authoring as an ergonomic path for plugin-related crates, but not as a required part of the core contract.

- a macro may wrap async functions into concrete `PluginOperation` implementors
- the macro may reject duplicate operation names within one declaration block
- the macro may generate metadata helpers for higher-level crates
- the macro must remain manifest-oriented and must not become the only valid way to implement `PluginOperation`

Illustrative input:

```rust
declare_operations! {
    async fn summarize(input: TextInput, output: &mut impl PluginSender<TextChunk>) -> RuntimeResult<()> {
        /* ... */
    }
}
```

Illustrative generated shape:

```rust
pub struct SummarizeOperation;

impl<S> PluginOperation<S> for SummarizeOperation
where
    S: PluginSender<TextChunk>,
{
    type Input = TextInput;
    type Output = TextChunk;

    fn name(&self) -> &str {
        "summarize"
    }
}

impl<S> FlowOperation<TextInput, TextChunk, S> for SummarizeOperation
where
    S: PluginSender<TextChunk>,
{
    fn run(
        &self,
        input: TextInput,
        output: &mut S,
    ) -> impl Future<Output = RuntimeResult<()>> + Send {
        async move {
            summarize(input, output).await
        }
    }
}
```

The exact generated shape may change during implementation to satisfy stable Rust rules, but the accepted contract boundary is the existence of generated `PluginOperation` wrappers over valid async functions, not any specific introspection or dispatch API.

### Ownership boundary

- This RFC defines execution contracts only
- This RFC does not define plugin identity
- This RFC does not define plugin introspection
- This RFC does not define operation-selection tokens
- This RFC does not define plugin lookup
- This RFC does not define plugin dispatch or orchestration
- This RFC does not define `PluginDispatcher`
- This RFC does not define plugin discovery, lifecycle management, versioning, or dependency resolution
- The `FlowOperation` sender-type refinement should be treated as an amendment to [[rfc-00005-runtime-core-operation-and-event-flow-primitives]]

### Compatibility rules

- `PluginSender<T>` must remain usable anywhere `CancelableSender<T>` is accepted
- `PluginReceiver<T>` must remain usable anywhere `CancelableReceiver<T>` is accepted
- `PluginOperation` must remain usable anywhere `FlowOperation<Self::Input, Self::Output, Self::Sender>` is accepted
- `FlowOperation<Input, Output, S>` must remain backward compatible with the plain-sender case when `S: Sender<Output>`

### Design rules

- `PluginOperation` must be trait-based as a sub-trait of `FlowOperation`
- `PluginSender<T>` and `PluginReceiver<T>` must remain pure naming aliases over existing cancel-aware contracts
- `PluginOperation::name` must be a `&str` return with no allocation on access
- identity, introspection, tokens, plugin dispatch, and orchestration must remain outside `runtime-core`
- macro-generated authoring may be the preferred ergonomic path, but manual `PluginOperation` implementations must remain valid

## 3. Alternatives Considered

- **`PluginOperation` as an independent trait unrelated to `FlowOperation`:** Discarded because it creates a parallel execution model and weakens shared composition with other runtime flow contracts.
- **`PluginOperation` as a sub-trait of `FlowOperation<Input, Output>` with `Sender<Output>` assumed:** Discarded because plugin execution writes through a cancel-aware sender boundary and the plain sender assumption cannot express that without losing cancel-awareness at the contract level.
- **Making `Sender<Output>` a fixed associated type on `FlowOperation` rather than a type parameter:** Discarded because it complicates the plain-sender case and makes the base trait harder to implement for non-plugin flows.
- **Keeping `Plugin`, `PluginId`, or `OperationToken` in `runtime-core`:** Discarded because those concepts belong to identity, introspection, selection, or orchestration rather than to the minimal core execution boundary.
- **`PluginSender<T>` and `PluginReceiver<T>` as new independent traits with new semantics:** Discarded because they would duplicate the cancel-aware sender and receiver contracts already accepted in [[rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations]].
- **Forcing all plugin-related implementations to use macros:** Discarded because the runtime contract must remain implementable manually and must not depend on a specific code-generation path for correctness.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Making the sender type a parameter of `FlowOperation` lets `PluginOperation` be a genuine sub-trait without losing cancel-awareness. | Adding a third type parameter to `FlowOperation` increases annotation burden at some call sites. |
| Keeping only `PluginSender`, `PluginReceiver`, and `PluginOperation` in `runtime-core` produces a smaller, more stable boundary. | Higher-level crates must define identity, introspection, and dispatch explicitly instead of inheriting them from core. |
| Named aliases preserve semantic clarity at plugin boundaries without duplicating cancel-aware contracts. | Stable Rust may still require implementation compromises in how those aliases are represented concretely. |
| Macro-generated wrappers can eliminate repetitive boilerplate for operation authors. | A procedural macro path likely requires an additional crate and increases tooling complexity. |
| Removing plugin identity and dispatch from core keeps policy out of the lowest shared crate. | Other crates must own those concerns sooner, which increases the number of architectural boundaries in the short term. |

## 5. Acceptance Criteria

- [ ] `runtime-core` refines `FlowOperation<Input, Output>` to `FlowOperation<Input, Output, S: Sender<Output>>`.
- [ ] The refined `FlowOperation` preserves the semantic commitments from [[rfc-00005-runtime-core-operation-and-event-flow-primitives]].
- [ ] The refined `ReceiverFlowOperation` is extended in parallel to accept `S: Sender<Output>` as a type parameter.
- [ ] `runtime-core` exposes `PluginSender<T>` as a named alias over `CancelableSender<T>`.
- [ ] `PluginSender<T>` remains usable anywhere `CancelableSender<T>` is accepted.
- [ ] `runtime-core` exposes `PluginReceiver<T>` as a named alias over `CancelableReceiver<T>`.
- [ ] `PluginReceiver<T>` remains usable anywhere `CancelableReceiver<T>` is accepted.
- [ ] `runtime-core` exposes `PluginOperation` as a sub-trait of `FlowOperation<Self::Input, Self::Output, Self::Sender>`.
- [ ] `PluginOperation` exposes `type Input: Send + 'static` as an associated type.
- [ ] `PluginOperation` exposes `type Output: Send + 'static` as an associated type.
- [ ] `PluginOperation` exposes `type Sender` constrained to a plugin-oriented sender boundary.
- [ ] `PluginOperation` exposes `fn name(&self) -> &str` as stable operation metadata.
- [ ] `PluginOperation` is `1:N` and writes through a cancel-aware plugin sender boundary.
- [ ] `PluginOperation` introduces no scheduling, retry, cancellation control, supervision, ordering, lifecycle, or execution policy.
- [ ] `runtime-core` exposes no `Plugin`, `PluginId`, or `OperationToken` contract in this RFC.
- [ ] This RFC keeps plugin identity, introspection, dispatch, and orchestration outside `runtime-core`.
- [ ] Macro-assisted authoring may generate `PluginOperation` wrappers from valid async functions.
- [ ] Manual `PluginOperation` implementations remain valid without requiring a macro path.
- [ ] No accepted type in this RFC encodes VECTOR feature policy, document schema policy, protocol adapter behavior, or workflow-specific decisions.

## 6. Open Questions

- Should `PluginOperation` associated types carry additional bounds beyond `Send + 'static` in v1, or should bounds be tightened only when a concrete implementor requires them?
- Should `PluginReceiver<T>` remain a named alias over `CancelableReceiver<T>`, or is there any real pressure for a dedicated plugin receiver contract later?
- Should macro-generated operation wrappers remain in `runtime-core`, or should they move to a companion crate such as `runtime-core-macros`?
