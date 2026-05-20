//! Standard Tokio-backed cancel-aware channel implementation.
//!
//! [`TokioCancelableSender<T>`] and [`TokioCancelableReceiver<T>`] implement the
//! [`CancelableSender<T>`] and [`CancelableReceiver<T>`] contracts from `runtime-core`.
//!
//! Cancellation state is observable synchronously via `is_cancelled()`. Pending awaits
//! are released through a `tokio::sync::watch` channel so no poll loop is required.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use runtime_core::RuntimeError;
use runtime_core::RuntimeResult;
use runtime_core::cancel::{CancelableReceiver, CancelableSender};
use runtime_core::channel::{Receiver, Sender};
use tokio::sync::{mpsc, watch};

use crate::config::ChannelConfig;
use crate::handler::{CancelState, TokioCancelHandler};

// Internal transport envelope: errors flow through the same channel as values.
type Item<T> = Result<T, RuntimeError>;

// ---------------------------------------------------------------------------
// Public cancel-aware channel pair type alias (RULE-5A)
// ---------------------------------------------------------------------------

/// Full cancel-aware channel returned by [`make_cancelable_channel`]:
/// handler, sender, and receiver.
pub type CancelableChannel<T> =
    (TokioCancelHandler, TokioCancelableSender<T>, TokioCancelableReceiver<T>);

// ---------------------------------------------------------------------------
// Sender
// ---------------------------------------------------------------------------

/// Tokio-backed cancel-aware sender.
pub struct TokioCancelableSender<T> {
    inner: mpsc::Sender<Item<T>>,
    state: CancelState,
}

impl<T: Send + 'static> Sender<T> for TokioCancelableSender<T> {
    fn send(&mut self, value: T) -> impl Future<Output = RuntimeResult<()>> + Send {
        CancelSendFuture {
            state: self.state.clone(),
            watch_rx: self.state.subscribe(),
            send_fut: Box::pin({
                let sender = self.inner.clone();
                async move { sender.send(Ok(value)).await.map_err(|_| RuntimeError::ChannelClosed) }
            }),
            watch_fut: None,
        }
    }
}

impl<T: Send + 'static> TokioCancelableSender<T> {
    /// Send an error through the channel so the receiver surfaces it as `Err(e)`.
    pub(crate) async fn send_err(&self, error: RuntimeError) -> RuntimeResult<()> {
        self.inner.send(Err(error)).await.map_err(|_| RuntimeError::ChannelClosed)
    }
}

impl<T: Send + 'static> CancelableSender<T> for TokioCancelableSender<T> {
    fn is_cancelled(&self) -> bool {
        self.state.is_cancelled()
    }
}

// ---------------------------------------------------------------------------
// Receiver
// ---------------------------------------------------------------------------

/// Tokio-backed cancel-aware receiver.
pub struct TokioCancelableReceiver<T> {
    inner: mpsc::Receiver<Item<T>>,
    state: CancelState,
    watch_rx: watch::Receiver<bool>,
}

impl<T: Send + 'static> Receiver<T> for TokioCancelableReceiver<T> {
    fn recv(&mut self) -> impl Future<Output = RuntimeResult<Option<T>>> + Send {
        CancelRecvFuture {
            inner: &mut self.inner,
            state: &self.state,
            watch_rx: &mut self.watch_rx,
            watch_fut: None,
        }
    }
}

impl<T: Send + 'static> CancelableReceiver<T> for TokioCancelableReceiver<T> {
    fn is_cancelled(&self) -> bool {
        self.state.is_cancelled()
    }
}

// ---------------------------------------------------------------------------
// Send future
// ---------------------------------------------------------------------------

type WatchFut = Pin<Box<dyn Future<Output = Result<(), watch::error::RecvError>> + Send>>;

struct CancelSendFuture {
    state: CancelState,
    watch_rx: watch::Receiver<bool>,
    send_fut: Pin<Box<dyn Future<Output = RuntimeResult<()>> + Send>>,
    // Lazily created so the waker is registered once and reused across polls.
    watch_fut: Option<WatchFut>,
}

impl Future for CancelSendFuture {
    type Output = RuntimeResult<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.state.is_cancelled() {
            return Poll::Ready(Err(RuntimeError::Cancelled));
        }
        // Lazily init the watch future and keep it alive so the waker persists.
        let watch_fut =
            this.watch_fut.get_or_insert_with(|| Box::pin(watch_changed(this.watch_rx.clone())));
        if watch_fut.as_mut().poll(cx).is_ready() && this.state.is_cancelled() {
            return Poll::Ready(Err(RuntimeError::Cancelled));
        }
        this.send_fut.as_mut().poll(cx)
    }
}

// SAFETY: CancelState is Send; watch::Receiver<bool> is Send; boxed futures are Send.
unsafe impl Send for CancelSendFuture {}

// ---------------------------------------------------------------------------
// Recv future
// ---------------------------------------------------------------------------

struct CancelRecvFuture<'a, T> {
    inner: &'a mut mpsc::Receiver<Item<T>>,
    state: &'a CancelState,
    watch_rx: &'a mut watch::Receiver<bool>,
    watch_fut: Option<WatchFut>,
}

impl<T: Send> Future for CancelRecvFuture<'_, T> {
    type Output = RuntimeResult<Option<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.state.is_cancelled() {
            return Poll::Ready(Ok(None));
        }
        // Lazily init the watch future and keep it alive so the waker persists.
        let watch_fut =
            this.watch_fut.get_or_insert_with(|| Box::pin(watch_changed(this.watch_rx.clone())));
        if watch_fut.as_mut().poll(cx).is_ready() && this.state.is_cancelled() {
            return Poll::Ready(Ok(None));
        }
        match this.inner.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(Ok(None)),
            Poll::Ready(Some(Ok(v))) => Poll::Ready(Ok(Some(v))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Err(e)),
        }
    }
}

// SAFETY: all fields are Send when T: Send.
unsafe impl<T: Send> Send for CancelRecvFuture<'_, T> {}

// ---------------------------------------------------------------------------
// Helper: wraps watch::Receiver::changed() to own the receiver by value.
// ---------------------------------------------------------------------------

async fn watch_changed(mut rx: watch::Receiver<bool>) -> Result<(), watch::error::RecvError> {
    rx.changed().await
}

// ---------------------------------------------------------------------------
// Factory helper
// ---------------------------------------------------------------------------

/// Construct a bounded Tokio-backed cancel-aware channel.
///
/// Returns a [`TokioCancelHandler`] plus one connected sender and one connected receiver.
/// The handler controls cancellation for both endpoints.
#[must_use]
pub fn make_cancelable_channel<T: Send + 'static>(config: ChannelConfig) -> CancelableChannel<T> {
    let (state, _initial_rx) = CancelState::new();
    let (tx, rx) = mpsc::channel::<Item<T>>(config.capacity());
    let watch_rx = state.subscribe();
    (
        TokioCancelHandler::new(state.clone()),
        TokioCancelableSender { inner: tx, state: state.clone() },
        TokioCancelableReceiver { inner: rx, state, watch_rx },
    )
}

#[cfg(test)]
#[path = "cancel_test.rs"]
mod tests;
