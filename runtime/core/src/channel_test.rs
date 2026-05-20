use super::{Receiver, Sender};
use crate::error::RuntimeError;
use crate::result::RuntimeResult;
use std::future::{Future, ready};

// Minimal stub implementations used only to verify the contract surface compiles.
// These are never instantiated at runtime.

struct StubSender;
struct StubReceiver;

impl Sender<u32> for StubSender {
    fn send(&mut self, _value: u32) -> impl Future<Output = RuntimeResult<()>> + Send {
        ready(Err(RuntimeError::ChannelClosed))
    }
}

impl Receiver<u32> for StubReceiver {
    fn recv(&mut self) -> impl Future<Output = RuntimeResult<Option<u32>>> + Send {
        ready(Ok(None))
    }
}

// ---------------------------------------------------------------------------
// Contract surface tests
// ---------------------------------------------------------------------------

fn requires_send_future<F: Future + Send>(_: F) {}
fn requires_result_option_future<F: Future<Output = RuntimeResult<Option<u32>>> + Send>(_: F) {}
fn requires_send<T: Send>(_: T) {}

#[test]
fn sender_send_returns_runtime_result() {
    let mut s = StubSender;
    requires_send_future(s.send(1_u32));
}

#[test]
fn receiver_recv_returns_result_option() {
    let mut r = StubReceiver;
    requires_result_option_future(r.recv());
}

#[test]
fn sender_is_send() {
    requires_send(StubSender);
}

#[test]
fn receiver_is_send() {
    requires_send(StubReceiver);
}
