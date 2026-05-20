//! Base channel contracts.
//!
//! [`Sender<T>`] and [`Receiver<T>`] define the async-first transport boundary for
//! single-publisher single-subscriber channels. All implementations of these traits must be
//! async-first: no blocking work may be performed before a returned future is polled.
//!
//! Concrete implementations and standard factories are owned by `runtime-channel`.

use std::future::Future;

use crate::result::RuntimeResult;

/// Canonical async contract for publishing values of type `T` to a single subscriber.
pub trait Sender<T>: Send {
    /// Send a value to the connected receiver.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::ChannelClosed`] when the receiving end has been dropped.
    fn send(&mut self, value: T) -> impl Future<Output = RuntimeResult<()>> + Send;
}

/// Canonical async contract for consuming values of type `T` from a single publisher.
pub trait Receiver<T>: Send {
    /// Receive the next value from the connected sender.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(value))` — a value was received successfully.
    /// - `Ok(None)` — the channel closed normally; the sending end was dropped.
    /// - `Err(e)` — the operation failed and forwarded a caller-meaningful error through
    ///   the channel before closing.
    fn recv(&mut self) -> impl Future<Output = RuntimeResult<Option<T>>> + Send;
}

#[cfg(test)]
#[path = "channel_test.rs"]
mod tests;
