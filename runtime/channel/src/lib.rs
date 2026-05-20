//! Standard Tokio-backed implementation of the `runtime-core` channel contracts.
//!
//! This crate owns the concrete channel implementation. The trait contracts
//! ([`Sender<T>`][runtime_core::channel::Sender], [`Receiver<T>`][runtime_core::channel::Receiver],
//! [`CancelableSender<T>`][runtime_core::cancel::CancelableSender],
//! [`CancelableReceiver<T>`][runtime_core::cancel::CancelableReceiver],
//! [`CancelHandler`][runtime_core::CancelHandler]) remain in `runtime-core`.
//!
//! # Transport
//!
//! The standard implementation uses bounded `tokio::sync::mpsc`. Capacity is supplied
//! through [`ChannelConfig`] — unbounded transport is never used.
//!
//! # Cancellation
//!
//! Cancellation uses an [`AtomicBool`][std::sync::atomic::AtomicBool] flag for synchronous
//! state observation and `tokio::sync::watch` for wake-up so pending awaits are released
//! when cancellation is signalled.
//!
//! # Factories
//!
//! [`channel`] and [`cancelable_channel`] are the standard entry points. Both use
//! [`ChannelConfig::default`] for capacity and hide concrete endpoint types behind
//! `impl Trait`.

pub mod cancel;
pub mod channel;
pub mod config;
pub mod dispatcher;
/// Standard Tokio-backed event emitter and listener implementation.
pub mod event;
pub mod factory;
pub mod handler;

pub use config::ChannelConfig;
pub use dispatcher::{InstrumentedSender, PluginDispatcher};
pub use factory::{cancelable_channel, channel, emitter};
pub use handler::TokioCancelHandler;
