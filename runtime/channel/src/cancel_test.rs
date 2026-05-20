#![allow(clippy::expect_used)]

use runtime_core::CancelHandler;
use runtime_core::RuntimeError;
use runtime_core::cancel::{CancelableReceiver, CancelableSender};
use runtime_core::channel::{Receiver, Sender};

use super::{TokioCancelableReceiver, TokioCancelableSender, make_cancelable_channel};
use crate::config::ChannelConfig;
use crate::handler::TokioCancelHandler;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime build failed")
}

type Fixtures = (TokioCancelHandler, TokioCancelableSender<u32>, TokioCancelableReceiver<u32>);

fn make() -> Fixtures {
    make_cancelable_channel::<u32>(ChannelConfig::default())
}

#[test]
fn endpoints_not_cancelled_before_cancel() {
    let (handler, tx, rx) = make();
    assert!(!handler.is_cancelled());
    assert!(!tx.is_cancelled());
    assert!(!rx.is_cancelled());
}

#[test]
fn cancel_signals_both_endpoints() {
    let (handler, tx, rx) = make();
    handler.cancel();
    assert!(tx.is_cancelled());
    assert!(rx.is_cancelled());
}

#[test]
fn send_after_cancel_returns_cancelled_error() {
    rt().block_on(async {
        let (handler, mut tx, _rx) = make();
        handler.cancel();
        let result = tx.send(1_u32).await;
        assert!(matches!(result, Err(RuntimeError::Cancelled)));
    });
}

#[test]
fn recv_after_cancel_returns_none() {
    rt().block_on(async {
        let (handler, _tx, mut rx) = make();
        handler.cancel();
        assert_eq!(rx.recv().await, Ok(None));
    });
}

#[test]
fn send_and_recv_work_before_cancellation() {
    rt().block_on(async {
        let (handler, mut tx, mut rx) = make();
        tx.send(99_u32).await.expect("send failed");
        assert_eq!(rx.recv().await, Ok(Some(99_u32)));
        assert!(!handler.is_cancelled());
    });
}

#[test]
fn cancelable_sender_satisfies_base_sender_contract() {
    fn requires_sender<T: Send, S: Sender<T>>(_: &mut S) {}
    let (_handler, mut tx, _rx) = make();
    requires_sender::<u32, _>(&mut tx);
}

#[test]
fn cancelable_receiver_satisfies_base_receiver_contract() {
    fn requires_receiver<T: Send, R: Receiver<T>>(_: &mut R) {}
    let (_handler, _tx, mut rx) = make();
    requires_receiver::<u32, _>(&mut rx);
}

#[test]
fn recv_none_due_to_cancellation_is_distinguishable_from_closure() {
    rt().block_on(async {
        let (handler, _tx, mut rx) = make();
        handler.cancel();
        assert_eq!(rx.recv().await, Ok(None));
        assert!(handler.is_cancelled());
    });
}

#[test]
fn recv_none_due_to_closure_is_distinguishable_from_cancellation() {
    rt().block_on(async {
        let (handler, tx, mut rx) = make();
        drop(tx);
        assert_eq!(rx.recv().await, Ok(None));
        assert!(!handler.is_cancelled());
    });
}

#[test]
fn pending_recv_released_by_cancellation() {
    rt().block_on(async {
        let (handler, _tx, mut rx) = make();
        handler.cancel();
        assert_eq!(rx.recv().await, Ok(None));
    });
}
