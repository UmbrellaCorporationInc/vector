//! Concrete [`CancelHandler`] implementation for `runtime-channel`.
//!
//! Uses an [`AtomicBool`] flag for synchronous state observation and a
//! `tokio::sync::watch` channel to wake pending awaits when cancellation is signalled.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use runtime_core::CancelHandler;
use tokio::sync::watch;

/// Initial cancel state plus its watch receiver, returned by [`CancelState::new`].
pub type CancelStateInit = (CancelState, watch::Receiver<bool>);

/// Shared cancellation state distributed to cancel-aware channel endpoints.
///
/// Cloning produces a handle that observes the same cancellation state.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct CancelState {
    flag: Arc<AtomicBool>,
    watch_tx: Arc<watch::Sender<bool>>,
}

impl CancelState {
    /// Construct a new [`CancelState`] and its initial watch receiver.
    #[must_use]
    pub fn new() -> CancelStateInit {
        let (watch_tx, watch_rx) = watch::channel(false);
        let state = Self { flag: Arc::new(AtomicBool::new(false)), watch_tx: Arc::new(watch_tx) };
        (state, watch_rx)
    }

    /// Returns `true` if cancellation has been signalled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Signal cancellation and wake all pending watch receivers.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
        // Ignore send errors: receivers may already be dropped.
        let _ = self.watch_tx.send(true);
    }

    /// Subscribe a new watch receiver for cancellation wake-up.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.watch_tx.subscribe()
    }
}

// ---------------------------------------------------------------------------
// Public CancelHandler implementation
// ---------------------------------------------------------------------------

/// Standard cancellation handler returned by [`cancelable_channel`][crate::cancelable_channel].
///
/// Calling [`cancel`][TokioCancelHandler::cancel] sets the shared flag and wakes all
/// pending awaits on connected cancel-aware endpoints via `tokio::sync::watch`.
pub struct TokioCancelHandler {
    state: CancelState,
}

impl TokioCancelHandler {
    /// Construct a [`TokioCancelHandler`] from the given [`CancelState`].
    #[must_use]
    pub const fn new(state: CancelState) -> Self {
        Self { state }
    }
}

impl CancelHandler for TokioCancelHandler {
    fn cancel(&self) {
        self.state.cancel();
    }

    fn is_cancelled(&self) -> bool {
        self.state.is_cancelled()
    }
}

#[cfg(test)]
#[path = "handler_test.rs"]
mod tests;
