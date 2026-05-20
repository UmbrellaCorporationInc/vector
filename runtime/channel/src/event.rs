//! Tokio-backed concrete event emitter implementation.
//!
//! `TokioEventEmitter` manages a collection of [`EventListener`] endpoints
//! and performs asynchronous fan-out dispatch. It satisfies the [`EventEmitter`]
//! contract by managing an internal task per registered listener.

use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use runtime_core::RuntimeResult;
use runtime_core::channel::Sender;
use runtime_core::event::{EventEmitter, EventListener};

/// Tokio-backed concrete event emitter implementation.
///
/// `TokioEventEmitter` manages a collection of [`EventListener`] endpoints
/// and performs asynchronous fan-out dispatch. It satisfies the [`EventEmitter`]
/// contract by managing an internal task per registered listener.
pub(crate) struct TokioEventEmitter<Event> {
    listeners: Arc<Mutex<Vec<mpsc::Sender<Event>>>>,
    capacity: usize,
}

impl<Event: Send + Clone + 'static> TokioEventEmitter<Event> {
    /// Creates a new event emitter with the specified internal per-listener capacity.
    pub(crate) fn new(capacity: usize) -> Self {
        Self { listeners: Arc::new(Mutex::new(Vec::new())), capacity }
    }
}

impl<Event: Send + Clone + 'static> Sender<Event> for TokioEventEmitter<Event> {
    fn send(&mut self, event: Event) -> impl Future<Output = RuntimeResult<()>> + Send {
        let listeners_lock = Arc::clone(&self.listeners);
        async move {
            let listeners =
                listeners_lock.lock().map_err(|_| runtime_core::RuntimeError::ChannelClosed)?;

            // Fan-out: send to all registered listener tasks.
            // We use fire-and-forget semantics as per RFC 00005.
            for tx in &*listeners {
                let tx = tx.clone();
                let event = event.clone();
                tokio::spawn(async move {
                    let _ = tx.send(event).await;
                });
            }
            drop(listeners);
            Ok(())
        }
    }
}

impl<Event: Send + Clone + 'static> EventEmitter<Event> for TokioEventEmitter<Event> {
    fn register_listener(
        &mut self,
        mut listener: impl EventListener<Event> + 'static,
    ) -> RuntimeResult<()> {
        let (tx, mut rx) = mpsc::channel::<Event>(self.capacity);

        // Spawn a dedicated task for this listener to handle its events sequentially.
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if listener.on_event(event).await.is_err() {
                    // If the listener fails, we close the dispatch loop for this listener.
                    break;
                }
            }
        });

        self.listeners.lock().map_err(|_| runtime_core::RuntimeError::ChannelClosed)?.push(tx);
        Ok(())
    }
}

#[cfg(test)]
#[path = "event_test.rs"]
mod tests;
