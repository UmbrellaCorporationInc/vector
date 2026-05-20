#![allow(clippy::expect_used)]

use runtime_core::CancelHandler;
use runtime_core::RuntimeError;
use runtime_core::cancel::{CancelableReceiver, CancelableSender};
use runtime_core::channel::{Receiver, Sender};

use super::{cancelable_channel, channel};

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime build failed")
}

// ---------------------------------------------------------------------------
// channel() — base factory
// ---------------------------------------------------------------------------

#[test]
fn channel_factory_sends_and_receives() {
    rt().block_on(async {
        let (mut tx, mut rx) = channel::<u32>();
        tx.send(7_u32).await.expect("send failed");
        assert_eq!(rx.recv().await, Ok(Some(7_u32)));
    });
}

#[test]
fn channel_factory_recv_returns_none_when_sender_dropped() {
    rt().block_on(async {
        let (tx, mut rx) = channel::<u32>();
        drop(tx);
        assert_eq!(rx.recv().await, Ok(None));
    });
}

#[test]
fn channel_factory_send_returns_error_when_receiver_dropped() {
    rt().block_on(async {
        let (mut tx, rx) = channel::<u32>();
        drop(rx);
        assert!(matches!(tx.send(1_u32).await, Err(RuntimeError::ChannelClosed)));
    });
}

#[test]
fn channel_factory_send_future_does_not_block_before_poll() {
    rt().block_on(async {
        let (mut tx, rx) = channel::<u32>();
        let fut = tx.send(1_u32);
        drop(rx);
        drop(fut);
    });
}

#[test]
fn channel_factory_hides_concrete_type_behind_impl_trait() {
    fn assert_sender<T: Send, S: Sender<T>>(_: &mut S) {}
    fn assert_receiver<T: Send, R: Receiver<T>>(_: &mut R) {}
    let (mut tx, mut rx) = channel::<u32>();
    assert_sender::<u32, _>(&mut tx);
    assert_receiver::<u32, _>(&mut rx);
}

// ---------------------------------------------------------------------------
// cancelable_channel() — cancel-aware factory
// ---------------------------------------------------------------------------

#[test]
fn cancelable_channel_factory_sends_and_receives() {
    rt().block_on(async {
        let (_handler, mut tx, mut rx) = cancelable_channel::<u32>();
        tx.send(42_u32).await.expect("send failed");
        assert_eq!(rx.recv().await, Ok(Some(42_u32)));
    });
}

#[test]
fn cancelable_channel_factory_not_cancelled_before_cancel() {
    let (handler, tx, rx) = cancelable_channel::<u32>();
    assert!(!handler.is_cancelled());
    assert!(!tx.is_cancelled());
    assert!(!rx.is_cancelled());
}

#[test]
fn cancelable_channel_factory_cancel_signals_both_endpoints() {
    let (handler, tx, rx) = cancelable_channel::<u32>();
    handler.cancel();
    assert!(tx.is_cancelled());
    assert!(rx.is_cancelled());
}

#[test]
fn cancelable_channel_factory_send_after_cancel_returns_cancelled_error() {
    rt().block_on(async {
        let (handler, mut tx, _rx) = cancelable_channel::<u32>();
        handler.cancel();
        assert!(matches!(tx.send(1_u32).await, Err(RuntimeError::Cancelled)));
    });
}

#[test]
fn cancelable_channel_factory_recv_after_cancel_returns_none() {
    rt().block_on(async {
        let (handler, _tx, mut rx) = cancelable_channel::<u32>();
        handler.cancel();
        assert_eq!(rx.recv().await, Ok(None));
    });
}

#[test]
fn cancelable_channel_factory_pending_recv_released_by_cancellation() {
    rt().block_on(async {
        let (handler, _tx, mut rx) = cancelable_channel::<u32>();
        handler.cancel();
        assert_eq!(rx.recv().await, Ok(None));
    });
}

#[test]
fn cancelable_channel_factory_cancellation_distinguishable_from_closure() {
    rt().block_on(async {
        let (handler, _tx, mut rx) = cancelable_channel::<u32>();
        handler.cancel();
        assert_eq!(rx.recv().await, Ok(None));
        assert!(handler.is_cancelled());
    });
}

#[test]
fn cancelable_channel_factory_closure_distinguishable_from_cancellation() {
    rt().block_on(async {
        let (handler, tx, mut rx) = cancelable_channel::<u32>();
        drop(tx);
        assert_eq!(rx.recv().await, Ok(None));
        assert!(!handler.is_cancelled());
    });
}

#[test]
fn cancelable_channel_factory_hides_concrete_types_behind_impl_trait() {
    fn assert_cancel_handler<H: CancelHandler>(_: &H) {}
    fn assert_cancelable_sender<T: Send, S: CancelableSender<T>>(_: &mut S) {}
    fn assert_cancelable_receiver<T: Send, R: CancelableReceiver<T>>(_: &mut R) {}
    let (handler, mut tx, mut rx) = cancelable_channel::<u32>();
    assert_cancel_handler(&handler);
    assert_cancelable_sender::<u32, _>(&mut tx);
    assert_cancelable_receiver::<u32, _>(&mut rx);
}
