//! Cancel-aware channel contracts.
//!
//! Extends the base [`Sender`] and [`Receiver`] contracts with optional cancellation
//! observation. Every `CancelableSender<T>` satisfies `Sender<T>` and every
//! `CancelableReceiver<T>` satisfies `Receiver<T>`.
//!
//! [`CancelHandler`] is the shared cancellation control trait boundary. A concrete
//! implementation is owned by `runtime-channel`.

use crate::channel::{Receiver, Sender};

/// Shared cancellation control trait boundary for a connected cancel-aware channel pair.
///
/// Calling [`cancel`][CancelHandler::cancel] signals both connected endpoints.
/// Both endpoints observe the cancellation state via `is_cancelled`.
pub trait CancelHandler: Send {
    /// Signal cancellation to both connected cancel-aware endpoints.
    fn cancel(&self);

    /// Returns `true` if cancellation has been signalled.
    #[must_use]
    fn is_cancelled(&self) -> bool;
}

/// Cancel-aware specialization of [`Sender<T>`], compatible with the base contract.
///
/// Every `CancelableSender<T>` is also a `Sender<T>`.
pub trait CancelableSender<T>: Sender<T> {
    /// Returns `true` if the shared cancellation flag has been set.
    fn is_cancelled(&self) -> bool;
}

/// Cancel-aware specialization of [`Receiver<T>`], compatible with the base contract.
///
/// Every `CancelableReceiver<T>` is also a `Receiver<T>`.
pub trait CancelableReceiver<T>: Receiver<T> {
    /// Returns `true` if the shared cancellation flag has been set.
    ///
    /// When `true`, `recv()` will yield `None` without waiting for a value, allowing
    /// callers to distinguish observed cancellation from ordinary channel closure.
    fn is_cancelled(&self) -> bool;
}

#[cfg(test)]
#[path = "cancel_test.rs"]
mod tests;
