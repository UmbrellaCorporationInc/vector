//! Standard bounded channel factories for `runtime-channel`.
//!
//! [`channel`] and [`cancelable_channel`] are the primary entry points for creating
//! channel pairs backed by the default [`ChannelConfig`].

use runtime_core::CancelHandler;
use runtime_core::cancel::{CancelableReceiver, CancelableSender};
use runtime_core::channel::{Receiver, Sender};
use runtime_core::event::EventEmitter;

use crate::cancel::make_cancelable_channel;
use crate::channel::make_channel;
use crate::config::ChannelConfig;
use crate::event::TokioEventEmitter;

/// Construct a standard bounded base channel pair using the default capacity.
///
/// Returns `(impl Sender<T>, impl Receiver<T>)`. Concrete endpoint types are not
/// part of the public API.
#[must_use]
pub fn channel<T: Send + 'static>() -> (impl Sender<T>, impl Receiver<T>) {
    make_channel(ChannelConfig::default())
}

/// Construct a standard bounded cancel-aware channel using the default capacity.
///
/// Returns `(impl CancelHandler, impl CancelableSender<T>, impl CancelableReceiver<T>)`.
/// Concrete endpoint types are not part of the public API.
#[must_use]
pub fn cancelable_channel<T: Send + 'static>()
-> (impl CancelHandler, impl CancelableSender<T>, impl CancelableReceiver<T>) {
    make_cancelable_channel(ChannelConfig::default())
}

/// Construct a standard Tokio-backed event emitter using the default capacity.
///
/// Returns `impl EventEmitter<Event>`. Concrete emitter type is not part of the public API.
#[must_use]
pub fn emitter<Event: Send + Clone + 'static>() -> impl EventEmitter<Event> {
    TokioEventEmitter::new(ChannelConfig::default().capacity())
}

#[cfg(test)]
#[path = "factory_test.rs"]
mod tests;
