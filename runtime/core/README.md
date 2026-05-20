# runtime-core

Transport-agnostic async channel contracts and error boundary for the vector runtime.

All channel operations defined by this crate are **async-first**: no send or receive path
performs blocking work before the returned future is polled. This crate owns contracts only —
it does not own any concrete channel implementation, queueing strategy, or async executor backend.

## Contracts

### `Sender<T>` and `Receiver<T>`

Transport-agnostic async traits for single-publisher, single-subscriber message flow.

### `CancelableSender<T>` and `CancelableReceiver<T>`

Cancel-aware specializations that remain compatible with the base `Sender<T>` and
`Receiver<T>` contracts. Every `CancelableSender<T>` is usable anywhere a `Sender<T>` is
accepted; every `CancelableReceiver<T>` is usable anywhere a `Receiver<T>` is accepted.

Both endpoints expose `is_cancelled()` to allow synchronous observation of cancellation state.
Cancellation is distinguishable from ordinary channel closure via this method.

### `Operation` and `FlowOperation`

Async computation contracts for `1:1`, `N:1`, `1:N`, and `N:N` execution shapes:

- **`Operation<Input, Output>`**: Canonical `1:1` async input-result contract.
- **`ReceiverOperation<Input, Output>`**: Canonical `N:1` async receiver-driven contract.
- **`FlowOperation<Input, Output, S>`**: Canonical `1:N` async dataflow contract. Uses an explicit `S: Sender<Output>` parameter for flexible output boundaries.
- **`ReceiverFlowOperation<Input, Output, S>`**: Canonical `N:N` async dataflow contract.

### Plugins and Managed Operations

This crate provides minimalist primitives for plugin-style extensibility, focusing on pure operation contracts rather than orchestration or identity.

#### `PluginSender<T>` and `PluginReceiver<T>`

Named cancel-aware aliases for plugin-oriented boundaries. They specialize `CancelableSender<T>` and `CancelableReceiver<T>` for the reactive pipeline.

#### `PluginOperation`

A specialized version of `FlowOperation` that fixes the sender to a `PluginSender`. It is the fundamental execution unit for the vector runtime.

#### `declare_plugin_operations!`

A minimalist macro to register independent operations from external async functions. It generates standard structs that implement `PluginOperation` without introducing business logic into the macro itself.

```rust
declare_plugin_operations! {
    AddOneOp => add_one(i32, i32)
}
```

### `EventEmitter` and `EventListener`

Transport-agnostic async event-emission and observation contracts:

- **`EventEmitter<Event>`**: Compatible with `Sender<Event>`, providing fan-out registration and an `emit` alias.
- **`EventListener<Event>`**: Dedicated callback-style contract for observing emitted events.

### `CancelHandler`

Shared cancellation control trait boundary for connected cancel-aware endpoints. A concrete
implementation of `CancelHandler` is not owned by this crate — it lives in the
standard implementation crate (`runtime-channel`).

### `ControlEvent`

Canonical control signals for runtime operations (e.g., `Cancel`). Used with `EventEmitter<ControlEvent>` to signal lifecycle changes across operation boundaries.

### `ObservabilityEvent<P>`

Canonical observability signals for monitoring operation lifecycle (`OperationStarted`, `OperationCompleted`) and data flow (`MessageSent`) through the standard event contracts.

### `Encoding`

Stateless UTF-8 encoding and decoding primitive. Provides a canonical boundary for text conversion, ensuring consistent UTF-8 enforcement across the runtime.

## Scope

This crate defines contracts, not policy. It does not commit to:

- A specific async executor or transport
- A concrete channel implementation or queueing strategy
- Backpressure or retry policy
- Multi-producer or multi-consumer semantics

Crates that need only the trait contracts depend on `runtime-core`. Crates that need the
standard Tokio-backed implementation depend on `runtime-channel`.

## Dependencies

- `std` only
- `thiserror` for the `RuntimeError` enum
