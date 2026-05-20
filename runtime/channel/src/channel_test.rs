#![allow(clippy::expect_used)]

use runtime_core::channel::{Receiver, Sender};

use super::make_channel;
use crate::config::ChannelConfig;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime build failed")
}

#[test]
fn send_and_recv_transfers_value() {
    rt().block_on(async {
        let (mut tx, mut rx) = make_channel::<u32>(ChannelConfig::default());
        tx.send(42_u32).await.expect("send failed");
        assert_eq!(rx.recv().await, Ok(Some(42_u32)));
    });
}

#[test]
fn recv_returns_none_when_sender_dropped() {
    rt().block_on(async {
        let (tx, mut rx) = make_channel::<u32>(ChannelConfig::default());
        drop(tx);
        assert_eq!(rx.recv().await, Ok(None));
    });
}

#[test]
fn send_returns_error_when_receiver_dropped() {
    use runtime_core::RuntimeError;
    rt().block_on(async {
        let (mut tx, rx) = make_channel::<u32>(ChannelConfig::default());
        drop(rx);
        let result = tx.send(1_u32).await;
        assert!(matches!(result, Err(RuntimeError::ChannelClosed)));
    });
}

#[test]
fn multiple_sends_received_in_order() {
    rt().block_on(async {
        let (mut tx, mut rx) = make_channel::<u32>(ChannelConfig::new(8));
        tx.send(1_u32).await.expect("send 1");
        tx.send(2_u32).await.expect("send 2");
        tx.send(3_u32).await.expect("send 3");
        assert_eq!(rx.recv().await, Ok(Some(1_u32)));
        assert_eq!(rx.recv().await, Ok(Some(2_u32)));
        assert_eq!(rx.recv().await, Ok(Some(3_u32)));
    });
}

#[test]
fn send_future_does_not_block_before_poll() {
    rt().block_on(async {
        let (mut tx, rx) = make_channel::<u32>(ChannelConfig::default());
        let send_fut = tx.send(1_u32);
        drop(rx);
        drop(send_fut);
    });
}

#[test]
fn channel_uses_bounded_capacity() {
    rt().block_on(async {
        let (mut tx, mut rx) = make_channel::<u32>(ChannelConfig::new(1));
        tx.send(10_u32).await.expect("first send");
        assert_eq!(rx.recv().await, Ok(Some(10_u32)));
    });
}
