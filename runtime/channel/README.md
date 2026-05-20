# runtime-channel

Standard Tokio-backed implementation of the `runtime-core` channel contracts for the vector runtime.

This crate owns the concrete channel implementation. The trait contracts (`Sender<T>`,
`Receiver<T>`, `CancelableSender<T>`, `CancelableReceiver<T>`, `CancelHandler`) remain in
`runtime-core`. This crate depends on both `runtime-core` and `tokio`.

## Factories

```rust
use runtime_channel::{channel, cancelable_channel};
use runtime_core::{Sender, Receiver, CancelHandler, CancelableSender, CancelableReceiver};

// Base channel — one sender, one receiver.
let (mut tx, mut rx) = channel::<u32>();

// Cancel-aware channel — one handler, one sender, one receiver.
let (handler, mut tx, mut rx) = cancelable_channel::<u32>();
handler.cancel();
assert!(tx.is_cancelled());
assert!(rx.is_cancelled());

// Event Emitter — fan-out broadcast.
let mut emitter = runtime_channel::emitter::<String>();
emitter.register_listener(my_listener)?;
emitter.emit("event".to_string()).await?;
```

Concrete endpoint types are hidden behind `impl Trait` — callers depend only on the
`runtime-core` trait contracts.

## Plugin Dispatcher

`PluginDispatcher` is a concrete builder that prepares one selected `PluginOperation` for
execution against the standard cancel-aware output channel. It is the standard entry point
for plugin execution wiring in `runtime-channel`.

```rust
use runtime_channel::PluginDispatcher;
use runtime_core::CancelHandler;
use runtime_core::channel::Receiver;

// Build: returns a cancel handle and a connected output receiver.
let (handler, mut rx) = PluginDispatcher::new(my_op)
    .input(my_input)
    .observe(my_listener)   // optional
    .build()?;

// Consume output values as the operation produces them.
while let Some(value) = rx.recv().await { ... }

// Cancel the output channel pair at any time.
handler.cancel();
```

### Output channel topology

```
PluginDispatcher::build()
    │
    ├─ cancelable_channel<Output>()
    │       ├─ CancelHandler  ──────────────────────────────────► returned to caller
    │       ├─ TokioCancelableSender<Output>
    │       │     └─ wrapped by InstrumentedSender
    │       │           └─ passed into PluginOperation::run()
    │       └─ TokioCancelableReceiver<Output> ─────────────────► returned to caller
    │
    └─ tokio::spawn  ──► OperationStarted → run() → OperationCompleted
```

### Observability

When listeners are registered via `observe()`, the dispatcher emits three event kinds through
`InstrumentedSender` and the lifecycle wiring:

| Event | When emitted |
|---|---|
| `OperationStarted` | Before `run()` is called, inside the spawned task |
| `MessageSent` | After each successful `send()` through `InstrumentedSender` |
| `OperationCompleted` | After `run()` returns, inside the spawned task |

All observability events are fire-and-forget. Listener-side failure never retroactively
fails output publication or other listeners.

### Dispatcher boundary

`PluginDispatcher` introduces no plugin identity, registry, discovery, loading, retry,
scheduling, lifecycle supervision, or dependency resolution concerns. It prepares execution
for one already-selected operation only.

## Transport

The standard implementation uses bounded `tokio::sync::mpsc`. Unbounded channel transport is
not used. Channel capacity is supplied through the `runtime-channel` configuration value;
it is not hard-coded.

## Cancellation

Cancellation uses two primitives:

- A shared flag for synchronous state observation via `is_cancelled()`.
- `tokio::sync::watch` for wake-up, so pending awaits are released when cancellation is
  signalled. A flag alone is not sufficient because it cannot wake a pending async task.

Cancellation remains distinguishable from ordinary channel closure through the
`is_cancelled()` observation rule defined in `runtime-core`.

## Scope

This crate owns:

- The concrete `CancelHandler` implementation.
- The Tokio-backed base channel implementation.
- The Tokio-backed cancel-aware channel implementation.
- The Tokio-backed event emitter (fan-out broadcaster) implementation.
- The `PluginDispatcher` builder and `InstrumentedSender` wrapper.
- Channel capacity configuration.
- Channel-oriented utilities specific to construction, configuration, or cancellation wake-up.

This crate does not own the channel trait contracts or the error boundary — those remain in
`runtime-core`.

## Dependencies

- `runtime-core` — channel contracts and error boundary
- `tokio` — async runtime backend (`tokio::sync::mpsc`, `tokio::sync::watch`)
- `thiserror` — error enum derivation (inherited from workspace)
