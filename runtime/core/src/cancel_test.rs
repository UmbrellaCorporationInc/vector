use super::{CancelHandler, CancelableReceiver, CancelableSender};
use crate::channel::{Receiver, Sender};
use crate::error::RuntimeError;
use crate::result::RuntimeResult;
use std::future::{Future, ready};

// Minimal stub implementations used only to verify the cancel contract surface compiles.
// These are never instantiated at runtime.

struct StubCancelHandler {
    cancelled: bool,
}

impl CancelHandler for StubCancelHandler {
    fn cancel(&self) {}

    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

struct StubCancelableSender {
    cancelled: bool,
}

impl Sender<u32> for StubCancelableSender {
    fn send(&mut self, _value: u32) -> impl Future<Output = RuntimeResult<()>> + Send {
        ready(Err(RuntimeError::Cancelled))
    }
}

impl CancelableSender<u32> for StubCancelableSender {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

struct StubCancelableReceiver {
    cancelled: bool,
}

impl Receiver<u32> for StubCancelableReceiver {
    fn recv(&mut self) -> impl Future<Output = RuntimeResult<Option<u32>>> + Send {
        ready(Ok(None))
    }
}

impl CancelableReceiver<u32> for StubCancelableReceiver {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

// ---------------------------------------------------------------------------
// Contract surface tests
// ---------------------------------------------------------------------------

#[test]
fn cancel_handler_exposes_cancel_and_is_cancelled() {
    let h = StubCancelHandler { cancelled: false };
    h.cancel();
    assert!(!h.is_cancelled());
}

#[test]
fn cancelable_sender_satisfies_sender_contract() {
    fn requires_sender<T: Send, S: Sender<T>>(_: &mut S) {}
    let mut s = StubCancelableSender { cancelled: false };
    requires_sender::<u32, _>(&mut s);
}

#[test]
fn cancelable_receiver_satisfies_receiver_contract() {
    fn requires_receiver<T: Send, R: Receiver<T>>(_: &mut R) {}
    let mut r = StubCancelableReceiver { cancelled: false };
    requires_receiver::<u32, _>(&mut r);
}

#[test]
fn cancelable_sender_exposes_is_cancelled() {
    let s = StubCancelableSender { cancelled: true };
    assert!(s.is_cancelled());
}

#[test]
fn cancelable_receiver_exposes_is_cancelled() {
    let r = StubCancelableReceiver { cancelled: true };
    assert!(r.is_cancelled());
}

#[test]
fn cancel_handler_is_send() {
    fn requires_send<T: Send>(_: T) {}
    requires_send(StubCancelHandler { cancelled: false });
}
