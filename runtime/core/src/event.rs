use crate::{RuntimeResult, Sender};
use std::future::Future;

/// Canonical control signals for runtime operations.
///
/// `ControlEvent` allows signaling lifecycle changes (like cancellation) to async
/// operations through the standard [`EventEmitter`] and [`EventListener`] contracts.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ControlEvent {
    /// Signals that the operation should stop processing and shutdown.
    Cancel,
}

/// Canonical observability signals for runtime operations.
///
/// `ObservabilityEvent<P>` allows monitoring the lifecycle and data flow of async
/// operations (like message publication or task execution) through the standard
/// [`EventEmitter`] and [`EventListener`] contracts.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ObservabilityEvent<P>
where
    P: std::fmt::Debug + Clone + Send + 'static,
{
    /// Signals that a tracked operation has begun.
    OperationStarted {
        /// Unique identifier for the operation instance.
        operation_id: String,
    },
    /// Signals that a tracked operation has finished successfully.
    OperationCompleted {
        /// Unique identifier for the operation instance.
        operation_id: String,
    },
    /// Signals that a message was successfully emitted by an operation.
    MessageSent {
        /// Unique identifier for the operation instance.
        operation_id: String,
        /// The payload that was sent.
        payload: P,
    },
}

/// Canonical transport-agnostic event-observation contract.
///
/// `EventListener<Event>` allows observing events emitted from an [`EventEmitter<Event>`] boundary.
pub trait EventListener<Event>: Send {
    /// Handles a dispatched event after emitter-side publication has already succeeded.
    fn on_event(&mut self, event: Event) -> impl Future<Output = RuntimeResult<()>> + Send;
}

/// Canonical transport-agnostic event-emission contract.
///
/// `EventEmitter<Event>` allows publishing shared runtime events into a fan-out boundary.
/// It is compatible with [`Sender<Event>`] and provides an `emit` alias for publication.
pub trait EventEmitter<Event>: Sender<Event> {
    /// Registers a new listener in the emitter boundary.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be registered (e.g., registry is full or closed).
    fn register_listener(
        &mut self,
        listener: impl EventListener<Event> + 'static,
    ) -> RuntimeResult<()>;

    /// Dispatches an event through sender semantics without waiting for listener-side processing.
    fn emit(&mut self, event: Event) -> impl Future<Output = RuntimeResult<()>> + Send {
        self.send(event)
    }
}

#[cfg(test)]
#[path = "event_test.rs"]
mod tests;
