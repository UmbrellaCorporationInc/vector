use runtime_core::channel::{Receiver, Sender};
use runtime_core::result::RuntimeResult;

use super::{Reader, Writer};

// ---------------------------------------------------------------------------
// Minimal fixtures
// ---------------------------------------------------------------------------

struct DummySender;
struct DummyReceiver;

impl Sender<u32> for DummySender {
    async fn send(&mut self, _value: u32) -> RuntimeResult<()> {
        Ok(())
    }
}

impl Receiver<u32> for DummyReceiver {
    async fn recv(&mut self) -> RuntimeResult<Option<u32>> {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn sender_satisfies_writer_bound() {
    fn assert_writer<T, W: Writer<T>>(_: &mut W) {}
    let mut s = DummySender;
    assert_writer::<u32, _>(&mut s);
}

#[test]
fn receiver_satisfies_reader_bound() {
    fn assert_reader<T, R: Reader<T>>(_: &mut R) {}
    let mut r = DummyReceiver;
    assert_reader::<u32, _>(&mut r);
}

#[test]
fn writer_is_also_sender() {
    fn assert_sender<T, S: Sender<T>>(_: &mut S) {}
    let mut s = DummySender;
    assert_sender::<u32, _>(&mut s);
}

#[test]
fn reader_is_also_receiver() {
    fn assert_receiver<T, R: Receiver<T>>(_: &mut R) {}
    let mut r = DummyReceiver;
    assert_receiver::<u32, _>(&mut r);
}
