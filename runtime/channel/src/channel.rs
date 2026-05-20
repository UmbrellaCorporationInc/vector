//! Standard Tokio-backed base channel implementation.
//!
//! [`TokioSender<T>`] and [`TokioReceiver<T>`] implement the [`Sender<T>`] and [`Receiver<T>`]
//! contracts from `runtime-core` using a bounded `tokio::sync::mpsc` channel.
//! Capacity is supplied by [`ChannelConfig`].
//!
//! # Error forwarding
//!
//! The internal transport carries `Result<T, RuntimeError>` so that operation errors can
//! flow through the same channel without a side channel. `TokioSender<T>::send` always
//! wraps the value in `Ok(_)`. The dispatcher uses `TokioSender::send_err` to inject an
//! `Err(_)` item when the spawned operation fails. `TokioReceiver<T>::recv` decodes the
//! envelope: `Some(Ok(v))` → `Ok(Some(v))`, `Some(Err(e))` → `Err(e)`, `None` → `Ok(None)`.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use runtime_core::RuntimeError;
use runtime_core::RuntimeResult;
use runtime_core::channel::{Receiver, Sender};
use tokio::sync::mpsc;

use crate::config::ChannelConfig;

// ---------------------------------------------------------------------------
// Public channel pair type alias (RULE-5A)
// ---------------------------------------------------------------------------

/// Base channel pair returned by [`make_channel`].
pub type BaseChannel<T> = (TokioSender<T>, TokioReceiver<T>);

// ---------------------------------------------------------------------------
// Sender
// ---------------------------------------------------------------------------

/// Tokio-backed sender end of a bounded base channel.
pub struct TokioSender<T> {
    inner: mpsc::Sender<Result<T, RuntimeError>>,
}

impl<T: Send + 'static> Sender<T> for TokioSender<T> {
    fn send(&mut self, value: T) -> impl Future<Output = RuntimeResult<()>> + Send {
        let sender = self.inner.clone();
        BaseSendFuture {
            fut: Box::pin(async move {
                sender.send(Ok(value)).await.map_err(|_| RuntimeError::ChannelClosed)
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Receiver
// ---------------------------------------------------------------------------

/// Tokio-backed receiver end of a bounded base channel.
pub struct TokioReceiver<T> {
    inner: mpsc::Receiver<Result<T, RuntimeError>>,
}

impl<T: Send + 'static> Receiver<T> for TokioReceiver<T> {
    fn recv(&mut self) -> impl Future<Output = RuntimeResult<Option<T>>> + Send {
        BaseRecvFuture { inner: &mut self.inner }
    }
}

// ---------------------------------------------------------------------------
// Send future
// ---------------------------------------------------------------------------

struct BaseSendFuture {
    fut: Pin<Box<dyn Future<Output = RuntimeResult<()>> + Send>>,
}

impl Future for BaseSendFuture {
    type Output = RuntimeResult<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.fut.as_mut().poll(cx)
    }
}

// SAFETY: the inner Box<dyn Future + Send> is Send.
unsafe impl Send for BaseSendFuture {}

// ---------------------------------------------------------------------------
// Recv future
// ---------------------------------------------------------------------------

struct BaseRecvFuture<'a, T> {
    inner: &'a mut mpsc::Receiver<Result<T, RuntimeError>>,
}

impl<T: Send> Future for BaseRecvFuture<'_, T> {
    type Output = RuntimeResult<Option<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match this.inner.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(Ok(None)),
            Poll::Ready(Some(Ok(v))) => Poll::Ready(Ok(Some(v))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Err(e)),
        }
    }
}

// SAFETY: mpsc::Receiver<Result<T, RuntimeError>>: Send when T: Send; the reference is exclusive.
unsafe impl<T: Send> Send for BaseRecvFuture<'_, T> {}

// ---------------------------------------------------------------------------
// Factory helper
// ---------------------------------------------------------------------------

/// Construct a bounded Tokio-backed base channel pair.
#[must_use]
pub fn make_channel<T: Send + 'static>(config: ChannelConfig) -> BaseChannel<T> {
    let (tx, rx) = mpsc::channel(config.capacity());
    (TokioSender { inner: tx }, TokioReceiver { inner: rx })
}

#[cfg(test)]
#[path = "channel_test.rs"]
mod tests;
