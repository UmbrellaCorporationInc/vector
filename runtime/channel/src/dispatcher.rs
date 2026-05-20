//! Plugin dispatcher builder for `runtime-channel`.
//!
//! [`PluginDispatcher`] is a concrete builder that prepares one selected [`PluginOperation`]
//! for execution against the standard Tokio-backed cancel-aware output channel.
//!
//! Accepted builder responsibilities:
//! - accept one selected `PluginOperation` at construction time
//! - accept one input value before build
//! - allow optional registration of observability listeners before build
//! - create one connected cancel-aware output channel and return `(CancelHandler, PluginReceiver<T>)`
//! - wrap the sender with observability instrumentation when listeners are registered
//! - emit `OperationStarted` and `OperationCompleted` lifecycle events around the operation
//!
//! This builder introduces no plugin identity, registry, discovery, loading,
//! retry, scheduling, lifecycle supervision, or dependency resolution concerns.
//!
//! # Failure behavior
//!
//! **Operation failure:** when the selected [`PluginOperation`] returns an error, the
//! spawned task discards it and exits. The sender endpoint is dropped on task exit,
//! which closes the output channel. The caller observes this as `None` from the
//! returned receiver, identical to normal completion. No retry or supervision is
//! applied.
//!
//! **Instrumentation failure:** `emit()` calls for [`ObservabilityEvent`] are
//! fire-and-forget. A failure to dispatch an observability event does not fail
//! output publication and does not surface to the caller.
//!
//! **Listener-side failure:** [`TokioEventEmitter`] dispatches each event to listeners
//! through independent spawned tasks. A listener that returns an error closes only
//! its own dispatch loop; it does not affect other listeners, the sender, or the
//! output channel.

use std::sync::atomic::{AtomicU64, Ordering};

use runtime_core::CancelHandler;
use runtime_core::RuntimeError;
use runtime_core::RuntimeResult;
use runtime_core::cancel::CancelableSender;
use runtime_core::channel::Sender;
use runtime_core::event::{EventEmitter, EventListener, ObservabilityEvent};
use runtime_core::plugin::{PluginOperation, PluginReceiver};

use crate::cancel::{TokioCancelableSender, make_cancelable_channel};
use crate::config::ChannelConfig;
use crate::event::TokioEventEmitter;

// ---------------------------------------------------------------------------
// Operation ID generation
// ---------------------------------------------------------------------------

static OP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_operation_id() -> String {
    let seq = OP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("op-{seq}")
}

// ---------------------------------------------------------------------------
// Instrumented sender
// ---------------------------------------------------------------------------

/// Sender wrapper that emits [`ObservabilityEvent::MessageSent`] after each
/// successful publication.
///
/// The inner [`TokioEventEmitter`] handles fan-out to registered listeners.
/// When no listeners are registered the emitter is a no-op, so instrumentation
/// carries no overhead in the unobserved case.
///
/// `InstrumentedSender` satisfies `PluginSender<T>` via the blanket impl on
/// every type that implements `CancelableSender<T>`.
pub struct InstrumentedSender<T: Clone + std::fmt::Debug + Send + 'static> {
    inner: TokioCancelableSender<T>,
    emitter: TokioEventEmitter<ObservabilityEvent<T>>,
    operation_id: String,
}

impl<T: Clone + std::fmt::Debug + Send + 'static> InstrumentedSender<T> {
    const fn new(
        inner: TokioCancelableSender<T>,
        emitter: TokioEventEmitter<ObservabilityEvent<T>>,
        operation_id: String,
    ) -> Self {
        Self { inner, emitter, operation_id }
    }
}

impl<T: Clone + std::fmt::Debug + Send + 'static> Sender<T> for InstrumentedSender<T> {
    async fn send(&mut self, value: T) -> RuntimeResult<()> {
        self.inner.send(value.clone()).await?;
        // Fire-and-forget: listener-side failure does not fail output publication.
        let _ = self
            .emitter
            .emit(ObservabilityEvent::MessageSent {
                operation_id: self.operation_id.clone(),
                payload: value,
            })
            .await;
        Ok(())
    }
}

impl<T: Clone + std::fmt::Debug + Send + 'static> CancelableSender<T> for InstrumentedSender<T> {
    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Concrete builder that prepares one selected [`PluginOperation`] for execution.
///
/// `Input` and `Output` must match the operation's associated types.
/// Constructed via [`PluginDispatcher::new`]. Input and observability listeners
/// are supplied before calling [`build`][Self::build].
pub struct PluginDispatcher<Op, Input, Output>
where
    Op: PluginOperation<InstrumentedSender<Output>, Input = Input, Output = Output>,
    Output: Clone + std::fmt::Debug + Send + 'static,
{
    pub(crate) operation: Op,
    pub(crate) input: Option<Input>,
    emitter: TokioEventEmitter<ObservabilityEvent<Output>>,
}

impl<Op, Input, Output> PluginDispatcher<Op, Input, Output>
where
    Op: PluginOperation<InstrumentedSender<Output>, Input = Input, Output = Output>,
    Output: Clone + std::fmt::Debug + Send + 'static,
{
    /// Construct the dispatcher builder with a selected operation.
    ///
    /// No input is accepted here; supply it via [`input`][Self::input].
    #[must_use]
    pub fn new(operation: Op) -> Self {
        Self {
            operation,
            input: None,
            emitter: TokioEventEmitter::new(ChannelConfig::default().capacity()),
        }
    }

    /// Supply the input value that will be passed to the operation during `build`.
    #[must_use]
    pub fn input(mut self, value: Input) -> Self {
        self.input = Some(value);
        self
    }

    /// Register an observability listener.
    ///
    /// Listeners receive [`ObservabilityEvent`] notifications from the dispatcher
    /// during execution. Registration is optional; omitting it does not change
    /// the [`PluginOperation`] contract.
    #[must_use]
    pub fn observe(
        mut self,
        listener: impl EventListener<ObservabilityEvent<Output>> + 'static,
    ) -> Self {
        // Ignore registration errors: if the emitter is closed (cannot happen at
        // construction time) the listener is silently dropped.
        let _ = self.emitter.register_listener(listener);
        self
    }
}

impl<Op, Input, Output> PluginDispatcher<Op, Input, Output>
where
    Op: PluginOperation<InstrumentedSender<Output>, Input = Input, Output = Output> + 'static,
    Input: Send + 'static,
    Output: Clone + std::fmt::Debug + Send + 'static,
{
    /// Prepare the operation for execution and return a cancellation handle plus a
    /// connected output receiver.
    ///
    /// `build` creates one cancel-aware channel pair. The sender endpoint is wrapped
    /// with observability instrumentation and passed into the selected
    /// [`PluginOperation`]; the receiver endpoint is returned to the caller. The
    /// returned [`CancelHandler`] controls both channel endpoints.
    ///
    /// Lifecycle events [`ObservabilityEvent::OperationStarted`] and
    /// [`ObservabilityEvent::OperationCompleted`] are emitted around the operation
    /// execution when listeners are registered. Lifecycle wiring is owned by the
    /// dispatcher and does not affect the [`PluginOperation`] contract.
    ///
    /// The operation is spawned immediately on the current Tokio runtime. Output
    /// values become available on the returned receiver as the operation sends them.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Operation`] if no input value was supplied before
    /// `build`.
    pub fn build(mut self) -> RuntimeResult<(impl CancelHandler, impl PluginReceiver<Output>)> {
        let input = self.input.take().ok_or_else(|| {
            RuntimeError::operation("no input supplied to dispatcher before build()")
        })?;
        let operation_id = next_operation_id();

        let (handler, inner_sender, receiver) =
            make_cancelable_channel::<Output>(ChannelConfig::default());

        let mut sender = InstrumentedSender::new(inner_sender, self.emitter, operation_id.clone());

        tokio::spawn(async move {
            // Lifecycle: OperationStarted — fire-and-forget, does not block execution.
            let _ = sender
                .emitter
                .emit(ObservabilityEvent::OperationStarted { operation_id: operation_id.clone() })
                .await;

            // Operation failure: forward the error through the channel so callers can
            // distinguish a failed operation from a normal empty completion. A send
            // failure here (channel already closed) is silently discarded.
            if let Err(e) = self.operation.run(input, &mut sender).await {
                let _ = sender.inner.send_err(e).await;
            }

            // Lifecycle: OperationCompleted — fire-and-forget, does not block caller.
            let _ =
                sender.emitter.emit(ObservabilityEvent::OperationCompleted { operation_id }).await;
        });

        Ok((handler, receiver))
    }
}

#[cfg(test)]
#[path = "dispatcher_test.rs"]
mod tests;
