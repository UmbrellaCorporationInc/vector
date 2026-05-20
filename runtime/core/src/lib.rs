//! Core runtime primitives for the vector system.
//!
//! This crate defines the fundamental traits and types used throughout the
//! runtime, including operations, events, and results.

/// Cancelation tokens and traits.
pub mod cancel;
/// Core channel traits.
pub mod channel;
/// Data encoding and decoding primitives.
pub mod encoding;
/// Error handling types.
pub mod error;
/// Event-driven primitives.
pub mod event;
/// Dataflow operation contracts.
pub mod operation;
/// Plugin architecture primitives.
pub mod plugin;
/// Result type aliases and future utilities.
pub mod result;

// Re-exports
pub use cancel::{CancelHandler, CancelableReceiver, CancelableSender};
pub use channel::{Receiver, Sender};
pub use encoding::Encoding;
pub use error::RuntimeError;
pub use event::{ControlEvent, EventEmitter, EventListener, ObservabilityEvent};
pub use operation::{FlowOperation, Operation};
pub use plugin::{PluginOperation, PluginReceiver, PluginSender};
pub use result::RuntimeResult;

/// Crate-level prelude for common imports.
pub mod prelude {
    pub use super::cancel::{CancelableReceiver, CancelableSender};
    pub use super::channel::{Receiver, Sender};
    pub use super::error::RuntimeError;
    pub use super::operation::{FlowOperation, Operation};
    pub use super::plugin::{PluginOperation, PluginReceiver, PluginSender};
    pub use super::result::RuntimeResult;
}
